//! Application Context
//!
//! Provides access to services and configuration for commands.

use crate::cli::OutputFormat;
use crate::output::Output;
use anyhow::{Context, Result};
use iron_core::services::bundle::{BundleService, DefaultBundleService};
use iron_core::services::host::{DefaultHostService, HostService};
use iron_core::services::module::{DefaultModuleService, ModuleService};
use iron_core::services::profile::{DefaultProfileService, ProfileService};
use iron_core::services::recovery::{DefaultRecoveryService, RecoveryService};
use iron_core::services::secrets::{DefaultSecretsService, SecretsService};
use iron_core::services::state::StateManager;
use iron_core::services::sync::{DefaultSyncService, SyncService};
use iron_core::services::update::{DefaultUpdateService, UpdateService};
use iron_core::snapshot::NoopManager;
use iron_core::validation::expand_home;
use std::path::{Path, PathBuf};

/// Application context containing services
pub struct AppContext {
    /// Iron root directory
    pub root: PathBuf,
    /// State manager
    pub state: StateManager,
    /// Output formatter
    pub output: Output,
}

impl AppContext {
    /// Create a new application context
    pub fn new(
        root: &str,
        format: OutputFormat,
        verbose: bool,
        quiet: bool,
        no_color: bool,
        explain: bool,
    ) -> Result<Self> {
        let root = expand_home(Path::new(root));

        // F3-007: Migrate legacy state files before initializing StateManager
        match StateManager::migrate_if_needed(&root) {
            Ok(iron_core::services::state::MigrationResult::Migrated { ref from, ref to }) => {
                if verbose {
                    eprintln!("Migrated state from {} to {}", from.display(), to.display());
                }
            }
            Ok(_) => {} // NoMigrationNeeded or AlreadyMigrated
            Err(e) => {
                // Migration failure is non-fatal — warn and continue
                eprintln!("Warning: state migration: {}", e);
            }
        }

        let state =
            StateManager::new(root.clone()).context("Failed to initialize state manager")?;

        let output = Output::new(format, verbose, quiet, no_color).with_explain(explain);

        Ok(Self {
            root,
            state,
            output,
        })
    }

    /// Get host service
    pub fn host_service(&self) -> impl HostService {
        DefaultHostService::new(&self.root)
    }

    /// Get bundle service
    pub fn bundle_service(&self) -> impl BundleService {
        DefaultBundleService::new(&self.root, self.state.clone())
    }

    /// Get module service
    pub fn module_service(&self) -> impl ModuleService {
        DefaultModuleService::new(&self.root, self.state.clone())
    }

    /// Get profile service
    pub fn profile_service(&self) -> impl ProfileService {
        let module_service = self.module_service();
        DefaultProfileService::new(&self.root, self.state.clone(), module_service)
    }

    /// Get sync service (resilient via CommandExecutor + pre-push secrets lock)
    pub fn sync_service(&self) -> impl SyncService {
        let mut svc = DefaultSyncService::with_resilience(&self.root, self.state.clone());

        // Wire secrets service so push auto-locks secrets first (A-010)
        if self.root.join(".git").exists() {
            let secrets = std::sync::Arc::new(self.secrets_service_inner());
            svc = svc.with_secrets_service(secrets);
        }

        svc
    }

    /// Get secrets service
    ///
    /// When the root is a git repository, injects the resilient
    /// `DefaultSecretsManager` from iron-git as a `SecretsBackend`.
    pub fn secrets_service(&self) -> impl SecretsService {
        self.secrets_service_inner()
    }

    /// Internal helper returning a concrete type so it can be wrapped in `Arc<dyn>`.
    fn secrets_service_inner(&self) -> DefaultSecretsService {
        let mut svc = DefaultSecretsService::new(&self.root);

        // Wire in the resilient backend when a .git dir exists
        if self.root.join(".git").exists() {
            let mgr = iron_git::DefaultSecretsManager::new(self.root.clone());
            svc = svc.with_backend(Box::new(mgr));
        }

        svc
    }

    /// Get update service
    pub fn update_service(&self) -> impl UpdateService {
        // Use noop snapshot manager for now
        // TODO: Detect and use timeshift/snapper
        let snapshot_manager = NoopManager;
        DefaultUpdateService::new(self.state.clone(), snapshot_manager)
    }

    /// Get recovery service (C-009: with package/service managers for full import)
    pub fn recovery_service(&self) -> impl RecoveryService {
        let snapshot_manager = NoopManager;
        DefaultRecoveryService::new(&self.root, self.state.clone(), snapshot_manager)
            .with_package_manager(std::sync::Arc::new(
                iron_pacman::DefaultPackageManager::new(),
            ))
    }

    /// Get apply service (F1-005)
    pub fn apply_service(&self) -> iron_core::services::apply::DefaultApplyService {
        iron_core::services::apply::DefaultApplyService::new(
            &self.root,
            self.state.clone(),
            std::sync::Arc::new(iron_pacman::DefaultPackageManager::new()),
            std::sync::Arc::new(iron_systemd::SystemdServiceAdapter::user()),
        )
    }

    /// Get drift service (F1-011)
    pub fn drift_service(&self) -> iron_core::services::drift::DefaultDriftService {
        iron_core::services::drift::DefaultDriftService::new(
            &self.root,
            self.state.clone(),
            std::sync::Arc::new(iron_pacman::DefaultPackageManager::new()),
            std::sync::Arc::new(iron_systemd::SystemdServiceAdapter::user()),
        )
    }

    /// Get security service (F2-016)
    pub fn security_service(&self) -> iron_core::services::security::DefaultSecurityService {
        iron_core::services::security::DefaultSecurityService::new(&self.root, self.state.clone())
    }

    /// Get snapshot service (F2-001)
    pub fn snapshot_service(
        &self,
    ) -> iron_core::services::snapshot_service::DefaultSnapshotService {
        iron_core::services::snapshot_service::DefaultSnapshotService::new(
            &self.root,
            self.state.clone(),
        )
        .with_package_manager(std::sync::Arc::new(
            iron_pacman::DefaultPackageManager::new(),
        ))
    }

    /// Check if Iron is initialized
    pub fn is_initialized(&self) -> bool {
        // F3-006: Check state directory instead of config root
        self.state.state_root().join("state.json").exists() || self.state.current_host().is_some()
    }

    /// Get current host ID
    pub fn current_host(&self) -> Option<String> {
        self.state.current_host()
    }
}

/// Ensure Iron is initialized, or return error
pub fn require_init(ctx: &AppContext) -> Result<()> {
    if !ctx.is_initialized() {
        anyhow::bail!("Iron not initialized. Run 'iron init' first.");
    }
    Ok(())
}
