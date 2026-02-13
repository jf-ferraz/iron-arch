//! Snapshot management - Integration with timeshift/snapper
//!
//! This module provides a unified interface for system snapshots.

use crate::{IronResult, SnapshotError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Snapshot backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotBackend {
    /// Timeshift (BTRFS or RSYNC)
    Timeshift,
    /// Snapper (BTRFS)
    Snapper,
    /// None available
    None,
}

impl SnapshotBackend {
    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            SnapshotBackend::Timeshift => "Timeshift",
            SnapshotBackend::Snapper => "Snapper",
            SnapshotBackend::None => "None",
        }
    }
}

/// Snapshot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// Unique snapshot identifier
    pub id: String,
    /// Snapshot description/comment
    pub description: String,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Snapshot type (single, pre, post)
    pub snapshot_type: SnapshotType,
    /// Backend used
    pub backend: SnapshotBackend,
}

/// Type of snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Single standalone snapshot
    Single,
    /// Pre-transaction snapshot
    Pre,
    /// Post-transaction snapshot
    Post,
    /// Boot snapshot
    Boot,
}

/// Snapshot manager trait for abstraction
pub trait SnapshotManager {
    /// Get the backend type
    fn backend(&self) -> SnapshotBackend;

    /// Create a new snapshot
    fn create(&self, description: &str) -> IronResult<SnapshotInfo>;

    /// List all snapshots
    fn list(&self) -> IronResult<Vec<SnapshotInfo>>;

    /// Delete a snapshot by ID
    fn delete(&self, id: &str) -> IronResult<()>;

    /// Restore to a snapshot (may require reboot)
    fn restore(&self, id: &str) -> IronResult<()>;

    /// Check if snapshots are available
    fn is_available(&self) -> bool;
}

/// Detect available snapshot backend
pub fn detect_backend() -> SnapshotBackend {
    // Check for timeshift
    if Command::new("which")
        .arg("timeshift")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return SnapshotBackend::Timeshift;
    }

    // Check for snapper
    if Command::new("which")
        .arg("snapper")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return SnapshotBackend::Snapper;
    }

    SnapshotBackend::None
}

/// Create a snapshot manager based on detected backend
pub fn create_manager() -> Box<dyn SnapshotManager> {
    match detect_backend() {
        SnapshotBackend::Timeshift => Box::new(TimeshiftManager::new()),
        SnapshotBackend::Snapper => Box::new(SnapperManager::new()),
        SnapshotBackend::None => Box::new(NoopManager),
    }
}

/// Timeshift snapshot manager
pub struct TimeshiftManager;

impl TimeshiftManager {
    /// Create a new timeshift manager
    pub fn new() -> Self {
        Self
    }

    /// Run timeshift command
    fn run_timeshift(&self, args: &[&str]) -> IronResult<String> {
        let output = Command::new("timeshift").args(args).output().map_err(|e| {
            SnapshotError::CreateFailed {
                message: format!("Failed to run timeshift: {}", e),
            }
        })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(SnapshotError::CreateFailed {
                message: stderr.to_string(),
            }
            .into())
        }
    }
}

impl Default for TimeshiftManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotManager for TimeshiftManager {
    fn backend(&self) -> SnapshotBackend {
        SnapshotBackend::Timeshift
    }

    fn create(&self, description: &str) -> IronResult<SnapshotInfo> {
        let output = self.run_timeshift(&["--create", "--comments", description])?;

        // Parse the output to get snapshot info
        // Timeshift output format varies, so we do our best to extract info
        let id = output
            .lines()
            .find(|l| l.contains("Tagged snapshot"))
            .and_then(|l| l.split('\'').nth(1))
            .unwrap_or("unknown")
            .to_string();

        Ok(SnapshotInfo {
            id,
            description: description.to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Single,
            backend: SnapshotBackend::Timeshift,
        })
    }

    fn list(&self) -> IronResult<Vec<SnapshotInfo>> {
        let output = self.run_timeshift(&["--list"])?;
        let mut snapshots = Vec::new();

        for line in output.lines().skip(3) {
            // Skip header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // Parse date from timeshift format (YYYY-MM-DD_HH-MM-SS)
                let date_str = parts[1];
                let created = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d_%H-%M-%S")
                    .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                    .unwrap_or_else(|_| Utc::now());

                snapshots.push(SnapshotInfo {
                    id: parts[0].to_string(),
                    description: parts.get(3..).map(|p| p.join(" ")).unwrap_or_default(),
                    created,
                    snapshot_type: SnapshotType::Single,
                    backend: SnapshotBackend::Timeshift,
                });
            }
        }

        Ok(snapshots)
    }

    fn delete(&self, id: &str) -> IronResult<()> {
        self.run_timeshift(&["--delete", "--snapshot", id])?;
        Ok(())
    }

    fn restore(&self, id: &str) -> IronResult<()> {
        self.run_timeshift(&["--restore", "--snapshot", id, "--skip-grub"])?;
        Ok(())
    }

    fn is_available(&self) -> bool {
        Command::new("timeshift")
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Snapper snapshot manager
pub struct SnapperManager {
    /// Snapper config to use
    config: String,
}

impl SnapperManager {
    /// Create a new snapper manager with default config
    pub fn new() -> Self {
        Self {
            config: "root".to_string(),
        }
    }

    /// Create a new snapper manager with specific config
    pub fn with_config(config: &str) -> Self {
        Self {
            config: config.to_string(),
        }
    }

    /// Run snapper command
    fn run_snapper(&self, args: &[&str]) -> IronResult<String> {
        let mut cmd_args = vec!["-c", &self.config];
        cmd_args.extend(args);

        let output = Command::new("snapper")
            .args(&cmd_args)
            .output()
            .map_err(|e| SnapshotError::CreateFailed {
                message: format!("Failed to run snapper: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(SnapshotError::CreateFailed {
                message: stderr.to_string(),
            }
            .into())
        }
    }
}

impl Default for SnapperManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotManager for SnapperManager {
    fn backend(&self) -> SnapshotBackend {
        SnapshotBackend::Snapper
    }

    fn create(&self, description: &str) -> IronResult<SnapshotInfo> {
        let output = self.run_snapper(&["create", "-d", description, "--print-number"])?;
        let id = output.trim().to_string();

        Ok(SnapshotInfo {
            id,
            description: description.to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Single,
            backend: SnapshotBackend::Snapper,
        })
    }

    fn list(&self) -> IronResult<Vec<SnapshotInfo>> {
        let output = self.run_snapper(&["list", "--columns", "number,date,description"])?;
        let mut snapshots = Vec::new();

        for line in output.lines().skip(2) {
            // Skip header lines
            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let id = parts[0].to_string();
                if id == "0" {
                    continue; // Skip the current subvolume
                }

                let date_str = parts[1];
                let created =
                    chrono::NaiveDateTime::parse_from_str(date_str, "%a %b %d %H:%M:%S %Y")
                        .or_else(|_| {
                            chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                        })
                        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                        .unwrap_or_else(|_| Utc::now());

                snapshots.push(SnapshotInfo {
                    id,
                    description: parts[2].to_string(),
                    created,
                    snapshot_type: SnapshotType::Single,
                    backend: SnapshotBackend::Snapper,
                });
            }
        }

        Ok(snapshots)
    }

    fn delete(&self, id: &str) -> IronResult<()> {
        self.run_snapper(&["delete", id])?;
        Ok(())
    }

    fn restore(&self, id: &str) -> IronResult<()> {
        // Snapper doesn't have direct restore, we need to use snapper-rollback or manual
        self.run_snapper(&["undochange", &format!("{}..0", id)])?;
        Ok(())
    }

    fn is_available(&self) -> bool {
        Command::new("snapper")
            .args(["-c", &self.config, "list"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// No-op snapshot manager when no backend is available
pub struct NoopManager;

impl SnapshotManager for NoopManager {
    fn backend(&self) -> SnapshotBackend {
        SnapshotBackend::None
    }

    fn create(&self, _description: &str) -> IronResult<SnapshotInfo> {
        Err(SnapshotError::NoSnapshotTool.into())
    }

    fn list(&self) -> IronResult<Vec<SnapshotInfo>> {
        Err(SnapshotError::NoSnapshotTool.into())
    }

    fn delete(&self, _id: &str) -> IronResult<()> {
        Err(SnapshotError::NoSnapshotTool.into())
    }

    fn restore(&self, _id: &str) -> IronResult<()> {
        Err(SnapshotError::NoSnapshotTool.into())
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_backend_name() {
        assert_eq!(SnapshotBackend::Timeshift.name(), "Timeshift");
        assert_eq!(SnapshotBackend::Snapper.name(), "Snapper");
        assert_eq!(SnapshotBackend::None.name(), "None");
    }

    #[test]
    fn test_noop_manager() {
        let manager = NoopManager;
        assert_eq!(manager.backend(), SnapshotBackend::None);
        assert!(!manager.is_available());
        assert!(manager.create("test").is_err());
        assert!(manager.list().is_err());
    }

    #[test]
    fn test_snapshot_info_serialization() {
        let info = SnapshotInfo {
            id: "123".to_string(),
            description: "Test snapshot".to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Single,
            backend: SnapshotBackend::Timeshift,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SnapshotInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "123");
        assert_eq!(deserialized.description, "Test snapshot");
    }

    #[test]
    fn test_detect_backend() {
        // This test will return whatever is actually installed
        let backend = detect_backend();
        // Just verify it returns a valid variant
        let _ = backend.name();
    }
}
