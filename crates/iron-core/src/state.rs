//! State management - Track active configurations

use crate::services::scan::ScanReport;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ==========================================================================
// Update Progress Types (FR-5.10: Partial Update Recovery)
// ==========================================================================

/// Phase of an update operation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UpdatePhase {
    /// Preparing update (checking, downloading)
    #[default]
    Preparing,
    /// Installing packages
    Installing,
    /// Running post-install hooks
    PostInstall,
    /// Update completed successfully
    Completed,
    /// Update was interrupted (detected on restart)
    Interrupted,
    /// Update failed with error
    Failed,
}

/// Record of a successfully updated package
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletedPackage {
    /// Package name
    pub name: String,
    /// Previous version
    pub old_version: String,
    /// New version
    pub new_version: String,
    /// When this package completed
    pub completed_at: DateTime<Utc>,
}

/// Simplified package info for saved update progress
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedPackage {
    /// Package name
    pub name: String,
    /// Current version before update
    pub current_version: String,
    /// Target version
    pub new_version: String,
}

/// Saved update plan for recovery purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedUpdatePlan {
    /// Packages in the update plan
    pub packages: Vec<SavedPackage>,
    /// Whether snapshot was recommended
    pub snapshot_recommended: bool,
    /// When plan was created
    pub created_at: DateTime<Utc>,
}

impl Default for SavedUpdatePlan {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            snapshot_recommended: false,
            created_at: Utc::now(),
        }
    }
}

/// Tracks progress of an ongoing update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgress {
    /// Unique session ID for this update (UUID)
    pub session_id: String,
    /// When the update started
    pub started_at: DateTime<Utc>,
    /// Total packages to update
    pub total_packages: usize,
    /// Packages successfully updated
    pub completed_packages: Vec<CompletedPackage>,
    /// Current phase of the update
    pub phase: UpdatePhase,
    /// Whether a snapshot was created before update
    pub snapshot_created: bool,
    /// Snapshot ID if created
    pub snapshot_id: Option<String>,
    /// The original update plan (simplified for persistence)
    pub plan: SavedUpdatePlan,
    /// Last error message if failed
    pub last_error: Option<String>,
}

impl UpdateProgress {
    /// Create new progress tracker
    pub fn new(plan: SavedUpdatePlan, snapshot_id: Option<String>) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            total_packages: plan.packages.len(),
            completed_packages: Vec::new(),
            phase: UpdatePhase::Preparing,
            snapshot_created: snapshot_id.is_some(),
            snapshot_id,
            plan,
            last_error: None,
        }
    }

    /// Mark a package as completed
    pub fn mark_completed(&mut self, pkg: CompletedPackage) {
        self.completed_packages.push(pkg);
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            return 100.0;
        }
        (self.completed_packages.len() as f64 / self.total_packages as f64) * 100.0
    }

    /// Check if update is incomplete
    pub fn is_incomplete(&self) -> bool {
        matches!(
            self.phase,
            UpdatePhase::Installing | UpdatePhase::Interrupted
        ) && self.completed_packages.len() < self.total_packages
    }

    /// Get remaining packages
    pub fn remaining_packages(&self) -> Vec<&SavedPackage> {
        let completed_names: HashSet<_> = self.completed_packages.iter().map(|p| &p.name).collect();

        self.plan
            .packages
            .iter()
            .filter(|p| !completed_names.contains(&p.name))
            .collect()
    }
}

/// Global Iron state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IronState {
    /// Current host ID
    pub current_host: Option<String>,

    /// Active bundle per host
    pub active_bundles: HashMap<String, String>,

    /// Active profile per host
    pub active_profiles: HashMap<String, String>,

    /// Active modules
    pub active_modules: Vec<String>,

    /// Last operations
    pub last_operations: Vec<OperationRecord>,

    /// Maintenance timestamps
    pub maintenance: MaintenanceState,

    /// Update progress for partial update recovery (FR-5.10)
    /// If present, indicates an update is in progress or was interrupted
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_progress: Option<UpdateProgress>,

    /// News acknowledgment tracking (Phase 2.2)
    #[serde(default)]
    pub news_acknowledgment: NewsAcknowledgment,

    /// Last scan report for scan history / re-scan (S1-P1.5-005)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_scan_report: Option<ScanReport>,

    /// Packages installed by Iron (via apply/module enable).
    /// Only packages in this list are candidates for removal by --prune.
    #[serde(default)]
    pub managed_packages: Vec<String>,

    /// Services enabled by Iron (via apply).
    /// Only services in this list are candidates for disabling by --prune.
    #[serde(default)]
    pub managed_services: Vec<String>,

    /// Dotfile target paths created by Iron (via apply).
    /// Only dotfiles in this list are candidates for removal by --prune.
    #[serde(default)]
    pub managed_dotfiles: Vec<String>,

    /// Timestamp of last successful apply execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_apply: Option<DateTime<Utc>>,

    /// Tracks which "Once" hooks have been executed per module.
    /// Key: module_id, Value: list of hook types that have run
    /// (e.g., ["post_install"])
    #[serde(default)]
    pub hooks_executed: HashMap<String, Vec<String>>,
}

/// Record of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub status: OperationStatus,
    pub details: Option<String>,

    /// Duration of the operation in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,

    /// Number of actions in the operation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_count: Option<usize>,
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Success,
    Failed,
    Partial,
    Skipped,
}

/// Maintenance operation timestamps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaintenanceState {
    pub last_update: Option<DateTime<Utc>>,
    pub last_clean: Option<DateTime<Utc>>,
    pub last_doctor: Option<DateTime<Utc>>,
    pub last_snapshot: Option<DateTime<Utc>>,
    pub last_sync: Option<DateTime<Utc>>,
}

// ==========================================================================
// News Acknowledgment (Phase 2.2)
// ==========================================================================

/// Tracks acknowledged Arch Linux news items
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewsAcknowledgment {
    /// URLs of acknowledged news items (used as unique identifiers)
    pub acknowledged_urls: HashSet<String>,
    /// Timestamp of last news fetch
    pub last_fetch: Option<DateTime<Utc>>,
    /// Timestamp of last acknowledgment
    pub last_acknowledged: Option<DateTime<Utc>>,
}

impl NewsAcknowledgment {
    /// Create a new empty acknowledgment tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a news item has been acknowledged
    pub fn is_acknowledged(&self, url: &str) -> bool {
        self.acknowledged_urls.contains(url)
    }

    /// Acknowledge a news item
    pub fn acknowledge(&mut self, url: &str) {
        self.acknowledged_urls.insert(url.to_string());
        self.last_acknowledged = Some(Utc::now());
    }

    /// Acknowledge multiple news items
    pub fn acknowledge_all(&mut self, urls: &[&str]) {
        for url in urls {
            self.acknowledged_urls.insert((*url).to_string());
        }
        if !urls.is_empty() {
            self.last_acknowledged = Some(Utc::now());
        }
    }

    /// Mark news as fetched
    pub fn mark_fetched(&mut self) {
        self.last_fetch = Some(Utc::now());
    }

    /// Get count of acknowledged items
    pub fn acknowledged_count(&self) -> usize {
        self.acknowledged_urls.len()
    }

    /// Clear old acknowledgments (keep last 100)
    pub fn prune(&mut self, keep: usize) {
        if self.acknowledged_urls.len() > keep {
            // Since HashSet doesn't have ordering, we just keep it as-is
            // In practice, old news URLs won't match new ones anyway
        }
    }
}

impl IronState {
    /// Load state from file
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let state: IronState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::default())
        }
    }

    /// Save state to file using an atomic write (write → fsync → rename).
    ///
    /// Atomic rename prevents a corrupt `state.json` if the process is killed
    /// or power is lost mid-write. The temporary file is placed next to the
    /// target so that the rename stays on the same filesystem (required for
    /// atomic behaviour on Linux).
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        use std::io::Write;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;

        let temp_path = path.with_extension("iron-tmp");
        {
            let mut file = std::fs::File::create(&temp_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }

        std::fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Record an operation
    pub fn record_operation(
        &mut self,
        operation: &str,
        status: OperationStatus,
        details: Option<String>,
    ) {
        self.last_operations.push(OperationRecord {
            operation: operation.to_string(),
            timestamp: Utc::now(),
            status,
            details,
            duration_secs: None,
            action_count: None,
        });

        // Keep only last 100 operations
        if self.last_operations.len() > 100 {
            self.last_operations = self
                .last_operations
                .split_off(self.last_operations.len() - 100);
        }
    }

    // ==========================================================================
    // News Acknowledgment Methods (Phase 2.2)
    // ==========================================================================

    /// Check if a news item has been acknowledged
    pub fn is_news_acknowledged(&self, url: &str) -> bool {
        self.news_acknowledgment.is_acknowledged(url)
    }

    /// Acknowledge a news item by URL
    pub fn acknowledge_news(&mut self, url: &str) {
        self.news_acknowledgment.acknowledge(url);
    }

    /// Acknowledge multiple news items
    pub fn acknowledge_all_news(&mut self, urls: &[&str]) {
        self.news_acknowledgment.acknowledge_all(urls);
    }

    /// Mark news as recently fetched
    pub fn mark_news_fetched(&mut self) {
        self.news_acknowledgment.mark_fetched();
    }

    /// Get the time since last news fetch
    pub fn time_since_news_fetch(&self) -> Option<chrono::Duration> {
        self.news_acknowledgment
            .last_fetch
            .map(|t| Utc::now().signed_duration_since(t))
    }

    /// Check if news should be refetched (default: older than 1 hour)
    pub fn should_refetch_news(&self) -> bool {
        match self.time_since_news_fetch() {
            Some(duration) => duration.num_hours() >= 1,
            None => true, // Never fetched
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ==========================================================================
    // OperationStatus Tests
    // ==========================================================================

    #[test]
    fn test_operation_status_debug() {
        let statuses = vec![
            OperationStatus::Success,
            OperationStatus::Failed,
            OperationStatus::Partial,
            OperationStatus::Skipped,
        ];

        for status in statuses {
            let debug_str = format!("{:?}", status);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_operation_status_clone() {
        let status = OperationStatus::Success;
        let cloned = status.clone();
        match cloned {
            OperationStatus::Success => {}
            _ => panic!("Clone should preserve status"),
        }
    }

    #[test]
    fn test_operation_status_serialization() {
        let status = OperationStatus::Failed;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: OperationStatus = serde_json::from_str(&json).unwrap();
        match deserialized {
            OperationStatus::Failed => {}
            _ => panic!("Deserialization should preserve status"),
        }
    }

    // ==========================================================================
    // OperationRecord Tests
    // ==========================================================================

    #[test]
    fn test_operation_record_creation() {
        let record = OperationRecord {
            operation: "test_op".to_string(),
            timestamp: Utc::now(),
            status: OperationStatus::Success,
            details: Some("Test details".to_string()),
            duration_secs: None,
            action_count: None,
        };

        assert_eq!(record.operation, "test_op");
        assert!(record.details.is_some());
    }

    #[test]
    fn test_operation_record_without_details() {
        let record = OperationRecord {
            operation: "simple_op".to_string(),
            timestamp: Utc::now(),
            status: OperationStatus::Partial,
            details: None,
            duration_secs: None,
            action_count: None,
        };

        assert!(record.details.is_none());
    }

    #[test]
    fn test_operation_record_clone() {
        let record = OperationRecord {
            operation: "clone_test".to_string(),
            timestamp: Utc::now(),
            status: OperationStatus::Skipped,
            details: Some("Cloned".to_string()),
            duration_secs: None,
            action_count: None,
        };

        let cloned = record.clone();
        assert_eq!(cloned.operation, "clone_test");
    }

    #[test]
    fn test_operation_record_serialization() {
        let record = OperationRecord {
            operation: "serialize_test".to_string(),
            timestamp: Utc::now(),
            status: OperationStatus::Success,
            details: None,
            duration_secs: None,
            action_count: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: OperationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.operation, "serialize_test");
    }

    // ==========================================================================
    // MaintenanceState Tests
    // ==========================================================================

    #[test]
    fn test_maintenance_state_default() {
        let state = MaintenanceState::default();

        assert!(state.last_update.is_none());
        assert!(state.last_clean.is_none());
        assert!(state.last_doctor.is_none());
        assert!(state.last_snapshot.is_none());
        assert!(state.last_sync.is_none());
    }

    #[test]
    fn test_maintenance_state_with_values() {
        let now = Utc::now();
        let state = MaintenanceState {
            last_update: Some(now),
            last_clean: Some(now),
            last_doctor: None,
            last_snapshot: Some(now),
            last_sync: None,
        };

        assert!(state.last_update.is_some());
        assert!(state.last_clean.is_some());
        assert!(state.last_doctor.is_none());
    }

    #[test]
    fn test_maintenance_state_clone() {
        let now = Utc::now();
        let state = MaintenanceState {
            last_update: Some(now),
            last_clean: None,
            last_doctor: None,
            last_snapshot: None,
            last_sync: None,
        };

        let cloned = state.clone();
        assert!(cloned.last_update.is_some());
    }

    #[test]
    fn test_maintenance_state_serialization() {
        let state = MaintenanceState {
            last_update: Some(Utc::now()),
            last_clean: None,
            last_doctor: None,
            last_snapshot: None,
            last_sync: None,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: MaintenanceState = serde_json::from_str(&json).unwrap();
        assert!(deserialized.last_update.is_some());
    }

    // ==========================================================================
    // IronState Tests
    // ==========================================================================

    #[test]
    fn test_iron_state_default() {
        let state = IronState::default();

        assert!(state.current_host.is_none());
        assert!(state.active_bundles.is_empty());
        assert!(state.active_profiles.is_empty());
        assert!(state.active_modules.is_empty());
        assert!(state.last_operations.is_empty());
    }

    #[test]
    fn test_iron_state_with_host() {
        let mut state = IronState::default();
        state.current_host = Some("desktop".to_string());

        assert_eq!(state.current_host, Some("desktop".to_string()));
    }

    #[test]
    fn test_iron_state_with_bundles() {
        let mut state = IronState::default();
        state
            .active_bundles
            .insert("desktop".to_string(), "hyprland".to_string());
        state
            .active_bundles
            .insert("laptop".to_string(), "niri".to_string());

        assert_eq!(state.active_bundles.len(), 2);
        assert_eq!(
            state.active_bundles.get("desktop"),
            Some(&"hyprland".to_string())
        );
    }

    #[test]
    fn test_iron_state_with_profiles() {
        let mut state = IronState::default();
        state
            .active_profiles
            .insert("desktop".to_string(), "developer".to_string());

        assert_eq!(state.active_profiles.len(), 1);
        assert_eq!(
            state.active_profiles.get("desktop"),
            Some(&"developer".to_string())
        );
    }

    #[test]
    fn test_iron_state_with_modules() {
        let mut state = IronState::default();
        state.active_modules.push("nvim-ide".to_string());
        state.active_modules.push("kitty-dev".to_string());

        assert_eq!(state.active_modules.len(), 2);
        assert!(state.active_modules.contains(&"nvim-ide".to_string()));
    }

    #[test]
    fn test_iron_state_clone() {
        let mut state = IronState::default();
        state.current_host = Some("test".to_string());
        state.active_modules.push("module1".to_string());

        let cloned = state.clone();
        assert_eq!(cloned.current_host, Some("test".to_string()));
        assert_eq!(cloned.active_modules.len(), 1);
    }

    #[test]
    fn test_iron_state_record_operation() {
        let mut state = IronState::default();

        state.record_operation(
            "test_op",
            OperationStatus::Success,
            Some("Details".to_string()),
        );

        assert_eq!(state.last_operations.len(), 1);
        assert_eq!(state.last_operations[0].operation, "test_op");
    }

    #[test]
    fn test_iron_state_record_multiple_operations() {
        let mut state = IronState::default();

        state.record_operation("op1", OperationStatus::Success, None);
        state.record_operation("op2", OperationStatus::Failed, None);
        state.record_operation("op3", OperationStatus::Partial, None);

        assert_eq!(state.last_operations.len(), 3);
    }

    #[test]
    fn test_iron_state_operation_limit() {
        let mut state = IronState::default();

        // Add 110 operations
        for i in 0..110 {
            state.record_operation(&format!("op_{}", i), OperationStatus::Success, None);
        }

        // Should only keep last 100
        assert_eq!(state.last_operations.len(), 100);
        // First operation should be op_10 (0-9 were removed)
        assert_eq!(state.last_operations[0].operation, "op_10");
    }

    // ==========================================================================
    // IronState Load/Save Tests
    // ==========================================================================

    #[test]
    fn test_iron_state_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json");

        let state = IronState::load(&path).unwrap();

        // Should return default state for nonexistent file
        assert!(state.current_host.is_none());
    }

    #[test]
    fn test_iron_state_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        let mut state = IronState::default();
        state.current_host = Some("test-host".to_string());
        state.active_modules.push("test-module".to_string());

        // Save
        state.save(&path).unwrap();

        // Load
        let loaded = IronState::load(&path).unwrap();

        assert_eq!(loaded.current_host, Some("test-host".to_string()));
        assert_eq!(loaded.active_modules.len(), 1);
    }

    #[test]
    fn test_iron_state_save_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("state.json");

        let state = IronState::default();
        state.save(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_iron_state_serialization() {
        let mut state = IronState::default();
        state.current_host = Some("serialize-test".to_string());
        state
            .active_bundles
            .insert("host".to_string(), "bundle".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: IronState = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.current_host,
            Some("serialize-test".to_string())
        );
        assert_eq!(
            deserialized.active_bundles.get("host"),
            Some(&"bundle".to_string())
        );
    }

    #[test]
    fn test_iron_state_full_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("roundtrip.json");

        let mut state = IronState::default();
        state.current_host = Some("roundtrip".to_string());
        state
            .active_bundles
            .insert("host1".to_string(), "bundle1".to_string());
        state
            .active_profiles
            .insert("host1".to_string(), "profile1".to_string());
        state.active_modules.push("module1".to_string());
        state.record_operation("test", OperationStatus::Success, None);
        state.maintenance.last_update = Some(Utc::now());

        // Save and reload
        state.save(&path).unwrap();
        let loaded = IronState::load(&path).unwrap();

        assert_eq!(loaded.current_host, state.current_host);
        assert_eq!(loaded.active_bundles.len(), 1);
        assert_eq!(loaded.active_profiles.len(), 1);
        assert_eq!(loaded.active_modules.len(), 1);
        assert_eq!(loaded.last_operations.len(), 1);
        assert!(loaded.maintenance.last_update.is_some());
    }

    // ==========================================================================
    // UpdatePhase Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_update_phase_default() {
        let phase = UpdatePhase::default();
        assert_eq!(phase, UpdatePhase::Preparing);
    }

    #[test]
    fn test_update_phase_all_variants() {
        let phases = vec![
            UpdatePhase::Preparing,
            UpdatePhase::Installing,
            UpdatePhase::PostInstall,
            UpdatePhase::Completed,
            UpdatePhase::Interrupted,
            UpdatePhase::Failed,
        ];

        for phase in phases {
            let debug_str = format!("{:?}", phase);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_update_phase_serialization() {
        let phases = vec![
            (UpdatePhase::Preparing, "\"Preparing\""),
            (UpdatePhase::Installing, "\"Installing\""),
            (UpdatePhase::PostInstall, "\"PostInstall\""),
            (UpdatePhase::Completed, "\"Completed\""),
            (UpdatePhase::Interrupted, "\"Interrupted\""),
            (UpdatePhase::Failed, "\"Failed\""),
        ];

        for (phase, expected) in phases {
            let json = serde_json::to_string(&phase).unwrap();
            assert_eq!(json, expected);

            let deserialized: UpdatePhase = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, phase);
        }
    }

    #[test]
    fn test_update_phase_clone_copy() {
        let phase = UpdatePhase::Installing;
        let cloned = phase.clone();
        let copied = phase; // Copy trait

        assert_eq!(cloned, UpdatePhase::Installing);
        assert_eq!(copied, UpdatePhase::Installing);
    }

    // ==========================================================================
    // CompletedPackage Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_completed_package_creation() {
        let now = Utc::now();
        let pkg = CompletedPackage {
            name: "linux".to_string(),
            old_version: "6.17.0".to_string(),
            new_version: "6.18.0".to_string(),
            completed_at: now,
        };

        assert_eq!(pkg.name, "linux");
        assert_eq!(pkg.old_version, "6.17.0");
        assert_eq!(pkg.new_version, "6.18.0");
        assert_eq!(pkg.completed_at, now);
    }

    #[test]
    fn test_completed_package_serialization() {
        let pkg = CompletedPackage {
            name: "neovim".to_string(),
            old_version: "0.9.0".to_string(),
            new_version: "0.10.0".to_string(),
            completed_at: Utc::now(),
        };

        let json = serde_json::to_string(&pkg).unwrap();
        let deserialized: CompletedPackage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "neovim");
        assert_eq!(deserialized.old_version, "0.9.0");
        assert_eq!(deserialized.new_version, "0.10.0");
    }

    #[test]
    fn test_completed_package_clone() {
        let pkg = CompletedPackage {
            name: "git".to_string(),
            old_version: "2.44.0".to_string(),
            new_version: "2.45.0".to_string(),
            completed_at: Utc::now(),
        };

        let cloned = pkg.clone();
        assert_eq!(cloned.name, "git");
    }

    // ==========================================================================
    // SavedPackage Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_saved_package_creation() {
        let pkg = SavedPackage {
            name: "firefox".to_string(),
            current_version: "125.0".to_string(),
            new_version: "126.0".to_string(),
        };

        assert_eq!(pkg.name, "firefox");
        assert_eq!(pkg.current_version, "125.0");
        assert_eq!(pkg.new_version, "126.0");
    }

    #[test]
    fn test_saved_package_serialization() {
        let pkg = SavedPackage {
            name: "rust".to_string(),
            current_version: "1.77.0".to_string(),
            new_version: "1.78.0".to_string(),
        };

        let json = serde_json::to_string(&pkg).unwrap();
        let deserialized: SavedPackage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "rust");
        assert_eq!(deserialized.current_version, "1.77.0");
        assert_eq!(deserialized.new_version, "1.78.0");
    }

    // ==========================================================================
    // SavedUpdatePlan Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_saved_update_plan_default() {
        let plan = SavedUpdatePlan::default();

        assert!(plan.packages.is_empty());
        assert!(!plan.snapshot_recommended);
    }

    #[test]
    fn test_saved_update_plan_with_packages() {
        let plan = SavedUpdatePlan {
            packages: vec![
                SavedPackage {
                    name: "linux".to_string(),
                    current_version: "6.17.0".to_string(),
                    new_version: "6.18.0".to_string(),
                },
                SavedPackage {
                    name: "nvidia".to_string(),
                    current_version: "550.0".to_string(),
                    new_version: "555.0".to_string(),
                },
            ],
            snapshot_recommended: true,
            created_at: Utc::now(),
        };

        assert_eq!(plan.packages.len(), 2);
        assert!(plan.snapshot_recommended);
    }

    #[test]
    fn test_saved_update_plan_serialization() {
        let plan = SavedUpdatePlan {
            packages: vec![SavedPackage {
                name: "mesa".to_string(),
                current_version: "24.0".to_string(),
                new_version: "24.1".to_string(),
            }],
            snapshot_recommended: false,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: SavedUpdatePlan = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.packages.len(), 1);
        assert_eq!(deserialized.packages[0].name, "mesa");
    }

    // ==========================================================================
    // UpdateProgress Tests (FR-5.10)
    // ==========================================================================

    fn create_test_plan() -> SavedUpdatePlan {
        SavedUpdatePlan {
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
                SavedPackage {
                    name: "pkg3".to_string(),
                    current_version: "1.0".to_string(),
                    new_version: "2.0".to_string(),
                },
            ],
            snapshot_recommended: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_update_progress_new() {
        let plan = create_test_plan();
        let progress = UpdateProgress::new(plan, Some("snap-123".to_string()));

        assert!(!progress.session_id.is_empty());
        assert_eq!(progress.total_packages, 3);
        assert!(progress.completed_packages.is_empty());
        assert_eq!(progress.phase, UpdatePhase::Preparing);
        assert!(progress.snapshot_created);
        assert_eq!(progress.snapshot_id, Some("snap-123".to_string()));
        assert!(progress.last_error.is_none());
    }

    #[test]
    fn test_update_progress_new_without_snapshot() {
        let plan = create_test_plan();
        let progress = UpdateProgress::new(plan, None);

        assert!(!progress.snapshot_created);
        assert!(progress.snapshot_id.is_none());
    }

    #[test]
    fn test_update_progress_mark_completed() {
        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, None);

        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        assert_eq!(progress.completed_packages.len(), 1);
        assert_eq!(progress.completed_packages[0].name, "pkg1");
    }

    #[test]
    fn test_update_progress_completion_percentage() {
        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, None);

        // 0% initially
        assert_eq!(progress.completion_percentage(), 0.0);

        // Mark one complete (1/3 = 33.33%)
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert!((progress.completion_percentage() - 33.333).abs() < 0.01);

        // Mark another (2/3 = 66.67%)
        progress.mark_completed(CompletedPackage {
            name: "pkg2".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert!((progress.completion_percentage() - 66.667).abs() < 0.01);

        // Mark all complete (100%)
        progress.mark_completed(CompletedPackage {
            name: "pkg3".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert_eq!(progress.completion_percentage(), 100.0);
    }

    #[test]
    fn test_update_progress_completion_percentage_empty_plan() {
        let plan = SavedUpdatePlan::default();
        let progress = UpdateProgress::new(plan, None);

        // Empty plan should show 100%
        assert_eq!(progress.completion_percentage(), 100.0);
    }

    #[test]
    fn test_update_progress_is_incomplete() {
        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, None);

        // Not incomplete in Preparing phase
        assert!(!progress.is_incomplete());

        // Move to Installing phase
        progress.phase = UpdatePhase::Installing;
        assert!(progress.is_incomplete()); // 0 of 3 completed

        // Mark one complete - still incomplete
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert!(progress.is_incomplete()); // 1 of 3 completed

        // Mark all complete
        progress.mark_completed(CompletedPackage {
            name: "pkg2".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        progress.mark_completed(CompletedPackage {
            name: "pkg3".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        assert!(!progress.is_incomplete()); // All completed

        // Interrupted phase is also incomplete
        progress.phase = UpdatePhase::Interrupted;
        progress.completed_packages.pop(); // Remove one to make it incomplete again
        assert!(progress.is_incomplete());
    }

    #[test]
    fn test_update_progress_remaining_packages() {
        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, None);

        // All packages remaining
        let remaining = progress.remaining_packages();
        assert_eq!(remaining.len(), 3);

        // Mark one complete
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        let remaining = progress.remaining_packages();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().all(|p| p.name != "pkg1"));

        // Mark another
        progress.mark_completed(CompletedPackage {
            name: "pkg3".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        let remaining = progress.remaining_packages();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "pkg2");
    }

    #[test]
    fn test_update_progress_serialization() {
        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, Some("snap-456".to_string()));
        progress.phase = UpdatePhase::Installing;
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });

        let json = serde_json::to_string(&progress).unwrap();
        let deserialized: UpdateProgress = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.session_id, progress.session_id);
        assert_eq!(deserialized.total_packages, 3);
        assert_eq!(deserialized.completed_packages.len(), 1);
        assert_eq!(deserialized.phase, UpdatePhase::Installing);
        assert!(deserialized.snapshot_created);
    }

    // ==========================================================================
    // IronState with UpdateProgress Tests (FR-5.10)
    // ==========================================================================

    #[test]
    fn test_iron_state_default_no_update_progress() {
        let state = IronState::default();
        assert!(state.update_progress.is_none());
    }

    #[test]
    fn test_iron_state_with_update_progress() {
        let mut state = IronState::default();
        let plan = create_test_plan();
        state.update_progress = Some(UpdateProgress::new(plan, None));

        assert!(state.update_progress.is_some());
        let progress = state.update_progress.as_ref().unwrap();
        assert_eq!(progress.total_packages, 3);
    }

    #[test]
    fn test_iron_state_save_load_with_update_progress() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state_with_progress.json");

        let mut state = IronState::default();
        state.current_host = Some("test".to_string());

        let plan = create_test_plan();
        let mut progress = UpdateProgress::new(plan, Some("snap-789".to_string()));
        progress.phase = UpdatePhase::Installing;
        progress.mark_completed(CompletedPackage {
            name: "pkg1".to_string(),
            old_version: "1.0".to_string(),
            new_version: "2.0".to_string(),
            completed_at: Utc::now(),
        });
        state.update_progress = Some(progress);

        // Save and reload
        state.save(&path).unwrap();
        let loaded = IronState::load(&path).unwrap();

        assert!(loaded.update_progress.is_some());
        let loaded_progress = loaded.update_progress.as_ref().unwrap();
        assert_eq!(loaded_progress.total_packages, 3);
        assert_eq!(loaded_progress.completed_packages.len(), 1);
        assert_eq!(loaded_progress.phase, UpdatePhase::Installing);
        assert!(loaded_progress.snapshot_created);
    }

    #[test]
    fn test_iron_state_backward_compatibility_no_update_progress() {
        // Simulate loading an old state file that doesn't have update_progress field
        let old_state_json = r#"{
            "current_host": "legacy-host",
            "active_bundles": {},
            "active_profiles": {},
            "active_modules": [],
            "last_operations": [],
            "maintenance": {
                "last_update": null,
                "last_clean": null,
                "last_doctor": null,
                "last_snapshot": null,
                "last_sync": null
            }
        }"#;

        let state: IronState = serde_json::from_str(old_state_json).unwrap();

        assert_eq!(state.current_host, Some("legacy-host".to_string()));
        assert!(state.update_progress.is_none()); // Should default to None
    }

    #[test]
    fn test_iron_state_serialization_skips_none_update_progress() {
        let state = IronState::default();
        let json = serde_json::to_string_pretty(&state).unwrap();

        // The update_progress field should be skipped when None (due to skip_serializing_if)
        assert!(!json.contains("update_progress"));
    }

    #[test]
    fn test_iron_state_clear_update_progress() {
        let mut state = IronState::default();
        let plan = create_test_plan();
        state.update_progress = Some(UpdateProgress::new(plan, None));

        assert!(state.update_progress.is_some());

        // Clear progress
        state.update_progress = None;
        assert!(state.update_progress.is_none());
    }

    // ==========================================================================
    // NewsAcknowledgment Tests (Phase 2.2)
    // ==========================================================================

    #[test]
    fn test_news_acknowledgment_new() {
        let ack = NewsAcknowledgment::new();
        assert!(ack.acknowledged_urls.is_empty());
        assert!(ack.last_fetch.is_none());
        assert!(ack.last_acknowledged.is_none());
    }

    #[test]
    fn test_news_acknowledgment_default() {
        let ack = NewsAcknowledgment::default();
        assert!(ack.acknowledged_urls.is_empty());
    }

    #[test]
    fn test_news_acknowledgment_is_acknowledged() {
        let mut ack = NewsAcknowledgment::new();
        let url = "https://archlinux.org/news/test/";

        assert!(!ack.is_acknowledged(url));
        ack.acknowledge(url);
        assert!(ack.is_acknowledged(url));
    }

    #[test]
    fn test_news_acknowledgment_acknowledge() {
        let mut ack = NewsAcknowledgment::new();
        let url = "https://archlinux.org/news/test/";

        ack.acknowledge(url);

        assert_eq!(ack.acknowledged_count(), 1);
        assert!(ack.last_acknowledged.is_some());
    }

    #[test]
    fn test_news_acknowledgment_acknowledge_all() {
        let mut ack = NewsAcknowledgment::new();
        let urls = [
            "https://archlinux.org/news/1/",
            "https://archlinux.org/news/2/",
            "https://archlinux.org/news/3/",
        ];

        ack.acknowledge_all(&urls);

        assert_eq!(ack.acknowledged_count(), 3);
        assert!(ack.is_acknowledged(urls[0]));
        assert!(ack.is_acknowledged(urls[1]));
        assert!(ack.is_acknowledged(urls[2]));
    }

    #[test]
    fn test_news_acknowledgment_acknowledge_all_empty() {
        let mut ack = NewsAcknowledgment::new();
        ack.acknowledge_all(&[]);

        assert_eq!(ack.acknowledged_count(), 0);
        assert!(ack.last_acknowledged.is_none());
    }

    #[test]
    fn test_news_acknowledgment_mark_fetched() {
        let mut ack = NewsAcknowledgment::new();
        assert!(ack.last_fetch.is_none());

        ack.mark_fetched();

        assert!(ack.last_fetch.is_some());
    }

    #[test]
    fn test_news_acknowledgment_acknowledged_count() {
        let mut ack = NewsAcknowledgment::new();
        assert_eq!(ack.acknowledged_count(), 0);

        ack.acknowledge("url1");
        assert_eq!(ack.acknowledged_count(), 1);

        ack.acknowledge("url2");
        assert_eq!(ack.acknowledged_count(), 2);

        // Duplicate should not increase count (HashSet)
        ack.acknowledge("url1");
        assert_eq!(ack.acknowledged_count(), 2);
    }

    #[test]
    fn test_news_acknowledgment_serialization() {
        let mut ack = NewsAcknowledgment::new();
        ack.acknowledge("https://archlinux.org/news/test/");
        ack.mark_fetched();

        let json = serde_json::to_string(&ack).unwrap();
        let deserialized: NewsAcknowledgment = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.acknowledged_count(), 1);
        assert!(deserialized.is_acknowledged("https://archlinux.org/news/test/"));
        assert!(deserialized.last_fetch.is_some());
    }

    #[test]
    fn test_news_acknowledgment_clone() {
        let mut ack = NewsAcknowledgment::new();
        ack.acknowledge("url1");
        ack.mark_fetched();

        let cloned = ack.clone();

        assert_eq!(cloned.acknowledged_count(), 1);
        assert!(cloned.is_acknowledged("url1"));
    }

    // ==========================================================================
    // IronState News Methods Tests (Phase 2.2)
    // ==========================================================================

    #[test]
    fn test_iron_state_is_news_acknowledged() {
        let mut state = IronState::default();
        let url = "https://archlinux.org/news/test/";

        assert!(!state.is_news_acknowledged(url));

        state.acknowledge_news(url);
        assert!(state.is_news_acknowledged(url));
    }

    #[test]
    fn test_iron_state_acknowledge_news() {
        let mut state = IronState::default();
        let url = "https://archlinux.org/news/important/";

        state.acknowledge_news(url);

        assert!(state.news_acknowledgment.is_acknowledged(url));
        assert!(state.news_acknowledgment.last_acknowledged.is_some());
    }

    #[test]
    fn test_iron_state_acknowledge_all_news() {
        let mut state = IronState::default();
        let urls = [
            "https://archlinux.org/news/1/",
            "https://archlinux.org/news/2/",
        ];

        state.acknowledge_all_news(&urls);

        assert!(state.is_news_acknowledged(urls[0]));
        assert!(state.is_news_acknowledged(urls[1]));
    }

    #[test]
    fn test_iron_state_mark_news_fetched() {
        let mut state = IronState::default();
        assert!(state.news_acknowledgment.last_fetch.is_none());

        state.mark_news_fetched();

        assert!(state.news_acknowledgment.last_fetch.is_some());
    }

    #[test]
    fn test_iron_state_time_since_news_fetch_none() {
        let state = IronState::default();
        assert!(state.time_since_news_fetch().is_none());
    }

    #[test]
    fn test_iron_state_time_since_news_fetch_some() {
        let mut state = IronState::default();
        state.mark_news_fetched();

        let duration = state.time_since_news_fetch();
        assert!(duration.is_some());
        // Should be very recent (less than 1 second)
        assert!(duration.unwrap().num_seconds() < 1);
    }

    #[test]
    fn test_iron_state_should_refetch_news_never_fetched() {
        let state = IronState::default();
        assert!(state.should_refetch_news()); // Never fetched = should refetch
    }

    #[test]
    fn test_iron_state_should_refetch_news_recent() {
        let mut state = IronState::default();
        state.mark_news_fetched();

        assert!(!state.should_refetch_news()); // Just fetched = should not refetch
    }

    #[test]
    fn test_iron_state_news_acknowledgment_persists_in_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("news_test.json");

        let mut state = IronState::default();
        state.acknowledge_news("https://archlinux.org/news/persisted/");
        state.mark_news_fetched();

        state.save(&path).unwrap();
        let loaded = IronState::load(&path).unwrap();

        assert!(loaded.is_news_acknowledged("https://archlinux.org/news/persisted/"));
        assert!(loaded.news_acknowledgment.last_fetch.is_some());
    }

    #[test]
    fn test_iron_state_backward_compatibility_no_news_acknowledgment() {
        // Simulate loading an old state file without news_acknowledgment field
        let old_state_json = r#"{
            "current_host": "legacy-host",
            "active_bundles": {},
            "active_profiles": {},
            "active_modules": [],
            "last_operations": [],
            "maintenance": {
                "last_update": null,
                "last_clean": null,
                "last_doctor": null,
                "last_snapshot": null,
                "last_sync": null
            }
        }"#;

        let state: IronState = serde_json::from_str(old_state_json).unwrap();

        // Should default to empty news acknowledgment
        assert_eq!(state.news_acknowledgment.acknowledged_count(), 0);
        assert!(state.news_acknowledgment.last_fetch.is_none());
    }
}
