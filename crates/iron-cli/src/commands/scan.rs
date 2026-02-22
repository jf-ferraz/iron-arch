//! Iron Scan Command
//!
//! Scans the system for existing configs, package overlaps, and conflicts.
//! Thin CLI wrapper around the shared `ScanService` from iron-core.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::scan::{
    ConflictSeverity, DefaultScanService, ScanReport, ScanService,
};
use iron_core::services::{BundleService, ModuleService};
use std::sync::Arc;

/// Map a `ConflictSeverity` to a CLI `StatusBadge`.
fn badge_for(severity: ConflictSeverity) -> StatusBadge {
    match severity {
        ConflictSeverity::Info => StatusBadge::Ok,
        ConflictSeverity::Warning => StatusBadge::Warning,
        ConflictSeverity::Error => StatusBadge::Error,
    }
}

/// Execute the scan command.
pub fn execute(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;

    // Gather bundles and modules for comparison
    let bundles = ctx.bundle_service().discover().unwrap_or_default();
    let modules = ctx.module_service().discover().unwrap_or_default();

    // Build scan service
    let home_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/home"));
    let package_manager = Arc::new(iron_pacman::DefaultPackageManager::default());
    let scan_service = DefaultScanService::new(&home_dir, package_manager);

    // Run scan
    let report: ScanReport = scan_service.scan(&bundles, &modules)?;

    // Render output
    if output.is_json() {
        output.json(&report);
    } else {
        output.header("System Scan Results");

        // Summary
        output.subheader("Summary");
        output.list_item(&format!(
            "Configs scanned: {}",
            report.summary.configs_scanned
        ));
        output.list_item(&format!(
            "Packages already installed: {}",
            report.summary.packages_already_installed
        ));
        output.list_item(&format!(
            "Conflicts found: {}",
            report.summary.conflicts_found
        ));
        output.list_item(&format!(
            "Recommendations: {}",
            report.summary.recommendations_count
        ));

        // Discovered configs
        if !report.existing_configs.is_empty() {
            output.subheader("Discovered Configs");
            for config in &report.existing_configs {
                let tag = if config.is_symlink {
                    " [symlink]"
                } else {
                    " [file]"
                };
                output.list_item_status(
                    &format!("{} ({}){}", config.path.display(), config.app_name, tag),
                    StatusBadge::Ok,
                );
            }
        }

        // Conflicts
        if !report.potential_conflicts.is_empty() {
            output.subheader("Potential Conflicts");
            for conflict in &report.potential_conflicts {
                output.list_item_status(
                    &format!(
                        "{}: {} (managed by {})",
                        conflict.path.display(),
                        conflict.description,
                        conflict.managed_by
                    ),
                    badge_for(conflict.severity),
                );
            }
        }

        // Installed package overlap
        if !report.installed_packages.is_empty() {
            output.subheader("Installed Package Overlap");
            for pkg in &report.installed_packages {
                output.list_item_status(pkg, StatusBadge::Ok);
            }
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            output.subheader("Recommendations");
            for (i, rec) in report.recommendations.iter().enumerate() {
                output.list_item(&format!("{}. {}", i + 1, rec));
            }
        }

        // Overall result
        output.separator();
        if report.potential_conflicts.is_empty() {
            output.success("No conflicts detected — safe to proceed");
        } else {
            output.warning(&format!(
                "{} conflict(s) require attention before applying configs",
                report.potential_conflicts.len()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_for_maps_severity() {
        assert!(matches!(badge_for(ConflictSeverity::Info), StatusBadge::Ok));
        assert!(matches!(
            badge_for(ConflictSeverity::Warning),
            StatusBadge::Warning
        ));
        assert!(matches!(
            badge_for(ConflictSeverity::Error),
            StatusBadge::Error
        ));
    }
}
