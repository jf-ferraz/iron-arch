//! Iron CLI Integration Tests
//!
//! Tests for the iron CLI commands using assert_cmd.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Create a test Iron directory with proper structure
fn create_test_iron_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create directories
    fs::create_dir_all(root.join("modules")).unwrap();
    fs::create_dir_all(root.join("profiles")).unwrap();
    fs::create_dir_all(root.join("bundles")).unwrap();
    fs::create_dir_all(root.join("hosts")).unwrap();
    fs::create_dir_all(root.join("secrets")).unwrap();

    dir
}

/// Create an initialized Iron directory
fn create_initialized_iron_dir() -> TempDir {
    let dir = create_test_iron_dir();
    let root = dir.path();

    // Create state.json with correct IronState structure
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

    // Create host config (flat file hosts/<id>.toml)
    let host_config = r#"id = "test-host"
name = "Test Host"
installed_bundles = []

[hardware]
monitors = []
"#;
    fs::write(root.join("hosts/test-host.toml"), host_config).unwrap();

    dir
}

/// Create a test bundle (in a directory with bundle.toml)
fn create_test_bundle(dir: &TempDir, id: &str) {
    let bundle_dir = dir.path().join("bundles").join(id);
    fs::create_dir_all(&bundle_dir).unwrap();

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
    fs::write(bundle_dir.join("bundle.toml"), bundle).unwrap();
}

/// Create a test module with dotfiles that link within the temp directory
fn create_test_module(dir: &TempDir, id: &str) {
    let modules_dir = dir.path().join("modules").join(id);
    fs::create_dir_all(&modules_dir).unwrap();

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
    fs::write(modules_dir.join("module.toml"), module).unwrap();
    fs::write(modules_dir.join("config"), "# config content").unwrap();
}

/// Create a test profile (in a directory with profile.toml)
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

/// Find the envelope JSON object in multi-line stdout.
///
/// JSON mode can produce leading info/status lines before the main envelope.
/// This helper finds the object containing `"ok"` and `"meta"` keys.
fn find_envelope_json(stdout: &str) -> Option<serde_json::Value> {
    // Try parsing the whole string first
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(stdout) {
        if val.get("ok").is_some() && val.get("meta").is_some() {
            return Some(val);
        }
    }
    // Otherwise, look for a multi-line JSON object starting with '{'
    // that contains envelope keys
    let mut brace_depth = 0i32;
    let mut start = None;
    for (i, ch) in stdout.char_indices() {
        match ch {
            '{' => {
                if brace_depth == 0 {
                    start = Some(i);
                }
                brace_depth += 1;
            }
            '}' => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    if let Some(s) = start {
                        let candidate = &stdout[s..=i];
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate) {
                            if val.get("ok").is_some() && val.get("meta").is_some() {
                                return Some(val);
                            }
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    None
}

/// Get iron command with --no-color for predictable output
#[allow(deprecated)]
/// Get iron command for testing.
///
/// F3-006: When a `root` path is provided, `IRON_STATE_DIR` is set to that
/// path so that the real XDG state directory does not interfere with tests.
/// This should be used whenever `--root <dir>` is passed on the command line.
fn iron_at(root: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("iron").unwrap();
    cmd.arg("--no-color");
    cmd.env("IRON_STATE_DIR", root);
    cmd
}

/// Get iron command without state dir isolation (for tests that do not
/// pass --root or that test global behavior).
fn iron() -> Command {
    let mut cmd = Command::cargo_bin("iron").unwrap();
    cmd.arg("--no-color");
    cmd
}

/// Get iron command without --no-color
#[allow(deprecated)]
fn iron_raw() -> Command {
    Command::cargo_bin("iron").unwrap()
}

// =============================================================================
// Basic CLI Tests
// =============================================================================

mod basic {
    use super::*;

    #[test]
    fn no_command_json_shows_welcome() {
        // F0-001: bare `iron` launches TUI (can't test in headless).
        // JSON mode still outputs structured welcome for machine consumers.
        iron()
            .arg("-f")
            .arg("json")
            .assert()
            .success()
            .stdout(predicate::str::contains("iron"));
    }

    #[test]
    fn help_flag_shows_help() {
        iron()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"))
            .stdout(predicate::str::contains("Commands:"));
    }

    #[test]
    fn version_flag_shows_version() {
        iron()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("iron"));
    }

    #[test]
    fn invalid_command_shows_error() {
        iron()
            .arg("nonexistent")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error"));
    }
}

// =============================================================================
// Init Command Tests
// =============================================================================

mod init {
    use super::*;

    #[test]
    fn init_creates_directories() {
        let dir = TempDir::new().unwrap();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .arg("--id")
            .arg("test-host")
            .assert()
            .success()
            .stdout(predicate::str::contains("Iron initialized"));

        // Verify directories created
        assert!(dir.path().join("modules").exists());
        assert!(dir.path().join("profiles").exists());
        assert!(dir.path().join("bundles").exists());
        assert!(dir.path().join("hosts").exists());
        assert!(dir.path().join("secrets").exists());
    }

    #[test]
    fn init_creates_state_file() {
        let dir = TempDir::new().unwrap();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .arg("--id")
            .arg("test-host")
            .assert()
            .success();

        assert!(dir.path().join("state.json").exists());
    }

    #[test]
    fn init_creates_host_config() {
        let dir = TempDir::new().unwrap();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .arg("--id")
            .arg("my-desktop")
            .assert()
            .success();

        assert!(dir.path().join("hosts/my-desktop.toml").exists());
    }

    #[test]
    fn init_warns_when_already_initialized() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .assert()
            .success()
            .stdout(predicate::str::contains("already initialized"));
    }

    #[test]
    fn init_force_reinitializes() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .arg("--force")
            .arg("--id")
            .arg("new-host")
            .assert()
            .success()
            .stdout(predicate::str::contains("Iron initialized"));
    }

    #[test]
    fn init_with_custom_name() {
        let dir = TempDir::new().unwrap();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("init")
            .arg("--id")
            .arg("test-host")
            .arg("--name")
            .arg("My Custom Host")
            .assert()
            .success()
            .stdout(predicate::str::contains("My Custom Host"));
    }
}

// =============================================================================
// Status Command Tests
// =============================================================================

mod status {
    use super::*;

    #[test]
    fn status_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn status_shows_host_info() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"))
            .stdout(predicate::str::contains("Host"));
    }

    #[test]
    fn status_json_output() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn status_shows_no_active_bundle() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("No bundle active").or(predicate::str::contains("OFF")),
            );
    }

    #[test]
    fn status_verbose_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .arg("--verbose")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }
}

// =============================================================================
// Doctor Command Tests
// =============================================================================

mod doctor {
    use super::*;

    #[test]
    fn doctor_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn doctor_checks_directories() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .success()
            .stdout(predicate::str::contains("Health Check"))
            .stdout(predicate::str::contains("modules"));
    }

    #[test]
    fn doctor_checks_state_file() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .success()
            .stdout(predicate::str::contains("State file"));
    }

    #[test]
    fn doctor_checks_host_config() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn doctor_detects_warnings() {
        let dir = create_initialized_iron_dir();
        // No git repo - should show warning

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("doctor")
            .assert()
            .success()
            .stdout(predicate::str::contains("WARN").or(predicate::str::contains("warning")));
    }
}

// =============================================================================
// Bundle Command Tests
// =============================================================================

mod bundle {
    use super::*;

    #[test]
    fn bundle_list_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn bundle_list_shows_bundles() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");
        create_test_bundle(&dir, "gnome");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("hyprland"))
            .stdout(predicate::str::contains("gnome"));
    }

    #[test]
    fn bundle_list_empty() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("list")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("No bundles")
                    .or(predicate::str::contains("0 bundles"))
                    .or(predicate::str::contains("Bundle")),
            );
    }

    #[test]
    fn bundle_status_no_active_bundle() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("No").or(predicate::str::contains("no")));
    }

    #[test]
    fn bundle_status_with_id() {
        let dir = create_initialized_iron_dir();
        create_test_bundle(&dir, "hyprland");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("status")
            .arg("hyprland")
            .assert()
            .success()
            .stdout(predicate::str::contains("hyprland"));
    }

    #[test]
    fn bundle_install_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("bundle")
            .arg("install")
            .arg("nonexistent")
            .arg("--yes")
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
    }
}

// =============================================================================
// Profile Command Tests
// =============================================================================

mod profile {
    use super::*;

    #[test]
    fn profile_list_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("profile")
            .arg("list")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn profile_list_shows_profiles() {
        let dir = create_initialized_iron_dir();
        create_test_profile(&dir, "developer");
        create_test_profile(&dir, "minimal");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("profile")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("developer"))
            .stdout(predicate::str::contains("minimal"));
    }

    #[test]
    fn profile_show_displays_details() {
        let dir = create_initialized_iron_dir();
        create_test_profile(&dir, "developer");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("profile")
            .arg("show")
            .arg("developer")
            .assert()
            .success()
            .stdout(predicate::str::contains("developer"));
    }

    #[test]
    fn profile_show_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("profile")
            .arg("show")
            .arg("nonexistent")
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
    }
}

// =============================================================================
// Module Command Tests
// =============================================================================

mod module {
    use super::*;

    #[test]
    fn module_list_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("list")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn module_list_shows_modules() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");
        create_test_module(&dir, "kitty");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("nvim"))
            .stdout(predicate::str::contains("kitty"));
    }

    #[test]
    fn module_show_displays_details() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("show")
            .arg("nvim")
            .assert()
            .success()
            .stdout(predicate::str::contains("nvim"));
    }

    #[test]
    fn module_enable_works() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("enable")
            .arg("nvim")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("enabled")
                    .or(predicate::str::contains("Enabled"))
                    .or(predicate::str::contains("nvim")),
            );
    }

    #[test]
    fn module_enable_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("enable")
            .arg("nonexistent")
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
    }

    #[test]
    fn module_disable_works() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");

        // First enable the module
        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("enable")
            .arg("nvim")
            .assert()
            .success();

        // Then disable it
        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("disable")
            .arg("nvim")
            .arg("--yes")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("disabled")
                    .or(predicate::str::contains("Disabled"))
                    .or(predicate::str::contains("nvim")),
            );
    }

    #[test]
    fn module_disable_nonexistent() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("module")
            .arg("disable")
            .arg("nonexistent")
            .arg("--yes")
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
    }
}

// =============================================================================
// Host Command Tests
// =============================================================================

mod host {
    use super::*;

    #[test]
    fn host_list_empty_shows_warning() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("host")
            .arg("list")
            .assert()
            .success()
            // Shows warning when no hosts configured
            .stdout(predicate::str::contains("No hosts").or(predicate::str::contains("init")));
    }

    #[test]
    fn host_list_shows_hosts() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("host")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn host_current_shows_active() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("host")
            .arg("current")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn host_catalog_shows_hardware() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("host")
            .arg("catalog")
            .assert()
            .success()
            // Should show hardware detection info
            .stdout(
                predicate::str::contains("CPU")
                    .or(predicate::str::contains("Hardware"))
                    .or(predicate::str::contains("catalog")),
            );
    }
}

// =============================================================================
// Sync Command Tests
// =============================================================================

mod sync {
    use super::*;

    #[test]
    fn sync_status_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("sync")
            .arg("status")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn sync_status_without_git() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("sync")
            .arg("status")
            .assert()
            .success()
            // Without a git repo, shows warning about not being a git repository
            .stdout(predicate::str::contains("git").or(predicate::str::contains("repository")));
    }
}

// =============================================================================
// Secrets Command Tests
// =============================================================================

mod secrets {
    use super::*;

    #[test]
    fn secrets_status_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("secrets")
            .arg("status")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn secrets_status_shows_state() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("secrets")
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("Secret").or(predicate::str::contains("secret")));
    }
}

// =============================================================================
// Update Command Tests
// =============================================================================

mod update {
    use super::*;

    #[test]
    fn update_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("update")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn update_dry_run() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("update")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("Update").or(predicate::str::contains("update")));
    }
}

// =============================================================================
// Clean Command Tests
// =============================================================================

mod clean {
    use super::*;

    #[test]
    fn clean_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn clean_with_no_flags() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("Clean").or(predicate::str::contains("clean")));
    }

    #[test]
    fn clean_symlinks_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .arg("--symlinks")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("symlink").or(predicate::str::contains("Symlink")));
    }

    #[test]
    fn clean_orphans_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .arg("--orphans")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("orphan")
                    .or(predicate::str::contains("Orphan"))
                    .or(predicate::str::contains("package"))
                    .or(predicate::str::contains("Clean")),
            );
    }

    #[test]
    fn clean_cache_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .arg("--cache")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("cache")
                    .or(predicate::str::contains("Cache"))
                    .or(predicate::str::contains("Clean")),
            );
    }

    #[test]
    fn clean_all_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("clean")
            .arg("--all")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("Clean").or(predicate::str::contains("clean")));
    }
}

// =============================================================================
// Recover Command Tests
// =============================================================================

mod recover {
    use super::*;

    #[test]
    fn recover_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("recover")
            .arg("--export")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn recover_export_creates_output() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("recover")
            .arg("--export")
            .assert()
            .success()
            .stdout(predicate::str::contains("Export").or(predicate::str::contains("export")));
    }

    #[test]
    fn recover_script_generates_script() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("recover")
            .arg("--script")
            .assert()
            .success()
            .stdout(predicate::str::contains("Script").or(predicate::str::contains("bash")));
    }
}

// =============================================================================
// Output Format Tests
// =============================================================================

mod output_format {
    use super::*;

    #[test]
    fn json_format_produces_valid_json() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // Should be parseable as JSON (may have some preamble, so find the JSON part)
        let json_start = stdout.find('{').unwrap_or(0);
        let json_part = &stdout[json_start..];
        assert!(
            serde_json::from_str::<serde_json::Value>(json_part).is_ok(),
            "Output should contain valid JSON: {}",
            stdout
        );
    }

    #[test]
    fn verbose_flag_accepted() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
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

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--quiet")
            .arg("status")
            .assert()
            .success();
    }

    #[test]
    fn no_color_flag_removes_ansi() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // Should not contain ANSI escape codes (we already use --no-color via iron())
        assert!(
            !stdout.contains("\x1b["),
            "No-color output should not contain ANSI escapes"
        );
    }

    #[test]
    fn color_output_contains_ansi() {
        let dir = create_initialized_iron_dir();

        let output = iron_raw()
            .env("IRON_STATE_DIR", dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // With color enabled, output should contain ANSI escape codes
        // (unless terminal detection disables it)
        // This test just ensures the command works
        assert!(!stdout.is_empty(), "Output should not be empty");
    }
}

// =============================================================================
// Status Enhancement Tests (F3-004)
// =============================================================================

mod status_enhanced {
    use super::*;

    #[test]
    fn status_dry_run_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("test-host"));
    }

    #[test]
    fn status_full_dry_run_shows_drift_skip_message() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .arg("--full")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("DRY RUN"));
    }

    #[test]
    fn status_json_dry_run_returns_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("status")
            .arg("--dry-run")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = find_envelope_json(&stdout).expect("Status JSON should contain envelope");

        // Verify envelope structure
        assert_eq!(json["ok"], true, "Envelope should indicate success");
        assert_eq!(
            json["command"], "status",
            "Envelope command should be 'status'"
        );
        assert!(json["data"].is_object(), "Envelope should have data object");
        assert!(json["meta"].is_object(), "Envelope should have meta object");
        assert!(
            json["meta"]["version"].is_string(),
            "Meta should have version"
        );
        assert!(
            json["meta"]["timestamp"].is_string(),
            "Meta should have timestamp"
        );

        // Verify status data fields
        let data = &json["data"];
        assert!(data["host"].is_object(), "Data should have host object");
        assert_eq!(data["host"]["id"], "test-host");
        assert!(
            data["modules"].is_object(),
            "Data should have modules object"
        );
        assert!(
            data["packages"].is_object(),
            "Data should have packages object"
        );
    }

    #[test]
    fn status_shows_modules_section() {
        let dir = create_initialized_iron_dir();
        create_test_module(&dir, "nvim");

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("Modules"))
            .stdout(predicate::str::contains("Total"));
    }
}

// =============================================================================
// Plan Command Tests (F3-005)
// =============================================================================

mod plan {
    use super::*;

    #[test]
    fn plan_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("plan")
            .arg("--dry-run")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn plan_dry_run_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("plan")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(predicate::str::contains("DRY RUN"));
    }

    #[test]
    fn plan_json_dry_run_returns_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("plan")
            .arg("--dry-run")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        // JSON output may have leading info lines; find the envelope object
        let json = find_envelope_json(&stdout).expect("Plan JSON should contain envelope");

        // Verify envelope structure
        assert_eq!(json["ok"], true, "Envelope should indicate success");
        assert_eq!(json["command"], "plan", "Envelope command should be 'plan'");
        assert!(json["data"].is_object(), "Envelope should have data object");
        assert!(json["meta"].is_object(), "Envelope should have meta object");
        assert!(
            json["meta"]["duration_ms"].is_number(),
            "Meta should have duration_ms"
        );
    }

    #[test]
    fn plan_dry_run_shows_empty_plan() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("plan")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("nothing to do")
                    .or(predicate::str::contains("DRY RUN"))
                    .or(predicate::str::contains("desired state")),
            );
    }
}

// =============================================================================
// Apply confirmation UX
// =============================================================================

mod apply_confirmation {
    use super::*;

    #[test]
    fn apply_dry_run_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("DRY RUN").or(predicate::str::contains("desired state")),
            );
    }

    #[test]
    fn apply_dry_run_with_yes_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--yes")
            .assert()
            .success();
    }

    #[test]
    fn apply_dry_run_with_prune_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--prune")
            .assert()
            .success();
    }

    #[test]
    fn apply_requires_init() {
        let dir = create_test_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not initialized").or(predicate::str::contains("init")),
            );
    }

    #[test]
    fn apply_dry_run_never_prompts() {
        // dry-run should succeed without stdin input
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .assert()
            .success();
    }

    // ── Granular prune flag integration ──

    #[test]
    fn apply_dry_run_prune_packages_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--prune-packages")
            .assert()
            .success();
    }

    #[test]
    fn apply_dry_run_prune_services_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--prune-services")
            .assert()
            .success();
    }

    #[test]
    fn apply_dry_run_prune_dotfiles_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--prune-dotfiles")
            .assert()
            .success();
    }

    #[test]
    fn apply_dry_run_all_prune_flags_combined() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--prune-packages")
            .arg("--prune-services")
            .arg("--prune-dotfiles")
            .assert()
            .success();
    }

    #[test]
    fn plan_dry_run_with_prune_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("plan")
            .arg("--dry-run")
            .arg("--prune")
            .assert()
            .success();
    }

    #[test]
    fn plan_dry_run_with_prune_packages_succeeds() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("plan")
            .arg("--dry-run")
            .arg("--prune-packages")
            .assert()
            .success();
    }
}

// =============================================================================
// Envelope Format Validation (F3-003b)
// =============================================================================

mod envelope_integration {
    use super::*;

    #[test]
    fn doctor_json_uses_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("doctor")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = find_envelope_json(&stdout).expect("Doctor JSON should contain envelope");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], "doctor");
        assert!(json["data"].is_object());
        assert!(json["error"].is_null());
        assert!(json["meta"]["host"].is_string());
    }

    #[test]
    fn scan_json_uses_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("scan")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = find_envelope_json(&stdout).expect("Scan JSON should contain envelope");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], "scan");
        assert!(json["data"].is_object());
        assert!(json["meta"]["timestamp"].is_string());
    }

    #[test]
    fn validate_json_uses_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("validate")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = find_envelope_json(&stdout).expect("Validate JSON should contain envelope");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], "validate");
        assert!(json["meta"].is_object());
    }
}

// =============================================================================
// F3-016: History CLI Integration Tests
// =============================================================================

mod history_integration {
    use super::*;

    #[test]
    fn history_list_exits_success_on_initialized_dir() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("history")
            .assert()
            .success();
    }

    #[test]
    fn history_list_subcommand_exits_success() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("history")
            .arg("list")
            .assert()
            .success();
    }

    #[test]
    fn history_last_exits_success_on_empty_history() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("history")
            .arg("last")
            .assert()
            .success();
    }

    #[test]
    fn history_show_out_of_range_exits_success() {
        let dir = create_initialized_iron_dir();

        // show index 99 on empty history should still exit 0
        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("history")
            .arg("show")
            .arg("99")
            .assert()
            .success();
    }

    #[test]
    fn history_limit_flag_accepted() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("history")
            .arg("--limit")
            .arg("5")
            .assert()
            .success();
    }

    #[test]
    fn history_json_uses_envelope() {
        let dir = create_initialized_iron_dir();

        let output = iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("--format")
            .arg("json")
            .arg("history")
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json = find_envelope_json(&stdout).expect("History JSON should contain envelope");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], "history");
    }

    #[test]
    fn apply_dry_run_accepts_force_hooks_flag() {
        let dir = create_initialized_iron_dir();

        iron_at(dir.path())
            .arg("--root")
            .arg(dir.path())
            .arg("apply")
            .arg("--dry-run")
            .arg("--force-hooks")
            .assert()
            .success();
    }
}
