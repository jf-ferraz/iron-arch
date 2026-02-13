//! Module management - Individual configuration components

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A module is the atomic unit of configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Module kind
    pub kind: ModuleKind,

    /// Packages required by this module
    pub packages: Vec<String>,

    /// AUR packages
    pub aur_packages: Vec<String>,

    /// Dotfiles to link
    pub dotfiles: Vec<DotfileMapping>,

    /// Conflicts with other modules
    pub conflicts: Vec<String>,

    /// Dependencies (other modules)
    pub depends: Vec<String>,

    /// Pre-install hook script
    pub pre_install: Option<String>,

    /// Post-install hook script
    pub post_install: Option<String>,
}

/// Module classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleKind {
    /// Application configuration (nvim, kitty, etc.)
    AppConfig,

    /// Shell configuration (bash, zsh, fish)
    Shell,

    /// Desktop component (waybar, rofi, etc.)
    DesktopComponent,

    /// Theme assets
    Theme,

    /// System utilities
    SystemUtil,

    /// Development tools
    DevTools,
}

/// Mapping of dotfile source to target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotfileMapping {
    /// Source path relative to module directory
    pub source: String,

    /// Target path (supports ~ expansion)
    pub target: String,

    /// Whether to create a symlink or copy
    pub link: bool,
}

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Not installed
    NotInstalled,

    /// Installed (packages present, dotfiles linked)
    Installed,

    /// Partially installed
    Partial,
}

impl Module {
    /// Load module configuration from a directory
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("module.toml");
        let content = std::fs::read_to_string(&config_path)?;
        let module: Module = toml::from_str(&content)?;
        Ok(module)
    }

    /// Save module configuration to a directory
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        let config_path = path.join("module.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Check if this module conflicts with another
    pub fn conflicts_with(&self, other: &str) -> bool {
        self.conflicts.iter().any(|c| c == other)
    }

    /// Get all dotfile target paths
    pub fn target_paths(&self) -> Vec<String> {
        self.dotfiles.iter().map(|d| d.target.clone()).collect()
    }
}

impl Default for DotfileMapping {
    fn default() -> Self {
        Self {
            source: String::new(),
            target: String::new(),
            link: true,
        }
    }
}
