//! Update Service - Safe system updates with risk assessment
//!
//! Provides update checking, risk assessment, and snapshot integration.
//! Includes partial update recovery (FR-5.10) with real-time progress tracking.
//! Includes pre-flight checks (Phase 2.1) for safe update execution.

use crate::services::state::StateManager;
use crate::snapshot::SnapshotManager;
use crate::state::{
    CompletedPackage, OperationStatus, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
};
use crate::{ArchNewsItem, IronResult, PackageError};
use chrono::{Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::LazyLock;

// ==========================================================================
// Pacman Output Parser (FR-5.10)
// ==========================================================================

/// Matches "Packages (N)" or "Pakete (N)" line (multilingual support)
static PACKAGES_COUNT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:Packages?|Pakete?)\s*\((\d+)\)").unwrap());

/// Matches "(X/N) upgrading package..." or "(X/N) reinstalling package..."
static UPGRADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((\d+)/(\d+)\)\s+(upgrading|reinstalling)\s+([^\s.]+)").unwrap()
});

/// Matches "(X/N) installing package..."
static INSTALLING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+)/(\d+)\)\s+installing\s+([^\s.]+)").unwrap());

/// Matches "error:" lines
static ERROR_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^error:\s*(.+)$").unwrap());

/// Events parsed from pacman output
#[derive(Debug, Clone, PartialEq)]
pub enum PacmanEvent {
    /// Total package count detected
    PackageCount(usize),
    /// A package operation has started
    PackageStarted {
        package: String,
        current: usize,
        total: usize,
    },
    /// A package operation has completed
    PackageCompleted { package: String },
    /// An error was encountered
    Error { message: String },
}

/// Parser for real-time pacman output
#[derive(Debug, Default)]
pub struct PacmanOutputParser {
    /// Total packages being updated
    pub total_packages: Option<usize>,
    /// Currently processing package
    pub current_package: Option<String>,
    /// Last started package (for completion detection)
    last_started_package: Option<String>,
}

impl PacmanOutputParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a single line of pacman output
    pub fn parse_line(&mut self, line: &str) -> Option<PacmanEvent> {
        let line = line.trim();

        // Check for error
        if let Some(caps) = ERROR_LINE.captures(line) {
            return Some(PacmanEvent::Error {
                message: caps[1].to_string(),
            });
        }

        // Check for package count
        if let Some(caps) = PACKAGES_COUNT.captures(line)
            && let Ok(count) = caps[1].parse::<usize>()
        {
            self.total_packages = Some(count);
            return Some(PacmanEvent::PackageCount(count));
        }

        // Check for upgrade/reinstall progress
        if let Some(caps) = UPGRADING.captures(line)
            && let (Ok(current), Ok(total)) = (caps[1].parse::<usize>(), caps[2].parse::<usize>())
        {
            let package = caps[4].to_string();

            // If we have a previous package, it completed
            let completed_event = self
                .last_started_package
                .take()
                .map(|p| PacmanEvent::PackageCompleted { package: p });

            self.current_package = Some(package.clone());
            self.last_started_package = Some(package.clone());

            // Return the started event (completed event handled via previous package tracking)
            if completed_event.is_some() {
                // Note: In real usage, we track completions when the next package starts
            }

            return Some(PacmanEvent::PackageStarted {
                package,
                current,
                total,
            });
        }

        // Check for install progress
        if let Some(caps) = INSTALLING.captures(line)
            && let (Ok(current), Ok(total)) = (caps[1].parse::<usize>(), caps[2].parse::<usize>())
        {
            let package = caps[3].to_string();

            self.current_package = Some(package.clone());
            self.last_started_package = Some(package.clone());

            return Some(PacmanEvent::PackageStarted {
                package,
                current,
                total,
            });
        }

        None
    }

    /// Mark the last package as completed (call at end of successful update)
    pub fn finalize(&mut self) -> Option<PacmanEvent> {
        self.last_started_package
            .take()
            .map(|p| PacmanEvent::PackageCompleted { package: p })
    }
}

/// Information about an interrupted update
#[derive(Debug, Clone)]
pub struct InterruptedUpdate {
    /// The progress state
    pub progress: UpdateProgress,
    /// Number of completed packages
    pub completed_count: usize,
    /// Number of remaining packages
    pub remaining_count: usize,
    /// Time since update started
    pub elapsed: Duration,
}

/// Update risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UpdateRisk {
    /// No special concerns
    Low,
    /// Some packages need attention
    Medium,
    /// Critical packages being updated (kernel, nvidia, systemd)
    High,
    /// Manual intervention may be required
    Critical,
}

/// Package update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUpdate {
    /// Package name
    pub name: String,
    /// Current version
    pub current_version: String,
    /// New version
    pub new_version: String,
    /// Risk level for this package
    pub risk: UpdateRisk,
    /// Reason for risk level
    pub risk_reason: Option<String>,
}

/// Update plan with risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlan {
    /// Packages to update
    pub packages: Vec<PackageUpdate>,
    /// Overall risk level
    pub overall_risk: UpdateRisk,
    /// Whether snapshot is recommended
    pub snapshot_recommended: bool,
    /// Any news items requiring attention
    pub news_items: Vec<String>,
    /// Timestamp of plan creation
    pub created_at: chrono::DateTime<Utc>,
}

// ==========================================================================
// Pre-flight Checks (Phase 2.1)
// ==========================================================================

/// Pre-flight check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreflightStatus {
    /// Check passed
    Pass,
    /// Check passed with warning
    Warning,
    /// Check failed (blocking)
    Fail,
    /// Check skipped or not applicable
    Skipped,
}

/// Individual pre-flight check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: PreflightStatus,
    /// Human-readable message
    pub message: String,
    /// Additional details
    pub details: Option<String>,
}

/// Unacknowledged news item requiring attention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnacknowledgedNews {
    /// News title
    pub title: String,
    /// News URL (used as identifier)
    pub url: String,
    /// News date
    pub date: String,
    /// Brief description
    pub description: String,
    /// Whether this news requires manual intervention before upgrading
    pub requires_manual: bool,
}

/// Result of all pre-flight checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    /// Network connectivity check
    pub network_ok: bool,
    /// Disk space check (minimum 2GB free)
    pub disk_space_ok: bool,
    /// Available disk space in bytes
    pub disk_space_available: u64,
    /// Battery check (>20% or AC power)
    pub battery_ok: bool,
    /// Battery percentage (None if on AC or no battery)
    pub battery_percent: Option<u8>,
    /// Whether on AC power
    pub on_ac_power: bool,
    /// Pacman lock file check
    pub pacman_lock_free: bool,
    /// Time synchronization check
    pub time_synced: bool,
    /// Unacknowledged Arch news items (Phase 2.2)
    pub unacknowledged_news: Vec<UnacknowledgedNews>,
    /// Whether there's critical news requiring acknowledgment before update
    pub news_blocks_update: bool,
    /// Individual check results
    pub checks: Vec<PreflightCheck>,
    /// Warning messages (non-blocking)
    pub warnings: Vec<String>,
    /// Blocker messages (update cannot proceed)
    pub blockers: Vec<String>,
}

impl PreflightResult {
    /// Create a new pre-flight result with defaults
    pub fn new() -> Self {
        Self {
            network_ok: false,
            disk_space_ok: false,
            disk_space_available: 0,
            battery_ok: true,
            battery_percent: None,
            on_ac_power: true,
            pacman_lock_free: true,
            time_synced: true,
            unacknowledged_news: Vec::new(),
            news_blocks_update: false,
            checks: Vec::new(),
            warnings: Vec::new(),
            blockers: Vec::new(),
        }
    }

    /// Check if all pre-flight checks passed including news acknowledgment
    pub fn can_proceed_with_news(&self) -> bool {
        self.blockers.is_empty() && !self.news_blocks_update
    }

    /// Get count of unacknowledged news requiring manual intervention
    pub fn critical_news_count(&self) -> usize {
        self.unacknowledged_news
            .iter()
            .filter(|n| n.requires_manual)
            .count()
    }

    /// Check if all pre-flight checks passed (no blockers)
    pub fn can_proceed(&self) -> bool {
        self.blockers.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Add a check result
    pub fn add_check(&mut self, check: PreflightCheck) {
        match check.status {
            PreflightStatus::Fail => {
                self.blockers.push(check.message.clone());
            }
            PreflightStatus::Warning => {
                self.warnings.push(check.message.clone());
            }
            _ => {}
        }
        self.checks.push(check);
    }
}

impl Default for PreflightResult {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================================================
// Post-Update Detection (Phase 2.4)
// ==========================================================================

/// Configuration file conflict type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigConflictType {
    /// .pacnew file - new default config from package
    Pacnew,
    /// .pacsave file - user config saved when package removed/replaced
    Pacsave,
}

/// A configuration file conflict detected after update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigConflict {
    /// Original config file path
    pub original: String,
    /// Path to the conflicting file (.pacnew or .pacsave)
    pub conflict_file: String,
    /// Type of conflict
    pub conflict_type: ConfigConflictType,
    /// Package that owns the original file (if known)
    pub package: Option<String>,
}

/// A failed systemd service detected after update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedService {
    /// Service unit name
    pub name: String,
    /// Service load state
    pub load_state: String,
    /// Service active state
    pub active_state: String,
    /// Brief description
    pub description: String,
}

/// Result of post-update checks (Phase 2.4)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PostUpdateResult {
    /// Configuration file conflicts (.pacnew/.pacsave)
    pub config_conflicts: Vec<ConfigConflict>,
    /// Whether a reboot is required (kernel/glibc/systemd updated)
    pub reboot_required: bool,
    /// Packages that require reboot
    pub reboot_packages: Vec<String>,
    /// Failed systemd services
    pub failed_services: Vec<FailedService>,
    /// Whether there are issues requiring attention
    pub has_issues: bool,
}

impl PostUpdateResult {
    /// Create a new empty post-update result
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are configuration conflicts to resolve
    pub fn has_config_conflicts(&self) -> bool {
        !self.config_conflicts.is_empty()
    }

    /// Get count of .pacnew files
    pub fn pacnew_count(&self) -> usize {
        self.config_conflicts
            .iter()
            .filter(|c| c.conflict_type == ConfigConflictType::Pacnew)
            .count()
    }

    /// Get count of .pacsave files
    pub fn pacsave_count(&self) -> usize {
        self.config_conflicts
            .iter()
            .filter(|c| c.conflict_type == ConfigConflictType::Pacsave)
            .count()
    }

    /// Check if there are failed services
    pub fn has_failed_services(&self) -> bool {
        !self.failed_services.is_empty()
    }

    /// Update the has_issues flag based on current state
    pub fn update_has_issues(&mut self) {
        self.has_issues = !self.config_conflicts.is_empty()
            || self.reboot_required
            || !self.failed_services.is_empty();
    }
}

/// Update service trait
pub trait UpdateService {
    /// Check for available updates
    fn check(&self) -> IronResult<UpdatePlan>;

    /// Run pre-flight checks before update (Phase 2.1)
    fn run_preflight_checks(&self) -> PreflightResult;

    /// Run pre-flight checks including Arch news acknowledgment (Phase 2.2)
    ///
    /// Takes pre-fetched news items and checks which ones are unacknowledged.
    /// Critical news (requiring manual intervention) will block the update.
    fn run_preflight_checks_with_news(&self, news_items: &[ArchNewsItem]) -> PreflightResult;

    /// Apply updates (optionally with snapshot)
    fn apply(&self, create_snapshot: bool) -> IronResult<()>;

    /// Apply updates with real-time progress tracking (FR-5.10)
    fn apply_with_progress(
        &self,
        plan: &UpdatePlan,
        create_snapshot: bool,
        on_progress: Option<&dyn Fn(&UpdateProgress)>,
    ) -> IronResult<()>;

    /// Apply specific packages only
    fn apply_packages(&self, packages: &[String], create_snapshot: bool) -> IronResult<()>;

    /// Get last update time
    fn last_update(&self) -> Option<chrono::DateTime<Utc>>;

    /// Clean package cache
    fn clean_cache(&self, keep_versions: usize) -> IronResult<u64>;

    // ==========================================================================
    // Partial Update Recovery (FR-5.10)
    // ==========================================================================

    /// Check if there's an interrupted update
    fn check_interrupted(&self) -> Option<InterruptedUpdate>;

    /// Resume an interrupted update
    fn resume(&self) -> IronResult<()>;

    /// Get current update progress
    fn get_progress(&self) -> Option<UpdateProgress>;

    /// Clear update progress (marks update as complete or abandons it)
    fn clear_progress(&self) -> IronResult<()>;

    // ==========================================================================
    // Post-Update Detection (Phase 2.4)
    // ==========================================================================

    /// Run post-update checks to detect issues after update completion
    ///
    /// Detects:
    /// - `.pacnew` and `.pacsave` configuration file conflicts
    /// - Packages that require a system reboot (kernel, glibc, systemd)
    /// - Failed systemd services
    fn run_post_update_checks(&self, updated_packages: &[String]) -> PostUpdateResult;

    /// Find .pacnew and .pacsave files in /etc
    fn find_config_conflicts(&self) -> Vec<ConfigConflict>;

    /// Check if updated packages require a reboot
    fn check_reboot_required(&self, packages: &[String]) -> (bool, Vec<String>);

    /// List failed systemd services
    fn find_failed_services(&self) -> Vec<FailedService>;
}

/// Default update service implementation
pub struct DefaultUpdateService<S: SnapshotManager> {
    /// State manager
    state_manager: StateManager,
    /// Snapshot manager
    snapshot_manager: S,
}

impl<S: SnapshotManager> DefaultUpdateService<S> {
    /// Create a new update service
    pub fn new(state_manager: StateManager, snapshot_manager: S) -> Self {
        Self {
            state_manager,
            snapshot_manager,
        }
    }

    /// Parse checkupdates output
    fn parse_updates(&self, output: &str) -> Vec<PackageUpdate> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0].to_string();
                    let current = parts[1].to_string();
                    let new = parts[3].to_string();
                    let (risk, reason) = self.assess_package_risk(&name, &current, &new);
                    Some(PackageUpdate {
                        name,
                        current_version: current,
                        new_version: new,
                        risk,
                        risk_reason: reason,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Assess risk level for a single package
    fn assess_package_risk(
        &self,
        name: &str,
        _current: &str,
        _new: &str,
    ) -> (UpdateRisk, Option<String>) {
        // Critical packages
        if name.starts_with("linux") || name == "linux" || name.starts_with("linux-") {
            return (
                UpdateRisk::Critical,
                Some("Kernel update requires reboot".to_string()),
            );
        }

        if name.starts_with("nvidia") || name.contains("nvidia-") {
            return (
                UpdateRisk::High,
                Some("NVIDIA driver update may require reboot".to_string()),
            );
        }

        if name == "systemd" || name.starts_with("systemd-") {
            return (
                UpdateRisk::High,
                Some("Systemd update is system-critical".to_string()),
            );
        }

        if name == "glibc" || name == "gcc-libs" {
            return (UpdateRisk::High, Some("Core library update".to_string()));
        }

        // Medium risk packages
        if name.starts_with("mesa") || name.starts_with("vulkan") {
            return (
                UpdateRisk::Medium,
                Some("Graphics driver update".to_string()),
            );
        }

        if name.starts_with("pipewire") || name.starts_with("wireplumber") {
            return (UpdateRisk::Medium, Some("Audio system update".to_string()));
        }

        (UpdateRisk::Low, None)
    }

    /// Calculate overall risk from package list
    fn calculate_overall_risk(&self, packages: &[PackageUpdate]) -> UpdateRisk {
        packages
            .iter()
            .map(|p| p.risk)
            .max()
            .unwrap_or(UpdateRisk::Low)
    }

    /// Run pacman command
    fn run_pacman(&self, args: &[&str]) -> IronResult<String> {
        let output =
            Command::new("pacman")
                .args(args)
                .output()
                .map_err(|_| PackageError::PacmanFailed {
                    message: "Failed to execute pacman".to_string(),
                })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(PackageError::PacmanFailed {
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            }
            .into())
        }
    }

    /// Check for updates using checkupdates
    fn run_checkupdates(&self) -> IronResult<String> {
        let output = Command::new("checkupdates").output();

        match output {
            Ok(o) if o.status.success() || o.status.code() == Some(2) => {
                // Exit code 2 means no updates available
                Ok(String::from_utf8_lossy(&o.stdout).to_string())
            }
            Ok(o) => Err(PackageError::PacmanFailed {
                message: String::from_utf8_lossy(&o.stderr).to_string(),
            }
            .into()),
            Err(_) => {
                // Fallback: use pacman -Qu
                self.run_pacman(&["-Qu"])
            }
        }
    }

    // ==========================================================================
    // Pre-flight Check Helpers (Phase 2.1)
    // ==========================================================================

    /// Check network connectivity
    fn check_network(&self) -> PreflightCheck {
        // Try to reach archlinux.org mirrors
        let result = Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--connect-timeout",
                "5",
                "https://archlinux.org",
            ])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let code = String::from_utf8_lossy(&output.stdout);
                if code.starts_with('2') || code.starts_with('3') {
                    PreflightCheck {
                        name: "Network".to_string(),
                        status: PreflightStatus::Pass,
                        message: "Network connectivity OK".to_string(),
                        details: None,
                    }
                } else {
                    PreflightCheck {
                        name: "Network".to_string(),
                        status: PreflightStatus::Fail,
                        message: format!("Network check failed (HTTP {})", code.trim()),
                        details: Some("Cannot reach archlinux.org".to_string()),
                    }
                }
            }
            _ => PreflightCheck {
                name: "Network".to_string(),
                status: PreflightStatus::Fail,
                message: "No network connectivity".to_string(),
                details: Some("Cannot reach archlinux.org".to_string()),
            },
        }
    }

    /// Check available disk space
    fn check_disk_space(&self) -> (PreflightCheck, u64) {
        const MIN_SPACE_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2GB
        const WARN_SPACE_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5GB

        // Check /var partition (where packages are cached/extracted)
        let result = Command::new("df")
            .args(["--output=avail", "-B1", "/var"])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let available: u64 = stdout
                    .lines()
                    .nth(1) // Skip header
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                let gb_available = available / (1024 * 1024 * 1024);

                if available >= WARN_SPACE_BYTES {
                    (
                        PreflightCheck {
                            name: "Disk Space".to_string(),
                            status: PreflightStatus::Pass,
                            message: format!("{}GB available on /var", gb_available),
                            details: None,
                        },
                        available,
                    )
                } else if available >= MIN_SPACE_BYTES {
                    (
                        PreflightCheck {
                            name: "Disk Space".to_string(),
                            status: PreflightStatus::Warning,
                            message: format!(
                                "Low disk space: {}GB available (recommended: 5GB+)",
                                gb_available
                            ),
                            details: Some("Consider cleaning package cache".to_string()),
                        },
                        available,
                    )
                } else {
                    (
                        PreflightCheck {
                            name: "Disk Space".to_string(),
                            status: PreflightStatus::Fail,
                            message: format!(
                                "Insufficient disk space: {}GB (minimum: 2GB)",
                                gb_available
                            ),
                            details: Some("Free up space before updating".to_string()),
                        },
                        available,
                    )
                }
            }
            _ => (
                PreflightCheck {
                    name: "Disk Space".to_string(),
                    status: PreflightStatus::Warning,
                    message: "Could not determine disk space".to_string(),
                    details: None,
                },
                0,
            ),
        }
    }

    /// Check battery status
    fn check_battery(&self) -> (PreflightCheck, Option<u8>, bool) {
        const MIN_BATTERY: u8 = 20;

        // Check for battery
        let battery_path = Path::new("/sys/class/power_supply/BAT0/capacity");
        let ac_path = Path::new("/sys/class/power_supply/AC/online");

        // Check AC power status
        let on_ac = fs::read_to_string(ac_path)
            .map(|s| s.trim() == "1")
            .unwrap_or(true); // Assume AC if can't read

        // Check battery capacity
        let battery_percent: Option<u8> = fs::read_to_string(battery_path)
            .ok()
            .and_then(|s| s.trim().parse().ok());

        match (battery_percent, on_ac) {
            (_, true) => (
                PreflightCheck {
                    name: "Power".to_string(),
                    status: PreflightStatus::Pass,
                    message: "On AC power".to_string(),
                    details: None,
                },
                battery_percent,
                true,
            ),
            (Some(pct), false) if pct >= MIN_BATTERY => (
                PreflightCheck {
                    name: "Power".to_string(),
                    status: PreflightStatus::Warning,
                    message: format!("On battery ({}%)", pct),
                    details: Some("Consider connecting to AC power".to_string()),
                },
                Some(pct),
                false,
            ),
            (Some(pct), false) => (
                PreflightCheck {
                    name: "Power".to_string(),
                    status: PreflightStatus::Fail,
                    message: format!("Battery too low ({}%)", pct),
                    details: Some(format!("Minimum {}% required for update", MIN_BATTERY)),
                },
                Some(pct),
                false,
            ),
            (None, false) => (
                PreflightCheck {
                    name: "Power".to_string(),
                    status: PreflightStatus::Pass,
                    message: "No battery detected (desktop)".to_string(),
                    details: None,
                },
                None,
                false,
            ),
        }
    }

    /// Check if pacman lock file exists
    fn check_pacman_lock(&self) -> PreflightCheck {
        let lock_path = Path::new("/var/lib/pacman/db.lck");

        if lock_path.exists() {
            PreflightCheck {
                name: "Pacman Lock".to_string(),
                status: PreflightStatus::Fail,
                message: "Pacman database is locked".to_string(),
                details: Some("Another package manager may be running, or remove stale lock with: sudo rm /var/lib/pacman/db.lck".to_string()),
            }
        } else {
            PreflightCheck {
                name: "Pacman Lock".to_string(),
                status: PreflightStatus::Pass,
                message: "Pacman database is available".to_string(),
                details: None,
            }
        }
    }

    /// Check time synchronization
    fn check_time_sync(&self) -> PreflightCheck {
        let result = Command::new("timedatectl")
            .args(["show", "--property=NTPSynchronized", "--value"])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let synced = String::from_utf8_lossy(&output.stdout).trim() == "yes";
                if synced {
                    PreflightCheck {
                        name: "Time Sync".to_string(),
                        status: PreflightStatus::Pass,
                        message: "System time is synchronized".to_string(),
                        details: None,
                    }
                } else {
                    PreflightCheck {
                        name: "Time Sync".to_string(),
                        status: PreflightStatus::Warning,
                        message: "System time may not be synchronized".to_string(),
                        details: Some("Run: sudo timedatectl set-ntp true".to_string()),
                    }
                }
            }
            _ => PreflightCheck {
                name: "Time Sync".to_string(),
                status: PreflightStatus::Skipped,
                message: "Could not check time synchronization".to_string(),
                details: None,
            },
        }
    }

    /// Check Arch news for unacknowledged items (Phase 2.2)
    ///
    /// Returns the check result, list of unacknowledged news, and whether update is blocked
    fn check_news(
        &self,
        news_items: &[ArchNewsItem],
    ) -> (PreflightCheck, Vec<UnacknowledgedNews>, bool) {
        if news_items.is_empty() {
            return (
                PreflightCheck {
                    name: "Arch News".to_string(),
                    status: PreflightStatus::Pass,
                    message: "No recent Arch news".to_string(),
                    details: None,
                },
                Vec::new(),
                false,
            );
        }

        // Filter to unacknowledged news using state manager
        let unacknowledged: Vec<UnacknowledgedNews> = news_items
            .iter()
            .filter(|item| !self.state_manager.is_news_acknowledged(&item.url))
            .map(|item| UnacknowledgedNews {
                title: item.title.clone(),
                url: item.url.clone(),
                date: item.date.clone(),
                description: if item.description.len() > 200 {
                    format!("{}...", &item.description[..200])
                } else {
                    item.description.clone()
                },
                requires_manual: item.requires_manual,
            })
            .collect();

        if unacknowledged.is_empty() {
            return (
                PreflightCheck {
                    name: "Arch News".to_string(),
                    status: PreflightStatus::Pass,
                    message: "All Arch news acknowledged".to_string(),
                    details: None,
                },
                Vec::new(),
                false,
            );
        }

        // Check for critical news (requiring manual intervention)
        let critical_count = unacknowledged.iter().filter(|n| n.requires_manual).count();
        let total_count = unacknowledged.len();

        if critical_count > 0 {
            (
                PreflightCheck {
                    name: "Arch News".to_string(),
                    status: PreflightStatus::Fail,
                    message: format!(
                        "{} unacknowledged news item(s) requiring manual intervention",
                        critical_count
                    ),
                    details: Some(
                        "Review news and acknowledge before updating. \
                        Critical news may require manual steps before or after the update."
                            .to_string(),
                    ),
                },
                unacknowledged,
                true, // Block update
            )
        } else {
            (
                PreflightCheck {
                    name: "Arch News".to_string(),
                    status: PreflightStatus::Warning,
                    message: format!("{} unacknowledged news item(s)", total_count),
                    details: Some("Consider reviewing before updating".to_string()),
                },
                unacknowledged,
                false, // Don't block, just warn
            )
        }
    }

    // ==========================================================================
    // Progress Tracking Helpers (FR-5.10)
    // ==========================================================================

    /// Convert an UpdatePlan to a SavedUpdatePlan for persistence
    fn to_saved_plan(plan: &UpdatePlan) -> SavedUpdatePlan {
        SavedUpdatePlan {
            packages: plan
                .packages
                .iter()
                .map(|p| SavedPackage {
                    name: p.name.clone(),
                    current_version: p.current_version.clone(),
                    new_version: p.new_version.clone(),
                })
                .collect(),
            snapshot_recommended: plan.snapshot_recommended,
            created_at: plan.created_at,
        }
    }

    /// Persist progress state atomically
    fn persist_progress(&self, progress: &UpdateProgress) -> IronResult<()> {
        self.state_manager
            .set_update_progress(Some(progress.clone()))
    }

    /// Run pacman with streaming output and progress callback
    fn run_pacman_with_progress(
        &self,
        args: &[&str],
        progress: &mut UpdateProgress,
        on_progress: Option<&dyn Fn(&UpdateProgress)>,
    ) -> IronResult<()> {
        let mut child = Command::new("sudo")
            .arg("pacman")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PackageError::PacmanFailed {
                message: format!("Failed to spawn pacman: {}", e),
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PackageError::PacmanFailed {
                message: "Failed to capture stdout".to_string(),
            })?;

        let mut parser = PacmanOutputParser::new();
        let reader = BufReader::new(stdout);

        // Track package versions for completion records
        let package_versions: std::collections::HashMap<String, (String, String)> = progress
            .plan
            .packages
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    (p.current_version.clone(), p.new_version.clone()),
                )
            })
            .collect();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if let Some(event) = parser.parse_line(&line) {
                match event {
                    PacmanEvent::PackageStarted { package, .. } => {
                        progress.phase = UpdatePhase::Installing;

                        // Check if we're transitioning from a previous package
                        if let Some(prev_pkg) = &progress
                            .plan
                            .packages
                            .iter()
                            .find(|p| {
                                progress.completed_packages.iter().all(|c| c.name != p.name)
                                    && p.name != package
                            })
                            .map(|p| p.name.clone())
                        {
                            // The previous package in the stream completed
                            if let Some((old_ver, new_ver)) = package_versions.get(prev_pkg) {
                                let completed = CompletedPackage {
                                    name: prev_pkg.clone(),
                                    old_version: old_ver.clone(),
                                    new_version: new_ver.clone(),
                                    completed_at: Utc::now(),
                                };
                                progress.mark_completed(completed);
                            }
                        }

                        // Persist and notify
                        self.persist_progress(progress)?;
                        if let Some(cb) = on_progress {
                            cb(progress);
                        }
                    }
                    PacmanEvent::Error { message } => {
                        progress.phase = UpdatePhase::Failed;
                        progress.last_error = Some(message);
                        self.persist_progress(progress)?;
                    }
                    _ => {}
                }
            }
        }

        // Wait for process to complete
        let status = child.wait().map_err(|e| PackageError::PacmanFailed {
            message: format!("Failed to wait for pacman: {}", e),
        })?;

        if status.success() {
            // Collect remaining packages to mark completed (to avoid borrow conflict)
            let remaining_pkgs: Vec<_> = progress
                .plan
                .packages
                .iter()
                .filter(|pkg| {
                    progress
                        .completed_packages
                        .iter()
                        .all(|c| c.name != pkg.name)
                })
                .map(|pkg| pkg.name.clone())
                .collect();

            // Mark all remaining packages as completed
            for pkg_name in remaining_pkgs {
                if let Some((old_ver, new_ver)) = package_versions.get(&pkg_name) {
                    let completed = CompletedPackage {
                        name: pkg_name,
                        old_version: old_ver.clone(),
                        new_version: new_ver.clone(),
                        completed_at: Utc::now(),
                    };
                    progress.mark_completed(completed);
                }
            }
            progress.phase = UpdatePhase::Completed;
            self.persist_progress(progress)?;
            Ok(())
        } else {
            // Check if we were interrupted
            if progress.phase != UpdatePhase::Failed {
                progress.phase = UpdatePhase::Interrupted;
            }
            self.persist_progress(progress)?;
            Err(PackageError::PacmanFailed {
                message: "Pacman exited with non-zero status".to_string(),
            }
            .into())
        }
    }
}

impl<S: SnapshotManager> UpdateService for DefaultUpdateService<S> {
    fn check(&self) -> IronResult<UpdatePlan> {
        let output = self.run_checkupdates()?;
        let packages = self.parse_updates(&output);
        let overall_risk = self.calculate_overall_risk(&packages);

        // Recommend snapshot for high/critical risk
        let snapshot_recommended = overall_risk >= UpdateRisk::High;

        Ok(UpdatePlan {
            packages,
            overall_risk,
            snapshot_recommended,
            news_items: vec![], // TODO: Integrate with Arch News parser
            created_at: Utc::now(),
        })
    }

    fn run_preflight_checks(&self) -> PreflightResult {
        let mut result = PreflightResult::new();

        // Run all pre-flight checks
        let network_check = self.check_network();
        result.network_ok = network_check.status == PreflightStatus::Pass;
        result.add_check(network_check);

        let (disk_check, disk_space) = self.check_disk_space();
        result.disk_space_ok = disk_check.status == PreflightStatus::Pass
            || disk_check.status == PreflightStatus::Warning;
        result.disk_space_available = disk_space;
        result.add_check(disk_check);

        let (battery_check, battery_pct, on_ac) = self.check_battery();
        result.battery_ok = battery_check.status == PreflightStatus::Pass
            || battery_check.status == PreflightStatus::Warning;
        result.battery_percent = battery_pct;
        result.on_ac_power = on_ac;
        result.add_check(battery_check);

        let pacman_check = self.check_pacman_lock();
        result.pacman_lock_free = pacman_check.status == PreflightStatus::Pass;
        result.add_check(pacman_check);

        let time_check = self.check_time_sync();
        result.time_synced = time_check.status == PreflightStatus::Pass;
        result.add_check(time_check);

        result
    }

    fn run_preflight_checks_with_news(&self, news_items: &[ArchNewsItem]) -> PreflightResult {
        // Run all standard pre-flight checks first
        let mut result = self.run_preflight_checks();

        // Add news check (Phase 2.2)
        let (news_check, unacknowledged, blocks_update) = self.check_news(news_items);
        result.add_check(news_check);
        result.unacknowledged_news = unacknowledged;
        result.news_blocks_update = blocks_update;

        // Mark news as fetched in state
        let _ = self.state_manager.mark_news_fetched();

        result
    }

    fn apply(&self, create_snapshot: bool) -> IronResult<()> {
        // Create pre-update snapshot if requested
        if create_snapshot {
            self.snapshot_manager.create("pre-update")?;
        }

        // Run system update
        let result = Command::new("sudo")
            .args(["pacman", "-Syu", "--noconfirm"])
            .status();

        match result {
            Ok(status) if status.success() => {
                self.state_manager.update_maintenance("update")?;
                self.state_manager.record_operation(
                    "system_update",
                    OperationStatus::Success,
                    None,
                )?;
                Ok(())
            }
            Ok(_) => {
                self.state_manager.record_operation(
                    "system_update",
                    OperationStatus::Failed,
                    Some("Update failed".to_string()),
                )?;
                Err(PackageError::PacmanFailed {
                    message: "System update failed".to_string(),
                }
                .into())
            }
            Err(e) => Err(PackageError::PacmanFailed {
                message: format!("Failed to run pacman: {}", e),
            }
            .into()),
        }
    }

    fn apply_packages(&self, packages: &[String], create_snapshot: bool) -> IronResult<()> {
        if packages.is_empty() {
            return Ok(());
        }

        // Create pre-update snapshot if requested
        if create_snapshot {
            self.snapshot_manager.create("pre-update")?;
        }

        // Build pacman command
        let mut args = vec!["-S", "--noconfirm"];
        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        args.extend(pkg_refs);

        let result = Command::new("sudo").arg("pacman").args(&args).status();

        match result {
            Ok(status) if status.success() => {
                self.state_manager.record_operation(
                    "package_update",
                    OperationStatus::Success,
                    Some(packages.join(", ")),
                )?;
                Ok(())
            }
            Ok(_) => Err(PackageError::PacmanFailed {
                message: "Package update failed".to_string(),
            }
            .into()),
            Err(e) => Err(PackageError::PacmanFailed {
                message: format!("Failed to run pacman: {}", e),
            }
            .into()),
        }
    }

    fn last_update(&self) -> Option<chrono::DateTime<Utc>> {
        self.state_manager.maintenance().last_update
    }

    fn clean_cache(&self, keep_versions: usize) -> IronResult<u64> {
        // Use paccache to clean old package versions
        let result = Command::new("sudo")
            .args(["paccache", "-r", "-k", &keep_versions.to_string()])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                // Parse output to get freed space (rough estimate)
                let stdout = String::from_utf8_lossy(&output.stdout);
                let freed =
                    stdout.lines().filter(|l| l.contains("removed")).count() as u64 * 50_000_000; // Rough estimate: 50MB per package

                self.state_manager.update_maintenance("clean")?;
                self.state_manager.record_operation(
                    "cache_clean",
                    OperationStatus::Success,
                    Some(format!("kept {}", keep_versions)),
                )?;

                Ok(freed)
            }
            Ok(_) => {
                // paccache not available, use pacman
                Command::new("sudo")
                    .args(["pacman", "-Sc", "--noconfirm"])
                    .status()
                    .ok();
                self.state_manager.update_maintenance("clean")?;
                Ok(0)
            }
            Err(_) => Ok(0),
        }
    }

    // ==========================================================================
    // Partial Update Recovery (FR-5.10)
    // ==========================================================================

    fn apply_with_progress(
        &self,
        plan: &UpdatePlan,
        create_snapshot: bool,
        on_progress: Option<&dyn Fn(&UpdateProgress)>,
    ) -> IronResult<()> {
        // Create snapshot if requested
        let snapshot_id = if create_snapshot {
            let info = self.snapshot_manager.create("pre-update")?;
            Some(info.id)
        } else {
            None
        };

        // Initialize progress tracking
        let saved_plan = Self::to_saved_plan(plan);
        let mut progress = UpdateProgress::new(saved_plan, snapshot_id);
        progress.phase = UpdatePhase::Preparing;
        self.persist_progress(&progress)?;

        if let Some(cb) = on_progress {
            cb(&progress);
        }

        // Run update with progress tracking
        let args = ["-Syu", "--noconfirm"];
        let result = self.run_pacman_with_progress(&args, &mut progress, on_progress);

        match &result {
            Ok(()) => {
                self.state_manager.update_maintenance("update")?;
                self.state_manager.record_operation(
                    "system_update",
                    OperationStatus::Success,
                    Some(format!("{} packages updated", plan.packages.len())),
                )?;
                // Clear progress on success
                self.clear_progress()?;
            }
            Err(_) => {
                let status = if progress.phase == UpdatePhase::Interrupted {
                    OperationStatus::Partial
                } else {
                    OperationStatus::Failed
                };
                self.state_manager.record_operation(
                    "system_update",
                    status,
                    progress.last_error.clone(),
                )?;
            }
        }

        result
    }

    fn check_interrupted(&self) -> Option<InterruptedUpdate> {
        let progress = self.state_manager.get_update_progress()?;

        // Only return if actually interrupted
        if !progress.is_incomplete() {
            return None;
        }

        let completed_count = progress.completed_packages.len();
        let remaining_count = progress.total_packages.saturating_sub(completed_count);
        let elapsed = Utc::now().signed_duration_since(progress.started_at);

        Some(InterruptedUpdate {
            progress,
            completed_count,
            remaining_count,
            elapsed,
        })
    }

    fn resume(&self) -> IronResult<()> {
        let progress = self
            .get_progress()
            .ok_or(crate::StateError::NoActiveUpdate)?;

        // Get remaining packages
        let remaining: Vec<String> = progress
            .remaining_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect();

        if remaining.is_empty() {
            // All packages completed, just clear progress
            self.clear_progress()?;
            return Ok(());
        }

        // Resume by installing remaining packages
        // Note: We don't create a new snapshot for resume
        let result = self.apply_packages(&remaining, false);

        match &result {
            Ok(()) => {
                self.state_manager.record_operation(
                    "update_resume",
                    OperationStatus::Success,
                    Some(format!("{} packages completed", remaining.len())),
                )?;
                self.clear_progress()?;
            }
            Err(_) => {
                self.state_manager.record_operation(
                    "update_resume",
                    OperationStatus::Failed,
                    Some(format!("Failed to resume {} packages", remaining.len())),
                )?;
            }
        }

        result
    }

    fn get_progress(&self) -> Option<UpdateProgress> {
        self.state_manager.get_update_progress()
    }

    fn clear_progress(&self) -> IronResult<()> {
        self.state_manager.set_update_progress(None)
    }

    // ==========================================================================
    // Post-Update Detection Implementation (Phase 2.4)
    // ==========================================================================

    fn run_post_update_checks(&self, updated_packages: &[String]) -> PostUpdateResult {
        let mut result = PostUpdateResult::new();

        // Find config conflicts (.pacnew/.pacsave files)
        result.config_conflicts = self.find_config_conflicts();

        // Check if any updated packages require reboot
        let (reboot_required, reboot_packages) = self.check_reboot_required(updated_packages);
        result.reboot_required = reboot_required;
        result.reboot_packages = reboot_packages;

        // Find failed systemd services
        result.failed_services = self.find_failed_services();

        // Update the has_issues flag
        result.update_has_issues();

        result
    }

    fn find_config_conflicts(&self) -> Vec<ConfigConflict> {
        let mut conflicts = Vec::new();

        // Search for .pacnew files in /etc
        if let Ok(output) = Command::new("find")
            .args(["/etc", "-name", "*.pacnew", "-type", "f"])
            .output()
            && output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        let original = line.trim_end_matches(".pacnew").to_string();
                        conflicts.push(ConfigConflict {
                            original: original.clone(),
                            conflict_file: line.to_string(),
                            conflict_type: ConfigConflictType::Pacnew,
                            package: Self::find_package_owner(&original),
                        });
                    }
                }
            }

        // Search for .pacsave files in /etc
        if let Ok(output) = Command::new("find")
            .args(["/etc", "-name", "*.pacsave", "-type", "f"])
            .output()
            && output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        let original = line.trim_end_matches(".pacsave").to_string();
                        conflicts.push(ConfigConflict {
                            original: original.clone(),
                            conflict_file: line.to_string(),
                            conflict_type: ConfigConflictType::Pacsave,
                            package: Self::find_package_owner(&original),
                        });
                    }
                }
            }

        conflicts
    }

    fn check_reboot_required(&self, packages: &[String]) -> (bool, Vec<String>) {
        let reboot_packages: Vec<String> = packages
            .iter()
            .filter(|p| {
                let name = p.to_lowercase();
                // Kernel packages
                name.starts_with("linux")
                    // Core system libraries
                    || name == "glibc"
                    || name == "gcc-libs"
                    // Systemd and related
                    || name == "systemd"
                    || name.starts_with("systemd-")
                    // Graphics drivers
                    || name.starts_with("nvidia")
                    || name.starts_with("mesa")
                    // DBus (affects many services)
                    || name == "dbus"
            })
            .cloned()
            .collect();

        (!reboot_packages.is_empty(), reboot_packages)
    }

    fn find_failed_services(&self) -> Vec<FailedService> {
        let mut failed = Vec::new();

        // Run systemctl --failed to find failed services
        if let Ok(output) = Command::new("systemctl")
            .args(["--failed", "--no-legend", "--no-pager"])
            .output()
            && output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    // Format: UNIT LOAD ACTIVE SUB DESCRIPTION...
                    if parts.len() >= 4 {
                        failed.push(FailedService {
                            name: parts[0].to_string(),
                            load_state: parts[1].to_string(),
                            active_state: parts[2].to_string(),
                            description: parts[4..].join(" "),
                        });
                    }
                }
            }

        failed
    }
}

impl<S: SnapshotManager> DefaultUpdateService<S> {
    /// Find which package owns a file using pacman -Qo
    fn find_package_owner(file_path: &str) -> Option<String> {
        if let Ok(output) = Command::new("pacman").args(["-Qo", file_path]).output()
            && output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Output format: "/path/to/file is owned by package version"
                if let Some(line) = stdout.lines().next() {
                    let parts: Vec<&str> = line.split(" is owned by ").collect();
                    if parts.len() >= 2 {
                        // Get package name (without version)
                        if let Some(pkg) = parts[1].split_whitespace().next() {
                            return Some(pkg.to_string());
                        }
                    }
                }
            }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::NoopManager;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultUpdateService<NoopManager>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let snapshot_manager = NoopManager;
        let service = DefaultUpdateService::new(state_manager, snapshot_manager);
        (service, temp_dir)
    }

    #[test]
    fn test_parse_updates() {
        let (service, _temp) = create_test_service();

        let output = "firefox 120.0-1 -> 121.0-1\nlinux 6.6.1-1 -> 6.6.2-1\n";
        let updates = service.parse_updates(output);

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "firefox");
        assert_eq!(updates[0].risk, UpdateRisk::Low);
        assert_eq!(updates[1].name, "linux");
        assert_eq!(updates[1].risk, UpdateRisk::Critical);
    }

    #[test]
    fn test_assess_kernel_risk() {
        let (service, _temp) = create_test_service();

        let (risk, reason) = service.assess_package_risk("linux", "6.6.1", "6.6.2");
        assert_eq!(risk, UpdateRisk::Critical);
        assert!(reason.is_some());
    }

    #[test]
    fn test_assess_nvidia_risk() {
        let (service, _temp) = create_test_service();

        let (risk, _) = service.assess_package_risk("nvidia-dkms", "545.29", "545.30");
        assert_eq!(risk, UpdateRisk::High);
    }

    #[test]
    fn test_calculate_overall_risk() {
        let (service, _temp) = create_test_service();

        let packages = vec![
            PackageUpdate {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
                risk: UpdateRisk::Low,
                risk_reason: None,
            },
            PackageUpdate {
                name: "linux".to_string(),
                current_version: "6.6.1".to_string(),
                new_version: "6.6.2".to_string(),
                risk: UpdateRisk::Critical,
                risk_reason: Some("Kernel".to_string()),
            },
        ];

        let overall = service.calculate_overall_risk(&packages);
        assert_eq!(overall, UpdateRisk::Critical);
    }

    #[test]
    fn test_last_update_none() {
        let (service, _temp) = create_test_service();
        assert!(service.last_update().is_none());
    }

    // ==========================================================================
    // UpdateRisk Tests
    // ==========================================================================

    #[test]
    fn test_update_risk_ordering() {
        assert!(UpdateRisk::Low < UpdateRisk::Medium);
        assert!(UpdateRisk::Medium < UpdateRisk::High);
        assert!(UpdateRisk::High < UpdateRisk::Critical);
    }

    #[test]
    fn test_update_risk_equality() {
        assert_eq!(UpdateRisk::Low, UpdateRisk::Low);
        assert_ne!(UpdateRisk::Low, UpdateRisk::High);
    }

    #[test]
    fn test_update_risk_clone() {
        let risk = UpdateRisk::High;
        let cloned = risk;
        assert_eq!(cloned, UpdateRisk::High);
    }

    #[test]
    fn test_update_risk_debug() {
        let risk = UpdateRisk::Critical;
        let debug_str = format!("{:?}", risk);
        assert!(debug_str.contains("Critical"));
    }

    // ==========================================================================
    // PackageUpdate Tests
    // ==========================================================================

    #[test]
    fn test_package_update_creation() {
        let update = PackageUpdate {
            name: "firefox".to_string(),
            current_version: "120.0".to_string(),
            new_version: "121.0".to_string(),
            risk: UpdateRisk::Low,
            risk_reason: None,
        };

        assert_eq!(update.name, "firefox");
        assert_eq!(update.current_version, "120.0");
        assert_eq!(update.new_version, "121.0");
        assert_eq!(update.risk, UpdateRisk::Low);
        assert!(update.risk_reason.is_none());
    }

    #[test]
    fn test_package_update_with_risk_reason() {
        let update = PackageUpdate {
            name: "linux".to_string(),
            current_version: "6.6.1".to_string(),
            new_version: "6.6.2".to_string(),
            risk: UpdateRisk::Critical,
            risk_reason: Some("Kernel update requires reboot".to_string()),
        };

        assert!(update.risk_reason.is_some());
        assert!(update.risk_reason.unwrap().contains("reboot"));
    }

    #[test]
    fn test_package_update_clone() {
        let update = PackageUpdate {
            name: "test".to_string(),
            current_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            risk: UpdateRisk::Medium,
            risk_reason: Some("Test".to_string()),
        };

        let cloned = update.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.risk, UpdateRisk::Medium);
    }

    // ==========================================================================
    // UpdatePlan Tests
    // ==========================================================================

    #[test]
    fn test_update_plan_creation() {
        let plan = UpdatePlan {
            packages: vec![],
            overall_risk: UpdateRisk::Low,
            snapshot_recommended: false,
            news_items: vec![],
            created_at: Utc::now(),
        };

        assert!(plan.packages.is_empty());
        assert!(!plan.snapshot_recommended);
    }

    #[test]
    fn test_update_plan_with_packages() {
        let packages = vec![PackageUpdate {
            name: "firefox".to_string(),
            current_version: "120.0".to_string(),
            new_version: "121.0".to_string(),
            risk: UpdateRisk::Low,
            risk_reason: None,
        }];

        let plan = UpdatePlan {
            packages,
            overall_risk: UpdateRisk::Low,
            snapshot_recommended: false,
            news_items: vec![],
            created_at: Utc::now(),
        };

        assert_eq!(plan.packages.len(), 1);
        assert_eq!(plan.packages[0].name, "firefox");
    }

    #[test]
    fn test_update_plan_high_risk_recommends_snapshot() {
        let plan = UpdatePlan {
            packages: vec![],
            overall_risk: UpdateRisk::High,
            snapshot_recommended: true,
            news_items: vec![],
            created_at: Utc::now(),
        };

        assert!(plan.snapshot_recommended);
    }

    #[test]
    fn test_update_plan_with_news() {
        let plan = UpdatePlan {
            packages: vec![],
            overall_risk: UpdateRisk::Low,
            snapshot_recommended: false,
            news_items: vec!["Important: Check /etc/pacman.d/mirrorlist".to_string()],
            created_at: Utc::now(),
        };

        assert_eq!(plan.news_items.len(), 1);
    }

    #[test]
    fn test_update_plan_clone() {
        let plan = UpdatePlan {
            packages: vec![],
            overall_risk: UpdateRisk::Medium,
            snapshot_recommended: true,
            news_items: vec!["News item".to_string()],
            created_at: Utc::now(),
        };

        let cloned = plan.clone();
        assert_eq!(cloned.overall_risk, UpdateRisk::Medium);
        assert_eq!(cloned.news_items.len(), 1);
    }

    // ==========================================================================
    // Risk Assessment Tests
    // ==========================================================================

    #[test]
    fn test_assess_linux_kernel_variants() {
        let (service, _temp) = create_test_service();

        // Test different kernel package names
        let (risk1, _) = service.assess_package_risk("linux", "6.6.1", "6.6.2");
        assert_eq!(risk1, UpdateRisk::Critical);

        let (risk2, _) = service.assess_package_risk("linux-zen", "6.6.1", "6.6.2");
        assert_eq!(risk2, UpdateRisk::Critical);

        let (risk3, _) = service.assess_package_risk("linux-lts", "5.15.1", "5.15.2");
        assert_eq!(risk3, UpdateRisk::Critical);
    }

    #[test]
    fn test_assess_systemd_risk() {
        let (service, _temp) = create_test_service();

        let (risk1, reason1) = service.assess_package_risk("systemd", "254.5", "254.6");
        assert_eq!(risk1, UpdateRisk::High);
        assert!(reason1.is_some());

        let (risk2, _) = service.assess_package_risk("systemd-libs", "254.5", "254.6");
        assert_eq!(risk2, UpdateRisk::High);
    }

    #[test]
    fn test_assess_core_library_risk() {
        let (service, _temp) = create_test_service();

        let (risk1, reason1) = service.assess_package_risk("glibc", "2.38", "2.39");
        assert_eq!(risk1, UpdateRisk::High);
        assert!(reason1.is_some());

        let (risk2, _) = service.assess_package_risk("gcc-libs", "13.2", "14.0");
        assert_eq!(risk2, UpdateRisk::High);
    }

    #[test]
    fn test_assess_graphics_risk() {
        let (service, _temp) = create_test_service();

        let (risk1, reason1) = service.assess_package_risk("mesa", "23.2", "23.3");
        assert_eq!(risk1, UpdateRisk::Medium);
        assert!(reason1.is_some());

        let (risk2, _) = service.assess_package_risk("vulkan-radeon", "23.2", "23.3");
        assert_eq!(risk2, UpdateRisk::Medium);
    }

    #[test]
    fn test_assess_audio_risk() {
        let (service, _temp) = create_test_service();

        let (risk1, reason1) = service.assess_package_risk("pipewire", "1.0", "1.1");
        assert_eq!(risk1, UpdateRisk::Medium);
        assert!(reason1.is_some());

        let (risk2, _) = service.assess_package_risk("wireplumber", "0.5", "0.6");
        assert_eq!(risk2, UpdateRisk::Medium);
    }

    #[test]
    fn test_assess_normal_package_risk() {
        let (service, _temp) = create_test_service();

        let (risk, reason) = service.assess_package_risk("firefox", "120.0", "121.0");
        assert_eq!(risk, UpdateRisk::Low);
        assert!(reason.is_none());
    }

    // ==========================================================================
    // Parse Updates Tests
    // ==========================================================================

    #[test]
    fn test_parse_updates_empty() {
        let (service, _temp) = create_test_service();

        let updates = service.parse_updates("");
        assert!(updates.is_empty());
    }

    #[test]
    fn test_parse_updates_single_package() {
        let (service, _temp) = create_test_service();

        let output = "firefox 120.0-1 -> 121.0-1";
        let updates = service.parse_updates(output);

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "firefox");
        assert_eq!(updates[0].current_version, "120.0-1");
        assert_eq!(updates[0].new_version, "121.0-1");
    }

    #[test]
    fn test_parse_updates_multiple_packages() {
        let (service, _temp) = create_test_service();

        let output =
            "firefox 120.0-1 -> 121.0-1\nchromium 119.0-1 -> 120.0-1\nneovim 0.9.4-1 -> 0.9.5-1";
        let updates = service.parse_updates(output);

        assert_eq!(updates.len(), 3);
    }

    #[test]
    fn test_parse_updates_invalid_format() {
        let (service, _temp) = create_test_service();

        let output = "invalid line\nalso invalid";
        let updates = service.parse_updates(output);

        assert!(updates.is_empty());
    }

    #[test]
    fn test_parse_updates_mixed_valid_invalid() {
        let (service, _temp) = create_test_service();

        let output = "invalid\nfirefox 120.0-1 -> 121.0-1\nalso invalid";
        let updates = service.parse_updates(output);

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "firefox");
    }

    // ==========================================================================
    // Calculate Overall Risk Tests
    // ==========================================================================

    #[test]
    fn test_calculate_overall_risk_empty() {
        let (service, _temp) = create_test_service();

        let overall = service.calculate_overall_risk(&[]);
        assert_eq!(overall, UpdateRisk::Low);
    }

    #[test]
    fn test_calculate_overall_risk_single_low() {
        let (service, _temp) = create_test_service();

        let packages = vec![PackageUpdate {
            name: "firefox".to_string(),
            current_version: "120.0".to_string(),
            new_version: "121.0".to_string(),
            risk: UpdateRisk::Low,
            risk_reason: None,
        }];

        let overall = service.calculate_overall_risk(&packages);
        assert_eq!(overall, UpdateRisk::Low);
    }

    #[test]
    fn test_calculate_overall_risk_mixed() {
        let (service, _temp) = create_test_service();

        let packages = vec![
            PackageUpdate {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
                risk: UpdateRisk::Low,
                risk_reason: None,
            },
            PackageUpdate {
                name: "mesa".to_string(),
                current_version: "23.2".to_string(),
                new_version: "23.3".to_string(),
                risk: UpdateRisk::Medium,
                risk_reason: Some("Graphics".to_string()),
            },
        ];

        let overall = service.calculate_overall_risk(&packages);
        assert_eq!(overall, UpdateRisk::Medium);
    }

    #[test]
    fn test_calculate_overall_risk_high_wins() {
        let (service, _temp) = create_test_service();

        let packages = vec![
            PackageUpdate {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
                risk: UpdateRisk::Low,
                risk_reason: None,
            },
            PackageUpdate {
                name: "nvidia".to_string(),
                current_version: "545.29".to_string(),
                new_version: "545.30".to_string(),
                risk: UpdateRisk::High,
                risk_reason: Some("NVIDIA".to_string()),
            },
        ];

        let overall = service.calculate_overall_risk(&packages);
        assert_eq!(overall, UpdateRisk::High);
    }

    // ==========================================================================
    // Service Creation Tests
    // ==========================================================================

    #[test]
    fn test_service_creation() {
        let (service, _temp) = create_test_service();
        // Service should be created without panicking
        assert!(service.last_update().is_none());
    }

    // ==========================================================================
    // Pacman Output Parser Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_pacman_parser_new() {
        let parser = PacmanOutputParser::new();
        assert!(parser.total_packages.is_none());
        assert!(parser.current_package.is_none());
    }

    #[test]
    fn test_pacman_parser_packages_count() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("Packages (15) firefox-120.0 chromium-119.0");
        assert_eq!(event, Some(PacmanEvent::PackageCount(15)));
        assert_eq!(parser.total_packages, Some(15));
    }

    #[test]
    fn test_pacman_parser_packages_count_german() {
        let mut parser = PacmanOutputParser::new();

        // German locale support
        let event = parser.parse_line("Pakete (10) firefox chromium");
        assert_eq!(event, Some(PacmanEvent::PackageCount(10)));
    }

    #[test]
    fn test_pacman_parser_upgrading() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("(1/5) upgrading firefox...");
        match event {
            Some(PacmanEvent::PackageStarted {
                package,
                current,
                total,
            }) => {
                assert_eq!(package, "firefox");
                assert_eq!(current, 1);
                assert_eq!(total, 5);
            }
            _ => panic!("Expected PackageStarted event"),
        }
        assert_eq!(parser.current_package, Some("firefox".to_string()));
    }

    #[test]
    fn test_pacman_parser_reinstalling() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("(3/10) reinstalling glibc...");
        match event {
            Some(PacmanEvent::PackageStarted {
                package,
                current,
                total,
            }) => {
                assert_eq!(package, "glibc");
                assert_eq!(current, 3);
                assert_eq!(total, 10);
            }
            _ => panic!("Expected PackageStarted event"),
        }
    }

    #[test]
    fn test_pacman_parser_installing() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("(2/8) installing new-package...");
        match event {
            Some(PacmanEvent::PackageStarted {
                package,
                current,
                total,
            }) => {
                assert_eq!(package, "new-package");
                assert_eq!(current, 2);
                assert_eq!(total, 8);
            }
            _ => panic!("Expected PackageStarted event"),
        }
    }

    #[test]
    fn test_pacman_parser_error() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("error: failed to commit transaction");
        match event {
            Some(PacmanEvent::Error { message }) => {
                assert_eq!(message, "failed to commit transaction");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_pacman_parser_unrelated_line() {
        let mut parser = PacmanOutputParser::new();

        let event = parser.parse_line("resolving dependencies...");
        assert!(event.is_none());
    }

    #[test]
    fn test_pacman_parser_finalize() {
        let mut parser = PacmanOutputParser::new();

        // Start a package
        parser.parse_line("(1/3) upgrading firefox...");

        // Finalize should return completion event
        let event = parser.finalize();
        match event {
            Some(PacmanEvent::PackageCompleted { package }) => {
                assert_eq!(package, "firefox");
            }
            _ => panic!("Expected PackageCompleted event"),
        }

        // Second finalize should return None
        let event2 = parser.finalize();
        assert!(event2.is_none());
    }

    #[test]
    fn test_pacman_parser_finalize_no_package() {
        let mut parser = PacmanOutputParser::new();

        // No package started
        let event = parser.finalize();
        assert!(event.is_none());
    }

    // ==========================================================================
    // InterruptedUpdate Tests (FR-5.10 Phase 3)
    // ==========================================================================

    #[test]
    fn test_interrupted_update_struct() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress};

        let plan = SavedUpdatePlan {
            packages: vec![
                SavedPackage {
                    name: "firefox".to_string(),
                    current_version: "120.0".to_string(),
                    new_version: "121.0".to_string(),
                },
                SavedPackage {
                    name: "chromium".to_string(),
                    current_version: "119.0".to_string(),
                    new_version: "120.0".to_string(),
                },
            ],
            snapshot_recommended: true,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, Some("snap-123".to_string()));
        progress.phase = UpdatePhase::Interrupted;

        // Mark one package as completed
        progress.mark_completed(crate::state::CompletedPackage {
            name: "firefox".to_string(),
            old_version: "120.0".to_string(),
            new_version: "121.0".to_string(),
            completed_at: Utc::now(),
        });

        let interrupted = InterruptedUpdate {
            progress: progress.clone(),
            completed_count: 1,
            remaining_count: 1,
            elapsed: Duration::seconds(120),
        };

        assert_eq!(interrupted.completed_count, 1);
        assert_eq!(interrupted.remaining_count, 1);
        assert_eq!(interrupted.elapsed.num_seconds(), 120);
        assert!(interrupted.progress.is_incomplete());
    }

    // ==========================================================================
    // check_interrupted() Tests (FR-5.10 Phase 3)
    // ==========================================================================

    #[test]
    fn test_check_interrupted_no_progress() {
        let (service, _temp) = create_test_service();

        // No progress set - should return None
        let result = service.check_interrupted();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_interrupted_completed_progress() {
        use crate::state::{
            CompletedPackage, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
        };

        let (service, _temp) = create_test_service();

        // Set up completed progress
        let plan = SavedUpdatePlan {
            packages: vec![SavedPackage {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
            }],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Completed;
        progress.mark_completed(CompletedPackage {
            name: "firefox".to_string(),
            old_version: "120.0".to_string(),
            new_version: "121.0".to_string(),
            completed_at: Utc::now(),
        });

        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        // Completed progress should not be considered interrupted
        let result = service.check_interrupted();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_interrupted_with_interrupted_progress() {
        use crate::state::{
            CompletedPackage, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
        };

        let (service, _temp) = create_test_service();

        // Set up interrupted progress (5/10 packages)
        let packages: Vec<SavedPackage> = (1..=10)
            .map(|i| SavedPackage {
                name: format!("pkg{}", i),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            })
            .collect();

        let plan = SavedUpdatePlan {
            packages,
            snapshot_recommended: true,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, Some("snap-456".to_string()));
        progress.phase = UpdatePhase::Interrupted;

        // Mark 5 packages as completed
        for i in 1..=5 {
            progress.mark_completed(CompletedPackage {
                name: format!("pkg{}", i),
                old_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
                completed_at: Utc::now(),
            });
        }

        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        let result = service.check_interrupted();
        assert!(result.is_some());

        let interrupted = result.unwrap();
        assert_eq!(interrupted.completed_count, 5);
        assert_eq!(interrupted.remaining_count, 5);
        assert!(interrupted.progress.snapshot_created);
        assert_eq!(
            interrupted.progress.snapshot_id,
            Some("snap-456".to_string())
        );
    }

    #[test]
    fn test_check_interrupted_installing_phase() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress};

        let (service, _temp) = create_test_service();

        let plan = SavedUpdatePlan {
            packages: vec![
                SavedPackage {
                    name: "pkg1".to_string(),
                    current_version: "1.0".to_string(),
                    new_version: "2.0".to_string(),
                },
                SavedPackage {
                    name: "pkg2".to_string(),
                    current_version: "1.0".to_string(),
                    new_version: "2.0".to_string(),
                },
            ],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Installing; // Mid-install

        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        // Installing phase with incomplete packages is considered interrupted
        let result = service.check_interrupted();
        assert!(result.is_some());
        assert_eq!(result.unwrap().remaining_count, 2);
    }

    // ==========================================================================
    // get_progress() Tests (FR-5.10 Phase 3)
    // ==========================================================================

    #[test]
    fn test_get_progress_none() {
        let (service, _temp) = create_test_service();
        assert!(service.get_progress().is_none());
    }

    #[test]
    fn test_get_progress_some() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress};

        let (service, _temp) = create_test_service();

        let plan = SavedUpdatePlan {
            packages: vec![SavedPackage {
                name: "test".to_string(),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            }],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let progress = UpdateProgress::new(plan, None);
        service
            .state_manager
            .set_update_progress(Some(progress.clone()))
            .unwrap();

        let result = service.get_progress();
        assert!(result.is_some());
        assert_eq!(result.unwrap().session_id, progress.session_id);
    }

    // ==========================================================================
    // clear_progress() Tests (FR-5.10 Phase 3)
    // ==========================================================================

    #[test]
    fn test_clear_progress() {
        use crate::state::{SavedUpdatePlan, UpdateProgress};

        let (service, _temp) = create_test_service();

        // Set some progress
        let plan = SavedUpdatePlan::default();
        let progress = UpdateProgress::new(plan, None);
        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        // Verify it's set
        assert!(service.get_progress().is_some());

        // Clear it
        service.clear_progress().unwrap();

        // Verify it's gone
        assert!(service.get_progress().is_none());
    }

    #[test]
    fn test_clear_progress_when_none() {
        let (service, _temp) = create_test_service();

        // Should not error when clearing nonexistent progress
        let result = service.clear_progress();
        assert!(result.is_ok());
    }

    // ==========================================================================
    // Edge Case Tests (FR-5.10 Phase 3)
    // ==========================================================================

    #[test]
    fn test_interrupted_update_all_packages_completed() {
        use crate::state::{
            CompletedPackage, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
        };

        let (service, _temp) = create_test_service();

        // Create progress where all packages are completed but phase is stuck on Interrupted
        let plan = SavedUpdatePlan {
            packages: vec![SavedPackage {
                name: "pkg1".to_string(),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            }],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Interrupted;
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        // All completed but phase is Interrupted - is_incomplete should return false
        // because completed_packages.len() == total_packages
        let result = service.check_interrupted();
        assert!(result.is_none()); // Not considered incomplete
    }

    #[test]
    fn test_interrupted_update_zero_packages() {
        use crate::state::{SavedUpdatePlan, UpdatePhase, UpdateProgress};

        let (service, _temp) = create_test_service();

        // Empty package list
        let plan = SavedUpdatePlan {
            packages: vec![],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Installing;

        service
            .state_manager
            .set_update_progress(Some(progress))
            .unwrap();

        // Zero packages - should not be considered incomplete
        let result = service.check_interrupted();
        assert!(result.is_none());
    }

    #[test]
    fn test_progress_completion_percentage() {
        use crate::state::{CompletedPackage, SavedPackage, SavedUpdatePlan, UpdateProgress};

        let packages: Vec<SavedPackage> = (1..=4)
            .map(|i| SavedPackage {
                name: format!("pkg{}", i),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            })
            .collect();

        let plan = SavedUpdatePlan {
            packages,
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        assert_eq!(progress.completion_percentage(), 0.0);

        // Complete 1 of 4 = 25%
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert_eq!(progress.completion_percentage(), 25.0);

        // Complete 2 of 4 = 50%
        progress.mark_completed(CompletedPackage {
            name: "pkg2".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert_eq!(progress.completion_percentage(), 50.0);
    }

    #[test]
    fn test_progress_completion_percentage_empty() {
        use crate::state::{SavedUpdatePlan, UpdateProgress};

        let plan = SavedUpdatePlan {
            packages: vec![],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let progress = UpdateProgress::new(plan, None);
        // Empty plan should return 100% (nothing to do)
        assert_eq!(progress.completion_percentage(), 100.0);
    }

    #[test]
    fn test_progress_remaining_packages() {
        use crate::state::{CompletedPackage, SavedPackage, SavedUpdatePlan, UpdateProgress};

        let packages: Vec<SavedPackage> = (1..=3)
            .map(|i| SavedPackage {
                name: format!("pkg{}", i),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            })
            .collect();

        let plan = SavedUpdatePlan {
            packages,
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);

        // All 3 remaining
        assert_eq!(progress.remaining_packages().len(), 3);

        // Complete pkg2
        progress.mark_completed(CompletedPackage {
            name: "pkg2".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        // 2 remaining (pkg1, pkg3)
        let remaining = progress.remaining_packages();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|p| p.name == "pkg1"));
        assert!(remaining.iter().any(|p| p.name == "pkg3"));
        assert!(!remaining.iter().any(|p| p.name == "pkg2"));
    }

    #[test]
    fn test_progress_is_incomplete_phases() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress};

        let plan = SavedUpdatePlan {
            packages: vec![SavedPackage {
                name: "pkg".to_string(),
                current_version: "1.0".to_string(),
                new_version: "2.0".to_string(),
            }],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        // Preparing phase is not incomplete
        let mut progress = UpdateProgress::new(plan.clone(), None);
        progress.phase = UpdatePhase::Preparing;
        assert!(!progress.is_incomplete());

        // Installing phase IS incomplete
        let mut progress = UpdateProgress::new(plan.clone(), None);
        progress.phase = UpdatePhase::Installing;
        assert!(progress.is_incomplete());

        // Interrupted phase IS incomplete
        let mut progress = UpdateProgress::new(plan.clone(), None);
        progress.phase = UpdatePhase::Interrupted;
        assert!(progress.is_incomplete());

        // Completed phase is not incomplete
        let mut progress = UpdateProgress::new(plan.clone(), None);
        progress.phase = UpdatePhase::Completed;
        assert!(!progress.is_incomplete());

        // Failed phase is not incomplete (it's failed, not recoverable)
        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Failed;
        assert!(!progress.is_incomplete());
    }

    #[test]
    fn test_to_saved_plan() {
        let plan = UpdatePlan {
            packages: vec![
                PackageUpdate {
                    name: "firefox".to_string(),
                    current_version: "120.0".to_string(),
                    new_version: "121.0".to_string(),
                    risk: UpdateRisk::Low,
                    risk_reason: None,
                },
                PackageUpdate {
                    name: "linux".to_string(),
                    current_version: "6.6.1".to_string(),
                    new_version: "6.6.2".to_string(),
                    risk: UpdateRisk::Critical,
                    risk_reason: Some("Kernel".to_string()),
                },
            ],
            overall_risk: UpdateRisk::Critical,
            snapshot_recommended: true,
            news_items: vec!["Important news".to_string()],
            created_at: Utc::now(),
        };

        let saved = DefaultUpdateService::<NoopManager>::to_saved_plan(&plan);

        assert_eq!(saved.packages.len(), 2);
        assert_eq!(saved.packages[0].name, "firefox");
        assert_eq!(saved.packages[0].current_version, "120.0");
        assert_eq!(saved.packages[1].name, "linux");
        assert!(saved.snapshot_recommended);
    }

    // ==========================================================================
    // Pre-flight Check Tests (Phase 2.1)
    // ==========================================================================

    #[test]
    fn test_preflight_status_equality() {
        assert_eq!(PreflightStatus::Pass, PreflightStatus::Pass);
        assert_ne!(PreflightStatus::Pass, PreflightStatus::Fail);
        assert_ne!(PreflightStatus::Warning, PreflightStatus::Skipped);
    }

    #[test]
    fn test_preflight_check_creation() {
        let check = PreflightCheck {
            name: "Network".to_string(),
            status: PreflightStatus::Pass,
            message: "Network OK".to_string(),
            details: None,
        };

        assert_eq!(check.name, "Network");
        assert_eq!(check.status, PreflightStatus::Pass);
        assert!(check.details.is_none());
    }

    #[test]
    fn test_preflight_check_with_details() {
        let check = PreflightCheck {
            name: "Disk Space".to_string(),
            status: PreflightStatus::Fail,
            message: "Not enough space".to_string(),
            details: Some("Need 2GB, have 500MB".to_string()),
        };

        assert_eq!(check.status, PreflightStatus::Fail);
        assert!(check.details.is_some());
        assert!(check.details.unwrap().contains("2GB"));
    }

    #[test]
    fn test_preflight_result_new() {
        let result = PreflightResult::new();

        // Default values
        assert!(!result.network_ok);
        assert!(!result.disk_space_ok);
        assert_eq!(result.disk_space_available, 0);
        assert!(result.battery_ok); // Defaults to true (assume AC)
        assert!(result.on_ac_power); // Defaults to true
        assert!(result.pacman_lock_free); // Defaults to true
        assert!(result.time_synced); // Defaults to true
        assert!(result.checks.is_empty());
        assert!(result.warnings.is_empty());
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn test_preflight_result_default() {
        let result = PreflightResult::default();
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn test_preflight_result_can_proceed_no_blockers() {
        let result = PreflightResult::new();
        assert!(result.can_proceed()); // No blockers = can proceed
    }

    #[test]
    fn test_preflight_result_can_proceed_with_blockers() {
        let mut result = PreflightResult::new();
        result.blockers.push("Network failed".to_string());
        assert!(!result.can_proceed());
    }

    #[test]
    fn test_preflight_result_has_warnings_empty() {
        let result = PreflightResult::new();
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_preflight_result_has_warnings_true() {
        let mut result = PreflightResult::new();
        result.warnings.push("Low battery".to_string());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_preflight_result_add_check_pass() {
        let mut result = PreflightResult::new();

        let check = PreflightCheck {
            name: "Test".to_string(),
            status: PreflightStatus::Pass,
            message: "Test passed".to_string(),
            details: None,
        };

        result.add_check(check);

        assert_eq!(result.checks.len(), 1);
        assert!(result.warnings.is_empty());
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn test_preflight_result_add_check_warning() {
        let mut result = PreflightResult::new();

        let check = PreflightCheck {
            name: "Battery".to_string(),
            status: PreflightStatus::Warning,
            message: "Low battery (30%)".to_string(),
            details: None,
        };

        result.add_check(check);

        assert_eq!(result.checks.len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.blockers.is_empty());
        assert!(result.warnings[0].contains("Low battery"));
    }

    #[test]
    fn test_preflight_result_add_check_fail() {
        let mut result = PreflightResult::new();

        let check = PreflightCheck {
            name: "Network".to_string(),
            status: PreflightStatus::Fail,
            message: "No network connectivity".to_string(),
            details: None,
        };

        result.add_check(check);

        assert_eq!(result.checks.len(), 1);
        assert!(result.warnings.is_empty());
        assert_eq!(result.blockers.len(), 1);
        assert!(result.blockers[0].contains("No network"));
    }

    #[test]
    fn test_preflight_result_add_check_skipped() {
        let mut result = PreflightResult::new();

        let check = PreflightCheck {
            name: "Time Sync".to_string(),
            status: PreflightStatus::Skipped,
            message: "Could not check".to_string(),
            details: None,
        };

        result.add_check(check);

        assert_eq!(result.checks.len(), 1);
        assert!(result.warnings.is_empty());
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn test_preflight_result_multiple_checks() {
        let mut result = PreflightResult::new();

        // Add a passing check
        result.add_check(PreflightCheck {
            name: "Network".to_string(),
            status: PreflightStatus::Pass,
            message: "OK".to_string(),
            details: None,
        });

        // Add a warning
        result.add_check(PreflightCheck {
            name: "Battery".to_string(),
            status: PreflightStatus::Warning,
            message: "Low".to_string(),
            details: None,
        });

        // Add a blocker
        result.add_check(PreflightCheck {
            name: "Disk".to_string(),
            status: PreflightStatus::Fail,
            message: "Full".to_string(),
            details: None,
        });

        assert_eq!(result.checks.len(), 3);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.blockers.len(), 1);
        assert!(!result.can_proceed());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_preflight_result_clone() {
        let mut result = PreflightResult::new();
        result.network_ok = true;
        result.disk_space_available = 1024;
        result.warnings.push("Test warning".to_string());

        let cloned = result.clone();
        assert!(cloned.network_ok);
        assert_eq!(cloned.disk_space_available, 1024);
        assert_eq!(cloned.warnings.len(), 1);
    }

    #[test]
    fn test_preflight_check_clone() {
        let check = PreflightCheck {
            name: "Test".to_string(),
            status: PreflightStatus::Pass,
            message: "Message".to_string(),
            details: Some("Details".to_string()),
        };

        let cloned = check.clone();
        assert_eq!(cloned.name, "Test");
        assert_eq!(cloned.status, PreflightStatus::Pass);
        assert!(cloned.details.is_some());
    }

    #[test]
    fn test_preflight_status_copy() {
        let status = PreflightStatus::Warning;
        let copied = status;
        assert_eq!(copied, PreflightStatus::Warning);
    }

    // Integration test for run_preflight_checks
    // Note: This test may have varying results depending on system state
    #[test]
    fn test_run_preflight_checks_returns_result() {
        let (service, _temp) = create_test_service();

        let result = service.run_preflight_checks();

        // Should have all 5 checks
        assert_eq!(result.checks.len(), 5);

        // Check names should be present
        let check_names: Vec<&str> = result.checks.iter().map(|c| c.name.as_str()).collect();
        assert!(check_names.contains(&"Network"));
        assert!(check_names.contains(&"Disk Space"));
        assert!(check_names.contains(&"Power"));
        assert!(check_names.contains(&"Pacman Lock"));
        assert!(check_names.contains(&"Time Sync"));
    }

    // ==========================================================================
    // Arch News Check Tests (Phase 2.2)
    // ==========================================================================

    fn create_test_news_item(title: &str, url: &str, requires_manual: bool) -> ArchNewsItem {
        ArchNewsItem {
            title: title.to_string(),
            url: url.to_string(),
            date: "2024-01-15".to_string(),
            description: "Test news description".to_string(),
            requires_manual,
        }
    }

    #[test]
    fn test_check_news_empty_items() {
        let (service, _temp) = create_test_service();

        let (check, unack, blocks) = service.check_news(&[]);

        assert_eq!(check.status, PreflightStatus::Pass);
        assert!(check.message.contains("No recent"));
        assert!(unack.is_empty());
        assert!(!blocks);
    }

    #[test]
    fn test_check_news_all_acknowledged() {
        let (service, temp) = create_test_service();

        // Acknowledge all news first
        let news = vec![
            create_test_news_item("Test News 1", "https://archlinux.org/news/1/", false),
            create_test_news_item("Test News 2", "https://archlinux.org/news/2/", false),
        ];
        for item in &news {
            service.state_manager.acknowledge_news(&item.url).unwrap();
        }

        let (check, unack, blocks) = service.check_news(&news);

        assert_eq!(check.status, PreflightStatus::Pass);
        assert!(check.message.contains("acknowledged"));
        assert!(unack.is_empty());
        assert!(!blocks);
        drop(temp);
    }

    #[test]
    fn test_check_news_unacknowledged_non_critical() {
        let (service, _temp) = create_test_service();

        let news = vec![create_test_news_item(
            "Test News",
            "https://archlinux.org/news/1/",
            false,
        )];

        let (check, unack, blocks) = service.check_news(&news);

        assert_eq!(check.status, PreflightStatus::Warning);
        assert!(check.message.contains("1 unacknowledged"));
        assert_eq!(unack.len(), 1);
        assert!(!blocks); // Non-critical news doesn't block
    }

    #[test]
    fn test_check_news_unacknowledged_critical() {
        let (service, _temp) = create_test_service();

        let news = vec![create_test_news_item(
            "Manual intervention required",
            "https://archlinux.org/news/critical/",
            true,
        )];

        let (check, unack, blocks) = service.check_news(&news);

        assert_eq!(check.status, PreflightStatus::Fail);
        assert!(check.message.contains("manual intervention"));
        assert_eq!(unack.len(), 1);
        assert!(unack[0].requires_manual);
        assert!(blocks); // Critical news blocks update
    }

    #[test]
    fn test_check_news_mixed_critical_and_non_critical() {
        let (service, _temp) = create_test_service();

        let news = vec![
            create_test_news_item("Regular update", "https://archlinux.org/news/1/", false),
            create_test_news_item("Critical update", "https://archlinux.org/news/2/", true),
            create_test_news_item("Another update", "https://archlinux.org/news/3/", false),
        ];

        let (check, unack, blocks) = service.check_news(&news);

        assert_eq!(check.status, PreflightStatus::Fail); // Critical takes precedence
        assert_eq!(unack.len(), 3);
        assert!(blocks); // Blocked due to critical news
    }

    #[test]
    fn test_check_news_partially_acknowledged() {
        let (service, temp) = create_test_service();

        let news = vec![
            create_test_news_item("Acknowledged news", "https://archlinux.org/news/1/", false),
            create_test_news_item("New news", "https://archlinux.org/news/2/", false),
        ];

        // Acknowledge only the first one
        service
            .state_manager
            .acknowledge_news("https://archlinux.org/news/1/")
            .unwrap();

        let (check, unack, blocks) = service.check_news(&news);

        assert_eq!(check.status, PreflightStatus::Warning);
        assert_eq!(unack.len(), 1);
        assert_eq!(unack[0].url, "https://archlinux.org/news/2/");
        assert!(!blocks);
        drop(temp);
    }

    #[test]
    fn test_run_preflight_checks_with_news() {
        let (service, _temp) = create_test_service();

        let news = vec![create_test_news_item(
            "Test News",
            "https://archlinux.org/news/1/",
            false,
        )];

        let result = service.run_preflight_checks_with_news(&news);

        // Should have 6 checks (5 standard + 1 news)
        assert_eq!(result.checks.len(), 6);

        let check_names: Vec<&str> = result.checks.iter().map(|c| c.name.as_str()).collect();
        assert!(check_names.contains(&"Arch News"));

        // Should have unacknowledged news
        assert_eq!(result.unacknowledged_news.len(), 1);
        assert!(!result.news_blocks_update);
    }

    #[test]
    fn test_run_preflight_checks_with_critical_news() {
        let (service, _temp) = create_test_service();

        let news = vec![create_test_news_item(
            "Manual intervention required",
            "https://archlinux.org/news/critical/",
            true,
        )];

        let result = service.run_preflight_checks_with_news(&news);

        assert!(result.news_blocks_update);
        assert_eq!(result.critical_news_count(), 1);
        assert!(!result.can_proceed_with_news()); // Should not be able to proceed
    }

    #[test]
    fn test_run_preflight_checks_with_no_news() {
        let (service, _temp) = create_test_service();

        let result = service.run_preflight_checks_with_news(&[]);

        // Should still have the Arch News check
        let check_names: Vec<&str> = result.checks.iter().map(|c| c.name.as_str()).collect();
        assert!(check_names.contains(&"Arch News"));

        assert!(result.unacknowledged_news.is_empty());
        assert!(!result.news_blocks_update);
    }

    #[test]
    fn test_unacknowledged_news_struct() {
        let news = UnacknowledgedNews {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            date: "2024-01-15".to_string(),
            description: "Test description".to_string(),
            requires_manual: true,
        };

        assert_eq!(news.title, "Test Title");
        assert_eq!(news.url, "https://example.com");
        assert!(news.requires_manual);
    }

    #[test]
    fn test_unacknowledged_news_serialization() {
        let news = UnacknowledgedNews {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            date: "2024-01-15".to_string(),
            description: "Description".to_string(),
            requires_manual: false,
        };

        let json = serde_json::to_string(&news).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("https://example.com"));

        let deserialized: UnacknowledgedNews = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, news.title);
        assert_eq!(deserialized.requires_manual, news.requires_manual);
    }

    #[test]
    fn test_preflight_result_can_proceed_with_news() {
        let mut result = PreflightResult::new();
        assert!(result.can_proceed_with_news()); // No blockers, no news blocking

        result.news_blocks_update = true;
        assert!(!result.can_proceed_with_news()); // Blocked by news

        result.news_blocks_update = false;
        result.blockers.push("Test blocker".to_string());
        assert!(!result.can_proceed_with_news()); // Blocked by other issue
    }

    #[test]
    fn test_preflight_result_critical_news_count() {
        let mut result = PreflightResult::new();
        assert_eq!(result.critical_news_count(), 0);

        result.unacknowledged_news.push(UnacknowledgedNews {
            title: "Test".to_string(),
            url: "url1".to_string(),
            date: "date".to_string(),
            description: "desc".to_string(),
            requires_manual: false,
        });
        assert_eq!(result.critical_news_count(), 0);

        result.unacknowledged_news.push(UnacknowledgedNews {
            title: "Critical".to_string(),
            url: "url2".to_string(),
            date: "date".to_string(),
            description: "desc".to_string(),
            requires_manual: true,
        });
        assert_eq!(result.critical_news_count(), 1);

        result.unacknowledged_news.push(UnacknowledgedNews {
            title: "Another Critical".to_string(),
            url: "url3".to_string(),
            date: "date".to_string(),
            description: "desc".to_string(),
            requires_manual: true,
        });
        assert_eq!(result.critical_news_count(), 2);
    }

    #[test]
    fn test_check_news_description_truncation() {
        let (service, _temp) = create_test_service();

        // Create news with long description
        let long_desc = "A".repeat(300);
        let news = vec![ArchNewsItem {
            title: "Test".to_string(),
            url: "https://archlinux.org/news/1/".to_string(),
            date: "2024-01-15".to_string(),
            description: long_desc,
            requires_manual: false,
        }];

        let (_, unack, _) = service.check_news(&news);

        // Description should be truncated to ~200 chars + "..."
        assert!(unack[0].description.len() <= 205);
        assert!(unack[0].description.ends_with("..."));
    }

    // ==========================================================================
    // Post-Update Detection Tests (Phase 2.4)
    // ==========================================================================

    #[test]
    fn test_post_update_result_new() {
        let result = PostUpdateResult::new();
        assert!(result.config_conflicts.is_empty());
        assert!(!result.reboot_required);
        assert!(result.reboot_packages.is_empty());
        assert!(result.failed_services.is_empty());
        assert!(!result.has_issues);
    }

    #[test]
    fn test_post_update_result_has_config_conflicts() {
        let mut result = PostUpdateResult::new();
        assert!(!result.has_config_conflicts());

        result.config_conflicts.push(ConfigConflict {
            original: "/etc/pacman.conf".to_string(),
            conflict_file: "/etc/pacman.conf.pacnew".to_string(),
            conflict_type: ConfigConflictType::Pacnew,
            package: Some("pacman".to_string()),
        });
        assert!(result.has_config_conflicts());
    }

    #[test]
    fn test_post_update_result_pacnew_count() {
        let mut result = PostUpdateResult::new();
        assert_eq!(result.pacnew_count(), 0);

        result.config_conflicts.push(ConfigConflict {
            original: "/etc/pacman.conf".to_string(),
            conflict_file: "/etc/pacman.conf.pacnew".to_string(),
            conflict_type: ConfigConflictType::Pacnew,
            package: None,
        });
        result.config_conflicts.push(ConfigConflict {
            original: "/etc/mkinitcpio.conf".to_string(),
            conflict_file: "/etc/mkinitcpio.conf.pacnew".to_string(),
            conflict_type: ConfigConflictType::Pacnew,
            package: None,
        });
        result.config_conflicts.push(ConfigConflict {
            original: "/etc/ssh/sshd_config".to_string(),
            conflict_file: "/etc/ssh/sshd_config.pacsave".to_string(),
            conflict_type: ConfigConflictType::Pacsave,
            package: None,
        });

        assert_eq!(result.pacnew_count(), 2);
        assert_eq!(result.pacsave_count(), 1);
    }

    #[test]
    fn test_post_update_result_has_failed_services() {
        let mut result = PostUpdateResult::new();
        assert!(!result.has_failed_services());

        result.failed_services.push(FailedService {
            name: "sshd.service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "failed".to_string(),
            description: "OpenSSH Daemon".to_string(),
        });
        assert!(result.has_failed_services());
    }

    #[test]
    fn test_post_update_result_update_has_issues() {
        let mut result = PostUpdateResult::new();
        result.update_has_issues();
        assert!(!result.has_issues);

        // Add a config conflict
        result.config_conflicts.push(ConfigConflict {
            original: "/etc/test".to_string(),
            conflict_file: "/etc/test.pacnew".to_string(),
            conflict_type: ConfigConflictType::Pacnew,
            package: None,
        });
        result.update_has_issues();
        assert!(result.has_issues);

        // Test with reboot required
        let mut result2 = PostUpdateResult::new();
        result2.reboot_required = true;
        result2.update_has_issues();
        assert!(result2.has_issues);

        // Test with failed services
        let mut result3 = PostUpdateResult::new();
        result3.failed_services.push(FailedService {
            name: "test.service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "failed".to_string(),
            description: "Test".to_string(),
        });
        result3.update_has_issues();
        assert!(result3.has_issues);
    }

    #[test]
    fn test_check_reboot_required_kernel() {
        let (service, _temp) = create_test_service();

        let packages = vec!["linux".to_string(), "firefox".to_string()];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert!(reboot_pkgs.contains(&"linux".to_string()));
        assert!(!reboot_pkgs.contains(&"firefox".to_string()));
    }

    #[test]
    fn test_check_reboot_required_glibc() {
        let (service, _temp) = create_test_service();

        let packages = vec!["glibc".to_string(), "vim".to_string()];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert!(reboot_pkgs.contains(&"glibc".to_string()));
    }

    #[test]
    fn test_check_reboot_required_systemd() {
        let (service, _temp) = create_test_service();

        let packages = vec!["systemd".to_string(), "systemd-libs".to_string()];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert_eq!(reboot_pkgs.len(), 2);
    }

    #[test]
    fn test_check_reboot_required_nvidia() {
        let (service, _temp) = create_test_service();

        let packages = vec!["nvidia".to_string(), "nvidia-utils".to_string()];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert_eq!(reboot_pkgs.len(), 2);
    }

    #[test]
    fn test_check_reboot_required_mesa() {
        let (service, _temp) = create_test_service();

        let packages = vec!["mesa".to_string(), "mesa-utils".to_string()];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert_eq!(reboot_pkgs.len(), 2);
    }

    #[test]
    fn test_check_reboot_required_none() {
        let (service, _temp) = create_test_service();

        let packages = vec![
            "firefox".to_string(),
            "vim".to_string(),
            "neovim".to_string(),
        ];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(!required);
        assert!(reboot_pkgs.is_empty());
    }

    #[test]
    fn test_check_reboot_required_mixed() {
        let (service, _temp) = create_test_service();

        let packages = vec![
            "firefox".to_string(),
            "linux-lts".to_string(),
            "vim".to_string(),
            "glibc".to_string(),
            "neovim".to_string(),
        ];
        let (required, reboot_pkgs) = service.check_reboot_required(&packages);

        assert!(required);
        assert_eq!(reboot_pkgs.len(), 2);
        assert!(reboot_pkgs.contains(&"linux-lts".to_string()));
        assert!(reboot_pkgs.contains(&"glibc".to_string()));
    }

    #[test]
    fn test_config_conflict_type_equality() {
        assert_eq!(ConfigConflictType::Pacnew, ConfigConflictType::Pacnew);
        assert_eq!(ConfigConflictType::Pacsave, ConfigConflictType::Pacsave);
        assert_ne!(ConfigConflictType::Pacnew, ConfigConflictType::Pacsave);
    }

    #[test]
    fn test_run_post_update_checks_no_packages() {
        let (service, _temp) = create_test_service();

        let result = service.run_post_update_checks(&[]);

        assert!(!result.reboot_required);
        assert!(result.reboot_packages.is_empty());
        // Config conflicts and failed services depend on system state
    }

    #[test]
    fn test_run_post_update_checks_with_kernel() {
        let (service, _temp) = create_test_service();

        let packages = vec!["linux".to_string(), "firefox".to_string()];
        let result = service.run_post_update_checks(&packages);

        assert!(result.reboot_required);
        assert!(result.reboot_packages.contains(&"linux".to_string()));
    }
}
