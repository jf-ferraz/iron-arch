//! Security Status Command
//!
//! F2-017: Show current security level and recommendations.

use crate::context::{AppContext, require_init};
use anyhow::Result;
use iron_core::services::security::SecurityService;
use std::time::Instant;

/// Execute security status command
pub fn execute(ctx: &AppContext) -> Result<()> {
    let start = Instant::now();
    require_init(ctx)?;

    let output = &ctx.output;
    let service = ctx.security_service();

    let report = service.calculate()?;

    if output.is_json() {
        output.json_envelope("security", &report, start);
        return Ok(());
    }

    output.header("Security Status");

    // Level badge
    let level_ansi = match report.level {
        iron_core::services::security::SecurityLevel::Basic => "\x1b[31m",
        iron_core::services::security::SecurityLevel::Standard => "\x1b[33m",
        iron_core::services::security::SecurityLevel::Advanced => "\x1b[32m",
        iron_core::services::security::SecurityLevel::Paranoid => "\x1b[36m",
    };
    let level_label = output.colored(report.level.label(), level_ansi);
    output.raw(&format!(
        "  Level: {} ({}/{} points)",
        level_label, report.score, report.max_score
    ));

    // Enabled modules
    if !report.enabled_modules.is_empty() {
        output.subheader("Enabled Security Modules");
        let rows: Vec<Vec<String>> = report
            .enabled_modules
            .iter()
            .map(|m| vec![m.id.clone(), m.name.clone(), format!("+{} pts", m.points)])
            .collect();
        output.table(&["ID", "NAME", "POINTS"], &rows);
    }

    // Recommendations
    if !report.recommendations.is_empty() {
        output.subheader("Recommendations");
        for rec in report.recommendations.iter().take(5) {
            output.list_item(rec);
        }
    }

    Ok(())
}
