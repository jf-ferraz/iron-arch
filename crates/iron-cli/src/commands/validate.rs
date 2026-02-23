//! Validate Command
//!
//! F2-015: Pre-apply configuration validation.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::apply::{ApplyService, ValidationSeverity};
use std::time::Instant;

/// Execute validate command
pub fn execute(ctx: &AppContext) -> Result<()> {
    let start = Instant::now();
    require_init(ctx)?;

    let output = &ctx.output;
    let service = ctx.apply_service();

    let host_id = ctx.current_host().unwrap_or_else(|| "default".to_string());

    output.header("Configuration Validation");
    output.info(&format!("Validating host '{}'...", host_id));

    let warnings = service.validate(&host_id)?;

    if output.is_json() {
        output.json_envelope("validate", &warnings, start);
        return Ok(());
    }

    if warnings.is_empty() {
        output.success("All configuration checks passed ✓");
        return Ok(());
    }

    let mut errors = 0;
    let mut warns = 0;
    let mut infos = 0;

    for w in &warnings {
        let badge = match w.severity {
            ValidationSeverity::Error => {
                errors += 1;
                StatusBadge::Error
            }
            ValidationSeverity::Warning => {
                warns += 1;
                StatusBadge::Warning
            }
            ValidationSeverity::Info => {
                infos += 1;
                StatusBadge::Ok
            }
        };

        output.list_item_status(&w.message, badge);
        if let Some(ref suggestion) = w.suggestion {
            output.verbose(&format!("  Hint: {}", suggestion));
        }
    }

    let status = if errors > 0 { "FAIL" } else { "PASS" };
    output.summary_block(
        &format!("Validation Result: {}", status),
        &[
            ("Errors", &errors.to_string()),
            ("Warnings", &warns.to_string()),
            ("Info", &infos.to_string()),
        ],
        None,
    );

    if errors > 0 {
        output.error("Validation failed. Fix errors before running 'iron apply'.");
    }

    Ok(())
}
