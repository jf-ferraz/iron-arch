//! State Management Service
//!
//! Provides robust state management with transactions and audit logging.

use crate::state::{IronState, MaintenanceState, OperationStatus};
use crate::{IronResult, StateError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// State file name
const STATE_FILE: &str = "state.json";

/// Audit log file name
const AUDIT_LOG_FILE: &str = "audit.log";

/// Maximum audit log entries
const MAX_AUDIT_ENTRIES: usize = 1000;

/// Transaction state for atomic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub id: String,
    /// Start time
    pub started: chrono::DateTime<Utc>,
    /// Operations in this transaction
    pub operations: Vec<String>,
    /// State snapshot before transaction
    pub snapshot: IronState,
}

/// RAII guard for transactions
pub struct TransactionGuard<'a> {
    manager: &'a StateManager,
    transaction: Transaction,
    committed: bool,
}

impl<'a> TransactionGuard<'a> {
    /// Record an operation in the transaction
    pub fn record(&mut self, operation: &str) {
        self.transaction.operations.push(operation.to_string());
    }

    /// Commit the transaction
    pub fn commit(mut self) -> IronResult<()> {
        self.committed = true;
        self.manager.commit_transaction(&self.transaction)
    }

    /// Explicitly rollback (also happens on drop if not committed)
    pub fn rollback(mut self) -> IronResult<()> {
        self.committed = true; // Prevent double rollback
        self.manager.rollback_transaction(&self.transaction)
    }
}

impl<'a> Drop for TransactionGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Auto-rollback on drop if not committed
            let _ = self.manager.rollback_transaction(&self.transaction);
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
    /// Operation performed
    pub operation: String,
    /// Status
    pub status: OperationStatus,
    /// Details
    pub details: Option<String>,
    /// User (from environment)
    pub user: Option<String>,
}

/// State Manager - handles all state operations
#[derive(Clone)]
pub struct StateManager {
    /// Root directory for Iron
    root: PathBuf,
    /// In-memory state
    state: Arc<Mutex<IronState>>,
    /// Audit log entries
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(root: PathBuf) -> IronResult<Self> {
        let state_path = root.join(STATE_FILE);
        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
            serde_json::from_str(&content)
                .map_err(|_| StateError::Corrupted { path: state_path })?
        } else {
            IronState::default()
        };

        let audit_log = Self::load_audit_log(&root);

        Ok(Self {
            root,
            state: Arc::new(Mutex::new(state)),
            audit_log: Arc::new(Mutex::new(audit_log)),
        })
    }

    /// Load audit log from disk
    fn load_audit_log(root: &Path) -> Vec<AuditEntry> {
        let log_path = root.join(AUDIT_LOG_FILE);
        if log_path.exists()
            && let Ok(content) = fs::read_to_string(&log_path)
            && let Ok(entries) = serde_json::from_str(&content)
        {
            return entries;
        }
        Vec::new()
    }

    /// Get the Iron root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get state path
    pub fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    /// Lock state for reading
    pub fn state(&self) -> MutexGuard<'_, IronState> {
        self.state.lock().unwrap()
    }

    /// Get current host ID
    pub fn current_host(&self) -> Option<String> {
        self.state().current_host.clone()
    }

    /// Set current host
    pub fn set_current_host(&self, host_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.current_host = Some(host_id.to_string());
        }
        self.persist()?;
        self.audit(
            "set_current_host",
            OperationStatus::Success,
            Some(host_id.to_string()),
        )
    }

    /// Get active bundle for a host
    pub fn active_bundle(&self, host_id: &str) -> Option<String> {
        self.state().active_bundles.get(host_id).cloned()
    }

    /// Set active bundle for a host
    pub fn set_active_bundle(&self, host_id: &str, bundle_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state
                .active_bundles
                .insert(host_id.to_string(), bundle_id.to_string());
        }
        self.persist()?;
        self.audit(
            "set_active_bundle",
            OperationStatus::Success,
            Some(format!("{}:{}", host_id, bundle_id)),
        )
    }

    /// Get active profile for a host
    pub fn active_profile(&self, host_id: &str) -> Option<String> {
        self.state().active_profiles.get(host_id).cloned()
    }

    /// Set active profile for a host
    pub fn set_active_profile(&self, host_id: &str, profile_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state
                .active_profiles
                .insert(host_id.to_string(), profile_id.to_string());
        }
        self.persist()?;
        self.audit(
            "set_active_profile",
            OperationStatus::Success,
            Some(format!("{}:{}", host_id, profile_id)),
        )
    }

    /// Get active modules
    pub fn active_modules(&self) -> Vec<String> {
        self.state().active_modules.clone()
    }

    /// Enable a module
    pub fn enable_module(&self, module_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            if !state.active_modules.contains(&module_id.to_string()) {
                state.active_modules.push(module_id.to_string());
            }
        }
        self.persist()?;
        self.audit(
            "enable_module",
            OperationStatus::Success,
            Some(module_id.to_string()),
        )
    }

    /// Disable a module
    pub fn disable_module(&self, module_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.active_modules.retain(|m| m != module_id);
        }
        self.persist()?;
        self.audit(
            "disable_module",
            OperationStatus::Success,
            Some(module_id.to_string()),
        )
    }

    /// Is a module active?
    pub fn is_module_active(&self, module_id: &str) -> bool {
        self.state().active_modules.contains(&module_id.to_string())
    }

    /// Get maintenance state
    pub fn maintenance(&self) -> MaintenanceState {
        self.state().maintenance.clone()
    }

    /// Update maintenance timestamp
    pub fn update_maintenance(&self, field: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            let now = Some(Utc::now());
            match field {
                "update" => state.maintenance.last_update = now,
                "clean" => state.maintenance.last_clean = now,
                "doctor" => state.maintenance.last_doctor = now,
                "snapshot" => state.maintenance.last_snapshot = now,
                "sync" => state.maintenance.last_sync = now,
                _ => {}
            }
        }
        self.persist()
    }

    /// Begin a transaction
    pub fn begin_transaction(&self, name: &str) -> IronResult<TransactionGuard<'_>> {
        let snapshot = self.state().clone();
        let transaction = Transaction {
            id: format!("{}_{}", name, Utc::now().timestamp_millis()),
            started: Utc::now(),
            operations: Vec::new(),
            snapshot,
        };

        self.audit(
            "begin_transaction",
            OperationStatus::Success,
            Some(transaction.id.clone()),
        )?;

        Ok(TransactionGuard {
            manager: self,
            transaction,
            committed: false,
        })
    }

    /// Commit a transaction (internal)
    fn commit_transaction(&self, transaction: &Transaction) -> IronResult<()> {
        self.persist()?;
        self.audit(
            "commit_transaction",
            OperationStatus::Success,
            Some(transaction.id.clone()),
        )
    }

    /// Rollback a transaction (internal)
    fn rollback_transaction(&self, transaction: &Transaction) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            *state = transaction.snapshot.clone();
        }
        self.persist()?;
        self.audit(
            "rollback_transaction",
            OperationStatus::Success,
            Some(transaction.id.clone()),
        )
    }

    /// Persist state to disk
    pub fn persist(&self) -> IronResult<()> {
        let state_path = self.state_path();

        // Create parent directory if needed
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
        }

        // Write to temp file first (atomic write)
        let temp_path = state_path.with_extension("tmp");
        let state = self.state();
        let content = serde_json::to_string_pretty(&*state).map_err(|_| StateError::Corrupted {
            path: state_path.clone(),
        })?;

        fs::write(&temp_path, &content).map_err(|_| StateError::Corrupted {
            path: state_path.clone(),
        })?;

        // Atomic rename
        fs::rename(&temp_path, &state_path)
            .map_err(|_| StateError::Corrupted { path: state_path })?;

        Ok(())
    }

    /// Record an audit entry
    pub fn audit(
        &self,
        operation: &str,
        status: OperationStatus,
        details: Option<String>,
    ) -> IronResult<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            operation: operation.to_string(),
            status,
            details,
            user: std::env::var("USER").ok(),
        };

        {
            let mut log = self.audit_log.lock().unwrap();
            log.push(entry);

            // Trim if too large
            if log.len() > MAX_AUDIT_ENTRIES {
                let keep_count = log.len() - MAX_AUDIT_ENTRIES;
                *log = log.split_off(keep_count);
            }
        }

        self.persist_audit_log()
    }

    /// Persist audit log to disk
    fn persist_audit_log(&self) -> IronResult<()> {
        let log_path = self.root.join(AUDIT_LOG_FILE);
        let log = self.audit_log.lock().unwrap();
        let content = serde_json::to_string_pretty(&*log).unwrap_or_default();
        fs::write(&log_path, content).ok();
        Ok(())
    }

    /// Get recent audit entries
    pub fn recent_audit(&self, count: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.lock().unwrap();
        log.iter().rev().take(count).cloned().collect()
    }

    /// Record an operation in history
    pub fn record_operation(
        &self,
        operation: &str,
        status: OperationStatus,
        details: Option<String>,
    ) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.record_operation(operation, status.clone(), details.clone());
        }
        self.persist()?;
        self.audit(operation, status, details)
    }

    /// Export state to JSON
    pub fn export(&self) -> IronResult<String> {
        let state = self.state();
        serde_json::to_string_pretty(&*state).map_err(|_| crate::IronError::OperationFailed {
            message: "Failed to export state".to_string(),
        })
    }

    /// Import state from JSON
    pub fn import(&self, json: &str) -> IronResult<()> {
        let new_state: IronState =
            serde_json::from_str(json).map_err(|e| crate::IronError::OperationFailed {
                message: format!("Failed to import state: {}", e),
            })?;

        {
            let mut state = self.state.lock().unwrap();
            *state = new_state;
        }

        self.persist()?;
        self.audit("import_state", OperationStatus::Success, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (StateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_state_manager_creation() {
        let (manager, _temp) = create_test_manager();
        assert!(manager.current_host().is_none());
    }

    #[test]
    fn test_set_current_host() {
        let (manager, _temp) = create_test_manager();
        manager.set_current_host("laptop").unwrap();
        assert_eq!(manager.current_host(), Some("laptop".to_string()));
    }

    #[test]
    fn test_active_bundle() {
        let (manager, _temp) = create_test_manager();
        manager.set_active_bundle("laptop", "hyprland").unwrap();
        assert_eq!(
            manager.active_bundle("laptop"),
            Some("hyprland".to_string())
        );
    }

    #[test]
    fn test_module_enable_disable() {
        let (manager, _temp) = create_test_manager();

        manager.enable_module("nvim-ide").unwrap();
        assert!(manager.is_module_active("nvim-ide"));

        manager.disable_module("nvim-ide").unwrap();
        assert!(!manager.is_module_active("nvim-ide"));
    }

    #[test]
    fn test_transaction_commit() {
        let (manager, _temp) = create_test_manager();

        let mut txn = manager.begin_transaction("test").unwrap();
        txn.record("operation1");
        manager.enable_module("test-mod").unwrap();
        txn.commit().unwrap();

        assert!(manager.is_module_active("test-mod"));
    }

    #[test]
    fn test_transaction_rollback() {
        let (manager, _temp) = create_test_manager();

        manager.enable_module("original-mod").unwrap();

        {
            let mut txn = manager.begin_transaction("test").unwrap();
            txn.record("operation1");
            manager.enable_module("new-mod").unwrap();
            txn.rollback().unwrap();
        }

        // After rollback, new-mod should not be active
        assert!(!manager.is_module_active("new-mod"));
        assert!(manager.is_module_active("original-mod"));
    }

    #[test]
    fn test_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let manager = StateManager::new(path.clone()).unwrap();
            manager.set_current_host("test-host").unwrap();
            manager.enable_module("test-mod").unwrap();
        }

        // Reload and verify
        let manager = StateManager::new(path).unwrap();
        assert_eq!(manager.current_host(), Some("test-host".to_string()));
        assert!(manager.is_module_active("test-mod"));
    }

    #[test]
    fn test_export_import() {
        let (manager, _temp) = create_test_manager();

        manager.set_current_host("export-host").unwrap();
        manager.enable_module("export-mod").unwrap();

        let exported = manager.export().unwrap();

        // Create new manager and import
        let (manager2, _temp2) = create_test_manager();
        manager2.import(&exported).unwrap();

        assert_eq!(manager2.current_host(), Some("export-host".to_string()));
        assert!(manager2.is_module_active("export-mod"));
    }

    #[test]
    fn test_audit_log() {
        let (manager, _temp) = create_test_manager();

        manager.set_current_host("host1").unwrap();
        manager.enable_module("mod1").unwrap();

        let audit = manager.recent_audit(10);
        assert!(audit.len() >= 2);
    }
}
