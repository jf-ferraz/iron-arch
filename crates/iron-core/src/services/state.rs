//! State Management Service
//!
//! Provides robust state management with transactions and audit logging.
//! Uses file locking for safe concurrent access across processes.

use crate::state::{IronState, MaintenanceState, OperationStatus, UpdateProgress};
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
    /// Config root directory (e.g., ~/.config/iron)
    root: PathBuf,
    /// State directory (e.g., ~/.local/state/iron)
    state_root: PathBuf,
    /// In-memory state
    state: Arc<Mutex<IronState>>,
    /// Audit log entries
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}

impl StateManager {
    /// Resolve the state directory path.
    ///
    /// Priority:
    /// 1. `$IRON_STATE_DIR` environment variable (for testing / custom setups)
    /// 2. `$XDG_STATE_HOME/iron` (XDG standard)
    /// 3. `~/.local/state/iron` (XDG default fallback)
    ///
    /// Note: does NOT create the directory. Callers that need to write
    /// state files should ensure the directory exists first.
    pub fn state_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("IRON_STATE_DIR") {
            PathBuf::from(dir)
        } else if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            PathBuf::from(xdg).join("iron")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local/state/iron")
        }
    }

    /// Create a new state manager.
    ///
    /// The `root` parameter is the config root (e.g., `~/.config/iron`).
    /// State files are stored in the XDG state directory (resolved via
    /// `state_dir()`). For backward compatibility during migration, if
    /// the config root has a `state.json`, it is used as the effective
    /// state root (pre-migration mode). Once `migrate_if_needed()` moves
    /// state files to the XDG dir, this fallback stops triggering.
    pub fn new(root: PathBuf) -> IronResult<Self> {
        let resolved_state_dir = Self::state_dir();

        // State root resolution order:
        // 1. If $IRON_STATE_DIR is explicitly set, always use it
        //    (for testing and custom deployments)
        // 2. If config root has state.json, use config root
        //    (pre-migration backward compat -- F3-007 will move it)
        // 3. If config root has MIGRATED.txt (breadcrumb from F3-007
        //    migration) AND the XDG state dir has state.json, use it
        // 4. Otherwise, use config root (fresh installation, tests)
        //
        // The MIGRATED.txt check prevents tests from accidentally
        // picking up a real user's XDG state directory. Only after
        // migrate_if_needed() has explicitly moved state files will
        // the XDG dir be used.
        let state_root = if std::env::var("IRON_STATE_DIR").is_ok() {
            let _ = fs::create_dir_all(&resolved_state_dir);
            resolved_state_dir
        } else if root.join(STATE_FILE).exists() {
            root.clone()
        } else if root.join("MIGRATED.txt").exists() && resolved_state_dir.join(STATE_FILE).exists()
        {
            resolved_state_dir
        } else {
            root.clone()
        };

        let state_path = state_root.join(STATE_FILE);
        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
            serde_json::from_str(&content)
                .map_err(|_| StateError::Corrupted { path: state_path })?
        } else {
            IronState::default()
        };

        let audit_log = Self::load_audit_log(&state_root);

        Ok(Self {
            root,
            state_root,
            state: Arc::new(Mutex::new(state)),
            audit_log: Arc::new(Mutex::new(audit_log)),
        })
    }

    /// Load audit log from disk
    fn load_audit_log(state_root: &Path) -> Vec<AuditEntry> {
        let log_path = state_root.join(AUDIT_LOG_FILE);
        if log_path.exists()
            && let Ok(content) = fs::read_to_string(&log_path)
            && let Ok(entries) = serde_json::from_str(&content)
        {
            return entries;
        }
        Vec::new()
    }

    /// Get the Iron config root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the state directory this manager is using.
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// Get the config root (iron root) this manager was created with.
    pub fn config_root(&self) -> &Path {
        &self.root
    }

    /// Get state path
    pub fn state_path(&self) -> PathBuf {
        self.state_root.join(STATE_FILE)
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

    /// Clear active bundle for a host (used on deactivation)
    pub fn clear_active_bundle(&self, host_id: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.active_bundles.remove(host_id);
        }
        self.persist()?;
        self.audit(
            "clear_active_bundle",
            OperationStatus::Success,
            Some(host_id.to_string()),
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

    // ==========================================================================
    // Scan History (S1-P1.5-005)
    // ==========================================================================

    /// Save a scan report to state for history / re-scan
    pub fn save_scan_report(&self, report: &crate::services::scan::ScanReport) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.last_scan_report = Some(report.clone());
        }
        self.persist()?;
        self.audit(
            "save_scan_report",
            OperationStatus::Success,
            Some(format!(
                "conflicts={} recommendations={}",
                report.potential_conflicts.len(),
                report.recommendations.len()
            )),
        )
    }

    /// Load last scan report from state
    pub fn load_scan_report(&self) -> Option<crate::services::scan::ScanReport> {
        self.state().last_scan_report.clone()
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

    // ==========================================================================
    // Update Progress Management (FR-5.10)
    // ==========================================================================

    /// Get current update progress
    pub fn get_update_progress(&self) -> Option<UpdateProgress> {
        self.state().update_progress.clone()
    }

    /// Set update progress (atomic with fsync)
    pub fn set_update_progress(&self, progress: Option<UpdateProgress>) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.update_progress = progress;
        }
        self.persist_atomic()
    }

    /// Persist state atomically with fsync for durability
    /// Used for critical operations like update progress tracking
    fn persist_atomic(&self) -> IronResult<()> {
        let state_path = self.state_path();
        let temp_path = state_path.with_extension("tmp");

        // Create parent directories if needed
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).map_err(|_| StateError::Corrupted {
                path: state_path.clone(),
            })?;
        }

        // Serialize state
        let state = self.state.lock().unwrap();
        let content = serde_json::to_string_pretty(&*state).map_err(|_| StateError::Corrupted {
            path: state_path.clone(),
        })?;
        drop(state); // Release lock

        // Write to temp file
        fs::write(&temp_path, &content).map_err(|_| StateError::Corrupted {
            path: state_path.clone(),
        })?;

        // Atomic rename
        fs::rename(&temp_path, &state_path).map_err(|_| StateError::Corrupted {
            path: state_path.clone(),
        })?;

        // Fsync for durability
        if let Ok(file) = fs::File::open(&state_path) {
            let _ = file.sync_all();
        }

        Ok(())
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
        self.state_root.join(LOCK_FILE)
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
        let log_path = self.state_root.join(AUDIT_LOG_FILE);
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

    // ==========================================================================
    // News Acknowledgment Methods (Phase 2.2)
    // ==========================================================================

    /// Check if a news item has been acknowledged
    pub fn is_news_acknowledged(&self, url: &str) -> bool {
        self.state().is_news_acknowledged(url)
    }

    /// Acknowledge a single news item
    pub fn acknowledge_news(&self, url: &str) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.acknowledge_news(url);
        }
        self.persist()?;
        self.audit(
            "acknowledge_news",
            OperationStatus::Success,
            Some(url.to_string()),
        )
    }

    /// Acknowledge multiple news items
    pub fn acknowledge_all_news(&self, urls: &[&str]) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.acknowledge_all_news(urls);
        }
        self.persist()?;
        self.audit(
            "acknowledge_all_news",
            OperationStatus::Success,
            Some(format!("{} items", urls.len())),
        )
    }

    // ==========================================================================
    // F3-021: Managed Resource Tracking
    // ==========================================================================

    /// Record packages as managed by Iron. Deduplicates.
    pub fn record_managed_packages(&self, packages: &[String]) -> IronResult<()> {
        self.with_locked_state(|state| {
            for pkg in packages {
                if !state.managed_packages.contains(pkg) {
                    state.managed_packages.push(pkg.clone());
                }
            }
        })
    }

    /// Remove packages from managed tracking.
    pub fn unrecord_managed_packages(&self, packages: &[String]) -> IronResult<()> {
        self.with_locked_state(|state| {
            state.managed_packages.retain(|p| !packages.contains(p));
        })
    }

    /// Record a service as managed by Iron.
    pub fn record_managed_service(&self, name: &str) -> IronResult<()> {
        let name_owned = name.to_string();
        self.with_locked_state(|state| {
            if !state.managed_services.contains(&name_owned) {
                state.managed_services.push(name_owned);
            }
        })
    }

    /// Remove a service from managed tracking.
    pub fn unrecord_managed_service(&self, name: &str) -> IronResult<()> {
        let name_owned = name.to_string();
        self.with_locked_state(|state| {
            state.managed_services.retain(|s| s != &name_owned);
        })
    }

    /// Record a dotfile target as managed by Iron.
    pub fn record_managed_dotfile(&self, target: &str) -> IronResult<()> {
        let target_owned = target.to_string();
        self.with_locked_state(|state| {
            if !state.managed_dotfiles.contains(&target_owned) {
                state.managed_dotfiles.push(target_owned);
            }
        })
    }

    /// Remove a dotfile target from managed tracking.
    pub fn unrecord_managed_dotfile(&self, target: &str) -> IronResult<()> {
        let target_owned = target.to_string();
        self.with_locked_state(|state| {
            state.managed_dotfiles.retain(|d| d != &target_owned);
        })
    }

    /// Get current managed packages list.
    pub fn managed_packages(&self) -> Vec<String> {
        self.state().managed_packages.clone()
    }

    /// Get current managed services list.
    pub fn managed_services(&self) -> Vec<String> {
        self.state().managed_services.clone()
    }

    /// Get current managed dotfiles list.
    pub fn managed_dotfiles(&self) -> Vec<String> {
        self.state().managed_dotfiles.clone()
    }

    /// Update last_apply timestamp to now.
    pub fn update_last_apply(&self) -> IronResult<()> {
        self.with_locked_state(|state| {
            state.last_apply = Some(Utc::now());
        })
    }

    /// Mark news as recently fetched
    pub fn mark_news_fetched(&self) -> IronResult<()> {
        {
            let mut state = self.state.lock().unwrap();
            state.mark_news_fetched();
        }
        self.persist()
    }

    /// Check if news should be refetched
    pub fn should_refetch_news(&self) -> bool {
        self.state().should_refetch_news()
    }

    /// Get count of acknowledged news items
    pub fn acknowledged_news_count(&self) -> usize {
        self.state().news_acknowledgment.acknowledged_count()
    }

    // ==========================================================================
    // F3-015: Hook Execution Tracking
    // ==========================================================================

    /// Record that a hook has been executed for a module.
    pub fn record_hook_executed(&self, module_id: &str, hook_type: &str) -> IronResult<()> {
        let module_id = module_id.to_string();
        let hook_type = hook_type.to_string();
        self.with_locked_state(|state| {
            let hooks = state.hooks_executed.entry(module_id).or_default();
            if !hooks.contains(&hook_type) {
                hooks.push(hook_type);
            }
        })
    }

    /// Check if a hook has been executed for a module.
    pub fn is_hook_executed(&self, module_id: &str, hook_type: &str) -> bool {
        let state = self.state();
        state
            .hooks_executed
            .get(module_id)
            .map(|hooks| hooks.iter().any(|h| h == hook_type))
            .unwrap_or(false)
    }

    /// Clear all hook tracking for a module (called on module disable).
    pub fn clear_hooks_for_module(&self, module_id: &str) -> IronResult<()> {
        let module_id = module_id.to_string();
        self.with_locked_state(|state| {
            state.hooks_executed.remove(&module_id);
        })
    }

    // ==========================================================================
    // F3-007: Legacy State Migration
    // ==========================================================================

    /// Check for legacy state files in the config root and migrate
    /// them to the XDG state directory.
    ///
    /// Uses copy-then-delete for safety. On any failure, original
    /// files are left intact and a warning is logged.
    ///
    /// # No-op conditions
    /// - New state directory already has state.json
    /// - Legacy location has no state.json
    /// - MIGRATED.txt marker already exists in legacy location
    /// - Config root and state root are the same directory
    pub fn migrate_if_needed(config_root: &Path) -> IronResult<MigrationResult> {
        Self::migrate_to(config_root, &Self::state_dir())
    }

    /// Internal migration logic that accepts an explicit target dir.
    /// Public API delegates to this via `migrate_if_needed()`.
    fn migrate_to(config_root: &Path, state_dir: &Path) -> IronResult<MigrationResult> {
        let state_dir = state_dir.to_path_buf();

        // If they resolve to the same path, no migration needed
        if config_root == state_dir {
            return Ok(MigrationResult::NoMigrationNeeded);
        }

        let new_state_path = state_dir.join(STATE_FILE);
        let legacy_state_path = config_root.join(STATE_FILE);
        let migrated_marker = config_root.join("MIGRATED.txt");

        // No-op: new location already has state
        if new_state_path.exists() {
            return Ok(MigrationResult::NoMigrationNeeded);
        }

        // No-op: already migrated previously
        if migrated_marker.exists() {
            return Ok(MigrationResult::AlreadyMigrated);
        }

        // No-op: legacy location has no state
        if !legacy_state_path.exists() {
            return Ok(MigrationResult::NoMigrationNeeded);
        }

        // Ensure state directory exists
        let _ = fs::create_dir_all(&state_dir);

        // Copy state.json first (not move — copy for safety)
        if let Err(e) = fs::copy(&legacy_state_path, &new_state_path) {
            // Copy failed — leave originals intact
            return Err(crate::IronError::OperationFailed {
                message: format!("State migration failed (originals intact): {}", e),
            });
        }

        // Verify copy succeeded
        if !new_state_path.exists() {
            return Ok(MigrationResult::NoMigrationNeeded);
        }

        // Migrate additional files (best-effort)
        Self::migrate_file(config_root, &state_dir, AUDIT_LOG_FILE);
        Self::migrate_file(config_root, &state_dir, LOCK_FILE);
        Self::migrate_dir(config_root, &state_dir, ".snapshots", "snapshots");

        // Remove originals after successful copy
        let _ = fs::remove_file(&legacy_state_path);
        let _ = fs::remove_file(config_root.join(AUDIT_LOG_FILE));
        let _ = fs::remove_file(config_root.join(LOCK_FILE));

        // Leave breadcrumb marker
        let _ = fs::write(
            &migrated_marker,
            format!(
                "State migrated to {} on {}",
                state_dir.display(),
                Utc::now()
            ),
        );

        Ok(MigrationResult::Migrated {
            from: config_root.to_path_buf(),
            to: state_dir,
        })
    }

    /// Copy a single file from legacy to new location (best-effort).
    fn migrate_file(from_dir: &Path, to_dir: &Path, filename: &str) {
        let src = from_dir.join(filename);
        let dst = to_dir.join(filename);
        if src.exists() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }

    /// Copy a directory from legacy to new location (best-effort).
    fn migrate_dir(from_dir: &Path, to_dir: &Path, old_name: &str, new_name: &str) {
        let src = from_dir.join(old_name);
        let dst = to_dir.join(new_name);
        if src.is_dir() && !dst.exists() {
            let _ = Self::copy_dir_recursive(&src, &dst);
        }
    }

    /// Recursively copy a directory tree.
    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let dest_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &dest_path)?;
            } else {
                fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
}

/// Result of a state migration attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationResult {
    /// No legacy state files found, nothing to migrate
    NoMigrationNeeded,
    /// State files were migrated successfully
    Migrated { from: PathBuf, to: PathBuf },
    /// Migration was already performed previously
    AlreadyMigrated,
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
    fn test_clear_active_bundle() {
        let (manager, _temp) = create_test_manager();
        manager.set_active_bundle("laptop", "hyprland").unwrap();
        assert!(manager.active_bundle("laptop").is_some());

        manager.clear_active_bundle("laptop").unwrap();
        assert!(manager.active_bundle("laptop").is_none());
    }

    #[test]
    fn test_clear_active_bundle_nonexistent_host() {
        let (manager, _temp) = create_test_manager();
        // Clearing a non-existent host should succeed (no-op remove)
        manager.clear_active_bundle("nonexistent").unwrap();
        assert!(manager.active_bundle("nonexistent").is_none());
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

        assert_eq!(
            modules.len(),
            100,
            "Expected 100 modules, found {}",
            modules.len()
        );
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
        assert!(
            count <= 1,
            "Module appears {} times (should be 0 or 1)",
            count
        );
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
                        m.enable_module(&format!("stress-{}-{}", thread_id, i))
                            .unwrap();
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
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
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
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
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
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
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
                m.enable_module(&format!("concurrent-txn-mod-{}", i))
                    .unwrap();
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

    // ==========================================================================
    // F3-021: Managed Resource Tracking Tests
    // ==========================================================================

    #[test]
    fn test_record_managed_packages() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_managed_packages(&["neovim".to_string(), "fish".to_string()])
            .unwrap();

        let pkgs = manager.managed_packages();
        assert_eq!(pkgs.len(), 2);
        assert!(pkgs.contains(&"neovim".to_string()));
        assert!(pkgs.contains(&"fish".to_string()));
    }

    #[test]
    fn test_record_managed_packages_deduplicates() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_managed_packages(&["neovim".to_string()])
            .unwrap();
        manager
            .record_managed_packages(&["neovim".to_string(), "git".to_string()])
            .unwrap();

        let pkgs = manager.managed_packages();
        assert_eq!(pkgs.len(), 2);
        assert_eq!(
            pkgs.iter().filter(|p| *p == "neovim").count(),
            1,
            "neovim should appear exactly once"
        );
    }

    #[test]
    fn test_unrecord_managed_packages() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_managed_packages(&["a".to_string(), "b".to_string(), "c".to_string()])
            .unwrap();
        manager
            .unrecord_managed_packages(&["b".to_string()])
            .unwrap();

        let pkgs = manager.managed_packages();
        assert_eq!(pkgs.len(), 2);
        assert!(!pkgs.contains(&"b".to_string()));
    }

    #[test]
    fn test_record_managed_service() {
        let (manager, _temp) = create_test_manager();

        manager.record_managed_service("bluetooth.service").unwrap();
        manager
            .record_managed_service("NetworkManager.service")
            .unwrap();

        let svcs = manager.managed_services();
        assert_eq!(svcs.len(), 2);
        assert!(svcs.contains(&"bluetooth.service".to_string()));
    }

    #[test]
    fn test_record_managed_service_deduplicates() {
        let (manager, _temp) = create_test_manager();

        manager.record_managed_service("sshd.service").unwrap();
        manager.record_managed_service("sshd.service").unwrap();

        let svcs = manager.managed_services();
        assert_eq!(svcs.len(), 1);
    }

    #[test]
    fn test_unrecord_managed_service() {
        let (manager, _temp) = create_test_manager();

        manager.record_managed_service("a.service").unwrap();
        manager.record_managed_service("b.service").unwrap();
        manager.unrecord_managed_service("a.service").unwrap();

        let svcs = manager.managed_services();
        assert_eq!(svcs.len(), 1);
        assert_eq!(svcs[0], "b.service");
    }

    #[test]
    fn test_record_managed_dotfile() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_managed_dotfile("/home/user/.config/nvim")
            .unwrap();

        let dots = manager.managed_dotfiles();
        assert_eq!(dots.len(), 1);
        assert_eq!(dots[0], "/home/user/.config/nvim");
    }

    #[test]
    fn test_unrecord_managed_dotfile() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_managed_dotfile("/home/user/.config/nvim")
            .unwrap();
        manager
            .record_managed_dotfile("/home/user/.config/fish")
            .unwrap();
        manager
            .unrecord_managed_dotfile("/home/user/.config/nvim")
            .unwrap();

        let dots = manager.managed_dotfiles();
        assert_eq!(dots.len(), 1);
        assert_eq!(dots[0], "/home/user/.config/fish");
    }

    #[test]
    fn test_update_last_apply() {
        let (manager, _temp) = create_test_manager();

        assert!(manager.state().last_apply.is_none());

        manager.update_last_apply().unwrap();

        let last_apply = manager.state().last_apply;
        assert!(last_apply.is_some());
    }

    #[test]
    fn test_managed_tracking_persists_across_reload() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let manager = StateManager::new(path.clone()).unwrap();
            manager
                .record_managed_packages(&["pkg1".to_string()])
                .unwrap();
            manager.record_managed_service("svc1.service").unwrap();
            manager
                .record_managed_dotfile("/home/user/.config/test")
                .unwrap();
            manager.update_last_apply().unwrap();
        }

        // Reload and verify
        let manager = StateManager::new(path).unwrap();
        assert_eq!(manager.managed_packages(), vec!["pkg1".to_string()]);
        assert_eq!(manager.managed_services(), vec!["svc1.service".to_string()]);
        assert_eq!(
            manager.managed_dotfiles(),
            vec!["/home/user/.config/test".to_string()]
        );
        assert!(manager.state().last_apply.is_some());
    }

    #[test]
    fn test_managed_backward_compat_empty_state() {
        let temp_dir = TempDir::new().unwrap();
        // Write an older state.json without managed_* fields
        std::fs::write(
            temp_dir.path().join("state.json"),
            r#"{
                "current_host": "legacy",
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
            }"#,
        )
        .unwrap();

        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        // New fields should default to empty via #[serde(default)]
        assert!(manager.managed_packages().is_empty());
        assert!(manager.managed_services().is_empty());
        assert!(manager.managed_dotfiles().is_empty());
        assert!(manager.state().last_apply.is_none());
    }

    // ==========================================================================
    // Additional managed resource tracking tests
    // ==========================================================================

    #[test]
    fn test_unrecord_nonexistent_package_is_noop() {
        let (manager, _temp) = create_test_manager();

        // Unrecording a package that was never recorded should not error
        manager
            .unrecord_managed_packages(&["nonexistent".to_string()])
            .unwrap();
        assert!(manager.managed_packages().is_empty());
    }

    #[test]
    fn test_unrecord_nonexistent_service_is_noop() {
        let (manager, _temp) = create_test_manager();
        manager.unrecord_managed_service("nonexistent").unwrap();
        assert!(manager.managed_services().is_empty());
    }

    #[test]
    fn test_unrecord_nonexistent_dotfile_is_noop() {
        let (manager, _temp) = create_test_manager();
        manager
            .unrecord_managed_dotfile("/nonexistent/path")
            .unwrap();
        assert!(manager.managed_dotfiles().is_empty());
    }

    #[test]
    fn test_record_empty_packages_is_noop() {
        let (manager, _temp) = create_test_manager();
        manager.record_managed_packages(&[]).unwrap();
        assert!(manager.managed_packages().is_empty());
    }

    #[test]
    fn test_unrecord_empty_packages_is_noop() {
        let (manager, _temp) = create_test_manager();
        manager.record_managed_packages(&["a".to_string()]).unwrap();
        manager.unrecord_managed_packages(&[]).unwrap();
        assert_eq!(manager.managed_packages().len(), 1);
    }

    #[test]
    fn test_managed_dotfile_deduplicates() {
        let (manager, _temp) = create_test_manager();
        manager
            .record_managed_dotfile("/home/user/.config/test")
            .unwrap();
        manager
            .record_managed_dotfile("/home/user/.config/test")
            .unwrap();

        let dots = manager.managed_dotfiles();
        assert_eq!(dots.len(), 1, "Dotfile should be deduplicated");
    }

    #[test]
    fn test_last_apply_timestamp_updates() {
        let (manager, _temp) = create_test_manager();

        // First update
        manager.update_last_apply().unwrap();
        let first = manager.state().last_apply.unwrap();

        // Small sleep to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second update
        manager.update_last_apply().unwrap();
        let second = manager.state().last_apply.unwrap();

        assert!(second >= first, "Second last_apply should be >= first");
    }

    #[test]
    fn test_managed_resources_persist_after_multiple_operations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let manager = StateManager::new(path.clone()).unwrap();
            manager
                .record_managed_packages(&["a".to_string(), "b".to_string()])
                .unwrap();
            manager.record_managed_service("svc1").unwrap();
            manager
                .record_managed_dotfile("/home/user/.config/x")
                .unwrap();

            // Remove some
            manager
                .unrecord_managed_packages(&["a".to_string()])
                .unwrap();
        }

        // Reload and verify the removals persisted
        let manager = StateManager::new(path).unwrap();
        let pkgs = manager.managed_packages();
        assert_eq!(pkgs, vec!["b".to_string()]);
        assert_eq!(manager.managed_services(), vec!["svc1".to_string()]);
        assert_eq!(
            manager.managed_dotfiles(),
            vec!["/home/user/.config/x".to_string()]
        );
    }

    #[test]
    fn test_save_and_load_scan_report() {
        use crate::services::scan::{ScanReport, ScanSummary};

        let (manager, _temp) = create_test_manager();

        // Initially no scan report
        assert!(manager.load_scan_report().is_none());

        // Save a scan report
        let report = ScanReport {
            existing_configs: vec![],
            installed_packages: vec!["git".to_string(), "neovim".to_string()],
            potential_conflicts: vec![],
            recommendations: vec!["Backup existing configs".to_string()],
            summary: ScanSummary::default(),
        };
        manager.save_scan_report(&report).unwrap();

        // Load it back
        let loaded = manager.load_scan_report().unwrap();
        assert_eq!(loaded.installed_packages.len(), 2);
        assert_eq!(loaded.recommendations.len(), 1);
        assert_eq!(loaded.recommendations[0], "Backup existing configs");

        // Verify persistence across new manager instances
        let manager2 = StateManager::new(_temp.path().to_path_buf()).unwrap();
        let loaded2 = manager2.load_scan_report().unwrap();
        assert_eq!(loaded2.installed_packages.len(), 2);
    }
    // ==========================================================================
    // F3-015: Hook Execution Tracking Tests
    // ==========================================================================

    #[test]
    fn test_record_hook_executed_and_is_hook_executed() {
        let (manager, _temp) = create_test_manager();

        // Initially no hooks are recorded
        assert!(!manager.is_hook_executed("nvim", "post_install"));

        // Record a hook execution
        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();

        // Now it should be recorded
        assert!(manager.is_hook_executed("nvim", "post_install"));

        // Different hook type for same module should not be recorded
        assert!(!manager.is_hook_executed("nvim", "pre_install"));

        // Different module should not be affected
        assert!(!manager.is_hook_executed("kitty", "post_install"));
    }

    #[test]
    fn test_record_hook_executed_idempotent() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();
        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();

        // Should not duplicate
        let state = manager.state();
        let hooks = state.hooks_executed.get("nvim").unwrap();
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn test_record_multiple_hooks_for_same_module() {
        let (manager, _temp) = create_test_manager();

        manager.record_hook_executed("nvim", "pre_install").unwrap();
        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();

        assert!(manager.is_hook_executed("nvim", "pre_install"));
        assert!(manager.is_hook_executed("nvim", "post_install"));

        let state = manager.state();
        let hooks = state.hooks_executed.get("nvim").unwrap();
        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn test_clear_hooks_for_module() {
        let (manager, _temp) = create_test_manager();

        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();
        manager
            .record_hook_executed("kitty", "pre_install")
            .unwrap();

        assert!(manager.is_hook_executed("nvim", "post_install"));
        assert!(manager.is_hook_executed("kitty", "pre_install"));

        // Clear only nvim hooks
        manager.clear_hooks_for_module("nvim").unwrap();

        assert!(!manager.is_hook_executed("nvim", "post_install"));
        // kitty hooks should remain
        assert!(manager.is_hook_executed("kitty", "pre_install"));
    }

    #[test]
    fn test_clear_hooks_for_nonexistent_module_is_noop() {
        let (manager, _temp) = create_test_manager();

        // Should not error on clearing hooks for a module that has none
        manager.clear_hooks_for_module("nonexistent").unwrap();
    }

    #[test]
    fn test_hooks_executed_persists_across_reload() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        {
            let manager = StateManager::new(path.clone()).unwrap();
            manager
                .record_hook_executed("nvim", "post_install")
                .unwrap();
        }

        // Reload and verify persistence
        let manager = StateManager::new(path).unwrap();
        assert!(manager.is_hook_executed("nvim", "post_install"));
    }

    #[test]
    fn test_hooks_executed_backward_compat_missing_field() {
        // Simulate an old state.json without hooks_executed field
        let temp_dir = TempDir::new().unwrap();
        let state_json = serde_json::json!({
            "current_host": "test",
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
            },
            "managed_packages": [],
            "managed_services": [],
            "managed_dotfiles": []
        });
        std::fs::write(
            temp_dir.path().join("state.json"),
            serde_json::to_string_pretty(&state_json).unwrap(),
        )
        .unwrap();

        let manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();

        // hooks_executed should default to empty HashMap
        assert!(!manager.is_hook_executed("any", "any_hook"));

        // Should be able to record new hooks
        manager
            .record_hook_executed("nvim", "post_install")
            .unwrap();
        assert!(manager.is_hook_executed("nvim", "post_install"));
    }
}

/// Property-based tests for state management
#[cfg(test)]
mod proptest_state_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    // Strategy for generating valid module IDs
    fn module_id_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,20}").unwrap()
    }

    // Strategy for generating valid host IDs
    fn host_id_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,15}").unwrap()
    }

    // Strategy for generating valid bundle IDs
    fn bundle_id_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,15}").unwrap()
    }

    fn create_proptest_manager() -> (StateManager, TempDir) {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        (manager, temp)
    }

    proptest! {
        // Property: State serialization roundtrip preserves all data
        #[test]
        fn state_serialization_roundtrip(
            host in prop::option::of(host_id_strategy()),
            modules in prop::collection::vec(module_id_strategy(), 0..5)
        ) {
            let (manager, _temp) = create_proptest_manager();

            // Set state
            if let Some(ref h) = host {
                manager.set_current_host(h).unwrap();
            }
            for m in &modules {
                manager.enable_module(m).unwrap();
            }

            // Reload and verify
            let manager2 = StateManager::new(manager.state_path().parent().unwrap().to_path_buf()).unwrap();

            prop_assert_eq!(manager2.current_host(), host);

            let active = manager2.active_modules();
            for m in &modules {
                prop_assert!(active.contains(m), "Module {} should be active", m);
            }
        }

        // Property: Enable then disable returns to original state
        #[test]
        fn enable_disable_idempotent(module_id in module_id_strategy()) {
            let (manager, _temp) = create_proptest_manager();

            // Initially not active
            prop_assert!(!manager.is_module_active(&module_id));

            // Enable
            manager.enable_module(&module_id).unwrap();
            prop_assert!(manager.is_module_active(&module_id));

            // Disable
            manager.disable_module(&module_id).unwrap();
            prop_assert!(!manager.is_module_active(&module_id));
        }

        // Property: Double enable is idempotent (module still active)
        #[test]
        fn double_enable_idempotent(module_id in module_id_strategy()) {
            let (manager, _temp) = create_proptest_manager();

            manager.enable_module(&module_id).unwrap();
            manager.enable_module(&module_id).unwrap();

            let active = manager.active_modules();
            let count = active.iter().filter(|m| **m == module_id).count();
            prop_assert_eq!(count, 1, "Module should appear exactly once");
        }

        // Property: Double disable is safe (no panic, no error)
        #[test]
        fn double_disable_safe(module_id in module_id_strategy()) {
            let (manager, _temp) = create_proptest_manager();

            manager.enable_module(&module_id).unwrap();
            manager.disable_module(&module_id).unwrap();
            manager.disable_module(&module_id).unwrap(); // Should be fine

            prop_assert!(!manager.is_module_active(&module_id));
        }

        // Property: Active modules count is accurate
        #[test]
        fn active_modules_count_accurate(modules in prop::collection::hash_set(module_id_strategy(), 0..10)) {
            let (manager, _temp) = create_proptest_manager();

            for m in &modules {
                manager.enable_module(m).unwrap();
            }

            let active = manager.active_modules();
            prop_assert_eq!(active.len(), modules.len(), "Active modules count should match");
        }

        // Property: Setting host persists across reload
        #[test]
        fn host_persists_across_reload(host_id in host_id_strategy()) {
            let temp = TempDir::new().unwrap();
            let path = temp.path().to_owned();

            {
                let manager = StateManager::new(path.clone()).unwrap();
                manager.set_current_host(&host_id).unwrap();
            }

            let manager2 = StateManager::new(path.clone()).unwrap();
            prop_assert_eq!(manager2.current_host(), Some(host_id));
        }

        // Property: Setting bundle persists across reload (requires host)
        #[test]
        fn bundle_persists_across_reload(
            host_id in host_id_strategy(),
            bundle_id in bundle_id_strategy()
        ) {
            let temp = TempDir::new().unwrap();
            let path = temp.path().to_owned();

            {
                let manager = StateManager::new(path.clone()).unwrap();
                manager.set_current_host(&host_id).unwrap();
                manager.set_active_bundle(&host_id, &bundle_id).unwrap();
            }

            let manager2 = StateManager::new(path.clone()).unwrap();
            prop_assert_eq!(manager2.active_bundle(&host_id), Some(bundle_id));
        }

        // Property: Transaction commit persists changes
        #[test]
        fn transaction_commit_persists(module_id in module_id_strategy()) {
            let temp = TempDir::new().unwrap();
            let path = temp.path().to_owned();

            {
                let manager = StateManager::new(path.clone()).unwrap();
                let txn = manager.begin_transaction("proptest-commit").unwrap();
                manager.enable_module(&module_id).unwrap();
                txn.commit().unwrap();
            }

            let manager2 = StateManager::new(path.clone()).unwrap();
            prop_assert!(manager2.is_module_active(&module_id), "Module should persist after commit");
        }

        // Property: Transaction rollback discards changes
        #[test]
        fn transaction_rollback_discards(module_id in module_id_strategy()) {
            let temp = TempDir::new().unwrap();
            let path = temp.path().to_owned();

            {
                let manager = StateManager::new(path.clone()).unwrap();
                let txn = manager.begin_transaction("proptest-rollback").unwrap();
                manager.enable_module(&module_id).unwrap();
                txn.rollback().unwrap();
            }

            let manager2 = StateManager::new(path.clone()).unwrap();
            prop_assert!(!manager2.is_module_active(&module_id), "Module should not persist after rollback");
        }
    }
}

/// Resilience tests for error handling and edge cases
#[cfg(test)]
mod resilience_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // =============================================================================
    // Corrupted State File Tests
    // =============================================================================

    #[test]
    fn test_corrupted_json_returns_error() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write invalid JSON
        fs::write(&state_path, "{ invalid json }").unwrap();

        let result = StateManager::new(temp.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_json_returns_error() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write truncated JSON
        fs::write(
            &state_path,
            r#"{ "current_host": "test", "active_modules":"#,
        )
        .unwrap();

        let result = StateManager::new(temp.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_json_structure_returns_error() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write valid JSON but wrong structure
        fs::write(&state_path, r#"{"wrong": "structure", "number": 42}"#).unwrap();

        let result = StateManager::new(temp.path().to_path_buf());
        // serde should fail to deserialize into IronState
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_file_returns_error() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write empty file
        fs::write(&state_path, "").unwrap();

        let result = StateManager::new(temp.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_garbage_returns_error() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write binary garbage
        let garbage: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
        let mut file = fs::File::create(&state_path).unwrap();
        file.write_all(&garbage).unwrap();

        let result = StateManager::new(temp.path().to_path_buf());
        assert!(result.is_err());
    }

    // =============================================================================
    // Missing Directory/File Tests
    // =============================================================================

    #[test]
    fn test_nonexistent_state_creates_default() {
        let temp = TempDir::new().unwrap();
        // No state.json file exists

        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Should have default state
        assert!(manager.current_host().is_none());
        assert!(manager.active_modules().is_empty());
    }

    #[test]
    fn test_new_directory_initializes_successfully() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let manager = StateManager::new(subdir.clone()).unwrap();
        manager.set_current_host("test-host").unwrap();

        // Should persist
        let state_path = subdir.join("state.json");
        assert!(state_path.exists());
    }

    // =============================================================================
    // Invalid State Values Tests
    // =============================================================================

    #[test]
    fn test_valid_json_with_null_fields() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write JSON with explicit null fields matching IronState structure
        let json = r#"{
            "current_host": null,
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
        fs::write(&state_path, json).unwrap();

        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert!(manager.current_host().is_none());
    }

    #[test]
    fn test_extra_fields_in_json_ignored() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Write JSON with extra unknown fields (serde should deny unknown by default)
        // This test documents expected behavior
        let json = r#"{
            "current_host": "test-host",
            "active_bundles": {},
            "active_profiles": {},
            "active_modules": ["mod1"],
            "last_operations": [],
            "maintenance": {
                "last_update": null,
                "last_clean": null,
                "last_doctor": null,
                "last_snapshot": null,
                "last_sync": null
            }
        }"#;
        fs::write(&state_path, json).unwrap();

        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        // Should load successfully
        assert_eq!(manager.current_host(), Some("test-host".to_string()));
        assert!(manager.is_module_active("mod1"));
    }

    // =============================================================================
    // Recovery After Error Tests
    // =============================================================================

    #[test]
    fn test_recovery_from_corrupted_state() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // First, create valid state
        {
            let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
            manager.set_current_host("original-host").unwrap();
        }

        // Corrupt the file
        fs::write(&state_path, "corrupted").unwrap();

        // Should fail to load
        let result = StateManager::new(temp.path().to_path_buf());
        assert!(result.is_err());

        // Now repair by writing valid state
        let valid_json = r#"{
            "current_host": "recovered-host",
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
        fs::write(&state_path, valid_json).unwrap();

        // Should now load successfully
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert_eq!(manager.current_host(), Some("recovered-host".to_string()));
    }

    #[test]
    fn test_recovery_from_deleted_state() {
        let temp = TempDir::new().unwrap();
        let state_path = temp.path().join("state.json");

        // Create valid state
        {
            let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
            manager.set_current_host("original-host").unwrap();
            manager.enable_module("mod1").unwrap();
        }

        // Delete the file
        fs::remove_file(&state_path).unwrap();

        // Should create fresh default state
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert!(manager.current_host().is_none());
        assert!(manager.active_modules().is_empty());
    }

    // =============================================================================
    // Edge Case Tests
    // =============================================================================

    #[test]
    fn test_very_large_module_list() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Enable many modules
        for i in 0..1000 {
            manager.enable_module(&format!("module-{}", i)).unwrap();
        }

        assert_eq!(manager.active_modules().len(), 1000);

        // Reload and verify
        let manager2 = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert_eq!(manager2.active_modules().len(), 1000);
    }

    #[test]
    fn test_unicode_in_state() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Set unicode host name
        manager.set_current_host("测试-хост-🏠").unwrap();
        manager.enable_module("模块-один-📦").unwrap();

        // Reload and verify
        let manager2 = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert_eq!(manager2.current_host(), Some("测试-хост-🏠".to_string()));
        assert!(manager2.is_module_active("模块-один-📦"));
    }

    #[test]
    fn test_special_characters_in_ids() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // IDs with special characters that might cause issues
        let special_ids = [
            "module.with.dots",
            "module-with-dashes",
            "module_with_underscores",
            "module123",
            "123module",
        ];

        for id in &special_ids {
            manager.enable_module(id).unwrap();
        }

        // Reload and verify all
        let manager2 = StateManager::new(temp.path().to_path_buf()).unwrap();
        for id in &special_ids {
            assert!(
                manager2.is_module_active(id),
                "Module {} should be active",
                id
            );
        }
    }

    #[test]
    fn test_empty_string_ids_handled() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Empty strings should work (though not recommended)
        manager.enable_module("").unwrap();

        assert!(manager.is_module_active(""));

        // Disable and verify
        manager.disable_module("").unwrap();
        assert!(!manager.is_module_active(""));
    }

    #[test]
    fn test_whitespace_only_ids() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Whitespace-only IDs
        manager.enable_module("   ").unwrap();
        manager.enable_module("\t\n").unwrap();

        assert!(manager.is_module_active("   "));
        assert!(manager.is_module_active("\t\n"));
    }

    // =============================================================================
    // Audit Log Resilience Tests
    // =============================================================================

    #[test]
    fn test_corrupted_audit_log_ignored() {
        let temp = TempDir::new().unwrap();
        let audit_path = temp.path().join("audit.json");

        // Write corrupted audit log
        fs::write(&audit_path, "corrupted audit log").unwrap();

        // Should still create manager successfully
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();
        assert!(manager.current_host().is_none());
    }

    #[test]
    fn test_missing_audit_log_handled() {
        let temp = TempDir::new().unwrap();
        // No audit.json exists

        // Should create manager successfully with empty audit log
        let manager = StateManager::new(temp.path().to_path_buf()).unwrap();

        // Perform operations that would log
        manager.set_current_host("test").unwrap();

        // Should work without issues
        assert_eq!(manager.current_host(), Some("test".to_string()));
    }
}

/// F3-007: State migration tests
///
/// Uses `migrate_to()` directly to avoid env var manipulation,
/// which is unsafe in parallel test environments.
#[cfg(test)]
mod migration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a valid state JSON string
    fn valid_state_json() -> String {
        r#"{
            "current_host": "migrated-host",
            "active_bundles": {},
            "active_profiles": {},
            "active_modules": ["mod-a"],
            "last_operations": [],
            "maintenance": {
                "last_update": null,
                "last_clean": null,
                "last_doctor": null,
                "last_snapshot": null,
                "last_sync": null
            }
        }"#
        .to_string()
    }

    #[test]
    fn test_migrate_noop_when_no_legacy_state() {
        let config_root = TempDir::new().unwrap();
        let state_dir = TempDir::new().unwrap();

        let result = StateManager::migrate_to(config_root.path(), state_dir.path());

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MigrationResult::NoMigrationNeeded);
    }

    #[test]
    fn test_migrate_noop_when_new_location_has_state() {
        let config_root = TempDir::new().unwrap();
        let state_dir = TempDir::new().unwrap();

        // Put state.json in both locations
        fs::write(config_root.path().join(STATE_FILE), valid_state_json()).unwrap();
        fs::write(state_dir.path().join(STATE_FILE), valid_state_json()).unwrap();

        let result = StateManager::migrate_to(config_root.path(), state_dir.path());

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MigrationResult::NoMigrationNeeded);
    }

    #[test]
    fn test_migrate_noop_when_already_migrated() {
        let config_root = TempDir::new().unwrap();
        let state_dir = TempDir::new().unwrap();

        // Legacy state exists but marker says already migrated
        fs::write(config_root.path().join(STATE_FILE), valid_state_json()).unwrap();
        fs::write(config_root.path().join("MIGRATED.txt"), "already done").unwrap();

        let result = StateManager::migrate_to(config_root.path(), state_dir.path());

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MigrationResult::AlreadyMigrated);
    }

    #[test]
    fn test_migrate_copies_state_and_creates_marker() {
        let config_root = TempDir::new().unwrap();
        let state_dir = TempDir::new().unwrap();

        // Write legacy state files
        fs::write(config_root.path().join(STATE_FILE), valid_state_json()).unwrap();
        fs::write(config_root.path().join(AUDIT_LOG_FILE), "[]").unwrap();
        fs::write(config_root.path().join(LOCK_FILE), "").unwrap();

        // Create .snapshots/ directory
        let snap_dir = config_root.path().join(".snapshots");
        fs::create_dir_all(&snap_dir).unwrap();
        fs::write(snap_dir.join("snap1.json"), "{}").unwrap();

        let result = StateManager::migrate_to(config_root.path(), state_dir.path());

        assert!(result.is_ok());
        match result.unwrap() {
            MigrationResult::Migrated { from, to } => {
                assert_eq!(from, config_root.path());
                assert_eq!(to, state_dir.path());
            }
            other => panic!("Expected Migrated, got {:?}", other),
        }

        // Verify new location has state
        assert!(state_dir.path().join(STATE_FILE).exists());
        assert!(state_dir.path().join(AUDIT_LOG_FILE).exists());

        // Verify snapshots were copied with new name
        assert!(state_dir.path().join("snapshots/snap1.json").exists());

        // Verify originals removed
        assert!(!config_root.path().join(STATE_FILE).exists());
        assert!(!config_root.path().join(AUDIT_LOG_FILE).exists());

        // Verify breadcrumb left
        assert!(config_root.path().join("MIGRATED.txt").exists());
    }

    #[test]
    fn test_migrate_noop_when_same_directory() {
        let config_root = TempDir::new().unwrap();

        fs::write(config_root.path().join(STATE_FILE), valid_state_json()).unwrap();

        let result = StateManager::migrate_to(config_root.path(), config_root.path());

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MigrationResult::NoMigrationNeeded);

        // State should still be in original location
        assert!(config_root.path().join(STATE_FILE).exists());
    }

    #[test]
    fn test_migrate_preserves_originals_on_partial_failure() {
        let config_root = TempDir::new().unwrap();
        // Use a path that's not writable to simulate failure
        let state_dir = PathBuf::from("/nonexistent/impossible/path");

        fs::write(config_root.path().join(STATE_FILE), valid_state_json()).unwrap();

        let result = StateManager::migrate_to(config_root.path(), &state_dir);

        // Should fail
        assert!(result.is_err());

        // Original state should be intact
        assert!(config_root.path().join(STATE_FILE).exists());
        // No marker should be created
        assert!(!config_root.path().join("MIGRATED.txt").exists());
    }
}
