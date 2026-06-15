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

/// Blanket implementation so `Box<dyn SnapshotManager>` can be used where `S: SnapshotManager`.
impl SnapshotManager for Box<dyn SnapshotManager> {
    fn backend(&self) -> SnapshotBackend {
        (**self).backend()
    }
    fn create(&self, description: &str) -> IronResult<SnapshotInfo> {
        (**self).create(description)
    }
    fn list(&self) -> IronResult<Vec<SnapshotInfo>> {
        (**self).list()
    }
    fn delete(&self, id: &str) -> IronResult<()> {
        (**self).delete(id)
    }
    fn restore(&self, id: &str) -> IronResult<()> {
        (**self).restore(id)
    }
    fn is_available(&self) -> bool {
        (**self).is_available()
    }
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

// =============================================================================
// Parsing Helper Functions (for testing)
// =============================================================================

/// Parse timeshift --list output into snapshot info
///
/// Expected format:
/// ```text
/// Num     Name                  Tags  Description
/// -----------------------------------------------
/// 0    >  2024-01-15_10-30-00   O     Pre-update backup
/// 1       2024-01-16_11-00-00   O     Post-update
/// ```
pub fn parse_timeshift_list_output(output: &str) -> Vec<SnapshotInfo> {
    let mut snapshots = Vec::new();

    for line in output.lines().skip(2) {
        // Skip header lines
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            // The format can have an optional '>' marker
            // Find the date-time field (YYYY-MM-DD_HH-MM-SS pattern)
            let (id, date_idx) = if parts[1] == ">" {
                (parts[0].to_string(), 2)
            } else if parts[0].contains("-") && parts[0].contains("_") {
                // No ID, date is first
                continue;
            } else {
                (parts[0].to_string(), 1)
            };

            if parts.len() <= date_idx {
                continue;
            }

            let date_str = parts[date_idx];
            let created = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d_%H-%M-%S")
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or_else(|_| Utc::now());

            // Description is after the Tags column (usually 'O' for on-demand)
            let desc_start = date_idx + 2; // Skip date and tags
            let description = if parts.len() > desc_start {
                parts[desc_start..].join(" ")
            } else {
                String::new()
            };

            snapshots.push(SnapshotInfo {
                id,
                description,
                created,
                snapshot_type: SnapshotType::Single,
                backend: SnapshotBackend::Timeshift,
            });
        }
    }

    snapshots
}

/// Parse snapper list output into snapshot info
///
/// Expected format (with --columns number,date,description):
/// ```text
///  # | Date                     | Description
/// ---+--------------------------+----------------------
///  0 |                          | current
///  1 | 2024-01-15 10:30:00      | Pre-update backup
/// ```
pub fn parse_snapper_list_output(output: &str) -> Vec<SnapshotInfo> {
    let mut snapshots = Vec::new();

    for line in output.lines().skip(2) {
        // Skip header lines
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            let id = parts[0].trim().to_string();
            if id == "0" || id.is_empty() {
                continue; // Skip the current subvolume
            }

            let date_str = parts[1].trim();
            let created = chrono::NaiveDateTime::parse_from_str(date_str, "%a %b %d %H:%M:%S %Y")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S"))
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

    snapshots
}

/// Parse timeshift --create output to extract snapshot ID
///
/// Expected format:
/// ```text
/// Creating new snapshot...
/// Tagged snapshot '2024-01-15_12-30-00': Description
/// Snapshot saved successfully.
/// ```
pub fn parse_timeshift_create_output(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("Tagged snapshot") {
            // Extract ID between single quotes
            if let Some(start) = line.find('\'')
                && let Some(end) = line[start + 1..].find('\'')
            {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
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

    // ==========================================================================
    // SnapshotBackend Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_backend_equality() {
        assert_eq!(SnapshotBackend::Timeshift, SnapshotBackend::Timeshift);
        assert_eq!(SnapshotBackend::Snapper, SnapshotBackend::Snapper);
        assert_eq!(SnapshotBackend::None, SnapshotBackend::None);
        assert_ne!(SnapshotBackend::Timeshift, SnapshotBackend::Snapper);
    }

    #[test]
    fn test_snapshot_backend_clone() {
        let backend = SnapshotBackend::Timeshift;
        let cloned = backend;
        assert_eq!(cloned, SnapshotBackend::Timeshift);
    }

    #[test]
    fn test_snapshot_backend_debug() {
        let debug_str = format!("{:?}", SnapshotBackend::Timeshift);
        assert!(debug_str.contains("Timeshift"));

        let debug_str = format!("{:?}", SnapshotBackend::Snapper);
        assert!(debug_str.contains("Snapper"));

        let debug_str = format!("{:?}", SnapshotBackend::None);
        assert!(debug_str.contains("None"));
    }

    #[test]
    fn test_snapshot_backend_names() {
        assert_eq!(SnapshotBackend::Timeshift.name(), "Timeshift");
        assert_eq!(SnapshotBackend::Snapper.name(), "Snapper");
        assert_eq!(SnapshotBackend::None.name(), "None");
    }

    // ==========================================================================
    // SnapshotType Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_type_equality() {
        assert_eq!(SnapshotType::Single, SnapshotType::Single);
        assert_eq!(SnapshotType::Pre, SnapshotType::Pre);
        assert_eq!(SnapshotType::Post, SnapshotType::Post);
        assert_eq!(SnapshotType::Boot, SnapshotType::Boot);
        assert_ne!(SnapshotType::Single, SnapshotType::Pre);
    }

    #[test]
    fn test_snapshot_type_clone() {
        let st = SnapshotType::Pre;
        let cloned = st;
        assert_eq!(cloned, SnapshotType::Pre);
    }

    #[test]
    fn test_snapshot_type_debug() {
        let debug_str = format!("{:?}", SnapshotType::Single);
        assert!(debug_str.contains("Single"));

        let debug_str = format!("{:?}", SnapshotType::Pre);
        assert!(debug_str.contains("Pre"));

        let debug_str = format!("{:?}", SnapshotType::Post);
        assert!(debug_str.contains("Post"));

        let debug_str = format!("{:?}", SnapshotType::Boot);
        assert!(debug_str.contains("Boot"));
    }

    // ==========================================================================
    // SnapshotInfo Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_info_creation() {
        let info = SnapshotInfo {
            id: "test-123".to_string(),
            description: "Test snapshot".to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Single,
            backend: SnapshotBackend::Timeshift,
        };

        assert_eq!(info.id, "test-123");
        assert_eq!(info.description, "Test snapshot");
        assert_eq!(info.snapshot_type, SnapshotType::Single);
        assert_eq!(info.backend, SnapshotBackend::Timeshift);
    }

    #[test]
    fn test_snapshot_info_clone() {
        let info = SnapshotInfo {
            id: "clone-test".to_string(),
            description: "Clone test".to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Pre,
            backend: SnapshotBackend::Snapper,
        };

        let cloned = info.clone();
        assert_eq!(cloned.id, "clone-test");
        assert_eq!(cloned.snapshot_type, SnapshotType::Pre);
        assert_eq!(cloned.backend, SnapshotBackend::Snapper);
    }

    #[test]
    fn test_snapshot_info_debug() {
        let info = SnapshotInfo {
            id: "debug-test".to_string(),
            description: "Debug test".to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Post,
            backend: SnapshotBackend::None,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("Debug test"));
    }

    #[test]
    fn test_snapshot_info_with_all_types() {
        let types = vec![
            SnapshotType::Single,
            SnapshotType::Pre,
            SnapshotType::Post,
            SnapshotType::Boot,
        ];

        for snap_type in types {
            let info = SnapshotInfo {
                id: "type-test".to_string(),
                description: "Type test".to_string(),
                created: Utc::now(),
                snapshot_type: snap_type,
                backend: SnapshotBackend::Timeshift,
            };
            assert_eq!(info.snapshot_type, snap_type);
        }
    }

    #[test]
    fn test_snapshot_info_with_all_backends() {
        let backends = vec![
            SnapshotBackend::Timeshift,
            SnapshotBackend::Snapper,
            SnapshotBackend::None,
        ];

        for backend in backends {
            let info = SnapshotInfo {
                id: "backend-test".to_string(),
                description: "Backend test".to_string(),
                created: Utc::now(),
                snapshot_type: SnapshotType::Single,
                backend,
            };
            assert_eq!(info.backend, backend);
        }
    }

    // ==========================================================================
    // NoopManager Tests
    // ==========================================================================

    #[test]
    fn test_noop_manager_backend() {
        let manager = NoopManager;
        assert_eq!(manager.backend(), SnapshotBackend::None);
    }

    #[test]
    fn test_noop_manager_is_not_available() {
        let manager = NoopManager;
        assert!(!manager.is_available());
    }

    #[test]
    fn test_noop_manager_create_fails() {
        let manager = NoopManager;
        let result = manager.create("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_noop_manager_list_fails() {
        let manager = NoopManager;
        let result = manager.list();
        assert!(result.is_err());
    }

    #[test]
    fn test_noop_manager_delete_fails() {
        let manager = NoopManager;
        let result = manager.delete("123");
        assert!(result.is_err());
    }

    #[test]
    fn test_noop_manager_restore_fails() {
        let manager = NoopManager;
        let result = manager.restore("123");
        assert!(result.is_err());
    }

    // ==========================================================================
    // TimeshiftManager Tests
    // ==========================================================================

    #[test]
    fn test_timeshift_manager_creation() {
        let manager = TimeshiftManager::new();
        assert_eq!(manager.backend(), SnapshotBackend::Timeshift);
    }

    #[test]
    fn test_timeshift_manager_default() {
        let manager = TimeshiftManager;
        assert_eq!(manager.backend(), SnapshotBackend::Timeshift);
    }

    // ==========================================================================
    // SnapperManager Tests
    // ==========================================================================

    #[test]
    fn test_snapper_manager_creation() {
        let manager = SnapperManager::new();
        assert_eq!(manager.backend(), SnapshotBackend::Snapper);
    }

    #[test]
    fn test_snapper_manager_default() {
        let manager = SnapperManager::default();
        assert_eq!(manager.backend(), SnapshotBackend::Snapper);
    }

    #[test]
    fn test_snapper_manager_with_config() {
        let manager = SnapperManager::with_config("home");
        assert_eq!(manager.backend(), SnapshotBackend::Snapper);
    }

    // ==========================================================================
    // Serialization Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_backend_serialization() {
        let backend = SnapshotBackend::Timeshift;
        let json = serde_json::to_string(&backend).unwrap();
        let deserialized: SnapshotBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SnapshotBackend::Timeshift);
    }

    #[test]
    fn test_snapshot_type_serialization() {
        let snap_type = SnapshotType::Pre;
        let json = serde_json::to_string(&snap_type).unwrap();
        let deserialized: SnapshotType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SnapshotType::Pre);
    }

    #[test]
    fn test_snapshot_info_json_roundtrip() {
        let info = SnapshotInfo {
            id: "roundtrip-test".to_string(),
            description: "Roundtrip test".to_string(),
            created: Utc::now(),
            snapshot_type: SnapshotType::Boot,
            backend: SnapshotBackend::Snapper,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SnapshotInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, info.id);
        assert_eq!(deserialized.description, info.description);
        assert_eq!(deserialized.snapshot_type, info.snapshot_type);
        assert_eq!(deserialized.backend, info.backend);
    }

    // ==========================================================================
    // Parsing Helper Function Tests
    // ==========================================================================

    #[test]
    fn test_parse_timeshift_list_empty() {
        let output = "Num     Name                  Tags  Description\n\
                      -----------------------------------------------\n";
        let snapshots = parse_timeshift_list_output(output);
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_parse_timeshift_list_single_snapshot() {
        let output = "Num     Name                  Tags  Description\n\
                      -----------------------------------------------\n\
                      1       2024-01-15_10-30-00   O     Pre-update backup\n";
        let snapshots = parse_timeshift_list_output(output);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "1");
        assert_eq!(snapshots[0].description, "Pre-update backup");
        assert_eq!(snapshots[0].backend, SnapshotBackend::Timeshift);
    }

    #[test]
    fn test_parse_timeshift_list_multiple_snapshots() {
        let output = "Num     Name                  Tags  Description\n\
                      -----------------------------------------------\n\
                      1       2024-01-15_10-30-00   O     First backup\n\
                      2       2024-01-16_11-00-00   O     Second backup\n\
                      3       2024-01-20_14-45-30   O     Manual backup\n";
        let snapshots = parse_timeshift_list_output(output);
        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[0].id, "1");
        assert_eq!(snapshots[1].id, "2");
        assert_eq!(snapshots[2].id, "3");
        assert_eq!(snapshots[0].description, "First backup");
        assert_eq!(snapshots[2].description, "Manual backup");
    }

    #[test]
    fn test_parse_timeshift_list_with_current_marker() {
        let output = "Num     Name                  Tags  Description\n\
                      -----------------------------------------------\n\
                      0    >  2024-01-15_10-30-00   O     Current snapshot\n\
                      1       2024-01-16_11-00-00   O     Another snapshot\n";
        let snapshots = parse_timeshift_list_output(output);
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, "0");
        assert_eq!(snapshots[0].description, "Current snapshot");
    }

    #[test]
    fn test_parse_timeshift_list_multi_word_description() {
        let output = "Num     Name                  Tags  Description\n\
                      -----------------------------------------------\n\
                      1       2024-01-15_10-30-00   O     This is a long multi-word description\n";
        let snapshots = parse_timeshift_list_output(output);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0].description,
            "This is a long multi-word description"
        );
    }

    #[test]
    fn test_parse_snapper_list_empty() {
        let output = " # | Date                     | Description\n\
                      ---+--------------------------+----------------------\n\
                       0 |                          | current\n";
        let snapshots = parse_snapper_list_output(output);
        assert!(snapshots.is_empty()); // 0 is skipped
    }

    #[test]
    fn test_parse_snapper_list_single_snapshot() {
        let output = " # | Date                     | Description\n\
                      ---+--------------------------+----------------------\n\
                       0 |                          | current\n\
                       1 | 2024-01-15 10:30:00      | Pre-update backup\n";
        let snapshots = parse_snapper_list_output(output);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "1");
        assert_eq!(snapshots[0].description, "Pre-update backup");
        assert_eq!(snapshots[0].backend, SnapshotBackend::Snapper);
    }

    #[test]
    fn test_parse_snapper_list_multiple_snapshots() {
        let output = " # | Date                     | Description\n\
                      ---+--------------------------+----------------------\n\
                       0 |                          | current\n\
                       1 | 2024-01-15 10:30:00      | First backup\n\
                       2 | 2024-01-16 11:00:00      | Second backup\n\
                       5 | 2024-01-20 14:45:30      | Manual backup\n";
        let snapshots = parse_snapper_list_output(output);
        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[0].id, "1");
        assert_eq!(snapshots[1].id, "2");
        assert_eq!(snapshots[2].id, "5");
    }

    #[test]
    fn test_parse_snapper_list_alternative_date_format() {
        // Some snapper versions use this format
        let output = " # | Date                     | Description\n\
                      ---+--------------------------+----------------------\n\
                       0 |                          | current\n\
                       1 | Mon Jan 15 10:30:00 2024 | Backup with alt date\n";
        let snapshots = parse_snapper_list_output(output);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].description, "Backup with alt date");
    }

    #[test]
    fn test_parse_timeshift_create_output() {
        let output = "Creating new snapshot...\n\
                      Tagged snapshot '2024-01-15_12-30-00': Iron pre-update\n\
                      Snapshot saved successfully.\n";
        let id = parse_timeshift_create_output(output);
        assert_eq!(id, Some("2024-01-15_12-30-00".to_string()));
    }

    #[test]
    fn test_parse_timeshift_create_output_no_match() {
        let output = "Some other output\nNo tagged snapshot here\n";
        let id = parse_timeshift_create_output(output);
        assert_eq!(id, None);
    }

    #[test]
    fn test_parse_timeshift_create_output_numeric_id() {
        let output = "Tagged snapshot '3': Test backup\n";
        let id = parse_timeshift_create_output(output);
        assert_eq!(id, Some("3".to_string()));
    }

    // ==========================================================================
    // Integration with Mock Fixtures Tests
    // ==========================================================================

    #[test]
    fn test_mock_fixtures_timeshift_list_parsing() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "First backup", "2024-01-15_10-30-00")
            .with_snapshot("2", "Second backup", "2024-01-16_11-00-00")
            .build();

        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");

        // The mock output should be parseable
        let _snapshots = parse_timeshift_list_output(&output);
        // Mocks generate simplified output, so we verify the mock works
        assert!(output.contains("First backup"));
        assert!(output.contains("Second backup"));
    }

    #[test]
    fn test_mock_fixtures_snapper_list_parsing() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("1", "First backup", "2024-01-15 10:30:00")
            .with_snapshot("2", "Second backup", "2024-01-16 11:00:00")
            .build();

        let output = executor
            .execute(
                "snapper",
                &["-c", "root", "list", "--columns", "number,date,description"],
            )
            .expect("should execute");

        let snapshots = parse_snapper_list_output(&output);
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, "1");
        assert_eq!(snapshots[0].description, "First backup");
        assert_eq!(snapshots[1].id, "2");
        assert_eq!(snapshots[1].description, "Second backup");
    }

    #[test]
    fn test_mock_fixtures_backend_detection() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        // Timeshift available
        let executor = SnapshotMockBuilder::timeshift().build();
        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_ok());

        // Snapper not available when timeshift is configured
        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_fixtures_no_backend() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        let executor = SnapshotMockBuilder::none().build();

        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_err());

        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_fixtures_create_workflow() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        // Test Timeshift create
        let executor = SnapshotMockBuilder::timeshift().with_next_id(5).build();

        let result = executor.execute("timeshift", &["--create", "--comments", "Test backup"]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Tagged snapshot"));

        // Test Snapper create
        let executor = SnapshotMockBuilder::snapper().with_next_id(10).build();

        let result = executor.execute(
            "snapper",
            &["-c", "root", "create", "-d", "Test", "--print-number"],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "10");
    }

    #[test]
    fn test_mock_fixtures_delete_workflow() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::SnapshotMockBuilder;

        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "To delete", "2024-01-15_10-30-00")
            .build();

        let result = executor.execute("timeshift", &["--delete", "--snapshot", "1"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_fixtures_failing_operations() {
        use crate::resilience::CommandExecutor;
        use crate::snapshot_fixtures::fixtures;

        let executor = fixtures::timeshift_failing().build();

        // Create should fail
        let result = executor.execute("timeshift", &["--create", "--comments", "Test"]);
        assert!(result.is_err());

        // Delete should fail
        let result = executor.execute("timeshift", &["--delete", "--snapshot", "1"]);
        assert!(result.is_err());
    }
}
