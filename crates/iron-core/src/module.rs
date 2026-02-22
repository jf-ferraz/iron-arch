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

    /// Pre-uninstall hook script
    #[serde(default)]
    pub pre_uninstall: Option<String>,

    /// Status check hook script
    #[serde(default)]
    pub status_check: Option<String>,

    /// Install ordering priority (lower = first)
    #[serde(default)]
    pub priority: Option<u32>,

    /// Whether hooks require root privileges
    #[serde(default)]
    pub requires_root: bool,
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

    /// Security hardening
    SecurityHardening,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_module() -> Module {
        Module {
            id: "nvim-ide".to_string(),
            name: "Neovim IDE".to_string(),
            description: Some("Full IDE experience with LSP".to_string()),
            kind: ModuleKind::AppConfig,
            packages: vec!["neovim".to_string(), "ripgrep".to_string()],
            aur_packages: vec!["neovim-nightly-bin".to_string()],
            dotfiles: vec![
                DotfileMapping {
                    source: "nvim".to_string(),
                    target: "~/.config/nvim".to_string(),
                    link: true,
                },
                DotfileMapping {
                    source: "lazygit".to_string(),
                    target: "~/.config/lazygit".to_string(),
                    link: true,
                },
            ],
            conflicts: vec!["nvim-minimal".to_string()],
            depends: vec!["base-dev".to_string()],
            pre_install: None,
            post_install: Some("nvim --headless +Lazy! sync +qa".to_string()),
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
        }
    }

    #[test]
    fn test_module_creation() {
        let module = create_test_module();
        assert_eq!(module.id, "nvim-ide");
        assert_eq!(module.name, "Neovim IDE");
        assert!(module.description.is_some());
    }

    #[test]
    fn test_module_kind_variants() {
        let kinds = vec![
            ModuleKind::AppConfig,
            ModuleKind::Shell,
            ModuleKind::DesktopComponent,
            ModuleKind::Theme,
            ModuleKind::SystemUtil,
            ModuleKind::DevTools,
            ModuleKind::SecurityHardening,
        ];

        for kind in kinds {
            assert!(!format!("{:?}", kind).is_empty());
        }
    }

    #[test]
    fn test_module_state_variants() {
        let not_installed = ModuleState::NotInstalled;
        let installed = ModuleState::Installed;
        let partial = ModuleState::Partial;

        assert_ne!(not_installed, installed);
        assert_ne!(installed, partial);
        assert_ne!(not_installed, partial);
    }

    #[test]
    fn test_module_state_equality() {
        let state1 = ModuleState::Installed;
        let state2 = ModuleState::Installed;

        assert_eq!(state1, state2);
    }

    #[test]
    fn test_dotfile_mapping_default() {
        let mapping = DotfileMapping::default();

        assert!(mapping.source.is_empty());
        assert!(mapping.target.is_empty());
        assert!(mapping.link);
    }

    #[test]
    fn test_dotfile_mapping_copy_mode() {
        let mapping = DotfileMapping {
            source: "config".to_string(),
            target: "~/.config/app".to_string(),
            link: false,
        };

        assert!(!mapping.link);
    }

    #[test]
    fn test_conflicts_with() {
        let module = create_test_module();

        assert!(module.conflicts_with("nvim-minimal"));
        assert!(!module.conflicts_with("nvim-ide"));
        assert!(!module.conflicts_with("kitty"));
    }

    #[test]
    fn test_target_paths() {
        let module = create_test_module();
        let paths = module.target_paths();

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"~/.config/nvim".to_string()));
        assert!(paths.contains(&"~/.config/lazygit".to_string()));
    }

    #[test]
    fn test_target_paths_empty() {
        let module = Module {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: None,
            kind: ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
        };

        assert!(module.target_paths().is_empty());
    }

    #[test]
    fn test_module_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let module = create_test_module();

        // Save
        module.save(temp_dir.path()).unwrap();

        // Verify file exists
        let config_path = temp_dir.path().join("module.toml");
        assert!(config_path.exists());

        // Load and verify
        let loaded = Module::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.id, module.id);
        assert_eq!(loaded.name, module.name);
        assert_eq!(loaded.dotfiles.len(), module.dotfiles.len());
    }

    #[test]
    fn test_module_toml_roundtrip() {
        let module = create_test_module();
        let serialized = toml::to_string_pretty(&module).unwrap();
        let deserialized: Module = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, module.id);
        assert_eq!(deserialized.packages, module.packages);
        assert_eq!(deserialized.depends, module.depends);
    }

    #[test]
    fn test_module_with_hooks() {
        let module = create_test_module();

        assert!(module.pre_install.is_none());
        assert!(module.post_install.is_some());
        assert!(module.post_install.unwrap().contains("nvim"));
    }

    #[test]
    fn test_module_dependencies() {
        let module = create_test_module();

        assert_eq!(module.depends.len(), 1);
        assert!(module.depends.contains(&"base-dev".to_string()));
    }

    #[test]
    fn test_module_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = Module::load(temp_dir.path());
        assert!(result.is_err());
    }
}
