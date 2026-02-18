//! Test Helpers for Iron Core
//!
//! Provides utilities for creating mock filesystems populated with
//! test bundles, modules, and profiles.
//!
//! # Example
//!
//! ```ignore
//! use iron_core::test_helpers::{MockFsBuilder, TestBundle, TestModule};
//!
//! let fs = MockFsBuilder::new("/iron")
//!     .add_bundle(TestBundle::new("hyprland")
//!         .with_package("hyprland")
//!         .with_profile("developer"))
//!     .add_module(TestModule::new("nvim-ide")
//!         .with_package("neovim")
//!         .with_dotfile("config", "~/.config/nvim"))
//!     .build();
//! ```

use crate::bundle::{Bundle, BundleType};
use crate::fs_trait::MockFileSystem;
use crate::module::{DotfileMapping, Module, ModuleKind};
use crate::profile::Profile;
use std::path::{Path, PathBuf};

// =============================================================================
// Test Bundle Builder
// =============================================================================

/// Builder for creating test Bundle configurations
#[derive(Debug, Clone)]
pub struct TestBundle {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub bundle_type: BundleType,
    pub packages: Vec<String>,
    pub aur_packages: Vec<String>,
    pub profiles: Vec<String>,
    pub default_profile: Option<String>,
    pub conflicts: Vec<String>,
    pub services: Vec<String>,
    pub post_install: Option<String>,
    pub dotfiles: Vec<(String, String)>, // (source_name, content)
}

impl TestBundle {
    /// Create a new test bundle with minimal defaults
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            description: None,
            bundle_type: BundleType::WaylandCompositor,
            packages: Vec::new(),
            aur_packages: Vec::new(),
            profiles: Vec::new(),
            default_profile: None,
            conflicts: Vec::new(),
            services: Vec::new(),
            post_install: None,
            dotfiles: Vec::new(),
        }
    }

    /// Set bundle name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set bundle description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set bundle type
    pub fn with_type(mut self, bundle_type: BundleType) -> Self {
        self.bundle_type = bundle_type;
        self
    }

    /// Add a package
    pub fn with_package(mut self, pkg: impl Into<String>) -> Self {
        self.packages.push(pkg.into());
        self
    }

    /// Add multiple packages
    pub fn with_packages(mut self, pkgs: &[&str]) -> Self {
        self.packages.extend(pkgs.iter().map(|s| s.to_string()));
        self
    }

    /// Add an AUR package
    pub fn with_aur_package(mut self, pkg: impl Into<String>) -> Self {
        self.aur_packages.push(pkg.into());
        self
    }

    /// Add a profile reference
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profiles.push(profile.into());
        self
    }

    /// Set default profile
    pub fn with_default_profile(mut self, profile: impl Into<String>) -> Self {
        self.default_profile = Some(profile.into());
        self
    }

    /// Add a conflict
    pub fn with_conflict(mut self, bundle_id: impl Into<String>) -> Self {
        self.conflicts.push(bundle_id.into());
        self
    }

    /// Add a service
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.services.push(service.into());
        self
    }

    /// Add a dotfile (will be placed in bundle's dotfiles/ directory)
    pub fn with_dotfile(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.dotfiles.push((name.into(), content.into()));
        self
    }

    /// Convert to Bundle struct
    pub fn to_bundle(&self) -> Bundle {
        Bundle {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            bundle_type: self.bundle_type.clone(),
            packages: self.packages.clone(),
            aur_packages: self.aur_packages.clone(),
            profiles: self.profiles.clone(),
            default_profile: self.default_profile.clone(),
            conflicts: self.conflicts.clone(),
            services: self.services.clone(),
            post_install: self.post_install.clone(),
        }
    }
}

impl Default for TestBundle {
    fn default() -> Self {
        Self::new("test-bundle")
    }
}

// =============================================================================
// Test Module Builder
// =============================================================================

/// Builder for creating test Module configurations
#[derive(Debug, Clone)]
pub struct TestModule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: ModuleKind,
    pub packages: Vec<String>,
    pub aur_packages: Vec<String>,
    pub dotfiles: Vec<DotfileMapping>,
    pub conflicts: Vec<String>,
    pub depends: Vec<String>,
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    /// Actual file contents to create (source_path, content)
    pub file_contents: Vec<(String, String)>,
}

impl TestModule {
    /// Create a new test module with minimal defaults
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            description: None,
            kind: ModuleKind::AppConfig,
            packages: Vec::new(),
            aur_packages: Vec::new(),
            dotfiles: Vec::new(),
            conflicts: Vec::new(),
            depends: Vec::new(),
            pre_install: None,
            post_install: None,
            file_contents: Vec::new(),
        }
    }

    /// Set module name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set module description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set module kind
    pub fn with_kind(mut self, kind: ModuleKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add a package
    pub fn with_package(mut self, pkg: impl Into<String>) -> Self {
        self.packages.push(pkg.into());
        self
    }

    /// Add multiple packages
    pub fn with_packages(mut self, pkgs: &[&str]) -> Self {
        self.packages.extend(pkgs.iter().map(|s| s.to_string()));
        self
    }

    /// Add a dotfile mapping
    pub fn with_dotfile(mut self, source: impl Into<String>, target: impl Into<String>) -> Self {
        self.dotfiles.push(DotfileMapping {
            source: source.into(),
            target: target.into(),
            link: true,
        });
        self
    }

    /// Add a dotfile mapping with copy (not link)
    pub fn with_dotfile_copy(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        self.dotfiles.push(DotfileMapping {
            source: source.into(),
            target: target.into(),
            link: false,
        });
        self
    }

    /// Add a dependency
    pub fn with_dependency(mut self, module_id: impl Into<String>) -> Self {
        self.depends.push(module_id.into());
        self
    }

    /// Add a conflict
    pub fn with_conflict(mut self, module_id: impl Into<String>) -> Self {
        self.conflicts.push(module_id.into());
        self
    }

    /// Add actual file content (source file within module directory)
    pub fn with_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.file_contents.push((path.into(), content.into()));
        self
    }

    /// Convert to Module struct
    pub fn to_module(&self) -> Module {
        Module {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            kind: self.kind.clone(),
            packages: self.packages.clone(),
            aur_packages: self.aur_packages.clone(),
            dotfiles: self.dotfiles.clone(),
            conflicts: self.conflicts.clone(),
            depends: self.depends.clone(),
            pre_install: self.pre_install.clone(),
            post_install: self.post_install.clone(),
        }
    }
}

impl Default for TestModule {
    fn default() -> Self {
        Self::new("test-module")
    }
}

// =============================================================================
// Test Profile Builder
// =============================================================================

/// Builder for creating test Profile configurations
#[derive(Debug, Clone)]
pub struct TestProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub modules: Vec<String>,
    pub theme: Option<String>,
    pub shell: Option<String>,
    pub extends: Option<String>,
    pub for_bundle: Option<String>,
}

impl TestProfile {
    /// Create a new test profile with minimal defaults
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            description: None,
            modules: Vec::new(),
            theme: None,
            shell: None,
            extends: None,
            for_bundle: None,
        }
    }

    /// Set profile name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set profile description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a module
    pub fn with_module(mut self, module_id: impl Into<String>) -> Self {
        self.modules.push(module_id.into());
        self
    }

    /// Add multiple modules
    pub fn with_modules(mut self, modules: &[&str]) -> Self {
        self.modules.extend(modules.iter().map(|s| s.to_string()));
        self
    }

    /// Set theme
    pub fn with_theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = Some(theme.into());
        self
    }

    /// Set shell
    pub fn with_shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }

    /// Set parent profile
    pub fn extends(mut self, parent: impl Into<String>) -> Self {
        self.extends = Some(parent.into());
        self
    }

    /// Set bundle association
    pub fn for_bundle(mut self, bundle_id: impl Into<String>) -> Self {
        self.for_bundle = Some(bundle_id.into());
        self
    }

    /// Convert to Profile struct
    pub fn to_profile(&self) -> Profile {
        Profile {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            modules: self.modules.clone(),
            theme: self.theme.clone(),
            shell: self.shell.clone(),
            extends: self.extends.clone(),
            for_bundle: self.for_bundle.clone(),
        }
    }
}

impl Default for TestProfile {
    fn default() -> Self {
        Self::new("test-profile")
    }
}

// =============================================================================
// Mock Filesystem Builder
// =============================================================================

/// Builder for creating mock filesystems with Iron project structure
pub struct MockFsBuilder {
    fs: MockFileSystem,
    root: PathBuf,
    bundles: Vec<TestBundle>,
    modules: Vec<TestModule>,
    profiles: Vec<TestProfile>,
    state_content: Option<String>,
}

impl MockFsBuilder {
    /// Create a new builder with specified root directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            fs: MockFileSystem::new(),
            root: root.as_ref().to_path_buf(),
            bundles: Vec::new(),
            modules: Vec::new(),
            profiles: Vec::new(),
            state_content: None,
        }
    }

    /// Add a bundle
    pub fn add_bundle(mut self, bundle: TestBundle) -> Self {
        self.bundles.push(bundle);
        self
    }

    /// Add a module
    pub fn add_module(mut self, module: TestModule) -> Self {
        self.modules.push(module);
        self
    }

    /// Add a profile
    pub fn add_profile(mut self, profile: TestProfile) -> Self {
        self.profiles.push(profile);
        self
    }

    /// Set custom state.json content
    pub fn with_state(mut self, content: impl Into<String>) -> Self {
        self.state_content = Some(content.into());
        self
    }

    /// Build the mock filesystem
    pub fn build(self) -> MockFileSystem {
        // Create root directories
        self.fs.add_dir(&self.root);
        self.fs.add_dir(self.root.join("bundles"));
        self.fs.add_dir(self.root.join("modules"));
        self.fs.add_dir(self.root.join("profiles"));
        self.fs.add_dir(self.root.join("hosts"));

        // Add bundles
        for bundle in &self.bundles {
            let bundle_dir = self.root.join("bundles").join(&bundle.id);
            self.fs.add_dir(&bundle_dir);

            // Write bundle.toml
            let bundle_struct = bundle.to_bundle();
            let toml_content = toml::to_string_pretty(&bundle_struct).unwrap();
            self.fs
                .add_file(bundle_dir.join("bundle.toml"), toml_content);

            // Create dotfiles directory and files
            if !bundle.dotfiles.is_empty() {
                let dotfiles_dir = bundle_dir.join("dotfiles");
                self.fs.add_dir(&dotfiles_dir);

                for (name, content) in &bundle.dotfiles {
                    self.fs.add_file(dotfiles_dir.join(name), content);
                }
            }
        }

        // Add modules
        for module in &self.modules {
            let module_dir = self.root.join("modules").join(&module.id);
            self.fs.add_dir(&module_dir);

            // Write module.toml
            let module_struct = module.to_module();
            let toml_content = toml::to_string_pretty(&module_struct).unwrap();
            self.fs
                .add_file(module_dir.join("module.toml"), toml_content);

            // Create file contents
            for (path, content) in &module.file_contents {
                self.fs.add_file(module_dir.join(path), content);
            }
        }

        // Add profiles
        for profile in &self.profiles {
            let profile_dir = self.root.join("profiles").join(&profile.id);
            self.fs.add_dir(&profile_dir);

            // Write profile.toml
            let profile_struct = profile.to_profile();
            let toml_content = toml::to_string_pretty(&profile_struct).unwrap();
            self.fs
                .add_file(profile_dir.join("profile.toml"), toml_content);
        }

        // Add state.json if specified
        if let Some(content) = self.state_content {
            self.fs.add_file(self.root.join("state.json"), content);
        }

        self.fs
    }

    /// Get the root path
    pub fn root(&self) -> &Path {
        &self.root
    }
}

// =============================================================================
// Preset Test Configurations
// =============================================================================

/// Create a typical hyprland bundle
pub fn hyprland_bundle() -> TestBundle {
    TestBundle::new("hyprland")
        .with_name("Hyprland")
        .with_description("Hyprland tiling compositor")
        .with_type(BundleType::WaylandCompositor)
        .with_packages(&["hyprland", "waybar", "wofi", "hyprpaper"])
        .with_aur_package("hyprshot")
        .with_profile("developer")
        .with_profile("minimal")
        .with_default_profile("developer")
        .with_service("pipewire")
        .with_conflict("niri")
}

/// Create a typical niri bundle
pub fn niri_bundle() -> TestBundle {
    TestBundle::new("niri")
        .with_name("Niri")
        .with_description("Niri scrollable compositor")
        .with_type(BundleType::WaylandCompositor)
        .with_packages(&["niri", "waybar", "fuzzel"])
        .with_profile("minimal")
        .with_default_profile("minimal")
        .with_conflict("hyprland")
}

/// Create a typical nvim-ide module
pub fn nvim_ide_module() -> TestModule {
    TestModule::new("nvim-ide")
        .with_name("Neovim IDE")
        .with_description("Full IDE configuration for Neovim")
        .with_kind(ModuleKind::DevTools)
        .with_packages(&["neovim", "ripgrep", "fd", "lazygit"])
        .with_dotfile("config", "~/.config/nvim")
        .with_file("config/init.lua", "-- Neovim configuration")
}

/// Create a typical kitty-dev module
pub fn kitty_dev_module() -> TestModule {
    TestModule::new("kitty-dev")
        .with_name("Kitty Terminal")
        .with_description("Developer terminal configuration")
        .with_kind(ModuleKind::AppConfig)
        .with_package("kitty")
        .with_dotfile("config", "~/.config/kitty")
        .with_file("config/kitty.conf", "# Kitty configuration")
}

/// Create a developer profile
pub fn developer_profile() -> TestProfile {
    TestProfile::new("developer")
        .with_name("Developer")
        .with_description("Full development workstation setup")
        .with_modules(&["nvim-ide", "kitty-dev", "git-config", "starship"])
        .with_shell("zsh")
}

/// Create a minimal profile
pub fn minimal_profile() -> TestProfile {
    TestProfile::new("minimal")
        .with_name("Minimal")
        .with_description("Minimal system configuration")
        .with_modules(&["kitty-dev"])
}

/// Create a complete test environment with common bundles/modules/profiles
pub fn complete_test_env(root: impl AsRef<Path>) -> MockFileSystem {
    MockFsBuilder::new(root)
        .add_bundle(hyprland_bundle())
        .add_bundle(niri_bundle())
        .add_module(nvim_ide_module())
        .add_module(kitty_dev_module())
        .add_profile(developer_profile())
        .add_profile(minimal_profile())
        .build()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_trait::FileSystem;

    #[test]
    fn test_bundle_builder() {
        let bundle = TestBundle::new("test")
            .with_name("Test Bundle")
            .with_description("A test bundle")
            .with_package("pkg1")
            .with_packages(&["pkg2", "pkg3"])
            .with_profile("developer")
            .to_bundle();

        assert_eq!(bundle.id, "test");
        assert_eq!(bundle.name, "Test Bundle");
        assert_eq!(bundle.packages, vec!["pkg1", "pkg2", "pkg3"]);
        assert_eq!(bundle.profiles, vec!["developer"]);
    }

    #[test]
    fn test_module_builder() {
        let module = TestModule::new("nvim")
            .with_description("Neovim config")
            .with_package("neovim")
            .with_dotfile("config", "~/.config/nvim")
            .with_dependency("base-tools")
            .to_module();

        assert_eq!(module.id, "nvim");
        assert_eq!(module.packages, vec!["neovim"]);
        assert_eq!(module.dotfiles.len(), 1);
        assert_eq!(module.depends, vec!["base-tools"]);
    }

    #[test]
    fn test_profile_builder() {
        let profile = TestProfile::new("dev")
            .with_description("Developer setup")
            .with_modules(&["nvim", "kitty"])
            .with_shell("zsh")
            .to_profile();

        assert_eq!(profile.id, "dev");
        assert_eq!(profile.modules, vec!["nvim", "kitty"]);
        assert_eq!(profile.shell, Some("zsh".to_string()));
    }

    #[test]
    fn test_mock_fs_builder() {
        let fs = MockFsBuilder::new("/iron")
            .add_bundle(TestBundle::new("hyprland").with_package("hyprland"))
            .add_module(TestModule::new("nvim").with_package("neovim"))
            .add_profile(TestProfile::new("dev").with_module("nvim"))
            .build();

        // Verify bundle exists
        assert!(fs.exists(Path::new("/iron/bundles/hyprland")));
        assert!(fs.exists(Path::new("/iron/bundles/hyprland/bundle.toml")));

        // Verify module exists
        assert!(fs.exists(Path::new("/iron/modules/nvim")));
        assert!(fs.exists(Path::new("/iron/modules/nvim/module.toml")));

        // Verify profile exists
        assert!(fs.exists(Path::new("/iron/profiles/dev")));
        assert!(fs.exists(Path::new("/iron/profiles/dev/profile.toml")));
    }

    #[test]
    fn test_mock_fs_builder_with_dotfiles() {
        let fs = MockFsBuilder::new("/iron")
            .add_bundle(
                TestBundle::new("hyprland").with_dotfile("hyprland.conf", "# Hyprland config"),
            )
            .add_module(TestModule::new("nvim").with_file("config/init.lua", "-- Init"))
            .build();

        // Verify bundle dotfile
        assert!(fs.exists(Path::new("/iron/bundles/hyprland/dotfiles/hyprland.conf")));
        assert_eq!(
            fs.get_content("/iron/bundles/hyprland/dotfiles/hyprland.conf"),
            Some("# Hyprland config".to_string())
        );

        // Verify module file
        assert!(fs.exists(Path::new("/iron/modules/nvim/config/init.lua")));
        assert_eq!(
            fs.get_content("/iron/modules/nvim/config/init.lua"),
            Some("-- Init".to_string())
        );
    }

    #[test]
    fn test_preset_bundles() {
        let hypr = hyprland_bundle().to_bundle();
        assert_eq!(hypr.id, "hyprland");
        assert!(hypr.packages.contains(&"hyprland".to_string()));
        assert!(hypr.conflicts.contains(&"niri".to_string()));

        let niri = niri_bundle().to_bundle();
        assert_eq!(niri.id, "niri");
        assert!(niri.conflicts.contains(&"hyprland".to_string()));
    }

    #[test]
    fn test_preset_modules() {
        let nvim = nvim_ide_module().to_module();
        assert_eq!(nvim.id, "nvim-ide");
        assert!(nvim.packages.contains(&"neovim".to_string()));

        let kitty = kitty_dev_module().to_module();
        assert_eq!(kitty.id, "kitty-dev");
    }

    #[test]
    fn test_preset_profiles() {
        let dev = developer_profile().to_profile();
        assert_eq!(dev.id, "developer");
        assert!(dev.modules.contains(&"nvim-ide".to_string()));

        let min = minimal_profile().to_profile();
        assert_eq!(min.id, "minimal");
    }

    #[test]
    fn test_complete_test_env() {
        let fs = complete_test_env("/iron");

        // Should have all preset entities
        assert!(fs.exists(Path::new("/iron/bundles/hyprland/bundle.toml")));
        assert!(fs.exists(Path::new("/iron/bundles/niri/bundle.toml")));
        assert!(fs.exists(Path::new("/iron/modules/nvim-ide/module.toml")));
        assert!(fs.exists(Path::new("/iron/modules/kitty-dev/module.toml")));
        assert!(fs.exists(Path::new("/iron/profiles/developer/profile.toml")));
        assert!(fs.exists(Path::new("/iron/profiles/minimal/profile.toml")));
    }

    #[test]
    fn test_bundle_toml_parsing() {
        let fs = MockFsBuilder::new("/iron")
            .add_bundle(hyprland_bundle())
            .build();

        // Read and parse the generated TOML
        let content = fs
            .read_to_string(Path::new("/iron/bundles/hyprland/bundle.toml"))
            .unwrap();
        let parsed: Bundle = toml::from_str(&content).unwrap();

        assert_eq!(parsed.id, "hyprland");
        assert_eq!(parsed.name, "Hyprland");
        assert!(parsed.packages.contains(&"hyprland".to_string()));
    }

    #[test]
    fn test_module_toml_parsing() {
        let fs = MockFsBuilder::new("/iron")
            .add_module(nvim_ide_module())
            .build();

        // Read and parse the generated TOML
        let content = fs
            .read_to_string(Path::new("/iron/modules/nvim-ide/module.toml"))
            .unwrap();
        let parsed: Module = toml::from_str(&content).unwrap();

        assert_eq!(parsed.id, "nvim-ide");
        assert!(parsed.packages.contains(&"neovim".to_string()));
    }

    #[test]
    fn test_profile_toml_parsing() {
        let fs = MockFsBuilder::new("/iron")
            .add_profile(developer_profile())
            .build();

        // Read and parse the generated TOML
        let content = fs
            .read_to_string(Path::new("/iron/profiles/developer/profile.toml"))
            .unwrap();
        let parsed: Profile = toml::from_str(&content).unwrap();

        assert_eq!(parsed.id, "developer");
        assert!(parsed.modules.contains(&"nvim-ide".to_string()));
    }
}
