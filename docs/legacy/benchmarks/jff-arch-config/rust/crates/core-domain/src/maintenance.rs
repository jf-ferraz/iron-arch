use chrono::{DateTime, Utc};
use std::io;
use std::path::Path;

/// Get time since last operation in seconds
pub fn time_since_last(last_run: &Option<DateTime<Utc>>) -> Option<i64> {
    last_run.map(|dt| {
        let now = Utc::now();
        (now - dt).num_seconds()
    })
}

/// Format time ago as human-readable string
pub fn format_time_ago(seconds: i64) -> String {
    if seconds < 60 {
        format!("{seconds}s ago")
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}d ago", seconds / 86400)
    }
}

/// Get maintenance operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceStatus {
    /// Recently run (< 1 day)
    Recent,
    /// Needs attention (1-7 days)
    NeedsAttention,
    /// Overdue (> 7 days)
    Overdue,
    /// Never run
    Never,
}

impl MaintenanceStatus {
    pub fn from_last_run(last_run: &Option<DateTime<Utc>>) -> Self {
        match time_since_last(last_run) {
            None => MaintenanceStatus::Never,
            Some(s) if s < 86400 => MaintenanceStatus::Recent,       // < 1 day
            Some(s) if s < 604800 => MaintenanceStatus::NeedsAttention, // < 7 days
            Some(_) => MaintenanceStatus::Overdue,                   // > 7 days
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            MaintenanceStatus::Recent => "🟢",
            MaintenanceStatus::NeedsAttention => "🟡",
            MaintenanceStatus::Overdue => "🔴",
            MaintenanceStatus::Never => "⚪",
        }
    }

    /// Text-based status tag for non-emoji display
    pub fn tag(&self) -> &'static str {
        match self {
            MaintenanceStatus::Recent => "[OK]",
            MaintenanceStatus::NeedsAttention => "[--]",
            MaintenanceStatus::Overdue => "[!!]",
            MaintenanceStatus::Never => "[??]",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MaintenanceStatus::Recent => "Recent",
            MaintenanceStatus::NeedsAttention => "Needs Attention",
            MaintenanceStatus::Overdue => "Overdue",
            MaintenanceStatus::Never => "Never Run",
        }
    }
}

/// Get all maintenance statuses
pub fn get_maintenance_statuses(
    root: &Path,
) -> io::Result<(MaintenanceStatus, MaintenanceStatus, MaintenanceStatus)> {
    let state = crate::get_maintenance_state(root)?;

    let clean_status = MaintenanceStatus::from_last_run(&state.last_clean);
    let doctor_status = MaintenanceStatus::from_last_run(&state.last_doctor);
    let update_status = MaintenanceStatus::from_last_run(&state.last_update);

    Ok((clean_status, doctor_status, update_status))
}

/// Update maintenance timestamp for an operation
pub fn record_maintenance_operation(
    root: &Path,
    operation: &str,
    success: bool,
    details: Option<String>,
) -> io::Result<()> {
    let status = if success { "success" } else { "failed" };
    crate::update_maintenance_state(root, operation, status, details)
}
