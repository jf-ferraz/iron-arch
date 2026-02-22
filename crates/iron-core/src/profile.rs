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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_profile() -> Profile {
        Profile {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: Some("Development environment".to_string()),
            modules: vec!["nvim-ide".to_string(), "tmux-config".to_string()],
            theme: Some("catppuccin".to_string()),
            shell: Some("fish".to_string()),
            extends: None,
            for_bundle: None,
        }
    }

    #[test]
    fn test_profile_serialization_roundtrip() {
        let profile = sample_profile();
        let toml = toml::to_string_pretty(&profile).unwrap();
        let deserialized: Profile = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.name, profile.name);
        assert_eq!(deserialized.description, profile.description);
        assert_eq!(deserialized.modules, profile.modules);
        assert_eq!(deserialized.theme, profile.theme);
        assert_eq!(deserialized.shell, profile.shell);
        assert_eq!(deserialized.extends, profile.extends);
        assert_eq!(deserialized.for_bundle, profile.for_bundle);
    }

    #[test]
    fn test_profile_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let profile = sample_profile();

        profile.save(temp_dir.path()).unwrap();
        let loaded = Profile::load(temp_dir.path()).unwrap();

        assert_eq!(loaded.id, "developer");
        assert_eq!(loaded.name, "Developer");
        assert_eq!(loaded.modules.len(), 2);
        assert_eq!(loaded.modules[0], "nvim-ide");
    }

    #[test]
    fn test_profile_load_nonexistent_fails() {
        let result = Profile::load(Path::new("/nonexistent/profile"));
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("nested").join("dir");
        let profile = sample_profile();

        profile.save(&nested).unwrap();
        assert!(nested.join("profile.toml").exists());
    }

    #[test]
    fn test_profile_all_modules_returns_own_modules() {
        let profile = sample_profile();
        let modules = profile.all_modules(&[]);
        assert_eq!(modules, vec!["nvim-ide", "tmux-config"]);
    }

    #[test]
    fn test_profile_minimal_deserialization() {
        let toml = r#"
id = "minimal"
name = "Minimal"
modules = []
"#;
        let profile: Profile = toml::from_str(toml).unwrap();
        assert_eq!(profile.id, "minimal");
        assert!(profile.description.is_none());
        assert!(profile.theme.is_none());
        assert!(profile.shell.is_none());
        assert!(profile.extends.is_none());
        assert!(profile.for_bundle.is_none());
        assert!(profile.modules.is_empty());
    }

    #[test]
    fn test_profile_with_extends() {
        let toml = r#"
id = "developer-pro"
name = "Developer Pro"
modules = ["extra-tools"]
extends = "developer"
for_bundle = "hyprland"
"#;
        let profile: Profile = toml::from_str(toml).unwrap();
        assert_eq!(profile.extends.as_deref(), Some("developer"));
        assert_eq!(profile.for_bundle.as_deref(), Some("hyprland"));
    }

    #[test]
    fn test_profile_state_equality() {
        assert_eq!(ProfileState::Active, ProfileState::Active);
        assert_ne!(ProfileState::Active, ProfileState::Inactive);
        assert_ne!(ProfileState::Partial, ProfileState::Active);
    }

    #[test]
    fn test_profile_state_serialization() {
        let active = serde_json::to_string(&ProfileState::Active).unwrap();
        let inactive = serde_json::to_string(&ProfileState::Inactive).unwrap();
        let partial = serde_json::to_string(&ProfileState::Partial).unwrap();

        assert_eq!(active, "\"Active\"");
        assert_eq!(inactive, "\"Inactive\"");
        assert_eq!(partial, "\"Partial\"");

        // Roundtrip
        let deserialized: ProfileState = serde_json::from_str(&active).unwrap();
        assert_eq!(deserialized, ProfileState::Active);
    }

    #[test]
    fn test_profile_clone_independence() {
        let original = sample_profile();
        let mut cloned = original.clone();
        cloned.id = "modified".to_string();
        cloned.modules.push("new-module".to_string());

        assert_eq!(original.id, "developer");
        assert_eq!(original.modules.len(), 2);
        assert_eq!(cloned.id, "modified");
        assert_eq!(cloned.modules.len(), 3);
    }
}
