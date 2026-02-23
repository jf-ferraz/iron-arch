# Phase 1 — Sprint Kanban Board

> **Phase:** 1 — Core Experience
> **Sprints:** 1.1 (Host as Truth) + 1.2 (Apply Command) + 1.3 (Diff & Drift) + 1.4 (Template Engine — Stretch)
> **Estimated Duration:** 4 sprints (~8 weeks)
> **Branch Convention:** `phase1/F1-XXX-short-description`
> **Commit Convention:** `F1-XXX: short description`
> **Status:** 🟢 IMPLEMENTED (all 4 sprints — 2026-02-22)
> **Depends On:** Phase 0 ✅ Complete (12/12 tasks, 2026-02-22)

---

## Phase 1 Overview

**Goal:** Implement the declarative **apply → diff → verify → rollback** loop. After Phase 1, a user can:

1. Declare their desired system in `hosts/desktop.toml` (bundle, profile, modules, variables)
2. Run `iron apply` to converge the system to that declared state
3. Run `iron diff` to see what drifted since last apply
4. Use `{{variable}}` templates for host-specific dotfile rendering

**Mental model shift:** Iron goes from "a tool that manages modules" to "a system that converges declared state."

### Dependency Graph

```
F1-001 → F1-002 → F1-003 → F1-005 → F1-006 → F1-007 → F1-008
                              ↓                    ↓
                           F1-004                F1-009
                                                   ↓
                                                F1-010

F1-003 + F1-005 → F1-011 → F1-012/013/014 → F1-015 → F1-016/017 → F1-018

F1-019 → F1-020 → F1-021
                     ↓
                  F1-022
```

### Phase 0 Lessons Applied

| # | Lesson | How We Apply It in Phase 1 |
|---|--------|---------------------------|
| L1 | Test helpers that construct structs directly break when fields are added | Every struct change (Host, etc.) must update ALL test helpers in the same PR |
| L2 | Integration tests hang on sudo or TUI launch | All CLI commands include `--dry-run`; integration tests always use it |
| L3 | `--dry-run` flags are essential for testability | `iron apply --dry-run` and `iron diff` are non-destructive by design |
| L4 | Pre-existing tech debt should be migrated incrementally | New services (ApplyService, DriftService) use CommandExecutor from day 1 |
| L5 | Kanban + technical-guide two-doc format works | This document + `phase1-technical-guide.md` |
| L6 | Tests that assert exact counts need updating when items added | Doctor check count, state field counts — note them in acceptance criteria |

---

## Sprint 1.1 — Host as Source of Truth

**Sprint Goal:** Make the host TOML file the single source of truth for what a machine should look like. This is the foundation for everything else in Phase 1.

**Duration:** ~2 weeks  
**Exit Criteria:** `hosts/desktop.toml` can declare bundle, profile, modules, and variables. `HostService::desired_state()` returns a resolved `DesiredState`. Backward compatible with existing host files.

---

### 🟡 TODO

#### F1-001: Extend Host struct with desired-state fields
- **Priority:** P0 — Critical path, everything depends on this
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-core/src/host.rs`, `hosts/desktop.toml`, `hosts/notebook.toml`
- **Description:** Add `bundle`, `profile`, `extra_modules`, and `variables` fields to the `Host` struct. All fields are `Option` or `Vec` with `#[serde(default)]` for backward compatibility.
- **Acceptance Criteria:**
  - [ ] `Host` struct has `bundle: Option<String>` field
  - [ ] `Host` struct has `profile: Option<String>` field
  - [ ] `Host` struct has `extra_modules: Vec<String>` field with `#[serde(default)]`
  - [ ] `Host` struct has `variables: HashMap<String, String>` field with `#[serde(default)]`
  - [ ] Existing `hosts/desktop.toml` and `hosts/notebook.toml` still parse without changes
  - [ ] `create_test_host()` and ALL host tests updated (L1)
  - [ ] TOML roundtrip test passes with new fields populated
  - [ ] TOML roundtrip test passes with new fields empty/absent
- **User Constraint:** Mid-level #1 ("Layered, composable configuration"), Newcomer #5 ("Declarative system definition")
- **Dependencies:** None
- **Risk:** Low — additive change with `#[serde(default)]`

---

#### F1-002: Backward-compatible TOML parsing & migration helper
- **Priority:** P0
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-core/src/host.rs`, `crates/iron-core/src/services/host.rs`, `crates/iron-core/src/services/state.rs`
- **Description:** Ensure existing host files (no bundle/profile fields) parse correctly. Add a `migrate_state_to_host()` function that reads `active_bundles`/`active_profiles` from `state.json` and writes them into the host TOML for first-time migration.
- **Acceptance Criteria:**
  - [ ] Loading `hosts/desktop.toml` (no bundle/profile) succeeds with `None`/empty defaults
  - [ ] `HostService::migrate_state_to_host(host_id)` copies active bundle/profile from state.json → host.toml
  - [ ] Migration is idempotent (running twice doesn't corrupt)
  - [ ] Migration preserves all existing host.toml content (hardware, etc.)
  - [ ] `iron doctor` warns when host.toml has no declared bundle/profile (suggesting migration)
- **User Constraint:** Newcomer #8 ("Idempotent operations")
- **Dependencies:** F1-001
- **Risk:** Medium — must not corrupt existing state

---

#### F1-003: HostService::desired_state() resolver
- **Priority:** P0 — Critical for ApplyService
- **Effort:** M (~6 hours)
- **Files:** `crates/iron-core/src/services/host.rs`, new struct `DesiredState`
- **Description:** Add `desired_state(host_id: &str) -> IronResult<DesiredState>` to `HostService`. This method reads the host TOML and resolves the full desired state: bundle packages + services, profile's module list expanded, extra_modules merged, all module packages/dotfiles/services collected. The `DesiredState` is the "target" that Apply and Diff compare against.
- **Acceptance Criteria:**
  - [ ] `DesiredState` struct defined: `bundle`, `profile`, `modules: Vec<String>`, `packages: Vec<String>`, `services: Vec<String>`, `dotfiles: Vec<DotfileMapping>`, `variables: HashMap<String, String>`
  - [ ] Profile's `modules` list is resolved (including `extends` inheritance)
  - [ ] `extra_modules` from host.toml are appended after profile modules
  - [ ] Duplicate modules are deduplicated
  - [ ] Module dependencies are resolved transitively
  - [ ] Module conflicts are checked and reported as errors
  - [ ] All packages/services/dotfiles from resolved modules are collected
  - [ ] Returns meaningful error when host.toml has no bundle/profile declared
  - [ ] Unit tests with mock host/profile/module data
- **User Constraint:** Mid-level #1 ("Layered, composable configuration")
- **Dependencies:** F1-001, F1-002
- **Risk:** Medium — complex resolution logic, needs thorough testing

---

#### F1-004: TUI wizard writes bundle/profile into host.toml
- **Priority:** P1
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/ui/host_selection.rs`
- **Description:** When the setup wizard completes (bundle selected + profile selected), write those choices into the host TOML file as `bundle = "..."` and `profile = "..."`, making the host file the persistent declaration. Currently these are only written to state.json.
- **Acceptance Criteria:**
  - [ ] Wizard completion writes `bundle` and `profile` to host.toml
  - [ ] TUI host selection updates host.toml when user changes bundle/profile
  - [ ] state.json continues to be updated (dual-write for transition period)
  - [ ] Existing host.toml fields (hardware, etc.) are preserved during write
  - [ ] Error message shown if host.toml write fails (doesn't crash TUI)
- **User Constraint:** Newcomer #1 ("TUI is the primary interface")
- **Dependencies:** F1-001
- **Risk:** Low

---

### 🔵 IN PROGRESS

*(Empty)*

### ✅ DONE

*(Empty)*

---
---

## Sprint 1.2 — The Apply Command

**Sprint Goal:** `iron apply` computes the difference between declared state (host.toml) and actual system state, then converges the system. This is the **centerpiece** of Iron's value proposition.

**Duration:** ~2 weeks  
**Exit Criteria:** `iron apply` installs missing packages, creates symlinks, enables services. `--dry-run` shows what would change without doing it. TUI has Apply view with progress.

---

### 🟡 TODO

#### F1-005: ApplyService — compare desired vs actual state
- **Priority:** P0 — Critical path
- **Effort:** L (~8 hours)
- **Files:** New `crates/iron-core/src/services/apply.rs`, `crates/iron-core/src/services/mod.rs`
- **Description:** New `ApplyService` trait with `DefaultApplyService` implementation. Takes `DesiredState` (from F1-003) and compares against actual system state (installed packages, existing symlinks, enabled services). Produces an `ApplyPlan`.
- **Acceptance Criteria:**
  - [ ] `ApplyService` trait with `plan(host_id: &str) -> IronResult<ApplyPlan>` and `execute(plan: &ApplyPlan) -> IronResult<ApplyResult>`
  - [ ] `DefaultApplyService` with builder pattern: `.with_package_manager()`, `.with_service_manager()`, `.with_host_service()`, etc.
  - [ ] `ApplyService::plan()` calls `HostService::desired_state()` then computes diff
  - [ ] Package diff: desired packages minus already-installed = packages to install
  - [ ] Symlink diff: desired dotfiles minus already-linked = symlinks to create
  - [ ] Service diff: desired services minus already-enabled = services to enable
  - [ ] Module diff: desired modules minus already-active = modules to enable
  - [ ] `ApplyService::execute()` runs the plan in order: packages → symlinks → services → state update
  - [ ] Registered in `services/mod.rs` with re-exports
  - [ ] Unit tests with mock services
- **User Constraint:** Newcomer #5 ("Declarative system definition"), Mid-level #5 ("Diff before apply")
- **Dependencies:** F1-003
- **Risk:** High — orchestration of multiple services, needs careful error handling

---

#### F1-006: ApplyPlan struct — action list with display
- **Priority:** P0
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-core/src/services/apply.rs`
- **Description:** `ApplyPlan` contains a `Vec<ApplyAction>` enum with variants for each operation type. Includes methods for summary, dry-run display, and serialization.
- **Acceptance Criteria:**
  - [ ] `ApplyAction` enum: `InstallBundle { id }`, `ActivateProfile { id }`, `EnableModule { id }`, `InstallPackages { packages: Vec<String> }`, `CreateSymlinks { mappings: Vec<DotfileMapping> }`, `EnableServices { services: Vec<String> }`
  - [ ] `ApplyPlan::is_empty() -> bool`
  - [ ] `ApplyPlan::summary() -> String` — "3 packages to install, 5 symlinks to create, 2 services to enable"
  - [ ] `ApplyPlan::action_count() -> usize`
  - [ ] `ApplyResult` struct: `succeeded: usize`, `failed: usize`, `errors: Vec<String>`, `duration: Duration`
  - [ ] Implements `Serialize` for JSON output
- **User Constraint:** Newcomer #2 ("Dry-run / preview mode")
- **Dependencies:** F1-005 (same file, designed together)
- **Risk:** Low

---

#### F1-007: `iron apply` CLI command
- **Priority:** P0
- **Effort:** L (~6 hours)
- **Files:** `crates/iron-cli/src/cli.rs`, new `crates/iron-cli/src/commands/apply.rs`, `crates/iron-cli/src/main.rs`, `crates/iron-cli/src/context.rs`
- **Description:** New `Apply` subcommand. Computes plan → displays preview → asks for confirmation → executes with progress → shows summary. Uses `Output::summary()` (F0-005) and `Output::explain_cmd()` (F0-006) from Phase 0.
- **Acceptance Criteria:**
  - [ ] `iron apply` computes and displays plan, then prompts for confirmation
  - [ ] `--yes` flag skips confirmation
  - [ ] Plan display shows each action with type icon and description
  - [ ] Execution shows progress for each step
  - [ ] Summary line at end (using F0-005 `output.summary()`)
  - [ ] `--explain` shows underlying commands (using F0-006)
  - [ ] Error handling: partial failures are reported, not fatal
  - [ ] `AppContext::apply_service()` factory method added
  - [ ] Integration test with `--dry-run` (L2, L3)
- **User Constraint:** Newcomer #1 ("No destructive ops without confirmation"), Newcomer #3 ("Clear output")
- **Dependencies:** F1-005, F1-006
- **Risk:** Medium

---

#### F1-008: `iron apply --dry-run`
- **Priority:** P0
- **Effort:** S (~2 hours)
- **Files:** `crates/iron-cli/src/commands/apply.rs`, `crates/iron-cli/src/cli.rs`
- **Description:** `--dry-run` computes and displays the plan but does NOT execute. No sudo, no state changes.
- **Acceptance Criteria:**
  - [ ] `--dry-run` flag on Apply command
  - [ ] Computes full plan (identical to real apply)
  - [ ] Displays plan with "[DRY RUN]" header
  - [ ] Exits without executing any actions
  - [ ] Exit code 0 even when plan is non-empty
  - [ ] Integration test uses `--dry-run` (no sudo prompts — L2)
- **User Constraint:** Newcomer #2 ("Dry-run / preview mode"), Mid-level #5 ("Diff before apply")
- **Dependencies:** F1-007
- **Risk:** Low

---

#### F1-009: `iron apply --module <id>` — selective apply
- **Priority:** P1
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-cli/src/commands/apply.rs`, `crates/iron-core/src/services/apply.rs`
- **Description:** `--module <id>` flag limits the apply to a single module. Skips bundle/profile resolution. Useful for testing a new module without touching the rest of the system.
- **Acceptance Criteria:**
  - [ ] `--module <id>` flag on Apply command
  - [ ] `ApplyService::plan_module(module_id: &str) -> IronResult<ApplyPlan>` method
  - [ ] Only includes the specified module's packages, dotfiles, services
  - [ ] Resolves module dependencies (if module depends on another, both are included)
  - [ ] Error if module not found
  - [ ] Works with `--dry-run`
- **User Constraint:** Mid-level #8 ("Selective operations")
- **Dependencies:** F1-005, F1-007
- **Risk:** Low

---

#### F1-010: TUI Apply view with visual plan + progress
- **Priority:** P1
- **Effort:** L (~8 hours)
- **Files:** New `crates/iron-tui/src/ui/apply.rs`, `crates/iron-tui/src/app/mod.rs`, `crates/iron-tui/src/app/handlers.rs`, `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/ui/mod.rs`
- **Description:** New Apply view accessible via `[a]` from Dashboard. Shows the computed plan as a list of actions. Enter confirms, execution shows per-action progress with checkmarks. Background thread for execution (like sync push/pull in F0-009).
- **Acceptance Criteria:**
  - [ ] New `View::Apply` enum variant
  - [ ] `[a]` keybinding from Dashboard navigates to Apply view
  - [ ] Plan computed on navigation (shows loading spinner)
  - [ ] Plan displayed as scrollable list with action icons
  - [ ] "Nothing to do" message when plan is empty
  - [ ] Enter to confirm, Esc to cancel
  - [ ] Execution runs in background thread (doesn't block TUI — uses F0-009 pattern)
  - [ ] Per-action progress with ✓/✗ status
  - [ ] Summary shown after completion
- **User Constraint:** Newcomer #1 ("TUI is the primary interface")
- **Dependencies:** F1-005, F1-006
- **Risk:** Medium — async execution in TUI

---

### 🔵 IN PROGRESS

*(Empty)*

### ✅ DONE

*(Empty)*

---
---

## Sprint 1.3 — Diff & Drift Detection

**Sprint Goal:** `iron diff` shows what drifted from the declared state. Users can adopt drift or correct it. Dashboard shows drift indicators.

**Duration:** ~2 weeks  
**Exit Criteria:** `iron diff` produces a drift report. `--adopt` incorporates drift. `--correct` converges. Dashboard shows drift count.

---

### 🟡 TODO

#### F1-011: DriftService — compare declared vs actual state
- **Priority:** P0
- **Effort:** L (~8 hours)
- **Files:** New `crates/iron-core/src/services/drift.rs`, `crates/iron-core/src/services/mod.rs`
- **Description:** New `DriftService` trait that compares `DesiredState` against the actual system. Returns a `DriftReport` with package drift, service drift, and config drift. Reuses `PackageManager::query_installed()`, new `SystemService::is_enabled()`, and `iron_fs::symlink::check()`.
- **Acceptance Criteria:**
  - [ ] `DriftService` trait with `detect(host_id: &str) -> IronResult<DriftReport>`
  - [ ] `DriftReport` struct: `package_drift: Vec<PackageDrift>`, `service_drift: Vec<ServiceDrift>`, `config_drift: Vec<ConfigDrift>`, `summary: DriftSummary`
  - [ ] `PackageDrift`: `Missing { name }`, `Extra { name }`, `VersionMismatch { name, expected, actual }`
  - [ ] `ServiceDrift`: `NotEnabled { name }`, `ExtraEnabled { name }`
  - [ ] `ConfigDrift`: `MissingSymlink { source, target }`, `BrokenSymlink { target }`, `WrongTarget { target, expected, actual }`, `ContentModified { path, expected_checksum, actual_checksum }`
  - [ ] `DriftSummary`: `total_drifts`, `packages_missing`, `packages_extra`, `configs_drifted`, `services_drifted`
  - [ ] `DriftReport::is_clean() -> bool` (no drifts)
  - [ ] Registered in `services/mod.rs`
  - [ ] Unit tests with mock services
- **User Constraint:** Mid-level #4 ("Drift detection")
- **Dependencies:** F1-003 (DesiredState)
- **Risk:** Medium

---

#### F1-012: Package drift detection
- **Priority:** P0
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-core/src/services/drift.rs`
- **Description:** Compare desired packages (from DesiredState) against `PackageManager::query_installed()`. Report missing, extra, and version mismatches.
- **Acceptance Criteria:**
  - [ ] Desired packages not in `query_installed()` → `Missing`
  - [ ] `query_installed()` packages not in desired (that were previously installed by Iron) → `Extra` (requires tracking in state.json)
  - [ ] Uses `PackageManager::is_installed()` for individual checks
  - [ ] AUR packages checked separately
  - [ ] Unit test with mock PackageManager returning known package list
- **Dependencies:** F1-011
- **Risk:** Low

---

#### F1-013: Service drift detection
- **Priority:** P1
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-core/src/services/drift.rs`, `crates/iron-core/src/system_service.rs`, `crates/iron-systemd/src/lib.rs`
- **Description:** Compare desired services against actual enabled services. Requires adding `is_enabled()` to `SystemService` trait.
- **Acceptance Criteria:**
  - [ ] `SystemService::is_enabled(name: &str) -> IronResult<bool>` added with default `Ok(false)` impl
  - [ ] `NoopSystemService` returns `Ok(false)`
  - [ ] `iron-systemd::SystemdServiceAdapter` implements by running `systemctl is-enabled <name>`
  - [ ] Desired services not enabled → `NotEnabled`
  - [ ] Services enabled by Iron but no longer in desired state → `ExtraEnabled`
  - [ ] Unit test with mock SystemService
- **Dependencies:** F1-011
- **Risk:** Low — trait extension with default impl

---

#### F1-014: Config drift detection (symlink + checksum)
- **Priority:** P0
- **Effort:** M (~6 hours)
- **Files:** `crates/iron-core/src/services/drift.rs`, `crates/iron-fs/src/lib.rs`
- **Description:** Check dotfile symlinks: exist, point to correct target, source file unchanged (SHA-256 checksum).
- **Acceptance Criteria:**
  - [ ] Missing symlink → `MissingSymlink`
  - [ ] Broken symlink → `BrokenSymlink`
  - [ ] Symlink pointing to wrong target → `WrongTarget`
  - [ ] Source file checksum differs from stored checksum → `ContentModified`
  - [ ] Checksum stored in state.json on first apply (new field `dotfile_checksums: HashMap<String, String>`)
  - [ ] Uses `iron_fs::symlink::check()` for status
  - [ ] Adds `iron_fs::checksum::sha256(path: &Path) -> IronResult<String>` utility
  - [ ] Unit tests with tempdir-based symlinks
- **Dependencies:** F1-011
- **Risk:** Medium — checksum storage adds state complexity

---

#### F1-015: `iron diff` CLI command
- **Priority:** P0
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-cli/src/cli.rs`, new `crates/iron-cli/src/commands/diff.rs`, `crates/iron-cli/src/main.rs`, `crates/iron-cli/src/context.rs`
- **Description:** New `Diff` subcommand that runs `DriftService::detect()` and displays the report. Tree-style output with color-coded drift types.
- **Acceptance Criteria:**
  - [ ] `iron diff` produces drift report
  - [ ] Packages section: missing (red), extra (yellow)
  - [ ] Configs section: missing symlinks (red), broken (red), wrong target (yellow), modified (yellow)
  - [ ] Services section: not enabled (red), extra enabled (yellow)
  - [ ] "System is clean ✓" when no drift
  - [ ] JSON output mode (`-f json`)
  - [ ] `AppContext::drift_service()` factory method
  - [ ] Integration test
- **User Constraint:** Mid-level #5 ("Diff before apply")
- **Dependencies:** F1-011
- **Risk:** Low

---

#### F1-016: `iron diff --adopt` — incorporate drift
- **Priority:** P2
- **Effort:** M (~6 hours)
- **Files:** `crates/iron-cli/src/commands/diff.rs`, `crates/iron-core/src/services/drift.rs`
- **Description:** `--adopt` takes discovered drift and writes it back into the canonical state: extra packages → added to host's extra_modules or a "manually installed" tracking list; modified configs → checksum updated.
- **Acceptance Criteria:**
  - [ ] `--adopt` flag on Diff command
  - [ ] Extra packages can be acknowledged (checksum updated, not flagged next time)
  - [ ] Modified config checksums are updated in state.json
  - [ ] Confirmation prompt before adopting (unless `--yes`)
  - [ ] Summary of what was adopted
- **User Constraint:** Mid-level #4 ("Drift detection")
- **Dependencies:** F1-015
- **Risk:** Medium — writes to state

---

#### F1-017: `iron diff --correct` — revert drift
- **Priority:** P2
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-cli/src/commands/diff.rs`, `crates/iron-core/src/services/drift.rs`
- **Description:** `--correct` calls `ApplyService::execute()` to converge the system back to declared state. Essentially `iron apply` but triggered from diff context.
- **Acceptance Criteria:**
  - [ ] `--correct` flag on Diff command
  - [ ] Computes apply plan from drift report
  - [ ] Confirmation prompt before correcting
  - [ ] Handles partial failures gracefully
  - [ ] Works with `--dry-run` to preview corrections
- **User Constraint:** Mid-level #4 ("Drift detection")
- **Dependencies:** F1-005 (ApplyService), F1-015
- **Risk:** Low — delegates to ApplyService

---

#### F1-018: TUI drift indicator on Dashboard + Drift view
- **Priority:** P2
- **Effort:** M (~6 hours)
- **Files:** `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-tui/src/app/mod.rs`, `crates/iron-tui/src/app/actions.rs`
- **Description:** Dashboard System Status panel shows drift count badge. New `[d]` keybinding opens drift detail view showing the full report.
- **Acceptance Criteria:**
  - [ ] Dashboard shows "Drift: 0 ✓" (green) or "Drift: 3 ⚠" (yellow) in System Status
  - [ ] Drift check runs on app startup and periodic refresh
  - [ ] `[d]` from Dashboard opens drift detail view
  - [ ] Drift detail shows same info as `iron diff` CLI
  - [ ] Background computation (doesn't block TUI)
- **User Constraint:** Newcomer #1 ("TUI is the primary interface")
- **Dependencies:** F1-011
- **Risk:** Low

---

### 🔵 IN PROGRESS

*(Empty)*

### ✅ DONE

*(Empty)*

---
---

## Sprint 1.4 — Template Engine (Stretch)

**Sprint Goal:** Enable host-specific dotfile customization via `{{variable}}` templates. This is the mechanism that makes multi-host setups practical.

**Duration:** ~2 weeks  
**Exit Criteria:** `.tmpl` files in modules are rendered with host variables before symlinking. Built-in variables available. TUI variable editor.

> **⚠ Stretch sprint.** This can defer to Phase 2 without blocking the apply/diff loop. The apply command works without templates — modules just use static dotfiles.

---

### 🟡 TODO

#### F1-019: Simple `{{variable}}` template engine
- **Priority:** P1
- **Effort:** M (~4 hours)
- **Files:** `crates/iron-fs/src/lib.rs` (new `template` module)
- **Description:** Add a `template::render(content: &str, vars: &HashMap<String, String>) -> Result<String>` function using regex `\{\{(\w+)\}\}`. No complex logic (no if/else/loops) — just variable substitution. Unknown variables are left as-is with a warning.
- **Acceptance Criteria:**
  - [ ] `template::render()` replaces `{{key}}` with value from map
  - [ ] Whitespace inside braces is trimmed: `{{ key }}` works
  - [ ] Unknown variables left unchanged, warning logged
  - [ ] Empty string values are valid (renders empty)
  - [ ] `template::has_variables(content: &str) -> bool` helper
  - [ ] `template::extract_variables(content: &str) -> Vec<String>` helper
  - [ ] Comprehensive unit tests (10+ cases)
- **User Constraint:** Mid-level #7 ("Template/variable system")
- **Dependencies:** None (can be built independently)
- **Risk:** Low

---

#### F1-020: Render `.tmpl` files during module apply
- **Priority:** P1
- **Effort:** M (~6 hours)
- **Files:** `crates/iron-core/src/services/module.rs`, `crates/iron-core/src/services/apply.rs`
- **Description:** During module enable/apply, if a dotfile source has `.tmpl` extension: render it with host variables → write rendered content to a generated file → symlink to the rendered file (not the template). Original `.tmpl` stays unmodified.
- **Acceptance Criteria:**
  - [ ] `.tmpl` detection in `DotfileMapping.source`
  - [ ] Rendered output written to `~/.config/iron/rendered/<module>/<filename>` (without `.tmpl`)
  - [ ] Symlink points to rendered file, not original template
  - [ ] Variables come from `DesiredState.variables` (host.toml `[variables]` section)
  - [ ] Re-render on `iron apply` if template or variables changed
  - [ ] Non-`.tmpl` files continue to be symlinked directly (no change)
  - [ ] Unit test with tempdir, template file, variable map
- **User Constraint:** Mid-level #7 ("Template/variable system")
- **Dependencies:** F1-019, F1-005
- **Risk:** Medium — adds a new file management layer

---

#### F1-021: Built-in template variables
- **Priority:** P2
- **Effort:** S (~2 hours)
- **Files:** `crates/iron-core/src/services/apply.rs` or `crates/iron-fs/src/lib.rs`
- **Description:** Provide built-in variables that are always available: `{{hostname}}`, `{{username}}`, `{{home}}`, `{{config_dir}}`, `{{iron_root}}`. These are merged with host variables (host variables take precedence).
- **Acceptance Criteria:**
  - [ ] `template::builtin_variables() -> HashMap<String, String>`
  - [ ] `hostname` from `gethostname`
  - [ ] `username` from `$USER` env var
  - [ ] `home` from `dirs::home_dir()`
  - [ ] `config_dir` from `dirs::config_dir()`
  - [ ] `iron_root` from app context
  - [ ] Host `[variables]` override built-ins
  - [ ] Unit test verifying merge precedence
- **Dependencies:** F1-019
- **Risk:** Low

---

#### F1-022: TUI variable editor in Host settings
- **Priority:** P3
- **Effort:** M (~6 hours)
- **Files:** New `crates/iron-tui/src/ui/variables.rs`, `crates/iron-tui/src/app/mod.rs`, `crates/iron-tui/src/app/handlers.rs`
- **Description:** New TUI view for editing the `[variables]` section of the active host's TOML. Key-value list with add/edit/delete. Shows which templates use each variable.
- **Acceptance Criteria:**
  - [ ] New view accessible from Host settings or `[v]` keybinding
  - [ ] List of current variables with values
  - [ ] Add new variable (key + value input)
  - [ ] Edit existing variable value
  - [ ] Delete variable (with confirmation)
  - [ ] Shows built-in variables as read-only with "(built-in)" label
  - [ ] Saves to host.toml on change
- **User Constraint:** Mid-level #7 ("Template/variable system")
- **Dependencies:** F1-001, F1-019
- **Risk:** Low

---

### 🔵 IN PROGRESS

*(Empty)*

### ✅ DONE

*(Empty)*

---
---

## Cross-Sprint Tracking

### Definition of Done (per task)
- [ ] Code changes implemented
- [ ] Code compiles with zero warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] All existing tests pass (`cargo test --workspace`)
- [ ] New tests written for new/changed behavior
- [ ] Code formatted (`cargo fmt --all`)
- [ ] No regressions in TUI render tests
- [ ] `--dry-run` path covered by integration test (where applicable)

### Test Targets

| Sprint | New Tests Target | Cumulative |
|--------|-----------------|------------|
| 1.1 | ~25 (Host struct, parsing, migration, desired_state) | ~1,730 |
| 1.2 | ~35 (ApplyService, ApplyPlan, CLI, TUI) | ~1,765 |
| 1.3 | ~30 (DriftService, package/service/config drift, CLI) | ~1,795 |
| 1.4 | ~15 (template engine, rendering, built-ins) | ~1,810 |

### New Files Created

| Sprint | New Files |
|--------|-----------|
| 1.1 | — (existing file modifications only) |
| 1.2 | `services/apply.rs`, `commands/apply.rs`, `ui/apply.rs` |
| 1.3 | `services/drift.rs`, `commands/diff.rs` |
| 1.4 | `iron-fs/src/template.rs` (or inline module), `ui/variables.rs` |

### Phase 1 Exit Criteria
- [ ] `hosts/desktop.toml` declares bundle, profile, modules, and variables
- [ ] `iron apply` converges system to declared state
- [ ] `iron apply --dry-run` shows plan without executing
- [ ] `iron diff` shows package/config/service drift
- [ ] `iron diff --adopt` and `--correct` work
- [ ] TUI has Apply view and drift indicator
- [ ] `.tmpl` files render with host variables (stretch)
- [ ] All tests pass, zero clippy warnings
- [ ] Test count ≥ 1,795
