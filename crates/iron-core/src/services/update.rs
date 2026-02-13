//! Update Service - Safe system updates with risk assessment
//!
//! Provides update checking, risk assessment, and snapshot integration.

use crate::services::state::StateManager;
use crate::snapshot::SnapshotManager;
use crate::state::OperationStatus;
use crate::{IronResult, PackageError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::process::Command;

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

    /// Apply specific packages only
    fn apply_packages(&self, packages: &[String], create_snapshot: bool) -> IronResult<()>;

    /// Get last update time
    fn last_update(&self) -> Option<chrono::DateTime<Utc>>;

    /// Clean package cache
    fn clean_cache(&self, keep_versions: usize) -> IronResult<u64>;
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
}
