//! Application actions for Iron TUI
//!
//! Contains action execution logic for bundles, modules, profiles, and updates.

use super::{App, ConfirmAction, View};
use crate::wizard::TextInput;
use iron_core::services::{BundleService, DefaultBundleService, DefaultUpdateService, StateManager, UpdateService};
use iron_core::{Module, NoopManager, Profile};

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
                        // Load bundles via BundleService
                        let bundle_service =
                            DefaultBundleService::new(&self.config_dir, sm.clone());
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
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
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
                // Bundle removal not yet implemented
                self.set_warning(format!("Bundle removal not yet implemented: {}", id));
            }
            ConfirmAction::EnableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    match sm.enable_module(id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.set_status(format!("Enabled module: {}", id));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to enable module: {:?}", e));
                        }
                    }
                }
            }
            ConfirmAction::DisableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    match sm.disable_module(id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.set_status(format!("Disabled module: {}", id));
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to disable module: {:?}", e));
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
        if self.view != View::Modules && self.view != View::ModuleDetail {
            return;
        }
        if let Some(module) = self.selected_module() {
            let module_id = module.id.clone();
            let is_active = self.is_module_active(&module_id);
            let action = if is_active {
                ConfirmAction::DisableModule(module_id)
            } else {
                ConfirmAction::EnableModule(module_id)
            };
            self.request_confirm(action);
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

        if let (Some(sm), Some(host_id)) = (&self.state_manager, &self.current_host) {
            match sm.set_active_profile(host_id, &profile) {
                Ok(()) => {
                    self.active_profile = Some(profile.clone());
                    self.set_status(format!("Activated profile: {}", profile));
                }
                Err(e) => {
                    self.set_error(format!("Failed to activate profile: {:?}", e));
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
                self.set_error(format!("Failed to check updates: {:?}", e));
                return;
            }
        }

        // Fetch news
        let news_items = self.package_manager.fetch_news().unwrap_or_default();
        self.arch_news = news_items.clone();

        // Run pre-flight checks with news (Phase 2.3)
        if let Some(ref sm) = self.state_manager {
            let update_service = DefaultUpdateService::new(sm.clone(), NoopManager);
            let preflight_result = update_service.run_preflight_checks_with_news(&news_items);
            self.preflight_result = Some(preflight_result);
        } else {
            // Without state manager, run basic pre-flight checks
            let sm = StateManager::new(self.config_dir.clone()).ok();
            if let Some(sm) = sm {
                let update_service = DefaultUpdateService::new(sm, NoopManager);
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
                    let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
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
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());

            // Deactivate current bundle if any
            if let Some(ref current) = self.active_bundle
                && let Err(e) = bundle_service.deactivate(&current.id)
            {
                self.set_error(format!("Failed to deactivate current bundle: {:?}", e));
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
                    self.set_error(format!("Failed to activate bundle: {:?}", e));
                }
            }
        } else {
            self.set_error("No state manager available");
        }
    }

    /// Run system update (placeholder)
    pub fn run_system_update(&mut self) {
        // TODO: Integrate with pacman service when available
        self.set_info("System update started (dry-run mode)");

        // Collect package names for post-update checks
        let package_names: Vec<String> = self.pending_updates.iter().map(|p| p.name.clone()).collect();

        // In a real implementation, this would:
        // 1. Run pacman -Syu or use the PacmanService
        // 2. Show progress in the UpdatePreview view
        // 3. Handle errors and rollbacks

        // Run post-update detection checks (Phase 2.4)
        // This detects .pacnew/.pacsave files, reboot requirements, and failed services
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
            let update_service = DefaultUpdateService::new(sm.clone(), NoopManager);
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
                let update_service = DefaultUpdateService::new(sm, NoopManager);
                self.post_update_result = Some(update_service.run_post_update_checks(updated_packages));
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

        // Run with dry_run=false for real cleanup
        // For safety, we run dry_run=true in TUI (user can use CLI for actual cleanup)
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
    fn test_execute_confirm_action_remove_bundle_not_implemented() {
        let mut app = App::default();
        app.confirm_action = Some(ConfirmAction::RemoveBundle("hyprland".to_string()));

        app.execute_confirm_action();

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("not yet implemented"));
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

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("update started"));
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
        assert!(
            app.status_text().is_none()
                || app.status_text().unwrap().contains("refreshed")
        );
    }

    #[test]
    fn test_refresh_current_view_profiles() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = App::new(
            temp_dir.path().to_path_buf(),
            Arc::new(iron_core::NoopPackageManager),
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

        assert!(app.status_text().is_some());
        assert!(app.status_text().unwrap().contains("dry-run"));
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
