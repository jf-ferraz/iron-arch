//! Iron Doctor Command
//!
//! Thin CLI wrapper around the shared `DoctorService` from iron-core.
//! Implements FR-10.1 through FR-10.8 health diagnostics requirements.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::detect_snapshot_backend;
use iron_core::services::doctor::{
    CheckStatus, DefaultDoctorService, DoctorConfig, DoctorService, HealthCheck, HealthReport,
};

/// Map a `CheckStatus` to a CLI `StatusBadge`.
fn badge_for(status: CheckStatus) -> StatusBadge {
    match status {
        CheckStatus::Pass => StatusBadge::Ok,
        CheckStatus::Warn => StatusBadge::Warning,
        CheckStatus::Fail => StatusBadge::Error,
    }
}

/// Section headers per check name (for pretty-printed output)
fn section_header(name: &str) -> Option<&'static str> {
    match name {
        "directories" => Some("Directory Structure"),
        "current_host" => Some("Host Configuration"),
        "git" => Some("Git Status"),
        "tools" => Some("External Tools"),
        "packages" => Some("Package Installation"),
        "snapshot" => Some("Snapshot Backend"),
        "secrets" => Some("Secrets Status"),
        "symlinks" => Some("Symlink Integrity"),
        "services" => Some("Service Availability"),
        _ => None,
    }
}

/// Print a single `HealthCheck` with section headers and detail lines.
fn print_check(ctx: &AppContext, check: &HealthCheck) {
    let output = &ctx.output;

    if let Some(header) = section_header(&check.name) {
        output.subheader(header);
    }

    // Print any detail sub-items first (e.g. each missing package or broken symlink)
    if !check.details.is_empty() {
        for detail in &check.details {
            output.list_item_status(detail, badge_for(check.status));
        }
    }

    // Main status line
    output.list_item_status(&check.message, badge_for(check.status));
}

/// Execute doctor command
pub fn execute(ctx: &AppContext) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;

    // Build DoctorConfig from AppContext
    let host_id = ctx.current_host();
    let active_bundle = host_id
        .as_ref()
        .and_then(|h| ctx.state.active_bundle(h));

    let config = DoctorConfig {
        root: ctx.root.clone(),
        current_host: host_id,
        active_bundle,
        snapshot_backend: detect_snapshot_backend(),
    };

    // Run all checks via the shared service
    let report: HealthReport = DefaultDoctorService::new(config).check_all()?;

    // --- Render output ---

    if output.is_json() {
        output.json(&report);
    } else {
        output.header("Iron Health Check");

        for check in &report.checks {
            print_check(ctx, check);
        }

        // Summary
        output.separator();
        let errors = report.errors();
        let warnings = report.warnings();

        match report.overall {
            CheckStatus::Fail => {
                output.error(&format!("{} errors, {} warnings", errors, warnings));
            }
            CheckStatus::Warn => {
                output.warning(&format!("{} warnings", warnings));
            }
            CheckStatus::Pass => {
                output.success("All checks passed");
            }
        }
    }

    if report.overall == CheckStatus::Fail {
        std::process::exit(1);
    }

    Ok(())
}
