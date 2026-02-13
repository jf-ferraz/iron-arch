//! Bundle management - Desktop environment handling

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A bundle represents a complete desktop environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    /// Unique identifier (e.g., "hyprland", "niri", "kde")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Bundle type
    pub bundle_type: BundleType,

    /// Core packages required for this bundle
    pub packages: Vec<String>,

    /// AUR packages
    pub aur_packages: Vec<String>,

    /// Available profiles for this bundle
    pub profiles: Vec<String>,

    /// Default profile
    pub default_profile: Option<String>,

    /// Conflicts with other bundles
    pub conflicts: Vec<String>,

    /// Services to enable
    pub services: Vec<String>,

    /// Post-install hooks
    pub post_install: Option<String>,
}

/// Bundle type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleType {
    /// Wayland compositor (Hyprland, Niri, Sway)
    WaylandCompositor,

    /// Full desktop environment (KDE, GNOME)
    DesktopEnvironment,

    /// X11 window manager
    X11WindowManager,
}

/// Bundle state on a host
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleState {
    /// Not installed
    NotInstalled,

    /// Installed but not active (configs in dormant/)
    Dormant,

    /// Installed and active (configs linked)
    Active,
}

impl Bundle {
    /// Load bundle configuration from a directory
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("bundle.toml");
        let content = std::fs::read_to_string(&config_path)?;
        let bundle: Bundle = toml::from_str(&content)?;
        Ok(bundle)
    }

    /// Save bundle configuration to a directory
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        let config_path = path.join("bundle.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Check if this bundle conflicts with another
    pub fn conflicts_with(&self, other: &str) -> bool {
        self.conflicts.iter().any(|c| c == other)
    }
}
