//! Screen Trait Pattern
//!
//! Standardizes screen behavior across the TUI application.
//! Inspired by dcli patterns for consistent screen management.

use crate::app::{App, View};
use crossterm::event::KeyEvent;
use ratatui::prelude::*;

/// Result type for screen actions
pub type ScreenResult<T> = anyhow::Result<T>;

/// Actions that can be returned from screen handlers
#[derive(Debug, Clone)]
pub enum ScreenAction {
    /// No action needed
    None,
    /// Navigate back to previous view
    Back,
    /// Navigate to a specific view
    Navigate(View),
    /// Refresh the current screen data
    Refresh,
    /// Show a dialog overlay
    ShowDialog(Dialog),
    /// Close currently shown dialog
    CloseDialog,
    /// Quit the application
    Quit,
}

/// Dialog types that can be shown as overlays
#[derive(Debug, Clone)]
pub enum Dialog {
    /// Help overlay
    Help,
    /// Confirmation dialog with message
    Confirm(ConfirmDialog),
    /// Progress indicator for long operations
    Progress(ProgressDialog),
    /// Error message display
    Error(String),
    /// Info message display
    Info(String),
}

/// Confirmation dialog data
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    /// Dialog title
    pub title: String,
    /// Message to display
    pub message: String,
    /// Action to execute on confirm
    pub on_confirm: ConfirmCallback,
}

/// Callback identifier for confirm actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmCallback {
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
    /// Run system cleanup
    RunCleanup,
    /// Install security module
    InstallSecurityModule(String),
    /// Generic quit confirmation
    Quit,
}

/// Progress dialog data for long-running operations
#[derive(Debug, Clone)]
pub struct ProgressDialog {
    /// Operation title
    pub title: String,
    /// Current operation description
    pub description: String,
    /// Progress percentage (0-100), None for indeterminate
    pub percentage: Option<u8>,
    /// Whether the operation can be cancelled
    pub cancellable: bool,
}

impl ProgressDialog {
    /// Create a new indeterminate progress dialog
    pub fn indeterminate(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            percentage: None,
            cancellable: false,
        }
    }

    /// Create a new progress dialog with percentage
    pub fn with_progress(
        title: impl Into<String>,
        description: impl Into<String>,
        percentage: u8,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            percentage: Some(percentage.min(100)),
            cancellable: false,
        }
    }

    /// Make this progress dialog cancellable
    pub fn cancellable(mut self) -> Self {
        self.cancellable = true;
        self
    }

    /// Update the progress percentage
    pub fn set_progress(&mut self, percentage: u8) {
        self.percentage = Some(percentage.min(100));
    }

    /// Update the description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = description.into();
    }
}

/// Screen trait for standardized screen behavior
///
/// Each screen in the TUI implements this trait to provide consistent
/// handling of key events, rendering, and lifecycle management.
pub trait Screen {
    /// Handle a key event
    ///
    /// Returns a ScreenAction indicating what should happen next.
    /// The app will process this action after the handler returns.
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> ScreenResult<ScreenAction>;

    /// Render the screen content
    ///
    /// This is called every frame to draw the screen's content.
    /// The area provided is the usable space (excluding header/footer).
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    /// Called when the screen becomes active
    ///
    /// Use this for lazy loading of data or initializing screen state.
    /// Default implementation does nothing.
    fn on_activate(&mut self, _app: &mut App) -> ScreenResult<()> {
        Ok(())
    }

    /// Called when the screen is deactivated
    ///
    /// Use this for cleanup or saving state before navigating away.
    /// Default implementation does nothing.
    fn on_deactivate(&mut self, _app: &mut App) -> ScreenResult<()> {
        Ok(())
    }

    /// Get the screen's title for display in the header
    fn title(&self) -> &str;

    /// Get available keybindings for the footer
    ///
    /// Returns a list of (key, description) pairs.
    fn keybindings(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("[Esc]", "Back"),
            ("[?]", "Help"),
            ("[q]", "Quit"),
        ]
    }
}

/// Screen registry for managing screen instances
///
/// This allows for lazy initialization and caching of screen state.
pub struct ScreenRegistry {
    screens: std::collections::HashMap<View, Box<dyn Screen + Send>>,
}

impl Default for ScreenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            screens: std::collections::HashMap::new(),
        }
    }

    /// Register a screen for a view
    pub fn register(&mut self, view: View, screen: Box<dyn Screen + Send>) {
        self.screens.insert(view, screen);
    }

    /// Get a mutable reference to a screen
    pub fn get_mut(&mut self, view: &View) -> Option<&mut Box<dyn Screen + Send>> {
        self.screens.get_mut(view)
    }

    /// Check if a screen is registered for a view
    pub fn has(&self, view: &View) -> bool {
        self.screens.contains_key(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_action_none() {
        let action = ScreenAction::None;
        assert!(matches!(action, ScreenAction::None));
    }

    #[test]
    fn test_screen_action_navigate() {
        let action = ScreenAction::Navigate(View::Dashboard);
        if let ScreenAction::Navigate(view) = action {
            assert_eq!(view, View::Dashboard);
        } else {
            panic!("Expected Navigate variant");
        }
    }

    #[test]
    fn test_screen_action_show_dialog() {
        let dialog = Dialog::Help;
        let action = ScreenAction::ShowDialog(dialog);
        assert!(matches!(action, ScreenAction::ShowDialog(Dialog::Help)));
    }

    #[test]
    fn test_confirm_dialog() {
        let dialog = ConfirmDialog {
            title: "Confirm".to_string(),
            message: "Are you sure?".to_string(),
            on_confirm: ConfirmCallback::Quit,
        };
        assert_eq!(dialog.title, "Confirm");
        assert_eq!(dialog.message, "Are you sure?");
        assert_eq!(dialog.on_confirm, ConfirmCallback::Quit);
    }

    #[test]
    fn test_progress_dialog_indeterminate() {
        let progress = ProgressDialog::indeterminate("Loading", "Please wait...");
        assert_eq!(progress.title, "Loading");
        assert_eq!(progress.description, "Please wait...");
        assert!(progress.percentage.is_none());
        assert!(!progress.cancellable);
    }

    #[test]
    fn test_progress_dialog_with_progress() {
        let progress = ProgressDialog::with_progress("Downloading", "50%", 50);
        assert_eq!(progress.percentage, Some(50));
    }

    #[test]
    fn test_progress_dialog_clamps_percentage() {
        let progress = ProgressDialog::with_progress("Test", "Over 100", 150);
        assert_eq!(progress.percentage, Some(100));
    }

    #[test]
    fn test_progress_dialog_cancellable() {
        let progress = ProgressDialog::indeterminate("Test", "Test").cancellable();
        assert!(progress.cancellable);
    }

    #[test]
    fn test_progress_dialog_set_progress() {
        let mut progress = ProgressDialog::indeterminate("Test", "Test");
        progress.set_progress(75);
        assert_eq!(progress.percentage, Some(75));
    }

    #[test]
    fn test_progress_dialog_set_description() {
        let mut progress = ProgressDialog::indeterminate("Test", "Initial");
        progress.set_description("Updated");
        assert_eq!(progress.description, "Updated");
    }

    #[test]
    fn test_confirm_callback_variants() {
        let callbacks = vec![
            ConfirmCallback::SwitchBundle("test".to_string()),
            ConfirmCallback::RemoveBundle("test".to_string()),
            ConfirmCallback::EnableModule("test".to_string()),
            ConfirmCallback::DisableModule("test".to_string()),
            ConfirmCallback::RunUpdate,
            ConfirmCallback::RunCleanup,
            ConfirmCallback::InstallSecurityModule("test".to_string()),
            ConfirmCallback::Quit,
        ];

        for callback in callbacks {
            let debug = format!("{:?}", callback);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_dialog_variants() {
        let dialogs = vec![
            Dialog::Help,
            Dialog::Confirm(ConfirmDialog {
                title: "Test".to_string(),
                message: "Test".to_string(),
                on_confirm: ConfirmCallback::Quit,
            }),
            Dialog::Progress(ProgressDialog::indeterminate("Test", "Test")),
            Dialog::Error("Error".to_string()),
            Dialog::Info("Info".to_string()),
        ];

        for dialog in dialogs {
            let debug = format!("{:?}", dialog);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_screen_registry_new() {
        let registry = ScreenRegistry::new();
        assert!(!registry.has(&View::Dashboard));
    }

    #[test]
    fn test_screen_registry_default() {
        let registry = ScreenRegistry::default();
        assert!(!registry.has(&View::Bundles));
    }
}
