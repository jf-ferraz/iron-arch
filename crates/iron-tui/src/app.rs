//! Iron TUI Application

/// Application state
pub struct App {
    /// Current view
    pub view: View,

    /// Should quit
    pub should_quit: bool,

    /// Current host
    pub current_host: Option<String>,

    /// Active bundle
    pub active_bundle: Option<String>,

    /// Active profile
    pub active_profile: Option<String>,
}

/// Available views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Dashboard home
    Dashboard,

    /// First-time setup wizard
    SetupWizard,

    /// Bundle management
    Bundles,

    /// Profile management
    Profiles,

    /// Module management
    Modules,

    /// Update preview
    UpdatePreview,

    /// Settings
    Settings,
}

impl Default for App {
    fn default() -> Self {
        Self {
            view: View::Dashboard,
            should_quit: false,
            current_host: None,
            active_bundle: None,
            active_profile: None,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn navigate(&mut self, view: View) {
        self.view = view;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
