# Phase 2 — Sprint Kanban Board

> **Phase:** 2 — Power User Features
> **Sprints:** 2.1 (Snapshot & Rollback) + 2.2 (Enhanced CLI Output) + 2.3 (Config Validation & Security)
> **Estimated Duration:** 3 sprints (~6 weeks)
> **Branch Convention:** `phase2/F2-XXX-short-description`
> **Commit Convention:** `F2-XXX: short description`
> **Status:** ✅ COMPLETE (2026-02-22)
> **Depends On:** Phase 1 ✅ Complete (sprints 1.1-1.4, 2026-02-22)

---

## Phase 2 Overview

**Goal:** Enable fearless experimentation, polished CLI output, and progressive security.

After Phase 2, a user can:

1. Create named snapshots before risky operations (`iron snapshot create pre-kde`)
2. Rollback to any previous state (`iron rollback` or `iron snapshot restore pre-kde`)
3. Auto-snapshots created before every `apply`/`update`/`bundle switch`
4. Beautiful tree-style CLI output with progress spinners
5. Pre-apply validation catches errors before they touch the system
6. Security level dashboard shows current hardening posture

**Mental model shift:** Iron goes from "a system that converges declared state" to "a safety net that lets you experiment fearlessly."

### Phase 1 Lessons Applied

| # | Lesson | How We Apply It in Phase 2 |
|---|--------|---------------------------|
| L1 | Test helpers break when struct fields added | SnapshotInfo struct uses builder pattern from day 1 |
| L2 | Integration tests hang on sudo/TUI | All snapshot commands include `--dry-run`; tests never create real btrfs/timeshift snapshots |
| L3 | `#[serde(default)]` for backward compat | All new state.json fields use `#[serde(default)]` |
| L4 | Exhaustive `View` matches in TUI | Add new views to ALL match arms in same PR (cycle_forward/backward, header, footer, help, test_view_names) |
| L5 | `StateManager::active_modules()` is the source of truth | Snapshot metadata references state.json, doesn't duplicate |
| L6 | Doctor check count assertion breaks easily | If adding doctor checks, update the count test |
| L7 | `ApplyService`/`DriftService` pattern works well | SnapshotService follows same pattern: trait + DefaultImpl + tests |
| L8 | Template engine `{{var}}` in iron-fs works | Security level calculation can reuse template vars for context |

---

## Sprint 2.1 — Snapshot & Rollback System ✅

**Goal:** Named snapshots + auto-snapshot before destructive ops + rollback.

### ✅ Done (8/8)

---

#### F2-001: SnapshotService trait + SnapshotRecord model ✅
**File:** `iron-core/src/services/snapshot_service.rs` (new)
**Priority:** 🔴 Critical (blocks all other F2-00X tasks)
**Effort:** L

**Description:**
Create `SnapshotService` trait with `create`, `list`, `restore`, `delete` methods.
`SnapshotRecord` model: id, name, timestamp, host_id, active_modules, active_bundle, checksums HashMap.

**Acceptance Criteria:**
- [x] `SnapshotRecord` struct with `#[serde(default)]` on all optional fields
- [x] `SnapshotService` trait: `create(name) -> SnapshotRecord`, `list() -> Vec<SnapshotRecord>`, `restore(id) -> Result`, `delete(id) -> Result`
- [x] `DefaultSnapshotService` stores snapshots as JSON in `$IRON_ROOT/.snapshots/`
- [x] Each snapshot captures: active_modules, active_bundle, active_profile, package list, dotfile checksums
- [x] Unit tests: create/list/restore/delete roundtrip
- [x] Registered in `services/mod.rs`

---

#### F2-002: CLI `iron snapshot create <name>` ✅
**File:** `iron-cli/src/commands/snapshot.rs` (new)
**Priority:** 🔴 Critical
**Effort:** M

**Description:**
CLI command to create a named snapshot. Captures current system state.

**Acceptance Criteria:**
- [x] `iron snapshot create <name>` creates snapshot and prints confirmation
- [x] `iron snapshot create` (no name) auto-generates name from timestamp
- [x] `--dry-run` flag shows what would be captured without saving
- [x] Registered in `cli.rs` Commands enum and `main.rs` dispatch
- [x] CLI parsing tests

---

#### F2-003: CLI `iron snapshot list` ✅
**File:** `iron-cli/src/commands/snapshot.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
List all snapshots with name, timestamp, module count.

**Acceptance Criteria:**
- [x] Table output: Name | Date | Modules | Bundle | Size
- [x] Sorted by date (newest first)
- [x] Empty state message when no snapshots
- [x] `--json` output format supported

---

#### F2-004: CLI `iron snapshot restore <name>` ✅
**File:** `iron-cli/src/commands/snapshot.rs`
**Priority:** 🔴 Critical
**Effort:** L

**Description:**
Restore system state to a named snapshot. Uses `ApplyService` internally.

**Acceptance Criteria:**
- [x] Loads snapshot record, builds `DesiredState` from snapshot data
- [x] Shows diff between current and snapshot state (uses `DriftService`)
- [x] Requires confirmation (or `--yes` flag)
- [x] `--dry-run` shows what would change
- [x] Creates auto-snapshot of current state before restoring ("pre-restore-*")
- [x] Uses `ApplyService::execute()` for convergence

---

#### F2-005: CLI `iron rollback` ✅
**File:** `iron-cli/src/commands/snapshot.rs`
**Priority:** 🔴 Critical
**Effort:** M

**Description:**
Quick rollback to the most recent auto-snapshot.

**Acceptance Criteria:**
- [x] `iron rollback` finds last auto-snapshot and restores it
- [x] `iron rollback --list` shows recent auto-snapshots
- [x] Error message if no auto-snapshots exist
- [x] Requires confirmation

---

#### F2-006: Per-module rollback ✅
**File:** `iron-core/src/services/snapshot_service.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Rollback a single module to its state in a snapshot.

**Acceptance Criteria:**
- [x] `iron rollback --module <id>` restores only that module's packages + dotfiles
- [x] Uses `ApplyService::plan_module()` internally
- [x] Shows module-specific diff before executing

---

#### F2-007: TUI Snapshot timeline view ✅
**File:** `iron-tui/src/ui/snapshot.rs` (new)
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Visual timeline of snapshots in TUI with restore action.

**Acceptance Criteria:**
- [x] New `View::Snapshots` variant (update ALL match arms)
- [x] `[S]` keybinding from Dashboard (Shift+S currently Secrets — remap to `[n]` for snapshots)
- [x] Timeline list: newest first, with name/date/badge
- [x] `[Enter]` on snapshot shows detail (modules, packages)
- [x] `[r]` on snapshot triggers restore with confirm
- [x] `[c]` creates new snapshot with name prompt
- [x] Render tests (no-panic tests)
- [x] `App::snapshot_list: Vec<SnapshotRecord>` field with `Default`

---

#### F2-008: Auto-snapshot before destructive operations ✅
**File:** `iron-core/src/services/apply.rs`, `iron-core/src/services/update.rs`
**Priority:** 🔴 Critical
**Effort:** S

**Description:**
Automatically create a snapshot before `apply`, `update`, and `bundle switch`.

**Acceptance Criteria:**
- [x] `ApplyService::execute()` creates snapshot named `auto-pre-apply-{timestamp}` before executing
- [x] `UpdateService` creates `auto-pre-update-{timestamp}` before running pacman
- [x] Auto-snapshots have `auto: true` flag in metadata
- [x] Old auto-snapshots pruned when count > 10 (configurable)
- [x] Tests verify auto-snapshot creation

---

## Sprint 2.2 — Enhanced CLI Output ✅

**Goal:** Beautiful, informative CLI output with progress indicators.

### ✅ Done (6/6)

---

#### F2-009: Tree-style output renderer ✅
**File:** `iron-cli/src/output.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Add tree rendering methods to `Output`: `tree_branch`, `tree_last`, `tree_indent`.

**Acceptance Criteria:**
- [x] `output.tree_root("Modules")` prints root
- [x] `output.tree_branch("nvim-ide")` prints `├── nvim-ide`
- [x] `output.tree_last("fish")` prints `└── fish`
- [x] Works in both color and plain modes
- [x] Used by `iron status`, `iron apply --dry-run`, `iron diff`

---

#### F2-010: Operation summary blocks ✅
**File:** `iron-cli/src/output.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
Standardize summary blocks across all commands.

**Acceptance Criteria:**
- [x] `output.summary_block()` method with title, items, duration
- [x] Box-drawing characters for visual border
- [x] Applied to: apply, diff, update, clean, snapshot commands
- [x] Tests for formatting

---

#### F2-011: Table output for list commands ✅
**File:** `iron-cli/src/output.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Column-aligned table output for list commands.

**Acceptance Criteria:**
- [x] `output.table(headers, rows)` method
- [x] Auto-width columns based on content
- [x] Right-align numeric columns
- [x] Used by: `iron module list`, `iron host list`, `iron snapshot list`

---

#### F2-012: Progress spinner for long operations ✅
**File:** `iron-cli/src/progress.rs` (new)
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Add `indicatif`-based progress spinner/bar for long operations.

**Acceptance Criteria:**
- [x] Add `indicatif` dependency to iron-cli
- [x] `ProgressReporter` struct wrapping indicatif
- [x] Spinner for indeterminate operations (sync, apply)
- [x] Progress bar for counted operations (install N packages)
- [x] Silent mode for `--json` output
- [x] Works in non-TTY environments (CI)

---

#### F2-013: `--explain` mode enhancement ✅
**File:** `iron-cli/src/output.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
Enhance existing `--explain` to show exact shell commands for all operations.

**Acceptance Criteria:**
- [x] `iron apply --explain` shows: "Would run: sudo pacman -S neovim fish"
- [x] `iron diff --explain` shows: "Would check: readlink ~/.config/nvim"
- [x] `iron update --explain` shows full pacman command chain

---

#### F2-014: Enhanced error messages ✅
**File:** `iron-core/src/error.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Every error type includes context, suggestions, and recovery hints.

**Acceptance Criteria:**
- [x] `IronError` gains `suggestion()` method returning `Option<&str>`
- [x] Common errors have suggestions: "Module not found → did you mean 'nvim-ide'? Run 'iron module list'"
- [x] `output.error_with_suggestion(err)` renders error + hint
- [x] At least 10 error types have suggestions

---

## Sprint 2.3 — Config Validation & Security Levels ✅

**Goal:** Pre-apply validation and security posture awareness.

### ✅ Done (5/5)

---

#### F2-015: Pre-apply config validation ✅
**File:** `iron-core/src/services/apply.rs`
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Validate all config before executing apply plan.

**Acceptance Criteria:**
- [x] `ApplyService::validate(host_id) -> Vec<ValidationWarning>` method
- [x] Checks: TOML syntax, referenced bundles/profiles/modules exist, paths are valid, no circular deps
- [x] `iron apply` runs validation before computing plan
- [x] `iron validate` standalone command
- [x] Validation errors are descriptive with fix suggestions

---

#### F2-016: Security level calculator ✅
**File:** `iron-core/src/services/security.rs` (new)
**Priority:** 🟡 Medium
**Effort:** M

**Description:**
Calculate security hardening level based on enabled modules.

**Acceptance Criteria:**
- [x] `SecurityLevel` enum: `Basic`, `Standard`, `Advanced`, `Paranoid`
- [x] Scoring: each security module contributes points (ufw=10, fail2ban=10, apparmor=15, etc.)
- [x] Thresholds: Basic(0-20), Standard(21-50), Advanced(51-80), Paranoid(81+)
- [x] `SecurityService::calculate_level()` returns level + breakdown
- [x] Tests for each level threshold

---

#### F2-017: CLI `iron security status` ✅
**File:** `iron-cli/src/commands/security.rs` (new)
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
Show current security level and recommendations.

**Acceptance Criteria:**
- [x] Shows: current level, score, enabled security modules
- [x] Recommendations: "Enable ufw for +10 points → Standard level"
- [x] `--json` output format

---

#### F2-018: Dashboard security indicator ✅
**File:** `iron-tui/src/ui/dashboard.rs`
**Priority:** 🟡 Medium
**Effort:** S

**Description:**
Show security level badge on TUI dashboard.

**Acceptance Criteria:**
- [x] `App::security_level: Option<SecurityLevel>` field
- [x] Dashboard renders: `[🛡 Standard]` with color per level
- [x] Updated on init and after module enable/disable
- [x] Default to None

---

#### F2-019: Security module tagging ✅
**File:** `iron-core/src/module.rs`
**Priority:** 🟢 Nice-to-have
**Effort:** S

**Description:**
Tag modules with security_level metadata.

**Acceptance Criteria:**
- [x] `Module` struct gains `#[serde(default)] pub security_points: u32`
- [x] Existing security modules updated with points values
- [x] Used by SecurityService for level calculation

---

## Summary

| Sprint | Tasks | Effort | Key Deliverable |
|--------|-------|--------|----------------|
| **2.1** | F2-001 → F2-008 (8 tasks) | 2 weeks | Snapshot/rollback safety net |
| **2.2** | F2-009 → F2-014 (6 tasks) | 2 weeks | Polished CLI experience |
| **2.3** | F2-015 → F2-019 (5 tasks) | 2 weeks | Validation + security posture |
| **Total** | **19 tasks** | **~6 weeks** | |
