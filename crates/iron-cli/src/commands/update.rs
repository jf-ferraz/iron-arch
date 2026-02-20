//! Iron Update Command
//!
//! Safe system update with risk assessment and partial update recovery (FR-5.10).

use crate::context::{AppContext, require_init};
use crate::output::{StatusBadge, render_risk};
use anyhow::Result;
use iron_core::OperationSpan;
use iron_core::services::update::{UpdateRisk, UpdateService};
use serde::Serialize;
use std::io::{self, Write};
use tracing::info;

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

/// JSON output for update progress status
#[derive(Serialize)]
struct ProgressStatus {
    in_progress: bool,
    interrupted: bool,
    session_id: Option<String>,
    started_at: Option<String>,
    total_packages: usize,
    completed_packages: usize,
    remaining_packages: usize,
    completion_percentage: f64,
    phase: String,
    snapshot_id: Option<String>,
}

/// Execute update command
pub fn execute(
    ctx: &AppContext,
    dry_run: bool,
    force: bool,
    no_snapshot: bool,
    resume: bool,
    status: bool,
    clear_progress: bool,
    yes: bool,
) -> Result<()> {
    require_init(ctx)?;

    // Create operation span for log correlation (NFR-9)
    let op_span = OperationSpan::new("update").with_component("update_command");
    let _guard = op_span.enter();

    info!(
        dry_run = dry_run,
        force = force,
        resume = resume,
        "Starting update operation"
    );

    let output = &ctx.output;
    let update_service = ctx.update_service();

    // Handle --status flag (FR-5.10)
    if status {
        return show_progress_status(ctx, &update_service);
    }

    // Handle --clear-progress flag (FR-5.10)
    if clear_progress {
        return clear_update_progress(ctx, &update_service);
    }

    // Handle --resume flag (FR-5.10)
    if resume {
        return resume_interrupted_update(ctx, &update_service);
    }

    // Check for interrupted update (FR-5.10)
    if let Some(interrupted) = update_service.check_interrupted() {
        return handle_interrupted_update(ctx, &update_service, interrupted, yes);
    }

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

    // Confirmation (use --yes to auto-approve low risk updates)
    if !force && can_proceed {
        let auto_approve = yes && matches!(overall_risk, UpdateRisk::Low);

        if !auto_approve {
            print!("\nProceed with update? [y/N] ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                output.info("Update cancelled");
                return Ok(());
            }
        } else {
            output.info("Auto-approved (low risk with --yes flag)");
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

    output.info("Running system update with progress tracking...");

    // Use apply_with_progress for FR-5.10 partial update recovery
    let result = update_service.apply_with_progress(
        &plan,
        create_snapshot,
        Some(&|progress| {
            // Progress callback - could be used for real-time display
            let pct = progress.completion_percentage();
            let completed = progress.completed_packages.len();
            let total = progress.total_packages;
            // In a real implementation, we'd update a progress bar here
            // For now, this runs silently
            let _ = (pct, completed, total);
        }),
    );

    match result {
        Ok(()) => {
            output.separator();
            output.success("System updated successfully");

            // Check for pacnew files
            let pacnew_count = check_pacnew_files();
            if pacnew_count > 0 {
                output.warning(&format!("{} .pacnew files detected", pacnew_count));
                output.info("Review with: sudo pacdiff");
            }
        }
        Err(e) => {
            output.error(&format!("Update failed: {}", e));
            output.info("Run 'iron update --status' to check progress");
            output.info("Run 'iron update --resume' to resume if interrupted");
            return Err(e.into());
        }
    }

    Ok(())
}

/// Show update progress status (FR-5.10)
fn show_progress_status(ctx: &AppContext, update_service: &impl UpdateService) -> Result<()> {
    let output = &ctx.output;

    if let Some(progress) = update_service.get_progress() {
        let completed = progress.completed_packages.len();
        let remaining = progress.total_packages.saturating_sub(completed);
        let pct = progress.completion_percentage();
        let phase = format!("{:?}", progress.phase);

        if output.is_json() {
            let status = ProgressStatus {
                in_progress: true,
                interrupted: progress.is_incomplete(),
                session_id: Some(progress.session_id.clone()),
                started_at: Some(progress.started_at.to_rfc3339()),
                total_packages: progress.total_packages,
                completed_packages: completed,
                remaining_packages: remaining,
                completion_percentage: pct,
                phase: phase.clone(),
                snapshot_id: progress.snapshot_id.clone(),
            };
            output.json(&status);
            return Ok(());
        }

        output.header("Update Progress Status");
        output.kv("Session ID", &progress.session_id);
        output.kv("Started At", progress.started_at.to_rfc3339());
        output.kv("Phase", &phase);
        output.kv(
            "Progress",
            format!("{}/{} ({:.1}%)", completed, progress.total_packages, pct),
        );
        output.kv("Remaining", remaining.to_string());

        if let Some(snap_id) = &progress.snapshot_id {
            output.kv("Snapshot", snap_id);
        }

        if progress.is_incomplete() {
            output.warning("Update was interrupted!");
            output.info("Run 'iron update --resume' to continue");
        }

        // Show completed packages
        if !progress.completed_packages.is_empty() && output.is_verbose() {
            output.subheader("Completed Packages");
            for pkg in &progress.completed_packages {
                output.list_item(&format!(
                    "{}: {} -> {}",
                    pkg.name, pkg.old_version, pkg.new_version
                ));
            }
        }

        // Show remaining packages
        let remaining_pkgs = progress.remaining_packages();
        if !remaining_pkgs.is_empty() && output.is_verbose() {
            output.subheader("Remaining Packages");
            for pkg in remaining_pkgs {
                output.list_item(&format!(
                    "{}: {} -> {}",
                    pkg.name, pkg.current_version, pkg.new_version
                ));
            }
        }
    } else {
        if output.is_json() {
            let status = ProgressStatus {
                in_progress: false,
                interrupted: false,
                session_id: None,
                started_at: None,
                total_packages: 0,
                completed_packages: 0,
                remaining_packages: 0,
                completion_percentage: 0.0,
                phase: "None".to_string(),
                snapshot_id: None,
            };
            output.json(&status);
            return Ok(());
        }

        output.info("No update in progress");
    }

    Ok(())
}

/// Clear update progress (FR-5.10)
fn clear_update_progress(ctx: &AppContext, update_service: &impl UpdateService) -> Result<()> {
    let output = &ctx.output;

    if update_service.get_progress().is_some() {
        update_service.clear_progress()?;
        output.success("Update progress cleared");
    } else {
        output.info("No update progress to clear");
    }

    Ok(())
}

/// Resume interrupted update (FR-5.10)
fn resume_interrupted_update(ctx: &AppContext, update_service: &impl UpdateService) -> Result<()> {
    let output = &ctx.output;

    if let Some(interrupted) = update_service.check_interrupted() {
        output.header("Resuming Interrupted Update");
        output.kv(
            "Completed",
            format!("{} packages", interrupted.completed_count),
        );
        output.kv(
            "Remaining",
            format!("{} packages", interrupted.remaining_count),
        );
        output.kv(
            "Elapsed",
            format!("{} minutes", interrupted.elapsed.num_minutes()),
        );

        output.info("Installing remaining packages...");
        update_service.resume()?;
        output.success("Update resumed and completed successfully");
    } else {
        output.warning("No interrupted update to resume");
        output.info("Run 'iron update' to check for and apply updates");
    }

    Ok(())
}

/// Handle interrupted update prompt (FR-5.10)
fn handle_interrupted_update(
    ctx: &AppContext,
    update_service: &impl UpdateService,
    interrupted: iron_core::services::update::InterruptedUpdate,
    auto_resume: bool,
) -> Result<()> {
    let output = &ctx.output;

    output.warning("Previous update was interrupted!");
    output.kv("Started", interrupted.progress.started_at.to_rfc3339());
    output.kv(
        "Progress",
        format!(
            "{}/{} packages completed ({:.1}%)",
            interrupted.completed_count,
            interrupted.progress.total_packages,
            interrupted.progress.completion_percentage()
        ),
    );
    output.kv(
        "Remaining",
        format!("{} packages", interrupted.remaining_count),
    );

    if let Some(snap_id) = &interrupted.progress.snapshot_id {
        output.kv("Snapshot", snap_id);
    }

    output.raw("");

    // Auto-resume if --yes flag is set
    if auto_resume {
        output.info("Auto-resuming with --yes flag...");
        update_service.resume()?;
        output.success("Update resumed and completed successfully");
        return Ok(());
    }

    // Interactive prompt
    output.info("Options:");
    output.list_item("[R]esume - Install remaining packages");
    output.list_item("[C]lear - Clear progress and start fresh");
    output.list_item("[A]bort - Do nothing");
    output.raw("");

    print!("What would you like to do? [R/c/a] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "r" | "" => {
            output.info("Resuming update...");
            update_service.resume()?;
            output.success("Update resumed and completed successfully");
        }
        "c" => {
            update_service.clear_progress()?;
            output.info("Progress cleared. Run 'iron update' for a fresh update.");
        }
        _ => {
            output.info("Update aborted. Progress preserved.");
            output.info("Run 'iron update --resume' to continue later.");
        }
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
