//! Iron History Command
//!
//! F3-016: View operation history from state.last_operations.

use crate::cli::HistoryAction;
use crate::context::AppContext;
use anyhow::Result;
use iron_core::services::history::{DefaultHistoryService, HistoryService};
use iron_core::state::OperationStatus;
use std::time::Instant;

/// Execute `iron history` command.
pub fn execute(ctx: &AppContext, action: &Option<HistoryAction>, limit: usize) -> Result<()> {
    let start = Instant::now();
    let svc = DefaultHistoryService::new(ctx.state.clone());

    match action {
        None | Some(HistoryAction::List) => execute_list(ctx, &svc, limit, start),
        Some(HistoryAction::Show { id }) => execute_show(ctx, &svc, *id, start),
        Some(HistoryAction::Last) => execute_last(ctx, &svc, start),
    }
}

fn execute_list(
    ctx: &AppContext,
    svc: &DefaultHistoryService,
    limit: usize,
    start: Instant,
) -> Result<()> {
    let output = &ctx.output;
    let entries = svc.list(limit)?;

    if output.is_json() {
        output.json_envelope("history", &entries, start);
        return Ok(());
    }

    if entries.is_empty() {
        output.info("No operations recorded yet.");
        return Ok(());
    }

    output.header("Operation History");

    let headers = &["#", "Time", "Operation", "Duration", "Actions", "Status"];
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|e| {
            vec![
                format!("{}", e.index),
                format_timestamp(&e.timestamp),
                e.operation.clone(),
                e.duration_secs
                    .map(|d| format!("{:.1}s", d))
                    .unwrap_or_else(|| "-".to_string()),
                e.action_count
                    .map(|c| format!("{}", c))
                    .unwrap_or_else(|| "-".to_string()),
                format_status(&e.status),
            ]
        })
        .collect();

    output.table(headers, &rows);

    output.info(&format!(
        "Showing {} of {} total operations",
        entries.len(),
        entries.len()
    ));

    Ok(())
}

fn execute_show(
    ctx: &AppContext,
    svc: &DefaultHistoryService,
    id: usize,
    start: Instant,
) -> Result<()> {
    let output = &ctx.output;

    let entry = svc.show(id)?;

    if output.is_json() {
        output.json_envelope("history.show", &entry, start);
        return Ok(());
    }

    match entry {
        Some(e) => {
            output.header(&format!("Operation #{}", e.index));
            output.kv("Operation", &e.operation);
            output.kv("Time", format_timestamp(&e.timestamp));
            output.kv("Status", format_status(&e.status));
            if let Some(d) = e.duration_secs {
                output.kv("Duration", format!("{:.1}s", d));
            }
            if let Some(c) = e.action_count {
                output.kv("Actions", c);
            }
            if let Some(ref details) = e.details {
                output.subheader("Details");
                for line in details.lines() {
                    output.list_item(line);
                }
            }
        }
        None => {
            output.warning(&format!(
                "No operation found at index {}. \
                 Use 'iron history' to see available operations.",
                id
            ));
        }
    }

    Ok(())
}

fn execute_last(ctx: &AppContext, svc: &DefaultHistoryService, start: Instant) -> Result<()> {
    let output = &ctx.output;

    let entry = svc.last()?;

    if output.is_json() {
        output.json_envelope("history.last", &entry, start);
        return Ok(());
    }

    match entry {
        Some(e) => {
            output.header("Last Operation");
            output.kv("Operation", &e.operation);
            output.kv("Time", format_timestamp(&e.timestamp));
            output.kv("Status", format_status(&e.status));
            if let Some(d) = e.duration_secs {
                output.kv("Duration", format!("{:.1}s", d));
            }
            if let Some(c) = e.action_count {
                output.kv("Actions", c);
            }
            if let Some(ref details) = e.details {
                output.subheader("Details");
                for line in details.lines() {
                    output.list_item(line);
                }
            }
        }
        None => {
            output.info("No operations recorded yet.");
        }
    }

    Ok(())
}

/// Format a timestamp for display (relative-ish)
fn format_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*ts);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        ts.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// Format OperationStatus for display
fn format_status(status: &OperationStatus) -> String {
    match status {
        OperationStatus::Success => "OK".to_string(),
        OperationStatus::Failed => "FAILED".to_string(),
        OperationStatus::Partial => "PARTIAL".to_string(),
        OperationStatus::Skipped => "SKIPPED".to_string(),
    }
}
