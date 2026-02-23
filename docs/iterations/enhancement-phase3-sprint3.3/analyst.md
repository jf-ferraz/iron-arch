# Analyst Report -- Sprint 3.3 (Execution Lifecycle Completion)

**Date:** 2026-02-23
**Type:** ENHANCEMENT (structural)
**Sprint:** 3.3 -- Execution Lifecycle Completion
**Tasks:** F3-014, F3-015, F3-016, F3-018, F3-017 (STRETCH) + 3 carryovers

---

## 1. Scope Verification

### F3-014: Hook Execution in Apply Lifecycle -- WELL-DEFINED

The Module struct already has `pre_install`, `post_install`, `pre_uninstall`, and `status_check` fields (lines 37-48 of `crates/iron-core/src/module.rs`). These are `Option<String>` shell commands. They are populated in tests and TOML parsing but **never executed** anywhere in the codebase.

**Ambiguities requiring architect decisions:**

- **AMB-014-1: HookType enum scope.** The spec lists `PreInstall`, `PostInstall`, `PreUninstall`, `StatusCheck`. When is `PreUninstall` triggered? Only during `--prune` (module deactivation)? Only `PreInstall` and `PostInstall` are relevant during normal apply. `StatusCheck` is informational -- when is it run?
- **AMB-014-2: Hook execution via CommandExecutor.** The CLAUDE.md states "no raw `Command::new()` in production paths" and hooks should use `CommandExecutor`. However, `DefaultApplyService` does not currently hold a `CommandExecutor`. The `execute_action()` method (line 1265) calls `package_manager.install()` and `service_manager.enable_service()` directly via trait methods, but these are not `CommandExecutor`. Hooks are arbitrary shell commands. The architect must decide: (a) add `Arc<dyn CommandExecutor>` to `DefaultApplyService`, or (b) run hooks via `std::process::Command` since they are user-defined and CommandExecutor's circuit breaker semantics may not apply.
- **AMB-014-3: Ask behavior in TUI context.** The TUI apply view (line 10 of `crates/iron-tui/src/ui/apply.rs`) currently shows a simple plan count and action list. It does not support interactive prompts. The `Ask` behavior for hooks in TUI should either skip with a warning or auto-approve with a log entry.
- **AMB-014-4: Hook working directory.** Hooks like `nvim --headless +Lazy! sync +qa` need a working directory. Should it be the module directory, the user's home, or the iron root?
- **AMB-014-5: Hook timeout.** Should hooks have a timeout? CommandExecutor has circuit breaker patterns. Raw shell commands could hang indefinitely.

### F3-015: Hook Execution Tracking -- WELL-DEFINED

Small, clear scope. Depends on F3-014.

**One ambiguity:**
- **AMB-015-1: When are Once entries cleared?** If a module is disabled and re-enabled, should Once hooks re-run? If the user reinstalls a module, the hooks_executed entry should probably be cleared for that module.

### F3-016: iron history Command -- WELL-DEFINED

The audit log infrastructure exists. `StateManager` stores `audit_log: Arc<Mutex<Vec<AuditEntry>>>` (line 100 of `crates/iron-core/src/services/state.rs`). `AuditEntry` has `timestamp`, `operation`, `status`, `details`, `user` fields (lines 76-88). The `last_operations` field on `IronState` (line 164 of `crates/iron-core/src/state.rs`) stores `Vec<OperationRecord>`.

**Ambiguities:**
- **AMB-016-1: Data source.** The spec says "reads from audit.log (JSONL)". However, the current `load_audit_log()` (line 183-192 of state.rs) loads the audit log as a single JSON array (`serde_json::from_str`), not JSONL. The format must be clarified. The current format is a JSON array.
- **AMB-016-2: Operation identity.** The spec shows `iron history show 3`. How are operations numbered? By index in the audit log? By a sequential ID? Currently `AuditEntry` has no `id` field and `OperationRecord` has no `id` field.
- **AMB-016-3: What constitutes an "operation"?** Every `audit()` call produces an entry (enable_module, set_current_host, etc). The history view should aggregate these into logical operations (e.g., a single "apply" that includes multiple sub-actions).

### F3-017: iron config Namespace -- STRETCH, CLEAR

Simple CLI grouping. Low risk. Can be deferred without impact.

**Assessment:** Implement only if all other tasks complete cleanly. See Section 10.

### F3-018: dotfiles_sync Auto-Mirror -- WELL-DEFINED

**Ambiguities:**
- **AMB-018-1: Interaction with explicit dotfiles.** The spec says "Explicit `[[dotfiles]]` entries override auto-discovered entries for the same target." This means if a module has `dotfiles_sync = true` AND `[[dotfiles]]` entries, the explicit entries take precedence for overlapping targets, but auto-discovered files that don't overlap are still deployed. The merge logic needs careful definition.
- **AMB-018-2: File discovery scope.** Should files starting with `.` (e.g., `.gitignore`) in the module's `dotfiles/` directory be included? Should binary files be excluded?
- **AMB-018-3: Default target derivation.** The spec says default is `~/.config/<module-id>/`. But `resolve_desired_state()` (line 59-158 of apply.rs) collects `module.dotfiles` directly. The dotfiles_sync discovery must happen at desired state resolution time, not at plan time, so that the desired dotfile list is complete before compute_plan runs.

### Carryover: SHOULD-1 (iron status managed resource display)

Clear scope. Wire `state.managed_packages.len()`, `managed_services.len()`, `managed_dotfiles.len()`, and `last_apply` into `iron status` text and JSON output. The `StatusData` struct (line 24 of `crates/iron-cli/src/commands/status.rs`) already has `PackagesStatus` but only shows declared counts, not managed counts.

### Carryover: SHOULD-3 (cargo fmt)

Trivial. Run `cargo fmt --all`.

### Carryover: SHOULD-5 (temporal comments)

Three locations identified in Sprint 3.2 review:
- `crates/iron-core/src/services/apply.rs` line 261: `// -- Sprint 3.2: New variants --`
- `crates/iron-core/src/services/apply.rs` line 1325: `// -- Sprint 3.2 new variants --`
- `crates/iron-core/src/services/apply.rs` line 287: doc comment `/// Risk classification for apply actions (Sprint 3.2, F3-013).`

---

## 2. Codebase Impact Analysis

### F3-014: Hook Execution in Apply Lifecycle

| File | Change | Lines |
|------|--------|-------|
| `crates/iron-core/src/module.rs` | Add `hook_behavior: HookBehavior` field with `#[serde(default)]` | After line 60 |
| `crates/iron-core/src/module.rs` | Define `HookBehavior` enum (Always/Once/Ask/Skip) | New enum after `ModuleKind` |
| `crates/iron-core/src/module.rs` | Define `HookType` enum (PreInstall/PostInstall/PreUninstall/StatusCheck) | New enum |
| `crates/iron-core/src/module.rs` | Update `create_test_module()` (line 158-188) to include `hook_behavior` | Line 186 |
| `crates/iron-core/src/services/apply.rs` | Add `RunHook` variant to `ApplyAction` enum (after line 284) | Line ~285 |
| `crates/iron-core/src/services/apply.rs` | Add `risk_level()` match arm for `RunHook` (after line 337) | Line ~338 |
| `crates/iron-core/src/services/apply.rs` | Add `is_prunable()` -- RunHook is NOT prunable | Line ~349 |
| `crates/iron-core/src/services/apply.rs` | Add `display()` for RunHook | Line ~411 |
| `crates/iron-core/src/services/apply.rs` | Add `summary()` count for hooks | Line ~545 |
| `crates/iron-core/src/services/apply.rs` | Insert hook planning into `compute_plan()` -- pre_install before packages, post_install after services | Lines ~905-1060 |
| `crates/iron-core/src/services/apply.rs` | Add `execute_action()` match arm for RunHook -- shell command execution | Line ~1435 |
| `crates/iron-core/src/services/apply.rs` | Add `record_managed_resource()` match arm for RunHook (no-op) | Line ~1193 |
| `crates/iron-core/src/services/mod.rs` | Export `HookBehavior`, `HookType` if needed from apply module | Line ~26 |
| `crates/iron-core/src/test_helpers.rs` | Update `TestModule.to_module()` to include `hook_behavior` field | Line 291 |
| `crates/iron-cli/src/commands/plan.rs` | Update plan display to show RunHook actions | Existing display logic |
| `crates/iron-tui/src/ui/apply.rs` | Render RunHook actions in plan display | Line ~63 |

### F3-015: Hook Execution Tracking

| File | Change | Lines |
|------|--------|-------|
| `crates/iron-core/src/state.rs` | Add `hooks_executed: HashMap<String, Vec<String>>` to `IronState` with `#[serde(default)]` | After line 199 |
| `crates/iron-core/src/services/state.rs` | Add methods: `record_hook_executed(module_id, hook_type)`, `is_hook_executed(module_id, hook_type)`, `clear_hooks_for_module(module_id)` | New methods |
| `crates/iron-core/src/services/apply.rs` | In `execute_action()` RunHook arm: check Once tracking before execution, record after | Inside RunHook match arm |
| `crates/iron-core/src/services/apply.rs` | In `compute_plan()`: skip Once hooks if already in hooks_executed | In hook planning section |
| `crates/iron-cli/src/cli.rs` | Add `--force-hooks` flag to Apply command | After line 188 |
| `crates/iron-cli/src/main.rs` | Pass `force_hooks` flag through dispatch | Line ~83 |
| `crates/iron-cli/src/commands/apply.rs` | Accept and use `force_hooks` parameter | Function signature |

### F3-016: iron history Command

| File | Change | Lines |
|------|--------|-------|
| `crates/iron-core/src/services/history.rs` | **NEW FILE** -- `HistoryService` trait + `DefaultHistoryService` | New |
| `crates/iron-core/src/services/mod.rs` | Add `pub mod history;` and re-export | Lines 6-58 |
| `crates/iron-cli/src/commands/history.rs` | **NEW FILE** -- `iron history list`, `iron history show <id>`, `iron history last` | New |
| `crates/iron-cli/src/commands/mod.rs` | Add `pub mod history;` | Line ~12 |
| `crates/iron-cli/src/cli.rs` | Add `History` command with `HistoryAction` subcommands to `Commands` enum | After line 337 |
| `crates/iron-cli/src/main.rs` | Add dispatch for `Commands::History` | After line 113 |

### F3-018: dotfiles_sync Auto-Mirror

| File | Change | Lines |
|------|--------|-------|
| `crates/iron-core/src/module.rs` | Add `dotfiles_sync: bool` with `#[serde(default)]` | After line 60 |
| `crates/iron-core/src/module.rs` | Add `dotfiles_sync_target: Option<String>` with `#[serde(default)]` | After dotfiles_sync |
| `crates/iron-core/src/module.rs` | Update `create_test_module()` with new fields | Lines 158-188 |
| `crates/iron-core/src/services/apply.rs` | In `resolve_desired_state()`: after loading module (line 140), if `dotfiles_sync`, discover files and merge with explicit dotfiles | Lines 137-145 |
| `crates/iron-core/src/test_helpers.rs` | Update `TestModule` with `dotfiles_sync` and `dotfiles_sync_target` fields, update `to_module()` | Lines 164-291 |

### Carryover: SHOULD-1 (iron status)

| File | Change | Lines |
|------|--------|-------|
| `crates/iron-cli/src/commands/status.rs` | Add `ManagedResources` section to `StatusData` struct | After line 37 |
| `crates/iron-cli/src/commands/status.rs` | Load StateManager, read managed counts and last_apply | After line 195 |
| `crates/iron-cli/src/commands/status.rs` | Render managed counts in text output | After line 331 |

### Carryover: SHOULD-3 & SHOULD-5

| File | Change |
|------|--------|
| `crates/iron-core/src/services/apply.rs` line 261 | Remove `// -- Sprint 3.2: New variants --` |
| `crates/iron-core/src/services/apply.rs` line 1325 | Remove `// -- Sprint 3.2 new variants --` |
| `crates/iron-core/src/services/apply.rs` line 287 | Remove `(Sprint 3.2, F3-013)` from doc comment |
| All files | Run `cargo fmt --all` |

---

## 3. Existing Infrastructure Audit

### What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| `Module.pre_install: Option<String>` | EXISTS, never executed | `module.rs` line 37 |
| `Module.post_install: Option<String>` | EXISTS, never executed | `module.rs` line 40 |
| `Module.pre_uninstall: Option<String>` | EXISTS, never executed | `module.rs` line 43 |
| `Module.status_check: Option<String>` | EXISTS, never executed | `module.rs` line 47 |
| `ApplyAction` enum (11 variants) | EXISTS, no `RunHook` | `apply.rs` lines 244-284 |
| `RiskLevel` enum | EXISTS (ReadOnly/Additive/Destructive/Critical) | `apply.rs` lines 290-303 |
| `ApplyAction::risk_level()` | EXISTS for all 11 variants | `apply.rs` lines 316-338 |
| `ApplyAction::display()` | EXISTS for all 11 variants | `apply.rs` lines 352-412 |
| `ApplyPlan::summary()` | EXISTS, counts all categories | `apply.rs` lines 450-553 |
| `execute_action()` | EXISTS for all 11 variants | `apply.rs` lines 1264-1436 |
| `record_managed_resource()` | EXISTS for all 11 variants | `apply.rs` lines 1167-1196 |
| `IronState.managed_packages/services/dotfiles` | EXISTS | `state.rs` lines 182-195 |
| `IronState.last_apply` | EXISTS | `state.rs` line 199 |
| `IronState.last_operations` | EXISTS, `Vec<OperationRecord>` | `state.rs` line 164 |
| `AuditEntry` struct | EXISTS | `services/state.rs` lines 76-88 |
| `StateManager.audit_log` | EXISTS, loaded from disk | `services/state.rs` lines 100, 183-192 |
| `StateManager.state_dir()` | EXISTS, XDG resolution | `services/state.rs` lines 113-123 |
| `CommandExecutor` trait | EXISTS (in resilience module) | `iron-core/src/resilience/` |
| `StatusData` struct for iron status | EXISTS | `commands/status.rs` lines 24-38 |
| `TestModule` builder | EXISTS, missing new fields | `test_helpers.rs` lines 164-293 |

### What Is New

| Component | Task | Description |
|-----------|------|-------------|
| `HookBehavior` enum | F3-014 | Always/Once/Ask/Skip |
| `HookType` enum | F3-014 | PreInstall/PostInstall/PreUninstall/StatusCheck |
| `Module.hook_behavior` field | F3-014 | Controls hook execution policy |
| `ApplyAction::RunHook` variant | F3-014 | New action for hook execution |
| Hook planning in `compute_plan()` | F3-014 | Insert hooks at correct positions |
| Hook execution in `execute_action()` | F3-014 | Shell command execution |
| `IronState.hooks_executed` | F3-015 | HashMap<String, Vec<String>> |
| `--force-hooks` CLI flag | F3-015 | Re-run Once hooks |
| `HistoryService` | F3-016 | New service module |
| `iron history` CLI command | F3-016 | New command with list/show/last subcommands |
| `Module.dotfiles_sync` field | F3-018 | Bool flag for auto-discovery |
| `Module.dotfiles_sync_target` field | F3-018 | Override default target |
| dotfiles_sync file discovery | F3-018 | Walk module's dotfiles/ directory |
| Managed resource counts in status | SHOULD-1 | Display-only change |

---

## 4. Dependency Validation

```
F3-014 (hooks) -> F3-015 (hook tracking)
                    |
                    v
                  --force-hooks flag

F3-016 (history)    -- independent
F3-018 (dotfiles_sync) -- independent

Carryovers: independent of all tasks

F3-017 (STRETCH) -- independent
```

**Hard dependency:** F3-015 cannot start until F3-014 is complete. The `RunHook` action must exist and be executable before tracking can be added.

**Soft dependency:** F3-018 modifies `resolve_desired_state()` which F3-014's hook planning also reads. They touch different parts of the same function but do not conflict. However, implementing F3-018 first would avoid merge conflicts since it changes the module loading loop (lines 137-145 of apply.rs) while F3-014 inserts hooks into `compute_plan()` (lines 903-1145 of apply.rs).

**Recommended order:**
1. Carryovers (SHOULD-3, SHOULD-5) -- clean first, avoid diffs on lines that will change
2. F3-014 (hook execution) -- largest task, foundational
3. F3-015 (hook tracking) -- depends on F3-014
4. F3-016 (iron history) -- independent, new files only
5. F3-018 (dotfiles_sync) -- independent, modifies module.rs and apply.rs
6. Carryover SHOULD-1 (iron status) -- display-only, after core tasks
7. F3-017 (STRETCH) -- only if time permits

---

## 5. Risk Assessment

### HIGH RISK

**F3-014: Hook execution order in compute_plan()**

The current `compute_plan()` builds an `actions` vector in this order:
1. InstallPackages (line 914)
2. InstallAurPackages (line 926)
3. Dotfile actions (line 933-1033)
4. EnableService (line 1045)
5. ActivateModule (line 1056)
6. Removal diffs (lines 1062-1142)

Hooks must be inserted at specific positions:
- `pre_install` hooks BEFORE InstallPackages
- `post_install` hooks AFTER EnableService
- `pre_uninstall` hooks BEFORE RemovePackages/DeactivateModule

This requires restructuring the action list construction, not just appending. The current linear push approach must be replaced with ordered sections.

**Risk mitigation:** Build actions in named vectors (pre_hooks, installs, dotfiles, services, post_hooks, removals, pre_uninstall_hooks), then concatenate in the correct order at the end.

### MEDIUM RISK

**F3-014: Shell command execution.**

Hooks are user-defined shell commands. They can:
- Hang indefinitely (no timeout)
- Require sudo (the module already has `requires_root: bool`)
- Fail with non-zero exit codes
- Have side effects not tracked by Iron

**Risk mitigation:** Implement with a configurable timeout (default 60 seconds). Log stderr/stdout. Non-fatal by default (log error, continue).

**F3-018: dotfiles_sync modifies resolve_desired_state().**

The `resolve_desired_state()` function (lines 59-158) is called by multiple consumers (CLI apply, CLI plan, CLI diff, CLI status, TUI). Changes to it cascade everywhere. The dotfiles_sync discovery adds dotfiles to the desired state that were not explicitly declared in module.toml.

**Risk mitigation:** The change is additive (appending discovered dotfiles to the existing list). Add the discovery after the explicit dotfiles are collected (line 143), and deduplicate by target path.

### LOW RISK

- F3-015: Small state field addition with `#[serde(default)]` -- backward compatible
- F3-016: Entirely new files, no modifications to existing code beyond registration
- SHOULD-1: Display-only change in status.rs
- SHOULD-3/5: Formatting and comment cleanup

---

## 6. Implementation Order

### Wave 1: Cleanup + Foundation

1. **SHOULD-3**: Run `cargo fmt --all`
2. **SHOULD-5**: Remove temporal comments from `apply.rs` (3 locations)

### Wave 2: Hook Execution (F3-014 + F3-015)

3. **F3-014 Part A**: Define `HookBehavior` and `HookType` enums in `module.rs`. Add `hook_behavior` field to `Module`. Update all test helpers.
4. **F3-014 Part B**: Add `RunHook` variant to `ApplyAction`. Add `risk_level()`, `display()`, `summary()`, `record_managed_resource()` match arms.
5. **F3-014 Part C**: Insert hook planning into `compute_plan()`. Restructure action ordering.
6. **F3-014 Part D**: Implement `execute_action()` for `RunHook` -- shell command execution with timeout and error handling.
7. **F3-015**: Add `hooks_executed` to `IronState`. Add tracking methods to `StateManager`. Wire Once behavior into plan and execute. Add `--force-hooks` flag.

### Wave 3: Independent Features

8. **F3-016**: Create `HistoryService` in `iron-core`. Create `history` command in `iron-cli`. Register in CLI and dispatch.
9. **F3-018**: Add `dotfiles_sync` and `dotfiles_sync_target` to `Module`. Implement file discovery in `resolve_desired_state()`. Update test helpers.

### Wave 4: Polish

10. **SHOULD-1**: Wire managed counts and last_apply into `iron status` output.
11. **F3-017** (STRETCH): Create `config` command namespace if time permits.

---

## 7. Test Strategy

### F3-014: Hook Execution (8+ tests)

| Test | What It Verifies |
|------|-----------------|
| `test_hook_behavior_default` | `HookBehavior` defaults to `Always` |
| `test_hook_behavior_serde_roundtrip` | All variants serialize/deserialize correctly in TOML and JSON |
| `test_compute_plan_includes_hooks` | Module with `post_install` produces `RunHook` action in plan |
| `test_hook_ordering` | pre_install hooks appear before InstallPackages; post_install after EnableService |
| `test_skip_behavior_omits_hook` | Module with `hook_behavior = "Skip"` produces no RunHook actions |
| `test_hook_display` | `RunHook::display()` returns correct format string |
| `test_hook_risk_level` | `RunHook.risk_level()` returns `Destructive` |
| `test_execute_hook_success` | Shell command execution succeeds (mock or simple `echo` command) |
| `test_execute_hook_failure_non_fatal` | Failed hook logs error but does not stop apply |
| `test_dry_run_shows_hooks` | `--dry-run` includes RunHook actions in plan output |

### F3-015: Hook Tracking (6+ tests)

| Test | What It Verifies |
|------|-----------------|
| `test_hooks_executed_serde_default` | Empty HashMap on deserialization of old state.json |
| `test_record_hook_executed` | After recording, `is_hook_executed` returns true |
| `test_once_hook_skipped_on_second_apply` | Once hook in plan on first apply, absent on second |
| `test_force_hooks_reruns_once` | `--force-hooks` causes Once hook to appear in plan |
| `test_clear_hooks_on_module_disable` | Disabling module clears its hooks_executed entries |
| `test_hooks_executed_backward_compat` | Old state.json without field loads with empty HashMap |

### F3-016: iron history (5+ tests)

| Test | What It Verifies |
|------|-----------------|
| `test_history_empty` | No operations returns empty list gracefully |
| `test_history_list_operations` | Multiple operations displayed in reverse chronological order |
| `test_history_show_detail` | Detail view shows actions and errors for a specific operation |
| `test_history_last_shortcut` | `iron history last` returns most recent operation |
| `test_history_limit` | `--limit 5` shows at most 5 entries |
| `test_cli_history_parsing` | CLI argument parsing for all subcommands |

### F3-018: dotfiles_sync (6+ tests)

| Test | What It Verifies |
|------|-----------------|
| `test_dotfiles_sync_discovery` | Files in module's `dotfiles/` directory become DotfileMapping entries |
| `test_dotfiles_sync_preserves_structure` | Nested directory `dotfiles/lua/init.lua` maps to `<target>/lua/init.lua` |
| `test_dotfiles_sync_explicit_override` | Explicit `[[dotfiles]]` entry takes precedence over auto-discovered entry with same target |
| `test_dotfiles_sync_custom_target` | `dotfiles_sync_target` overrides default `~/.config/<id>/` |
| `test_dotfiles_sync_false_default` | Module without `dotfiles_sync` field defaults to `false` (no auto-discovery) |
| `test_dotfiles_sync_empty_dir` | Empty `dotfiles/` directory produces no additional mappings |
| `test_dotfiles_sync_hyphen_warning` | Module ID with hyphens using default target logs a warning |

### Carryover: SHOULD-1 (2+ tests)

| Test | What It Verifies |
|------|-----------------|
| `test_status_shows_managed_counts` | Text output includes managed package/service/dotfile counts |
| `test_status_json_includes_managed` | JSON envelope includes managed counts and last_apply timestamp |

### Integration Tests (all use --dry-run)

```bash
iron history                    # exits 0 (read-only, no --dry-run needed)
iron history list               # exits 0
iron history last               # exits 0 (even with empty history)
iron apply --dry-run            # still works, shows RunHook if applicable
iron apply --force-hooks --dry-run  # accepted flag
```

---

## 8. Acceptance Criteria

### AC-014: Hook Execution in Apply Lifecycle

- AC-014-1: `HookBehavior` enum exists with `Always`, `Once`, `Ask`, `Skip` variants. `Default` is `Always`.
- AC-014-2: `HookType` enum exists with `PreInstall`, `PostInstall`, `PreUninstall`, `StatusCheck` variants.
- AC-014-3: `Module.hook_behavior` field exists with `#[serde(default)]`.
- AC-014-4: `ApplyAction::RunHook { module_id, hook_type, command, behavior }` variant exists.
- AC-014-5: `compute_plan()` inserts `RunHook` actions at correct positions: `pre_install` before package installs, `post_install` after service enables.
- AC-014-6: `Always` hooks produce RunHook actions on every plan.
- AC-014-7: `Skip` hooks never produce RunHook actions.
- AC-014-8: `Ask` hooks produce RunHook actions but prompt user before execution (skip in `--yes` mode, skip in TUI with warning).
- AC-014-9: Hook execution runs command via `std::process::Command` or `CommandExecutor` (architect decides).
- AC-014-10: Hook failure is non-fatal by default: log error, continue with remaining actions.
- AC-014-11: `--dry-run` shows hooks that would run without executing them.
- AC-014-12: `risk_level()` returns `Destructive` for `RunHook`.
- AC-014-13: All test helpers updated for new `Module` field.
- AC-014-14: 8+ unit tests pass.

### AC-015: Hook Execution Tracking

- AC-015-1: `IronState.hooks_executed: HashMap<String, Vec<String>>` field exists with `#[serde(default)]`.
- AC-015-2: After successful `Once` hook execution, entry recorded in state.
- AC-015-3: `Once` hooks check `hooks_executed` at plan time -- skip if already recorded.
- AC-015-4: `--force-hooks` flag on `iron apply` forces Once hooks to be re-planned.
- AC-015-5: `--force-hooks --dry-run` shows Once hooks without executing.
- AC-015-6: Disabling a module clears its `hooks_executed` entries.
- AC-015-7: Old state.json without `hooks_executed` field loads with empty HashMap.
- AC-015-8: 6+ unit tests pass.

### AC-016: iron history Command

- AC-016-1: `iron history` (or `iron history list`) shows recent operations in table format with columns: #, Time, Command, Duration, Actions, Status.
- AC-016-2: `iron history show <id>` shows detailed view of a specific operation.
- AC-016-3: `iron history last` shows the most recent operation in detail.
- AC-016-4: `--json` output uses response envelope.
- AC-016-5: `--limit <n>` controls number of entries (default 20).
- AC-016-6: Reads from audit log in state directory (respects XDG path).
- AC-016-7: Empty history displays gracefully (no crash, informative message).
- AC-016-8: Registered in CLI `Commands` enum and `main.rs` dispatch.
- AC-016-9: CLI parsing tests exist.
- AC-016-10: 5+ unit tests pass.

### AC-018: dotfiles_sync Auto-Mirror

- AC-018-1: `Module.dotfiles_sync: bool` field exists with `#[serde(default)]` (default false).
- AC-018-2: `Module.dotfiles_sync_target: Option<String>` field exists with `#[serde(default)]`.
- AC-018-3: When `dotfiles_sync = true`, files in `modules/<id>/dotfiles/` are auto-discovered and added to desired state as `DotfileMapping` entries.
- AC-018-4: Preserves subdirectory structure recursively.
- AC-018-5: Explicit `[[dotfiles]]` entries override auto-discovered entries for the same target path.
- AC-018-6: Default target is `~/.config/<module-id>/` when `dotfiles_sync_target` is not set.
- AC-018-7: Custom `dotfiles_sync_target` overrides the default.
- AC-018-8: Template detection works on auto-discovered files (files with `{{var}}` get `RenderAndCopy`).
- AC-018-9: Warning logged when module ID contains hyphens and default target is used.
- AC-018-10: All test helpers updated for new fields.
- AC-018-11: 6+ unit tests pass.

### AC-SHOULD-1: iron status Managed Resource Display

- AC-SHOULD-1-1: `iron status` text output includes a "Managed State" section showing managed package, service, and dotfile counts.
- AC-SHOULD-1-2: `iron status` text output shows `last_apply` timestamp as relative time (e.g., "2h ago").
- AC-SHOULD-1-3: `iron status --format json` includes `managed_packages_count`, `managed_services_count`, `managed_dotfiles_count`, and `last_apply` fields in the envelope data.

### AC-SHOULD-3: Formatting

- AC-SHOULD-3-1: `cargo fmt --all -- --check` exits with 0.

### AC-SHOULD-5: Temporal Comments

- AC-SHOULD-5-1: No comments in the codebase contain "Sprint 3.2" or "Sprint 3.1" references.

---

## 9. Architectural Questions for Architect

### AQ-014-1: Hook Execution Mechanism

Should hooks run via `CommandExecutor` (circuit breaker semantics, retries) or via raw `std::process::Command`? Hooks are user-defined shell commands that may not benefit from circuit breakers. The `CommandExecutor` trait is designed for known system commands (pacman, systemctl, git). Hooks are arbitrary.

**Analyst recommendation:** Use `std::process::Command` directly, with a configurable timeout (default 60s). This avoids coupling hooks to infrastructure-level resilience patterns that are designed for predictable commands.

### AQ-014-2: Hook Working Directory

What should the working directory be when executing a hook?
- Option A: Module directory (`iron_root/modules/<module-id>/`)
- Option B: User home directory
- Option C: Iron root directory

**Analyst recommendation:** Option A -- the module directory. This lets hooks reference module-local files (e.g., `./scripts/setup.sh`).

### AQ-014-3: Hook Ordering Restructure

The current `compute_plan()` pushes actions linearly into a single `Vec<ApplyAction>`. To insert hooks at correct positions, should we:
- Option A: Build separate vectors and concatenate
- Option B: Add an `order` or `phase` field to `ApplyAction` and sort at the end
- Option C: Insert hooks at computed indices in the single vector

**Analyst recommendation:** Option A -- separate vectors concatenated. Simplest, most readable, no sort needed.

### AQ-014-4: Ask Behavior in Non-Interactive Contexts

When `hook_behavior = Ask` and the context is non-interactive (`--yes` flag, TUI, CI pipeline), should the hook:
- Option A: Skip with warning
- Option B: Run with info log ("Auto-approved in non-interactive mode")
- Option C: Fail the plan

**Analyst recommendation:** Option A -- skip with warning. "Ask" implies user consent; auto-running defeats the purpose.

### AQ-016-1: History Data Model

The current audit log entries are low-level (enable_module, set_current_host, persist, etc.). The history command needs higher-level "operation" groupings. Should we:
- Option A: Aggregate consecutive audit entries by timestamp proximity into operations
- Option B: Add a new `OperationHistory` model that records start/end/actions at apply/update time
- Option C: Use the existing `IronState.last_operations: Vec<OperationRecord>` which already has operation-level records

**Analyst recommendation:** Option C -- use `last_operations`. It already records operations at the right granularity. Enhance `OperationRecord` with `duration_secs` and `action_count` fields if needed. The audit log is supplementary detail.

### AQ-018-1: dotfiles_sync Discovery Location

Should dotfiles_sync discovery happen:
- Option A: In `resolve_desired_state()` (apply.rs) -- at desired state resolution time
- Option B: In `compute_plan()` -- at plan computation time
- Option C: In a new `Module::resolved_dotfiles()` method

**Analyst recommendation:** Option A -- in `resolve_desired_state()`. This ensures all consumers (plan, diff, status) see the same desired state. Discovery must happen at the same point where `module.dotfiles` are collected (line 143 of apply.rs).

---

## 10. Stretch Task Assessment: F3-017 (iron config namespace)

### Should F3-017 Be Implemented?

**Recommendation: DEFER to Phase 4.**

**Reasons:**

1. **No blocking dependency.** No Sprint 3.4 task depends on `iron config`. No Phase 4 task depends on it.

2. **Low user impact.** The existing `iron validate` command already covers the most critical use case. `iron config path` is trivially achievable with `echo ~/.config/iron`. `iron config edit` is `$EDITOR ~/.config/iron`.

3. **Sprint scope is already full.** F3-014 (hook execution) is a Large task with structural changes to `compute_plan()`. F3-018 modifies `resolve_desired_state()`. F3-016 adds new files. Combined with 3 carryovers, Sprint 3.3 has substantial scope.

4. **Risk of scope creep.** The `iron config show` subcommand (resolved config summary) duplicates parts of `iron status`. Defining clear boundaries between `iron config show` and `iron status` adds design overhead.

5. **Aliasing complexity.** Making `iron validate` an alias for `iron config validate` without deprecation warning, while keeping both paths working, is fiddly.

**If implemented anyway:** Implement as the last task, after all other tasks pass validation. Limit to `iron config path` and `iron config edit` only. Defer `iron config validate` aliasing and `iron config show` to Phase 4.

---

## 11. TUI Impact Summary

| Task | TUI Impact | Severity |
|------|-----------|----------|
| F3-014 | Apply view must render `RunHook` actions in plan list. `display()` method handles formatting. No new View variant needed. | Low -- display-only addition |
| F3-014 | `Ask` behavior in TUI: skip with warning log. No interactive prompt needed in TUI. | Low |
| F3-015 | No TUI changes. Hook tracking is state-only. | None |
| F3-016 | No TUI view for history. CLI-only command. | None |
| F3-018 | Auto-discovered dotfile actions appear in apply view automatically via existing `display()` formatting. No new View variant needed. | None |
| F3-017 | No TUI impact. CLI-only namespace. | None |

**No new TUI View variants.** No updates to the 7 exhaustive match locations.
