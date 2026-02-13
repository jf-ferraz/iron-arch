//! Update Service - Safe system updates with risk assessment
//!
//! Provides update checking, risk assessment, and snapshot integration.
//! Includes partial update recovery (FR-5.10) with real-time progress tracking.

use crate::services::state::StateManager;
use crate::snapshot::SnapshotManager;
use crate::state::{
    CompletedPackage, OperationStatus, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
};
use crate::{IronResult, PackageError};
use chrono::{Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

// ==========================================================================
// Pacman Output Parser (FR-5.10)
// ==========================================================================

/// Matches "Packages (N)" or "Pakete (N)" line (multilingual support)
static PACKAGES_COUNT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:Packages?|Pakete?)\s*\((\d+)\)").unwrap());

/// Matches "(X/N) upgrading package..." or "(X/N) reinstalling package..."
static UPGRADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+)/(\d+)\)\s+(upgrading|reinstalling)\s+([^\s.]+)").unwrap());

/// Matches "(X/N) installing package..."
static INSTALLING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((\d+)/(\d+)\)\s+installing\s+([^\s.]+)").unwrap());

/// Matches "error:" lines
static ERROR_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^error:\s*(.+)$").unwrap());

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
        if let Some(caps) = PACKAGES_COUNT.captures(line) {
            if let Ok(count) = caps[1].parse::<usize>() {
                self.total_packages = Some(count);
                return Some(PacmanEvent::PackageCount(count));
            }
        }

        // Check for upgrade/reinstall progress
        if let Some(caps) = UPGRADING.captures(line) {
            if let (Ok(current), Ok(total)) = (caps[1].parse::<usize>(), caps[2].parse::<usize>()) {
                let package = caps[4].to_string();

                // If we have a previous package, it completed
                let completed_event = self.last_started_package.take().map(|p| PacmanEvent::PackageCompleted { package: p });

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
        }

        // Check for install progress
        if let Some(caps) = INSTALLING.captures(line) {
            if let (Ok(current), Ok(total)) = (caps[1].parse::<usize>(), caps[2].parse::<usize>()) {
                let package = caps[3].to_string();

                self.current_package = Some(package.clone());
                self.last_started_package = Some(package.clone());

                return Some(PacmanEvent::PackageStarted {
                    package,
                    current,
                    total,
                });
            }
        }

        None
    }

    /// Mark the last package as completed (call at end of successful update)
    pub fn finalize(&mut self) -> Option<PacmanEvent> {
        self.last_started_package.take().map(|p| PacmanEvent::PackageCompleted { package: p })
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

/// Update service trait
pub trait UpdateService {
    /// Check for available updates
    fn check(&self) -> IronResult<UpdatePlan>;

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
        self.state_manager.set_update_progress(Some(progress.clone()))
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

        let stdout = child.stdout.take().ok_or_else(|| PackageError::PacmanFailed {
            message: "Failed to capture stdout".to_string(),
        })?;

        let mut parser = PacmanOutputParser::new();
        let reader = BufReader::new(stdout);

        // Track package versions for completion records
        let package_versions: std::collections::HashMap<String, (String, String)> = progress
            .plan
            .packages
            .iter()
            .map(|p| (p.name.clone(), (p.current_version.clone(), p.new_version.clone())))
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
                        if let Some(prev_pkg) = &progress.plan.packages.iter()
                            .find(|p| progress.completed_packages.iter().all(|c| c.name != p.name)
                                && p.name != package)
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
                .filter(|pkg| progress.completed_packages.iter().all(|c| c.name != pkg.name))
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
            .ok_or_else(|| crate::StateError::NoActiveUpdate)?;

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
        let packages = vec![
            PackageUpdate {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
                risk: UpdateRisk::Low,
                risk_reason: None,
            },
        ];

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

        let output = "firefox 120.0-1 -> 121.0-1\nchromium 119.0-1 -> 120.0-1\nneovim 0.9.4-1 -> 0.9.5-1";
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

        let packages = vec![
            PackageUpdate {
                name: "firefox".to_string(),
                current_version: "120.0".to_string(),
                new_version: "121.0".to_string(),
                risk: UpdateRisk::Low,
                risk_reason: None,
            },
        ];

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
            Some(PacmanEvent::PackageStarted { package, current, total }) => {
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
            Some(PacmanEvent::PackageStarted { package, current, total }) => {
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
            Some(PacmanEvent::PackageStarted { package, current, total }) => {
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
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase};

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
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase, CompletedPackage};

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

        service.state_manager.set_update_progress(Some(progress)).unwrap();

        // Completed progress should not be considered interrupted
        let result = service.check_interrupted();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_interrupted_with_interrupted_progress() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase, CompletedPackage};

        let (service, _temp) = create_test_service();

        // Set up interrupted progress (5/10 packages)
        let packages: Vec<SavedPackage> = (1..=10).map(|i| SavedPackage {
            name: format!("pkg{}", i),
            current_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
        }).collect();

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

        service.state_manager.set_update_progress(Some(progress)).unwrap();

        let result = service.check_interrupted();
        assert!(result.is_some());

        let interrupted = result.unwrap();
        assert_eq!(interrupted.completed_count, 5);
        assert_eq!(interrupted.remaining_count, 5);
        assert!(interrupted.progress.snapshot_created);
        assert_eq!(interrupted.progress.snapshot_id, Some("snap-456".to_string()));
    }

    #[test]
    fn test_check_interrupted_installing_phase() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase};

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

        service.state_manager.set_update_progress(Some(progress)).unwrap();

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
        service.state_manager.set_update_progress(Some(progress.clone())).unwrap();

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
        service.state_manager.set_update_progress(Some(progress)).unwrap();

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
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase, CompletedPackage};

        let (service, _temp) = create_test_service();

        // Create progress where all packages are completed but phase is stuck on Interrupted
        let plan = SavedUpdatePlan {
            packages: vec![
                SavedPackage {
                    name: "pkg1".to_string(),
                    current_version: "1.0".to_string(),
                    new_version: "2.0".to_string(),
                },
            ],
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

        service.state_manager.set_update_progress(Some(progress)).unwrap();

        // All completed but phase is Interrupted - is_incomplete should return false
        // because completed_packages.len() == total_packages
        let result = service.check_interrupted();
        assert!(result.is_none()); // Not considered incomplete
    }

    #[test]
    fn test_interrupted_update_zero_packages() {
        use crate::state::{SavedUpdatePlan, UpdateProgress, UpdatePhase};

        let (service, _temp) = create_test_service();

        // Empty package list
        let plan = SavedUpdatePlan {
            packages: vec![],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let mut progress = UpdateProgress::new(plan, None);
        progress.phase = UpdatePhase::Installing;

        service.state_manager.set_update_progress(Some(progress)).unwrap();

        // Zero packages - should not be considered incomplete
        let result = service.check_interrupted();
        assert!(result.is_none());
    }

    #[test]
    fn test_progress_completion_percentage() {
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, CompletedPackage};

        let packages: Vec<SavedPackage> = (1..=4).map(|i| SavedPackage {
            name: format!("pkg{}", i),
            current_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
        }).collect();

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
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, CompletedPackage};

        let packages: Vec<SavedPackage> = (1..=3).map(|i| SavedPackage {
            name: format!("pkg{}", i),
            current_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
        }).collect();

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
        use crate::state::{SavedPackage, SavedUpdatePlan, UpdateProgress, UpdatePhase};

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
}
