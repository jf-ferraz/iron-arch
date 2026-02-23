//! Integration tests for TOML parsing
//!
//! These tests verify that module, bundle, and profile TOML files
//! parse correctly, including edge cases and error handling.

use iron_core::bundle::Bundle;
use iron_core::module::Module;
use iron_core::profile::Profile;
use std::path::Path;

/// Path to test fixtures
const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Path to actual Iron modules directory
const MODULES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../modules");

/// Path to actual Iron bundles directory
const BUNDLES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bundles");

/// Path to actual Iron profiles directory
const PROFILES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../profiles");

// ============================================================================
// Module Parsing Tests
// ============================================================================

mod module_parsing {
    use super::*;

    #[test]
    fn test_parse_valid_module_fixture() {
        let path = Path::new(FIXTURES_DIR).join("modules/valid-module");
        let module = Module::load(&path).expect("Should parse valid module");

        assert_eq!(module.id, "valid-module");
        assert_eq!(module.name, "Valid Module");
        assert_eq!(
            module.description,
            Some("A test module with proper TOML ordering".to_string())
        );
        assert_eq!(module.packages, vec!["pkg1", "pkg2"]);
        assert_eq!(module.aur_packages, vec!["aur-pkg1"]);
        assert_eq!(module.conflicts, vec!["other-module"]);
        assert_eq!(module.depends, vec!["base-module"]);
        assert_eq!(module.pre_install, Some("hooks/pre.sh".to_string()));
        assert_eq!(module.post_install, Some("hooks/post.sh".to_string()));
        assert_eq!(module.dotfiles.len(), 2);
        assert_eq!(module.dotfiles[0].source, "config");
        assert_eq!(module.dotfiles[0].target, "~/.config/test");
        assert!(module.dotfiles[0].link);
        assert!(!module.dotfiles[1].link);
    }

    #[test]
    fn test_parse_invalid_ordering_fails() {
        // This demonstrates the TOML ordering bug that was fixed
        let path = Path::new(FIXTURES_DIR).join("modules/invalid-ordering");
        let result = Module::load(&path);

        // Should fail because 'conflicts' appears after [[dotfiles]]
        // and TOML interprets it as part of the dotfiles table
        assert!(
            result.is_err(),
            "Module with invalid TOML ordering should fail to parse"
        );

        if let Err(e) = result {
            let error_msg = e.to_string();
            // The error should mention missing required fields
            assert!(
                error_msg.contains("missing field") || error_msg.contains("unknown field"),
                "Error should indicate field parsing issue: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_parse_all_actual_modules() {
        let modules_path = Path::new(MODULES_DIR);
        if !modules_path.exists() {
            eprintln!("Modules directory not found, skipping actual module tests");
            return;
        }

        let entries = std::fs::read_dir(modules_path).expect("Should read modules directory");
        let mut parsed_count = 0;
        let mut failed = Vec::new();

        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let module_path = entry.path();
                let module_name = module_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                // Skip hidden directories (e.g., .example)
                if module_name.starts_with('.') {
                    continue;
                }

                match Module::load(&module_path) {
                    Ok(module) => {
                        // Verify essential fields
                        assert!(
                            !module.id.is_empty(),
                            "Module {} should have non-empty id",
                            module_name
                        );
                        assert!(
                            !module.name.is_empty(),
                            "Module {} should have non-empty name",
                            module_name
                        );
                        parsed_count += 1;
                    }
                    Err(e) => {
                        failed.push((module_name.to_string(), e.to_string()));
                    }
                }
            }
        }

        // Report failures
        if !failed.is_empty() {
            for (name, error) in &failed {
                eprintln!("Failed to parse module '{}': {}", name, error);
            }
            panic!(
                "{} modules failed to parse out of {} attempted",
                failed.len(),
                parsed_count + failed.len()
            );
        }

        assert!(
            parsed_count > 0,
            "Should have parsed at least one actual module"
        );
        println!("Successfully parsed {} modules", parsed_count);
    }

    #[test]
    fn test_module_nvim_ide() {
        let path = Path::new(MODULES_DIR).join("nvim-ide");
        if !path.exists() {
            return;
        }

        let module = Module::load(&path).expect("nvim-ide should parse correctly");

        assert_eq!(module.id, "nvim-ide");
        assert!(!module.packages.is_empty(), "nvim-ide should have packages");
        assert!(
            module.packages.contains(&"neovim".to_string()),
            "nvim-ide should include neovim package"
        );
        assert_eq!(module.conflicts, vec!["vim-minimal"]);
        assert!(
            module.post_install.is_some(),
            "nvim-ide should have post_install hook"
        );
    }

    #[test]
    fn test_module_kitty_dev() {
        let path = Path::new(MODULES_DIR).join("kitty-dev");
        if !path.exists() {
            return;
        }

        let module = Module::load(&path).expect("kitty-dev should parse correctly");

        assert_eq!(module.id, "kitty-dev");
        assert!(
            module.packages.contains(&"kitty".to_string()),
            "kitty-dev should include kitty package"
        );
    }

    #[test]
    fn test_module_dev_tools() {
        let path = Path::new(MODULES_DIR).join("dev-tools");
        if !path.exists() {
            return;
        }

        let module = Module::load(&path).expect("dev-tools should parse correctly");

        assert_eq!(module.id, "dev-tools");
        // dev-tools is packages-only, should have empty dotfiles
        assert!(
            module.dotfiles.is_empty(),
            "dev-tools should have empty dotfiles array"
        );
        assert!(
            !module.packages.is_empty(),
            "dev-tools should have packages"
        );
    }
}

// ============================================================================
// Bundle Parsing Tests
// ============================================================================

mod bundle_parsing {
    use super::*;

    #[test]
    fn test_parse_valid_bundle_fixture() {
        let path = Path::new(FIXTURES_DIR).join("bundles/test-bundle");
        let bundle = Bundle::load(&path).expect("Should parse valid bundle");

        assert_eq!(bundle.id, "test-bundle");
        assert_eq!(bundle.name, "Test Bundle");
        assert_eq!(bundle.packages, vec!["compositor", "status-bar"]);
        assert_eq!(bundle.profiles, vec!["minimal", "developer"]);
        assert_eq!(bundle.default_profile, Some("minimal".to_string()));
        assert_eq!(bundle.conflicts, vec!["other-bundle"]);
        assert_eq!(bundle.services, vec!["pipewire", "wireplumber"]);
    }

    #[test]
    fn test_parse_all_actual_bundles() {
        let bundles_path = Path::new(BUNDLES_DIR);
        if !bundles_path.exists() {
            eprintln!("Bundles directory not found, skipping actual bundle tests");
            return;
        }

        let entries = std::fs::read_dir(bundles_path).expect("Should read bundles directory");
        let mut parsed_count = 0;
        let mut failed = Vec::new();

        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let bundle_path = entry.path();
                let bundle_name = bundle_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                // Skip hidden directories (e.g., .example)
                if bundle_name.starts_with('.') {
                    continue;
                }

                match Bundle::load(&bundle_path) {
                    Ok(bundle) => {
                        assert!(
                            !bundle.id.is_empty(),
                            "Bundle {} should have non-empty id",
                            bundle_name
                        );
                        assert!(
                            !bundle.name.is_empty(),
                            "Bundle {} should have non-empty name",
                            bundle_name
                        );
                        parsed_count += 1;
                    }
                    Err(e) => {
                        failed.push((bundle_name.to_string(), e.to_string()));
                    }
                }
            }
        }

        if !failed.is_empty() {
            for (name, error) in &failed {
                eprintln!("Failed to parse bundle '{}': {}", name, error);
            }
            panic!(
                "{} bundles failed to parse out of {} attempted",
                failed.len(),
                parsed_count + failed.len()
            );
        }

        assert!(
            parsed_count > 0,
            "Should have parsed at least one actual bundle"
        );
        println!("Successfully parsed {} bundles", parsed_count);
    }

    #[test]
    fn test_bundle_hyprland() {
        let path = Path::new(BUNDLES_DIR).join("hyprland");
        if !path.exists() {
            return;
        }

        let bundle = Bundle::load(&path).expect("hyprland bundle should parse correctly");

        assert_eq!(bundle.id, "hyprland");
        assert!(
            bundle.packages.contains(&"hyprland".to_string()),
            "hyprland bundle should include hyprland package"
        );
        assert!(
            bundle.conflicts.contains(&"niri".to_string()),
            "hyprland should conflict with niri"
        );
        assert!(
            !bundle.profiles.is_empty(),
            "hyprland should have profiles defined"
        );
    }

    #[test]
    fn test_bundle_niri() {
        let path = Path::new(BUNDLES_DIR).join("niri");
        if !path.exists() {
            return;
        }

        let bundle = Bundle::load(&path).expect("niri bundle should parse correctly");

        assert_eq!(bundle.id, "niri");
        assert!(
            bundle.conflicts.contains(&"hyprland".to_string()),
            "niri should conflict with hyprland"
        );
    }
}

// ============================================================================
// Profile Parsing Tests
// ============================================================================

mod profile_parsing {
    use super::*;

    #[test]
    fn test_parse_valid_profile_fixture() {
        let path = Path::new(FIXTURES_DIR).join("profiles/test-profile");
        let profile = Profile::load(&path).expect("Should parse valid profile");

        assert_eq!(profile.id, "test-profile");
        assert_eq!(profile.name, "Test Profile");
        assert_eq!(profile.modules, vec!["module-a", "module-b", "module-c"]);
        assert_eq!(profile.theme, Some("test-theme".to_string()));
        assert_eq!(profile.shell, Some("zsh".to_string()));
        assert_eq!(profile.for_bundle, Some("test-bundle".to_string()));
    }

    #[test]
    fn test_parse_minimal_profile() {
        let path = Path::new(FIXTURES_DIR).join("profiles/minimal-profile");
        let profile = Profile::load(&path).expect("Should parse minimal profile");

        assert_eq!(profile.id, "minimal-profile");
        assert_eq!(profile.name, "Minimal");
        assert!(profile.modules.is_empty());
        // Optional fields should be None
        assert!(profile.description.is_none());
        assert!(profile.theme.is_none());
        assert!(profile.shell.is_none());
        assert!(profile.extends.is_none());
        assert!(profile.for_bundle.is_none());
    }

    #[test]
    fn test_parse_all_actual_profiles() {
        let profiles_path = Path::new(PROFILES_DIR);
        if !profiles_path.exists() {
            eprintln!("Profiles directory not found, skipping actual profile tests");
            return;
        }

        let entries = std::fs::read_dir(profiles_path).expect("Should read profiles directory");
        let mut parsed_count = 0;
        let mut failed = Vec::new();

        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let profile_path = entry.path();
                let profile_name = profile_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                // Skip hidden directories (e.g., .example)
                if profile_name.starts_with('.') {
                    continue;
                }

                match Profile::load(&profile_path) {
                    Ok(profile) => {
                        assert!(
                            !profile.id.is_empty(),
                            "Profile {} should have non-empty id",
                            profile_name
                        );
                        assert!(
                            !profile.name.is_empty(),
                            "Profile {} should have non-empty name",
                            profile_name
                        );
                        parsed_count += 1;
                    }
                    Err(e) => {
                        failed.push((profile_name.to_string(), e.to_string()));
                    }
                }
            }
        }

        if !failed.is_empty() {
            for (name, error) in &failed {
                eprintln!("Failed to parse profile '{}': {}", name, error);
            }
            panic!(
                "{} profiles failed to parse out of {} attempted",
                failed.len(),
                parsed_count + failed.len()
            );
        }

        assert!(
            parsed_count > 0,
            "Should have parsed at least one actual profile"
        );
        println!("Successfully parsed {} profiles", parsed_count);
    }

    #[test]
    fn test_profile_developer() {
        let path = Path::new(PROFILES_DIR).join("developer");
        if !path.exists() {
            return;
        }

        let profile = Profile::load(&path).expect("developer profile should parse correctly");

        assert_eq!(profile.id, "developer");
        assert!(
            !profile.modules.is_empty(),
            "developer profile should have modules"
        );
        assert!(
            profile.modules.contains(&"nvim-ide".to_string()),
            "developer profile should include nvim-ide"
        );
    }

    #[test]
    fn test_profile_minimal() {
        let path = Path::new(PROFILES_DIR).join("minimal");
        if !path.exists() {
            return;
        }

        let profile = Profile::load(&path).expect("minimal profile should parse correctly");

        assert_eq!(profile.id, "minimal");
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_module_not_found() {
        let path = Path::new("/nonexistent/path/to/module");
        let result = Module::load(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let temp = TempDir::new().unwrap();
        let module_dir = temp.path().join("broken-module");
        fs::create_dir_all(&module_dir).unwrap();

        // Write invalid TOML
        fs::write(
            module_dir.join("module.toml"),
            "id = \"test\"\nname = \"Test\ninvalid syntax here",
        )
        .unwrap();

        let result = Module::load(&module_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_field() {
        let temp = TempDir::new().unwrap();
        let module_dir = temp.path().join("incomplete-module");
        fs::create_dir_all(&module_dir).unwrap();

        // Write TOML missing required 'id' field
        fs::write(
            module_dir.join("module.toml"),
            r#"
name = "Test Module"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []
dotfiles = []
"#,
        )
        .unwrap();

        let result = Module::load(&module_dir);
        assert!(result.is_err(), "Should fail when id is missing");

        if let Err(e) = result {
            assert!(
                e.to_string().contains("missing field"),
                "Error should mention missing field: {}",
                e
            );
        }
    }

    #[test]
    fn test_invalid_enum_value() {
        let temp = TempDir::new().unwrap();
        let module_dir = temp.path().join("bad-enum-module");
        fs::create_dir_all(&module_dir).unwrap();

        fs::write(
            module_dir.join("module.toml"),
            r#"
id = "test"
name = "Test Module"
kind = "InvalidKind"  # Not a valid ModuleKind
packages = []
aur_packages = []
conflicts = []
depends = []
dotfiles = []
"#,
        )
        .unwrap();

        let result = Module::load(&module_dir);
        assert!(result.is_err(), "Should fail with invalid enum value");
    }

    #[test]
    fn test_bundle_missing_required_fields() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("incomplete-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        // Missing bundle_type
        fs::write(
            bundle_dir.join("bundle.toml"),
            r#"
id = "test"
name = "Test"
packages = []
aur_packages = []
profiles = []
conflicts = []
services = []
"#,
        )
        .unwrap();

        let result = Bundle::load(&bundle_dir);
        assert!(result.is_err(), "Should fail when bundle_type is missing");
    }

    #[test]
    fn test_profile_missing_modules() {
        let temp = TempDir::new().unwrap();
        let profile_dir = temp.path().join("incomplete-profile");
        fs::create_dir_all(&profile_dir).unwrap();

        // Missing modules field
        fs::write(
            profile_dir.join("profile.toml"),
            r#"
id = "test"
name = "Test"
"#,
        )
        .unwrap();

        let result = Profile::load(&profile_dir);
        assert!(result.is_err(), "Should fail when modules is missing");
    }
}

// ============================================================================
// Serialization Round-Trip Tests
// ============================================================================

mod roundtrip {
    use super::*;
    use iron_core::module::{DotfileMapping, ModuleKind};
    use tempfile::TempDir;

    #[test]
    fn test_module_roundtrip() {
        let temp = TempDir::new().unwrap();
        let module_dir = temp.path().join("roundtrip-module");

        let original = Module {
            id: "roundtrip-test".to_string(),
            name: "Roundtrip Test".to_string(),
            description: Some("Testing serialization roundtrip".to_string()),
            kind: ModuleKind::DevTools,
            packages: vec!["pkg1".to_string(), "pkg2".to_string()],
            aur_packages: vec!["aur1".to_string()],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "~/.config/test".to_string(),
                link: true,
            }],
            conflicts: vec!["other".to_string()],
            depends: vec!["dep1".to_string()],
            pre_install: Some("pre.sh".to_string()),
            post_install: Some("post.sh".to_string()),
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
            security_points: 0,
            hook_behavior: iron_core::module::HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
        };

        // Save
        original.save(&module_dir).expect("Should save module");

        // Load
        let loaded = Module::load(&module_dir).expect("Should load saved module");

        // Verify
        assert_eq!(original.id, loaded.id);
        assert_eq!(original.name, loaded.name);
        assert_eq!(original.description, loaded.description);
        assert_eq!(original.packages, loaded.packages);
        assert_eq!(original.aur_packages, loaded.aur_packages);
        assert_eq!(original.conflicts, loaded.conflicts);
        assert_eq!(original.depends, loaded.depends);
        assert_eq!(original.pre_install, loaded.pre_install);
        assert_eq!(original.post_install, loaded.post_install);
        assert_eq!(original.dotfiles.len(), loaded.dotfiles.len());
    }

    #[test]
    fn test_bundle_roundtrip() {
        let temp = TempDir::new().unwrap();
        let bundle_dir = temp.path().join("roundtrip-bundle");

        let original = Bundle {
            id: "roundtrip-bundle".to_string(),
            name: "Roundtrip Bundle".to_string(),
            description: Some("Testing serialization".to_string()),
            bundle_type: iron_core::bundle::BundleType::WaylandCompositor,
            packages: vec!["pkg1".to_string()],
            aur_packages: vec![],
            profiles: vec!["profile1".to_string()],
            default_profile: Some("profile1".to_string()),
            conflicts: vec!["other-bundle".to_string()],
            services: vec!["service1".to_string()],
            post_install: Some("setup.sh".to_string()),
        };

        original.save(&bundle_dir).expect("Should save bundle");
        let loaded = Bundle::load(&bundle_dir).expect("Should load saved bundle");

        assert_eq!(original.id, loaded.id);
        assert_eq!(original.name, loaded.name);
        assert_eq!(original.packages, loaded.packages);
        assert_eq!(original.conflicts, loaded.conflicts);
    }

    #[test]
    fn test_profile_roundtrip() {
        let temp = TempDir::new().unwrap();
        let profile_dir = temp.path().join("roundtrip-profile");

        let original = Profile {
            id: "roundtrip-profile".to_string(),
            name: "Roundtrip Profile".to_string(),
            description: Some("Testing serialization".to_string()),
            modules: vec!["mod1".to_string(), "mod2".to_string()],
            theme: Some("theme1".to_string()),
            shell: Some("zsh".to_string()),
            extends: None,
            for_bundle: Some("bundle1".to_string()),
        };

        original.save(&profile_dir).expect("Should save profile");
        let loaded = Profile::load(&profile_dir).expect("Should load saved profile");

        assert_eq!(original.id, loaded.id);
        assert_eq!(original.modules, loaded.modules);
        assert_eq!(original.theme, loaded.theme);
        assert_eq!(original.shell, loaded.shell);
    }
}
