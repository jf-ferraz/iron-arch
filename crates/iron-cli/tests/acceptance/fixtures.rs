//! Acceptance test fixtures and helpers
//!
//! This module provides common test infrastructure for acceptance tests.
//! Each acceptance test file (at1_*.rs through at6_*.rs) tests a specific
//! user story or workflow.

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture providing isolated iron environment
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub iron_root: PathBuf,
}

impl TestFixture {
    /// Create a new empty fixture
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let iron_root = temp_dir.path().to_path_buf();
        Self { temp_dir, iron_root }
    }

    /// Create fixture with initialized iron state
    pub fn with_initialized_state() -> Self {
        let fixture = Self::new();
        fixture
            .run_iron(&["init", "--id", "test-host", "--name", "Test Host"])
            .success();
        fixture
    }

    /// Run iron command with fixture's root directory
    pub fn run_iron(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        Command::cargo_bin("iron")
            .expect("iron binary not found")
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .assert()
    }

    /// Run iron command and return JSON parsed output
    pub fn run_iron_json(&self, args: &[&str]) -> serde_json::Value {
        let output = Command::cargo_bin("iron")
            .expect("iron binary not found")
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .arg("--format")
            .arg("json")
            .output()
            .expect("Failed to execute iron");

        serde_json::from_slice(&output.stdout).expect("Failed to parse JSON output")
    }

    /// Create a test bundle in the fixture's bundles directory
    pub fn create_bundle(&self, id: &str) {
        let bundle_dir = self.iron_root.join("bundles").join(id);
        std::fs::create_dir_all(&bundle_dir).expect("Failed to create bundle dir");

        let bundle = format!(
            r#"id = "{id}"
name = "Test Bundle {id}"
description = "A test bundle"
bundle_type = "WaylandCompositor"
packages = ["hyprland"]
aur_packages = []
profiles = []
conflicts = []
services = []
"#
        );
        std::fs::write(bundle_dir.join("bundle.toml"), bundle)
            .expect("Failed to write bundle.toml");
    }

    /// Create a test profile in the fixture's profiles directory
    pub fn create_profile(&self, id: &str) {
        let profile_dir = self.iron_root.join("profiles").join(id);
        std::fs::create_dir_all(&profile_dir).expect("Failed to create profile dir");

        let profile = format!(
            r#"id = "{id}"
name = "Test Profile {id}"
description = "A test profile"
modules = []
"#
        );
        std::fs::write(profile_dir.join("profile.toml"), profile)
            .expect("Failed to write profile.toml");
    }

    /// Create a test module in the fixture's modules directory
    pub fn create_module(&self, id: &str) {
        let module_dir = self.iron_root.join("modules").join(id);
        std::fs::create_dir_all(&module_dir).expect("Failed to create module dir");

        // Create dotfile target directory within the temp dir
        let dotfile_target = self.iron_root.join("home").join(".config").join(id);
        std::fs::create_dir_all(dotfile_target.parent().unwrap())
            .expect("Failed to create dotfile target parent");

        let module = format!(
            r#"id = "{id}"
name = "Test Module {id}"
description = "A test module"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []

[[dotfiles]]
source = "config"
target = "{target}"
link = true
"#,
            target = dotfile_target.display()
        );
        std::fs::write(module_dir.join("module.toml"), module)
            .expect("Failed to write module.toml");
        std::fs::write(module_dir.join("config"), "# config content")
            .expect("Failed to write config file");
    }

    /// Get path to a file within the iron root
    pub fn path(&self, relative: &str) -> PathBuf {
        self.iron_root.join(relative)
    }

    /// Check if a file exists within the iron root
    pub fn file_exists(&self, relative: &str) -> bool {
        self.path(relative).exists()
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}
