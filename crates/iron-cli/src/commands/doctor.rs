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
use std::time::Instant;

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
        "state_file" => Some("State file"),
        "directories" => Some("Directory Structure"),
        "current_host" => Some("Host Configuration"),
        "git" => Some("Git Status"),
        "tools" => Some("External Tools"),
        "packages" => Some("Package Installation"),
        "snapshot" => Some("Snapshot Backend"),
        "secrets" => Some("Secrets Status"),
        "symlinks" => Some("Symlink Integrity"),
        "services" => Some("Service Availability"),
        "security_modules" => Some("Security Modules"),
        "firewall" => Some("Firewall"),
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
    let start = Instant::now();
    require_init(ctx)?;

    let output = &ctx.output;

    // Build DoctorConfig from AppContext
    let host_id = ctx.current_host();
    let active_bundle = host_id.as_ref().and_then(|h| ctx.state.active_bundle(h));

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
        output.json_envelope("doctor", &report, start);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_for_pass() {
        assert!(matches!(badge_for(CheckStatus::Pass), StatusBadge::Ok));
    }

    #[test]
    fn test_badge_for_warn() {
        assert!(matches!(badge_for(CheckStatus::Warn), StatusBadge::Warning));
    }

    #[test]
    fn test_badge_for_fail() {
        assert!(matches!(badge_for(CheckStatus::Fail), StatusBadge::Error));
    }

    #[test]
    fn test_section_header_known() {
        assert_eq!(section_header("state_file"), Some("State file"));
        assert_eq!(section_header("directories"), Some("Directory Structure"));
        assert_eq!(section_header("current_host"), Some("Host Configuration"));
        assert_eq!(section_header("git"), Some("Git Status"));
        assert_eq!(section_header("tools"), Some("External Tools"));
        assert_eq!(section_header("packages"), Some("Package Installation"));
        assert_eq!(section_header("snapshot"), Some("Snapshot Backend"));
        assert_eq!(section_header("secrets"), Some("Secrets Status"));
        assert_eq!(section_header("symlinks"), Some("Symlink Integrity"));
        assert_eq!(section_header("services"), Some("Service Availability"));
        assert_eq!(section_header("security_modules"), Some("Security Modules"));
        assert_eq!(section_header("firewall"), Some("Firewall"));
    }

    #[test]
    fn test_section_header_unknown() {
        assert_eq!(section_header("unknown_check"), None);
        assert_eq!(section_header(""), None);
    }

    #[test]
    fn test_health_check_with_details() {
        let check = HealthCheck {
            name: "packages".to_string(),
            status: CheckStatus::Warn,
            message: "2 missing packages".to_string(),
            details: vec!["git-delta".to_string(), "starship".to_string()],
        };
        assert_eq!(check.details.len(), 2);
        assert_eq!(check.status, CheckStatus::Warn);
    }

    #[test]
    fn test_health_report_error_counting() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "a".to_string(),
                    status: CheckStatus::Pass,
                    message: "ok".to_string(),
                    details: vec![],
                },
                HealthCheck {
                    name: "b".to_string(),
                    status: CheckStatus::Fail,
                    message: "fail".to_string(),
                    details: vec![],
                },
                HealthCheck {
                    name: "c".to_string(),
                    status: CheckStatus::Warn,
                    message: "warn".to_string(),
                    details: vec![],
                },
            ],
            overall: CheckStatus::Fail,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(report.errors(), 1);
        assert_eq!(report.warnings(), 1);
    }
}
