//! Profile management - Dotfile collections

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A profile is a collection of modules (dotfiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Modules included in this profile
    pub modules: Vec<String>,

    /// Theme identifier
    pub theme: Option<String>,

    /// Shell preference
    pub shell: Option<String>,

    /// Parent profile to inherit from
    pub extends: Option<String>,

    /// Bundle this profile is designed for (optional)
    pub for_bundle: Option<String>,
}

/// Profile state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileState {
    /// Not applied
    Inactive,

    /// Currently applied
    Active,

    /// Partially applied (some modules active)
    Partial,
}

impl Profile {
    /// Load profile configuration from a directory
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("profile.toml");
        let content = std::fs::read_to_string(&config_path)?;
        let profile: Profile = toml::from_str(&content)?;
        Ok(profile)
    }

    /// Save profile configuration to a directory
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        let config_path = path.join("profile.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get all modules including inherited ones
    pub fn all_modules(&self, _profiles: &[Profile]) -> Vec<String> {
        // TODO: Resolve inheritance chain
        self.modules.clone()
    }
}
