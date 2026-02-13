//! CLI Output Validation Tests
//!
//! Comprehensive tests for validating CLI output structure:
//! - JSON output structure validation
//! - Table/text format validation
//! - Error message format validation
//! - Verbose/quiet mode behavior

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

/// Create a test Iron directory with proper structure
fn create_test_iron_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("modules")).unwrap();
    fs::create_dir_all(root.join("profiles")).unwrap();
    fs::create_dir_all(root.join("bundles")).unwrap();
    fs::create_dir_all(root.join("hosts")).unwrap();
    fs::create_dir_all(root.join("secrets")).unwrap();

    dir
}

/// Create an initialized Iron directory with state
fn create_initialized_iron_dir() -> TempDir {
    let dir = create_test_iron_dir();
    let root = dir.path();

    let state = serde_json::json!({
        "current_host": "test-host",
        "active_bundles": {},
        "active_profiles": {},
        "active_modules": [],
        "last_operations": [],
        "maintenance": {
            "last_update": null,
            "last_clean": null,
            "last_doctor": null,
            "last_snapshot": null,
            "last_sync": null
        }
    });
    fs::write(
        root.join("state.json"),
        serde_json::to_string_pretty(&state).unwrap(),
    )
    .unwrap();

    let host_config = r#"id = "test-host"
name = "Test Host"
installed_bundles = []

[hardware]
monitors = []
"#;
    fs::write(root.join("hosts/test-host.toml"), host_config).unwrap();

    dir
}

/// Create test bundle
fn create_test_bundle(dir: &TempDir, id: &str) {
    let bundle_dir = dir.path().join("bundles").join(id);
    fs::create_dir_all(&bundle_dir).unwrap();

    let bundle = format!(
        r#"id = "{id}"
name = "Test Bundle {id}"
description = "A test bundle"
bundle_type = "WaylandCompositor"
packages = ["pkg1", "pkg2"]
aur_packages = []
profiles = []
conflicts = []
services = []
"#
    );
    fs::write(bundle_dir.join("bundle.toml"), bundle).unwrap();
}

/// Create test module with dotfiles
fn create_test_module(dir: &TempDir, id: &str) {
    let module_dir = dir.path().join("modules").join(id);
    fs::create_dir_all(&module_dir).unwrap();

    // Create dotfile target directory within the temp dir
    let dotfile_target = dir.path().join("home").join(".config").join(id);
    fs::create_dir_all(dotfile_target.parent().unwrap()).unwrap();

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
    fs::write(module_dir.join("module.toml"), module).unwrap();
    fs::write(module_dir.join("config"), "# config content").unwrap();
}

/// Create test profile
fn create_test_profile(dir: &TempDir, id: &str) {
    let profile_dir = dir.path().join("profiles").join(id);
    fs::create_dir_all(&profile_dir).unwrap();

    let profile = format!(
        r#"id = "{id}"
name = "Test Profile {id}"
description = "A test profile"
modules = []
"#
    );
    fs::write(profile_dir.join("profile.toml"), profile).unwrap();
}

/// Get iron command with --no-color
#[allow(deprecated)]
fn iron() -> Command {
    let mut cmd = Command::cargo_bin("iron").unwrap();
    cmd.arg("--no-color");
    cmd
}

/// Try to parse JSON from CLI output (handles potential preamble)
fn try_parse_json(output: &str) -> Option<Value> {
    // Find JSON object or array
    if let Some(start) = output.find('{') {
        // Find matching closing brace
        let json_part = &output[start..];
        // Try to find a complete JSON object
        let mut depth = 0;
        let mut end_pos = 0;
        for (i, ch) in json_part.chars().enumerate() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if end_pos > 0 {
            if let Ok(v) = serde_json::from_str(&json_part[..end_pos]) {
                return Some(v);
            }
        }
    }

    // Try array
    if let Some(start) = output.find('[') {
        let json_part = &output[start..];
        let mut depth = 0;
        let mut end_pos = 0;
        for (i, ch) in json_part.chars().enumerate() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if end_pos > 0 {
            if let Ok(v) = serde_json::from_str(&json_part[..end_pos]) {
                return Some(v);
            }
        }
    }

    None
}

// =============================================================================
// JSON Structure Validation Tests
// =============================================================================

mod json_structure {
    use super::*;

    #[test]
    fn status_json_produces_valid_json() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = try_parse_json(&stdout);
        assert!(
            json.is_some(),
            "Status JSON output should be parseable: {}",
            stdout
        );
    }

    #[test]
    fn status_json_contains_host_info() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // Should contain host identifier somewhere
        assert!(
            stdout.contains("test-host"),
            "Status JSON should contain host: {}",
            stdout
        );
    }

    #[test]
    fn bundle_list_json_contains_bundle_data() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // Should contain bundle ID
        assert!(
            stdout.contains("hyprland"),
            "Bundle list JSON should contain bundle: {}",
            stdout
        );
    }

    #[test]
    fn bundle_list_json_with_multiple_bundles() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");
        create_test_bundle(&dir, "plasma");
        create_test_bundle(&dir, "gnome");

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(stdout.contains("hyprland"), "Should contain hyprland");
        assert!(stdout.contains("plasma"), "Should contain plasma");
        assert!(stdout.contains("gnome"), "Should contain gnome");
    }

    #[test]
    fn module_list_json_with_modules() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");
        create_test_module(&dir, "fish");

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("module")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(
            stdout.contains("nvim"),
            "Module list should contain nvim: {}",
            stdout
        );
        assert!(
            stdout.contains("fish"),
            "Module list should contain fish: {}",
            stdout
        );
    }

    #[test]
    fn profile_list_json_with_profiles() {
        let dir = create_initialized_iron_dir();
        create_test_profile(&dir, "dev");
        create_test_profile(&dir, "minimal");

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("profile")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(
            stdout.contains("dev"),
            "Profile list should contain dev: {}",
            stdout
        );
        assert!(
            stdout.contains("minimal"),
            "Profile list should contain minimal: {}",
            stdout
        );
    }

    #[test]
    fn doctor_json_produces_output() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("doctor")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // Doctor should produce some output
        assert!(!stdout.trim().is_empty(), "Doctor should produce output");
    }

    /// Test FR-10.8: Verify doctor JSON has all required health check fields
    #[test]
    fn doctor_json_has_required_structure() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("doctor")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json: Value = serde_json::from_str(&stdout).expect("Doctor output should be valid JSON");

        // Verify top-level structure
        assert!(json.get("checks").is_some(), "Missing 'checks' array");
        assert!(json.get("overall").is_some(), "Missing 'overall' status");
        assert!(json.get("timestamp").is_some(), "Missing 'timestamp'");

        // Verify checks array structure
        let checks = json["checks"].as_array().expect("'checks' should be an array");
        assert!(!checks.is_empty(), "Should have at least one health check");

        // Each check should have name, status, and message
        for check in checks {
            assert!(check.get("name").is_some(), "Check missing 'name'");
            assert!(check.get("status").is_some(), "Check missing 'status'");
            assert!(check.get("message").is_some(), "Check missing 'message'");

            // Status should be one of pass, warn, fail
            let status = check["status"].as_str().unwrap();
            assert!(
                ["pass", "warn", "fail"].contains(&status),
                "Invalid status: {}",
                status
            );
        }

        // Verify overall status is valid
        let overall = json["overall"].as_str().expect("'overall' should be a string");
        assert!(
            ["pass", "warn", "fail"].contains(&overall),
            "Invalid overall status: {}",
            overall
        );
    }

    /// Test FR-10.1 through FR-10.7: Verify all required health checks are present
    #[test]
    fn doctor_json_contains_all_fr10_checks() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("doctor")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json: Value = serde_json::from_str(&stdout).expect("Doctor output should be valid JSON");

        let checks = json["checks"].as_array().expect("'checks' should be an array");
        let check_names: Vec<&str> = checks
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();

        // FR-10.1: State file validation
        assert!(
            check_names.contains(&"state_file"),
            "Missing FR-10.1 state_file check. Got: {:?}",
            check_names
        );

        // FR-10.2: Symlink integrity
        assert!(
            check_names.contains(&"symlinks"),
            "Missing FR-10.2 symlinks check. Got: {:?}",
            check_names
        );

        // FR-10.3: Package installation
        assert!(
            check_names.contains(&"packages"),
            "Missing FR-10.3 packages check. Got: {:?}",
            check_names
        );

        // FR-10.4: Snapshot backend
        assert!(
            check_names.contains(&"snapshot"),
            "Missing FR-10.4 snapshot check. Got: {:?}",
            check_names
        );

        // FR-10.5: Config directories
        assert!(
            check_names.contains(&"directories"),
            "Missing FR-10.5 directories check. Got: {:?}",
            check_names
        );

        // FR-10.6: Git repository
        assert!(
            check_names.contains(&"git"),
            "Missing FR-10.6 git check. Got: {:?}",
            check_names
        );

        // FR-10.7: Secrets status
        assert!(
            check_names.contains(&"secrets"),
            "Missing FR-10.7 secrets check. Got: {:?}",
            check_names
        );

        // Additional checks
        assert!(
            check_names.contains(&"current_host"),
            "Missing current_host check. Got: {:?}",
            check_names
        );
        assert!(
            check_names.contains(&"tools"),
            "Missing tools check. Got: {:?}",
            check_names
        );
    }

    #[test]
    fn host_list_json_produces_output() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("host")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(
            stdout.contains("test-host"),
            "Host list should contain test-host: {}",
            stdout
        );
    }
}

// =============================================================================
// Table/Text Format Validation Tests
// =============================================================================

mod text_format {
    use super::*;

    #[test]
    fn status_text_contains_host() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn bundle_list_text_shows_bundles() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");
        create_test_bundle(&dir, "plasma");

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("hyprland"))
            .stdout(predicate::str::contains("plasma"));
    }

    #[test]
    fn module_list_text_shows_modules() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");
        create_test_module(&dir, "fish");

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("nvim"))
            .stdout(predicate::str::contains("fish"));
    }

    #[test]
    fn profile_list_text_shows_profiles() {
        let dir = create_initialized_iron_dir();
        create_test_profile(&dir, "dev");
        create_test_profile(&dir, "minimal");

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("profile")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("dev"))
            .stdout(predicate::str::contains("minimal"));
    }

    #[test]
    fn doctor_text_runs_health_check() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("Health")
                    .or(predicate::str::contains("Check"))
                    .or(predicate::str::contains("OK"))
                    .or(predicate::str::contains("modules")),
            );
    }

    #[test]
    fn empty_bundle_list_handled() {
        let dir = create_initialized_iron_dir();

        // No bundles created
        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success();
        // Should not crash, output can vary
    }

    #[test]
    fn empty_module_list_handled() {
        let dir = create_initialized_iron_dir();

        // No modules created
        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("list")
            .assert()
            .success();
        // Should not crash
    }
}

// =============================================================================
// Verbose/Quiet Mode Tests
// =============================================================================

mod output_modes {
    use super::*;

    #[test]
    fn verbose_flag_accepted() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--verbose")
            .arg("status")
            .assert()
            .success();
    }

    #[test]
    fn quiet_flag_accepted() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--quiet")
            .arg("status")
            .assert()
            .success();
    }

    #[test]
    fn verbose_produces_output() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--verbose")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(!stdout.trim().is_empty(), "Verbose should produce output");
    }

    #[test]
    fn quiet_still_works() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--quiet")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();
    }

    #[test]
    fn minimal_format_accepted() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("minimal")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();
    }
}

// =============================================================================
// Error Message Format Tests
// =============================================================================

mod error_format {
    use super::*;

    #[test]
    fn uninitialized_error_message() {
        let dir = create_test_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("init")
                    .or(predicate::str::contains("initialize"))
                    .or(predicate::str::contains("not initialized")),
            );
    }

    #[test]
    fn invalid_command_error() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("invalid-command-xyz")
            .assert()
            .failure();
    }

    #[test]
    fn bundle_show_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("show")
            .arg("nonexistent-bundle")
            .assert()
            .failure();
    }

    #[test]
    fn module_show_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("show")
            .arg("nonexistent-module")
            .assert()
            .failure();
    }

    #[test]
    fn error_no_panic() {
        let dir = create_initialized_iron_dir();
        let long_name = "x".repeat(1000);

        // Long names should error gracefully, not panic
        let result = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("show")
            .arg(&long_name)
            .assert();

        let stderr = String::from_utf8_lossy(&result.get_output().stderr);
        assert!(
            !stderr.to_lowercase().contains("panic"),
            "Should not panic: {}",
            stderr
        );
    }

    #[test]
    fn missing_arg_shows_usage() {
        let dir = create_initialized_iron_dir();

        iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("install")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("Usage")
                    .or(predicate::str::contains("usage"))
                    .or(predicate::str::contains("required"))
                    .or(predicate::str::contains("argument")),
            );
    }
}

// =============================================================================
// Output Consistency Tests
// =============================================================================

mod consistency {
    use super::*;

    #[test]
    fn repeated_calls_deterministic() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        let output1 = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let output2 = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
        let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);

        assert_eq!(stdout1, stdout2, "Repeated calls should be deterministic");
    }

    #[test]
    fn json_and_text_both_contain_data() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        let text_output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let json_output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("bundle")
            .arg("list")
            .assert()
            .success();

        let text = String::from_utf8_lossy(&text_output.get_output().stdout);
        let json = String::from_utf8_lossy(&json_output.get_output().stdout);

        // Both should mention the bundle
        assert!(text.contains("hyprland"), "Text output should show bundle");
        assert!(json.contains("hyprland"), "JSON output should show bundle");
    }

    #[test]
    fn no_color_removes_ansi() {
        let dir = create_initialized_iron_dir();

        let output = iron()
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(
            !stdout.contains("\x1b["),
            "No-color should not have ANSI escapes"
        );
    }
}
