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
    pub fn new(root: &str, format: OutputFormat, verbose: bool, quiet: bool, no_color: bool) -> Result<Self> {
        let root = expand_home(Path::new(root));

        let state = StateManager::new(root.clone())
            .context("Failed to initialize state manager")?;

        let output = Output::new(format, verbose, quiet, no_color);

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

    /// Get sync service
    pub fn sync_service(&self) -> impl SyncService {
        DefaultSyncService::new(&self.root, self.state.clone())
    }

    /// Get secrets service
    pub fn secrets_service(&self) -> impl SecretsService {
        DefaultSecretsService::new(&self.root)
    }

    /// Get update service
    pub fn update_service(&self) -> impl UpdateService {
        // Use noop snapshot manager for now
        // TODO: Detect and use timeshift/snapper
        let snapshot_manager = NoopManager;
        DefaultUpdateService::new(self.state.clone(), snapshot_manager)
    }

    /// Get recovery service
    pub fn recovery_service(&self) -> impl RecoveryService {
        let snapshot_manager = NoopManager;
        DefaultRecoveryService::new(&self.root, self.state.clone(), snapshot_manager)
    }

    /// Check if Iron is initialized
    pub fn is_initialized(&self) -> bool {
        self.root.join("state.json").exists() || self.state.current_host().is_some()
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
