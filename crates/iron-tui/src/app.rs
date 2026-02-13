//! Iron TUI Application State
//!
//! Manages application state, navigation, and service integration.

use crate::wizard::{TextInput, WizardState, WizardStep};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use iron_core::{
    services::{BundleService, DefaultBundleService, StateManager},
    Bundle, Module, NoopPackageManager, PackageManager, PackageUpdate, Profile, RiskLevel,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Application state
pub struct App {
    /// Current view
    pub view: View,
    /// Previous view for back navigation
    pub previous_view: Option<View>,
    /// Should quit
    pub should_quit: bool,
    /// Config directory path
    pub config_dir: PathBuf,
    /// State manager
    pub state_manager: Option<StateManager>,
    /// Current host ID
    pub current_host: Option<String>,
    /// Active bundle
    pub active_bundle: Option<Bundle>,
    /// Active profile ID
    pub active_profile: Option<String>,
    /// Available bundles
    pub bundles: Vec<Bundle>,
    /// Available profiles
    pub profiles: Vec<Profile>,
    /// Available modules
    pub modules: Vec<Module>,
    /// Active module IDs
    pub active_modules: Vec<String>,
    /// Selected index in list views
    pub selected_index: usize,
    /// Status message
    pub status_message: Option<String>,
    /// Error message
    pub error_message: Option<String>,
    /// Show help overlay
    pub show_help: bool,
    /// Show confirm dialog
    pub show_confirm: bool,
    /// Confirm action pending
    pub confirm_action: Option<ConfirmAction>,
    /// Wizard state
    pub wizard: WizardState,
    /// Host name input
    pub host_input: TextInput,
    /// Package manager (injected)
    pub package_manager: Arc<dyn PackageManager>,
    /// Installed package count (cached)
    pub installed_count: usize,
    /// Pending updates (cached)
    pub pending_updates: Vec<PackageUpdate>,
    /// Update risk level
    pub update_risk: RiskLevel,
}

/// Available views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Dashboard home
    Dashboard,
    /// First-time setup wizard
    SetupWizard,
    /// Bundle list
    Bundles,
    /// Bundle detail
    BundleDetail,
    /// Profile list
    Profiles,
    /// Profile detail
    ProfileDetail,
    /// Module list
    Modules,
    /// Module detail
    ModuleDetail,
    /// Update preview
    UpdatePreview,
    /// Sync status
    Sync,
    /// Settings
    Settings,
}

/// Actions that require confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Switch to a bundle
    SwitchBundle(String),
    /// Remove a bundle
    RemoveBundle(String),
    /// Enable a module
    EnableModule(String),
    /// Disable a module
    DisableModule(String),
    /// Run system update
    RunUpdate,
    /// Quit application
    Quit,
}

impl Default for App {
    fn default() -> Self {
        Self::new(
            std::path::PathBuf::from("."),
            Arc::new(NoopPackageManager),
        )
    }
}

impl App {
    /// Create a new application instance with a package manager
    pub fn new(config_dir: PathBuf, package_manager: Arc<dyn PackageManager>) -> Self {
        Self {
            view: View::Dashboard,
            previous_view: None,
            should_quit: false,
            config_dir,
            state_manager: None,
            current_host: None,
            active_bundle: None,
            active_profile: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            modules: Vec::new(),
            active_modules: Vec::new(),
            selected_index: 0,
            status_message: None,
            error_message: None,
            show_help: false,
            show_confirm: false,
            confirm_action: None,
            wizard: WizardState::new(),
            host_input: TextInput::new(""),
            package_manager,
            installed_count: 0,
            pending_updates: Vec::new(),
            update_risk: RiskLevel::Low,
        }
    }

    /// Initialize application state by loading from services
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
                        let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
                        self.bundles = bundle_service.discover().unwrap_or_default();
                        self.active_bundle = self.bundles.iter().find(|b| b.id == bundle_id).cloned();
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
            && let Some(ref sm) = self.state_manager {
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

    /// Load package data from pacman
    fn load_package_data(&mut self) {
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
    fn load_profiles(&mut self) {
        let profiles_dir = self.config_dir.join("profiles");
        if profiles_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&profiles_dir) {
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
    fn load_modules(&mut self) {
        let modules_dir = self.config_dir.join("modules");
        if modules_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&modules_dir) {
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

    /// Initialize wizard state
    fn init_wizard(&mut self) {
        self.wizard = WizardState::new();
        self.wizard.detect_host();
        self.wizard.load_bundles(&self.config_dir);
        self.wizard.load_profiles(&self.config_dir);
        self.host_input = TextInput::new(&self.wizard.host_id);
    }

    /// Navigate to a view
    pub fn navigate(&mut self, view: View) {
        self.previous_view = Some(self.view);
        self.view = view;
        self.selected_index = 0;
        self.clear_messages();
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_view.take() {
            self.view = prev;
            self.selected_index = 0;
        } else {
            self.view = View::Dashboard;
        }
        self.clear_messages();
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                _ => {}
            }
        }

        // Help overlay
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Confirm dialog
        if self.show_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.execute_confirm_action();
                    self.show_confirm = false;
                    self.confirm_action = None;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.show_confirm = false;
                    self.confirm_action = None;
                }
                _ => {}
            }
            return;
        }

        // Wizard handling
        if self.view == View::SetupWizard {
            self.handle_wizard_key(key);
            return;
        }

        // View-specific key handling
        match self.view {
            View::UpdatePreview => {
                match key.code {
                    KeyCode::Char('r') => self.refresh_updates(),
                    KeyCode::Enter | KeyCode::Char('u') => {
                        self.request_confirm(ConfirmAction::RunUpdate);
                    }
                    KeyCode::Esc => self.go_back(),
                    KeyCode::Char('?') => self.show_help = true,
                    KeyCode::Char('q') => self.should_quit = true,
                    _ => {}
                }
                return;
            }
            View::ProfileDetail => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('a') => self.activate_selected_profile(),
                    KeyCode::Esc => self.go_back(),
                    KeyCode::Char('?') => self.show_help = true,
                    KeyCode::Char('q') => self.should_quit = true,
                    _ => {}
                }
                return;
            }
            _ => {}
        }

        // General key handling
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Esc => self.go_back(),
            KeyCode::Tab => self.cycle_view_forward(),
            KeyCode::BackTab => self.cycle_view_backward(),

            // Navigation
            KeyCode::Char('d') => self.navigate(View::Dashboard),
            KeyCode::Char('b') => self.navigate(View::Bundles),
            KeyCode::Char('p') => self.navigate(View::Profiles),
            KeyCode::Char('m') => self.navigate(View::Modules),
            KeyCode::Char('u') => self.navigate(View::UpdatePreview),
            KeyCode::Char('s') => self.navigate(View::Settings),

            // List navigation
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.select_item(),
            KeyCode::Home => self.selected_index = 0,
            KeyCode::End => self.select_last(),

            // Module/Bundle actions
            KeyCode::Char('e') => self.toggle_selected_module(),
            KeyCode::Char('a') => self.activate_selected_bundle(),

            // Refresh
            KeyCode::Char('r') => self.refresh_current_view(),

            _ => {}
        }
    }

    /// Handle wizard key input
    fn handle_wizard_key(&mut self, key: KeyEvent) {
        // Handle text input mode
        if self.host_input.is_editing() {
            match key.code {
                KeyCode::Esc => {
                    self.host_input.exit_edit_mode();
                }
                KeyCode::Enter => {
                    self.wizard.host_id = self.host_input.value.clone();
                    self.host_input.exit_edit_mode();
                }
                KeyCode::Backspace => {
                    self.host_input.delete();
                }
                KeyCode::Delete => {
                    self.host_input.delete_forward();
                }
                KeyCode::Left => {
                    self.host_input.move_left();
                }
                KeyCode::Right => {
                    self.host_input.move_right();
                }
                KeyCode::Home => {
                    self.host_input.move_start();
                }
                KeyCode::End => {
                    self.host_input.move_end();
                }
                KeyCode::Char(c) => {
                    self.host_input.insert(c);
                }
                _ => {}
            }
            return;
        }

        match self.wizard.step {
            WizardStep::Welcome => {
                match key.code {
                    KeyCode::Enter => self.wizard.next_step(),
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    _ => {}
                }
            }
            WizardStep::HostSetup => {
                match key.code {
                    KeyCode::Enter => {
                        if self.wizard.can_proceed() {
                            self.wizard.next_step();
                        }
                    }
                    KeyCode::Char('e') => {
                        self.host_input.enter_edit_mode();
                    }
                    KeyCode::Backspace | KeyCode::Esc => {
                        self.wizard.prev_step();
                    }
                    _ => {}
                }
            }
            WizardStep::BundleSelection => {
                match key.code {
                    KeyCode::Enter => self.wizard.next_step(),
                    KeyCode::Up | KeyCode::Char('k') => self.wizard.select_prev_bundle(),
                    KeyCode::Down | KeyCode::Char('j') => self.wizard.select_next_bundle(),
                    KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                    _ => {}
                }
            }
            WizardStep::ProfileSelection => {
                match key.code {
                    KeyCode::Enter => self.wizard.next_step(),
                    KeyCode::Up | KeyCode::Char('k') => self.wizard.select_prev_profile(),
                    KeyCode::Down | KeyCode::Char('j') => self.wizard.select_next_profile(),
                    KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                    _ => {}
                }
            }
            WizardStep::Confirmation => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('y') => {
                        if let Ok(()) = self.wizard.apply(&self.config_dir) {
                            // Reinitialize app after wizard
                            let _ = self.init();
                            self.view = View::Dashboard;
                            self.set_status("Setup complete! Welcome to Iron.");
                        }
                    }
                    KeyCode::Backspace | KeyCode::Esc => self.wizard.prev_step(),
                    _ => {}
                }
            }
            WizardStep::Complete => {
                if key.code == KeyCode::Enter {
                    self.view = View::Dashboard;
                }
            }
        }
    }

    /// Toggle enable/disable for selected module
    fn toggle_selected_module(&mut self) {
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
    fn activate_selected_bundle(&mut self) {
        if self.view != View::Bundles && self.view != View::BundleDetail {
            return;
        }
        if let Some(bundle) = self.selected_bundle() {
            let bundle_id = bundle.id.clone();
            self.request_confirm(ConfirmAction::SwitchBundle(bundle_id));
        }
    }

    /// Cycle to next view
    fn cycle_view_forward(&mut self) {
        let next = match self.view {
            View::Dashboard => View::Bundles,
            View::Bundles | View::BundleDetail => View::Profiles,
            View::Profiles | View::ProfileDetail => View::Modules,
            View::Modules | View::ModuleDetail => View::UpdatePreview,
            View::UpdatePreview => View::Settings,
            View::Settings => View::Dashboard,
            _ => View::Dashboard,
        };
        self.navigate(next);
    }

    /// Cycle to previous view
    fn cycle_view_backward(&mut self) {
        let prev = match self.view {
            View::Dashboard => View::Settings,
            View::Settings => View::UpdatePreview,
            View::UpdatePreview => View::Modules,
            View::Modules | View::ModuleDetail => View::Profiles,
            View::Profiles | View::ProfileDetail => View::Bundles,
            View::Bundles | View::BundleDetail => View::Dashboard,
            _ => View::Dashboard,
        };
        self.navigate(prev);
    }

    /// Select previous item in list
    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Select next item in list
    fn select_next(&mut self) {
        let max = self.current_list_len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Select last item in list
    fn select_last(&mut self) {
        self.selected_index = self.current_list_len().saturating_sub(1);
    }

    /// Get current list length based on view
    fn current_list_len(&self) -> usize {
        match self.view {
            View::Bundles => self.bundles.len(),
            View::Profiles => self.profiles.len(),
            View::Modules => self.modules.len(),
            _ => 0,
        }
    }

    /// Handle item selection in list views
    fn select_item(&mut self) {
        match self.view {
            View::Bundles if !self.bundles.is_empty() => {
                self.navigate(View::BundleDetail);
            }
            View::Profiles if !self.profiles.is_empty() => {
                self.navigate(View::ProfileDetail);
            }
            View::Modules if !self.modules.is_empty() => {
                self.navigate(View::ModuleDetail);
            }
            _ => {}
        }
    }

    /// Execute confirmed action
    fn execute_confirm_action(&mut self) {
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
                            self.error_message = Some(format!("Failed to enable module: {:?}", e));
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
                            self.error_message = Some(format!("Failed to disable module: {:?}", e));
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

    /// Switch to a different bundle
    fn switch_bundle(&mut self, bundle_id: String) {
        if let Some(ref sm) = self.state_manager {
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());

            // Deactivate current bundle if any
            if let Some(ref current) = self.active_bundle
                && let Err(e) = bundle_service.deactivate(&current.id) {
                    self.error_message = Some(format!("Failed to deactivate current bundle: {:?}", e));
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
    fn run_system_update(&mut self) {
        // TODO: Integrate with pacman service when available
        self.status_message = Some("System update started (dry-run mode)".to_string());
        // In a real implementation, this would:
        // 1. Run pacman -Syu or use the PacmanService
        // 2. Show progress in the UpdatePreview view
        // 3. Handle errors and rollbacks
    }

    /// Activate the selected profile
    fn activate_selected_profile(&mut self) {
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

    /// Refresh update information
    fn refresh_updates(&mut self) {
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

    /// Refresh current view data
    fn refresh_current_view(&mut self) {
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

    /// Request confirmation for an action
    pub fn request_confirm(&mut self, action: ConfirmAction) {
        self.confirm_action = Some(action);
        self.show_confirm = true;
    }

    /// Clear status and error messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }

    /// Set error message
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }

    /// Set status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Get the selected bundle (if in bundle views)
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.bundles.get(self.selected_index)
    }

    /// Get the selected profile (if in profile views)
    pub fn selected_profile(&self) -> Option<&Profile> {
        self.profiles.get(self.selected_index)
    }

    /// Get the selected module (if in module views)
    pub fn selected_module(&self) -> Option<&Module> {
        self.modules.get(self.selected_index)
    }

    /// Check if a module is active
    pub fn is_module_active(&self, module_id: &str) -> bool {
        self.active_modules.contains(&module_id.to_string())
    }

    /// Check if state needs refresh (called on tick)
    pub fn tick(&mut self) {
        // Placeholder for periodic state refresh
    }

    /// Get system health status
    /// Get system health status based on update risk and pending updates
    pub fn system_health(&self) -> HealthStatus {
        match self.update_risk {
            RiskLevel::Critical => HealthStatus::Error,
            RiskLevel::High => HealthStatus::Warning,
            _ if self.pending_updates.len() > 50 => HealthStatus::Warning,
            _ => HealthStatus::Ok,
        }
    }

    /// Get installed package count
    pub fn package_count(&self) -> usize {
        self.installed_count
    }

    /// Get enabled module count
    pub fn enabled_module_count(&self) -> usize {
        self.active_modules.len()
    }

    /// Get pending update count
    pub fn pending_update_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Get update risk level
    pub fn update_risk_level(&self) -> RiskLevel {
        self.update_risk
    }

    /// Get pending updates list
    pub fn pending_updates_list(&self) -> &[PackageUpdate] {
        &self.pending_updates
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
    Warning,
    Error,
}
