//! Module management - Individual configuration components

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Controls when and how module hooks are executed.
///
/// Module hooks (pre_install, post_install, pre_uninstall, status_check)
/// are shell commands. This enum controls the execution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HookBehavior {
    /// Run every time the module is applied
    #[default]
    Always,
    /// Run only the first time (tracked in state.json, re-run with --force-hooks)
    Once,
    /// Prompt the user before running (skipped in non-interactive mode)
    Ask,
    /// Never run
    Skip,
}

/// Type of module lifecycle hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Runs before packages are installed and dotfiles deployed
    PreInstall,
    /// Runs after all packages, dotfiles, and services are configured
    PostInstall,
    /// Runs before module removal (packages still available)
    PreUninstall,
    /// Informational check of module health (not part of apply flow)
    StatusCheck,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreInstall => write!(f, "pre_install"),
            Self::PostInstall => write!(f, "post_install"),
            Self::PreUninstall => write!(f, "pre_uninstall"),
            Self::StatusCheck => write!(f, "status_check"),
        }
    }
}

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

    /// F2-019: Security hardening points this module contributes
    #[serde(default)]
    pub security_points: u32,

    /// Hook execution policy for this module
    #[serde(default)]
    pub hook_behavior: HookBehavior,

    /// Auto-mirror all files in module's dotfiles/ directory
    #[serde(default)]
    pub dotfiles_sync: bool,

    /// Override default target directory for dotfiles_sync.
    /// Default is ~/.config/<module-id>/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dotfiles_sync_target: Option<String>,
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
            security_points: 0,
            hook_behavior: HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
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
            security_points: 0,
            hook_behavior: HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
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

    #[test]
    fn test_hook_behavior_default() {
        let behavior = HookBehavior::default();
        assert_eq!(behavior, HookBehavior::Always);
    }

    #[test]
    fn test_hook_behavior_serde_roundtrip() {
        let variants = vec![
            (HookBehavior::Always, "\"always\""),
            (HookBehavior::Once, "\"once\""),
            (HookBehavior::Ask, "\"ask\""),
            (HookBehavior::Skip, "\"skip\""),
        ];
        for (variant, expected_json) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let deser: HookBehavior = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, variant);
        }
    }

    #[test]
    fn test_hook_behavior_toml_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            behavior: HookBehavior,
        }
        for variant in [
            HookBehavior::Always,
            HookBehavior::Once,
            HookBehavior::Ask,
            HookBehavior::Skip,
        ] {
            let w = Wrapper { behavior: variant };
            let toml_str = toml::to_string(&w).unwrap();
            let deser: Wrapper = toml::from_str(&toml_str).unwrap();
            assert_eq!(deser.behavior, variant);
        }
    }

    #[test]
    fn test_hook_type_display() {
        assert_eq!(HookType::PreInstall.to_string(), "pre_install");
        assert_eq!(HookType::PostInstall.to_string(), "post_install");
        assert_eq!(HookType::PreUninstall.to_string(), "pre_uninstall");
        assert_eq!(HookType::StatusCheck.to_string(), "status_check");
    }

    #[test]
    fn test_module_hook_behavior_defaults_on_deserialize() {
        // A minimal TOML without hook_behavior should default to Always
        let module = create_test_module();
        assert_eq!(module.hook_behavior, HookBehavior::Always);
    }
}
