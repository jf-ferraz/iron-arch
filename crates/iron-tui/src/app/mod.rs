//! Iron TUI Application State
//!
//! Manages application state, navigation, and service integration.

mod actions;
mod handlers;

use crate::wizard::{TextInput, WizardState};
use iron_core::{
    Bundle, Module, NoopPackageManager, PackageManager, PackageUpdate, Profile, RiskLevel,
    services::StateManager,
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

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
    Warning,
    Error,
}

impl Default for App {
    fn default() -> Self {
        Self::new(std::path::PathBuf::from("."), Arc::new(NoopPackageManager))
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
