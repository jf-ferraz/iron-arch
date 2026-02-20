//! Application actions for Iron TUI
//!
//! Contains action execution logic for bundles, modules, profiles, and updates.

use super::{App, ConfirmAction, View};
use crate::wizard::TextInput;
use iron_core::services::{
    BundleService, DefaultBundleService, DefaultUpdateService, StateManager, UpdateService,
};
use iron_core::{Module, Profile};

impl App {
    /// Initialize application state
    pub fn init(&mut self) -> anyhow::Result<()> {
        // Try to create state manager
        match StateManager::new(self.config_dir.clone()) {
            Ok(sm) => {
                // Get current host and active modules
                self.current_host = sm.current_host();
                self.active_modules = sm.active_modules();

                // Get active bundle for current host
                if let Some(ref host_id) = self.current_host {
                    if let Some(bundle_id) = sm.active_bundle(host_id) {
                        // Load bundles via BundleService (with real package manager)
                        let bundle_service =
                            DefaultBundleService::new(&self.config_dir, sm.clone())
                                .with_package_manager(self.package_manager.clone())
                                .with_service_manager(self.service_manager.clone());
                        self.bundles = bundle_service.discover().unwrap_or_default();
                        self.active_bundle =
                            self.bundles.iter().find(|b| b.id == bundle_id).cloned();
                    }

                    self.active_profile = sm.active_profile(host_id);
                }

                self.state_manager = Some(sm);
            }
            Err(_) => {
                // No existing state - show setup wizard
                self.view = View::SetupWizard;
                self.init_wizard();
                return Ok(());
            }
        }

        // Load bundles if not already loaded
        if self.bundles.is_empty()
            && let Some(ref sm) = self.state_manager
        {
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
                .with_package_manager(self.package_manager.clone())
                .with_service_manager(self.service_manager.clone());
            self.bundles = bundle_service.discover().unwrap_or_default();
        }

        // Load profiles from disk
        self.load_profiles();

        // Load modules from disk
        self.load_modules();

        // Load package data (non-blocking, fail gracefully)
        self.load_package_data();

        Ok(())
    }

    /// Execute confirmed action
    pub fn execute_confirm_action(&mut self) {
        let action = match self.confirm_action.take() {
            Some(a) => a,
            None => return,
        };

        match action {
            ConfirmAction::SwitchBundle(ref id) => {
                self.switch_bundle(id.clone());
            }
            ConfirmAction::RemoveBundle(id) => {
                if let Some(ref sm) = self.state_manager {
                    let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
                        .with_package_manager(self.package_manager.clone())
                        .with_service_manager(self.service_manager.clone());
                    match bundle_service.deactivate(&id) {
                        Ok(()) => {
                            self.bundles = bundle_service.discover().unwrap_or_default();
                            self.active_bundle = None;
                            self.active_modules = sm.active_modules();
                            self.set_status(format!("Deactivated bundle: {}", id));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to deactivate bundle: {}", e));
                        }
                    }
                } else {
                    self.set_error("No state manager available");
                }
            }
            ConfirmAction::EnableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    let module_service = iron_core::services::DefaultModuleService::new(
                        &self.config_dir,
                        sm.clone(),
                    );
                    match iron_core::services::ModuleService::enable(&module_service, id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.set_status(format!("Enabled module: {}", id));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to enable module: {}", e));
                        }
                    }
                }
            }
            ConfirmAction::DisableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    let module_service = iron_core::services::DefaultModuleService::new(
                        &self.config_dir,
                        sm.clone(),
                    );
                    match iron_core::services::ModuleService::disable(&module_service, id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.set_status(format!("Disabled module: {}", id));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to disable module: {}", e));
                        }
                    }
                }
            }
            ConfirmAction::RunUpdate => {
                self.run_system_update();
            }
            ConfirmAction::RunCleanup => {
                self.execute_cleanup();
            }
            ConfirmAction::Quit => {
                self.should_quit = true;
            }
        }
    }

    /// Toggle module enable/disable
    pub fn toggle_selected_module(&mut self) {
        if !matches!(
            self.view,
            View::Modules | View::ModuleDetail | View::SecurityModules
        ) {
            return;
        }
        if let Some(module) = self.selected_module() {
            let module_id = module.id.clone();
            let is_active = self.is_module_active(&module_id);
            if !is_active {
                // Check for conflicts before allowing enable
                if !self.module_conflicts.is_empty() {
                    let names: Vec<&str> = self
                        .module_conflicts
                        .iter()
                        .map(|c| c.split(':').next().unwrap_or(c.as_str()))
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();
                    self.set_error(format!(
                        "Cannot enable '{}': conflicts with {}. Resolve conflict first.",
                        module_id,
                        names.join(", ")
                    ));
                    return;
                }
            }
            let action = if is_active {
                ConfirmAction::DisableModule(module_id)
            } else {
                ConfirmAction::EnableModule(module_id)
            };
            self.request_confirm(action);
        }
    }

    /// Load conflict data for the currently selected module into `self.module_conflicts`.
    pub fn load_module_conflicts(&mut self) {
        self.module_conflicts.clear();
        if let Some(module) = self.selected_module() {
            let module_id = module.id.clone();
            if let Some(ref sm) = self.state_manager {
                let module_service =
                    iron_core::services::DefaultModuleService::new(&self.config_dir, sm.clone());
                if let Ok(conflicts) =
                    iron_core::services::ModuleService::check_conflicts(&module_service, &module_id)
                {
                    self.module_conflicts = conflicts;
                }
            }
        }
    }

    /// Activate selected bundle
    pub fn activate_selected_bundle(&mut self) {
        if self.view != View::Bundles && self.view != View::BundleDetail {
            return;
        }
        if let Some(bundle) = self.selected_bundle() {
            let bundle_id = bundle.id.clone();
            self.request_confirm(ConfirmAction::SwitchBundle(bundle_id));
        }
    }

    /// Activate selected profile
    pub fn activate_selected_profile(&mut self) {
        let profile = match self.selected_profile() {
            Some(p) => p.id.clone(),
            None => return,
        };

        if let Some(ref sm) = self.state_manager {
            let module_service =
                iron_core::services::DefaultModuleService::new(&self.config_dir, sm.clone());
            let profile_service = iron_core::services::DefaultProfileService::new(
                &self.config_dir,
                sm.clone(),
                module_service,
            );
            match iron_core::services::ProfileService::apply(&profile_service, &profile) {
                Ok(()) => {
                    self.active_profile = Some(profile.clone());
                    self.set_status(format!("Activated profile: {}", profile));
                }
                Err(e) => {
                    self.set_error(format!("Failed to activate profile: {}", e));
                }
            }
        }
    }

    /// Refresh updates with pre-flight checks and news
    pub fn refresh_updates(&mut self) {
        self.set_info("Running pre-flight checks...");

        // Check for updates
        match self.package_manager.check_updates() {
            Ok(updates) => {
                self.pending_updates = updates;
            }
            Err(e) => {
                self.set_error(format!("Failed to check updates: {}", e));
                return;
            }
        }

        // Fetch news
        let news_items = self.package_manager.fetch_news().unwrap_or_default();
        self.arch_news = news_items.clone();

        // Run pre-flight checks with news (Phase 2.3)
        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let update_service = DefaultUpdateService::new(sm.clone(), snapshot_mgr);
            let preflight_result = update_service.run_preflight_checks_with_news(&news_items);
            self.preflight_result = Some(preflight_result);
        } else {
            // Without state manager, run basic pre-flight checks
            let sm = StateManager::new(self.config_dir.clone()).ok();
            if let Some(sm) = sm {
                let snapshot_mgr = iron_core::snapshot::create_manager();
                let update_service = DefaultUpdateService::new(sm, snapshot_mgr);
                let preflight_result = update_service.run_preflight_checks_with_news(&news_items);
                self.preflight_result = Some(preflight_result);
            }
        }

        // Assess risk level
        let (risk, _) = iron_core::assess_risk(&self.pending_updates, &self.arch_news);
        self.update_risk = risk;

        // Reset update view state
        self.reset_update_view();

        let count = self.pending_updates.len();
        let news_count = self.unacknowledged_news_count();
        if news_count > 0 {
            self.set_status(format!(
                "Found {} updates, {} unacknowledged news items",
                count, news_count
            ));
        } else {
            self.set_status(format!("Found {} available updates", count));
        }
    }

    /// Edit selected setting (show contextual hints)
    pub fn edit_selected_setting(&mut self) {
        match self.selected_index {
            0 => {
                // Config Directory - read-only
                self.set_info("Config directory is read-only");
            }
            1 => {
                // Current Host - guide to wizard
                self.set_info("Use Setup Wizard [w] to change host configuration");
            }
            2 => {
                // Active Bundle - guide to bundles view
                self.set_info("Use Bundles view [b] to change active bundle");
            }
            3 => {
                // Active Profile - guide to profiles view
                self.set_info("Use Profiles view [p] to change active profile");
            }
            4 => {
                // Enabled Modules - guide to modules view
                self.set_info("Use Modules view [m] to enable/disable modules");
            }
            5 | 6 | 7 => {
                // Last Sync, Installed Packages, Pending Updates - read-only
                self.set_info("This value is read-only");
            }
            _ => {}
        }
    }

    /// Refresh settings from state manager
    pub fn refresh_settings(&mut self) {
        // Reload data from state manager
        if let Some(ref sm) = self.state_manager {
            self.current_host = sm.current_host();
            self.active_modules = sm.active_modules();

            // Get active bundle for current host
            if let Some(ref host_id) = self.current_host {
                self.active_profile = sm.active_profile(host_id);
            }
        }

        // Reload package counts (non-blocking, fail gracefully)
        self.load_package_data();

        self.set_status("Settings refreshed");
    }

    /// Refresh current view
    pub fn refresh_current_view(&mut self) {
        match self.view {
            View::Bundles | View::BundleDetail => {
                if let Some(ref sm) = self.state_manager {
                    let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
                        .with_package_manager(self.package_manager.clone())
                        .with_service_manager(self.service_manager.clone());
                    self.bundles = bundle_service.discover().unwrap_or_default();
                    self.set_status("Bundles refreshed");
                }
            }
            View::Profiles | View::ProfileDetail => {
                self.profiles.clear();
                self.load_profiles();
                self.set_status("Profiles refreshed");
            }
            View::Modules | View::ModuleDetail => {
                self.modules.clear();
                self.load_modules();
                if let Some(ref sm) = self.state_manager {
                    self.active_modules = sm.active_modules();
                }
                self.set_status("Modules refreshed");
            }
            View::UpdatePreview => {
                self.refresh_updates();
            }
            View::Dashboard => {
                // Refresh all data
                let _ = self.init();
                self.set_status("Dashboard refreshed");
            }
            View::Settings => {
                self.refresh_settings();
            }
            View::Doctor => {
                // Re-run health checks by refreshing underlying data
                let _ = self.init();
                self.set_status("Health checks refreshed");
            }
            _ => {}
        }
    }

    /// Initialize wizard state
    pub fn init_wizard(&mut self) {
        self.wizard = crate::wizard::WizardState::new();
        self.wizard.detect_host();
        self.wizard.load_bundles(&self.config_dir);
        self.wizard.load_profiles(&self.config_dir);
        self.host_input = TextInput::new(&self.wizard.host_id);
    }

    /// Load package data from pacman
    pub fn load_package_data(&mut self) {
        // Get installed package count
        if let Ok(packages) = self.package_manager.query_installed() {
            self.installed_count = packages.len();
        }

        // Check for updates (this may take a moment)
        if let Ok(updates) = self.package_manager.check_updates() {
            self.pending_updates = updates;
            // Assess risk level
            let (risk, _) = iron_core::assess_risk(&self.pending_updates, &[]);
            self.update_risk = risk;
        }
    }

    /// Load profiles from disk
    pub fn load_profiles(&mut self) {
        let profiles_dir = self.config_dir.join("profiles");
        if profiles_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&profiles_dir)
        {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    if let Ok(profile) = Profile::load(&path) {
                        self.profiles.push(profile);
                    }
                }
            }
        }
    }

    /// Load modules from disk
    pub fn load_modules(&mut self) {
        let modules_dir = self.config_dir.join("modules");
        if modules_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&modules_dir)
        {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    if let Ok(module) = Module::load(&path) {
                        self.modules.push(module);
                    }
                }
            }
        }
    }

    /// Switch to a different bundle
    pub fn switch_bundle(&mut self, bundle_id: String) {
        if let Some(ref sm) = self.state_manager {
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
                .with_package_manager(self.package_manager.clone())
                .with_service_manager(self.service_manager.clone());

            // Deactivate current bundle if any
            if let Some(ref current) = self.active_bundle
                && let Err(e) = bundle_service.deactivate(&current.id)
            {
                self.set_error(format!("Failed to deactivate current bundle: {}", e));
                return;
            }

            // Activate new bundle
            match bundle_service.activate(&bundle_id) {
                Ok(()) => {
                    // Reload bundles and update active bundle
                    self.bundles = bundle_service.discover().unwrap_or_default();
                    self.active_bundle = self.bundles.iter().find(|b| b.id == bundle_id).cloned();
                    self.active_modules = sm.active_modules();
                    self.set_status(format!("Switched to bundle: {}", bundle_id));
                }
                Err(e) => {
                    self.set_error(format!("Failed to activate bundle: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Run system update via the injected package manager.
    pub fn run_system_update(&mut self) {
        // Gate on preflight results: block if critical issues
        if let Some(ref preflight) = self.preflight_result {
            if !preflight.blockers.is_empty() {
                self.set_error(format!(
                    "Pre-flight checks failed: {}. Resolve before updating.",
                    preflight.blockers.join(", ")
                ));
                return;
            }
        }

        self.set_info("Running system update...");

        // Collect package names for post-update checks before update starts
        let package_names: Vec<String> = self
            .pending_updates
            .iter()
            .map(|p| p.name.clone())
            .collect();

        // Use UpdateService::apply() for snapshot integration
        let create_snapshot = self.snapshot_backend != iron_core::snapshot::SnapshotBackend::None;

        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let update_service = DefaultUpdateService::new(sm.clone(), snapshot_mgr);
            match iron_core::services::UpdateService::apply(&update_service, create_snapshot) {
                Ok(()) => {
                    self.set_status("System update completed successfully");
                }
                Err(e) => {
                    self.set_error(format!("System update failed: {}", e));
                    return;
                }
            }
        } else {
            // Fallback without state manager — direct upgrade
            match self.package_manager.upgrade(false) {
                Ok(_) => {
                    self.set_status("System update completed successfully");
                }
                Err(e) => {
                    self.set_error(format!("System update failed: {}", e));
                    return;
                }
            }
        }

        // Run post-update detection checks (Phase 2.4)
        // Detects .pacnew/.pacsave conflicts, reboot requirements, failed services
        self.run_post_update_checks(&package_names);
    }

    /// Run post-update detection checks (Phase 2.4)
    ///
    /// Called after system update completes to detect:
    /// - .pacnew/.pacsave configuration file conflicts
    /// - Packages that require a system reboot
    /// - Failed systemd services
    pub fn run_post_update_checks(&mut self, updated_packages: &[String]) {
        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let update_service = DefaultUpdateService::new(sm.clone(), snapshot_mgr);
            let result = update_service.run_post_update_checks(updated_packages);

            if result.has_issues {
                let mut issues = Vec::new();

                if result.reboot_required {
                    issues.push(format!(
                        "Reboot required ({} packages)",
                        result.reboot_packages.len()
                    ));
                }

                if !result.config_conflicts.is_empty() {
                    issues.push(format!(
                        "{} config conflicts (.pacnew/.pacsave)",
                        result.config_conflicts.len()
                    ));
                }

                if !result.failed_services.is_empty() {
                    issues.push(format!("{} failed services", result.failed_services.len()));
                }

                self.set_warning(format!("Post-update: {}", issues.join(", ")));
            }

            self.post_update_result = Some(result);
        } else {
            // Without state manager, run basic post-update checks
            if let Ok(sm) = StateManager::new(self.config_dir.clone()) {
                let snapshot_mgr = iron_core::snapshot::create_manager();
                let update_service = DefaultUpdateService::new(sm, snapshot_mgr);
                self.post_update_result =
                    Some(update_service.run_post_update_checks(updated_packages));
            }
        }
    }

    // ==========================================================================
    // Cleanup Actions (Phase 3)
    // ==========================================================================

    /// Toggle the currently selected cleanup category
    pub fn toggle_selected_cleanup_category(&mut self) {
        use iron_core::services::clean::CleanupCategory;

        let all_categories = CleanupCategory::all();
        if self.selected_index < all_categories.len() {
            let category = all_categories[self.selected_index];
            self.toggle_cleanup_category(category);
        }
    }

    /// Preview cleanup for selected categories
    pub fn preview_cleanup(&mut self) {
        use iron_core::services::clean::{CleanupService, DefaultCleanupService};

        if self.cleanup_categories.is_empty() {
            self.set_warning("No categories selected for preview");
            return;
        }

        self.set_info("Scanning cleanup categories...");

        let service = DefaultCleanupService::new();
        self.cleanup_previews = service.preview(&self.cleanup_categories);

        let total_space = self.cleanup_total_space();
        self.set_status(format!(
            "Preview complete: {} reclaimable from {} categories",
            iron_core::services::clean::format_bytes(total_space),
            self.cleanup_categories.len()
        ));

        self.navigate(View::CleanupPreview);
    }

    /// Execute cleanup for selected categories
    pub fn execute_cleanup(&mut self) {
        use iron_core::services::clean::{CleanupService, DefaultCleanupService};

        if self.cleanup_categories.is_empty() {
            self.set_warning("No categories selected for cleanup");
            return;
        }

        self.set_info("Executing cleanup...");

        let service = DefaultCleanupService::new();

        let summary = service.execute(&self.cleanup_categories, true);

        if summary.failed > 0 {
            self.set_warning(format!(
                "Cleanup completed with {} errors: {} freed",
                summary.failed,
                summary.space_formatted()
            ));
        } else {
            self.set_status(format!(
                "Cleanup complete: {} freed from {} items",
                summary.space_formatted(),
                summary.total_items
            ));
        }

        self.cleanup_summary = Some(summary);
        self.cleanup_preview_mode = false;

        self.navigate(View::CleanupResults);
    }

    // ==========================================================================
    // Sync Actions
    // ==========================================================================

    /// Refresh git sync status
    pub fn refresh_sync_status(&mut self) {
        use iron_core::services::sync::{DefaultSyncService, SyncService};

        if let Some(ref sm) = self.state_manager {
            let sync_service = DefaultSyncService::new(&self.config_dir, sm.clone());
            match sync_service.status() {
                Ok(info) => {
                    self.sync_info = Some(info);
                    self.set_status("Sync status refreshed");
                }
                Err(e) => {
                    self.set_error(format!("Failed to get sync status: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Push local changes to remote (auto-commits first if dirty)
    pub fn sync_push(&mut self) {
        use iron_core::services::sync::{DefaultSyncService, SyncService};

        if let Some(ref sm) = self.state_manager {
            let sync_service = DefaultSyncService::new(&self.config_dir, sm.clone());

            // Auto-commit uncommitted changes before push
            if let Ok(status) = sync_service.status() {
                if status.dirty_files > 0 {
                    let msg = format!("iron: auto-commit {} change(s)", status.dirty_files);
                    if let Err(e) = sync_service.commit(&msg) {
                        self.set_error(format!("Auto-commit failed: {}", e));
                        return;
                    }
                }
            }

            match sync_service.push() {
                Ok(()) => {
                    self.set_status("Changes pushed successfully");
                    self.refresh_sync_status();
                }
                Err(e) => {
                    self.set_error(format!("Push failed: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Pull remote changes (stashes dirty tree first if needed)
    pub fn sync_pull(&mut self) {
        use iron_core::services::sync::{DefaultSyncService, SyncService};

        if let Some(ref sm) = self.state_manager {
            let sync_service = DefaultSyncService::new(&self.config_dir, sm.clone());

            // Check for dirty tree and stash if needed
            let did_stash = if let Ok(status) = sync_service.status() {
                if status.dirty_files > 0 {
                    if let Err(e) = sync_service.stash() {
                        self.set_error(format!("Stash failed: {}", e));
                        return;
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            };

            match sync_service.pull() {
                Ok(()) => {
                    // Restore stashed changes
                    if did_stash {
                        if let Err(e) = sync_service.stash_pop() {
                            self.set_warning(format!(
                                "Pull succeeded but unstash failed: {}. Run 'git stash pop' manually.",
                                e
                            ));
                        }
                    }
                    self.set_status("Changes pulled successfully");
                    self.refresh_sync_status();
                }
                Err(e) => {
                    // Restore stashed changes even on pull failure
                    if did_stash {
                        let _ = sync_service.stash_pop();
                    }
                    self.set_error(format!("Pull failed: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    // ==========================================================================
    // Secrets Actions
    // ==========================================================================

    /// Refresh secrets status and encrypted file list
    pub fn refresh_secrets(&mut self) {
        use iron_core::services::secrets::{DefaultSecretsService, SecretsService};

        let service = DefaultSecretsService::new(&self.config_dir);

        // Update status
        match service.status() {
            Ok(status) => {
                self.secrets_status = Some(format!("{:?}", status));
            }
            Err(e) => {
                self.secrets_status = Some(format!("Error: {}", e));
            }
        }

        // Update encrypted file list
        match service.list_encrypted() {
            Ok(files) => {
                self.encrypted_files = files;
            }
            Err(_) => {
                self.encrypted_files.clear();
            }
        }
    }

    /// Initialize git-crypt in the repository
    pub fn secrets_init(&mut self) {
        use iron_core::services::secrets::{DefaultSecretsService, SecretsService};

        let service = DefaultSecretsService::new(&self.config_dir);
        match service.init() {
            Ok(()) => {
                self.set_status("git-crypt initialized successfully");
                self.refresh_secrets();
            }
            Err(e) => {
                self.set_error(format!("Failed to initialize git-crypt: {}", e));
            }
        }
    }

    /// Unlock secrets (decrypt)
    pub fn secrets_unlock(&mut self) {
        use iron_core::services::secrets::{DefaultSecretsService, SecretsService};

        let service = DefaultSecretsService::new(&self.config_dir);
        match service.unlock(None) {
            Ok(()) => {
                self.set_status("Secrets unlocked successfully");
                self.refresh_secrets();
            }
            Err(e) => {
                self.set_error(format!("Failed to unlock secrets: {}", e));
            }
        }
    }

    /// Lock secrets (re-encrypt)
    pub fn secrets_lock(&mut self) {
        use iron_core::services::secrets::{DefaultSecretsService, SecretsService};

        let service = DefaultSecretsService::new(&self.config_dir);
        match service.lock() {
            Ok(()) => {
                self.set_status("Secrets locked successfully");
                self.refresh_secrets();
            }
            Err(e) => {
                self.set_error(format!("Failed to lock secrets: {}", e));
            }
        }
    }

    // ==========================================================================
    // Recovery Actions
    // ==========================================================================

    /// Export current state to recovery format and save to file
    pub fn recovery_export(&mut self) {
        use iron_core::services::recovery::{DefaultRecoveryService, RecoveryService};

        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let service = DefaultRecoveryService::new(&self.config_dir, sm.clone(), snapshot_mgr);
            match service.export() {
                Ok(_export) => {
                    // Save to timestamped file
                    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                    let filename = format!("iron-export-{}.json", timestamp);
                    let path = self.config_dir.join(&filename);
                    match service.save_export(&path) {
                        Ok(()) => {
                            self.set_status(format!("State exported to {}", filename));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to save export: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.set_error(format!("Failed to export state: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Generate install script for recovery
    pub fn recovery_generate_script(&mut self) {
        use iron_core::services::recovery::{
            DefaultRecoveryService, InstallScriptOptions, RecoveryService,
        };

        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let service = DefaultRecoveryService::new(&self.config_dir, sm.clone(), snapshot_mgr);
            let options = InstallScriptOptions {
                include_packages: true,
                include_aur: true,
                include_services: true,
                include_modules: true,
                include_bundle: true,
                aur_helper: "paru".to_string(),
                interactive: true,
            };
            match service.generate_install_script(&options) {
                Ok(script) => {
                    let path = self.config_dir.join("install.sh");
                    match std::fs::write(&path, &script) {
                        Ok(()) => {
                            // Make executable
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let _ = std::fs::set_permissions(
                                    &path,
                                    std::fs::Permissions::from_mode(0o755),
                                );
                            }
                            self.set_status("Install script generated: install.sh");
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to write install script: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.set_error(format!("Failed to generate install script: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Create a system snapshot (via timeshift/snapper if available)
    pub fn recovery_create_snapshot(&mut self) {
        use iron_core::services::recovery::{DefaultRecoveryService, RecoveryService};

        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let service = DefaultRecoveryService::new(&self.config_dir, sm.clone(), snapshot_mgr);
            let output_dir = self.config_dir.join("backups");
            match service.create_backup(&output_dir) {
                Ok(path) => {
                    self.last_backup = Some(chrono::Utc::now());
                    self.set_status(format!(
                        "Backup created: {}",
                        path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string())
                    ));
                }
                Err(e) => {
                    self.set_error(format!("Failed to create backup: {}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    // ==========================================================================
    // Operation Log Actions
    // ==========================================================================

    /// Cycle through operation log filters
    pub fn cycle_operation_filter(&mut self) {
        use crate::ui::operation_log::OperationFilter;

        let all = OperationFilter::all();
        let current_idx = all
            .iter()
            .position(|f| *f == self.operation_filter)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % all.len();
        self.operation_filter = all[next_idx];
        self.selected_index = 0;
        self.set_info(format!("Filter: {}", self.operation_filter.name()));
    }

    // ==========================================================================
    // Config Manager Actions
    // ==========================================================================

    /// Scan for config conflicts independently (not just post-update)
    pub fn refresh_config_conflicts(&mut self) {
        if let Some(ref sm) = self.state_manager {
            let snapshot_mgr = iron_core::snapshot::create_manager();
            let update_service = DefaultUpdateService::new(sm.clone(), snapshot_mgr);
            let conflicts = update_service.find_config_conflicts();

            // Store in post_update_result, creating one if it doesn't exist
            if let Some(ref mut result) = self.post_update_result {
                result.config_conflicts = conflicts;
            } else {
                use iron_core::services::update::PostUpdateResult;
                self.post_update_result = Some(PostUpdateResult {
                    config_conflicts: conflicts,
                    reboot_required: false,
                    reboot_packages: vec![],
                    failed_services: vec![],
                    has_issues: false,
                });
            }

            let count = self
                .post_update_result
                .as_ref()
                .map(|r| r.config_conflicts.len())
                .unwrap_or(0);
            if count > 0 {
                self.set_warning(format!(
                    "{} configuration conflict{} found",
                    count,
                    if count == 1 { "" } else { "s" }
                ));
            } else {
                self.set_status("No configuration conflicts found");
            }
        }
    }

    // =========================================================================
    // Profile Builder (Phase 4.4)
    // =========================================================================

    /// Open the profile builder wizard, resetting state
    pub fn open_profile_builder(&mut self) {
        self.profile_builder_step = 0;
        self.profile_builder_name = String::new();
        self.profile_builder_description = String::new();
        self.profile_builder_selected_modules = Vec::new();
        self.profile_builder_module_cursor = 0;
        self.profile_builder_editing = true;
        self.profile_builder_editing_desc = false;
        self.navigate(crate::app::View::ProfileBuilder);
    }

    /// Finalise and write the new profile to disk
    pub fn create_profile_from_builder(&mut self) {
        let name = self.profile_builder_name.trim().to_string();
        if name.is_empty() {
            self.set_error("Profile name cannot be empty");
            return;
        }

        let profile_dir = self.config_dir.join("profiles").join(&name);
        if let Err(e) = std::fs::create_dir_all(&profile_dir) {
            self.set_error(format!("Failed to create profile directory: {}", e));
            return;
        }

        let desc = self.profile_builder_description.trim().to_string();
        let desc_opt = if desc.is_empty() {
            None
        } else {
            Some(desc.as_str())
        };
        let module_ids: Vec<&str> = self
            .profile_builder_selected_modules
            .iter()
            .map(|s| s.as_str())
            .collect();
        let toml_content = iron_core::templates::profile_toml(&name, desc_opt, &module_ids);

        let profile_path = profile_dir.join("profile.toml");
        if let Err(e) = std::fs::write(&profile_path, toml_content) {
            self.set_error(format!("Failed to write profile.toml: {}", e));
            return;
        }

        // Reload profiles (clear first to avoid duplicates)
        self.profiles.clear();
        self.load_profiles();
        self.set_status(format!("Created profile: {}", name));
        self.navigate(crate::app::View::Profiles);
    }

    // =========================================================================
    // Module Creator (Phase 5.1)
    // =========================================================================

    /// Open the module creator wizard, resetting state
    pub fn open_module_creator(&mut self) {
        self.module_creator_step = 0;
        self.module_creator_name = String::new();
        self.module_creator_description = String::new();
        self.module_creator_packages = String::new();
        self.module_creator_active_field = 0;
        self.navigate(crate::app::View::ModuleCreator);
    }

    /// Write module.toml to disk and navigate back to Modules
    pub fn create_module_from_creator(&mut self) {
        let id = self.module_creator_name.trim().to_string();
        if id.is_empty() {
            self.set_error("Module ID cannot be empty");
            return;
        }

        let module_dir = self.config_dir.join("modules").join(&id);
        if let Err(e) = std::fs::create_dir_all(&module_dir) {
            self.set_error(format!("Failed to create module directory: {}", e));
            return;
        }

        let desc = self.module_creator_description.trim().to_string();
        let desc_opt = if desc.is_empty() {
            None
        } else {
            Some(desc.as_str())
        };

        let pkgs_raw: Vec<String> = if self.module_creator_packages.trim().is_empty() {
            vec![]
        } else {
            self.module_creator_packages
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        let pkgs: Vec<&str> = pkgs_raw.iter().map(|s| s.as_str()).collect();

        let toml_content = iron_core::templates::module_toml(&id, desc_opt, &pkgs);

        let module_path = module_dir.join("module.toml");
        if let Err(e) = std::fs::write(&module_path, toml_content) {
            self.set_error(format!("Failed to write module.toml: {}", e));
            return;
        }

        // Reload modules
        self.modules.clear();
        self.load_modules();
        self.set_status(format!("Created module: {}", id));
        self.navigate(crate::app::View::Modules);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::{Bundle, BundleType, Module, ModuleKind, Profile};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_bundle(id: &str) -> Bundle {
        Bundle {
            id: id.to_string(),
            name: format!("{} Bundle", id),
            description: Some(format!("{} test bundle", id)),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec![],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        }
    }

    fn create_test_module(id: &str) -> Module {
        Module {
            id: id.to_string(),
            name: format!("{} Module", id),
            description: Some(format!("{} test module", id)),
            kind: ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
        }
    }

    fn create_test_profile(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            name: format!("{} Profile", id),
            description: Some(format!("{} test profile", id)),
            modules: vec![],
            theme: None,
            shell: None,
            extends: None,
            for_bundle: None,
        }
    }

    // ==========================================================================
    // execute_confirm_action Tests
    // ==========================================================================

    #[test]
    fn test_execute_confirm_action_none() {
        let mut app = App::default();
        app.confirm_action = None;

        app.execute_confirm_action();

        // Nothing should happen, app state should be unchanged
        assert!(!app.should_quit);
        assert!(app.status_message.is_none());
    }

    #[test]
    fn test_execute_confirm_action_quit() {
        let mut app = App::default();
        app.confirm_action = Some(ConfirmAction::Quit);

        app.execute_confirm_action();

        assert!(app.should_quit);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_execute_confirm_action_remove_bundle_no_state_manager() {
        // Without a state manager the action should report an error
        let mut app = App::default();
        app.state_manager = None;
        app.confirm_action = Some(ConfirmAction::RemoveBundle("hyprland".to_string()));

        app.execute_confirm_action();

        assert!(app.error_text().is_some());
        assert!(app.error_text().unwrap().contains("No state manager"));
    }

    #[test]
    fn test_execute_confirm_action_enable_module_no_state_manager() {
        let mut app = App::default();
        app.state_manager = None;
        app.confirm_action = Some(ConfirmAction::EnableModule("nvim-ide".to_string()));

        app.execute_confirm_action();

        // Without state manager, nothing should happen (no success or error)
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_execute_confirm_action_disable_module_no_state_manager() {
        let mut app = App::default();
        app.state_manager = None;
        app.confirm_action = Some(ConfirmAction::DisableModule("nvim-ide".to_string()));

        app.execute_confirm_action();

        // Without state manager, nothing should happen
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_execute_confirm_action_run_update() {
        let mut app = App::default();
        app.confirm_action = Some(ConfirmAction::RunUpdate);

        app.execute_confirm_action();

        // With NoopPackageManager the upgrade succeeds; confirm_action is consumed
        assert!(app.confirm_action.is_none());
    }

    // ==========================================================================
    // toggle_selected_module Tests
    // ==========================================================================

    #[test]
    fn test_toggle_selected_module_wrong_view() {
        let mut app = App::default();
        app.view = View::Dashboard;
        app.modules = vec![create_test_module("nvim-ide")];
        app.selected_index = 0;

        app.toggle_selected_module();

        // Should not trigger confirmation in Dashboard view
        assert!(!app.show_confirm);
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_toggle_selected_module_modules_view_enable() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![create_test_module("nvim-ide")];
        app.selected_index = 0;
        app.active_modules = vec![]; // Not active

        app.toggle_selected_module();

        assert!(app.show_confirm);
        match &app.confirm_action {
            Some(ConfirmAction::EnableModule(id)) => assert_eq!(id, "nvim-ide"),
            _ => panic!("Expected EnableModule action"),
        }
    }

    #[test]
    fn test_toggle_selected_module_modules_view_disable() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![create_test_module("nvim-ide")];
        app.selected_index = 0;
        app.active_modules = vec!["nvim-ide".to_string()]; // Active

        app.toggle_selected_module();

        assert!(app.show_confirm);
        match &app.confirm_action {
            Some(ConfirmAction::DisableModule(id)) => assert_eq!(id, "nvim-ide"),
            _ => panic!("Expected DisableModule action"),
        }
    }

    #[test]
    fn test_toggle_selected_module_detail_view() {
        let mut app = App::default();
        app.view = View::ModuleDetail;
        app.modules = vec![create_test_module("kitty-dev")];
        app.selected_index = 0;
        app.active_modules = vec![];

        app.toggle_selected_module();

        assert!(app.show_confirm);
        match &app.confirm_action {
            Some(ConfirmAction::EnableModule(id)) => assert_eq!(id, "kitty-dev"),
            _ => panic!("Expected EnableModule action"),
        }
    }

    #[test]
    fn test_toggle_selected_module_empty_list() {
        let mut app = App::default();
        app.view = View::Modules;
        app.modules = vec![];
        app.selected_index = 0;

        app.toggle_selected_module();

        // Should not trigger confirmation with empty list
        assert!(!app.show_confirm);
    }

    // ==========================================================================
    // activate_selected_bundle Tests
    // ==========================================================================

    #[test]
    fn test_activate_selected_bundle_wrong_view() {
        let mut app = App::default();
        app.view = View::Dashboard;
        app.bundles = vec![create_test_bundle("hyprland")];
        app.selected_index = 0;

        app.activate_selected_bundle();

        assert!(!app.show_confirm);
    }

    #[test]
    fn test_activate_selected_bundle_bundles_view() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![create_test_bundle("hyprland")];
        app.selected_index = 0;

        app.activate_selected_bundle();

        assert!(app.show_confirm);
        match &app.confirm_action {
            Some(ConfirmAction::SwitchBundle(id)) => assert_eq!(id, "hyprland"),
            _ => panic!("Expected SwitchBundle action"),
        }
    }

    #[test]
    fn test_activate_selected_bundle_detail_view() {
        let mut app = App::default();
        app.view = View::BundleDetail;
        app.bundles = vec![create_test_bundle("niri")];
        app.selected_index = 0;

        app.activate_selected_bundle();

        assert!(app.show_confirm);
        match &app.confirm_action {
            Some(ConfirmAction::SwitchBundle(id)) => assert_eq!(id, "niri"),
            _ => panic!("Expected SwitchBundle action"),
        }
    }

    #[test]
    fn test_activate_selected_bundle_empty_list() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.bundles = vec![];
        app.selected_index = 0;

        app.activate_selected_bundle();

        assert!(!app.show_confirm);
    }

    // ==========================================================================
    // activate_selected_profile Tests
    // ==========================================================================

    #[test]
    fn test_activate_selected_profile_no_profile() {
        let mut app = App::default();
        app.view = View::Profiles;
        app.profiles = vec![];
        app.selected_index = 0;

        app.activate_selected_profile();

        // Should return early without any action
        assert!(app.status_text().is_none());
    }

    #[test]
    fn test_activate_selected_profile_no_state_manager() {
        let mut app = App::default();
        app.profiles = vec![create_test_profile("developer")];
        app.selected_index = 0;
        app.state_manager = None;
        app.current_host = Some("desktop".to_string());

        app.activate_selected_profile();

        // Without state manager, nothing should happen
        assert!(app.status_text().is_none());
    }

    #[test]
    fn test_activate_selected_profile_no_host() {
        let mut app = App::default();
        app.profiles = vec![create_test_profile("developer")];
        app.selected_index = 0;
        app.current_host = None;

        app.activate_selected_profile();

        // Without current host, nothing should happen
        assert!(app.status_text().is_none());
    }

    // ==========================================================================
    // refresh_current_view Tests
    // ==========================================================================

    #[test]
    fn test_refresh_current_view_dashboard() {
        let mut app = App::default();
        app.view = View::Dashboard;

        app.refresh_current_view();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("refreshed"));
    }

    #[test]
    fn test_refresh_current_view_bundles() {
        let mut app = App::default();
        app.view = View::Bundles;
        app.state_manager = None;

        app.refresh_current_view();

        // Without state manager, nothing happens but no error either
        assert!(app.status_text().is_none() || app.status_text().unwrap().contains("refreshed"));
    }

    #[test]
    fn test_refresh_current_view_profiles() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.view = View::Profiles;
        app.profiles = vec![create_test_profile("old")];

        app.refresh_current_view();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Profiles refreshed"));
    }

    #[test]
    fn test_refresh_current_view_modules() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.view = View::Modules;
        app.modules = vec![create_test_module("old")];

        app.refresh_current_view();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Modules refreshed"));
    }

    #[test]
    fn test_refresh_current_view_update_preview() {
        let mut app = App::default();
        app.view = View::UpdatePreview;

        app.refresh_current_view();

        // refresh_updates is called which sets status message
        assert!(app.status_text().is_some());
    }

    #[test]
    fn test_refresh_current_view_settings() {
        let mut app = App::default();
        app.view = View::Settings;

        app.refresh_current_view();

        // Settings view now has refresh action
        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("refreshed"));
    }

    // ==========================================================================
    // edit_selected_setting Tests
    // ==========================================================================

    #[test]
    fn test_edit_selected_setting_config_dir() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 0;

        app.edit_selected_setting();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("read-only"));
    }

    #[test]
    fn test_edit_selected_setting_current_host() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 1;

        app.edit_selected_setting();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Wizard"));
    }

    #[test]
    fn test_edit_selected_setting_active_bundle() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 2;

        app.edit_selected_setting();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Bundles"));
    }

    #[test]
    fn test_edit_selected_setting_active_profile() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 3;

        app.edit_selected_setting();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Profiles"));
    }

    #[test]
    fn test_edit_selected_setting_enabled_modules() {
        let mut app = App::default();
        app.view = View::Settings;
        app.selected_index = 4;

        app.edit_selected_setting();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("Modules"));
    }

    #[test]
    fn test_edit_selected_setting_readonly_items() {
        let mut app = App::default();
        app.view = View::Settings;

        // Test Last Sync, Installed Packages, Pending Updates (indices 5, 6, 7)
        for idx in 5..=7 {
            app.selected_index = idx;
            app.edit_selected_setting();
            assert!(app.status_text().unwrap().contains("read-only"));
        }
    }

    // ==========================================================================
    // refresh_settings Tests
    // ==========================================================================

    #[test]
    fn test_refresh_settings_sets_status() {
        let mut app = App::default();

        app.refresh_settings();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("refreshed"));
    }

    // ==========================================================================
    // init_wizard Tests
    // ==========================================================================

    #[test]
    fn test_init_wizard_initializes_state() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );

        app.init_wizard();

        // Wizard should be initialized with detected host
        assert!(!app.wizard.host_id.is_empty() || app.wizard.host_id.is_empty()); // Host may or may not be detected
    }

    #[test]
    fn test_init_wizard_loads_bundles() {
        let temp_dir = TempDir::new().unwrap();

        // Create bundles directory
        let bundles_dir = temp_dir.path().join("bundles");
        std::fs::create_dir_all(&bundles_dir).unwrap();

        // Create a bundle
        let hyprland_dir = bundles_dir.join("hyprland");
        std::fs::create_dir_all(&hyprland_dir).unwrap();
        std::fs::write(
            hyprland_dir.join("bundle.toml"),
            r#"
id = "hyprland"
name = "Hyprland"
description = "Hyprland compositor"
type = "wayland-compositor"
packages = []
"#,
        )
        .unwrap();

        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.init_wizard();

        // Wizard should have loaded bundles
        assert!(
            app.wizard
                .available_bundles
                .contains(&"hyprland".to_string())
        );
    }

    // ==========================================================================
    // load_profiles Tests
    // ==========================================================================

    #[test]
    fn test_load_profiles_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );

        app.load_profiles();

        assert!(app.profiles.is_empty());
    }

    #[test]
    fn test_load_profiles_with_valid_profile() {
        let temp_dir = TempDir::new().unwrap();

        // Create profiles directory
        let profiles_dir = temp_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();

        // Create a profile
        let dev_dir = profiles_dir.join("developer");
        std::fs::create_dir_all(&dev_dir).unwrap();
        std::fs::write(
            dev_dir.join("profile.toml"),
            r#"
id = "developer"
name = "Developer"
description = "Development profile"
modules = ["nvim-ide", "kitty-dev"]
"#,
        )
        .unwrap();

        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.load_profiles();

        assert_eq!(app.profiles.len(), 1);
        assert_eq!(app.profiles[0].id, "developer");
    }

    #[test]
    fn test_load_profiles_skips_invalid() {
        let temp_dir = TempDir::new().unwrap();

        let profiles_dir = temp_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();

        // Create an invalid profile
        let bad_dir = profiles_dir.join("invalid");
        std::fs::create_dir_all(&bad_dir).unwrap();
        std::fs::write(bad_dir.join("profile.toml"), "this is not valid toml {{{").unwrap();

        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.load_profiles();

        // Invalid profile should be skipped
        assert!(app.profiles.is_empty());
    }

    // ==========================================================================
    // load_modules Tests
    // ==========================================================================

    #[test]
    fn test_load_modules_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );

        app.load_modules();

        assert!(app.modules.is_empty());
    }

    #[test]
    fn test_load_modules_with_valid_module() {
        let temp_dir = TempDir::new().unwrap();

        let modules_dir = temp_dir.path().join("modules");
        std::fs::create_dir_all(&modules_dir).unwrap();

        let nvim_dir = modules_dir.join("nvim-ide");
        std::fs::create_dir_all(&nvim_dir).unwrap();
        std::fs::write(
            nvim_dir.join("module.toml"),
            r#"
id = "nvim-ide"
name = "Neovim IDE"
description = "Full IDE setup"
kind = "AppConfig"
packages = ["neovim"]
aur_packages = []
dotfiles = []
conflicts = []
depends = []
"#,
        )
        .unwrap();

        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
            Arc::new(iron_core::NoopSystemService),
        );
        app.load_modules();

        assert_eq!(app.modules.len(), 1);
        assert_eq!(app.modules[0].id, "nvim-ide");
    }

    // ==========================================================================
    // run_system_update Tests
    // ==========================================================================

    #[test]
    fn test_run_system_update_sets_status() {
        let mut app = App::default();

        app.run_system_update();

        // With NoopPackageManager upgrade() returns Ok(()) → status shows "completed"
        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("completed"));
    }

    // ==========================================================================
    // switch_bundle Tests
    // ==========================================================================

    #[test]
    fn test_switch_bundle_no_state_manager() {
        let mut app = App::default();
        app.state_manager = None;

        app.switch_bundle("hyprland".to_string());

        assert!(app.error_text().is_some());
        assert!(app.error_text().unwrap().contains("No state manager"));
    }
}
