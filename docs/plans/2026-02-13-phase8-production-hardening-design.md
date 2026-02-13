# Phase 8 Production Hardening Design

> **Status**: Approved
> **Date**: 2026-02-13
> **Covers**: NFR-9, NFR-10, NFR-11, AT-1 through AT-6

---

## Overview

This document defines the design for three Phase 8 production hardening features:
1. Structured JSON logging with rotation
2. Graceful degradation for optional services
3. Acceptance test suite

---

## 1. Structured JSON Logging (NFR-9, NFR-10)

### Requirements

| NFR | Requirement | Specification |
|-----|-------------|---------------|
| NFR-9 | Structured logging | JSON format with timestamp, level, component, message |
| NFR-10 | Log rotation | 10MB max file size, keep 5 rotated files |

### Design

**Log location**: `~/.local/share/iron/logs/` (XDG data directory)

**File structure**:
```
~/.local/share/iron/logs/
├── iron.log          # Current log (JSON lines)
├── iron.log.1        # Rotated (newest)
├── iron.log.2
├── iron.log.3
└── iron.log.4        # Oldest
```

**Log format** (JSON lines, one object per line):
```json
{"timestamp":"2026-02-13T14:30:00.123Z","level":"INFO","target":"iron_core::services::update","message":"Starting update check","packages":42}
```

**Implementation approach**: `tracing-subscriber` with `tracing-appender`

### New Dependencies

```toml
# Cargo.toml (workspace)
tracing-appender = "0.2"
```

### New Module

**File**: `crates/iron-core/src/logging.rs`

```rust
use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct LogConfig {
    pub log_dir: PathBuf,
    pub max_size_mb: u64,      // 10 MB default
    pub max_files: usize,      // 5 files default
    pub default_level: String, // "info" default
}

impl Default for LogConfig {
    fn default() -> Self {
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("iron")
            .join("logs");

        Self {
            log_dir,
            max_size_mb: 10,
            max_files: 5,
            default_level: "info".to_string(),
        }
    }
}

pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    // Create log directory
    std::fs::create_dir_all(&config.log_dir)?;

    // File appender with rotation
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER) // We handle size-based rotation
        .filename_prefix("iron")
        .filename_suffix("log")
        .max_log_files(config.max_files)
        .build(&config.log_dir)?;

    // JSON formatting layer for file
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_ansi(false);

    // Stderr layer for warnings/errors
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::WARN);

    // Environment filter (IRON_LOG env var)
    let env_filter = EnvFilter::try_from_env("IRON_LOG")
        .unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stderr_layer)
        .init();

    Ok(())
}
```

### Integration

**File**: `crates/iron-cli/src/main.rs`

```rust
use iron_core::logging::{init_logging, LogConfig};

fn main() -> Result<()> {
    // Initialize structured logging
    let log_config = LogConfig::default();
    init_logging(&log_config)?;

    // ... rest of main
}
```

---

## 2. Graceful Degradation (NFR-11)

### Requirements

| NFR | Requirement | Specification |
|-----|-------------|---------------|
| NFR-11 | Graceful degradation | System remains usable when optional components fail |

### Degradation Matrix

| Service | Detection | Degraded Behavior | User Notification |
|---------|-----------|-------------------|-------------------|
| Secrets (git-crypt) | `which git-crypt` | Skip secret operations | Warning message |
| Sync (git remote) | `git remote -v` | Work offline, queue ops | Warning message |
| Snapshots (timeshift/snapper) | `which timeshift` | Require `--no-snapshot` | Warning + confirmation |
| AUR Helper (paru/yay) | `which paru && which yay` | Fall back to pacman | Warning message |

### Design

**New module**: `crates/iron-core/src/availability.rs`

```rust
use std::fmt;

#[derive(Debug, Clone)]
pub enum AvailabilityStatus {
    Available,
    Degraded { reason: String },
    Unavailable { reason: String },
}

impl AvailabilityStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }
}

#[derive(Debug, Clone)]
pub struct ServiceAvailability {
    pub secrets: AvailabilityStatus,
    pub sync: AvailabilityStatus,
    pub snapshots: AvailabilityStatus,
    pub aur: AvailabilityStatus,
}

impl ServiceAvailability {
    pub fn check() -> Self {
        Self {
            secrets: Self::check_secrets(),
            sync: Self::check_sync(),
            snapshots: Self::check_snapshots(),
            aur: Self::check_aur(),
        }
    }

    fn check_secrets() -> AvailabilityStatus {
        if which::which("git-crypt").is_ok() {
            AvailabilityStatus::Available
        } else {
            AvailabilityStatus::Unavailable {
                reason: "git-crypt not installed".to_string(),
            }
        }
    }

    fn check_sync() -> AvailabilityStatus {
        // Check if git remote is configured and reachable
        // Implementation uses git2 or command
    }

    fn check_snapshots() -> AvailabilityStatus {
        if which::which("timeshift").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("snapper").is_ok() {
            AvailabilityStatus::Degraded {
                reason: "Using snapper (timeshift preferred)".to_string(),
            }
        } else {
            AvailabilityStatus::Unavailable {
                reason: "No snapshot tool installed".to_string(),
            }
        }
    }

    fn check_aur() -> AvailabilityStatus {
        if which::which("paru").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("yay").is_ok() {
            AvailabilityStatus::Degraded {
                reason: "Using yay (paru preferred)".to_string(),
            }
        } else {
            AvailabilityStatus::Unavailable {
                reason: "No AUR helper installed".to_string(),
            }
        }
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let AvailabilityStatus::Unavailable { reason } = &self.secrets {
            warnings.push(format!("Secrets: {}", reason));
        }
        if let AvailabilityStatus::Unavailable { reason } = &self.sync {
            warnings.push(format!("Sync: {}", reason));
        }
        if let AvailabilityStatus::Unavailable { reason } = &self.snapshots {
            warnings.push(format!("Snapshots: {}", reason));
        }
        if let AvailabilityStatus::Unavailable { reason } = &self.aur {
            warnings.push(format!("AUR: {}", reason));
        }

        warnings
    }
}
```

### New Dependency

```toml
# Cargo.toml (workspace)
which = "6.0"
```

### Integration Points

1. **`iron status`**: Show service availability section
2. **`iron doctor`**: Include availability in health checks (already has FR-10.x)
3. **`iron update`**: Check snapshot availability, warn if unavailable
4. **`iron sync`**: Check remote availability, queue if offline
5. **`iron secrets`**: Check git-crypt availability, skip gracefully

---

## 3. Acceptance Test Suite (AT-1 through AT-6)

### Requirements

Map user stories to acceptance tests:

| Test | User Story | Description |
|------|------------|-------------|
| AT-1 | US-1 | First-Time Setup |
| AT-2 | US-5 | Bundle Management |
| AT-3 | US-6 | Profile Management |
| AT-4 | FR-4.x | Module Operations |
| AT-5 | US-2 | Update Workflow |
| AT-6 | US-4 | Recovery Workflow |

### Design

**Directory structure**:
```
tests/
└── acceptance/
    ├── mod.rs                      # Common fixtures and helpers
    ├── at1_first_time_setup.rs     # 6 scenarios
    ├── at2_bundle_management.rs    # 5 scenarios
    ├── at3_profile_management.rs   # 4 scenarios
    ├── at4_module_operations.rs    # 4 scenarios
    ├── at5_update_workflow.rs      # 4 scenarios
    └── at6_recovery_workflow.rs    # 3 scenarios
```

**Total**: 26 acceptance test scenarios

### Test Helpers Module

**File**: `tests/acceptance/mod.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestFixture {
    pub temp_dir: TempDir,
    pub iron_root: PathBuf,
}

impl TestFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let iron_root = temp_dir.path().to_path_buf();
        Self { temp_dir, iron_root }
    }

    pub fn with_initialized_state() -> Self {
        let fixture = Self::new();
        fixture.run_iron(&["init", "--id", "test-host", "--name", "Test Host"]);
        fixture
    }

    pub fn with_bundle(bundle_id: &str) -> Self {
        let fixture = Self::with_initialized_state();
        // Create bundle directory and bundle.toml
        fixture
    }

    pub fn run_iron(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        Command::cargo_bin("iron")
            .unwrap()
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .assert()
    }

    pub fn run_iron_json(&self, args: &[&str]) -> serde_json::Value {
        let output = Command::cargo_bin("iron")
            .unwrap()
            .args(args)
            .arg("--root")
            .arg(&self.iron_root)
            .arg("--json")
            .output()
            .unwrap();

        serde_json::from_slice(&output.stdout).unwrap()
    }
}
```

### AT-1: First-Time Setup (6 scenarios)

**File**: `tests/acceptance/at1_first_time_setup.rs`

```rust
use super::*;

#[test]
fn at1_1_fresh_install_shows_welcome() {
    let fixture = TestFixture::new();
    fixture.run_iron(&[])
        .success()
        .stdout(predicate::str::contains("Welcome to Iron"));
}

#[test]
fn at1_2_init_creates_state_file() {
    let fixture = TestFixture::new();
    fixture.run_iron(&["init", "--id", "test", "--name", "Test"])
        .success();
    assert!(fixture.iron_root.join("state.json").exists());
}

#[test]
fn at1_3_init_detects_hostname() {
    // GIVEN empty iron root
    // WHEN I run iron init without --id
    // THEN it uses system hostname
}

#[test]
fn at1_4_init_creates_host_config() {
    // GIVEN empty iron root
    // WHEN I run iron init
    // THEN hosts/<id>.toml is created
}

#[test]
fn at1_5_init_idempotent_without_force() {
    // GIVEN already initialized iron
    // WHEN I run iron init again
    // THEN it fails with "already initialized" message
}

#[test]
fn at1_6_init_force_reinitializes() {
    // GIVEN already initialized iron
    // WHEN I run iron init --force
    // THEN it succeeds and overwrites state
}
```

### AT-2: Bundle Management (5 scenarios)

```rust
#[test]
fn at2_1_bundle_list_shows_available() { }

#[test]
fn at2_2_bundle_install_activates() { }

#[test]
fn at2_3_bundle_switch_moves_configs() { }

#[test]
fn at2_4_bundle_switch_creates_snapshot() { }

#[test]
fn at2_5_bundle_switch_rollback_on_failure() { }
```

### AT-3: Profile Management (4 scenarios)

```rust
#[test]
fn at3_1_profile_list_shows_available() { }

#[test]
fn at3_2_profile_select_updates_state() { }

#[test]
fn at3_3_profile_inheritance_resolves() { }

#[test]
fn at3_4_profile_modules_effective() { }
```

### AT-4: Module Operations (4 scenarios)

```rust
#[test]
fn at4_1_module_list_shows_all() { }

#[test]
fn at4_2_module_enable_creates_symlinks() { }

#[test]
fn at4_3_module_disable_removes_symlinks() { }

#[test]
fn at4_4_module_conflict_detected() { }
```

### AT-5: Update Workflow (4 scenarios)

```rust
#[test]
fn at5_1_update_dry_run_shows_packages() { }

#[test]
fn at5_2_update_shows_risk_score() { }

#[test]
fn at5_3_update_creates_snapshot() { }

#[test]
fn at5_4_update_resume_continues() { }
```

### AT-6: Recovery Workflow (3 scenarios)

```rust
#[test]
fn at6_1_recover_export_creates_archive() { }

#[test]
fn at6_2_recover_script_generates_installer() { }

#[test]
fn at6_3_recover_import_restores_state() { }
```

---

## Implementation Order

1. **Logging** (foundation for other features)
2. **Graceful Degradation** (independent, enables better UX)
3. **Acceptance Tests** (validates all features)

---

## Success Criteria

- [ ] Logs written to `~/.local/share/iron/logs/` in JSON format
- [ ] Log rotation at 10MB, keeping 5 files
- [ ] `IRON_LOG` env var controls log level
- [ ] `iron status` shows service availability
- [ ] `iron doctor` includes availability in health report
- [ ] Warning messages shown when services unavailable
- [ ] 26 acceptance tests passing
- [ ] All user stories (US-1 through US-6) have test coverage
