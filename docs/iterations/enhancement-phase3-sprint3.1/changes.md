# Changes -- Phase 3 Sprint 3.1 (Wave 1)

> Updated by developer agent during implementation.

## Files Created

| File | Purpose |
|------|---------|
| `crates/iron-core/src/actual_state.rs` | F3-001: `ActualState`, `ActualServiceState`, `ActualFileState`, `FileStateType` structs with `scan()` method. Includes `ManagedFileSpec` and `ManagedServiceSpec` input types. Full unit test suite (14 tests) with mock PackageManager and SystemService implementations. |
| `crates/iron-core/src/envelope.rs` | F3-003a: `IronEnvelope<T>`, `EnvelopeError`, `EnvelopeMeta` structs. Success/error constructors. Manual `Serialize` impl with `T: Serialize` bound on impl (not struct). Unit tests (8 tests) covering construction, serialization, and field validation. |

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/Cargo.toml` | Added `gethostname = "0.5"` and `sha2 = "0.10"` dependencies | F3-001/F3-002a: hostname detection for `ActualState::scan()` and `EnvelopeMeta`, SHA256 checksums for file state |
| `crates/iron-core/src/lib.rs` | Added `pub mod actual_state;` and `pub mod envelope;` module registrations. Added re-exports for `ActualState`, `ActualFileState`, `ActualServiceState`, `FileStateType`, `ManagedFileSpec`, `ManagedServiceSpec` | F3-001, F3-003a: module registration and convenience re-exports |
| `crates/iron-cli/src/output.rs` | Added `json_envelope()` and `json_error_envelope()` methods to `Output`. Added imports for `IronEnvelope`, `Instant` | F3-003a: CLI-side envelope convenience methods for `--json` output |
| `crates/iron-core/src/services/state.rs` | Added `state_root` field to `StateManager`. Added `state_dir()`, `state_root()`, `config_root()` methods. Updated `new()` to resolve state directory with backward compat. Updated `state_path()`, `lock_path()`, `persist_audit_log()`, `load_audit_log()` to use `state_root` | F3-006: XDG state directory separation |
| `crates/iron-cli/src/context.rs` | Updated `is_initialized()` to check `state.state_root().join("state.json")` instead of `self.root.join("state.json")` | F3-006: check state file in XDG state directory |
| `crates/iron-cli/tests/acceptance/fixtures.rs` | Added `env("IRON_STATE_DIR", ...)` to `run_iron()` and `run_iron_json()` methods | F3-006: ensure acceptance tests use the temp dir for state files |

## Flagged Items

- The `json_envelope` and `json_error_envelope` methods in `output.rs` generate "never used" warnings. These will be consumed in Wave 4 (F3-003b) when existing `--json` outputs are migrated to envelope format.
- The `state_dir()` function creates the directory only when `$IRON_STATE_DIR` is explicitly set. For real XDG resolution without the env var, directory creation is deferred to `migrate_if_needed()` (F3-007, Wave 2).

## Notes

### StateManager backward compatibility strategy

The `StateManager::new(root)` constructor resolves the state root with a 4-step priority:

1. If `$IRON_STATE_DIR` is set, use it (testing/custom deployments)
2. If the config root (`root` param) has `state.json`, use the config root (pre-migration backward compat)
3. If the XDG state dir has `state.json`, use it (post-migration)
4. Otherwise, use the config root (fresh installation)

This means existing installations continue working without any migration step. All existing tests pass without modification because they write `state.json` to the config root (temp dir), which triggers rule #2. The `migrate_if_needed()` function (F3-007, Wave 2) will move state files from config root to XDG dir, at which point rule #3 takes over.

### ActualState design decisions

- `installed_packages` uses `HashSet<String>` (not `Vec<String>`) per architect decision AQ-1: the primary access pattern is membership testing (`contains()`), which is O(1) for HashSet vs O(n) for Vec.
- AUR packages are tracked in both `installed_packages` (all packages) and `aur_packages` (AUR-only). This matches the existing `PackageManager::query_installed()` return type.
- `running` field on `ActualServiceState` defaults to `false` -- the `SystemService` trait does not yet have an `is_running()` method. This will be extended in Sprint 3.2 if needed.

### IronEnvelope design decisions

- `T: Serialize` bound is on the `impl Serialize for IronEnvelope<T>` block, not on the struct definition, per architect decision AQ-3. This allows constructing envelopes with any type in test code.
- `EnvelopeMeta::now()` uses `gethostname` (same dependency as `ActualState::scan()`) and `env!("CARGO_PKG_VERSION")` which resolves to iron-core's version (shared across workspace via `version.workspace = true`).

---

# Changes -- Phase 3 Sprint 3.1 (Wave 2)

> Updated by developer agent during implementation.

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/services/apply.rs` | Added `scan_actual_state()` private method. Changed `compute_plan()` to accept `&ActualState`. Refactored package/service/symlink diff logic to read from `ActualState` instead of querying `PackageManager`/`SystemService`/filesystem directly. `plan()` and `plan_module()` now scan once then pass result to `compute_plan()`. | F3-002a: Single ActualState scan replaces ad-hoc system queries in apply pipeline. Keeps public `ApplyService` trait unchanged per AQ-2. |
| `crates/iron-core/src/services/drift.rs` | Added `scan_actual_state()` private method. Changed `detect_package_drift()`, `detect_service_drift()`, `detect_config_drift()` to accept `&ActualState`. `detect()` scans once then passes `&actual` to all three detect methods. | F3-002a: Single ActualState scan replaces ad-hoc system queries in drift pipeline. Keeps public `DriftService` trait unchanged per AQ-2. |
| `crates/iron-core/src/services/state.rs` | Added `migrate_if_needed()` public method, `migrate_to()` private method, `migrate_file()`, `migrate_dir()`, `copy_dir_recursive()` helpers. Added `MigrationResult` enum. Updated `StateManager::new()` resolution logic to use `MIGRATED.txt` breadcrumb for XDG dir detection. Added 6 migration tests. | F3-009: State file migration from config root to XDG state dir. Breadcrumb-based resolution avoids breaking tests that use temp dirs. |
| `crates/iron-cli/src/context.rs` | Added `StateManager::migrate_if_needed(&root)` call before `StateManager::new()` in `AppContext::new()`. Migration result logged when verbose, errors treated as non-fatal warnings. | F3-009: Migration runs automatically on CLI startup. |
| `crates/iron-cli/tests/cli_integration.rs` | Added `iron_at()` helper that sets `IRON_STATE_DIR` env var to temp dir. Replaced all `iron().arg("--root")` patterns with `iron_at(dir.path()).arg("--root")`. Fixed `iron_raw()` call similarly. | F3-009/F3-006: Test isolation -- prevent StateManager from resolving to real system's XDG state directory. |
| `crates/iron-cli/tests/cli_output_validation.rs` | Added same `iron_at()` helper. Replaced all `iron()` calls that use `--root` with `iron_at(dir.path())`. | F3-009/F3-006: Test isolation -- same fix as cli_integration.rs. |

## Flagged Items

- None. All changes are within scope of F3-002a and F3-009.

## Notes

### F3-002a: ActualState integration strategy

Both `DefaultApplyService` and `DefaultDriftService` received the same structural change:

1. A private `scan_actual_state(&self, desired: &DesiredState) -> IronResult<ActualState>` method builds `ManagedServiceSpec` and `ManagedFileSpec` vectors from the desired state, then calls `ActualState::scan()`.
2. The public trait method (`plan()` / `detect()`) scans once, then passes `&ActualState` to all internal diff methods.
3. Internal diff methods read from `ActualState` fields (`installed_packages`, `services`, `managed_files`) instead of making individual system calls.

This is a purely internal refactor -- no public API changes, no new trait methods, no signature changes to `ApplyService` or `DriftService` traits.

### F3-009: Migration implementation details

**Migration flow**: `migrate_if_needed(config_root)` checks if state files exist in the config root but not in the XDG state dir. If so, it copies `state.json`, `audit.log`, `.state.lock`, and `.snapshots/` to the XDG dir, writes a `MIGRATED.txt` breadcrumb in the config root, then deletes the originals.

**StateManager resolution logic** was updated from Wave 1's approach. The new 4-step priority:

1. If `$IRON_STATE_DIR` is set, use it (testing/custom deployments)
2. If the config root has `state.json`, use the config root (pre-migration)
3. If `MIGRATED.txt` exists in config root AND XDG state dir has `state.json`, use XDG dir (post-migration)
4. Otherwise, use the config root (fresh installation)

The key difference from Wave 1 is step 3: instead of unconditionally checking the XDG dir for `state.json`, we require the `MIGRATED.txt` breadcrumb. This prevents the StateManager from accidentally finding the real system's XDG state when running tests with temp directories.

### Test isolation fix

The `MIGRATED.txt` breadcrumb fix resolved 37 iron-core unit test failures and 12 CLI integration test failures. The root cause was that `StateManager::new(temp_dir)` would check `~/.local/state/iron/state.json` (from the developer's real system) and find it, making the temp dir appear "initialized" when it should not be. The `iron_at()` helper in CLI tests provides additional isolation by setting `IRON_STATE_DIR` explicitly.

### Migration testability

Migration tests use a `migrate_to(config_root, state_dir)` private method that accepts an explicit target directory, avoiding `std::env::set_var` which is unsafe in Rust 2024 edition. The public `migrate_if_needed()` calls `migrate_to()` internally with the resolved XDG directory.

---

# Changes -- Phase 3 Sprint 3.1 (Wave 3)

> Updated by developer agent during implementation.

## Task 1: F3-002b -- Refactor consumers of old scan APIs

### Result: No consumer changes needed

The architect decided (AQ-2) to keep `ApplyService` and `DriftService` trait method signatures unchanged. Wave 2 already refactored the private internals:

- `DefaultApplyService::compute_plan()` accepts `&ActualState` and reads from it instead of querying `PackageManager`/`SystemService`/filesystem directly.
- `DefaultDriftService::detect_package_drift()`, `detect_service_drift()`, `detect_config_drift()` all accept `&ActualState`.
- Both services have a private `scan_actual_state()` that builds specs from `DesiredState` and calls `ActualState::scan()`.

**Consumer verification performed:**

| Consumer | File | Status |
|----------|------|--------|
| CLI apply | `crates/iron-cli/src/commands/apply.rs` | No change needed -- calls `service.plan(host_id)` and `service.plan_module(mod_id)` via unchanged trait |
| CLI diff | `crates/iron-cli/src/commands/diff.rs` | No change needed -- calls `service.detect(host_id)` via unchanged trait |
| CLI diff --correct | `crates/iron-cli/src/commands/diff.rs` | No change needed -- calls `apply_svc.plan(host_id)` via unchanged trait |
| TUI actions | `crates/iron-tui/src/app/actions.rs` | No direct plan/detect calls (confirmed by grep) |
| TUI app | `crates/iron-tui/src/app/mod.rs` | No direct plan/detect calls (confirmed by grep) |
| Snapshot restore | `crates/iron-core/src/services/snapshot_service.rs` | No plan/detect calls (confirmed by grep) |
| Snapshot CLI restore | `crates/iron-cli/src/commands/snapshot.rs` | Calls `apply_svc.plan(host_id)` via unchanged trait |

**Ad-hoc query verification:**

- `apply.rs` `compute_plan()`: reads `actual.installed_packages`, `actual.managed_files`, `actual.services` -- no `self.package_manager.query_installed()` or `self.service_manager.is_enabled()` calls inside compute_plan.
- `apply.rs` `execute()`: still uses `self.package_manager.install()` and `self.service_manager.enable_service()` -- correct, these are write operations, not reads.
- `drift.rs` detect methods: all read from `&ActualState` -- no `self.package_manager` or `self.service_manager` calls in detect_*_drift methods.

## Task 2: F3-003b -- Migrate existing CLI commands to envelope

### Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-cli/src/commands/scan.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Changed `output.json(&report)` to `output.json_envelope("scan", &report, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/module.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Threaded `start` to `list()` and `show()` functions. Changed 2 `output.json()` calls to `output.json_envelope("module.list", ...)` and `output.json_envelope("module.show", ...)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/secrets.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Threaded `start` to `status()` function. Changed `output.json(&info)` to `output.json_envelope("secrets.status", &info, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/doctor.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Changed `output.json(&report)` to `output.json_envelope("doctor", &report, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/update.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Threaded `start` to `show_progress_status()`. Changed 3 `output.json()` calls to `output.json_envelope("update", ...)` and `output.json_envelope("update.status", ...)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/recover.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Threaded `start` to `export_state()`. Changed `output.json(&export_data)` to `output.json_envelope("recover.export", &export_data, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/snapshot.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Threaded `start` to `execute_list()`. Changed `output.json(&records)` to `output.json_envelope("snapshot.list", &records, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/validate.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Changed `output.json(&warnings)` to `output.json_envelope("validate", &warnings, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/security.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Changed `output.json(&report)` to `output.json_envelope("security", &report, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/commands/status.rs` | Added `use std::time::Instant`, `let start = Instant::now()` at handler entry. Changed `output.json(&data)` to `output.json_envelope("status", &data, start)` | F3-003b: envelope migration |
| `crates/iron-cli/src/output.rs` | Added `#[allow(dead_code)]` on `json_error_envelope()` | Suppress warning -- error envelope paths will be migrated in a future wave |
| `crates/iron-core/src/actual_state.rs` | Replaced manual `impl Default for FileStateType` with `#[derive(Default)]` and `#[default]` attribute on `Missing` variant | Clippy fix: `derivable_impls` lint |
| `crates/iron-cli/tests/cli_output_validation.rs` | Updated `doctor_json_has_required_structure` and `doctor_json_contains_all_fr10_checks` tests to unwrap envelope (`json["data"]`) before accessing `checks`, `overall`, `timestamp` fields | F3-003b: tests must account for new envelope wrapper |

### Commands not migrated (not in task scope)

The following commands have `output.json()` calls but were not listed in the task scope:

| File | Calls | Notes |
|------|-------|-------|
| `crates/iron-cli/src/commands/host.rs` | 3 calls | `host.list`, `host.show`, `host.switch` |
| `crates/iron-cli/src/commands/profile.rs` | 2 calls | `profile.list`, `profile.show` |
| `crates/iron-cli/src/commands/bundle.rs` | 2 calls | `bundle.list`, `bundle.show` |
| `crates/iron-cli/src/commands/sync.rs` | 1 call | `sync.status` |

### Commands with no JSON output to migrate

| File | Notes |
|------|-------|
| `crates/iron-cli/src/commands/apply.rs` | No `output.json()` calls -- uses text output only |
| `crates/iron-cli/src/commands/diff.rs` | No `output.json()` calls -- uses text output only |
| `crates/iron-cli/src/commands/clean.rs` | No `output.json()` calls -- uses text/summary output only |

## Flagged Items

- **Out-of-scope `output.json()` calls**: 8 remaining `output.json()` calls in `host.rs` (3), `profile.rs` (2), `bundle.rs` (2), `sync.rs` (1). These were not listed in the task scope. Should be migrated in a follow-up task.
- **`json_error_envelope` unused**: The error envelope method is defined but not yet consumed. Error paths in CLI commands still use `output.error()` (text-only). Migrating error paths to structured JSON errors is a future task.

## Notes

### Envelope format

All migrated `--json` output now wraps data in the `IronEnvelope` structure:

```json
{
  "ok": true,
  "command": "scan",
  "data": { ... },
  "error": null,
  "meta": {
    "timestamp": "2026-02-23T...",
    "duration_ms": 42,
    "host": "hostname",
    "version": "0.1.0"
  }
}
```

### Human-readable output unchanged

Non-JSON output (text mode, minimal mode) is completely unaffected. The `json_envelope()` method checks `self.is_json()` and returns immediately if the format is not JSON.

### Test impact

Only 2 tests required updates (`doctor_json_has_required_structure`, `doctor_json_contains_all_fr10_checks`) because they structurally parsed the JSON output and expected fields at the top level. All other JSON tests used `stdout.contains("...")` string matching which works through the envelope wrapper since the data values are still present in the serialized output.

---

# Changes -- Phase 3 Sprint 3.1 (Wave 4)

> Updated by developer agent during implementation.

## Task 1: F3-004 -- Enhance `iron status` command

### Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-cli/src/cli.rs` | Changed `Commands::Status` from unit variant to struct variant with `full: bool` and `dry_run: bool` fields. Updated 2 test assertions from `Commands::Status` to `Commands::Status { .. }`. Added 2 new tests (`test_cli_status_full_flag`, `test_cli_status_dry_run_flag`). | F3-004: Status command needs `--full` and `--dry-run` flags |
| `crates/iron-cli/src/main.rs` | Updated `Commands::Status` dispatch from `Commands::Status =>` to `Commands::Status { full, dry_run } =>`. Passes both flags to `status::execute()`. | F3-004: Wire new flags to status handler |
| `crates/iron-cli/src/commands/status.rs` | Enhanced `execute()` to accept `full` and `dry_run` params. Added `resolve_desired_state()` call to compute package/service/dotfile counts from DesiredState. Added `SecurityService::calculate()` for security level. Added `DriftService::detect()` for `--full` drift summary. New JSON fields: `packages`, `security`, `drift`. New text sections: "Declared State", "Security", "Drift". New structs: `PackagesStatus`, `SecurityStatus`, `DriftSummary`. | F3-004: Lightweight status with declared counts by default; full drift scan with `--full` |

### Design decisions

- **Default path (no `--full`)**: Reads state.json + resolves DesiredState from TOML files. Shows declared package/service/dotfile counts. Does NOT scan the system (no pacman queries). Performance target: < 2s.
- **`--full` path**: Additionally calls `DriftService::detect()` which internally scans ActualState. Shows drift summary (missing/extra packages, drifted configs/services).
- **`--dry-run` with `--full`**: Skips the drift scan but still shows the "Drift" section with a "[DRY RUN]" message. This allows integration tests to exercise the `--full` flag without system queries.
- **Security level**: Always computed (fast -- reads module TOML files only). Shows level label, score, and max score.
- **Package counts from DesiredState**: Per architect decision (section 3.9), shows "Packages (declared)" to distinguish from actual installed counts. The `resolve_desired_state()` call loads TOML files from disk, which is fast.

## Task 2: F3-005 -- `iron plan` command

### Files Created

| File | Purpose |
|------|---------|
| `crates/iron-cli/src/commands/plan.rs` | F3-005: `iron plan` command. Computes and displays an ApplyPlan without executing. Groups actions by type (packages, dotfiles, services, modules) with colored `+` indicators. Supports `--json` via envelope, `--module` for single-module plan, `--dry-run` for testing. |

### Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-cli/src/cli.rs` | Added `Commands::Plan { module: Option<String>, dry_run: bool }` variant. Added 3 tests (`test_cli_plan_basic`, `test_cli_plan_module`, `test_cli_plan_dry_run`). | F3-005: Register plan command in CLI |
| `crates/iron-cli/src/commands/mod.rs` | Added `pub mod plan;` | F3-005: Module registration |
| `crates/iron-cli/src/main.rs` | Added `Commands::Plan { module, dry_run }` dispatch to `commands::plan::execute()` | F3-005: Wire plan command |

### Design decisions

- **Display-only**: `iron plan` never prompts for confirmation or executes any actions. It shows what `iron apply` would do.
- **Grouped output**: Actions are grouped by type (Packages, Dotfiles, Services, Modules) for readability, with colored `+` indicators for additive actions.
- **`--dry-run`**: Returns empty plan immediately without system queries. Allows integration tests to verify the command exists and parses correctly.
- **`--json`**: Uses `json_envelope("plan", &plan, start)` for consistent envelope format.
- **No `--output` or `--plan` serialization**: Deferred to Phase 4 per architect Decision D6.
- **TUI integration deferred**: The TUI does not call `plan()` or `detect()` directly (confirmed in Wave 3 analysis). A plan view could be added in a future sprint.

## Flagged Items

- **TUI plan view**: Not implemented. The TUI does not currently have a plan view. This could be added as a future enhancement but is not in Wave 4 scope.
- **Remaining `output.json()` calls**: 8 `output.json()` calls remain in `host.rs` (3), `profile.rs` (2), `bundle.rs` (2), `sync.rs` (1) from Wave 3. Not in Wave 4 scope.

## Notes

### Test verification

All workspace tests pass (1,397 tests total, 0 failures):
- iron-core: 107 unit tests
- iron-core integration: 23 tests
- iron-cli integration: 61 tests
- iron-cli output validation: 31 tests
- iron-core scan/toml: 977 tests
- iron-tui: 100 tests
- All other crate tests pass

### Backward compatibility

- The `Commands::Status` variant change from unit to struct is a breaking change for the `main.rs` match arm, handled atomically in the same wave.
- JSON output for `iron status` now includes additional fields (`packages`, `security`, `drift`) but all previous fields remain unchanged. Consumers accessing `data.host`, `data.bundle`, `data.modules`, etc. are unaffected.
- The `iron plan` command is entirely new -- no backward compatibility concerns.
