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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_bundle() -> Bundle {
        Bundle {
            id: "hyprland".to_string(),
            name: "Hyprland".to_string(),
            description: Some("Modern Wayland compositor".to_string()),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec!["hyprland".to_string(), "waybar".to_string()],
            aur_packages: vec!["hyprshot".to_string()],
            profiles: vec!["developer".to_string(), "minimal".to_string()],
            default_profile: Some("developer".to_string()),
            conflicts: vec!["niri".to_string(), "sway".to_string()],
            services: vec!["pipewire".to_string()],
            post_install: Some("hyprctl reload".to_string()),
        }
    }

    #[test]
    fn test_bundle_creation() {
        let bundle = create_test_bundle();
        assert_eq!(bundle.id, "hyprland");
        assert_eq!(bundle.name, "Hyprland");
        assert!(bundle.description.is_some());
    }

    #[test]
    fn test_bundle_type_variants() {
        let wayland = BundleType::WaylandCompositor;
        let desktop = BundleType::DesktopEnvironment;
        let x11 = BundleType::X11WindowManager;

        // Test debug formatting
        assert!(format!("{:?}", wayland).contains("WaylandCompositor"));
        assert!(format!("{:?}", desktop).contains("DesktopEnvironment"));
        assert!(format!("{:?}", x11).contains("X11WindowManager"));
    }

    #[test]
    fn test_bundle_state_variants() {
        let not_installed = BundleState::NotInstalled;
        let dormant = BundleState::Dormant;
        let active = BundleState::Active;

        assert_ne!(not_installed, dormant);
        assert_ne!(dormant, active);
        assert_ne!(not_installed, active);
    }

    #[test]
    fn test_bundle_state_equality() {
        let state1 = BundleState::Active;
        let state2 = BundleState::Active;
        let state3 = BundleState::Dormant;

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_conflicts_with() {
        let bundle = create_test_bundle();

        assert!(bundle.conflicts_with("niri"));
        assert!(bundle.conflicts_with("sway"));
        assert!(!bundle.conflicts_with("kde"));
        assert!(!bundle.conflicts_with("hyprland"));
    }

    #[test]
    fn test_bundle_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let bundle = create_test_bundle();

        // Save
        bundle.save(temp_dir.path()).unwrap();

        // Verify file exists
        let config_path = temp_dir.path().join("bundle.toml");
        assert!(config_path.exists());

        // Load and verify
        let loaded = Bundle::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.id, bundle.id);
        assert_eq!(loaded.name, bundle.name);
        assert_eq!(loaded.packages.len(), bundle.packages.len());
        assert_eq!(loaded.conflicts.len(), bundle.conflicts.len());
    }

    #[test]
    fn test_bundle_toml_roundtrip() {
        let bundle = create_test_bundle();
        let serialized = toml::to_string_pretty(&bundle).unwrap();
        let deserialized: Bundle = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, bundle.id);
        assert_eq!(deserialized.name, bundle.name);
        assert_eq!(deserialized.packages, bundle.packages);
        assert_eq!(deserialized.aur_packages, bundle.aur_packages);
    }

    #[test]
    fn test_bundle_minimal() {
        let minimal = Bundle {
            id: "minimal".to_string(),
            name: "Minimal Bundle".to_string(),
            description: None,
            bundle_type: BundleType::WaylandCompositor,
            packages: vec![],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        };

        assert!(minimal.description.is_none());
        assert!(minimal.packages.is_empty());
        assert!(minimal.default_profile.is_none());
        assert!(!minimal.conflicts_with("anything"));
    }

    #[test]
    fn test_bundle_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = Bundle::load(temp_dir.path());
        assert!(result.is_err());
    }
}
