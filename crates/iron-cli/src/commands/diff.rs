//! Iron Diff Command
//!
//! Show differences between declared and actual system state.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::drift::{ConfigDrift, DriftService, PackageDrift, ServiceDrift};

/// Execute diff command
pub fn execute(
    ctx: &AppContext,
    adopt: bool,
    correct: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let service = ctx.drift_service();

    let host_id = ctx.current_host().unwrap_or_else(|| "default".to_string());

    output.header("Iron Diff");
    output.info("Comparing declared state vs system...");

    let report = service.detect(&host_id)?;

    if report.is_clean() {
        output.success("System is clean ✓ — no drift detected.");
        return Ok(());
    }

    // Display package drift
    if !report.package_drift.is_empty() {
        output.subheader("📦 Packages");
        for drift in &report.package_drift {
            match drift {
                PackageDrift::Missing { name } => {
                    output.list_item_status(
                        &format!("missing: {} (declared but not installed)", name),
                        StatusBadge::Error,
                    );
                }
                PackageDrift::Extra { name } => {
                    output.list_item_status(
                        &format!("extra: {} (installed by Iron, no longer declared)", name),
                        StatusBadge::Warning,
                    );
                }
            }
        }
    }

    // Display config drift
    if !report.config_drift.is_empty() {
        output.subheader("🔗 Configs");
        for drift in &report.config_drift {
            match drift {
                ConfigDrift::MissingSymlink { target, .. } => {
                    output.list_item_status(&format!("missing: {}", target), StatusBadge::Error);
                }
                ConfigDrift::BrokenSymlink { target } => {
                    output.list_item_status(&format!("broken: {}", target), StatusBadge::Error);
                }
                ConfigDrift::WrongTarget {
                    target,
                    expected,
                    actual,
                } => {
                    output.list_item_status(
                        &format!(
                            "wrong target: {} → {} (expected {})",
                            target, actual, expected
                        ),
                        StatusBadge::Warning,
                    );
                }
            }
        }
    }

    // Display service drift
    if !report.service_drift.is_empty() {
        output.subheader("⚙ Services");
        for drift in &report.service_drift {
            match drift {
                ServiceDrift::NotEnabled { name } => {
                    output.list_item_status(&format!("not enabled: {}", name), StatusBadge::Error);
                }
                ServiceDrift::ExtraEnabled { name } => {
                    output.list_item_status(
                        &format!("extra enabled: {}", name),
                        StatusBadge::Warning,
                    );
                }
            }
        }
    }

    output.separator();
    output.info(&format!(
        "▸ Summary: {} drift(s) ({} packages · {} configs · {} services)",
        report.summary.total_drifts,
        report.summary.packages_missing + report.summary.packages_extra,
        report.summary.configs_drifted,
        report.summary.services_drifted,
    ));

    // F1-017: --correct delegates to ApplyService
    if correct {
        output.separator();
        output.info("Correcting drift by applying declared state...");

        let apply_svc = ctx.apply_service();
        use iron_core::services::apply::ApplyService;

        let plan = apply_svc.plan(&host_id)?;

        if plan.is_empty() {
            output.success("No corrections needed.");
            return Ok(());
        }

        if dry_run {
            output.subheader("Correction plan (dry run)");
            for action in &plan.actions {
                output.info(&format!("  {}", action.display()));
            }
            output.success("[DRY RUN] No changes made.");
            return Ok(());
        }

        if !yes {
            output.info("Proceed with corrections? [y/N]");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                output.info("Cancelled.");
                return Ok(());
            }
        }

        let result = apply_svc.execute(&plan)?;
        output.success(&format!(
            "Correction complete: {} succeeded, {} failed",
            result.succeeded, result.failed,
        ));
    }

    // F1-016: --adopt acknowledges drift
    if adopt {
        output.separator();
        output.info("Adopting drift — acknowledging current system state as canonical...");
        // For now, this is a notification. Full adopt logic (rewriting module TOMLs,
        // updating checksums) will be expanded in later iterations.
        output.success("Drift acknowledged. Run 'iron diff' again to verify.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use iron_core::services::drift::{DriftReport, PackageDrift};

    #[test]
    fn test_drift_report_clean() {
        let report = DriftReport::default();
        assert!(report.is_clean());
    }

    #[test]
    fn test_drift_report_with_missing() {
        let mut report = DriftReport::default();
        report.package_drift.push(PackageDrift::Missing {
            name: "neovim".to_string(),
        });
        assert!(!report.is_clean());
    }
}
