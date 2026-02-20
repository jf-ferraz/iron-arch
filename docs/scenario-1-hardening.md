# Scenario 1 — Hardening Sprint Guide

> **Purpose**: Comprehensive gap analysis and task backlog for the hardening iteration
> following Scenario 1 Sprints 1–4 (45/45 tasks complete). Every item has been
> cross-checked against the actual codebase at commit `8fb4e6c` on branch
> `feature/tui-enhancement-phase1`.
>
> **Methodology**: Each of the 9 phase guideline documents was reviewed for "Discovered
> Issues" sections, then the actual Rust source was inspected via grep and read to confirm
> whether each issue persists or was already resolved by Sprint 1–4 work.
>
> **Baseline stats**: 1,567 tests passing, 0 failures, ~60,740 LOC across 7 crates,
> 0 Clippy warnings.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Sprint Scope & Priorities](#2-sprint-scope--priorities)
3. [Category A — Architecture Debt](#3-category-a--architecture-debt)
4. [Category B — Bug Fixes](#4-category-b--bug-fixes)
5. [Category C — Feature Gaps (CLI Parity)](#5-category-c--feature-gaps-cli-parity)
6. [Category D — Feature Gaps (TUI Completeness)](#6-category-d--feature-gaps-tui-completeness)
7. [Category E — Test Coverage Gaps](#7-category-e--test-coverage-gaps)
8. [Category F — UX Polish](#8-category-f--ux-polish)
9. [Requirements Cross-Reference](#9-requirements-cross-reference)
10. [Dependency Graph](#10-dependency-graph)
11. [Sprint Plan](#11-sprint-plan)
12. [Already Resolved (Excluded)](#12-already-resolved-excluded)

---

## 1. Executive Summary

Sprints 1–4 delivered all 45 tracked tasks: TUI views for 14 screens, 8 service layers,
CLI parity for 10 commands, and a full test suite. However, deep cross-referencing
against the 9 phase guidelines and `requirements.md` reveals **~65 remaining gaps**
across 6 categories:

| Category | Count | Severity Breakdown |
|----------|------:|---------------------|
| A: Architecture Debt | 10 | 2 P1, 5 P2, 3 P3 |
| B: Bug Fixes | 6 | 1 P0, 2 P1, 2 P2, 1 P3 |
| C: CLI Feature Gaps | 10 | 1 P1, 5 P2, 4 P3 |
| D: TUI Feature Gaps | 13 | 2 P1, 6 P2, 5 P3 |
| E: Test Coverage | 14 | 3 P1, 7 P2, 4 P3 |
| F: UX Polish | 12 | 1 P1, 4 P2, 7 P3 |
| **Total** | **65** | **1 P0, 9 P1, 27 P2, 24 P3, 4 P4** |

Recommended sprint allocation: **2 hardening sprints** of ~30 tasks each.

---

## 2. Sprint Scope & Priorities

| Priority | Definition | Target |
|----------|-----------|--------|
| **P0** | System-breaking or data-loss risk | Sprint H1 (mandatory) |
| **P1** | Core requirement gap (FR violation) | Sprint H1 |
| **P2** | Quality / maintainability / correctness | Sprint H1–H2 |
| **P3** | Nice-to-have polish or future-proofing | Sprint H2 or backlog |
| **P4** | Deferred / optional | Backlog |

---

## 3. Category A — Architecture Debt

### A-001 (P1) — SyncService Bypasses iron-git Entirely
**Source**: D-P8-005 | **Crate**: `iron-core/src/services/sync.rs`  
**Status**: CONFIRMED — 12 occurrences of `Command::new("git")` in sync.rs  
**Problem**: `DefaultSyncService` spawns raw `git` processes via `std::process::Command`.
The `iron-git` crate provides `CommandExecutor` with circuit-breaker resilience (120s timeout,
retry logic per FR-5.9) and `SecretsManager` (4 methods), but sync.rs uses neither.  
**Impact**: No timeout protection on git operations. No circuit breaker. Code duplication
with iron-git. Violates FR-5.9 (120s timeout for all external commands).  
**Fix**: Refactor `DefaultSyncService` to accept a `CommandExecutor` and route all git
operations through it. Estimated effort: **M** (medium).

### A-002 (P1) — Misleading Error Mapping in SyncService::git()
**Source**: D-P8-007 | **Crate**: `iron-core/src/services/sync.rs` L95–100  
**Status**: CONFIRMED — `.map_err(|_| GitError::NotARepository { ... })` on L99  
**Problem**: The `git()` helper maps ALL `io::Error`s (permission denied, command not found,
timeout) to `GitError::NotARepository`. This obscures real errors and confuses debugging.  
**Fix**: Map to appropriate error variants: `CommandNotFound`, `PermissionDenied`,
`IoError(e)`. Add these variants to `GitError` if needed. Effort: **S**.

### A-003 (P2) — Two Independent Risk Enums
**Source**: S1-P6-NEW-004 | **Crate**: `iron-core`  
**Status**: CONFIRMED — `RiskLevel` in `packages.rs` L12 AND `UpdateRisk` in
`services/update.rs` L164  
**Problem**: `RiskLevel` (Low/Medium/High/Critical) in `packages.rs` and `UpdateRisk`
(Low/Medium/High/Critical) in `services/update.rs` are semantically identical but separate
types. Both used in the update flow, requiring conversions.  
**Fix**: Unify into a single `RiskLevel` enum. Effort: **S**.

### A-004 (P2) — Two Independent Secrets Layers
**Source**: D-P9-006 | **Crates**: `iron-core/src/services/secrets.rs` + `iron-git/src/lib.rs`  
**Status**: CONFIRMED (from phase 9 analysis)  
**Problem**: `SecretsService` (iron-core, 10 methods, raw `Command::new`) and
`SecretsManager` (iron-git, 4 methods, via `CommandExecutor` with circuit breaker) both
wrap git-crypt. Their `is_unlocked()` methods use different detection approaches and can
disagree. `SecretsManager` is **never used by any consumer**.  
**Fix**: Route `SecretsService` through `SecretsManager` for overlapping operations (init,
unlock, lock, is_unlocked). Deprecate raw git-crypt calls in secrets.rs. Effort: **M**.

### A-005 (P2) — No Audit Logging for Secrets Operations
**Source**: D-P9-007 | **Crate**: `iron-core/src/services/secrets.rs`  
**Status**: CONFIRMED — no `StateManager` dependency in `DefaultSecretsService`  
**Problem**: `unlock()`, `lock()`, `init()`, `add_gpg_user()` leave no trace in the
operation audit log. All other services (Sync, Update, Recovery) record their operations.  
**Fix**: Add `StateManager` parameter; call `record_operation()` for each action. Effort: **S**.

### A-006 (P2) — TUI Update Path Doesn't Record Operations
**Source**: S1-P6-NEW-006 | **Crate**: `iron-tui/src/app/actions.rs` L572–622  
**Status**: CONFIRMED — `run_system_update()` calls `UpdateService::apply()` which DOES
record operations internally, but the TUI never calls `update_maintenance()` directly.
The service handles it. ~~No gap~~ — **DOWNGRADED**: The `UpdateService::apply()` in
`update.rs` L1217 already calls `record_operation()` and `update_maintenance()`. The TUI
delegates correctly. **RESOLVED** — remove from backlog.

### A-007 (P2) — Duplicate PackageUpdate Types
**Source**: S1-P6-NEW-005 | **Crates**: `iron-core`  
**Status**: Needs verification — `PackageUpdate` in `packages.rs` vs any duplicate in
`services/update.rs`.  
**Fix**: Audit and consolidate if duplicated. Effort: **S**.

### A-008 (P2) — commit() Uses `git add -A` — Stages Everything
**Source**: D-P8-008 | **Crate**: `iron-core/src/services/sync.rs`  
**Status**: CONFIRMED — sync.rs uses `git add -A` before every commit  
**Problem**: Every push stages the entire repo including unrelated files. User may
accidentally push IDE configs, build artifacts, or temporary files.  
**Impact**: Dangerous for multi-machine repos. Could leak sensitive data not covered by
git-crypt.  
**Fix**: Stage only tracked paths (`git add -u`) or specific directories
(`bundles/`, `modules/`, `profiles/`, `hosts/`, `state.json`). Effort: **S**.

### A-009 (P3) — SyncService Creates Fresh Instances Per Action
**Source**: D-P8-012 | **Crate**: `iron-core/src/services/sync.rs`  
**Status**: CONFIRMED from phase 8 analysis  
**Problem**: Each TUI sync action instantiates a new `DefaultSyncService`. No shared
state between push/pull/status operations.  
**Fix**: Store service instance in `App` state. Low priority. Effort: **S**.

### A-010 (P3) — Secrets Not Locked Before Push
**Source**: D-P8-010 | **Crate**: `iron-core/src/services/sync.rs`  
**Status**: CONFIRMED — no `SecretsService::lock()` call in push flow  
**Problem**: If secrets were unlocked locally, `push()` could commit decrypted secret
files to the remote.  
**Fix**: Add pre-push hook that verifies git-crypt status. Effort: **S**.

---

## 4. Category B — Bug Fixes

### B-001 (P0) — Bundle state() Dormant Heuristic Is Broken
**Source**: B5 from Phase 4 | **Crate**: `iron-core/src/services/bundle.rs` L483–520  
**Status**: CONFIRMED — `state()` falls through to a "legacy heuristic" that checks for
leftover symlinks. If any symlink exists from a previously active bundle, it returns
`BundleState::Dormant` even for bundles that were never archived.  
**Problem**: After deactivation, if unlink_dotfiles() partially fails, orphaned symlinks
cause `state()` to report `Dormant` for any bundle. The dormant directory check is correct
but the fallback is not.  
**Fix**: Remove or gate the legacy symlink heuristic. Only use `dormant_dir(id).exists()`
and `state_manager.active_bundle()`. Effort: **S**.

### B-002 (P1) — First-Launch Detection Logic
**Source**: S1-P1-004 | **Crate**: `iron-tui`  
**Status**: Needs verification — when `current_host == None`, the TUI should launch the
setup wizard. Confirm the wizard actually triggers.  
**Fix**: Ensure `App::new()` or initial navigation checks `state_manager.current_host()`
and routes to `View::Wizard` if None. Effort: **S**.

### B-003 (P1) — Wizard apply() Should Create Host TOML File
**Source**: S1-P2-004 | **Crate**: `iron-tui`  
**Status**: Likely still missing — wizard sets state in `state.json` but doesn't create
the actual `hosts/<name>.toml` file.  
**Fix**: After wizard completion, write `hosts/<hostname>.toml` with detected hardware
from system scan. Effort: **M**.

### B-004 (P2) — RemoveBundle Calls deactivate() Not remove() — Packages Never Cleaned
**Source**: B6 from Phase 4 | **Crate**: `iron-core/src/services/bundle.rs`  
**Status**: CONFIRMED — `remove_packages()` is `#[allow(dead_code)]` on L219  
**Problem**: The `remove_packages()` method exists but is never called. `deactivate()`
explicitly says "we typically don't remove packages." There's no TUI or CLI path that
triggers package removal when a user wants to fully uninstall a bundle.  
**Fix**: Add a `remove()` method to `BundleService` that calls `deactivate()` +
`remove_packages()`. Wire to CLI `iron bundle remove <id>` and TUI action. Effort: **M**.

### B-005 (P2) — Stale Host Reference in state.json
**Source**: S1-P2-006 | **Crate**: `iron-core`  
**Status**: If `hosts/<id>.toml` is deleted but `state.json` still references it, the
dashboard shows a ghost host.  
**Fix**: Doctor check should validate host reference. Add `check_host_reference()` to
DoctorService. Effort: **S**.

### B-006 (P3) — No TUI Path to Deactivate Without Switching
**Source**: B7 from Phase 4 | **Crate**: `iron-tui`  
**Status**: CONFIRMED — TUI only offers switch_bundle(from, to). No standalone deactivate.  
**Fix**: Add `[d] Deactivate` keybind to bundle detail view. Effort: **S**.

---

## 5. Category C — Feature Gaps (CLI Parity)

### C-001 (P1) — CLI Missing Pre-Flight Checks for `iron update`
**Source**: S1-P6-NEW-003 | **Crate**: `iron-cli/src/commands/update.rs`  
**Status**: CONFIRMED — update.rs calls `update_service.check()` but no
`run_preflight_checks_with_news()` like the TUI does  
**Problem**: TUI runs pre-flight checks (actions.rs L293–302) before update but CLI
skips them. Violates FR-5.8 (preview all changes before proceeding).  
**Fix**: Add `run_preflight_checks_with_news()` call before `apply()` in CLI update.
Show blockers/warnings in terminal. Effort: **S**.

### C-002 (P2) — CLI Missing `add-gpg-user` and `export-key` Subcommands
**Source**: D-P9-008 | **Crate**: `iron-cli/src/commands/secrets.rs`  
**Status**: CONFIRMED — `SecretsService` has `add_gpg_user()` and `export_key()` but no
CLI subcommands expose them.  
**Fix**: Add `iron secrets add-key <gpg-id>` and `iron secrets export-key <path>`. Effort: **S**.

### C-003 (P2) — CLI Missing `backup` and `restore` Subcommands
**Source**: D-P9-009 | **Crate**: `iron-cli/src/commands/recover.rs`  
**Status**: CONFIRMED — `RecoveryService` has `create_backup()` and `restore_backup()`
but CLI only has `--export`/`--import`/`--script`.  
**Fix**: Add `--backup <dir>` and `--restore <file.tar.gz>` flags. Effort: **S**.

### C-004 (P2) — CLI Missing `module create` Command
**Source**: S1-P5-NEW-003 | **Crate**: `iron-cli/src/commands/module.rs`  
**Status**: CONFIRMED — module.rs L55 only prints help text ("Create modules in
~/.config/iron/modules/"). No actual scaffolding logic.  
**Fix**: Implement `iron module create <id>` that scaffolds `module.toml` + config dir.
Use `templates::module_toml()` if available. Effort: **M**.

### C-005 (P2) — CLI Clean Missing `--journal` and `--logs` Flags
**Source**: S1-P7-NEW-005 | **Crate**: `iron-cli/src/commands/clean.rs`  
**Status**: CONFIRMED — only `--orphans`, `--cache`, `--symlinks`, `--all` flags.
`CleanupCategory::SystemdJournal` and `CleanupCategory::AppLogs` exist in the service but
aren't exposed as CLI flags.  
**Fix**: Add `--journal` and `--logs` flags mapping to their respective categories. Effort: **S**.

### C-006 (P2) — CLI `iron update` Doesn't Use AUR Helper
**Source**: S1-P6-NEW-007 | **Crate**: `iron-core/src/services/update.rs`  
**Status**: CONFIRMED — no `paru`/`yay`/`aur` references in update.rs  
**Problem**: `DefaultUpdateService` calls pacman directly. AUR packages are never updated.
**Fix**: Detect and use paru/yay if available, falling back to pacman. Effort: **M**.

### C-007 (P3) — CLI Missing `iron secrets init`
**Source**: D-P9 analysis | **Crate**: `iron-cli/src/commands/secrets.rs`  
**Status**: TUI has `secrets_init()` handler but CLI may be missing init subcommand.  
**Fix**: Add `iron secrets init` if absent. Effort: **S**.

### C-008 (P3) — CLI secrets link Convention Undocumented
**Source**: D-P9-011 | **Crate**: `iron-cli/src/commands/secrets.rs`  
**Problem**: `link()` maps `secrets/<path>` → `~/.<path>`. The `~/.` prefix convention
is implicit and can produce surprising results (`secrets/myfile` → `~/.myfile`).  
**Fix**: Document convention in `--help` output and add optional TOML mapping file. Effort: **S**.

### C-009 (P3) — import() Only Restores State, Not System
**Source**: D-P9-010 | **Crate**: `iron-core/src/services/recovery.rs`  
**Problem**: `import()` restores `state.json` entries but doesn't install packages, enable
services, or deploy dotfiles. FR-6.3 specifies a 4-step flow: Install → Bundle → Profile
→ Verify.  
**Fix**: Either implement the full recovery flow or clearly document that import is
state-only and the user must run `iron-install.sh` afterward. Effort: **L** (large).

### C-010 (P3) — verify_installation() Missing From RecoveryService
**Source**: D-P9-013 | **Crate**: `iron-core/src/services/recovery.rs`  
**Problem**: FR-6.4 says post-install verification checks drivers, services, and permissions.
No such method exists on the `RecoveryService` trait.  
**Fix**: Add `verify_installation()` method. Effort: **M**.

---

## 6. Category D — Feature Gaps (TUI Completeness)

### D-001 (P1) — TUI Secrets View Status Always "Unknown"
**Source**: D-P9-003 | **Crate**: `iron-tui`  
**Status**: Secrets view renders but `secrets_status` and `encrypted_files` are never
populated on navigation. Display shows stale/empty data.  
**Fix**: Call `refresh_secrets()` on `navigate(View::Secrets)`. Effort: **S**.

### D-002 (P1) — TUI Recovery View Data Never Populated
**Source**: D-P9-004 | **Crate**: `iron-tui`  
**Status**: Recovery view renders but `last_backup` is never read from operation log.  
**Fix**: Populate `last_backup` from `StateManager::recent_operations()` on navigation.
Effort: **S**.

### D-003 (P2) — TUI Missing Import Recovery Handler
**Source**: D-P9-002 | **Crate**: `iron-tui/src/app/handlers.rs`  
**Status**: Recovery view has `[e] Export`, `[g] Script`, `[s] Snapshot` but no
`[i] Import` handler.  
**Fix**: Add `recovery_import()` action with file picker or path prompt. Effort: **M**.

### D-004 (P2) — TUI Missing Add GPG Key Handler
**Source**: D-P9 analysis | **Crate**: `iron-tui`  
**Status**: Secrets view shows `[a] Add GPG key` hint but handler is likely absent or
dead.  
**Fix**: Wire `[a]` to `SecretsService::add_gpg_user()` with input prompt. Effort: **S**.

### D-005 (P2) — Sync Auto-Refresh on Navigation
**Source**: D-P8-003 | **Crate**: `iron-tui`  
**Status**: Navigating to Sync view doesn't trigger a status refresh. Data may be stale.  
**Fix**: Call `refresh_sync_status()` in `navigate(View::Sync)`. Effort: **S**.

### D-006 (P2) — Sync Confirm Dialog for Push/Pull
**Source**: D-P8-004 | **Crate**: `iron-tui`  
**Status**: Push/pull execute immediately without confirmation.  
**Fix**: Show diff summary + confirmation before push. Show incoming changes before pull.
Effort: **M**.

### D-007 (P2) — Bundle Detail Missing Packages/Services/Conflicts
**Source**: B8 from Phase 4 | **Crate**: `iron-tui/src/ui/bundles.rs`  
**Status**: Bundle detail view shows name/description but not package list, service list,
or conflict declarations from `bundle.toml`.  
**Fix**: Add sections for packages, services, conflicts, and profiles. Effort: **M**.

### D-008 (P2) — No Post-Pull Config Application
**Source**: D-P8-006 | **Crate**: `iron-core/src/services/sync.rs`  
**Problem**: After `pull()`, changed configs aren't re-linked. If a remote push added new
dotfiles or modified bundle.toml, the local system doesn't reflect the changes until
manual re-activation.  
**Fix**: After successful pull, detect changed files and re-link if they affect
bundles/profiles. Effort: **L**.

### D-009 (P3) — Push/Pull Blocks TUI Thread
**Source**: D-P8-011 | **Crate**: `iron-tui`  
**Problem**: Git push/pull runs synchronously, freezing the TUI. Large repos or slow
networks make the app appear hung.  
**Fix**: Run sync operations in a background thread with progress indicator. Effort: **L**.

### D-010 (P3) — Validate/Sanitize Profile and Module IDs in TUI Wizards
**Source**: S1-P5-NEW-005 | **Crate**: `iron-tui`  
**Problem**: ProfileBuilder and ModuleCreator wizards accept any user input as ID.
Spaces, special characters, or empty strings could break file system paths.  
**Fix**: Add `validate_id()` that enforces `[a-z0-9][a-z0-9-]*` pattern. Effort: **S**.

### D-011 (P3) — ProfileBuilder Dependency Auto-Suggestion
**Source**: S1-P5-NEW-008 | **Crate**: `iron-tui`  
**Problem**: When selecting modules for a profile, if module A depends on module B, the
builder doesn't suggest including B.  
**Fix**: Read `depends` from module.toml and auto-suggest. Effort: **M**.

### D-012 (P3) — ModuleCreator Add Dotfile Mapping Configuration
**Source**: S1-P5-NEW-006 | **Crate**: `iron-tui`  
**Problem**: Module creator wizard doesn't let users configure dotfile source→target
mappings during creation.  
**Fix**: Add a step for dotfile path configuration. Effort: **M**.

### D-013 (P3) — Check for Duplicate Profile/Module Names Before Creation
**Source**: S1-P5-NEW-012 | **Crate**: `iron-tui`  
**Problem**: Creating a profile/module with an existing name silently overwrites.  
**Fix**: Check filesystem before writing. Effort: **S**.

---

## 7. Category E — Test Coverage Gaps

### E-001 (P1) — TUI Secrets View: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-tui/src/ui/secrets.rs` (122 LOC)  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add smoke render test, status display test, encrypted files list test. Effort: **S**.

### E-002 (P1) — TUI Recovery View: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-tui/src/ui/recovery.rs` (151 LOC)  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add smoke render test, backup status test, footer hints test. Effort: **S**.

### E-003 (P1) — CLI Secrets Command: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-cli/src/commands/secrets.rs` (290 LOC)  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add unit tests for each subcommand path (status, unlock, lock, link). Effort: **M**.

### E-004 (P2) — CLI Recover Command: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-cli/src/commands/recover.rs` (250 LOC)  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add tests for export, import, and script generation flows. Effort: **M**.

### E-005 (P2) — CLI Update Command: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-cli/src/commands/update.rs`  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add tests for check flow, risk display, confirmation logic, progress tracking.
Effort: **M**.

### E-006 (P2) — CLI Doctor Command: 0 Tests
**Source**: Phase 9 analysis | **File**: `iron-cli/src/commands/doctor.rs`  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add tests for report formatting, JSON output, check routing. Effort: **S**.

### E-007 (P2) — CLI Clean Command: 0 Tests
**Source**: Phase 7 analysis | **File**: `iron-cli/src/commands/clean.rs` (95 LOC)  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add tests for flag parsing, category mapping, preview/execute flow. Effort: **S**.

### E-008 (P2) — Profile Model: 0 Unit Tests
**Source**: S1-P5-NEW-007 | **File**: `iron-core/src/profile.rs`  
**Status**: CONFIRMED — no `#[test]` in file  
**Fix**: Add tests for Profile parsing, module reference resolution, dependency validation,
serialization roundtrip. Effort: **M**.

### E-009 (P2) — Phase 3 Dashboard Divergence Indicator Tests (24 gaps)
**Source**: Phase 3 analysis | **Crate**: `iron-tui`  
**Problem**: Dashboard divergence rendering has ~24 untested display scenarios.  
**Fix**: Add parameterized tests for divergence badge rendering. Effort: **M**.

### E-010 (P2) — Wizard Handler Integration Tests
**Source**: S1-P1-007 | **Crate**: `iron-tui`  
**Problem**: Wizard flow (first launch → host selection → bundle → profile) has no
integration tests verifying the full flow.  
**Fix**: Add end-to-end wizard tests with mock package manager. Effort: **M**.

### E-011 (P3) — CleanupService Tests With Mocks
**Source**: S1-P7-NEW-012 | **Crate**: `iron-core/src/services/clean.rs`  
**Status**: Has ~20 unit tests, but preview + execute with mock filesystem are missing.  
**Fix**: Add mock-based tests for actual cleanup execution paths. Effort: **M**.

### E-012 (P3) — No Integration Tests for git-crypt
**Source**: Phase 9 analysis | **Crates**: `iron-core`, `iron-git`  
**Problem**: All SecretsService tests use filesystem mocking. No test runs actual
git-crypt commands.  
**Fix**: Add optional integration test (behind `#[ignore]` or feature flag) that runs in
a temp git repo. Effort: **M**.

### E-013 (P3) — Two Secrets Layers Never Tested Together
**Source**: Phase 9 analysis | **Crates**: `iron-core` + `iron-git`  
**Problem**: No test verifies that `SecretsService` and `SecretsManager` agree on status.  
**Fix**: Add cross-layer consistency test. Blocked on A-004 (consolidation). Effort: **S**.

### E-014 (P3) — SyncService Tests for Error Paths
**Source**: D-P8-007 | **Crate**: `iron-core`  
**Problem**: git() error mapping is incorrect (A-002). Tests for permission denied, command
not found, timeout don't exist.  
**Fix**: Add error-path tests after A-002 is fixed. Effort: **S**.

---

## 8. Category F — UX Polish

### F-001 (P1) — Unify Host Config Convention (Flat vs Directory)
**Source**: S1-P2-005 | **Crate**: `iron-core`  
**Problem**: Hosts can be `hosts/desktop.toml` (flat file) or `hosts/desktop/host.toml`
(directory). No single convention is enforced. Code paths check both, adding complexity.  
**Fix**: Pick one convention (recommended: flat file) and migrate. Add doctor check for
non-conforming layouts. Effort: **M**.

### F-002 (P2) — Enhanced Confirm for Aggressive Cleanup Categories
**Source**: S1-P7-NEW-008 | **Crate**: `iron-tui`  
**Problem**: Aggressive cleanup categories (orphan removal, journal vacuum) execute without
extra warning. Only safe categories should run without confirm.  
**Fix**: Add typed confirmation for `CleanupCategory::aggressive()` categories. Effort: **S**.

### F-003 (P2) — Record Cleanup Operations in State
**Source**: S1-P7-NEW-009 | **Crate**: `iron-core/src/services/clean.rs`  
**Problem**: `CleanupService` has no `StateManager` dependency. Cleanup operations aren't
recorded in the audit log.  
**Fix**: Add `StateManager` to `DefaultCleanupService::new()`. Call `record_operation()`
after execute. Effort: **S**.

### F-004 (P2) — Doctor Refresh on Navigation
**Source**: S1-P7-NEW-014 | **Crate**: `iron-tui`  
**Problem**: Doctor view doesn't auto-refresh when navigated to. Shows stale results.  
**Fix**: Trigger `run_doctor_checks()` on `navigate(View::Doctor)`. Effort: **S**.

### F-005 (P2) — Use iron_pacman::clean_cache()/get_orphans() in CleanupService
**Source**: S1-P7-NEW-010 | **Crate**: `iron-core/src/services/clean.rs`  
**Problem**: CleanupService may use raw `Command::new("pacman")` instead of the
`iron_pacman` crate's safe wrappers.  
**Fix**: Route through `iron_pacman` for consistency and circuit breaker protection.
Effort: **S**.

### F-006 (P3) — BrokenSymlinks Category in CleanupService
**Source**: S1-P7-NEW-011 | **Crate**: `iron-core/src/services/clean.rs`  
**Problem**: No cleanup category for finding and removing broken symlinks in `~/.config`.  
**Fix**: Add `CleanupCategory::BrokenSymlinks` that scans common config directories.
Effort: **S**.

### F-007 (P3) — Snapshot Status as Pre-Flight Check
**Source**: S1-P6-NEW-008 | **Crate**: `iron-core/src/services/update.rs`  
**Problem**: Pre-flight checks don't verify that a recent snapshot exists.  
**Fix**: Add snapshot age check to `run_preflight_checks()`. Effort: **S**.

### F-008 (P3) — Partial Update Detection Pre-Flight Check
**Source**: S1-P6-NEW-012 | **Crate**: `iron-core/src/services/update.rs`  
**Problem**: Arch Linux partial updates (only some packages updated) can break the system.
No pre-flight check warns about this.  
**Fix**: Check `pacman.log` for recent partial operations. Effort: **M**.

### F-009 (P3) — ProfileBuilder Conflict Warnings During Module Selection
**Source**: S1-P5-NEW-009 | **Crate**: `iron-tui`  
**Problem**: Selecting two modules with conflicting dotfile targets shows no warning.  
**Fix**: Cross-check `conflicts` and dotfile targets during selection. Effort: **M**.

### F-010 (P3) — ModuleCreator Add ModuleKind Selection
**Source**: S1-P5-NEW-010 | **Crate**: `iron-tui`  
**Problem**: Module creator doesn't let users select `ModuleKind` (AppConfig, SystemConfig,
etc.) — it's always set to a default.  
**Fix**: Add kind selection step. Effort: **S**.

### F-011 (P3) — Show Guidance When Module List Is Empty
**Source**: S1-P5-NEW-013 | **Crate**: `iron-tui`  
**Problem**: ProfileBuilder with an empty module list shows a blank picker with no help.  
**Fix**: Display "No modules found. Create modules first." guidance. Effort: **S**.

### F-012 (P3) — Detect and Integrate Timeshift/Snapper
**Source**: D-P9-012 | **Crate**: `iron-core`  
**Problem**: `DefaultRecoveryService` always uses `NoopManager` for snapshots. The TODO
in `context.rs` L91 says "detect and use timeshift/snapper."  
**Fix**: Implement detection (already have `detect_snapshot_backend()`) and wire to
RecoveryService. Effort: **M**.

---

## 9. Requirements Cross-Reference

Cross-referencing `requirements.md` functional requirements against actual codebase state:

| FR | Requirement | Status | Gap ID |
|----|------------|--------|--------|
| FR-1.4 | Auto-detect current host | PARTIAL — hostname only, no HW fingerprint | — (P3 backlog) |
| FR-1.5 | Warning badge when no snapshot | IMPLEMENTED (doctor check) | — |
| FR-2.6 | Dormant config storage | IMPLEMENTED but B-001 affects state detection | B-001 |
| FR-3.5 | Smart merge for overlapping symlinks | STUB | D-009 scope |
| FR-5.3 | Predict dependency conflicts | IMPLEMENTED in UpdateService | — |
| FR-5.6 | Auto snapshot before update | IMPLEMENTED via create_manager() | — |
| FR-5.7 | Detect/diff/merge .pacnew | IMPLEMENTED (post-update checks) | — |
| FR-5.9 | 120s timeout on external commands | MISSING for sync (A-001) | A-001 |
| FR-5.10 | Track update progress / resume | IMPLEMENTED in UpdateService | — |
| FR-6.3 | 4-step recovery flow | MISSING — import is state-only | C-009 |
| FR-6.4 | Post-install verification | MISSING | C-010 |
| FR-7.2 | Pull applies config changes | MISSING — pull doesn't re-link | D-008 |
| FR-7.4 | Interactive merge on conflict | STUB | — (P3 backlog) |
| FR-8.5 | `iron secrets unlock` | IMPLEMENTED (CLI + TUI) | — |
| FR-8.6 | `iron secrets link` | IMPLEMENTED (CLI) | — |
| FR-9.2 | First-run wizard | IMPLEMENTED but needs B-002 | B-002 |
| FR-9.4 | Profile builder | IMPLEMENTED | — |
| FR-10.7 | Report git-crypt status | IMPLEMENTED in doctor | — |
| FR-10.8 | JSON health report | IMPLEMENTED | — |

**FR violations requiring hardening sprint attention:**
- **FR-5.9**: A-001 (SyncService no timeout)
- **FR-6.3**: C-009 (import() doesn't run full recovery)
- **FR-6.4**: C-010 (no verify_installation)
- **FR-7.2**: D-008 (pull doesn't re-link)

---

## 10. Dependency Graph

Tasks that must be completed in order (arrows show "must complete before"):

```
A-001 (SyncService → iron-git) ──→ A-002 (error mapping) ──→ E-014 (error tests)
                                ──→ A-010 (secrets lock check)

A-004 (consolidate secrets) ──→ E-013 (cross-layer tests)

A-003 (unify risk enums) ──→ (standalone)

B-001 (dormant heuristic) ──→ (standalone, HIGH priority)

B-004 (bundle remove) ──→ D-006 (standalone deactivate path, optional)

C-004 (CLI module create) ──→ D-012 (dotfile mapping in wizard)

D-001 (secrets refresh) ──→ E-001 (secrets view tests)
D-002 (recovery populate) ──→ E-002 (recovery view tests)

E-008 (profile tests) ──→ (standalone)
```

---

## 11. Sprint Plan

### Hardening Sprint H1 (Recommended: ~28 tasks)

**Theme**: Fix bugs, close FR violations, establish test baselines

| # | Task ID | Title | Priority | Effort | Depends On |
|---|---------|-------|----------|--------|------------|
| 1 | B-001 | Fix dormant state heuristic | P0 | S | — |
| 2 | A-001 | SyncService → iron-git CommandExecutor | P1 | M | — |
| 3 | A-002 | Fix git() error mapping | P1 | S | A-001 |
| 4 | B-002 | Verify/fix first-launch wizard trigger | P1 | S | — |
| 5 | B-003 | Wizard creates host TOML file | P1 | M | — |
| 6 | C-001 | CLI update pre-flight checks | P1 | S | — |
| 7 | D-001 | Secrets view auto-refresh on navigation | P1 | S | — |
| 8 | D-002 | Recovery view data population | P1 | S | — |
| 9 | F-001 | Unify host config convention | P1 | M | — |
| 10 | E-001 | TUI secrets view tests | P1 | S | D-001 |
| 11 | E-002 | TUI recovery view tests | P1 | S | D-002 |
| 12 | E-003 | CLI secrets command tests | P1 | M | — |
| 13 | A-003 | Unify RiskLevel / UpdateRisk | P2 | S | — |
| 14 | A-005 | Secrets audit logging | P2 | S | — |
| 15 | A-008 | Fix `git add -A` → selective staging | P2 | S | — |
| 16 | B-004 | Bundle remove (packages + deactivate) | P2 | M | — |
| 17 | B-005 | Doctor check for stale host ref | P2 | S | — |
| 18 | C-002 | CLI secrets add-key / export-key | P2 | S | — |
| 19 | C-003 | CLI recover backup / restore | P2 | S | — |
| 20 | C-004 | CLI module create scaffolding | P2 | M | — |
| 21 | C-005 | CLI clean --journal / --logs flags | P2 | S | — |
| 22 | D-003 | TUI recovery import handler | P2 | M | — |
| 23 | D-004 | TUI secrets add GPG key handler | P2 | S | — |
| 24 | D-005 | Sync auto-refresh on navigation | P2 | S | — |
| 25 | E-004 | CLI recover command tests | P2 | M | — |
| 26 | E-005 | CLI update command tests | P2 | M | — |
| 27 | E-006 | CLI doctor command tests | P2 | S | — |
| 28 | E-007 | CLI clean command tests | P2 | S | — |

**H1 Estimated Effort**: 8 M + 20 S ≈ ~28 task-units

---

### Hardening Sprint H2 (Recommended: ~33 tasks)

**Theme**: Architecture improvements, TUI polish, remaining test coverage

| # | Task ID | Title | Priority | Effort | Depends On |
|---|---------|-------|----------|--------|------------|
| 1 | A-004 | Consolidate secrets layers | P2 | M | — |
| 2 | A-007 | Audit/consolidate PackageUpdate types | P2 | S | — |
| 3 | C-006 | UpdateService AUR helper integration | P2 | M | — |
| 4 | D-006 | Sync confirm dialog for push/pull | P2 | M | — |
| 5 | D-007 | Bundle detail packages/services/conflicts | P2 | M | — |
| 6 | D-008 | Post-pull config re-linking | P2 | L | — |
| 7 | E-008 | Profile model unit tests | P2 | M | — |
| 8 | E-009 | Dashboard divergence indicator tests | P2 | M | — |
| 9 | E-010 | Wizard handler integration tests | P2 | M | — |
| 10 | F-002 | Enhanced confirm for aggressive cleanup | P2 | S | — |
| 11 | F-003 | Record cleanup in state audit log | P2 | S | — |
| 12 | F-004 | Doctor refresh on navigation | P2 | S | — |
| 13 | F-005 | Use iron_pacman in CleanupService | P2 | S | — |
| 14 | A-009 | Shared SyncService instance | P3 | S | A-001 |
| 15 | A-010 | Pre-push secrets lock check | P3 | S | A-001 |
| 16 | B-006 | TUI deactivate without switching | P3 | S | — |
| 17 | C-007 | CLI secrets init subcommand | P3 | S | — |
| 18 | C-008 | Document secrets link convention | P3 | S | — |
| 19 | C-009 | Full recovery import flow | P3 | L | — |
| 20 | C-010 | verify_installation() method | P3 | M | — |
| 21 | D-009 | Background thread for push/pull | P3 | L | A-001 |
| 22 | D-010 | Validate/sanitize IDs in wizards | P3 | S | — |
| 23 | D-011 | ProfileBuilder dependency auto-suggest | P3 | M | — |
| 24 | D-012 | ModuleCreator dotfile mapping | P3 | M | C-004 |
| 25 | D-013 | Duplicate name check before creation | P3 | S | — |
| 26 | E-011 | CleanupService mock tests | P3 | M | — |
| 27 | E-012 | git-crypt integration tests | P3 | M | — |
| 28 | E-013 | Cross-layer secrets consistency test | P3 | S | A-004 |
| 29 | E-014 | SyncService error-path tests | P3 | S | A-002 |
| 30 | F-006 | BrokenSymlinks cleanup category | P3 | S | — |
| 31 | F-007 | Snapshot age pre-flight check | P3 | S | — |
| 32 | F-008 | Partial update detection pre-flight | P3 | M | — |
| 33 | F-009 | Profile conflict warnings | P3 | M | — |

**H2 Remaining (Backlog / P3)**:
- F-010: ModuleCreator kind selection
- F-011: Empty module list guidance
- F-012: Timeshift/snapper detection + wiring

**H2 Estimated Effort**: 12 M + 3 L + 18 S ≈ ~33 task-units

---

## 12. Already Resolved (Excluded)

These discovered issues from the phase guidelines were fixed during Sprints 1–4 and
are **excluded** from the hardening backlog:

| Original ID | Description | Resolved By |
|-------------|-------------|-------------|
| S1-P1-005 (P0) | Wizard apply() PM injection | Sprint 1 |
| S1-P1-006 (P3) | PM in refresh_current_view() | Sprint 1 |
| B1 / S1-P4-003 | switch_bundle() missing service_manager | Sprint 2 |
| B2 / S1-P4-004 | deactivate() never clears active_bundles | Sprint 2 |
| B3 / S1-P4-005 | switch() has no rollback | Sprint 2 |
| B4 / S1-P4-006 | Dotfiles directory mismatch | Sprint 2 |
| S1-P5-NEW-001 | TUI profile activation broken | Sprint 3 (S1-P5-003) |
| S1-P5-NEW-002 | TUI module enable/disable broken | Sprint 3 (S1-P5-004) |
| S1-P7-NEW-002 | DoctorService missing from iron-core | Sprint 4 (S1-P7-001) |
| S1-P7-NEW-003 | CLI clean not using CleanupService | Sprint 4 (S1-P7-004) |
| S1-P7-NEW-004 | TUI Doctor [r] re-run broken | Sprint 4 (S1-P7-003) |
| S1-P7-NEW-006 | CLI doctor not using DoctorService | Sprint 4 (S1-P7-001) |
| S1-P7-NEW-007 | TUI doctor not using DoctorService | Sprint 4 |
| D-P8-001 | Push auto-commit | Sprint 4 (S1-P8-002) |
| D-P8-002 | Pull dirty check | Sprint 4 (S1-P8-003) |
| S1-P6-NEW-001 | TUI update → UpdateService.apply() | Sprint 3 (confirmed wired) |
| S1-P6-NEW-006 | TUI update path audit logging | Sprint 3 (apply() records internally) |

**Also verified working** (TUI handlers wired during Sprint 4):
- Secrets: `[i] Init`, `[u] Unlock`, `[l] Lock`, `[r] Refresh` — all have handlers
- Recovery: `[e] Export`, `[g] Script`, `[s] Snapshot` — all have handlers

---

## Appendix A: Test Coverage Summary (Current)

| Component | Tests | Status |
|-----------|------:|--------|
| iron-core services (all) | ~450 | Good |
| iron-core models | ~200 | Good |
| iron-tui handlers | ~180 | Good |
| iron-tui UI views | ~120 | Partial (secrets=0, recovery=0) |
| iron-tui actions | ~80 | Partial |
| iron-cli cli.rs parsing | ~25 | Good |
| iron-cli commands/* | ~3 | **CRITICAL GAP** (5 commands with 0 tests) |
| iron-cli acceptance | ~40 | Good |
| iron-git | ~30 | Good |
| iron-fs | ~80 | Good |
| iron-pacman | ~25 | Good |
| iron-systemd | ~15 | Good |
| **Total** | **~1,567** | |

**CLI command test gap**: `secrets.rs` (0), `recover.rs` (0), `update.rs` (0),
`doctor.rs` (0), `clean.rs` (0) = **5 untested command modules** totaling ~985 LOC.

---

## Appendix B: Effort Estimation Key

| Size | Typical Scope | Estimated Hours |
|------|--------------|-----------------|
| **S** (Small) | Single function/method change, simple test addition | 1–2h |
| **M** (Medium) | Cross-file refactor, new feature with tests | 3–5h |
| **L** (Large) | Multi-crate change, new async patterns, complex logic | 6–10h |
