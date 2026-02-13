# Phase 8 Production Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement structured JSON logging with rotation, graceful degradation for optional services, and a comprehensive acceptance test suite covering all user stories.

**Architecture:** Three independent features: (1) logging module using tracing-appender for JSON file output with rotation, (2) availability module for detecting and handling missing optional services, (3) acceptance test suite using Rust integration tests with assert_cmd.

**Tech Stack:** Rust, tracing, tracing-appender, tracing-subscriber, which crate, assert_cmd, predicates, tempfile

---

## Part 1: Structured JSON Logging (NFR-9, NFR-10)

### Task 1.1: Add tracing-appender dependency

**Files:**
- Modify: `Cargo.toml:43-44`
- Modify: `crates/iron-core/Cargo.toml:19`

**Step 1: Add tracing-appender to workspace**

```toml
# In Cargo.toml workspace dependencies, after tracing-subscriber line:
tracing-appender = "0.2"
```

**Step 2: Add to iron-core dependencies**

```toml
# In crates/iron-core/Cargo.toml dependencies:
tracing.workspace = true
tracing-subscriber = { workspace = true, features = ["json", "env-filter"] }
tracing-appender.workspace = true
```

**Step 3: Verify build**

Run: `cargo build -p iron-core`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add Cargo.toml crates/iron-core/Cargo.toml
git commit -m "deps: add tracing-appender for structured logging"
```

---

### Task 1.2: Create LogConfig struct with tests

**Files:**
- Create: `crates/iron-core/src/logging.rs`
- Modify: `crates/iron-core/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/iron-core/src/logging.rs`:

```rust
//! Structured JSON logging with file rotation.
//!
//! Provides NFR-9 (JSON logging) and NFR-10 (log rotation) support.

use std::path::PathBuf;

/// Configuration for the logging system.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Directory where log files are written.
    pub log_dir: PathBuf,
    /// Maximum number of rotated log files to keep.
    pub max_files: usize,
    /// Default log level (can be overridden by IRON_LOG env var).
    pub default_level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("iron")
            .join("logs");

        Self {
            log_dir,
            max_files: 5,
            default_level: "info".to_string(),
        }
    }
}

impl LogConfig {
    /// Create a new LogConfig with a custom log directory.
    pub fn with_log_dir(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_config_default_uses_xdg_data_dir() {
        let config = LogConfig::default();
        assert!(config.log_dir.to_string_lossy().contains("iron"));
        assert!(config.log_dir.to_string_lossy().contains("logs"));
    }

    #[test]
    fn log_config_default_values() {
        let config = LogConfig::default();
        assert_eq!(config.max_files, 5);
        assert_eq!(config.default_level, "info");
    }

    #[test]
    fn log_config_with_custom_dir() {
        let custom = PathBuf::from("/tmp/test-logs");
        let config = LogConfig::with_log_dir(custom.clone());
        assert_eq!(config.log_dir, custom);
        assert_eq!(config.max_files, 5); // Other defaults preserved
    }
}
```

**Step 2: Add module to lib.rs**

Add to `crates/iron-core/src/lib.rs`:

```rust
pub mod logging;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test -p iron-core logging`
Expected: 3 tests pass

**Step 4: Commit**

```bash
git add crates/iron-core/src/logging.rs crates/iron-core/src/lib.rs
git commit -m "feat(logging): add LogConfig struct with XDG data directory support"
```

---

### Task 1.3: Implement init_logging function

**Files:**
- Modify: `crates/iron-core/src/logging.rs`

**Step 1: Write the failing test**

Add to `crates/iron-core/src/logging.rs` tests module:

```rust
    #[test]
    fn init_logging_creates_log_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs");
        let config = LogConfig::with_log_dir(log_dir.clone());

        // Directory doesn't exist yet
        assert!(!log_dir.exists());

        // Initialize logging
        let result = init_logging(&config);
        assert!(result.is_ok());

        // Directory now exists
        assert!(log_dir.exists());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p iron-core init_logging_creates`
Expected: FAIL with "cannot find function `init_logging`"

**Step 3: Write minimal implementation**

Add to `crates/iron-core/src/logging.rs` before the tests module:

```rust
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logging system with the given configuration.
///
/// This sets up:
/// - JSON-formatted logs written to files in `config.log_dir`
/// - Automatic log rotation keeping `config.max_files` files
/// - Log level controlled by `IRON_LOG` env var or `config.default_level`
/// - Warnings and errors also printed to stderr
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the file
/// appender cannot be initialized.
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    // Create log directory
    std::fs::create_dir_all(&config.log_dir)?;

    // File appender with daily rotation (tracing-appender handles file management)
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("iron")
        .filename_suffix("log")
        .max_log_files(config.max_files)
        .build(&config.log_dir)?;

    // JSON formatting layer for file
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_ansi(false);

    // Environment filter (IRON_LOG env var)
    let env_filter = EnvFilter::try_from_env("IRON_LOG")
        .unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p iron-core init_logging_creates`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/iron-core/src/logging.rs
git commit -m "feat(logging): implement init_logging with JSON output and rotation"
```

---

### Task 1.4: Integrate logging into CLI main.rs

**Files:**
- Modify: `crates/iron-cli/src/main.rs`

**Step 1: Update main.rs to use new logging**

Replace the logging initialization in `crates/iron-cli/src/main.rs`:

```rust
use iron_core::logging::{init_logging, LogConfig};

fn main() -> Result<()> {
    // Initialize structured JSON logging
    let log_config = LogConfig::default();
    if let Err(e) = init_logging(&log_config) {
        eprintln!("Warning: Failed to initialize logging: {}", e);
        // Fall back to basic stderr logging
        tracing_subscriber::fmt::init();
    }

    let cli = Cli::parse();
    // ... rest unchanged
```

**Step 2: Verify CLI still works**

Run: `cargo run -p iron-cli -- --help`
Expected: Help output displayed, no errors

**Step 3: Verify logs are created**

Run: `cargo run -p iron-cli -- status 2>/dev/null; ls -la ~/.local/share/iron/logs/`
Expected: Log file exists in the directory

**Step 4: Commit**

```bash
git add crates/iron-cli/src/main.rs
git commit -m "feat(cli): integrate structured JSON logging"
```

---

### Task 1.5: Add logging documentation and final tests

**Files:**
- Modify: `crates/iron-core/src/logging.rs`

**Step 1: Add comprehensive tests**

Add to tests module in `crates/iron-core/src/logging.rs`:

```rust
    #[test]
    fn log_config_respects_max_files() {
        let config = LogConfig {
            log_dir: PathBuf::from("/tmp"),
            max_files: 10,
            default_level: "debug".to_string(),
        };
        assert_eq!(config.max_files, 10);
        assert_eq!(config.default_level, "debug");
    }
```

**Step 2: Run all logging tests**

Run: `cargo test -p iron-core logging`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/iron-core/src/logging.rs
git commit -m "test(logging): add comprehensive logging configuration tests"
```

---

## Part 2: Graceful Degradation (NFR-11)

### Task 2.1: Add which crate dependency

**Files:**
- Modify: `Cargo.toml:44`
- Modify: `crates/iron-core/Cargo.toml:20`

**Step 1: Add which to workspace**

Add to `Cargo.toml` workspace dependencies:

```toml
which = "6.0"
```

**Step 2: Add to iron-core**

Add to `crates/iron-core/Cargo.toml`:

```toml
which = { workspace = true }
```

**Step 3: Verify build**

Run: `cargo build -p iron-core`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add Cargo.toml crates/iron-core/Cargo.toml
git commit -m "deps: add which crate for command availability detection"
```

---

### Task 2.2: Create AvailabilityStatus enum

**Files:**
- Create: `crates/iron-core/src/availability.rs`
- Modify: `crates/iron-core/src/lib.rs`

**Step 1: Write the type definitions with tests**

Create `crates/iron-core/src/availability.rs`:

```rust
//! Service availability detection for graceful degradation.
//!
//! Implements NFR-11: System remains usable when optional components fail.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of an optional service's availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum AvailabilityStatus {
    /// Service is fully available.
    Available,
    /// Service is available but using a fallback (e.g., yay instead of paru).
    Degraded(String),
    /// Service is not available.
    Unavailable(String),
}

impl AvailabilityStatus {
    /// Returns true if the service is fully available.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns true if the service can be used (available or degraded).
    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unavailable(_))
    }

    /// Returns the reason string if degraded or unavailable.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Degraded(r) | Self::Unavailable(r) => Some(r),
        }
    }
}

impl fmt::Display for AvailabilityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::Degraded(reason) => write!(f, "degraded: {}", reason),
            Self::Unavailable(reason) => write!(f, "unavailable: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_status_is_available() {
        assert!(AvailabilityStatus::Available.is_available());
        assert!(!AvailabilityStatus::Degraded("test".into()).is_available());
        assert!(!AvailabilityStatus::Unavailable("test".into()).is_available());
    }

    #[test]
    fn availability_status_is_usable() {
        assert!(AvailabilityStatus::Available.is_usable());
        assert!(AvailabilityStatus::Degraded("test".into()).is_usable());
        assert!(!AvailabilityStatus::Unavailable("test".into()).is_usable());
    }

    #[test]
    fn availability_status_reason() {
        assert_eq!(AvailabilityStatus::Available.reason(), None);
        assert_eq!(
            AvailabilityStatus::Degraded("using fallback".into()).reason(),
            Some("using fallback")
        );
        assert_eq!(
            AvailabilityStatus::Unavailable("not installed".into()).reason(),
            Some("not installed")
        );
    }

    #[test]
    fn availability_status_display() {
        assert_eq!(format!("{}", AvailabilityStatus::Available), "available");
        assert_eq!(
            format!("{}", AvailabilityStatus::Degraded("yay".into())),
            "degraded: yay"
        );
        assert_eq!(
            format!("{}", AvailabilityStatus::Unavailable("missing".into())),
            "unavailable: missing"
        );
    }

    #[test]
    fn availability_status_serializes_to_json() {
        let available = serde_json::to_string(&AvailabilityStatus::Available).unwrap();
        assert!(available.contains("Available"));

        let degraded = serde_json::to_string(&AvailabilityStatus::Degraded("test".into())).unwrap();
        assert!(degraded.contains("Degraded"));
        assert!(degraded.contains("test"));
    }
}
```

**Step 2: Add module to lib.rs**

Add to `crates/iron-core/src/lib.rs`:

```rust
pub mod availability;
```

**Step 3: Run tests**

Run: `cargo test -p iron-core availability`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add crates/iron-core/src/availability.rs crates/iron-core/src/lib.rs
git commit -m "feat(availability): add AvailabilityStatus enum for graceful degradation"
```

---

### Task 2.3: Create ServiceAvailability struct with check methods

**Files:**
- Modify: `crates/iron-core/src/availability.rs`

**Step 1: Write the failing test**

Add to tests module:

```rust
    #[test]
    fn service_availability_check_returns_all_services() {
        let availability = ServiceAvailability::check();
        // All fields should be set (not panicking is the test)
        let _ = availability.secrets;
        let _ = availability.sync;
        let _ = availability.snapshots;
        let _ = availability.aur;
    }

    #[test]
    fn service_availability_warnings_collects_unavailable() {
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Unavailable("no remote".into()),
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Unavailable("no helper".into()),
        };
        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("Sync")));
        assert!(warnings.iter().any(|w| w.contains("AUR")));
    }

    #[test]
    fn service_availability_has_warnings() {
        let all_available = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Available,
        };
        assert!(!all_available.has_warnings());

        let some_unavailable = ServiceAvailability {
            secrets: AvailabilityStatus::Unavailable("test".into()),
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Available,
        };
        assert!(some_unavailable.has_warnings());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p iron-core service_availability`
Expected: FAIL with "cannot find struct `ServiceAvailability`"

**Step 3: Write implementation**

Add to `crates/iron-core/src/availability.rs` before tests:

```rust
/// Availability status of all optional services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAvailability {
    /// git-crypt for secrets management.
    pub secrets: AvailabilityStatus,
    /// Git remote for sync operations.
    pub sync: AvailabilityStatus,
    /// Snapshot tool (timeshift or snapper).
    pub snapshots: AvailabilityStatus,
    /// AUR helper (paru or yay).
    pub aur: AvailabilityStatus,
}

impl ServiceAvailability {
    /// Check availability of all optional services.
    pub fn check() -> Self {
        Self {
            secrets: Self::check_secrets(),
            sync: Self::check_sync(),
            snapshots: Self::check_snapshots(),
            aur: Self::check_aur(),
        }
    }

    /// Check git-crypt availability.
    fn check_secrets() -> AvailabilityStatus {
        if which::which("git-crypt").is_ok() {
            AvailabilityStatus::Available
        } else {
            AvailabilityStatus::Unavailable("git-crypt not installed".into())
        }
    }

    /// Check git remote availability.
    fn check_sync() -> AvailabilityStatus {
        // For now, just check if git is available
        // Full remote check would require repository context
        if which::which("git").is_ok() {
            AvailabilityStatus::Available
        } else {
            AvailabilityStatus::Unavailable("git not installed".into())
        }
    }

    /// Check snapshot tool availability.
    fn check_snapshots() -> AvailabilityStatus {
        if which::which("timeshift").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("snapper").is_ok() {
            AvailabilityStatus::Degraded("using snapper (timeshift preferred)".into())
        } else {
            AvailabilityStatus::Unavailable("no snapshot tool installed (timeshift or snapper)".into())
        }
    }

    /// Check AUR helper availability.
    fn check_aur() -> AvailabilityStatus {
        if which::which("paru").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("yay").is_ok() {
            AvailabilityStatus::Degraded("using yay (paru preferred)".into())
        } else {
            AvailabilityStatus::Unavailable("no AUR helper installed (paru or yay)".into())
        }
    }

    /// Get warning messages for unavailable or degraded services.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let AvailabilityStatus::Unavailable(reason) | AvailabilityStatus::Degraded(reason) = &self.secrets {
            warnings.push(format!("Secrets: {}", reason));
        }
        if let AvailabilityStatus::Unavailable(reason) | AvailabilityStatus::Degraded(reason) = &self.sync {
            warnings.push(format!("Sync: {}", reason));
        }
        if let AvailabilityStatus::Unavailable(reason) | AvailabilityStatus::Degraded(reason) = &self.snapshots {
            warnings.push(format!("Snapshots: {}", reason));
        }
        if let AvailabilityStatus::Unavailable(reason) | AvailabilityStatus::Degraded(reason) = &self.aur {
            warnings.push(format!("AUR: {}", reason));
        }

        warnings
    }

    /// Returns true if any service is unavailable or degraded.
    pub fn has_warnings(&self) -> bool {
        !self.secrets.is_available()
            || !self.sync.is_available()
            || !self.snapshots.is_available()
            || !self.aur.is_available()
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p iron-core availability`
Expected: All 8 tests pass

**Step 5: Commit**

```bash
git add crates/iron-core/src/availability.rs
git commit -m "feat(availability): add ServiceAvailability with check methods for all optional services"
```

---

### Task 2.4: Integrate availability into iron doctor

**Files:**
- Modify: `crates/iron-cli/src/commands/doctor.rs`

**Step 1: Read current doctor implementation**

First, check the current structure of doctor.rs to understand the integration point.

**Step 2: Add availability check to doctor**

Add to the doctor command output (after existing checks):

```rust
use iron_core::availability::ServiceAvailability;

// In the execute function, add:
let availability = ServiceAvailability::check();

// Add to checks vector:
checks.push(HealthCheck {
    name: "services".to_string(),
    status: if availability.has_warnings() {
        CheckStatus::Warn
    } else {
        CheckStatus::Pass
    },
    message: if availability.has_warnings() {
        format!("{} service(s) degraded or unavailable", availability.warnings().len())
    } else {
        "all optional services available".to_string()
    },
});
```

**Step 3: Run doctor to verify**

Run: `cargo run -p iron-cli -- doctor --json`
Expected: JSON output includes "services" check

**Step 4: Commit**

```bash
git add crates/iron-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add service availability check (NFR-11)"
```

---

### Task 2.5: Integrate availability into iron status

**Files:**
- Modify: `crates/iron-cli/src/commands/status.rs`

**Step 1: Add availability section to status output**

Add to status command:

```rust
use iron_core::availability::ServiceAvailability;

// In execute function, after main status output:
let availability = ServiceAvailability::check();
if availability.has_warnings() {
    ctx.output.warning("Service Availability:");
    for warning in availability.warnings() {
        ctx.output.warning(&format!("  {}", warning));
    }
}
```

**Step 2: Run status to verify**

Run: `cargo run -p iron-cli -- status`
Expected: Shows service warnings if any

**Step 3: Commit**

```bash
git add crates/iron-cli/src/commands/status.rs
git commit -m "feat(status): show service availability warnings (NFR-11)"
```

---

## Part 3: Acceptance Test Suite (AT-1 through AT-6)

### Task 3.1: Create acceptance test directory structure

**Files:**
- Create: `crates/iron-cli/tests/acceptance/mod.rs`

**Step 1: Create test helpers module**

Create `crates/iron-cli/tests/acceptance/mod.rs`:

```rust
//! Acceptance test helpers and fixtures.
//!
//! Provides common utilities for AT-1 through AT-6 acceptance tests.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture providing isolated Iron environment.
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub iron_root: PathBuf,
}

impl TestFixture {
    /// Create a new empty test fixture.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let iron_root = temp_dir.path().to_path_buf();

        // Create required directories
        fs::create_dir_all(iron_root.join("bundles")).unwrap();
        fs::create_dir_all(iron_root.join("profiles")).unwrap();
        fs::create_dir_all(iron_root.join("modules")).unwrap();
        fs::create_dir_all(iron_root.join("hosts")).unwrap();

        Self { temp_dir, iron_root }
    }

    /// Create a fixture with Iron already initialized.
    pub fn with_initialized_state() -> Self {
        let fixture = Self::new();
        fixture
            .run_iron(&["init", "--id", "test-host", "--name", "Test Host"])
            .success();
        fixture
    }

    /// Run iron CLI with the given arguments.
    pub fn run_iron(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        Command::cargo_bin("iron")
            .expect("Failed to find iron binary")
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .assert()
    }

    /// Run iron CLI and return JSON output.
    pub fn run_iron_json(&self, args: &[&str]) -> serde_json::Value {
        let output = Command::cargo_bin("iron")
            .expect("Failed to find iron binary")
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .arg("--json")
            .output()
            .expect("Failed to execute iron");

        serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "Failed to parse JSON output: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    /// Create a test bundle in the fixture.
    pub fn create_bundle(&self, id: &str) {
        let bundle_dir = self.iron_root.join("bundles").join(id);
        fs::create_dir_all(&bundle_dir).unwrap();
        fs::write(
            bundle_dir.join("bundle.toml"),
            format!(
                r#"[bundle]
id = "{}"
name = "Test Bundle {}"
description = "A test bundle"
bundle_type = "Desktop"

[packages]
packages = ["test-package"]
"#,
                id,
                id.to_uppercase()
            ),
        )
        .unwrap();
    }

    /// Create a test profile in the fixture.
    pub fn create_profile(&self, id: &str) {
        let profile_dir = self.iron_root.join("profiles").join(id);
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(
            profile_dir.join("profile.toml"),
            format!(
                r#"[profile]
id = "{}"
name = "Test Profile {}"
description = "A test profile"
"#,
                id,
                id.to_uppercase()
            ),
        )
        .unwrap();
    }

    /// Create a test module in the fixture.
    pub fn create_module(&self, id: &str) {
        let module_dir = self.iron_root.join("modules").join(id);
        fs::create_dir_all(&module_dir).unwrap();
        fs::write(
            module_dir.join("module.toml"),
            format!(
                r#"[module]
id = "{}"
name = "Test Module {}"
description = "A test module"
kind = "Application"
"#,
                id,
                id.to_uppercase()
            ),
        )
        .unwrap();
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Verify module compiles**

Run: `cargo build -p iron-cli --tests`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/
git commit -m "test(acceptance): add test fixture helpers for acceptance tests"
```

---

### Task 3.2: Implement AT-1 First-Time Setup tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at1_first_time_setup.rs`

**Step 1: Write AT-1 tests**

Create `crates/iron-cli/tests/acceptance/at1_first_time_setup.rs`:

```rust
//! AT-1: First-Time Setup acceptance tests.
//!
//! Tests US-1: First-Time Setup user story.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at1_1_fresh_install_shows_welcome() {
    let fixture = TestFixture::new();
    fixture
        .run_iron(&[])
        .success()
        .stdout(predicate::str::contains("Welcome to Iron"));
}

#[test]
fn at1_2_init_creates_state_file() {
    let fixture = TestFixture::new();
    fixture
        .run_iron(&["init", "--id", "test", "--name", "Test"])
        .success();
    assert!(fixture.iron_root.join("state.json").exists());
}

#[test]
fn at1_3_init_creates_host_config() {
    let fixture = TestFixture::new();
    fixture
        .run_iron(&["init", "--id", "myhost", "--name", "My Host"])
        .success();
    assert!(fixture.iron_root.join("hosts").join("myhost.toml").exists());
}

#[test]
fn at1_4_init_idempotent_without_force() {
    let fixture = TestFixture::with_initialized_state();
    fixture
        .run_iron(&["init", "--id", "test", "--name", "Test"])
        .failure()
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn at1_5_init_force_reinitializes() {
    let fixture = TestFixture::with_initialized_state();
    fixture
        .run_iron(&["init", "--id", "new-host", "--name", "New Host", "--force"])
        .success();
}

#[test]
fn at1_6_status_requires_initialization() {
    let fixture = TestFixture::new();
    fixture
        .run_iron(&["status"])
        .failure()
        .stderr(predicate::str::contains("not initialized"));
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at1_`
Expected: All 6 tests pass (adjust expectations based on actual CLI behavior)

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at1_first_time_setup.rs
git commit -m "test(acceptance): add AT-1 first-time setup tests (US-1)"
```

---

### Task 3.3: Implement AT-2 Bundle Management tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at2_bundle_management.rs`

**Step 1: Write AT-2 tests**

Create `crates/iron-cli/tests/acceptance/at2_bundle_management.rs`:

```rust
//! AT-2: Bundle Management acceptance tests.
//!
//! Tests US-5: Environment Switch user story.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at2_1_bundle_list_shows_available() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("hyprland");
    fixture.create_bundle("niri");

    fixture
        .run_iron(&["bundle", "list"])
        .success()
        .stdout(predicate::str::contains("hyprland"))
        .stdout(predicate::str::contains("niri"));
}

#[test]
fn at2_2_bundle_status_shows_none_active() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("test-bundle");

    fixture
        .run_iron(&["bundle", "status"])
        .success()
        .stdout(predicate::str::contains("No active bundle"));
}

#[test]
fn at2_3_bundle_install_activates() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("test-bundle");

    fixture
        .run_iron(&["bundle", "install", "test-bundle"])
        .success();

    fixture
        .run_iron(&["bundle", "status"])
        .success()
        .stdout(predicate::str::contains("test-bundle"));
}

#[test]
fn at2_4_bundle_install_nonexistent_fails() {
    let fixture = TestFixture::with_initialized_state();

    fixture
        .run_iron(&["bundle", "install", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn at2_5_bundle_list_json_format() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("hyprland");

    let json = fixture.run_iron_json(&["bundle", "list"]);
    assert!(json.is_object() || json.is_array());
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at2_`
Expected: Tests pass (adjust based on actual CLI)

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at2_bundle_management.rs
git commit -m "test(acceptance): add AT-2 bundle management tests (US-5)"
```

---

### Task 3.4: Implement AT-3 Profile Management tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at3_profile_management.rs`

**Step 1: Write AT-3 tests**

Create `crates/iron-cli/tests/acceptance/at3_profile_management.rs`:

```rust
//! AT-3: Profile Management acceptance tests.
//!
//! Tests US-6: Custom Profile Creation user story.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at3_1_profile_list_shows_available() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("minimal");
    fixture.create_profile("developer");

    fixture
        .run_iron(&["profile", "list"])
        .success()
        .stdout(predicate::str::contains("minimal"))
        .stdout(predicate::str::contains("developer"));
}

#[test]
fn at3_2_profile_show_displays_details() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("developer");

    fixture
        .run_iron(&["profile", "show", "developer"])
        .success()
        .stdout(predicate::str::contains("developer"));
}

#[test]
fn at3_3_profile_show_nonexistent_fails() {
    let fixture = TestFixture::with_initialized_state();

    fixture
        .run_iron(&["profile", "show", "nonexistent"])
        .failure();
}

#[test]
fn at3_4_profile_list_json_format() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("test");

    let json = fixture.run_iron_json(&["profile", "list"]);
    assert!(json.is_object() || json.is_array());
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at3_`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at3_profile_management.rs
git commit -m "test(acceptance): add AT-3 profile management tests (US-6)"
```

---

### Task 3.5: Implement AT-4 Module Operations tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at4_module_operations.rs`

**Step 1: Write AT-4 tests**

Create `crates/iron-cli/tests/acceptance/at4_module_operations.rs`:

```rust
//! AT-4: Module Operations acceptance tests.
//!
//! Tests FR-4.x: Module Management functional requirements.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at4_1_module_list_shows_all() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");
    fixture.create_module("zsh");

    fixture
        .run_iron(&["module", "list"])
        .success()
        .stdout(predicate::str::contains("neovim"))
        .stdout(predicate::str::contains("zsh"));
}

#[test]
fn at4_2_module_show_displays_details() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");

    fixture
        .run_iron(&["module", "show", "neovim"])
        .success()
        .stdout(predicate::str::contains("neovim"));
}

#[test]
fn at4_3_module_show_nonexistent_fails() {
    let fixture = TestFixture::with_initialized_state();

    fixture
        .run_iron(&["module", "show", "nonexistent"])
        .failure();
}

#[test]
fn at4_4_module_list_json_format() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("test");

    let json = fixture.run_iron_json(&["module", "list"]);
    assert!(json.is_object() || json.is_array());
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at4_`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at4_module_operations.rs
git commit -m "test(acceptance): add AT-4 module operations tests (FR-4.x)"
```

---

### Task 3.6: Implement AT-5 Update Workflow tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at5_update_workflow.rs`

**Step 1: Write AT-5 tests**

Create `crates/iron-cli/tests/acceptance/at5_update_workflow.rs`:

```rust
//! AT-5: Update Workflow acceptance tests.
//!
//! Tests US-2: Safe Updates user story.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at5_1_update_dry_run_shows_packages() {
    let fixture = TestFixture::with_initialized_state();

    fixture
        .run_iron(&["update", "--dry-run"])
        .success()
        .stdout(predicate::str::contains("dry run"));
}

#[test]
fn at5_2_update_status_shows_progress() {
    let fixture = TestFixture::with_initialized_state();

    fixture
        .run_iron(&["update", "--status"])
        .success();
}

#[test]
fn at5_3_update_requires_initialization() {
    let fixture = TestFixture::new();

    fixture
        .run_iron(&["update", "--dry-run"])
        .failure()
        .stderr(predicate::str::contains("not initialized"));
}

#[test]
fn at5_4_update_json_format() {
    let fixture = TestFixture::with_initialized_state();

    let json = fixture.run_iron_json(&["update", "--dry-run"]);
    assert!(json.is_object());
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at5_`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at5_update_workflow.rs
git commit -m "test(acceptance): add AT-5 update workflow tests (US-2)"
```

---

### Task 3.7: Implement AT-6 Recovery Workflow tests

**Files:**
- Create: `crates/iron-cli/tests/acceptance/at6_recovery_workflow.rs`

**Step 1: Write AT-6 tests**

Create `crates/iron-cli/tests/acceptance/at6_recovery_workflow.rs`:

```rust
//! AT-6: Recovery Workflow acceptance tests.
//!
//! Tests US-4: Disaster Recovery user story.

mod acceptance;

use acceptance::TestFixture;
use predicates::prelude::*;

#[test]
fn at6_1_recover_export_creates_archive() {
    let fixture = TestFixture::with_initialized_state();
    let export_path = fixture.iron_root.join("export.json");

    fixture
        .run_iron(&["recover", "--export", export_path.to_str().unwrap()])
        .success();

    assert!(export_path.exists());
}

#[test]
fn at6_2_recover_script_generates_installer() {
    let fixture = TestFixture::with_initialized_state();
    let script_path = fixture.iron_root.join("install.sh");

    fixture
        .run_iron(&["recover", "--script", script_path.to_str().unwrap()])
        .success();

    assert!(script_path.exists());
}

#[test]
fn at6_3_recover_requires_initialization() {
    let fixture = TestFixture::new();

    fixture
        .run_iron(&["recover", "--export", "/tmp/export.json"])
        .failure()
        .stderr(predicate::str::contains("not initialized"));
}
```

**Step 2: Run tests**

Run: `cargo test -p iron-cli at6_`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/iron-cli/tests/acceptance/at6_recovery_workflow.rs
git commit -m "test(acceptance): add AT-6 recovery workflow tests (US-4)"
```

---

### Task 3.8: Create main acceptance test entry point

**Files:**
- Create: `crates/iron-cli/tests/acceptance.rs`

**Step 1: Create entry point**

Create `crates/iron-cli/tests/acceptance.rs`:

```rust
//! Acceptance Test Suite (AT-1 through AT-6)
//!
//! This module provides the entry point for all acceptance tests.
//! Tests are organized by user story:
//!
//! - AT-1: First-Time Setup (US-1)
//! - AT-2: Bundle Management (US-5)
//! - AT-3: Profile Management (US-6)
//! - AT-4: Module Operations (FR-4.x)
//! - AT-5: Update Workflow (US-2)
//! - AT-6: Recovery Workflow (US-4)

mod acceptance;
```

**Step 2: Run all acceptance tests**

Run: `cargo test -p iron-cli acceptance`
Expected: All 26 acceptance tests pass

**Step 3: Final commit**

```bash
git add crates/iron-cli/tests/acceptance.rs
git commit -m "test(acceptance): complete AT-1 through AT-6 acceptance test suite"
```

---

## Final Validation

### Task 4.1: Run full test suite

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass (1337+ existing + new tests)

**Step 2: Check test count**

Run: `cargo test --workspace 2>&1 | grep -E "test result" | awk '{sum += $4} END {print "Total:", sum}'`
Expected: Total: 1363+ (26 new acceptance tests)

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: complete Phase 8.4-8.6 production hardening

- NFR-9: Structured JSON logging
- NFR-10: Log rotation (5 files)
- NFR-11: Graceful degradation for optional services
- AT-1 through AT-6: Acceptance test suite (26 tests)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Part | Tasks | Tests Added | Files Created/Modified |
|------|-------|-------------|------------------------|
| 1. Logging | 5 | 5 | logging.rs, main.rs, Cargo.toml |
| 2. Graceful Degradation | 5 | 8 | availability.rs, doctor.rs, status.rs |
| 3. Acceptance Tests | 8 | 26 | 7 test files |
| **Total** | **18** | **39** | **12** |

**Estimated time**: 3-4 hours for experienced developer
