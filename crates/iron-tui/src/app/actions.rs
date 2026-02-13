//! Application actions for Iron TUI
//!
//! Contains action execution logic for bundles, modules, profiles, and updates.

use super::{App, ConfirmAction, View};
use crate::wizard::TextInput;
use iron_core::services::{BundleService, DefaultBundleService, StateManager};
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
                self.status_message = Some(format!("Bundle removal not yet implemented: {}", id));
            }
            ConfirmAction::EnableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    match sm.enable_module(id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.status_message = Some(format!("Enabled module: {}", id));
                        }
                        Err(e) => {
                            self.error_message =
                                Some(format!("Failed to enable module: {:?}", e));
                        }
                    }
                }
            }
            ConfirmAction::DisableModule(ref id) => {
                if let Some(ref sm) = self.state_manager {
                    match sm.disable_module(id) {
                        Ok(()) => {
                            self.active_modules = sm.active_modules();
                            self.status_message = Some(format!("Disabled module: {}", id));
                        }
                        Err(e) => {
                            self.error_message =
                                Some(format!("Failed to disable module: {:?}", e));
                        }
                    }
                }
            }
            ConfirmAction::RunUpdate => {
                self.run_system_update();
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
                    self.status_message = Some(format!("Activated profile: {}", profile));
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to activate profile: {:?}", e));
                }
            }
        }
    }

    /// Refresh updates
    pub fn refresh_updates(&mut self) {
        self.status_message = Some("Checking for updates...".to_string());

        // Refresh package database and check for updates
        match self.package_manager.check_updates() {
            Ok(updates) => {
                let count = updates.len();
                self.pending_updates = updates;
                let (risk, _) = iron_core::assess_risk(&self.pending_updates, &[]);
                self.update_risk = risk;
                self.status_message = Some(format!("Found {} available updates", count));
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to check updates: {:?}", e));
            }
        }
    }

    /// Refresh current view
    pub fn refresh_current_view(&mut self) {
        match self.view {
            View::Bundles | View::BundleDetail => {
                if let Some(ref sm) = self.state_manager {
                    let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
                    self.bundles = bundle_service.discover().unwrap_or_default();
                    self.status_message = Some("Bundles refreshed".to_string());
                }
            }
            View::Profiles | View::ProfileDetail => {
                self.profiles.clear();
                self.load_profiles();
                self.status_message = Some("Profiles refreshed".to_string());
            }
            View::Modules | View::ModuleDetail => {
                self.modules.clear();
                self.load_modules();
                if let Some(ref sm) = self.state_manager {
                    self.active_modules = sm.active_modules();
                }
                self.status_message = Some("Modules refreshed".to_string());
            }
            View::UpdatePreview => {
                self.refresh_updates();
            }
            View::Dashboard => {
                // Refresh all data
                let _ = self.init();
                self.status_message = Some("Dashboard refreshed".to_string());
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
                self.error_message =
                    Some(format!("Failed to deactivate current bundle: {:?}", e));
                return;
            }

            // Activate new bundle
            match bundle_service.activate(&bundle_id) {
                Ok(()) => {
                    // Reload bundles and update active bundle
                    self.bundles = bundle_service.discover().unwrap_or_default();
                    self.active_bundle = self.bundles.iter().find(|b| b.id == bundle_id).cloned();
                    self.active_modules = sm.active_modules();
                    self.status_message = Some(format!("Switched to bundle: {}", bundle_id));
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to activate bundle: {:?}", e));
                }
            }
        } else {
            self.error_message = Some("No state manager available".to_string());
        }
    }

    /// Run system update (placeholder)
    pub fn run_system_update(&mut self) {
        // TODO: Integrate with pacman service when available
        self.status_message = Some("System update started (dry-run mode)".to_string());
        // In a real implementation, this would:
        // 1. Run pacman -Syu or use the PacmanService
        // 2. Show progress in the UpdatePreview view
        // 3. Handle errors and rollbacks
    }
}
