pub mod hook_state;
pub mod module_state;
pub mod theme_state;

pub use hook_state::{HookBehavior, HookState};
pub use module_state::ModuleState;
pub use theme_state::{ThemeDescriptor, ThemeState};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Processing mode for module operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingMode {
    Sequential,
    Parallel,
}

impl Default for ProcessingMode {
    fn default() -> Self {
        ProcessingMode::Parallel
    }
}

/// Operation record for tracking execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation_id: String,
    pub executed_at: DateTime<Utc>,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Load state from JSON file
pub fn load_state<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse state: {}", e),
        )
    })
}

/// Save state to JSON file
pub fn save_state<T: Serialize>(path: &Path, state: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(state)?;
    fs::write(path, content)
}

/// Active modules state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveModules {
    pub active: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

impl Default for ActiveModules {
    fn default() -> Self {
        ActiveModules {
            active: Vec::new(),
            last_updated: None,
        }
    }
}

/// Maintenance operation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceState {
    pub last_clean: Option<DateTime<Utc>>,
    pub last_doctor: Option<DateTime<Utc>>,
    pub last_update: Option<DateTime<Utc>>,
    pub operations: Vec<MaintenanceRecord>,
}

impl Default for MaintenanceState {
    fn default() -> Self {
        MaintenanceState {
            last_clean: None,
            last_doctor: None,
            last_update: None,
            operations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceRecord {
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub details: Option<String>,
}

/// Hook hashes for change detection
pub type HookHashes = HashMap<String, String>;

// =============================================================================
// System Maintenance Summaries
// =============================================================================

/// Summary of doctor health check results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DoctorSummary {
    /// Number of critical issues found
    pub critical_count: usize,
    /// Number of errors found
    pub error_count: usize,
    /// Number of warnings found
    pub warning_count: usize,
    /// Number of informational items
    pub info_count: usize,
    /// Timestamp of last run
    pub last_run: Option<DateTime<Utc>>,
    /// Whether auto-fixes were applied
    pub fixes_applied: bool,
}

impl DoctorSummary {
    /// Check if there are any issues (critical or errors)
    pub fn has_issues(&self) -> bool {
        self.critical_count > 0 || self.error_count > 0
    }

    /// Check if healthy (no critical/errors, optionally no warnings)
    pub fn is_healthy(&self, strict: bool) -> bool {
        if strict {
            !self.has_issues() && self.warning_count == 0
        } else {
            !self.has_issues()
        }
    }
}

/// Summary of cleanup operation results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanSummary {
    /// Total space reclaimed in bytes
    pub space_reclaimed_bytes: u64,
    /// Number of orphan packages removed
    pub packages_removed: usize,
    /// Space reclaimed from package cache
    pub package_cache_freed: u64,
    /// Space reclaimed from journal
    pub journal_freed: u64,
    /// Space reclaimed from user cache
    pub user_cache_freed: u64,
    /// Timestamp of last run
    pub last_run: Option<DateTime<Utc>>,
    /// Whether cleanup was dry-run or applied
    pub applied: bool,
}

impl CleanSummary {
    /// Get human-readable space reclaimed
    pub fn space_reclaimed_human(&self) -> String {
        bytes_to_human(self.space_reclaimed_bytes)
    }
}

/// Summary of update operation results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSummary {
    /// Number of packages upgraded
    pub packages_upgraded: usize,
    /// Whether a reboot is required
    pub reboot_required: bool,
    /// List of .pacnew files that need attention
    pub pacnew_files: Vec<String>,
    /// Number of failed services post-upgrade
    pub failed_services: usize,
    /// Timestamp of last run
    pub last_run: Option<DateTime<Utc>>,
    /// Whether update was dry-run or applied
    pub applied: bool,
    /// Kernel version after update
    pub kernel_version: Option<String>,
}

impl UpdateSummary {
    /// Check if post-update actions are needed
    pub fn needs_attention(&self) -> bool {
        self.reboot_required || !self.pacnew_files.is_empty() || self.failed_services > 0
    }
}

/// Combined system maintenance state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMaintenanceState {
    pub doctor: Option<DoctorSummary>,
    pub clean: Option<CleanSummary>,
    pub update: Option<UpdateSummary>,
}

/// Helper function to convert bytes to human-readable format
fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
