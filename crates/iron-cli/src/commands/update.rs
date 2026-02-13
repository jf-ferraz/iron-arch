//! Iron Update Command
//!
//! Safe system update with risk assessment.

use crate::context::{AppContext, require_init};
use crate::output::{StatusBadge, render_risk};
use anyhow::Result;
use iron_core::services::update::{UpdateRisk, UpdateService};
use serde::Serialize;
use std::io::{self, Write};

#[derive(Serialize)]
struct UpdatePreview {
    risk: String,
    packages: Vec<PackageInfo>,
    can_proceed: bool,
}

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    current: String,
    new: String,
    risk: String,
}

/// Execute update command
pub fn execute(ctx: &AppContext, dry_run: bool, force: bool, no_snapshot: bool) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let update_service = ctx.update_service();

    output.header("Iron Safe Update");

    // Check for updates
    output.info("Checking for updates...");
    let plan = update_service.check()?;

    if plan.packages.is_empty() {
        output.success("System is up to date");
        return Ok(());
    }

    // Display update info
    output.subheader(&format!("{} packages to update", plan.packages.len()));

    // Group by risk
    let critical: Vec<_> = plan
        .packages
        .iter()
        .filter(|p| matches!(p.risk, UpdateRisk::Critical))
        .collect();
    let high: Vec<_> = plan
        .packages
        .iter()
        .filter(|p| matches!(p.risk, UpdateRisk::High))
        .collect();
    let medium: Vec<_> = plan
        .packages
        .iter()
        .filter(|p| matches!(p.risk, UpdateRisk::Medium))
        .collect();
    let low: Vec<_> = plan
        .packages
        .iter()
        .filter(|p| matches!(p.risk, UpdateRisk::Low))
        .collect();

    if !critical.is_empty() {
        output.warning(&format!("Critical risk packages ({})", critical.len()));
        for pkg in &critical {
            output.list_item_status(
                &format!(
                    "{}: {} -> {}",
                    pkg.name, pkg.current_version, pkg.new_version
                ),
                StatusBadge::Error,
            );
            if let Some(reason) = &pkg.risk_reason {
                output.verbose(&format!("  Reason: {}", reason));
            }
        }
    }

    if !high.is_empty() {
        output.warning(&format!("High risk packages ({})", high.len()));
        for pkg in &high {
            output.list_item_status(
                &format!(
                    "{}: {} -> {}",
                    pkg.name, pkg.current_version, pkg.new_version
                ),
                StatusBadge::Warning,
            );
            if let Some(reason) = &pkg.risk_reason {
                output.verbose(&format!("  Reason: {}", reason));
            }
        }
    }

    if !medium.is_empty() && output.is_verbose() {
        output.info(&format!("Medium risk packages ({})", medium.len()));
        for pkg in &medium {
            output.list_item(&format!(
                "{}: {} -> {}",
                pkg.name, pkg.current_version, pkg.new_version
            ));
        }
    }

    if !low.is_empty() && output.is_verbose() {
        output.info(&format!("Low risk packages ({})", low.len()));
        for pkg in &low {
            output.list_item(&format!(
                "{}: {} -> {}",
                pkg.name, pkg.current_version, pkg.new_version
            ));
        }
    }

    // Overall risk assessment
    output.separator();
    let overall_risk = plan.overall_risk;
    let risk_str = format!("{:?}", overall_risk);
    output.kv("Overall Risk", render_risk(&risk_str, false));

    // Check if update should proceed
    let can_proceed = match overall_risk {
        UpdateRisk::Low | UpdateRisk::Medium => true,
        UpdateRisk::High => {
            output.warning("High risk update detected!");
            force
        }
        UpdateRisk::Critical => {
            output.error("Critical risk update - manual intervention recommended");
            if !force {
                output.info("Manual steps:");
                output.list_item("Review Arch Linux news: https://archlinux.org/news/");
                output.list_item("Create a system snapshot");
                output.list_item("Run 'iron update --force' if you understand the risks");
                return Ok(());
            }
            true
        }
    };

    // JSON output
    if output.is_json() {
        let preview = UpdatePreview {
            risk: risk_str.clone(),
            packages: plan
                .packages
                .iter()
                .map(|p| PackageInfo {
                    name: p.name.clone(),
                    current: p.current_version.clone(),
                    new: p.new_version.clone(),
                    risk: format!("{:?}", p.risk),
                })
                .collect(),
            can_proceed,
        };
        output.json(&preview);
        if dry_run {
            return Ok(());
        }
    }

    // Dry run stops here
    if dry_run {
        output.info("Dry run - no changes made");
        return Ok(());
    }

    // Confirmation
    if !force && can_proceed {
        print!("\nProceed with update? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output.info("Update cancelled");
            return Ok(());
        }
    }

    if !can_proceed {
        output.error("Update aborted due to risk level");
        return Ok(());
    }

    // Execute update with or without snapshot
    let create_snapshot = !no_snapshot && plan.snapshot_recommended;
    if create_snapshot {
        output.info("Creating system snapshot...");
    }

    output.info("Running system update...");
    update_service.apply(create_snapshot)?;

    output.separator();
    output.success("System updated successfully");

    // Check for pacnew files
    let pacnew_count = check_pacnew_files();
    if pacnew_count > 0 {
        output.warning(&format!("{} .pacnew files detected", pacnew_count));
        output.info("Review with: sudo pacdiff");
    }

    Ok(())
}

/// Check for .pacnew files
fn check_pacnew_files() -> usize {
    std::process::Command::new("find")
        .args(["/etc", "-name", "*.pacnew", "-type", "f"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0)
}
