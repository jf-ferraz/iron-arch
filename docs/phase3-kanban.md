# Phase 3 — Sprint Kanban Board

> **Phase:** 3 — Declarative Convergence & Multi-Machine Readiness
> **Sprints:** 3.1 (Foundation Contracts) + 3.2 (Declarative Convergence) + 3.3 (Lifecycle Completion) + 3.4 (Multi-Machine)
> **Estimated Duration:** 4 sprints + 1 buffer (~10 weeks)
> **Branch Convention:** `phase3/F3-XXX-short-description`
> **Commit Convention:** `F3-XXX: short description`
> **Status:** PLANNING
> **Depends On:** Phase 2 ✅ Complete (19/19 tasks, 2026-02-22)
> **Inputs:** Gap analysis (5 critical + 8 significant gaps), benchmark analysis (dcli/arch-config)
>
> **Note on Phase Numbering:** The original `product-review-and-roadmap.md` defined Phase 3 as "Ecosystem & Community" (import/sharing/registry). That scope has been deferred to Phase 4. This Phase 3 addresses foundational gaps that must be resolved before ecosystem features are viable. See Decision D7.

---

## Phase 3 Overview

**Goal:** Complete the declarative pipeline and prepare for multi-machine workflows.

After Phase 3, a user can:

1. Run `iron status` for a complete system overview from the CLI
2. Run `iron plan` to see what would change without applying
3. Use `{{variable}}` templates in dotfiles for host-specific rendering
4. Have packages/services/symlinks **removed** when no longer declared (`--prune`)
5. Module hooks execute automatically during apply (post-install scripts, etc.)
6. Runtime state lives in `~/.local/state/iron/` — config dir is purely declarative
7. Auto-detect host by system hostname
8. Compare two host configurations side-by-side
9. All `--json` output wrapped in consistent response envelope
10. View operation history via `iron history`

**Mental model shift:** Iron goes from "a safety net for experimentation" to "a declarative system that converges exactly to what you declare — including removing what you don't."

### Phase 2 Lessons Applied

| # | Lesson | How We Apply It in Phase 3 |
|---|--------|---------------------------|
| L1 | Test helpers break when struct fields added | All new State/Host/Module fields use `#[serde(default)]` and update test helpers in same PR |
| L2 | Integration tests hang on sudo/TUI | All new CLI commands include `--dry-run` or are read-only |
| L3 | `#[allow(dead_code)]` accumulates on output methods | New Output methods wired into commands immediately |
| L4 | Auto-snapshot needs PackageManager injection | ActualState scan centralizes all system queries |
| L5 | Snapshot restore must converge via ApplyService | All state mutations followed by apply convergence |
| L6 | Fragile string parsing for sorting/scoring | Risk levels and hook behaviors use typed enums |
| L7 | Truncate must be UTF-8 safe | Use existing `truncate_str()` from output.rs |
| L8 | Service traits need Send+Sync | All new traits include `Send + Sync` bounds from day 1 |
| L9 | API refactors cascade through CLI + TUI | F3-002b lists all affected files; TUI impact noted per task |

### Dependency Graph

```
Sprint 3.1 (Foundation):
F3-001 → F3-002a → F3-002b ──→ F3-004, F3-005
F3-003a → F3-003b ─────────→ F3-004, F3-005
F3-006 → F3-007

Sprint 3.2 (Convergence):
F3-021 ──→ F3-010, F3-011, F3-012   (managed tracking MUST precede removal)
F3-008 → F3-009
F3-013 (depends on F3-009, F3-010, F3-011, F3-012)

Sprint 3.3 (Lifecycle):
F3-014 → F3-015
F3-016, F3-018 (parallel, mostly independent)
F3-017 (STRETCH — nice-to-have)

Sprint 3.4 (Multi-Machine):
F3-019 (independent)
F3-020 (STRETCH — depends on DesiredState resolver)
F3-022 (STRETCH — depends on F3-001)
```

### Stretch Tasks

These tasks can slip to Phase 4 without blocking Phase 3 goals:
- **F3-017** (iron config namespace) — organizational, not blocking
- **F3-020** (host comparison) — nice-to-have, not blocking
- **F3-022** (scan serialization) — nice-to-have, not blocking

---

## Sprint 3.1 — Foundation Contracts

**Goal:** Define `ActualState` contract, separate runtime state, standardize JSON output, add `iron status` and `iron plan`.

### Backlog (9 tasks)

---

#### F3-001: `ActualState` struct and contract
**File:** `iron-core/src/actual_state.rs` (new)
**Priority:** 🔴 Critical (blocks F3-002a, F3-004, F3-005, F3-022)
**Effort:** M
**Gaps Addressed:** C2

**Description:**
Define the `ActualState` struct that represents a point-in-time snapshot of the real system state. This is the counterpart to `DesiredState` — the apply plan is their diff.

**Acceptance Criteria:**
- [ ] `ActualState` struct with fields: `hostname`, `installed_packages`, `aur_packages`, `services: Vec<ActualServiceState>`, `managed_files: Vec<ActualFileState>`, `scanned_at`
- [ ] `ActualServiceState` struct: `name`, `enabled`, `running`
- [ ] `ActualFileState` struct: `target`, `exists`, `symlink_target`, `checksum`, `file_type`
- [ ] `FileStateType` enum: `Symlink`, `Regular`, `Missing`, `Directory`
- [ ] All structs derive `Debug, Clone, Serialize, Deserialize`
- [ ] All optional fields use `#[serde(default)]`
- [ ] Unit tests: construction, serialization roundtrip, deserialization with missing fields
- [ ] Registered in `iron-core/src/lib.rs` as public module

---

#### F3-002a: `scan_actual_state()` implementation
**File:** `iron-core/src/actual_state.rs`
**Priority:** 🔴 Critical (blocks F3-002b)
**Effort:** M

**Description:**
Implement `ActualState::scan()` that queries the real system once and captures all relevant state. This is the scan implementation only — refactoring consumers is F3-002b.

**Acceptance Criteria:**
- [ ] `ActualState::scan(package_manager, service_manager, managed_files)` queries system
- [ ] Uses `PackageManager::query_installed()` for packages
- [ ] Uses `PackageManager::query_aur_installed()` for AUR packages (graceful fallback)
- [ ] Uses `SystemService::is_enabled()` for each declared service
- [ ] Uses `std::fs::read_link()` and `std::fs::metadata()` for managed files
- [ ] Computes SHA256 checksums for regular files (not symlinks)
- [ ] `hostname` crate (or `gethostname`) for system hostname
- [ ] Unit tests with mocked PackageManager and SystemService (8+)
- [ ] Serialization roundtrip test (scan → JSON → deserialize → assert equal)

---

#### F3-002b: Refactor `compute_plan()` and `detect()` to consume `ActualState`
**File:** Multiple files (see list below)
**Priority:** 🔴 Critical (blocks F3-004, F3-005)
**Effort:** L

**Description:**
Refactor both `ApplyService::compute_plan()` and `DriftService::detect()` to accept `&ActualState` instead of querying the system independently. This is a breaking API change that cascades through all consumers.

**Affected Files:**
- `iron-core/src/services/apply.rs` — `compute_plan()` signature change
- `iron-core/src/services/drift.rs` — `detect()` signature change
- `iron-cli/src/commands/apply.rs` — caller: scan ActualState, pass to compute_plan
- `iron-cli/src/commands/diff.rs` — caller: scan ActualState, pass to detect
- `iron-cli/src/commands/snapshot.rs` — restore flow calls compute_plan
- `iron-tui/src/ui/apply.rs` — TUI apply view calls compute_plan
- `iron-tui/src/app/actions.rs` — TUI action dispatchers
- All integration tests that call these functions

**Acceptance Criteria:**
- [ ] `ApplyService::compute_plan()` accepts `&ActualState` parameter
- [ ] `DriftService::detect()` accepts `&ActualState` parameter
- [ ] Both methods remove internal `package_manager.query_installed()` calls
- [ ] All CLI callers updated: scan ActualState first, pass to service
- [ ] All TUI callers updated: scan ActualState first, pass to service
- [ ] Snapshot restore flow updated
- [ ] No duplicate system queries between plan and drift
- [ ] All existing tests pass after refactor
- [ ] No new `#[allow(dead_code)]` introduced

**TUI Impact:** Apply view and drift detail view — update action dispatch to scan first.

---

#### F3-003a: Response envelope infrastructure
**File:** `iron-core/src/envelope.rs` (new), `iron-cli/src/output.rs`
**Priority:** 🔴 Critical
**Effort:** M
**Gaps Addressed:** C3

**Description:**
Define `IronEnvelope<T>` wrapper and add `Output::json_envelope()` method. This task defines the infrastructure only — migration of existing commands is F3-003b.

**Acceptance Criteria:**
- [ ] `IronEnvelope<T>` struct: `ok`, `command`, `data: Option<T>`, `error: Option<EnvelopeError>`, `meta: EnvelopeMeta`
- [ ] `EnvelopeError`: `code`, `message`, `suggestion`, `details`
- [ ] `EnvelopeMeta`: `timestamp`, `duration_ms`, `host`, `version`
- [ ] `IronEnvelope::success()` and `IronEnvelope::error()` constructors
- [ ] `Output::json_envelope()` method wraps data in envelope
- [ ] `Output::json_error_envelope()` method wraps IronError in error envelope
- [ ] Unit tests: success envelope, error envelope, serialization, meta fields populated (6+)
- [ ] Registered in `iron-core/src/lib.rs` as public module

---

#### F3-003b: Migrate existing `--json` commands to envelope
**File:** `iron-cli/src/commands/*.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Migrate all existing CLI commands with `--json` output to use `Output::json_envelope()` instead of raw `json_value()`.

**Affected Files:**
- `iron-cli/src/commands/apply.rs` — `--json` plan output
- `iron-cli/src/commands/diff.rs` — `--json` drift report
- `iron-cli/src/commands/snapshot.rs` — `--json` list/create output
- `iron-cli/src/commands/security.rs` — `--json` security report
- `iron-cli/src/commands/scan.rs` — `--json` scan output
- `iron-cli/src/commands/module.rs` — `--json` module list
- `iron-cli/src/commands/validate.rs` — `--json` validation result

**Acceptance Criteria:**
- [ ] All commands that emit `--json` output use `output.json_envelope()` or `output.json_error_envelope()`
- [ ] All JSON output includes `ok`, `command`, `data`, `meta` fields
- [ ] Error JSON output includes `error.code`, `error.message`, `error.suggestion`
- [ ] `meta.timestamp` and `meta.duration_ms` populated correctly
- [ ] Integration test: `iron diff --json --dry-run` returns valid envelope
- [ ] Integration test: at least 2 other commands verified with envelope format
- [ ] No commands use raw `output.json_value()` for structured output after migration

---

#### F3-004: CLI `iron status` command
**File:** `iron-cli/src/commands/status.rs` (new)
**Priority:** 🔴 Critical
**Effort:** M
**Gaps Addressed:** S1

**Description:**
Quick system overview showing active host, bundle, profile, modules, drift summary, security level, and sync status.

**Performance Note:** This command should target < 2 second latency. It uses `StateManager` data (instant) plus lightweight system checks. It does NOT perform a full `ActualState::scan()` — instead it reads cached state and performs quick spot-checks (e.g., count installed packages via `pacman -Qq | wc -l` rather than full query). For full accuracy, direct users to `iron diff` or `iron plan`.

**Acceptance Criteria:**
- [ ] Shows: active host (with auto-detect indicator), bundle, profile, module count
- [ ] Shows: package summary (N managed) — from state.managed_packages, not live pacman query
- [ ] Shows: security level + score
- [ ] Shows: last apply timestamp, last sync timestamp
- [ ] Shows: drift indicator (quick check: count of diverged modules from existing TUI logic)
- [ ] `--full` flag triggers full `ActualState::scan()` for accurate package/service/dotfile counts
- [ ] `--json` output uses response envelope
- [ ] `--dry-run` flag (for integration tests)
- [ ] Registered in `cli.rs` Commands enum and `main.rs` dispatch
- [ ] CLI parsing tests

---

#### F3-005: CLI `iron plan` command
**File:** `iron-cli/src/commands/plan.rs` (new)
**Priority:** 🔴 Critical
**Effort:** M
**Gaps Addressed:** C5

**Description:**
Generate and display an apply plan without executing. The plan is a first-class read-only command, distinct from `iron apply --dry-run` (which prompts for confirmation).

**Scope Note:** Plan serialization (`--output plan.json`) and plan replay (`iron apply --plan <file>`) are deferred to Phase 4. The `ApplyPlan` contains trait object references that require a serialization strategy and staleness detection — not worth the complexity in Phase 3.

**Acceptance Criteria:**
- [ ] `iron plan` generates full system plan and displays it (no confirmation prompt)
- [ ] `iron plan --module <id>` filters plan to one module
- [ ] `iron plan --json` outputs plan in envelope format
- [ ] Plan uses `ActualState::scan()` + `resolve_desired_state()` + `compute_plan()`
- [ ] Plan display uses tree output for actions, shows risk level per action (after F3-013)
- [ ] `--dry-run` flag (for integration tests — plan is always read-only but flag exists for consistency)
- [ ] Registered in `cli.rs` Commands enum and `main.rs` dispatch
- [ ] CLI parsing tests

**Difference from `iron apply --dry-run`:**
- `iron plan` is purely read-only — never prompts for confirmation
- `iron apply --dry-run` shows the plan AND asks "would you like to proceed?" (exit without applying)

---

#### F3-006: State directory resolution (XDG separation)
**File:** `iron-core/src/services/state.rs`, `iron-cli/src/context.rs`
**Priority:** 🔴 Critical (blocks multi-machine Git sync)
**Effort:** M
**Gaps Addressed:** S4

**Description:**
Move runtime state (`state.json`, `audit.log`, `.state.lock`, `.snapshots/`) from the Git-tracked config directory to `~/.local/state/iron/` (XDG state directory).

**Dependency:** Verify `dirs` crate is in dependency tree. If not, add `dirs = "5"` to `iron-core/Cargo.toml`. Alternatively, implement XDG resolution manually using `$XDG_STATE_HOME` env var with `~/.local/state` fallback.

**Acceptance Criteria:**
- [ ] `StateManager::state_dir()` resolves: `$IRON_STATE_DIR` > `$XDG_STATE_HOME/iron` > `~/.local/state/iron`
- [ ] `StateManager::new()` uses `state_dir()` for state.json location
- [ ] `AuditLog` uses `state_dir()` for audit.log location
- [ ] `SnapshotService` uses `state_dir()/snapshots/` for snapshot storage
- [ ] Lock file uses `state_dir()` for .state.lock location
- [ ] `state_dir()` creates directory if it doesn't exist
- [ ] `$IRON_STATE_DIR` env var overrides for testing
- [ ] Unit tests: resolution priority (env > XDG > default)
- [ ] Verify `dirs` crate available or add dependency

---

#### F3-007: Legacy state migration
**File:** `iron-core/src/services/state.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
On startup, detect if state files exist in the legacy location (config dir) and migrate them to the new XDG state directory. Use copy-then-delete for safety.

**Acceptance Criteria:**
- [ ] `StateManager::migrate_if_needed(config_root)` checks for legacy `state.json`
- [ ] Migrates: `state.json`, `audit.log`, `.state.lock`, `.snapshots/` directory
- [ ] Uses **copy-then-delete** (not move) — if copy succeeds, delete original
- [ ] Leaves a `MIGRATED.txt` marker in old location with message: "State migrated to ~/.local/state/iron/ by Iron vX.Y.Z on DATE"
- [ ] Creates `state_dir()` if needed
- [ ] Logs migration action to audit log (after migration)
- [ ] No-op if new location already has state.json
- [ ] No-op if legacy location has no state.json
- [ ] On failure: leaves original files intact, logs warning, continues with legacy location
- [ ] Unit tests: migration from legacy, no-op when already migrated, no-op when no legacy, failure recovery

---

## Sprint 3.2 — Full Declarative Convergence

**Goal:** Managed resource tracking, template rendering, file copy, removal actions, risk levels.

### Backlog (8 tasks)

**IMPORTANT:** F3-021 (Managed Resource Tracking) MUST be implemented before F3-010/011/012 (removal actions). The removal tasks consume `State.managed_packages/services/dotfiles` data that F3-021 creates.

---

#### F3-021: Managed resource tracking
**File:** `iron-core/src/services/state.rs`, `iron-core/src/services/apply.rs`
**Priority:** 🔴 Critical (prerequisite for F3-010, F3-011, F3-012)
**Effort:** M
**Gaps Addressed:** BM-2 (dcli benchmark pattern)

**Description:**
Track which packages, services, and dotfiles were installed/created by Iron. Essential for safe removal in F3-010/011/012. **Must be implemented first in Sprint 3.2.**

**Acceptance Criteria:**
- [ ] `State.managed_packages: Vec<String>` — packages installed by Iron
- [ ] `State.managed_services: Vec<String>` — services enabled by Iron
- [ ] `State.managed_dotfiles: Vec<String>` — dotfile targets created by Iron
- [ ] All three fields use `#[serde(default)]`
- [ ] After `InstallPackages` execution: record packages in managed_packages
- [ ] After `EnableService` execution: record service in managed_services
- [ ] After `CreateSymlink`/`CopyFile`/`RenderAndCopy` execution: record target in managed_dotfiles
- [ ] After removal execution: remove from managed lists
- [ ] Bootstrap: on first apply, mark all installed+desired packages as managed
- [ ] Unit tests: record, unrecord, bootstrap, persistence across apply cycles
- [ ] Update test helpers for new State fields

---

#### F3-008: Template variable rendering in apply pipeline
**File:** `iron-core/src/services/apply.rs`, `iron-fs/src/lib.rs`
**Priority:** 🔴 Critical
**Effort:** L
**Gaps Addressed:** C1

**Description:**
Wire the existing iron-fs template engine into the apply pipeline. When a dotfile source contains `{{variable}}` patterns, render with host variables and deploy as a copy (not symlink).

**Acceptance Criteria:**
- [ ] `iron_fs::template::has_variables(content)` function detects `{{...}}` patterns
- [ ] During `compute_plan()`, dotfiles with templates produce `RenderAndCopy` actions (not `CreateSymlink`)
- [ ] Dotfiles without templates continue producing `CreateSymlink` actions
- [ ] `RenderAndCopy` execution: render template → backup existing → write rendered content
- [ ] Host variables from `DesiredState.variables` are passed to template engine
- [ ] Built-in variables added: `{{hostname}}`, `{{username}}`, `{{home}}`
- [ ] `--dry-run` shows rendered diff (before/after preview)
- [ ] Unit tests: template detection, rendering, render+copy execution, built-in variables
- [ ] Integration test: module with `{{terminal}}` variable renders correctly

**TUI Impact:** Apply view must render `RenderAndCopy` actions (display as "Render + copy: <target>").

---

#### F3-009: File copy deployment mode (CopyFile action)
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** S5

**Description:**
Add `CopyFile` action type to `ApplyAction`. When `DotfileMapping.link = false` (already exists but is a no-op), deploy as copy instead of symlink.

**Acceptance Criteria:**
- [ ] `ApplyAction::CopyFile { source, target, backup_existing }` variant added
- [ ] `DotfileMapping.link = false` produces `CopyFile` action in plan
- [ ] `CopyFile` execution: backup existing target (if backup_existing) → copy source to target
- [ ] Creates parent directories as needed
- [ ] Drift detection recognizes copies (checksum comparison, not symlink check)
- [ ] Unit tests: copy action planning, execution with backup, execution without backup
- [ ] `--dry-run` shows copy action in plan output

**TUI Impact:** Apply view must render `CopyFile` actions (display as "Copy: <source> → <target>").

---

#### F3-010: Package removal in apply (RemovePackages action)
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🔴 Critical
**Effort:** L
**Gaps Addressed:** S8
**Depends On:** F3-021 (managed resource tracking)

**Description:**
Add `RemovePackages` action. Only remove packages that are tracked as "managed by Iron" and are no longer in the desired state. Requires `--prune` flag (or `--prune-packages` for granular control).

**Acceptance Criteria:**
- [ ] `ApplyAction::RemovePackages { packages }` variant added
- [ ] Plan computation: packages in managed_packages AND installed AND NOT in desired_packages → removal candidates
- [ ] Removal candidates shown in plan output even without `--prune` (with hint message)
- [ ] Executed when `--prune` (all types) or `--prune-packages` (packages only) flag is passed
- [ ] Execution: calls `PackageManager::remove(packages)`
- [ ] After execution: updates `managed_packages` in state
- [ ] After install execution: records newly installed packages in `managed_packages`
- [ ] **Safety:** Never removes packages not in `managed_packages` — protects manually installed packages
- [ ] Unit tests: removal planning, managed tracking, safety (unmanaged not removed)
- [ ] `PackageManager` trait: add `remove(packages: &[&str]) -> IronResult<()>` method with default impl

**TUI Impact:** Apply view must render `RemovePackages` actions with risk badge (display as "[!!] Remove packages: ...").

---

#### F3-011: Service disable action (DisableService)
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** S8
**Depends On:** F3-021 (managed resource tracking)

**Description:**
Add `DisableService` action for services that are managed by Iron but no longer in desired state.

**Acceptance Criteria:**
- [ ] `ApplyAction::DisableService { name }` variant added
- [ ] Plan: services in managed_services AND enabled AND NOT in desired → disable candidates
- [ ] Requires `--prune` or `--prune-services` flag
- [ ] Execution: calls `SystemService::disable(name)`
- [ ] After execution: updates managed_services in state
- [ ] `SystemService` trait: add `disable(name: &str) -> IronResult<()>` method
- [ ] Unit tests: disable planning, managed tracking

**TUI Impact:** Apply view must render `DisableService` actions with risk badge.

---

#### F3-012: Symlink and file removal (RemoveSymlink, DeactivateModule)
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** S8
**Depends On:** F3-021 (managed resource tracking)

**Description:**
Add `RemoveSymlink` and `DeactivateModule` actions for managed dotfiles/modules no longer in desired state.

**Acceptance Criteria:**
- [ ] `ApplyAction::RemoveSymlink { target }` variant added
- [ ] `ApplyAction::DeactivateModule { id }` variant added
- [ ] Plan: managed symlinks whose module is no longer active → removal candidates
- [ ] Requires `--prune` or `--prune-dotfiles` flag
- [ ] `RemoveSymlink` execution: backup target → remove symlink
- [ ] `DeactivateModule` execution: remove module from enabled list in state
- [ ] Unit tests: removal planning, execution with backup

**TUI Impact:** Apply view must render `RemoveSymlink` and `DeactivateModule` actions with risk badge.

---

#### F3-013: Risk levels on ApplyAction
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** S
**Gaps Addressed:** S6

**Description:**
Add `RiskLevel` enum and `risk_level()` method to `ApplyAction`. Confirmation policy scales with risk.

**Acceptance Criteria:**
- [ ] `RiskLevel` enum: `ReadOnly`, `Additive`, `Destructive`, `Critical`
- [ ] `ApplyAction::risk_level()` returns appropriate level for each variant
- [ ] `ApplyPlan::max_risk()` returns highest risk across all actions
- [ ] Plan output shows risk level badge per action (e.g., `[+]` additive, `[!]` destructive, `[!!]` critical)
- [ ] Confirmation prompt scales: Additive → simple, Destructive → detailed, Critical → typed
- [ ] `--json` plan output includes risk_level per action
- [ ] Unit tests: risk classification for each action type, max_risk computation

**TUI Impact:** Apply view should color-code actions by risk level. Dashboard drift indicator should reflect risk.

---

## Sprint 3.3 — Execution Lifecycle Completion

**Goal:** Hook execution, operation history, config namespace, dotfiles_sync.

### Backlog (5 tasks)

---

#### F3-014: Hook execution in apply lifecycle
**File:** `iron-core/src/services/apply.rs`, `iron-core/src/module.rs`
**Priority:** 🔴 Critical
**Effort:** L
**Gaps Addressed:** S7

**Description:**
Wire module hooks (`pre_install`, `post_install`, `pre_uninstall`, `status_check`) into the apply pipeline. Add `HookBehavior` enum for execution policy.

**Acceptance Criteria:**
- [ ] `HookBehavior` enum: `Always`, `Once`, `Ask`, `Skip` (with `Default = Always`)
- [ ] `Module.hook_behavior: HookBehavior` field added with `#[serde(default)]`
- [ ] `ApplyAction::RunHook { module_id, hook_type, command, behavior }` variant added
- [ ] `HookType` enum: `PreInstall`, `PostInstall`, `PreUninstall`, `StatusCheck`
- [ ] Hooks planned in correct order: pre_install → (packages, dotfiles, services) → post_install
- [ ] `Always` hooks run every apply
- [ ] `Once` hooks run only if not in `state.hooks_executed`
- [ ] `Ask` hooks prompt user before running (skip in `--yes` mode)
- [ ] `Skip` hooks never run
- [ ] Hook execution: runs command via `CommandExecutor` (not raw `Command::new`)
- [ ] Hook failure: log error, continue with remaining actions (non-fatal by default)
- [ ] `--dry-run` shows hooks that would run
- [ ] Unit tests: hook planning for each behavior, execution order
- [ ] Update test helpers for new Module fields

**TUI Impact:** Apply view must render `RunHook` actions. `Ask` behavior must prompt in TUI context (or skip with warning).

---

#### F3-015: Hook execution tracking (`Once` behavior)
**File:** `iron-core/src/services/state.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
Track which module hooks have been executed for `Once` behavior in state.json.

**Acceptance Criteria:**
- [ ] `State.hooks_executed: HashMap<String, Vec<String>>` field with `#[serde(default)]`
- [ ] Key: module_id, Value: list of hook types already run (e.g., `["PostInstall"]`)
- [ ] After successful `Once` hook execution, record in state
- [ ] `Once` hooks check state before planning — skip if already executed
- [ ] `iron apply --force-hooks` flag to re-run `Once` hooks
- [ ] Unit tests: tracking, skip on second apply, force-hooks override
- [ ] Update test helpers for new State field

---

#### F3-016: CLI `iron history` command
**File:** `iron-cli/src/commands/history.rs` (new), `iron-core/src/services/history.rs` (new)
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** S3

**Description:**
CLI command to view operation history from existing `AuditLog` and `StateManager.last_operations` data.

**Acceptance Criteria:**
- [ ] `iron history` (or `iron history list`) shows recent operations in table format
- [ ] Columns: #, Time (relative), Command, Duration, Actions, Status
- [ ] `iron history show <id>` shows detailed view: actions, errors, suggestions
- [ ] `iron history last` shows the most recent operation in detail
- [ ] `--json` output uses response envelope
- [ ] `--limit <n>` to control how many entries to show (default 20)
- [ ] Reads from `audit.log` (JSONL) in state directory (respects F3-006 XDG path)
- [ ] Registered in `cli.rs` Commands enum and `main.rs` dispatch
- [ ] CLI parsing tests

---

#### F3-017: CLI `iron config` namespace _(STRETCH)_
**File:** `iron-cli/src/commands/config.rs` (new)
**Priority:** 🟢 Nice-to-have (can slip to Phase 4)
**Effort:** S
**Gaps Addressed:** S2

**Description:**
Group configuration management commands under `iron config` namespace.

**Acceptance Criteria:**
- [ ] `iron config path` prints the config directory path
- [ ] `iron config edit` opens root config directory in `$EDITOR` (or `$VISUAL`)
- [ ] `iron config validate` delegates to existing `iron validate` logic
- [ ] `iron config show` shows resolved config summary (active host, bundle, profile, modules)
- [ ] `iron validate` continues to work as a standalone alias
- [ ] Registered in `cli.rs` Commands enum and `main.rs` dispatch
- [ ] CLI parsing tests

---

#### F3-018: `dotfiles_sync` automatic directory mirroring
**File:** `iron-core/src/module.rs`, `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** BM-1 (dcli benchmark pattern)

**Description:**
When `dotfiles_sync = true` on a module, automatically discover and mirror files from the module's `dotfiles/` directory to the target config location. Inspired by dcli's `dotfiles_sync` pattern observed in arch-config.

**Important:** The default target `~/.config/<module-id>/` may not match XDG convention for all modules (e.g., `nvim-ide` module maps to `~/.config/nvim`). Users should set `dotfiles_sync_target` explicitly for modules where the ID differs from the XDG config directory name. Document this in the module spec.

**Acceptance Criteria:**
- [ ] `Module.dotfiles_sync: bool` field added with `#[serde(default)]`
- [ ] `Module.dotfiles_sync_target: Option<String>` field for custom target (default: `~/.config/<module-id>/`)
- [ ] When `dotfiles_sync = true` and no explicit `[[dotfiles]]` entries, auto-discover files in `modules/<id>/dotfiles/`
- [ ] Each discovered file produces a symlink action: `modules/<id>/dotfiles/<path>` → `<target>/<path>`
- [ ] Preserves subdirectory structure (recursive discovery)
- [ ] Explicit `[[dotfiles]]` entries override auto-discovered entries for the same target
- [ ] Works with template rendering (files containing `{{var}}` get `RenderAndCopy` instead of symlink)
- [ ] Warning logged when using default target and module ID contains hyphens (likely mismatch)
- [ ] Unit tests: discovery, override, nested directories, empty directory, custom target
- [ ] Update test helpers for new Module fields

---

## Sprint 3.4 — Multi-Machine Readiness

**Goal:** Hostname auto-detection, host comparison, state serialization.

### Backlog (3 tasks)

---

#### F3-019: Hostname auto-detection
**File:** `iron-core/src/host.rs`, `iron-core/src/services/host.rs`
**Priority:** 🟡 Medium
**Effort:** M
**Gaps Addressed:** BM-3 (dcli benchmark pattern)

**Description:**
Auto-detect the current host by matching system hostname against host.toml `hostname` fields. Eliminates need for manual host selection on each machine.

**Acceptance Criteria:**
- [ ] `Host.hostname: Option<String>` field added with `#[serde(default)]`
- [ ] `HostService::detect_host(config_root)` queries system hostname and matches against host files
- [ ] Match priority: `hostname` field exact match > `id` field match > no match
- [ ] On startup: if no `--host` flag and no host in state, attempt auto-detection
- [ ] If auto-detection finds exactly one match, use it (log info)
- [ ] If no match or multiple matches, fall back to interactive selection (existing behavior)
- [ ] `iron status` shows "(auto-detected)" indicator when host was auto-detected
- [ ] Unit tests: exact match, id match, no match, multiple match fallback
- [ ] Update test helpers for new Host field

---

#### F3-020: Host comparison command _(STRETCH)_
**File:** `iron-cli/src/commands/host.rs` (new or extend existing)
**Priority:** 🟢 Nice-to-have (can slip to Phase 4)
**Effort:** M
**Gaps Addressed:** BM-4 (dcli benchmark pattern)

**Description:**
Compare two host configurations side-by-side, showing differences in bundles, profiles, modules, packages, and services.

**Acceptance Criteria:**
- [ ] `iron host compare <host1> <host2>` shows side-by-side comparison
- [ ] Shows: bundle diff, profile diff, module diff (only-on-A, only-on-B, shared)
- [ ] Shows: package diff (computed from resolved DesiredState)
- [ ] Shows: variable diff
- [ ] `--json` output uses response envelope
- [ ] `--dry-run` flag for integration tests
- [ ] Unit tests: same host, different hosts, one host missing

---

#### F3-022: ActualState serialization for cross-host comparison _(STRETCH)_
**File:** `iron-cli/src/commands/scan.rs` (extend)
**Priority:** 🟢 Nice-to-have (can slip to Phase 4)
**Effort:** S

**Description:**
Allow saving and loading `ActualState` snapshots for historical comparison and cross-host analysis.

**Acceptance Criteria:**
- [ ] `iron scan --save` saves ActualState to `~/.local/state/iron/scans/<timestamp>.json`
- [ ] `iron scan --load <file>` loads and displays a saved ActualState
- [ ] Scan directory auto-created on first save
- [ ] `--json` output uses response envelope
- [ ] Unit tests: save, load, roundtrip

---

## Summary

| Sprint | Tasks | Effort | Key Deliverable |
|--------|-------|--------|----------------|
| **3.1** | F3-001 → F3-007 (9 tasks) | ~2.5 weeks | Foundation: ActualState, state separation, envelope, status, plan |
| **3.2** | F3-021 + F3-008 → F3-013 (8 tasks) | ~2.5 weeks | Convergence: managed tracking, templates, file copy, removal, risk levels |
| **3.3** | F3-014 → F3-018 (5 tasks) | ~2 weeks | Lifecycle: hooks, history, config namespace, dotfiles_sync |
| **3.4** | F3-019 → F3-022 (3 tasks) | ~1.5 weeks | Multi-machine: auto-detect, compare, scan export |
| **Buffer** | — | ~1.5 weeks | Integration, stretch tasks, unforeseen cascading fixes |
| **Total** | **25 tasks** | **~10 weeks** | |

---

## Decision Log

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Stay with TOML, defer Lua/scripting (GAP-C4) | TOML + template variables covers 90% of use cases. Lua adds vendored runtime complexity. |
| D2 | Removal requires `--prune` flag | Safety: don't remove manually-installed packages without explicit opt-in |
| D2a | Granular prune flags: `--prune-packages`, `--prune-services`, `--prune-dotfiles` | Users may want to prune packages but not services. `--prune` is shorthand for all three. |
| D3 | Hooks are shell commands with behavior enum | Simple, debuggable, matches dcli pattern without Lua dependency |
| D4 | State separation uses XDG dirs | Follows XDG spec, enables multi-machine Git sync |
| D5 | ActualState is a single unified scan | Ensures consistency between plan and drift, eliminates redundant queries |
| D6 | `iron plan` is display-only (no `--output`/`--plan`) | Plan serialization deferred to Phase 4 — requires serialization strategy and staleness detection |
| D7 | Original roadmap Phase 3 (Ecosystem) deferred to Phase 4 | Foundational gaps (ActualState, state separation, removal, hooks) must be resolved before ecosystem features are viable. Task IDs F3-XXX refer to this plan, not the original roadmap. |
