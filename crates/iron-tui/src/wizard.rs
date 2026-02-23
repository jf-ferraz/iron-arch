//! Setup Wizard for Iron TUI
//!
//! Multi-step wizard for first-time setup and configuration.

use iron_core::host::HardwareSpec;
use iron_core::services::{
    BundleService, DefaultBundleService, DefaultModuleService, DefaultProfileService,
    ProfileService, StateManager,
};
use iron_core::{PackageManager, SystemService};
use std::path::Path;
use std::sync::Arc;

/// Summary of a bundle for display in the setup wizard.
#[derive(Debug, Clone, Default)]
pub struct BundleSummary {
    /// Bundle identifier
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Total number of packages (official + AUR)
    pub package_count: usize,
}

impl BundleSummary {
    /// Format for display: `name — description (N packages)`
    pub fn display_line(&self) -> String {
        if self.description.is_empty() {
            if self.package_count > 0 {
                format!("{} ({} packages)", self.name, self.package_count)
            } else {
                self.name.clone()
            }
        } else if self.package_count > 0 {
            format!(
                "{} — {} ({} packages)",
                self.name, self.description, self.package_count
            )
        } else {
            format!("{} — {}", self.name, self.description)
        }
    }
}

/// Summary of a profile for display in the setup wizard.
#[derive(Debug, Clone, Default)]
pub struct ProfileSummary {
    /// Profile identifier
    pub name: String,
    /// Human-readable description
    pub description: String,
}

/// Wizard state machine
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WizardStep {
    /// Welcome screen
    #[default]
    Welcome,
    /// Detect or configure host
    HostSetup,
    /// Select bundle (desktop environment)
    BundleSelection,
    /// Select profile
    ProfileSelection,
    /// Confirm and apply
    Confirmation,
    /// Setup complete
    Complete,
}

/// Wizard state
#[derive(Debug, Clone, Default)]
pub struct WizardState {
    /// Current step
    pub step: WizardStep,
    /// Host ID (detected or entered)
    pub host_id: String,
    /// Selected bundle index
    pub selected_bundle_index: usize,
    /// Selected profile index
    pub selected_profile_index: usize,
    /// Error message
    pub error: Option<String>,
    /// Is processing
    pub processing: bool,
    /// Available bundles (cached) — plain names for backward compatibility
    pub available_bundles: Vec<String>,
    /// Available bundle summaries with descriptions and package counts
    pub bundle_summaries: Vec<BundleSummary>,
    /// Available profiles (cached)
    pub available_profiles: Vec<String>,
    /// Available profile summaries with descriptions
    pub profile_summaries: Vec<ProfileSummary>,
    /// Detected hardware specifications
    pub detected_hardware: Option<HardwareSpec>,
}

impl WizardState {
    /// Create new wizard state
    pub fn new() -> Self {
        Self::default()
    }

    /// Move to next step
    pub fn next_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::HostSetup,
            WizardStep::HostSetup => WizardStep::BundleSelection,
            WizardStep::BundleSelection => WizardStep::ProfileSelection,
            WizardStep::ProfileSelection => WizardStep::Confirmation,
            WizardStep::Confirmation => WizardStep::Complete,
            WizardStep::Complete => WizardStep::Complete,
        };
        self.error = None;
    }

    /// Move to previous step
    pub fn prev_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::HostSetup => WizardStep::Welcome,
            WizardStep::BundleSelection => WizardStep::HostSetup,
            WizardStep::ProfileSelection => WizardStep::BundleSelection,
            WizardStep::Confirmation => WizardStep::ProfileSelection,
            WizardStep::Complete => WizardStep::Complete,
        };
        self.error = None;
    }

    /// Get step number (1-indexed)
    pub fn step_number(&self) -> usize {
        match self.step {
            WizardStep::Welcome => 1,
            WizardStep::HostSetup => 2,
            WizardStep::BundleSelection => 3,
            WizardStep::ProfileSelection => 4,
            WizardStep::Confirmation => 5,
            WizardStep::Complete => 6,
        }
    }

    /// Total steps
    pub fn total_steps(&self) -> usize {
        5
    }

    /// Can go back?
    pub fn can_go_back(&self) -> bool {
        !matches!(self.step, WizardStep::Welcome | WizardStep::Complete)
    }

    /// Can proceed?
    pub fn can_proceed(&self) -> bool {
        match self.step {
            WizardStep::Welcome => true,
            WizardStep::HostSetup => !self.host_id.is_empty(),
            WizardStep::BundleSelection => !self.available_bundles.is_empty(),
            WizardStep::ProfileSelection => true, // Profile is optional
            WizardStep::Confirmation => true,
            WizardStep::Complete => false,
        }
    }

    /// Get selected bundle ID
    pub fn selected_bundle(&self) -> Option<&str> {
        self.available_bundles
            .get(self.selected_bundle_index)
            .map(|s| s.as_str())
    }

    /// Get selected profile ID
    pub fn selected_profile(&self) -> Option<&str> {
        self.available_profiles
            .get(self.selected_profile_index)
            .map(|s| s.as_str())
    }

    /// Detect host from system
    pub fn detect_host(&mut self) {
        // Try to detect hostname from various sources
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            self.host_id = hostname;
        } else if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            self.host_id = hostname.trim().to_string();
        } else if let Ok(output) = std::process::Command::new("hostname").output() {
            if output.status.success() {
                self.host_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        } else {
            self.host_id = "desktop".to_string();
        }

        // Fallback if still empty
        if self.host_id.is_empty() {
            self.host_id = "desktop".to_string();
        }
    }

    /// Load available bundles from config dir, populating both name list and summaries.
    pub fn load_bundles(&mut self, config_dir: &Path) {
        self.available_bundles.clear();
        self.bundle_summaries.clear();

        let state_manager = StateManager::new(config_dir.to_path_buf()).ok();

        if let Some(sm) = state_manager {
            let service = DefaultBundleService::new(config_dir, sm);
            if let Ok(bundles) = service.discover()
                && !bundles.is_empty()
            {
                for bundle in &bundles {
                    self.available_bundles.push(bundle.id.clone());
                    self.bundle_summaries.push(BundleSummary {
                        name: bundle.id.clone(),
                        description: bundle.description.clone().unwrap_or_default(),
                        package_count: bundle.packages.len() + bundle.aur_packages.len(),
                    });
                }
                return;
            }
        }

        // Fallback: scan directory names only (no descriptions)
        let bundles_dir = config_dir.join("bundles");
        if bundles_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&bundles_dir)
        {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    && let Some(name) = entry.file_name().to_str()
                {
                    self.available_bundles.push(name.to_string());
                    self.bundle_summaries.push(BundleSummary {
                        name: name.to_string(),
                        ..Default::default()
                    });
                }
            }
        }
        self.available_bundles.sort();
        self.bundle_summaries.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Load available profiles from config dir, populating both name list and summaries.
    pub fn load_profiles(&mut self, config_dir: &Path) {
        self.available_profiles.clear();
        self.profile_summaries.clear();

        let state_manager = StateManager::new(config_dir.to_path_buf()).ok();

        if let Some(sm) = state_manager {
            let module_service = DefaultModuleService::new(config_dir, sm.clone());
            let service = DefaultProfileService::new(config_dir, sm, module_service);
            if let Ok(profiles) = service.discover()
                && !profiles.is_empty()
            {
                for profile in &profiles {
                    self.available_profiles.push(profile.id.clone());
                    self.profile_summaries.push(ProfileSummary {
                        name: profile.id.clone(),
                        description: profile.description.clone().unwrap_or_default(),
                    });
                }
                return;
            }
        }

        // Fallback: scan directory names only (no descriptions)
        let profiles_dir = config_dir.join("profiles");
        if profiles_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&profiles_dir)
        {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    && let Some(name) = entry.file_name().to_str()
                {
                    self.available_profiles.push(name.to_string());
                    self.profile_summaries.push(ProfileSummary {
                        name: name.to_string(),
                        ..Default::default()
                    });
                }
            }
        }
        self.available_profiles.sort();
        self.profile_summaries.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Select next bundle
    pub fn select_next_bundle(&mut self) {
        if !self.available_bundles.is_empty() {
            self.selected_bundle_index =
                (self.selected_bundle_index + 1) % self.available_bundles.len();
        }
    }

    /// Select previous bundle
    pub fn select_prev_bundle(&mut self) {
        if !self.available_bundles.is_empty() {
            if self.selected_bundle_index == 0 {
                self.selected_bundle_index = self.available_bundles.len() - 1;
            } else {
                self.selected_bundle_index -= 1;
            }
        }
    }

    /// Select next profile
    pub fn select_next_profile(&mut self) {
        if !self.available_profiles.is_empty() {
            self.selected_profile_index =
                (self.selected_profile_index + 1) % self.available_profiles.len();
        }
    }

    /// Select previous profile
    pub fn select_prev_profile(&mut self) {
        if !self.available_profiles.is_empty() {
            if self.selected_profile_index == 0 {
                self.selected_profile_index = self.available_profiles.len() - 1;
            } else {
                self.selected_profile_index -= 1;
            }
        }
    }

    /// Apply wizard configuration
    pub fn apply(
        &mut self,
        config_dir: &Path,
        package_manager: Arc<dyn PackageManager>,
        service_manager: Arc<dyn SystemService>,
    ) -> Result<(), String> {
        self.processing = true;
        self.error = None;

        // Create state manager
        let state_manager = match StateManager::new(config_dir.to_path_buf()) {
            Ok(sm) => sm,
            Err(e) => {
                self.processing = false;
                self.error = Some(format!("Failed to create state: {:?}", e));
                return Err(self.error.clone().unwrap());
            }
        };

        // Set current host
        if let Err(e) = state_manager.set_current_host(&self.host_id) {
            self.processing = false;
            self.error = Some(format!("Failed to set host: {:?}", e));
            return Err(self.error.clone().unwrap());
        }

        // B-003: Create host TOML file with detected hardware
        {
            use iron_core::services::host::{DefaultHostService, HostService};
            let host_service = DefaultHostService::new(config_dir);
            // Only create if the host file doesn't already exist
            if host_service.load_host(&self.host_id).is_err()
                && let Err(e) = host_service.create_from_current(&self.host_id, &self.host_id)
            {
                // Non-fatal: log but continue (host TOML is advisory, not blocking)
                self.error = Some(format!("Warning: Could not create host config: {:?}", e));
            }
        }

        // Activate bundle if selected
        if let Some(bundle_id) = self.selected_bundle() {
            let bundle_service = DefaultBundleService::new(config_dir, state_manager.clone())
                .with_package_manager(package_manager)
                .with_service_manager(service_manager);
            if let Err(e) = bundle_service.activate(bundle_id) {
                self.processing = false;
                self.error = Some(format!("Failed to activate bundle: {:?}", e));
                return Err(self.error.clone().unwrap());
            }
        }

        // Apply active profile if selected (creates symlinks + hooks)
        if let Some(profile_id) = self.selected_profile() {
            let module_service =
                iron_core::services::DefaultModuleService::new(config_dir, state_manager.clone());
            let profile_service = iron_core::services::DefaultProfileService::new(
                config_dir,
                state_manager.clone(),
                module_service,
            );
            if let Err(e) = iron_core::services::ProfileService::apply(&profile_service, profile_id)
            {
                self.processing = false;
                self.error = Some(format!("Failed to apply profile: {:?}", e));
                return Err(self.error.clone().unwrap());
            }
        }

        self.processing = false;
        self.next_step(); // Move to Complete
        Ok(())
    }
}

/// Input mode for text fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

/// Text input state
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    /// Current value
    pub value: String,
    /// Cursor position
    pub cursor: usize,
    /// Input mode
    pub mode: InputMode,
}

impl TextInput {
    pub fn new(initial: impl Into<String>) -> Self {
        let value: String = initial.into();
        let cursor = value.len();
        Self {
            value,
            cursor,
            mode: InputMode::Normal,
        }
    }

    pub fn enter_edit_mode(&mut self) {
        self.mode = InputMode::Editing;
    }

    pub fn exit_edit_mode(&mut self) {
        self.mode = InputMode::Normal;
    }

    pub fn is_editing(&self) -> bool {
        self.mode == InputMode::Editing
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn delete(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // WizardStep tests
    // ==========================================================================

    #[test]
    fn test_wizard_step_default() {
        let step = WizardStep::default();
        assert_eq!(step, WizardStep::Welcome);
    }

    #[test]
    fn test_wizard_step_equality() {
        assert_eq!(WizardStep::Welcome, WizardStep::Welcome);
        assert_ne!(WizardStep::Welcome, WizardStep::HostSetup);
    }

    #[test]
    fn test_wizard_step_clone() {
        let step = WizardStep::BundleSelection;
        let cloned = step.clone();
        assert_eq!(step, cloned);
    }

    // ==========================================================================
    // WizardState progression tests
    // ==========================================================================

    #[test]
    fn test_wizard_step_progression() {
        let mut state = WizardState::new();
        assert_eq!(state.step, WizardStep::Welcome);

        state.next_step();
        assert_eq!(state.step, WizardStep::HostSetup);

        state.next_step();
        assert_eq!(state.step, WizardStep::BundleSelection);

        state.prev_step();
        assert_eq!(state.step, WizardStep::HostSetup);
    }

    #[test]
    fn test_wizard_full_progression() {
        let mut state = WizardState::new();

        // Forward through all steps
        assert_eq!(state.step_number(), 1);
        state.next_step();
        assert_eq!(state.step_number(), 2);
        state.next_step();
        assert_eq!(state.step_number(), 3);
        state.next_step();
        assert_eq!(state.step_number(), 4);
        state.next_step();
        assert_eq!(state.step_number(), 5);
        state.next_step();
        assert_eq!(state.step_number(), 6); // Complete
    }

    #[test]
    fn test_wizard_cannot_go_past_complete() {
        let mut state = WizardState::new();
        state.step = WizardStep::Complete;
        state.next_step();
        assert_eq!(state.step, WizardStep::Complete);
    }

    #[test]
    fn test_wizard_cannot_go_before_welcome() {
        let mut state = WizardState::new();
        state.prev_step();
        assert_eq!(state.step, WizardStep::Welcome);
    }

    // ==========================================================================
    // WizardState can_proceed tests
    // ==========================================================================

    #[test]
    fn test_wizard_can_proceed() {
        let mut state = WizardState::new();
        assert!(state.can_proceed()); // Welcome always can proceed

        state.next_step(); // HostSetup
        assert!(!state.can_proceed()); // Need host ID

        state.host_id = "test-host".to_string();
        assert!(state.can_proceed());
    }

    #[test]
    fn test_wizard_can_proceed_bundle_selection() {
        let mut state = WizardState::new();
        state.step = WizardStep::BundleSelection;
        assert!(!state.can_proceed()); // No bundles available

        state.available_bundles = vec!["hyprland".to_string()];
        assert!(state.can_proceed());
    }

    #[test]
    fn test_wizard_can_proceed_profile_selection() {
        let mut state = WizardState::new();
        state.step = WizardStep::ProfileSelection;
        // Profile is optional, so always can proceed
        assert!(state.can_proceed());
    }

    #[test]
    fn test_wizard_can_proceed_confirmation() {
        let mut state = WizardState::new();
        state.step = WizardStep::Confirmation;
        assert!(state.can_proceed());
    }

    #[test]
    fn test_wizard_cannot_proceed_from_complete() {
        let mut state = WizardState::new();
        state.step = WizardStep::Complete;
        assert!(!state.can_proceed());
    }

    // ==========================================================================
    // WizardState can_go_back tests
    // ==========================================================================

    #[test]
    fn test_wizard_can_go_back() {
        let mut state = WizardState::new();
        assert!(!state.can_go_back()); // Can't go back from Welcome

        state.step = WizardStep::HostSetup;
        assert!(state.can_go_back());

        state.step = WizardStep::Complete;
        assert!(!state.can_go_back()); // Can't go back from Complete
    }

    // ==========================================================================
    // WizardState selection tests
    // ==========================================================================

    #[test]
    fn test_wizard_bundle_selection() {
        let mut state = WizardState::new();
        state.available_bundles = vec![
            "hyprland".to_string(),
            "niri".to_string(),
            "sway".to_string(),
        ];

        assert_eq!(state.selected_bundle(), Some("hyprland"));

        state.select_next_bundle();
        assert_eq!(state.selected_bundle(), Some("niri"));

        state.select_next_bundle();
        assert_eq!(state.selected_bundle(), Some("sway"));

        state.select_next_bundle(); // Wraps around
        assert_eq!(state.selected_bundle(), Some("hyprland"));

        state.select_prev_bundle(); // Wraps back
        assert_eq!(state.selected_bundle(), Some("sway"));
    }

    #[test]
    fn test_wizard_profile_selection() {
        let mut state = WizardState::new();
        state.available_profiles = vec!["minimal".to_string(), "developer".to_string()];

        assert_eq!(state.selected_profile(), Some("minimal"));

        state.select_next_profile();
        assert_eq!(state.selected_profile(), Some("developer"));

        state.select_prev_profile();
        assert_eq!(state.selected_profile(), Some("minimal"));
    }

    #[test]
    fn test_wizard_empty_bundles_selection() {
        let mut state = WizardState::new();
        // No bundles
        state.select_next_bundle();
        assert_eq!(state.selected_bundle(), None);
        state.select_prev_bundle();
        assert_eq!(state.selected_bundle(), None);
    }

    #[test]
    fn test_wizard_empty_profiles_selection() {
        let mut state = WizardState::new();
        // No profiles
        state.select_next_profile();
        assert_eq!(state.selected_profile(), None);
        state.select_prev_profile();
        assert_eq!(state.selected_profile(), None);
    }

    // ==========================================================================
    // WizardState step_number tests
    // ==========================================================================

    #[test]
    fn test_wizard_step_numbers() {
        let mut state = WizardState::new();
        assert_eq!(state.step_number(), 1);
        assert_eq!(state.total_steps(), 5);

        state.step = WizardStep::HostSetup;
        assert_eq!(state.step_number(), 2);

        state.step = WizardStep::BundleSelection;
        assert_eq!(state.step_number(), 3);

        state.step = WizardStep::ProfileSelection;
        assert_eq!(state.step_number(), 4);

        state.step = WizardStep::Confirmation;
        assert_eq!(state.step_number(), 5);

        state.step = WizardStep::Complete;
        assert_eq!(state.step_number(), 6);
    }

    // ==========================================================================
    // WizardState error clearing tests
    // ==========================================================================

    #[test]
    fn test_wizard_clears_error_on_step_change() {
        let mut state = WizardState::new();
        state.error = Some("Test error".to_string());

        state.next_step();
        assert!(state.error.is_none());

        state.error = Some("Another error".to_string());
        state.prev_step();
        assert!(state.error.is_none());
    }

    // ==========================================================================
    // InputMode tests
    // ==========================================================================

    #[test]
    fn test_input_mode_default() {
        let mode = InputMode::default();
        assert_eq!(mode, InputMode::Normal);
    }

    #[test]
    fn test_input_mode_equality() {
        assert_eq!(InputMode::Normal, InputMode::Normal);
        assert_eq!(InputMode::Editing, InputMode::Editing);
        assert_ne!(InputMode::Normal, InputMode::Editing);
    }

    // ==========================================================================
    // TextInput tests
    // ==========================================================================

    #[test]
    fn test_text_input() {
        let mut input = TextInput::new("hello");
        assert_eq!(input.value, "hello");
        assert_eq!(input.cursor, 5);

        input.insert('!');
        assert_eq!(input.value, "hello!");

        input.delete();
        assert_eq!(input.value, "hello");

        input.move_start();
        assert_eq!(input.cursor, 0);

        input.insert('H');
        assert_eq!(input.value, "Hhello");
    }

    #[test]
    fn test_text_input_default() {
        let input = TextInput::default();
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);
        assert_eq!(input.mode, InputMode::Normal);
    }

    #[test]
    fn test_text_input_edit_mode() {
        let mut input = TextInput::new("test");
        assert!(!input.is_editing());

        input.enter_edit_mode();
        assert!(input.is_editing());

        input.exit_edit_mode();
        assert!(!input.is_editing());
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut input = TextInput::new("hello");

        input.move_start();
        assert_eq!(input.cursor, 0);

        input.move_right();
        assert_eq!(input.cursor, 1);

        input.move_end();
        assert_eq!(input.cursor, 5);

        input.move_left();
        assert_eq!(input.cursor, 4);
    }

    #[test]
    fn test_text_input_cursor_bounds() {
        let mut input = TextInput::new("hi");

        input.move_start();
        input.move_left(); // Should not go below 0
        assert_eq!(input.cursor, 0);

        input.move_end();
        input.move_right(); // Should not go past length
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_text_input_delete_forward() {
        let mut input = TextInput::new("hello");
        input.move_start();
        input.delete_forward();
        assert_eq!(input.value, "ello");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_text_input_delete_at_start() {
        let mut input = TextInput::new("hello");
        input.move_start();
        input.delete(); // Should do nothing at start
        assert_eq!(input.value, "hello");
    }

    #[test]
    fn test_text_input_delete_forward_at_end() {
        let mut input = TextInput::new("hello");
        input.move_end();
        input.delete_forward(); // Should do nothing at end
        assert_eq!(input.value, "hello");
    }

    #[test]
    fn test_text_input_clear() {
        let mut input = TextInput::new("hello world");
        input.clear();
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_text_input_insert_in_middle() {
        let mut input = TextInput::new("hllo");
        input.cursor = 1;
        input.insert('e');
        assert_eq!(input.value, "hello");
        assert_eq!(input.cursor, 2);
    }
}
