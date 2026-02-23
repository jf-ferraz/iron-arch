//! Snapshot Service — Named snapshots for safe experimentation
//!
//! F2-001: SnapshotService trait + SnapshotRecord model
//! F2-006: Per-module rollback support
//!
//! Stores snapshots as JSON files in `$IRON_ROOT/.snapshots/`.

use crate::IronResult;
use crate::packages::PackageManager;
use crate::services::state::StateManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Directory name for snapshot storage
const SNAPSHOTS_DIR: &str = ".snapshots";

/// Default maximum auto-snapshots to keep
pub const DEFAULT_AUTO_KEEP: usize = 10;

// ==========================================================================
// F2-001: SnapshotRecord model
// ==========================================================================

/// A snapshot captures system state at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotRecord {
    /// Unique ID (UUID v4)
    pub id: String,

    /// Human-readable name ("pre-kde", "backup-2026-02-22")
    pub name: String,

    /// When the snapshot was created
    pub timestamp: DateTime<Utc>,

    /// Host this snapshot belongs to
    #[serde(default)]
    pub host_id: Option<String>,

    /// Active bundle at snapshot time
    #[serde(default)]
    pub active_bundle: Option<String>,

    /// Active profile at snapshot time
    #[serde(default)]
    pub active_profile: Option<String>,

    /// All active module IDs at snapshot time
    #[serde(default)]
    pub active_modules: Vec<String>,

    /// Explicitly installed packages at snapshot time
    #[serde(default)]
    pub explicit_packages: Vec<String>,

    /// Dotfile checksums: target_path -> sha256
    #[serde(default)]
    pub dotfile_checksums: HashMap<String, String>,

    /// Whether this was auto-created (vs user-created)
    #[serde(default)]
    pub auto: bool,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

impl SnapshotRecord {
    /// Create a new SnapshotRecord with generated UUID and current timestamp.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }

    /// Summary line for display
    pub fn summary(&self) -> String {
        let auto_tag = if self.auto { " [auto]" } else { "" };
        format!(
            "{} — {} ({} modules, {} packages){}",
            self.name,
            self.timestamp.format("%Y-%m-%d %H:%M"),
            self.active_modules.len(),
            self.explicit_packages.len(),
            auto_tag,
        )
    }
}

// ==========================================================================
// F2-001: SnapshotService trait
// ==========================================================================

/// Service for managing named snapshots.
pub trait SnapshotService: Send + Sync {
    /// Create a named snapshot of current state
    fn create(&self, name: &str, description: Option<&str>) -> IronResult<SnapshotRecord>;

    /// Create auto-snapshot (named with timestamp, auto=true)
    fn create_auto(&self, prefix: &str) -> IronResult<SnapshotRecord>;

    /// List all snapshots, newest first
    fn list(&self) -> IronResult<Vec<SnapshotRecord>>;

    /// Get a specific snapshot by name or ID
    fn get(&self, name_or_id: &str) -> IronResult<SnapshotRecord>;

    /// Delete a snapshot
    fn delete(&self, name_or_id: &str) -> IronResult<()>;

    /// Prune old auto-snapshots, keeping at most `keep` recent ones.
    /// Returns number of snapshots pruned.
    fn prune_auto(&self, keep: usize) -> IronResult<usize>;
}

// ==========================================================================
// F2-001: DefaultSnapshotService
// ==========================================================================

/// Default implementation storing snapshots as JSON in `.snapshots/`.
pub struct DefaultSnapshotService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Option<Arc<dyn PackageManager>>,
    cache: std::sync::Mutex<Option<Vec<SnapshotRecord>>>,
}

impl DefaultSnapshotService {
    pub fn new(iron_root: &Path, state_manager: StateManager) -> Self {
        Self {
            iron_root: iron_root.to_path_buf(),
            state_manager,
            package_manager: None,
            cache: std::sync::Mutex::new(None),
        }
    }

    pub fn with_package_manager(mut self, pm: Arc<dyn PackageManager>) -> Self {
        self.package_manager = Some(pm);
        self
    }

    /// Get the snapshots directory, creating it if necessary.
    fn snapshots_dir(&self) -> IronResult<PathBuf> {
        let dir = self.iron_root.join(SNAPSHOTS_DIR);
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| crate::FsError::IoError {
                message: format!("Failed to create snapshots directory: {}", e),
            })?;
        }
        Ok(dir)
    }

    /// Path to a snapshot file by ID.
    fn snapshot_path(&self, id: &str) -> IronResult<PathBuf> {
        Ok(self.snapshots_dir()?.join(format!("snap-{}.json", id)))
    }

    /// Capture current system state into a SnapshotRecord.
    fn capture_state(
        &self,
        name: &str,
        description: Option<&str>,
        auto: bool,
    ) -> IronResult<SnapshotRecord> {
        let host_id = self.state_manager.current_host();
        let active_modules = self.state_manager.active_modules();

        // Get active bundle/profile from state
        let active_bundle = host_id
            .as_ref()
            .and_then(|h| self.state_manager.active_bundle(h));
        let active_profile = host_id
            .as_ref()
            .and_then(|h| self.state_manager.active_profile(h));

        // Get explicit packages if package manager available
        let explicit_packages = self
            .package_manager
            .as_ref()
            .and_then(|pm| pm.query_installed().ok())
            .map(|pkgs| pkgs.into_iter().map(|p| p.name).collect())
            .unwrap_or_default();

        Ok(SnapshotRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            timestamp: Utc::now(),
            host_id,
            active_bundle,
            active_profile,
            active_modules,
            explicit_packages,
            dotfile_checksums: HashMap::new(),
            auto,
            description: description.map(String::from),
        })
    }

    /// Save a snapshot record to disk.
    fn save_record(&self, record: &SnapshotRecord) -> IronResult<()> {
        let path = self.snapshot_path(&record.id)?;
        let json = serde_json::to_string_pretty(record).map_err(|e| crate::FsError::IoError {
            message: format!("Failed to serialize snapshot: {}", e),
        })?;
        fs::write(&path, json).map_err(|e| crate::FsError::IoError {
            message: format!("Failed to write snapshot {}: {}", path.display(), e),
        })?;
        Ok(())
    }

    /// Invalidate the snapshot cache (call after any mutation).
    fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }

    /// Load all snapshot records from disk, using cache if available.
    fn load_all(&self) -> IronResult<Vec<SnapshotRecord>> {
        if let Ok(cache) = self.cache.lock()
            && let Some(ref cached) = *cache
        {
            return Ok(cached.clone());
        }

        let dir = self.snapshots_dir()?;
        let mut records = Vec::new();

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json")
                    && path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("snap-"))
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(record) = serde_json::from_str::<SnapshotRecord>(&content)
                {
                    records.push(record);
                }
            }
        }

        // Sort newest first
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Populate cache
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(records.clone());
        }

        Ok(records)
    }
}

impl SnapshotService for DefaultSnapshotService {
    fn create(&self, name: &str, description: Option<&str>) -> IronResult<SnapshotRecord> {
        let record = self.capture_state(name, description, false)?;
        self.save_record(&record)?;
        self.invalidate_cache();
        Ok(record)
    }

    fn create_auto(&self, prefix: &str) -> IronResult<SnapshotRecord> {
        let name = format!("{}-{}", prefix, Utc::now().format("%Y%m%d-%H%M%S"));
        let record = self.capture_state(&name, None, true)?;
        self.save_record(&record)?;
        self.invalidate_cache();
        Ok(record)
    }

    fn list(&self) -> IronResult<Vec<SnapshotRecord>> {
        self.load_all()
    }

    fn get(&self, name_or_id: &str) -> IronResult<SnapshotRecord> {
        let records = self.load_all()?;
        records
            .into_iter()
            .find(|r| r.name == name_or_id || r.id == name_or_id)
            .ok_or_else(|| {
                crate::FsError::IoError {
                    message: format!("Snapshot '{}' not found", name_or_id),
                }
                .into()
            })
    }

    fn delete(&self, name_or_id: &str) -> IronResult<()> {
        let record = self.get(name_or_id)?;
        let path = self.snapshot_path(&record.id)?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| crate::FsError::IoError {
                message: format!("Failed to delete snapshot: {}", e),
            })?;
        }
        self.invalidate_cache();
        Ok(())
    }

    fn prune_auto(&self, keep: usize) -> IronResult<usize> {
        let records = self.load_all()?;
        let auto_records: Vec<&SnapshotRecord> = records.iter().filter(|r| r.auto).collect();

        if auto_records.len() <= keep {
            return Ok(0);
        }

        let mut pruned = 0;
        // Records are already sorted newest-first; skip `keep`, delete the rest
        for record in auto_records.iter().skip(keep) {
            let path = self.snapshot_path(&record.id)?;
            if path.exists() {
                fs::remove_file(&path).ok();
                pruned += 1;
            }
        }

        self.invalidate_cache();
        Ok(pruned)
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultSnapshotService, TempDir) {
        let tmp = TempDir::new().unwrap();
        let state = StateManager::new(tmp.path().to_path_buf()).unwrap();
        let svc = DefaultSnapshotService::new(tmp.path(), state);
        (svc, tmp)
    }

    #[test]
    fn test_create_and_list() {
        let (svc, _tmp) = create_test_service();

        let record = svc.create("test-snap", Some("A test")).unwrap();
        assert_eq!(record.name, "test-snap");
        assert_eq!(record.description, Some("A test".to_string()));
        assert!(!record.auto);

        let list = svc.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, record.id);
    }

    #[test]
    fn test_create_auto() {
        let (svc, _tmp) = create_test_service();

        let record = svc.create_auto("pre-apply").unwrap();
        assert!(record.auto);
        assert!(record.name.starts_with("pre-apply-"));
    }

    #[test]
    fn test_get_by_name() {
        let (svc, _tmp) = create_test_service();

        svc.create("my-snap", None).unwrap();
        let found = svc.get("my-snap").unwrap();
        assert_eq!(found.name, "my-snap");
    }

    #[test]
    fn test_get_by_id() {
        let (svc, _tmp) = create_test_service();

        let record = svc.create("id-test", None).unwrap();
        let found = svc.get(&record.id).unwrap();
        assert_eq!(found.name, "id-test");
    }

    #[test]
    fn test_get_not_found() {
        let (svc, _tmp) = create_test_service();
        assert!(svc.get("nonexistent").is_err());
    }

    #[test]
    fn test_delete() {
        let (svc, _tmp) = create_test_service();

        svc.create("to-delete", None).unwrap();
        assert_eq!(svc.list().unwrap().len(), 1);

        svc.delete("to-delete").unwrap();
        assert_eq!(svc.list().unwrap().len(), 0);
    }

    #[test]
    fn test_prune_auto() {
        let (svc, _tmp) = create_test_service();

        // Create 5 auto-snapshots
        for i in 0..5 {
            let name = format!("auto-{}", i);
            let record = svc.capture_state(&name, None, true).unwrap();
            svc.save_record(&record).unwrap();
        }

        assert_eq!(svc.list().unwrap().len(), 5);

        // Prune keeping 2
        let pruned = svc.prune_auto(2).unwrap();
        assert_eq!(pruned, 3);
        assert_eq!(svc.list().unwrap().len(), 2);
    }

    #[test]
    fn test_prune_keeps_manual_snapshots() {
        let (svc, _tmp) = create_test_service();

        // Create 3 auto + 2 manual
        for i in 0..3 {
            let record = svc
                .capture_state(&format!("auto-{}", i), None, true)
                .unwrap();
            svc.save_record(&record).unwrap();
        }
        svc.create("manual-1", None).unwrap();
        svc.create("manual-2", None).unwrap();

        assert_eq!(svc.list().unwrap().len(), 5);

        // Prune auto keeping 1
        svc.prune_auto(1).unwrap();

        let remaining = svc.list().unwrap();
        // 2 manual + 1 auto = 3
        assert_eq!(remaining.len(), 3);
        let manual_count = remaining.iter().filter(|r| !r.auto).count();
        assert_eq!(manual_count, 2);
    }

    #[test]
    fn test_list_sorted_newest_first() {
        let (svc, _tmp) = create_test_service();

        svc.create("first", None).unwrap();
        svc.create("second", None).unwrap();
        svc.create("third", None).unwrap();

        let list = svc.list().unwrap();
        assert_eq!(list.len(), 3);
        // Newest first
        assert!(list[0].timestamp >= list[1].timestamp);
        assert!(list[1].timestamp >= list[2].timestamp);
    }

    #[test]
    fn test_snapshot_record_summary() {
        let record = SnapshotRecord {
            id: "test".to_string(),
            name: "test-snap".to_string(),
            timestamp: Utc::now(),
            host_id: None,
            active_bundle: None,
            active_profile: None,
            active_modules: vec!["nvim".into(), "fish".into()],
            explicit_packages: vec!["neovim".into()],
            dotfile_checksums: HashMap::new(),
            auto: false,
            description: None,
        };
        let summary = record.summary();
        assert!(summary.contains("test-snap"));
        assert!(summary.contains("2 modules"));
        assert!(summary.contains("1 packages"));
    }

    #[test]
    fn test_auto_snapshot_summary_has_tag() {
        let record = SnapshotRecord {
            id: "test".to_string(),
            name: "auto-test".to_string(),
            timestamp: Utc::now(),
            host_id: None,
            active_bundle: None,
            active_profile: None,
            active_modules: vec![],
            explicit_packages: vec![],
            dotfile_checksums: HashMap::new(),
            auto: true,
            description: None,
        };
        assert!(record.summary().contains("[auto]"));
    }

    #[test]
    fn test_snapshot_serialization_roundtrip() {
        let record = SnapshotRecord {
            id: "abc-123".to_string(),
            name: "roundtrip-test".to_string(),
            timestamp: Utc::now(),
            host_id: Some("desktop".to_string()),
            active_bundle: Some("hyprland".to_string()),
            active_profile: Some("developer".to_string()),
            active_modules: vec!["nvim".into(), "fish".into()],
            explicit_packages: vec!["neovim".into(), "fish".into()],
            dotfile_checksums: HashMap::from([("~/.config/nvim".into(), "abc123".into())]),
            auto: false,
            description: Some("A test snapshot".to_string()),
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: SnapshotRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, record.id);
        assert_eq!(deserialized.name, record.name);
        assert_eq!(deserialized.host_id, record.host_id);
        assert_eq!(deserialized.active_bundle, record.active_bundle);
        assert_eq!(deserialized.active_modules.len(), 2);
        assert_eq!(deserialized.explicit_packages.len(), 2);
        assert_eq!(deserialized.dotfile_checksums.len(), 1);
    }

    #[test]
    fn snapshot_service_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DefaultSnapshotService>();
    }

    #[test]
    fn test_backward_compat_minimal_json() {
        let json = r#"{"id":"x","name":"y","timestamp":"2026-02-22T00:00:00Z"}"#;
        let record: SnapshotRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.id, "x");
        assert!(record.active_modules.is_empty());
        assert!(!record.auto);
    }
}
