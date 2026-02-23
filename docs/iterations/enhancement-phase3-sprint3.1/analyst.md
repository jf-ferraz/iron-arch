# Analyst Report -- Sprint 3.1 (Foundation Contracts)

**Date:** 2026-02-22
**Type:** ENHANCEMENT (structural)
**Sprint:** 3.1 -- Foundation Contracts
**Tasks:** F3-001, F3-002a, F3-002b, F3-003a, F3-003b, F3-004, F3-005, F3-006, F3-007

---

## 1. Scope Verification

### Task-by-Task Scope Audit

Each Sprint 3.1 task was verified against the current codebase. Key corrections from the orchestrator/kanban assumptions are noted.

**F3-001: ActualState struct and contract**
- Scope confirmed. No `actual_state.rs` exists. New file at `iron-core/src/actual_state.rs`.
- The kanban specifies fields: `hostname`, `installed_packages`, `aur_packages`, `services: Vec<ActualServiceState>`, `managed_files: Vec<ActualFileState>`, `scanned_at`.
- No conflicts with existing structs. `DesiredState` is defined in `iron-core/src/services/apply.rs` (lines 29-46) and is purely a resolution artifact -- `ActualState` is its system-side counterpart.

**F3-002a: scan_actual_state() implementation**
- Scope confirmed. `ActualState::scan()` will use `PackageManager::query_installed()`, `SystemService::is_enabled()`, and filesystem queries.
- Current ad-hoc queries to replicate and centralize are found in:
  - `apply.rs` lines 587-593 (package query), 620-659 (symlink checks), 664 (service checks)
  - `drift.rs` lines 175-181 (package query), 226 (service checks), 243-295 (config drift checks)
- Needs `sha2` crate for file checksums (not currently a dependency of iron-core).
- Needs `gethostname` crate (or equivalent) for hostname field (not currently a dependency).

**F3-002b: Refactor compute_plan/detect consumers**
- SCOPE CORRECTION: `compute_plan()` is a **private method** on `DefaultApplyService` (line 583 of apply.rs), not a method on the `ApplyService` trait. The public trait methods are `plan(&self, host_id: &str)` and `plan_module(&self, module_id: &str)`.
- The refactor has two options: (a) change `compute_plan()` internal signature only, keeping trait methods unchanged, or (b) add `&ActualState` to the trait methods as well. Option (a) reduces cascade. Option (b) gives callers control over scanning. This is an **architectural decision** for the architect to resolve.
- For `DriftService`, `detect(&self, host_id: &str)` is on the trait (line 115 of drift.rs). Changing this signature affects all consumers.
- SCOPE REDUCTION: The TUI does **not** directly call `.plan()` or `.detect()`. Grep across `crates/iron-tui/src/` returned zero hits. The TUI uses `app.apply_plan_count` and dispatches through `App` methods that construct services internally. This means the TUI cascade is contained to `app/actions.rs` and `app/mod.rs` service construction, not the UI views.
- `snapshot_service.rs` also does **not** call `.plan()` or `.detect()` directly.

**F3-003a: Response envelope infrastructure**
- Scope confirmed. No `envelope.rs` exists. New file at `iron-core/src/envelope.rs`.
- `Output::json()` in `output.rs` (line 222-228) currently calls `serde_json::to_string_pretty()` directly with no wrapping. New methods `json_envelope()` and `json_error_envelope()` are needed.
- `chrono` is already a dependency (timestamps for `EnvelopeMeta`).

**F3-003b: Migrate existing --json to envelope**
- Scope confirmed. Commands currently using `output.json()` for structured output:
  - `commands/status.rs` (line 194)
  - `commands/scan.rs`, `commands/module.rs`, `commands/security.rs`, `commands/snapshot.rs`, `commands/diff.rs`, `commands/apply.rs`, `commands/validate.rs`
- Each call site needs migration from `output.json(&data)` to `output.json_envelope("command_name", &data)`.

**F3-004: iron status command -- CORRECTION**
- IMPORTANT FINDING: `iron status` **already exists** as a fully implemented command in `crates/iron-cli/src/commands/status.rs` (276 lines). The kanban says "File: iron-cli/src/commands/status.rs (new)" -- this is incorrect.
- The existing command shows: host, bundle, profile, modules (active/available), sync status, secrets status, services availability.
- The existing command has JSON output via `output.json(&data)` (not using envelope).
- The existing `Commands::Status` variant in `cli.rs` (line 78-79) has **no flags** -- no `--full`, `--json` (uses global format), `--dry-run`.
- What is MISSING from the existing command: managed package counts, security level/score, last apply timestamp, last sync timestamp, drift indicator, `--full` flag for full ActualState scan.
- This task should be scoped as an **enhancement of the existing command**, not a creation from scratch.

**F3-005: iron plan command**
- Scope confirmed. No `plan.rs` exists in `commands/`. No `Plan` variant in the `Commands` enum.
- New file at `iron-cli/src/commands/plan.rs`.
- Needs registration in `commands/mod.rs` and `cli.rs`.

**F3-006: XDG state directory separation**
- Scope confirmed. `StateManager::new(root: PathBuf)` (line 103 of state.rs) uses `root` directly as the state directory. `state_path()` returns `self.root.join(STATE_FILE)` (line 142-144). Lock file at `self.root.join(LOCK_FILE)` (line 417-419). Audit log at `self.root.join(AUDIT_LOG_FILE)` (line 586).
- The `dirs` crate is already a dependency of iron-core (`dirs = "5.0"` in Cargo.toml). No new dependency needed.
- `AppContext::is_initialized()` in `context.rs` (line 167) checks `self.root.join("state.json")` -- this **will break** after state.json moves to XDG state dir. Must be updated.

**F3-007: Legacy state migration**
- Scope confirmed. No migration logic exists in `state.rs`.
- Files to migrate: `state.json`, `audit.log`, `.state.lock`, `.snapshots/` directory.
- `SnapshotService` in `snapshot_service.rs` stores snapshots in the iron root's `.snapshots/` directory -- this path must also be updated for F3-006.

---

## 2. Codebase Impact Analysis

### New Files

| File | Task | Content |
|------|------|---------|
| `iron-core/src/actual_state.rs` | F3-001, F3-002a | `ActualState`, `ActualServiceState`, `ActualFileState`, `FileStateType`, `ActualState::scan()` |
| `iron-core/src/envelope.rs` | F3-003a | `IronEnvelope<T>`, `EnvelopeError`, `EnvelopeMeta`, constructors |
| `iron-cli/src/commands/plan.rs` | F3-005 | `iron plan` and `iron plan --module <id>` command |

### Modified Files (by task)

**F3-001:**
- `iron-core/src/lib.rs` -- add `pub mod actual_state;` registration

**F3-002a:**
- `iron-core/Cargo.toml` -- add `gethostname` and `sha2` dependencies

**F3-002b (highest cascade):**
- `iron-core/src/services/apply.rs` -- `compute_plan()` signature: add `&ActualState` param, remove inline package/symlink/service queries
- `iron-core/src/services/drift.rs` -- `detect()` internal methods: replace inline system queries with `ActualState` field reads. Trait signature change depends on architect decision.
- `iron-cli/src/commands/apply.rs` -- add `ActualState::scan()` call before `service.plan()`, pass through to service
- `iron-cli/src/commands/diff.rs` -- add `ActualState::scan()` call before `service.detect()`, pass through to service
- `iron-tui/src/app/actions.rs` -- update service construction/invocation to scan ActualState first
- `iron-tui/src/app/mod.rs` -- may need `ActualState` field or scan method on App
- Integration tests: `iron-core/tests/scan_integration.rs`, `iron-cli/tests/cli_integration.rs`
- Unit tests in `apply.rs` and `drift.rs` -- all tests constructing `DefaultApplyService` and calling `plan()`/`detect()` must be updated

**F3-003a:**
- `iron-core/src/lib.rs` -- add `pub mod envelope;` registration
- `iron-cli/src/output.rs` -- add `json_envelope()` and `json_error_envelope()` methods

**F3-003b:**
- `iron-cli/src/commands/status.rs` -- migrate `output.json(&data)` to envelope
- `iron-cli/src/commands/scan.rs` -- migrate JSON output
- `iron-cli/src/commands/module.rs` -- migrate JSON output
- `iron-cli/src/commands/security.rs` -- migrate JSON output
- `iron-cli/src/commands/snapshot.rs` -- migrate JSON output
- `iron-cli/src/commands/diff.rs` -- migrate JSON output
- `iron-cli/src/commands/apply.rs` -- migrate JSON output
- `iron-cli/src/commands/validate.rs` -- migrate JSON output

**F3-004:**
- `iron-cli/src/commands/status.rs` -- enhance existing command (add managed counts, security level, timestamps, drift indicator, `--full` flag)
- `iron-cli/src/cli.rs` -- add flags to `Commands::Status` variant (`--full`, `--dry-run`)
- `iron-cli/src/main.rs` -- update status dispatch if needed for new flags

**F3-005:**
- `iron-cli/src/commands/mod.rs` -- add `pub mod plan;`
- `iron-cli/src/cli.rs` -- add `Commands::Plan` variant with `--module`, `--json`, `--dry-run` flags
- `iron-cli/src/main.rs` -- add plan dispatch

**F3-006:**
- `iron-core/src/services/state.rs` -- add `state_dir()` method, change `StateManager::new()` to use it, update lock/audit paths
- `iron-core/src/services/snapshot_service.rs` -- update snapshot directory from `iron_root/.snapshots/` to `state_dir()/snapshots/`
- `iron-cli/src/context.rs` -- update `is_initialized()` check (line 167) to use state_dir, update `StateManager::new()` call (line 43-44)

**F3-007:**
- `iron-core/src/services/state.rs` -- add `migrate_if_needed(config_root)` method
- `iron-cli/src/context.rs` or `main.rs` -- call migration on startup

### Test Helpers Requiring Updates

- `iron-core/src/test_helpers.rs` -- `TestModule::to_module()` (line 274-292) does not need changes for Sprint 3.1 (no new Module fields this sprint). However, tests constructing `DefaultApplyService` and `DefaultDriftService` will need updating for F3-002b.
- Host struct tests in `iron-core/src/host.rs` (line 167-213) -- no changes needed this sprint (no new Host fields).
- Tests constructing `StateManager::new(root)` -- may need updating for F3-006 if the constructor signature changes.

---

## 3. Dependency Validation

### Crate Dependencies

| Dependency | Crate | Status | Task |
|-----------|-------|--------|------|
| `chrono` | iron-core | Already present | F3-003a (EnvelopeMeta timestamp) |
| `dirs = "5.0"` | iron-core | Already present | F3-006 (XDG resolution) |
| `serde`, `serde_json` | iron-core | Already present | F3-001, F3-003a |
| `gethostname` | iron-core | **NEEDS ADDING** | F3-002a (system hostname) |
| `sha2` | iron-core | **NEEDS ADDING** | F3-002a (file checksums) |

### Internal Dependencies (Task Ordering)

```
Independent starts (no blockers):
  F3-001  -- no dependencies
  F3-003a -- no dependencies
  F3-006  -- no dependencies

After F3-001:
  F3-002a -- depends on ActualState struct

After F3-006:
  F3-007  -- depends on state_dir() existing

After F3-002a:
  F3-002b -- depends on scan() implementation

After F3-003a:
  F3-003b -- depends on envelope infrastructure

After F3-002b AND F3-003a:
  F3-004  -- depends on ActualState consumer refactor + envelope
  F3-005  -- depends on ActualState consumer refactor + envelope
```

### Trait Bound Requirements

Per lesson L8: all new traits must include `Send + Sync` bounds from day 1.
- `ActualState` -- no trait needed (concrete struct), but must be `Send + Sync` for TUI thread safety. Ensure all fields are `Send + Sync` (they will be, since they're String/Vec/bool/Option primitives).
- `IronEnvelope<T>` -- generic struct, needs `T: Serialize` bound. Should also derive `Clone` for flexibility.

---

## 4. Risk Assessment

### High Risk

**F3-002b: Consumer refactor cascade** (Risk: HIGH)
- This is the riskiest task in the sprint. It changes internal method signatures that propagate through CLI and TUI layers.
- Mitigation: Keep trait method signatures unchanged if possible (architect decision). Only change `compute_plan()` and `detect()`'s internal private methods. This contains the cascade to within `DefaultApplyService` and `DefaultDriftService`.
- Mitigation: Implement F3-002b last among the core tasks so all tests are green before the cascade begins.
- Regression surface: All apply and drift tests, CLI `apply`/`diff` commands, TUI apply flow.

**F3-006: State directory separation** (Risk: HIGH)
- Moving state files changes a fundamental assumption (state lives in config root) that permeates `StateManager`, `AppContext`, `SnapshotService`, and potentially any code that does `iron_root.join("state.json")`.
- `AppContext::is_initialized()` (context.rs:167) checks `self.root.join("state.json")` -- this will fail after migration.
- Mitigation: `$IRON_STATE_DIR` env var override allows tests to control state location without XDG.
- Mitigation: F3-007 migration ensures existing installations do not break.

### Medium Risk

**F3-003b: Envelope migration** (Risk: MEDIUM)
- Changing JSON output format is a breaking change for any tooling or scripts consuming `iron --json` output.
- Mitigation: The `data` field in the envelope contains the same payload as before -- tools accessing `data.*` fields can adapt.
- Mitigation: Document the format change prominently.

**F3-004: Status enhancement** (Risk: MEDIUM)
- The existing `iron status` command works. Enhancing it risks breaking current behavior.
- Mitigation: Add new fields additively. Existing JSON fields should remain under `data.*` in the envelope.
- The `--full` flag performance risk: full `ActualState::scan()` could be slow on systems with many packages. Mitigation: document that `--full` may take several seconds.

### Low Risk

**F3-001, F3-002a, F3-003a** (Risk: LOW)
- Pure additions with no modification to existing code. Self-contained new files with unit tests.

**F3-005, F3-007** (Risk: LOW)
- F3-005 is a new read-only command. No side effects.
- F3-007 is a migration utility with copy-then-delete safety and multiple no-op conditions.

---

## 5. Implementation Order

The recommended implementation order, accounting for dependencies and risk containment:

### Wave 1 (Parallel, No Dependencies)

| Task | Rationale |
|------|-----------|
| F3-001 | Foundation struct, blocks F3-002a. Pure addition. |
| F3-003a | Envelope infra, blocks F3-003b/F3-004/F3-005. Pure addition. |
| F3-006 | XDG separation, blocks F3-007. Isolated change to state.rs and context.rs. |

### Wave 2 (After Wave 1 Completes)

| Task | Rationale |
|------|-----------|
| F3-002a | Depends on F3-001. Implements scan(). Pure addition. |
| F3-007 | Depends on F3-006. Small, isolated migration logic. |

### Wave 3 (After F3-002a Completes)

| Task | Rationale |
|------|-----------|
| F3-002b | Depends on F3-002a. Highest-risk cascade -- do alone for clear regression attribution. |

### Wave 4 (After F3-002b and F3-003a Complete)

| Task | Rationale |
|------|-----------|
| F3-003b | Depends on F3-003a. Mechanical migration, low risk. |
| F3-004 | Depends on F3-002b + F3-003a. Enhancement of existing command. |
| F3-005 | Depends on F3-002b + F3-003a. New command. |

---

## 6. Test Strategy Notes

### Per-Task Test Requirements

**F3-001 (ActualState struct):**
- Unit tests: construction with all fields, `#[serde(default)]` deserialization with missing optional fields, serialization roundtrip.
- Minimum: 4 tests.

**F3-002a (scan implementation):**
- Unit tests with mocked `PackageManager` and `SystemService` (via existing trait abstractions).
- Test scan with: no packages installed, some packages installed, AUR packages, service enabled/disabled, symlink exists/missing/wrong-target, regular file with checksum, missing file.
- Serialization roundtrip: scan -> JSON -> deserialize -> assert fields match.
- Minimum: 8 tests.

**F3-002b (consumer refactor):**
- All existing tests in `apply.rs` and `drift.rs` must pass after refactor with updated signatures.
- New test: verify `compute_plan()` reads from `ActualState` fields, not from `package_manager.query_installed()` internally.
- New test: verify `detect()` reads from `ActualState` fields.
- Integration test: `iron apply --dry-run` still works end-to-end.
- Integration test: `iron diff --dry-run` still works end-to-end.
- Minimum: all existing tests pass + 4 new tests.

**F3-003a (envelope infrastructure):**
- Unit tests: `IronEnvelope::success()` populates `ok: true`, `data: Some(T)`, `error: None`.
- Unit tests: `IronEnvelope::error()` populates `ok: false`, `data: None`, `error: Some(...)`.
- Unit tests: `EnvelopeMeta` has non-empty `timestamp`, `version`.
- Serialization test: envelope serializes to JSON with expected top-level keys.
- Minimum: 6 tests.

**F3-003b (envelope migration):**
- Integration tests: at least 3 commands verified with `--json` output containing envelope fields (`ok`, `command`, `data`, `meta`).
- Recommended commands for integration testing: `iron diff --json --dry-run`, `iron scan --json`, `iron security --json --dry-run`.
- Minimum: 3 integration tests.

**F3-004 (iron status enhancement):**
- CLI parsing tests: `--full` flag, `--dry-run` flag.
- Integration test: `iron status --dry-run` returns without error.
- Integration test: `iron status --json --dry-run` returns valid envelope.
- Unit test: status data struct populates all expected fields from state.
- Minimum: 4 tests.

**F3-005 (iron plan command):**
- CLI parsing tests: `--module <id>`, `--json`, `--dry-run` flags.
- Integration test: `iron plan --dry-run` returns without error.
- Integration test: `iron plan --json --dry-run` returns valid envelope.
- Minimum: 3 tests.

**F3-006 (XDG state directory):**
- Unit tests: resolution priority -- `$IRON_STATE_DIR` overrides XDG, XDG overrides default.
- Unit test: `state_dir()` creates directory if it does not exist.
- Unit test: `StateManager::new()` uses state_dir for state.json path.
- Integration test: set `$IRON_STATE_DIR` to temp dir, verify state operations work.
- Minimum: 4 tests.

**F3-007 (legacy migration):**
- Unit test: migration from legacy location copies files and creates MIGRATED.txt.
- Unit test: no-op when new location already has state.json.
- Unit test: no-op when legacy location has no state.json.
- Unit test: failure recovery -- original files left intact on copy failure.
- Minimum: 4 tests.

### Test Infrastructure Notes

- All new CLI commands must include `--dry-run` flag per lesson L2 (prevents sudo prompts and TUI hangs in integration tests).
- Use `$IRON_STATE_DIR` env var in tests to isolate state directory (avoids polluting real XDG dirs).
- Mock-based unit tests for `ActualState::scan()` use the existing `PackageManager` and `SystemService` trait abstractions -- no new mock infrastructure needed.
- F3-002b tests should use a helper function to construct `ActualState` for test scenarios (empty system, partial install, fully converged).

---

## 7. Acceptance Criteria

### F3-001: ActualState struct and contract

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-1.1 | `ActualState` struct exists in `iron-core/src/actual_state.rs` with fields: `hostname: String`, `installed_packages: HashSet<String>`, `aur_packages: HashSet<String>`, `services: Vec<ActualServiceState>`, `managed_files: Vec<ActualFileState>`, `scanned_at: DateTime<Utc>` | Code review |
| AC-1.2 | `ActualServiceState` has fields: `name: String`, `enabled: bool`, `running: bool` | Code review |
| AC-1.3 | `ActualFileState` has fields: `target: String`, `exists: bool`, `symlink_target: Option<String>`, `checksum: Option<String>`, `file_type: FileStateType` | Code review |
| AC-1.4 | `FileStateType` enum: `Symlink`, `Regular`, `Missing`, `Directory` | Code review |
| AC-1.5 | All structs derive `Debug, Clone, Serialize, Deserialize` | Code review |
| AC-1.6 | Optional fields use `#[serde(default)]` | Code review |
| AC-1.7 | Module registered in `iron-core/src/lib.rs` as `pub mod actual_state` | Code review |
| AC-1.8 | Unit tests pass: construction, serde roundtrip, missing-field deserialization | `cargo test -p iron-core actual_state` |

### F3-002a: scan_actual_state() implementation

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-2a.1 | `ActualState::scan()` accepts `&dyn PackageManager`, `&dyn SystemService`, `&[String]` (managed file paths) | Code review |
| AC-2a.2 | Queries `PackageManager::query_installed()` for installed packages | Unit test with mock |
| AC-2a.3 | Queries `SystemService::is_enabled()` for each declared service | Unit test with mock |
| AC-2a.4 | Checks `std::fs::read_link()` and `std::fs::metadata()` for managed files | Unit test with temp files |
| AC-2a.5 | Computes SHA256 checksums for regular files (not symlinks) | Unit test |
| AC-2a.6 | Populates `hostname` via `gethostname` crate | Unit test |
| AC-2a.7 | `scanned_at` set to current UTC time | Unit test |
| AC-2a.8 | Serialization roundtrip: scan result serializes to JSON and deserializes back correctly | Unit test |
| AC-2a.9 | `gethostname` and `sha2` added to `iron-core/Cargo.toml` | Dependency check |

### F3-002b: Refactor compute_plan/detect consumers

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-2b.1 | `DefaultApplyService::compute_plan()` accepts `&ActualState` parameter | Code review |
| AC-2b.2 | `compute_plan()` reads `actual_state.installed_packages` instead of calling `self.package_manager.query_installed()` | Code review: no `query_installed()` call inside compute_plan |
| AC-2b.3 | `compute_plan()` reads `actual_state.managed_files` for symlink checks instead of inline `std::fs::read_link()` | Code review |
| AC-2b.4 | `compute_plan()` reads `actual_state.services` for service checks instead of calling `self.service_manager.is_enabled()` | Code review |
| AC-2b.5 | `DriftService::detect()` (trait or internal) uses `&ActualState` instead of ad-hoc queries | Code review |
| AC-2b.6 | CLI `commands/apply.rs` calls `ActualState::scan()` before invoking plan | Code review |
| AC-2b.7 | CLI `commands/diff.rs` calls `ActualState::scan()` before invoking detect | Code review |
| AC-2b.8 | TUI apply flow updated to scan ActualState | Code review of `app/actions.rs` |
| AC-2b.9 | All existing tests pass | `cargo test --workspace` |
| AC-2b.10 | No new `#[allow(dead_code)]` annotations | `grep -rn "allow(dead_code)" crates/` -- count unchanged |

### F3-003a: Response envelope infrastructure

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-3a.1 | `IronEnvelope<T>` struct with fields: `ok: bool`, `command: String`, `data: Option<T>`, `error: Option<EnvelopeError>`, `meta: EnvelopeMeta` | Code review |
| AC-3a.2 | `EnvelopeError` with fields: `code: String`, `message: String`, `suggestion: Option<String>`, `details: Option<String>` | Code review |
| AC-3a.3 | `EnvelopeMeta` with fields: `timestamp: String`, `duration_ms: Option<u64>`, `host: Option<String>`, `version: String` | Code review |
| AC-3a.4 | `IronEnvelope::success(command, data)` constructor | Code review |
| AC-3a.5 | `IronEnvelope::error(command, error)` constructor | Code review |
| AC-3a.6 | `Output::json_envelope()` method in `output.rs` | Code review |
| AC-3a.7 | `Output::json_error_envelope()` method in `output.rs` | Code review |
| AC-3a.8 | Module registered in `iron-core/src/lib.rs` as `pub mod envelope` | Code review |
| AC-3a.9 | Unit tests pass: success envelope, error envelope, serialization, meta fields | `cargo test -p iron-core envelope` |

### F3-003b: Migrate existing --json to envelope

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-3b.1 | All CLI commands with `--json` output use `output.json_envelope()` or `output.json_error_envelope()` | Code review of all `commands/*.rs` |
| AC-3b.2 | All JSON output includes top-level `ok`, `command`, `data`, `meta` fields | Integration test: parse JSON output |
| AC-3b.3 | Error JSON includes `error.code`, `error.message` | Integration test: trigger error with --json |
| AC-3b.4 | `meta.timestamp` populated with ISO-8601 string | Integration test |
| AC-3b.5 | No commands use raw `output.json()` for structured command output | Grep: no remaining `output.json(` calls in command files (utility/debug uses excepted) |

### F3-004: iron status enhancement

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-4.1 | Shows active host, bundle, profile, module count (preserves existing behavior) | Integration test: `iron status --dry-run` |
| AC-4.2 | Shows managed package count from state data | Integration test |
| AC-4.3 | Shows security level and score | Integration test |
| AC-4.4 | Shows last apply timestamp | Integration test |
| AC-4.5 | Shows drift indicator (diverged module count) | Integration test |
| AC-4.6 | `--full` flag triggers full `ActualState::scan()` for accurate counts | CLI parsing test + integration test |
| AC-4.7 | `--json` output uses response envelope | Integration test: parse JSON |
| AC-4.8 | `--dry-run` flag works for testing | Integration test |
| AC-4.9 | `Commands::Status` variant in `cli.rs` has `full` and `dry_run` fields | Code review |
| AC-4.10 | Latency without `--full` is under 2 seconds on typical system | Manual verification |

### F3-005: iron plan command

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-5.1 | `iron plan` generates and displays full system plan (no confirmation prompt) | Integration test: `iron plan --dry-run` |
| AC-5.2 | `iron plan --module <id>` filters plan to single module | CLI parsing test |
| AC-5.3 | `iron plan --json` outputs plan in envelope format | Integration test |
| AC-5.4 | Plan uses `ActualState::scan()` + desired state resolution + `compute_plan()` | Code review |
| AC-5.5 | Plan display uses tree output for actions | Code review of output calls |
| AC-5.6 | `--dry-run` flag exists for test consistency | CLI parsing test |
| AC-5.7 | Registered in `cli.rs` Commands enum | Code review |
| AC-5.8 | Registered in `commands/mod.rs` | Code review |
| AC-5.9 | Dispatch added to `main.rs` | Code review |

### F3-006: XDG state directory separation

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-6.1 | `StateManager::state_dir()` resolves: `$IRON_STATE_DIR` > `$XDG_STATE_HOME/iron` > `~/.local/state/iron` | Unit test with env vars |
| AC-6.2 | `StateManager::new()` uses `state_dir()` for state.json location | Unit test |
| AC-6.3 | Audit log uses `state_dir()` for file path | Code review |
| AC-6.4 | Lock file uses `state_dir()` for file path | Code review |
| AC-6.5 | `SnapshotService` uses `state_dir()/snapshots/` | Code review |
| AC-6.6 | `state_dir()` creates directory if it does not exist | Unit test |
| AC-6.7 | `AppContext::is_initialized()` updated to check state_dir, not config root | Code review |
| AC-6.8 | `$IRON_STATE_DIR` env var override works | Integration test |

### F3-007: Legacy state migration

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-7.1 | `StateManager::migrate_if_needed(config_root)` exists | Code review |
| AC-7.2 | Migrates state.json, audit.log, .state.lock, .snapshots/ | Unit test with temp dirs |
| AC-7.3 | Uses copy-then-delete (not move) | Code review |
| AC-7.4 | Leaves MIGRATED.txt marker in old location | Unit test |
| AC-7.5 | No-op if new location already has state.json | Unit test |
| AC-7.6 | No-op if legacy location has no state.json | Unit test |
| AC-7.7 | On failure: original files intact, warning logged, continues with legacy location | Unit test |
| AC-7.8 | Called during startup (context.rs or main.rs) | Code review |

---

## 8. Architectural Questions for Architect

The following decisions are outside the analyst's scope and require architect input:

1. **F3-002b trait signature decision:** Should `ApplyService::plan()` and `DriftService::detect()` trait methods change to accept `&ActualState`, or should only the private `compute_plan()` and internal detect methods change? Changing trait methods maximizes caller control but increases cascade. Keeping trait methods unchanged contains the cascade but hides `ActualState` from callers.

2. **F3-006 StateManager constructor change:** Should `StateManager::new()` still accept a `root: PathBuf` (config root) and internally resolve state_dir, or should it accept separate `config_root` and `state_dir` parameters? The former is simpler; the latter is more explicit and testable.

3. **F3-003a envelope generics:** Should `IronEnvelope<T>` require `T: Serialize` at the struct level or only at the serialization method? This affects ergonomics when constructing envelopes in test code.

4. **F3-004 status data source:** The kanban says "from state.managed_packages" for package counts, but `IronState` does not currently have a `managed_packages` field (that is F3-021 in Sprint 3.2). What should `iron status` show for package counts before F3-021? Options: (a) skip package counts until Sprint 3.2, (b) count packages from resolved DesiredState, (c) count from state's module list.

---

## 9. Unchanged Requirements

The following existing behaviors must NOT change during Sprint 3.1:

- `iron apply` behavior (plan + execute + snapshot) -- only internal implementation changes
- `iron diff` output format (text mode) -- only JSON wrapping changes
- `iron scan` behavior and output
- `iron snapshot create/list/restore/delete/prune` behavior
- `iron security` behavior
- `iron validate` behavior
- TUI navigation, views, keybindings
- Module/Bundle/Profile/Host TOML parsing
- Git sync operations
- Circuit breaker / resilience patterns
- All existing test assertions (values, not just pass/fail)
