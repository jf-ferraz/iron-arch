# Architect Report -- Sprint 3.1 (Foundation Contracts)

**Date:** 2026-02-22
**Type:** ENHANCEMENT (structural)
**Sprint:** 3.1 -- Foundation Contracts
**Tasks:** F3-001, F3-002a, F3-002b, F3-003a, F3-003b, F3-004, F3-005, F3-006, F3-007

---

## 1. Architectural Decisions

### Decision AQ-1: ActualState File Location

- **Choice**: New file at `iron-core/src/actual_state.rs`, registered as `pub mod actual_state` in `lib.rs`.
- **Rationale**: `DesiredState` lives in `services/apply.rs` because it is an intermediate computation artifact of the apply pipeline. `ActualState` is fundamentally different -- it is a first-class domain model representing system reality, consumed by multiple services (apply, drift, status, plan). Placing it in a dedicated module at the crate root (alongside `host.rs`, `module.rs`, `state.rs`) signals that it is a core domain concept, not an implementation detail of any single service. It also avoids making `apply.rs` even larger (it is already 1034 lines).
- **Rejected**: Extending `host.rs`. The Host struct represents the user's declaration in TOML -- mixing system-queried state into the same file would conflate two distinct concepts (what the user wants vs what the system has). Different lifecycles, different serialization contexts, different consumers.
- **Consequences**: New `pub mod actual_state;` line in `lib.rs`. New re-exports for `ActualState`, `ActualServiceState`, `ActualFileState`, `FileStateType`. The `scan()` method lives on `ActualState` as an associated function, keeping the scanning logic co-located with the struct definition.

### Decision AQ-2: Trait Signature Strategy for F3-002b

- **Choice**: Keep `ApplyService` and `DriftService` trait method signatures unchanged. Refactor only the private internal methods (`compute_plan()` on `DefaultApplyService`, `detect_package_drift()`/`detect_service_drift()`/`detect_config_drift()` on `DefaultDriftService`).
- **Rationale**: The analyst confirmed that `compute_plan()` is already a private method on `DefaultApplyService` (not on the trait). The public trait methods are `plan(&self, host_id: &str)` and `plan_module(&self, module_id: &str)`. These orchestrate the full flow: load host, resolve desired state, scan actual state, compute plan. The scanning is an implementation detail that callers should not need to manage. Changing trait signatures would cascade through every consumer (CLI commands, TUI, snapshot restore) with no benefit -- callers never have a pre-existing `ActualState` to pass in.
- **Rejected**: Adding `&ActualState` to trait method signatures. This would give callers "control over scanning" but there is no use case where a caller has a pre-scanned `ActualState` they want to reuse. The scanning takes ~1-2 seconds (pacman query) and is always done immediately before plan/detect. Forcing callers to scan first adds boilerplate with no practical benefit. It also breaks the clean `plan(host_id)` API.
- **Rejected**: Adding new methods alongside old ones (deprecation path). Unnecessary complexity -- since the trait methods are not changing, there is nothing to deprecate.
- **Consequences**: The cascade is contained entirely within `DefaultApplyService` and `DefaultDriftService`. The public `plan()` and `detect()` methods scan `ActualState` internally, then pass it to the private computation methods. CLI commands, TUI, and snapshot service require zero changes for F3-002b. Tests that mock `ApplyService` or `DriftService` via the trait require zero changes. Only tests that construct `DefaultApplyService`/`DefaultDriftService` directly and test `compute_plan()` behavior need updating -- and those are the tests that should be verifying ActualState consumption anyway. This dramatically reduces F3-002b's risk level from HIGH to MEDIUM.

### Decision AQ-3: IronEnvelope Location

- **Choice**: Place `IronEnvelope<T>`, `EnvelopeError`, and `EnvelopeMeta` in `iron-core/src/envelope.rs`. The `Output::json_envelope()` and `Output::json_error_envelope()` convenience methods stay in `iron-cli/src/output.rs`.
- **Rationale**: The envelope structs need `Serialize` and are pure data containers with no CLI dependency. Placing them in `iron-core` allows the TUI crate to use them if needed (e.g., for exporting TUI state as JSON). It also allows `iron-core` tests to construct and verify envelope serialization without pulling in CLI dependencies. The `Output` methods are CLI-specific sugar that delegates to the core structs.
- **Rejected**: Placing everything in `iron-cli/src/output.rs`. This would prevent `iron-tui` and `iron-core` tests from accessing the envelope types. Since the envelope is a data format contract (not a presentation concern), it belongs in the domain layer.
- **Consequences**: `iron-core/src/lib.rs` gains `pub mod envelope;`. The CLI crate imports envelope types from `iron_core::envelope`. The `T: Serialize` bound is on the struct definition (see AQ-3 sub-decision below).

**Sub-decision: Serialize bound placement**

- **Choice**: `T: Serialize` bound on the `impl` blocks, not on the struct definition. The struct itself uses `#[derive(Debug, Clone)]` only. The `Serialize` impl requires `T: Serialize`.
- **Rationale**: This is idiomatic Rust. Putting the bound on the struct definition (`struct IronEnvelope<T: Serialize>`) forces all code that even mentions the type to satisfy the bound, including test code that might want to construct envelopes with simple types. Keeping the bound on `impl` blocks is more ergonomic and matches patterns used by `serde_json::Value` and similar types in the ecosystem.
- **Consequences**: Manual `Serialize` implementation instead of `#[derive(Serialize)]` on the struct. Alternatively, use `#[derive(Serialize)]` with `#[serde(bound = "T: Serialize")]` which achieves the same effect while keeping derive convenience.

### Decision AQ-4: XDG State Path Resolution Location

- **Choice**: Place the `state_dir()` resolution function as a public associated function on `StateManager` in `iron-core/src/services/state.rs`. The `AppContext` in `iron-cli` calls it during construction.
- **Rationale**: State directory resolution is a concern of `StateManager` -- it determines where state files live. Placing it in `iron-cli` would mean `iron-tui` needs its own copy (the TUI constructs services independently in `app/mod.rs`). Centralizing in `iron-core` ensures all consumers resolve the same path. The function is pure (reads env vars and returns a PathBuf) so it has no CLI dependencies.
- **Rejected**: Placing in `iron-cli/src/context.rs`. The TUI crate also needs this resolution (it constructs `StateManager` directly). Duplicating the logic would violate DRY and risk divergence.
- **Consequences**: `StateManager::new()` signature changes from `new(root: PathBuf)` to `new(config_root: PathBuf)` where `config_root` is the iron configuration root (e.g., `~/.config/iron`). Internally, `new()` calls `Self::state_dir()` to determine where `state.json` lives. The `root()` method continues to return the config root. A new `state_root()` method returns the state directory path.

### Decision: StateManager Constructor Signature (analyst question 2)

- **Choice**: `StateManager::new(config_root: PathBuf)` continues to accept a single path (the config root). State directory is resolved internally via `Self::state_dir()`.
- **Rationale**: The existing constructor takes a single `root` path. Keeping a single parameter maintains backward compatibility with all existing callers (`AppContext::new`, TUI `App::new`, test helpers). The state directory is derived from environment variables, not from the caller -- so the caller has no useful second parameter to provide. Tests override via `$IRON_STATE_DIR` env var.
- **Rejected**: Two-parameter constructor `new(config_root, state_dir)`. This forces every caller to know about XDG resolution, which is an implementation detail. It also complicates test setup (every test would need to compute both paths).
- **Consequences**: Existing callers remain unchanged. The `StateManager` internally stores both `config_root` (for migration reference) and `state_root` (for state.json/audit.log/lock). Test code sets `$IRON_STATE_DIR` to a temp directory to control state location.

### Decision: Status Command Package Count Source (analyst question 4)

- **Choice**: Option (b) -- count packages from resolved `DesiredState`. Without `--full`, `iron status` resolves the desired state (which requires loading TOML files from disk, a fast operation) and reports `desired.packages.len()` as the "managed packages" count. With `--full`, it scans `ActualState` and reports both desired and installed counts.
- **Rationale**: Option (a) would make `iron status` conspicuously incomplete -- a status command that cannot show package counts is not useful. Option (c) counting from state's module list is indirect and inaccurate (module lists do not contain package counts without loading module.toml files). Option (b) gives an accurate "managed count" from the declaration layer, which is fast (TOML file reads) and does not require the `managed_packages` state field from Sprint 3.2.
- **Rejected**: Option (a) skip until Sprint 3.2. Makes the status command feel incomplete for the entire sprint.
- **Rejected**: Option (c) count from state module list. Requires loading all module.toml files anyway to get package counts, which is equivalent to option (b) with extra indirection.
- **Consequences**: `iron status` resolves `DesiredState` on every invocation (fast -- file reads only). The "managed" count is the desired package count, labeled as "Packages (declared)" to distinguish from actual installed counts.

---

## 2. Module/File Layout

### New Files

| File | Responsibility |
|------|---------------|
| `crates/iron-core/src/actual_state.rs` | `ActualState`, `ActualServiceState`, `ActualFileState`, `FileStateType` structs + `ActualState::scan()` |
| `crates/iron-core/src/envelope.rs` | `IronEnvelope<T>`, `EnvelopeError`, `EnvelopeMeta` structs + constructors |
| `crates/iron-cli/src/commands/plan.rs` | `iron plan` command implementation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/iron-core/src/lib.rs` | Add `pub mod actual_state;` and `pub mod envelope;`, add re-exports |
| `crates/iron-core/Cargo.toml` | Add `gethostname = "0.2"` and `sha2 = "0.10"` dependencies |
| `crates/iron-core/src/services/apply.rs` | `compute_plan()` accepts `&ActualState`, reads from it instead of querying system |
| `crates/iron-core/src/services/drift.rs` | `detect_package_drift()`, `detect_service_drift()`, `detect_config_drift()` accept `&ActualState` |
| `crates/iron-core/src/services/state.rs` | Add `state_dir()`, `state_root()`, `migrate_if_needed()`. Change internal paths. |
| `crates/iron-core/src/services/snapshot_service.rs` | Change `SNAPSHOTS_DIR` to use `state_dir()/snapshots/` |
| `crates/iron-core/src/services/mod.rs` | No changes needed (actual_state and envelope are at crate root, not in services/) |
| `crates/iron-cli/src/output.rs` | Add `json_envelope()` and `json_error_envelope()` methods |
| `crates/iron-cli/src/context.rs` | Update `is_initialized()` to check state_dir. Call `migrate_if_needed()` during construction. |
| `crates/iron-cli/src/cli.rs` | Add `Commands::Plan` variant, add `--full` and `--dry-run` to `Commands::Status` |
| `crates/iron-cli/src/commands/mod.rs` | Add `pub mod plan;` |
| `crates/iron-cli/src/commands/status.rs` | Enhance with package counts, security level, drift indicator, `--full` flag |
| `crates/iron-cli/src/commands/scan.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/module.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/security.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/snapshot.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/diff.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/apply.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/commands/validate.rs` | Migrate `output.json()` to `output.json_envelope()` |
| `crates/iron-cli/src/main.rs` | Wire `Commands::Plan` dispatch |

---

## 3. API Contracts

### 3.1 ActualState (F3-001)

File: `crates/iron-core/src/actual_state.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The current state of the system as queried from real sources.
/// Counterpart to DesiredState -- the plan is their diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualState {
    /// System hostname
    pub hostname: String,

    /// Explicitly installed packages (from pacman -Qqe equivalent)
    pub installed_packages: HashSet<String>,

    /// AUR/foreign packages (from pacman -Qqm equivalent)
    #[serde(default)]
    pub aur_packages: HashSet<String>,

    /// State of declared services
    #[serde(default)]
    pub services: Vec<ActualServiceState>,

    /// State of managed dotfiles/config files
    #[serde(default)]
    pub managed_files: Vec<ActualFileState>,

    /// When this snapshot was captured
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualServiceState {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualFileState {
    /// Target path (e.g., ~/.config/nvim/init.lua)
    pub target: String,
    /// Whether the file/symlink exists at target
    pub exists: bool,
    /// If symlink, where it points
    #[serde(default)]
    pub symlink_target: Option<String>,
    /// SHA256 of file content (regular files only)
    #[serde(default)]
    pub checksum: Option<String>,
    /// Type of entry at target path
    pub file_type: FileStateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileStateType {
    Symlink,
    Regular,
    Missing,
    Directory,
}

impl Default for FileStateType {
    fn default() -> Self {
        Self::Missing
    }
}
```

**Key design note:** `installed_packages` and `aur_packages` use `HashSet<String>` instead of `Vec<String>`. The primary consumer operation is membership testing (`actual.installed_packages.contains(&pkg)`), which is O(1) for HashSet vs O(n) for Vec. The technical guide specified `Vec<String>` but `HashSet` is the correct container for this access pattern. Serialization is transparent (serde handles HashSet as a JSON array).

### 3.2 ActualState::scan() (F3-002a)

File: `crates/iron-core/src/actual_state.rs`

```rust
use crate::packages::PackageManager;
use crate::system_service::SystemService;
use crate::IronResult;
use std::path::Path;

/// Specification for a file to check during scan.
/// Constructed from DesiredState dotfile mappings + service declarations.
pub struct ManagedFileSpec {
    /// The target path to check (e.g., ~/.config/nvim)
    pub target: String,
    /// The expected source for symlinks (for reference, not checked by scan)
    pub expected_source: Option<String>,
}

/// Specification for a service to check during scan.
pub struct ManagedServiceSpec {
    /// Service name (e.g., "bluetooth.service")
    pub name: String,
}

impl ActualState {
    /// Scan the system and capture all relevant state.
    ///
    /// This is the single source of system truth. Both `compute_plan()`
    /// and `detect()` consume this instead of querying independently.
    ///
    /// # Arguments
    /// * `package_manager` - trait object for querying installed packages
    /// * `service_manager` - trait object for querying service status
    /// * `managed_services` - list of service names to check
    /// * `managed_files` - list of file paths to check
    pub fn scan(
        package_manager: &dyn PackageManager,
        service_manager: &dyn SystemService,
        managed_services: &[ManagedServiceSpec],
        managed_files: &[ManagedFileSpec],
    ) -> IronResult<Self> {
        // Implementation queries package_manager, service_manager, filesystem
        // See section 6 for how this connects to consumers
        todo!()
    }

    /// Scan services for their enabled/running status.
    fn scan_services(
        service_manager: &dyn SystemService,
        services: &[ManagedServiceSpec],
    ) -> IronResult<Vec<ActualServiceState>> {
        todo!()
    }

    /// Scan managed files for existence, symlink targets, checksums.
    fn scan_files(files: &[ManagedFileSpec]) -> IronResult<Vec<ActualFileState>> {
        todo!()
    }

    /// Compute SHA256 checksum of a file.
    fn checksum_file(path: &Path) -> Option<String> {
        todo!()
    }
}
```

### 3.3 Modified compute_plan() and detect() (F3-002b)

File: `crates/iron-core/src/services/apply.rs`

The trait is UNCHANGED:

```rust
pub trait ApplyService {
    fn plan(&self, host_id: &str) -> IronResult<ApplyPlan>;
    fn plan_module(&self, module_id: &str) -> IronResult<ApplyPlan>;
    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult>;
    fn validate(&self, host_id: &str) -> IronResult<Vec<ValidationWarning>>;
}
```

The private method changes from:

```rust
// BEFORE
fn compute_plan(&self, desired: &DesiredState) -> IronResult<ApplyPlan>
```

to:

```rust
// AFTER
fn compute_plan(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<ApplyPlan>
```

The public `plan()` method orchestrates scanning:

```rust
impl ApplyService for DefaultApplyService {
    fn plan(&self, host_id: &str) -> IronResult<ApplyPlan> {
        let host = /* load host */;
        let desired = resolve_desired_state(&self.iron_root, &host)?;

        // NEW: Scan actual state once
        let managed_services: Vec<ManagedServiceSpec> = desired.services
            .iter()
            .map(|s| ManagedServiceSpec { name: s.clone() })
            .collect();
        let managed_files: Vec<ManagedFileSpec> = desired.dotfiles
            .iter()
            .map(|d| ManagedFileSpec {
                target: d.target.clone(),
                expected_source: Some(d.source.clone()),
            })
            .collect();

        let actual = ActualState::scan(
            self.package_manager.as_ref(),
            self.service_manager.as_ref(),
            &managed_services,
            &managed_files,
        )?;

        self.compute_plan(&desired, &actual)
    }
}
```

File: `crates/iron-core/src/services/drift.rs`

The trait is UNCHANGED:

```rust
pub trait DriftService {
    fn detect(&self, host_id: &str) -> IronResult<DriftReport>;
}
```

The private methods change to accept `&ActualState`:

```rust
// BEFORE
fn detect_package_drift(&self, desired: &DesiredState) -> IronResult<Vec<PackageDrift>>
fn detect_service_drift(&self, desired: &DesiredState) -> IronResult<Vec<ServiceDrift>>
fn detect_config_drift(&self, desired: &DesiredState) -> IronResult<Vec<ConfigDrift>>

// AFTER
fn detect_package_drift(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<Vec<PackageDrift>>
fn detect_service_drift(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<Vec<ServiceDrift>>
fn detect_config_drift(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<Vec<ConfigDrift>>
```

The public `detect()` method orchestrates scanning (same pattern as `plan()`):

```rust
impl DriftService for DefaultDriftService {
    fn detect(&self, host_id: &str) -> IronResult<DriftReport> {
        let host = /* load host */;
        let desired = resolve_desired_state(&self.iron_root, &host)?;

        // NEW: Scan actual state once
        let actual = ActualState::scan(
            self.package_manager.as_ref(),
            self.service_manager.as_ref(),
            &/* services from desired */,
            &/* files from desired */,
        )?;

        let package_drift = self.detect_package_drift(&desired, &actual)?;
        let service_drift = self.detect_service_drift(&desired, &actual)?;
        let config_drift = self.detect_config_drift(&desired, &actual)?;
        // ...
    }
}
```

### 3.4 IronEnvelope (F3-003a)

File: `crates/iron-core/src/envelope.rs`

```rust
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Standard response envelope for all --json CLI output.
///
/// Wraps command results with metadata, status, and error information.
/// All Iron CLI commands producing JSON output use this format.
#[derive(Debug, Clone)]
pub struct IronEnvelope<T> {
    pub ok: bool,
    pub command: String,
    pub data: Option<T>,
    pub error: Option<EnvelopeError>,
    pub meta: EnvelopeMeta,
}

/// Error detail in an envelope.
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Metadata attached to every envelope response.
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeMeta {
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    pub version: String,
}

impl<T: Serialize> Serialize for IronEnvelope<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("IronEnvelope", 5)?;
        state.serialize_field("ok", &self.ok)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("error", &self.error)?;
        state.serialize_field("meta", &self.meta)?;
        state.end()
    }
}

impl<T> IronEnvelope<T> {
    /// Create a success envelope wrapping data.
    pub fn success(command: &str, data: T, duration_ms: Option<u64>) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            data: Some(data),
            error: None,
            meta: EnvelopeMeta::now(duration_ms),
        }
    }
}

impl IronEnvelope<()> {
    /// Create an error envelope.
    pub fn error(command: &str, code: &str, message: &str, duration_ms: Option<u64>) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            data: None,
            error: Some(EnvelopeError {
                code: code.to_string(),
                message: message.to_string(),
                suggestion: None,
                details: None,
            }),
            meta: EnvelopeMeta::now(duration_ms),
        }
    }

    /// Create an error envelope with suggestion.
    pub fn error_with_suggestion(
        command: &str,
        code: &str,
        message: &str,
        suggestion: &str,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            data: None,
            error: Some(EnvelopeError {
                code: code.to_string(),
                message: message.to_string(),
                suggestion: Some(suggestion.to_string()),
                details: None,
            }),
            meta: EnvelopeMeta::now(duration_ms),
        }
    }
}

impl EnvelopeMeta {
    /// Create metadata with current timestamp.
    pub fn now(duration_ms: Option<u64>) -> Self {
        Self {
            timestamp: Utc::now(),
            duration_ms,
            host: gethostname::gethostname().to_string_lossy().into_owned().into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
```

**Note on `env!("CARGO_PKG_VERSION")`:** This compiles to the version of the crate where the macro is invoked. Since `envelope.rs` is in `iron-core`, it will use `iron-core`'s version. All workspace crates share the same version via `version.workspace = true`, so this is correct. If this is ever incorrect, it can be changed to accept a version parameter.

**Note on `host` field in EnvelopeMeta:** Since we already need `gethostname` for `ActualState::scan()`, we reuse it here. The `gethostname` dependency serves both purposes.

### 3.5 Output Envelope Methods (F3-003a CLI side)

File: `crates/iron-cli/src/output.rs`

```rust
use iron_core::envelope::IronEnvelope;
use serde::Serialize;
use std::time::Instant;

impl Output {
    /// Wrap data in a standard envelope and output as JSON.
    /// Only produces output when format is JSON.
    pub fn json_envelope<T: Serialize>(
        &self,
        command: &str,
        data: T,
        start_time: Instant,
    ) {
        if !self.is_json() {
            return;
        }
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let envelope = IronEnvelope::success(command, data, Some(duration_ms));
        if let Ok(json) = serde_json::to_string_pretty(&envelope) {
            println!("{}", json);
        }
    }

    /// Wrap an error in a standard envelope and output as JSON.
    /// Only produces output when format is JSON.
    pub fn json_error_envelope(
        &self,
        command: &str,
        code: &str,
        message: &str,
        start_time: Instant,
    ) {
        if !self.is_json() {
            return;
        }
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let envelope = IronEnvelope::<()>::error(command, code, message, Some(duration_ms));
        if let Ok(json) = serde_json::to_string_pretty(&envelope) {
            eprintln!("{}", json);
        }
    }
}
```

### 3.6 XDG State Directory Resolution (F3-006)

File: `crates/iron-core/src/services/state.rs`

```rust
impl StateManager {
    /// Resolve the state directory path.
    ///
    /// Priority:
    /// 1. `$IRON_STATE_DIR` environment variable (for testing and custom setups)
    /// 2. `$XDG_STATE_HOME/iron` (XDG standard)
    /// 3. `~/.local/state/iron` (XDG default fallback)
    ///
    /// Creates the directory if it does not exist.
    pub fn state_dir() -> PathBuf {
        let dir = if let Ok(dir) = std::env::var("IRON_STATE_DIR") {
            PathBuf::from(dir)
        } else if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            PathBuf::from(xdg).join("iron")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local/state/iron")
        };

        // Ensure directory exists
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Get the state directory this manager is using.
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// Get the config root (iron root) this manager was created with.
    pub fn config_root(&self) -> &Path {
        &self.root
    }
}
```

The `StateManager` struct gains a new field:

```rust
pub struct StateManager {
    /// Config root directory (e.g., ~/.config/iron) -- RENAMED from `root`
    root: PathBuf,
    /// State directory (e.g., ~/.local/state/iron) -- NEW
    state_root: PathBuf,
    /// In-memory state
    state: Arc<Mutex<IronState>>,
    /// Audit log entries
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}
```

The constructor changes internally but keeps the same signature:

```rust
impl StateManager {
    pub fn new(root: PathBuf) -> IronResult<Self> {
        let state_root = Self::state_dir();
        let state_path = state_root.join(STATE_FILE);
        // ... rest is same but uses state_root for all state file paths
    }
}
```

All internal methods using `self.root.join(STATE_FILE)`, `self.root.join(LOCK_FILE)`, `self.root.join(AUDIT_LOG_FILE)` change to use `self.state_root.join(...)`.

### 3.7 State Migration (F3-007)

File: `crates/iron-core/src/services/state.rs`

```rust
impl StateManager {
    /// Check for legacy state files in the config root and migrate
    /// them to the XDG state directory.
    ///
    /// Uses copy-then-delete for safety. On any failure, original
    /// files are left intact and a warning is logged.
    ///
    /// # No-op conditions
    /// - New state directory already has state.json
    /// - Legacy location has no state.json
    /// - MIGRATED.txt marker already exists in legacy location
    pub fn migrate_if_needed(config_root: &Path) -> IronResult<()> {
        let state_dir = Self::state_dir();
        let new_state_path = state_dir.join(STATE_FILE);
        let legacy_state_path = config_root.join(STATE_FILE);
        let migrated_marker = config_root.join("MIGRATED.txt");

        // No-op conditions
        if new_state_path.exists() {
            return Ok(());
        }
        if !legacy_state_path.exists() {
            return Ok(());
        }
        if migrated_marker.exists() {
            return Ok(());
        }

        // Ensure state directory exists
        std::fs::create_dir_all(&state_dir)?;

        // Copy state.json (not move -- copy first for safety)
        std::fs::copy(&legacy_state_path, &new_state_path)?;

        // Verify copy
        if !new_state_path.exists() {
            return Ok(()); // Copy failed silently, leave originals
        }

        // Migrate additional files (best-effort)
        Self::migrate_file(config_root, &state_dir, AUDIT_LOG_FILE);
        Self::migrate_file(config_root, &state_dir, LOCK_FILE);
        Self::migrate_dir(config_root, &state_dir, ".snapshots", "snapshots");

        // Remove originals after successful copy
        let _ = std::fs::remove_file(&legacy_state_path);
        let _ = std::fs::remove_file(config_root.join(AUDIT_LOG_FILE));
        let _ = std::fs::remove_file(config_root.join(LOCK_FILE));

        // Leave breadcrumb
        let _ = std::fs::write(
            &migrated_marker,
            format!(
                "State migrated to {} on {}",
                state_dir.display(),
                Utc::now()
            ),
        );

        Ok(())
    }

    /// Copy a single file from legacy to new location.
    fn migrate_file(from_dir: &Path, to_dir: &Path, filename: &str) {
        let src = from_dir.join(filename);
        let dst = to_dir.join(filename);
        if src.exists() && !dst.exists() {
            let _ = std::fs::copy(&src, &dst);
        }
    }

    /// Copy a directory from legacy to new location (rename allowed).
    fn migrate_dir(from_dir: &Path, to_dir: &Path, old_name: &str, new_name: &str) {
        let src = from_dir.join(old_name);
        let dst = to_dir.join(new_name);
        if src.is_dir() && !dst.exists() {
            let _ = Self::copy_dir_recursive(&src, &dst);
        }
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dest_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
}
```

### 3.8 CLI Commands -- Plan (F3-005)

File: `crates/iron-cli/src/cli.rs`

```rust
// Add to Commands enum:

/// Preview what iron apply would do (read-only, no confirmation)
Plan {
    /// Show plan for a specific module only
    #[arg(short, long)]
    module: Option<String>,

    /// Dry run (for testing -- no system queries)
    #[arg(long)]
    dry_run: bool,
},
```

File: `crates/iron-cli/src/commands/plan.rs`

The command resolves desired state, scans actual state, computes plan, and displays it. Uses tree output for text mode, envelope for JSON mode.

### 3.9 CLI Commands -- Status Enhancement (F3-004)

File: `crates/iron-cli/src/cli.rs`

```rust
// Change Commands::Status from unit variant to struct variant:

/// Show system status overview
Status {
    /// Full status with ActualState scan (slower, more accurate)
    #[arg(long)]
    full: bool,

    /// Dry run (for testing)
    #[arg(long)]
    dry_run: bool,
},
```

### 3.10 AppContext Update (F3-006/F3-007)

File: `crates/iron-cli/src/context.rs`

```rust
impl AppContext {
    pub fn new(/* same params */) -> Result<Self> {
        let root = expand_home(Path::new(root));

        // F3-007: Migrate legacy state if needed
        StateManager::migrate_if_needed(&root)
            .context("State migration warning")?;

        // StateManager::new() now internally resolves state_dir()
        let state = StateManager::new(root.clone())
            .context("Failed to initialize state manager")?;

        // ... rest unchanged
    }

    pub fn is_initialized(&self) -> bool {
        // F3-006: Check state directory instead of config root
        self.state.state_root().join("state.json").exists()
            || self.state.current_host().is_some()
    }
}
```

---

## 4. Dependency Graph

### Crate Dependencies

```
iron-core (existing deps: chrono, dirs, serde, serde_json, ...)
  + gethostname = "0.2"    -- for ActualState::scan() hostname + EnvelopeMeta
  + sha2 = "0.10"          -- for ActualState file checksum computation

iron-cli (unchanged external deps)
  iron-core                -- gains access to actual_state, envelope modules

iron-tui (unchanged external deps)
  iron-core                -- gains access to actual_state, envelope modules
```

### Internal Module Dependencies

```
actual_state.rs
  depends on: packages::PackageManager, system_service::SystemService, IronResult
  depended on by: services/apply.rs, services/drift.rs

envelope.rs
  depends on: chrono, serde, serde_json, gethostname
  depended on by: iron-cli/output.rs, iron-cli/commands/*.rs

services/state.rs
  depends on: dirs (existing), chrono (existing)
  depended on by: all services, iron-cli/context.rs, iron-tui/app/mod.rs

services/apply.rs
  depends on: actual_state::ActualState (NEW)

services/drift.rs
  depends on: actual_state::ActualState (NEW)
```

---

## 5. Migration Strategy

### Phase-by-Phase Implementation (preserving green tests at each step)

**Wave 1: Pure Additions (tests stay green)**

1. **F3-001**: Create `actual_state.rs` with structs. Add `pub mod actual_state;` to `lib.rs`. Add re-exports. No existing code modified. Run `cargo test --workspace` -- all green.

2. **F3-003a**: Create `envelope.rs` with structs. Add `pub mod envelope;` to `lib.rs`. Add `json_envelope()` and `json_error_envelope()` to `output.rs`. No existing methods removed. Run `cargo test --workspace` -- all green.

3. **F3-006**: Add `state_root` field to `StateManager`. Add `state_dir()` method. Change internal path resolution. Update `is_initialized()` in `context.rs`. **Critical step**: all existing tests using `StateManager::new(temp_dir)` must still work. Solution: When `$IRON_STATE_DIR` is not set and no XDG vars are set, the state_dir defaults to `~/.local/state/iron`. For tests, set `$IRON_STATE_DIR` to the temp directory. However, to avoid breaking ALL existing tests at once, the `StateManager::new()` method should have a fallback: if the resolved state_dir cannot be created OR has no state.json but `config_root/state.json` exists, use config_root as state_dir (legacy mode). This means tests continue working without env var changes, and migration handles the transition for real users.

    **Alternative (simpler)**: Since tests use `TempDir` paths with no real XDG dirs, and `StateManager::new()` currently writes to `root.join("state.json")`, we can make the state_dir fallback to `config_root` when `$IRON_STATE_DIR` is not set AND `$XDG_STATE_HOME` is not set AND the state_dir does not yet exist but `config_root/state.json` does exist. This preserves backward compatibility during the transition.

    **Chosen approach**: In `StateManager::new()`, try `state_dir()` first. If `state_dir/state.json` does not exist but `config_root/state.json` does exist, use `config_root` as the effective state root (pre-migration mode). This makes F3-006 safe to deploy before F3-007. Existing tests all use `TempDir` with state.json in the same directory, so they work in pre-migration mode.

4. **F3-002a**: Add `ActualState::scan()` implementation. Add `gethostname` and `sha2` to `Cargo.toml`. Pure addition. Run `cargo test --workspace` -- all green.

**Wave 2: After F3-006**

5. **F3-007**: Add `migrate_if_needed()`. Call from `AppContext::new()`. Pure addition -- the method is no-op when new location already has state or legacy location has no state. Run `cargo test --workspace` -- all green.

**Wave 3: Internal Refactor (most existing tests unchanged)**

6. **F3-002b**: Change `compute_plan()` to accept `&ActualState`. Change `detect_*` methods to accept `&ActualState`. The public `plan()` and `detect()` methods now scan ActualState internally. Since the trait signatures are unchanged, all callers (CLI, TUI, tests using trait objects) continue to work. Only unit tests within `apply.rs` and `drift.rs` that directly test `compute_plan()` logic need updating. Run `cargo test --workspace` -- fix any failures in apply/drift unit tests.

**Wave 4: Mechanical Migration**

7. **F3-003b**: Replace `output.json(&data)` with `output.json_envelope("command", data, start_time)` in each command file. Add `let start_time = Instant::now();` at the top of each command function. Mechanical, low-risk.

8. **F3-004**: Enhance `status.rs` with new fields, `--full` flag. Add `--dry-run` flag. Update `Commands::Status` in `cli.rs`.

9. **F3-005**: Create `plan.rs`. Register in `mod.rs`, `cli.rs`, `main.rs`.

### Handling the Status Unit Variant to Struct Variant Change

Changing `Commands::Status` from a unit variant to a struct variant (`Status { full: bool, dry_run: bool }`) is a breaking change for the `main.rs` match arm. The fix is trivial: `Commands::Status => ...` becomes `Commands::Status { full, dry_run } => ...`. But it must be done atomically with the status.rs changes.

---

## 6. Component Diagram

```
                    CLI Layer (iron-cli)
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    │  commands/apply.rs ──┐                  │
                    │  commands/diff.rs  ──┤                  │
                    │  commands/plan.rs  ──┤  output.rs       │
                    │  commands/status.rs ─┤  ├─json_envelope │
                    │  commands/*.rs ──────┘  └─json_error_.. │
                    │                                         │
                    │  context.rs                              │
                    │  ├─ calls migrate_if_needed() on init   │
                    │  ├─ constructs StateManager              │
                    │  └─ is_initialized() checks state_dir   │
                    └──────────────┬──────────────────────────┘
                                   │ uses
                    ┌──────────────▼──────────────────────────┐
                    │           Core Layer (iron-core)         │
                    │                                         │
                    │  actual_state.rs  ◄────────────────┐    │
                    │  ├─ ActualState                    │    │
                    │  ├─ ActualServiceState             │    │
                    │  ├─ ActualFileState                │    │
                    │  └─ scan() ◄── PackageManager      │    │
                    │              ◄── SystemService      │    │
                    │                                    │    │
                    │  services/apply.rs                 │    │
                    │  ├─ plan() ──── scan() ────────────┘    │
                    │  └─ compute_plan(desired, actual)        │
                    │                                    │    │
                    │  services/drift.rs                 │    │
                    │  ├─ detect() ── scan() ────────────┘    │
                    │  └─ detect_*_drift(desired, actual)      │
                    │                                         │
                    │  envelope.rs                             │
                    │  ├─ IronEnvelope<T>                      │
                    │  ├─ EnvelopeError                        │
                    │  └─ EnvelopeMeta                         │
                    │                                         │
                    │  services/state.rs                       │
                    │  ├─ StateManager                         │
                    │  │  ├─ state_dir() ── $IRON_STATE_DIR   │
                    │  │  │              ── $XDG_STATE_HOME    │
                    │  │  │              ── ~/.local/state     │
                    │  │  ├─ state_root() → state dir path    │
                    │  │  ├─ config_root() → config dir path  │
                    │  │  └─ migrate_if_needed()               │
                    │  └─ (state.json, audit.log, .state.lock) │
                    │      now in state_dir, not config_root   │
                    │                                         │
                    │  services/snapshot_service.rs             │
                    │  └─ snapshots/ now in state_dir          │
                    └─────────────────────────────────────────┘

Flow: plan(host_id)
  1. Load Host TOML from config_root/hosts/
  2. resolve_desired_state() → DesiredState (packages, dotfiles, services)
  3. ActualState::scan(pkg_mgr, svc_mgr, services, files) → ActualState
  4. compute_plan(desired, actual) → ApplyPlan
  5. Display or execute plan

Flow: detect(host_id)
  1. Load Host TOML
  2. resolve_desired_state() → DesiredState
  3. ActualState::scan(...) → ActualState
  4. detect_*_drift(desired, actual) → DriftReport

Flow: status
  1. Read state.json from state_dir → host, bundle, profile, modules
  2. resolve_desired_state() → package/service/dotfile counts
  3. SecurityService::calculate() → security level
  4. (optional --full) ActualState::scan() → accurate system counts
  5. Display status

Flow: JSON output (any command)
  1. let start_time = Instant::now();
  2. Execute command logic → data
  3. output.json_envelope("command", data, start_time)
  4. → IronEnvelope::success() → serde_json → stdout
```

---

## 7. Boundaries

- **In scope**: ActualState struct + scan, envelope infrastructure + migration, XDG state separation + migration, status enhancement, plan command, all for Sprint 3.1 only.
- **Out of scope**: Template rendering in apply (Sprint 3.2), package/service/symlink removal (Sprint 3.2), hooks (Sprint 3.3), plan serialization/replay (Phase 4), managed resource tracking (Sprint 3.2).
- **Extension points**:
  - `ActualState` is `Serialize + Deserialize`, ready for `iron scan --save` (F3-022 stretch).
  - `IronEnvelope<T>` can wrap any serializable type, including future history/comparison responses.
  - `state_dir()` supports `$IRON_STATE_DIR` override for testing and custom deployments.
  - `compute_plan()` and `detect_*_drift()` now accept `&ActualState`, making it straightforward to add new drift categories or action types in Sprint 3.2 by extending the ActualState scan.
