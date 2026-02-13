//! State Management Service
//!
//! Provides robust state management with transactions and audit logging.
//! Uses file locking for safe concurrent access across processes.

use crate::state::{IronState, MaintenanceState, OperationStatus};
use crate::{IronResult, StateError};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// State file name
const STATE_FILE: &str = "state.json";

/// Lock file name for concurrent access protection
const LOCK_FILE: &str = ".state.lock";

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

    /// Enable a module (with file locking for concurrent safety)
    pub fn enable_module(&self, module_id: &str) -> IronResult<()> {
        let module_id_owned = module_id.to_string();
        self.with_locked_state(|state| {
            if !state.active_modules.contains(&module_id_owned) {
                state.active_modules.push(module_id_owned.clone());
            }
        })?;
        self.audit(
            "enable_module",
            OperationStatus::Success,
            Some(module_id.to_string()),
        )
    }

    /// Disable a module (with file locking for concurrent safety)
    pub fn disable_module(&self, module_id: &str) -> IronResult<()> {
        let module_id_owned = module_id.to_string();
        self.with_locked_state(|state| {
            state.active_modules.retain(|m| m != &module_id_owned);
        })?;
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

    /// Get the lock file path
    fn lock_path(&self) -> PathBuf {
        self.root.join(LOCK_FILE)
    }

    /// Acquire an exclusive lock on the state file
    /// Returns the lock file handle which must be held for the duration of the operation
    fn acquire_lock(&self) -> IronResult<File> {
        let lock_path = self.lock_path();

        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|_| StateError::Corrupted {
                path: lock_path.clone(),
            })?;
        }

        // Open or create the lock file
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|_| StateError::Corrupted {
                path: lock_path.clone(),
            })?;

        // Acquire exclusive lock (blocks until available)
        lock_file
            .lock_exclusive()
            .map_err(|_| StateError::Corrupted { path: lock_path })?;

        Ok(lock_file)
    }

    /// Reload state from disk (call after acquiring lock to get latest state)
    fn reload_from_disk(&self) -> IronResult<()> {
        let state_path = self.state_path();
        if state_path.exists() {
            let content = fs::read_to_string(&state_path).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
            let new_state: IronState = serde_json::from_str(&content)
                .map_err(|_| StateError::Corrupted { path: state_path })?;

            let mut state = self.state.lock().unwrap();
            *state = new_state;
        }
        Ok(())
    }

    /// Execute a locked state operation
    /// This acquires the lock, reloads state from disk, executes the operation,
    /// and persists the result atomically.
    pub fn with_locked_state<F, T>(&self, operation: F) -> IronResult<T>
    where
        F: FnOnce(&mut IronState) -> T,
    {
        // Acquire exclusive file lock
        let _lock = self.acquire_lock()?;

        // Reload state from disk to get latest changes from other processes
        self.reload_from_disk()?;

        // Execute the operation
        let result = {
            let mut state = self.state.lock().unwrap();
            operation(&mut state)
        };

        // Persist the changes
        self.persist_unlocked()?;

        Ok(result)
    }

    /// Persist state to disk with file locking for concurrent safety
    pub fn persist(&self) -> IronResult<()> {
        let state_path = self.state_path();

        // Create parent directory if needed
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
        }

        // Acquire exclusive file lock to prevent concurrent writes
        let _lock = self.acquire_lock()?;

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

        // Lock is automatically released when _lock goes out of scope
        Ok(())
    }

    /// Internal persist without acquiring lock (called when lock is already held)
    fn persist_unlocked(&self) -> IronResult<()> {
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

    #[test]
    fn test_concurrent_module_operations() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create two managers pointing to the same state file
        let manager1 = Arc::new(StateManager::new(path.clone()).unwrap());
        let manager2 = Arc::new(StateManager::new(path.clone()).unwrap());

        // Spawn two threads that modify state concurrently
        let m1 = Arc::clone(&manager1);
        let handle1 = thread::spawn(move || {
            for i in 0..10 {
                m1.enable_module(&format!("mod-a-{}", i)).unwrap();
            }
        });

        let m2 = Arc::clone(&manager2);
        let handle2 = thread::spawn(move || {
            for i in 0..10 {
                m2.enable_module(&format!("mod-b-{}", i)).unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Create a fresh manager to read the final state from disk
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();
        let a_count = modules.iter().filter(|m| m.starts_with("mod-a-")).count();
        let b_count = modules.iter().filter(|m| m.starts_with("mod-b-")).count();

        assert_eq!(a_count, 10, "All mod-a modules should be present");
        assert_eq!(b_count, 10, "All mod-b modules should be present");
    }

    #[test]
    fn test_with_locked_state() {
        let (manager, _temp) = create_test_manager();

        // Test that with_locked_state works correctly
        let result = manager
            .with_locked_state(|state| {
                state.active_modules.push("test-mod".to_string());
                42
            })
            .unwrap();

        assert_eq!(result, 42);
        assert!(manager.is_module_active("test-mod"));
    }

    // ==========================================
    // Phase 8.4: Comprehensive Concurrent Access Tests
    // ==========================================

    #[test]
    fn test_concurrent_reads_no_blocking() {
        use std::sync::Arc;
        use std::thread;
        use std::time::Instant;

        let (manager, _temp) = create_test_manager();

        // Set up some initial state
        manager.set_current_host("test-host").unwrap();
        manager.enable_module("mod-1").unwrap();
        manager.enable_module("mod-2").unwrap();

        let manager = Arc::new(manager);
        let mut handles = vec![];

        // Spawn 10 concurrent readers
        let start = Instant::now();
        for _ in 0..10 {
            let m = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = m.current_host();
                    let _ = m.active_modules();
                    let _ = m.is_module_active("mod-1");
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();
        // Concurrent reads should complete quickly (< 1 second for 1000 total reads)
        assert!(
            elapsed.as_secs() < 2,
            "Concurrent reads took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_concurrent_writes_no_data_loss() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let mut handles = vec![];

        // Spawn 5 threads each enabling 20 different modules
        for thread_id in 0..5 {
            let m = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                for i in 0..20 {
                    m.enable_module(&format!("thread{}-mod{}", thread_id, i))
                        .unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all 100 modules are present
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();

        for thread_id in 0..5 {
            for i in 0..20 {
                let mod_name = format!("thread{}-mod{}", thread_id, i);
                assert!(
                    modules.contains(&mod_name),
                    "Module {} not found in final state",
                    mod_name
                );
            }
        }

        assert_eq!(modules.len(), 100, "Expected 100 modules, found {}", modules.len());
    }

    #[test]
    fn test_concurrent_enable_disable_same_module() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());

        // Spawn threads that enable and disable the same module
        let m1 = Arc::clone(&manager);
        let handle1 = thread::spawn(move || {
            for _ in 0..50 {
                m1.enable_module("contested-mod").unwrap();
            }
        });

        let m2 = Arc::clone(&manager);
        let handle2 = thread::spawn(move || {
            for _ in 0..50 {
                m2.disable_module("contested-mod").unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // State should be consistent (either enabled or disabled, not corrupted)
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();

        // The module should appear at most once
        let count = modules.iter().filter(|m| *m == "contested-mod").count();
        assert!(count <= 1, "Module appears {} times (should be 0 or 1)", count);
    }

    #[test]
    fn test_stress_test_many_threads() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let mut handles = vec![];

        // Spawn 20 threads with mixed operations
        for thread_id in 0..20 {
            let m = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    // Alternate between enable and disable operations
                    if i % 2 == 0 {
                        m.enable_module(&format!("stress-{}-{}", thread_id, i)).unwrap();
                    } else {
                        // Read operation
                        let _ = m.active_modules();
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify state is not corrupted
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();

        // Should have some modules enabled (at least half of even iterations)
        assert!(
            !modules.is_empty(),
            "Expected some modules to be enabled after stress test"
        );

        // Verify all module names are valid format
        for module in &modules {
            assert!(
                module.starts_with("stress-"),
                "Unexpected module format: {}",
                module
            );
        }
    }

    #[test]
    fn test_file_locking_prevents_corruption() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn threads that perform locked operations
        for i in 0..10 {
            let m = Arc::clone(&manager);
            let count = Arc::clone(&success_count);
            handles.push(thread::spawn(move || {
                let result = m.with_locked_state(|state| {
                    // Simulate some work
                    thread::sleep(std::time::Duration::from_millis(1));
                    state.active_modules.push(format!("locked-mod-{}", i));
                    true
                });
                if result.is_ok() {
                    count.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All operations should succeed
        assert_eq!(
            success_count.load(Ordering::SeqCst),
            10,
            "All locked operations should succeed"
        );

        // Verify final state
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();
        assert_eq!(modules.len(), 10, "All 10 modules should be present");
    }

    #[test]
    fn test_sequential_transaction_commit_and_rollback() {
        // Note: Transaction rollback restores the full state snapshot from when
        // the transaction began. This means concurrent transactions with rollback
        // can interfere with each other. This test verifies sequential behavior.
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Set up initial state
        let manager = StateManager::new(path.clone()).unwrap();
        manager.enable_module("initial-mod").unwrap();

        // Transaction 1: Commit changes
        {
            let mut txn = manager.begin_transaction("txn1").unwrap();
            txn.record("enabling txn1-mod");
            manager.enable_module("txn1-mod").unwrap();
            txn.commit().unwrap();
        }

        // Verify txn1 changes persisted
        assert!(manager.is_module_active("txn1-mod"));
        assert!(manager.is_module_active("initial-mod"));

        // Transaction 2: Rollback changes
        {
            let mut txn = manager.begin_transaction("txn2").unwrap();
            txn.record("enabling txn2-mod");
            manager.enable_module("txn2-mod").unwrap();
            txn.rollback().unwrap();
        }

        // Verify state after rollback: txn2-mod should be gone
        let final_manager = StateManager::new(path).unwrap();
        assert!(
            final_manager.is_module_active("txn1-mod"),
            "txn1-mod should still be present after txn2 rollback"
        );
        assert!(
            final_manager.is_module_active("initial-mod"),
            "initial-mod should be present"
        );
        assert!(
            !final_manager.is_module_active("txn2-mod"),
            "txn2-mod should be rolled back"
        );
    }

    #[test]
    fn test_mixed_read_write_operations() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let read_count = Arc::new(AtomicUsize::new(0));
        let write_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn reader threads
        for _ in 0..5 {
            let m = Arc::clone(&manager);
            let count = Arc::clone(&read_count);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = m.active_modules();
                    count.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Spawn writer threads
        for i in 0..5 {
            let m = Arc::clone(&manager);
            let count = Arc::clone(&write_count);
            handles.push(thread::spawn(move || {
                for j in 0..10 {
                    m.enable_module(&format!("rw-{}-{}", i, j)).unwrap();
                    count.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All operations should complete
        assert_eq!(read_count.load(Ordering::SeqCst), 500);
        assert_eq!(write_count.load(Ordering::SeqCst), 50);

        // Verify state integrity
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();
        assert_eq!(modules.len(), 50, "All 50 modules should be present");
    }

    #[test]
    fn test_concurrent_host_and_bundle_operations() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let mut handles = vec![];

        // Thread 1: Set hosts
        let m1 = Arc::clone(&manager);
        handles.push(thread::spawn(move || {
            for i in 0..10 {
                m1.set_current_host(&format!("host-{}", i)).unwrap();
            }
        }));

        // Thread 2: Set bundles
        let m2 = Arc::clone(&manager);
        handles.push(thread::spawn(move || {
            for i in 0..10 {
                m2.set_active_bundle(&format!("host-{}", i), &format!("bundle-{}", i))
                    .unwrap();
            }
        }));

        // Thread 3: Set profiles
        let m3 = Arc::clone(&manager);
        handles.push(thread::spawn(move || {
            for i in 0..10 {
                m3.set_active_profile(&format!("host-{}", i), &format!("profile-{}", i))
                    .unwrap();
            }
        }));

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify state is consistent
        let final_manager = StateManager::new(path).unwrap();

        // Current host should be one of the valid hosts
        let host = final_manager.current_host();
        assert!(host.is_some(), "Current host should be set");
        assert!(
            host.as_ref().unwrap().starts_with("host-"),
            "Host should have valid format"
        );

        // Bundles and profiles should be set for some hosts
        let state = final_manager.state();
        assert!(
            !state.active_bundles.is_empty(),
            "Some bundles should be set"
        );
        assert!(
            !state.active_profiles.is_empty(),
            "Some profiles should be set"
        );
    }

    #[test]
    fn test_audit_log_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let mut handles = vec![];

        // Spawn threads that generate audit entries
        for i in 0..10 {
            let m = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                for j in 0..10 {
                    m.enable_module(&format!("audit-{}-{}", i, j)).unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify audit log has entries
        let final_manager = StateManager::new(path).unwrap();
        let audit = final_manager.recent_audit(200);

        // Should have at least 100 enable_module entries
        let enable_entries = audit
            .iter()
            .filter(|e| e.operation == "enable_module")
            .count();
        assert!(
            enable_entries >= 100,
            "Expected at least 100 enable_module audit entries, found {}",
            enable_entries
        );
    }

    #[test]
    fn test_state_reload_consistency() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create two managers pointing to the same state
        let manager1 = Arc::new(StateManager::new(path.clone()).unwrap());
        let manager2 = Arc::new(StateManager::new(path.clone()).unwrap());

        // Manager 1 makes changes
        let m1 = Arc::clone(&manager1);
        let handle1 = thread::spawn(move || {
            for i in 0..5 {
                m1.enable_module(&format!("m1-mod-{}", i)).unwrap();
                thread::sleep(std::time::Duration::from_millis(5));
            }
        });

        // Manager 2 makes changes (interleaved)
        let m2 = Arc::clone(&manager2);
        let handle2 = thread::spawn(move || {
            for i in 0..5 {
                thread::sleep(std::time::Duration::from_millis(2));
                m2.enable_module(&format!("m2-mod-{}", i)).unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Both managers should see all changes when reloaded
        let final_manager = StateManager::new(path).unwrap();
        let modules = final_manager.active_modules();

        let m1_count = modules.iter().filter(|m| m.starts_with("m1-")).count();
        let m2_count = modules.iter().filter(|m| m.starts_with("m2-")).count();

        assert_eq!(m1_count, 5, "All m1 modules should be present");
        assert_eq!(m2_count, 5, "All m2 modules should be present");
    }

    #[test]
    fn test_transaction_auto_rollback_on_drop() {
        // Note: Transaction auto-rollback restores the full state snapshot.
        // This test verifies the RAII pattern works correctly for sequential operations.
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = StateManager::new(path.clone()).unwrap();
        manager.enable_module("original").unwrap();

        // Transaction 1: Commit normally
        {
            let mut txn = manager.begin_transaction("will-commit").unwrap();
            txn.record("adding committed-mod");
            manager.enable_module("committed-mod").unwrap();
            txn.commit().unwrap();
        }

        // Transaction 2: Drop without commit (should auto-rollback)
        {
            let mut txn = manager.begin_transaction("will-drop").unwrap();
            txn.record("adding dropped-mod");
            manager.enable_module("dropped-mod").unwrap();
            // Transaction is dropped here without commit - auto-rollback
        }

        // Verify state
        let final_manager = StateManager::new(path).unwrap();

        // Original should be there
        assert!(
            final_manager.is_module_active("original"),
            "Original module should be present"
        );

        // committed-mod should be there (committed before the dropped transaction)
        assert!(
            final_manager.is_module_active("committed-mod"),
            "Committed module should be present"
        );

        // dropped-mod should NOT be there (auto-rollback)
        assert!(
            !final_manager.is_module_active("dropped-mod"),
            "Dropped transaction should have been rolled back"
        );
    }

    #[test]
    fn test_concurrent_transactions_both_commit() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let manager = Arc::new(StateManager::new(path.clone()).unwrap());
        let commit_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn multiple threads that all commit (no rollback interference)
        for i in 0..5 {
            let m = Arc::clone(&manager);
            let count = Arc::clone(&commit_count);
            handles.push(thread::spawn(move || {
                let mut txn = m.begin_transaction(&format!("txn-{}", i)).unwrap();
                txn.record(&format!("enabling mod-{}", i));
                m.enable_module(&format!("concurrent-txn-mod-{}", i)).unwrap();
                txn.commit().unwrap();
                count.fetch_add(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All commits should succeed
        assert_eq!(commit_count.load(Ordering::SeqCst), 5);

        // Verify all modules are present
        let final_manager = StateManager::new(path).unwrap();
        for i in 0..5 {
            assert!(
                final_manager.is_module_active(&format!("concurrent-txn-mod-{}", i)),
                "Module {} should be present",
                i
            );
        }
    }
}
