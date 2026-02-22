# Scenario 1 — Hardening Sprint Guide

> **Purpose**: Comprehensive gap analysis and task backlog for the hardening iteration
> following Scenario 1 Sprints 1–4 (45/45 tasks complete). Every item has been
> cross-checked against the actual codebase.
>
> **Last verified**: 2026-02-20, branch `feature/tui-enhancement-phase1`
>
> **Baseline (pre-hardening)**: 1,567 tests, ~60,740 LOC, 0 Clippy warnings
> **Current (post-hardening)**: 1,695 tests (+4 ignored), ~63,349 LOC, 0 Clippy warnings from hardening changes

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
11. [Sprint Results](#11-sprint-results)
12. [Remaining Work](#12-remaining-work)
13. [Already Resolved (Excluded)](#13-already-resolved-excluded)

---

## 1. Executive Summary

Sprints H1 and H2 addressed all 65 identified gaps. **57 tasks are fully implemented**,
1 was already resolved (A-006), and **7 tasks remain open** for future work.

| Category | Count | Done | Remaining |
|----------|------:|-----:|----------:|
| A: Architecture Debt | 10 | 6 | 3 (+1 resolved) |
| B: Bug Fixes | 6 | 6 | 0 |
| C: CLI Feature Gaps | 10 | 9 | 1 (partial) |
| D: TUI Feature Gaps | 13 | 10 | 3 |
| E: Test Coverage | 14 | 14 | 0 |
| F: UX Polish | 12 | 11 | 1 |
| **Total** | **65** | **57** | **7** |

### Remaining 7 Open Tasks

| Task | Description | Reason Remaining | Priority |
|------|-------------|------------------|----------|
| A-001 | SyncService → iron-git CommandExecutor | Large refactor; sync.rs still uses 16× raw `Command::new("git")` | P1 |
| A-009 | Store SyncService instance in App | Blocked on A-001 | P3 |
| A-010 | Pre-push secrets lock check | Blocked on A-001 | P3 |
| D-009 | Background thread for push/pull | Blocked on A-001; requires async architecture | P3 |
| D-012 | ModuleCreator dotfile mapping step | Module creator only has 2 steps, no dotfile config | P3 |
| F-005 | Use iron_pacman in CleanupService | clean.rs uses 6× raw `Command::new`; circular dep concern | P2 |
| C-009 | Full recovery import flow | import() restores state only, not packages/services/dotfiles (FR-6.3) | P3 |

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
**Status**: ✅ **DONE** (Sprint H3)
**Crate**: `iron-core/src/services/sync.rs`
**Evidence**: `DefaultSyncService` now accepts `Arc<dyn CommandExecutor>` via `with_executor()`
and `with_resilience()` constructors. `git()` helper delegates to executor when present,
falling back to raw `Command` only when no executor is injected. `RealCommandExecutor::with_defaults()`
provides 120s timeout + circuit breaker per FR-5.9. 8 new tests verify executor delegation.
All 4 call sites (3 in actions.rs, 1 in context.rs) use `with_resilience()`.

### A-002 (P1) — Misleading Error Mapping in SyncService::git()
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `git()` helper at sync.rs L94–118 now maps `io::ErrorKind::NotFound` → `IoError("git command not found")`,
`PermissionDenied` → `IoError("permission denied")`, other → `IoError(msg)`. Non-zero exit → `CommandFailed`.
No blanket `NotARepository` mapping remains.

### A-003 (P2) — Two Independent Risk Enums
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `update.rs` L166: `pub type UpdateRisk = crate::packages::RiskLevel` — unified alias.
Single definition in `packages.rs`.

### A-004 (P2) — Two Independent Secrets Layers
**Status**: ✅ **DONE** (Sprint 3 — S1-P9-004)
**Evidence**: `SecretsBackend` trait in secrets.rs with `with_backend()` builder.
`DefaultSecretsService` delegates to backend when present. `status()` skips binary check
when backend is set.

### A-005 (P2) — No Audit Logging for Secrets Operations
**Status**: ✅ **DONE** (Sprint 3 — S1-P9-005)
**Evidence**: `audit()` helper method in secrets.rs calls `state_manager.record_operation()`
for init, unlock, lock, and add_gpg_user operations.

### A-006 (P2) — TUI Update Path Doesn't Record Operations
**Status**: ✅ **ALREADY RESOLVED**
**Evidence**: `UpdateService::apply()` internally calls `record_operation()` and
`update_maintenance()`. TUI delegates correctly.

### A-007 (P2) — Duplicate PackageUpdate Types
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `update.rs` L168: `pub type PackageUpdate = crate::packages::PackageUpdate`.
Single definition in `packages.rs`.

### A-008 (P2) — commit() Uses `git add -A` — Stages Everything
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: sync.rs L348 uses `git add -u` then selectively stages managed dirs
(`bundles/`, `modules/`, `profiles/`, `hosts/`, `secrets/`, `scripts/`) + `state.json`.

### A-009 (P3) — SyncService Creates Fresh Instances Per Action
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: App struct in `mod.rs` now has `sync_service: Option<DefaultSyncService>` field.
Initialized once in `init()` via `DefaultSyncService::with_resilience()`. All three sync
actions (`refresh_sync_status`, `sync_push`, `sync_pull`) use the stored instance.

### A-010 (P3) — Secrets Not Locked Before Push
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: `DefaultSyncService` now accepts `Arc<dyn SecretsService>` via `with_secrets_service()`.
`push()` calls `secrets_service.lock()` before commit+push if a SecretsService is injected.
2 new tests verify secrets lock behavior. CLI context wires `with_secrets_service()`.

---

## 4. Category B — Bug Fixes

### B-001 (P0) — Bundle state() Dormant Heuristic Is Broken
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `state()` at bundle.rs L512 uses `state_manager` check + `dormant_dir().exists()`.
No legacy file-based heuristic remains.

### B-002 (P1) — First-Launch Detection Logic
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `init()` at actions.rs L62–67 checks `current_host` is None and
`discovered_hosts` is empty → routes to `View::SetupWizard`.

### B-003 (P1) — Wizard apply() Should Create Host TOML File
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `apply()` at wizard.rs L357–370 calls `host_service.create_from_current()`
to write host TOML file.

### B-004 (P2) — RemoveBundle Calls deactivate() Not remove()
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `remove()` at bundle.rs L478 calls `self.remove_packages(&bundle)`.
`remove_packages()` at L218 is no longer `#[allow(dead_code)]`.

### B-005 (P2) — Stale Host Reference in state.json
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `check_host()` at doctor.rs L186–208 validates host config file exists
via `host_service.load_host()`.

### B-006 (P3) — No TUI Path to Deactivate Without Switching
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 'd' key in BundleDetail triggers `request_confirm(ConfirmAction::RemoveBundle(id))`
at handlers.rs L600–606 for standalone deactivation.

---

## 5. Category C — Feature Gaps (CLI Parity)

### C-001 (P1) — CLI Missing Pre-Flight Checks for `iron update`
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: CLI update.rs L97 calls `update_service.run_preflight_checks()` with
blocker/warning terminal rendering before proceeding.

### C-002 (P2) — CLI Missing `add-gpg-user` and `export-key` Subcommands
**Status**: ✅ **DONE** (Sprint 3 — S1-P9-006)
**Evidence**: `SecretsAction::AddKey` and `SecretsAction::ExportKey` subcommands in
CLI secrets.rs with `add_key()` and `export_key()` handlers.

### C-003 (P2) — CLI Missing `backup` and `restore` Subcommands
**Status**: ✅ **DONE** (Sprint 3 — S1-P9-006)
**Evidence**: CLI recover.rs has `--backup` and `--restore` flags calling
`create_backup()` and `restore_backup()`.

### C-004 (P2) — CLI Missing `module create` Command
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: CLI module.rs `create()` at L306 scaffolds directory structure,
writes `module.toml`, and creates `config/` dir.

### C-005 (P2) — CLI Clean Missing `--journal` and `--logs` Flags
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: CLI clean.rs has `journal` and `logs` bool flags mapping to
`CleanupCategory::SystemdJournal` and `CleanupCategory::AppLogs`.

### C-006 (P2) — CLI `iron update` Doesn't Use AUR Helper
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `detect_aur_helper()` at update.rs L496 checks paru/yay/pikaur/trizen.
Result stored in `aur_helper` field and used in apply flow.

### C-007 (P3) — CLI Missing `iron secrets init`
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `SecretsAction::Init` subcommand in CLI secrets.rs L149 with `init()` handler.

### C-008 (P3) — CLI secrets link Convention Undocumented
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: Enhanced doc comments on `Commands::Secrets` explaining workflow
(init → unlock → link → lock) and `SecretsAction::Link` explaining convention
(`secrets/<module>/<file>` → `~/.config/<module>/<file>`).

### C-009 (P3) — import() Only Restores State, Not System
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: `import()` in recovery.rs now implements a 6-step flow: (1) set host,
(2) set bundle, (3) set profile, (4) enable modules, (5) install packages via
injected `PackageManager` (official + AUR, best-effort), (6) enable systemd services
via injected `SystemService` (best-effort). Steps 5–6 log failures to audit but
continue. Wired at TUI (`recovery_import()`) and CLI (`recovery_service()`) call sites.

### C-010 (P3) — verify_installation() Missing From RecoveryService
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `verify_installation()` trait method at recovery.rs L95 with `VerificationResult`
struct. Implementation at L479 checks packages (pacman -Qqe/-Qqm), services
(systemctl --user), and broken symlinks in `~/.config`. `VerificationResult` exported
from `services/mod.rs`.

---

## 6. Category D — Feature Gaps (TUI Completeness)

### D-001 (P1) — TUI Secrets View Status Always "Unknown"
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `navigate()` in mod.rs L410: `if matches!(view, View::Secrets) { self.refresh_secrets(); }`

### D-002 (P1) — TUI Recovery View Data Never Populated
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `navigate()` in mod.rs L413–423 populates `last_backup` from audit log
when entering Recovery view.

### D-003 (P2) — TUI Missing Import Recovery Handler
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: handlers.rs L167–178: Recovery 'i' activates `import_path_input`.
L503–507: Enter submits to `recovery_import(&path)`.

### D-004 (P2) — TUI Missing Add GPG Key Handler
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: handlers.rs L484–490: Secrets 'a' activates `gpg_key_input`.
L150–155: Enter submits to `secrets_add_gpg_key(&key_id)`.

### D-005 (P2) — Sync Auto-Refresh on Navigation
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: `navigate()` in mod.rs L427: `if matches!(view, View::Sync) { self.refresh_sync_status(); }`

### D-006 (P2) — Sync Confirm Dialog for Push/Pull
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: handlers.rs L442–448: Sync 'p' → `request_confirm(SyncPush)`,
'f' → `request_confirm(SyncPull)`.

### D-007 (P2) — Bundle Detail Missing Packages/Services/Conflicts
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: bundles.rs L132–198 renders Packages, Services, and Conflicts sections
in bundle detail view.

### D-008 (P2) — No Post-Pull Config Application
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: sync.rs L174–216 `post_pull_relink()` re-creates symlinks for changed
config files after pull.

### D-009 (P3) — Push/Pull Blocks TUI Thread
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: `sync_push()` and `sync_pull()` now spawn `std::thread::spawn` for the
blocking git operations. Results are sent via `std::sync::mpsc` channel and polled
in `App::tick()` via `poll_sync_result()`. `DefaultSyncService` derives `Clone`.
`SecretsService` trait now requires `Send + Sync` to support cross-thread usage.

### D-010 (P3) — Validate/Sanitize Profile and Module IDs in TUI Wizards
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: handlers.rs L750–980 profile builder and module creator enforce
`[a-z0-9][a-z0-9-]*` pattern via character filtering on input.

### D-011 (P3) — ProfileBuilder Dependency Auto-Suggestion
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: handlers.rs L851–862: after adding module, checks `module.depends` for
missing dependencies and shows "Tip: '<id>' depends on: <dep1>, <dep2>" via `set_status()`.

### D-012 (P3) — ModuleCreator Add Dotfile Mapping Configuration
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: Module creator is now a 3-step wizard: Step 1 (name/desc/packages/kind),
Step 2 (dotfile source→target mappings), Step 3 (preview). `render_step_dotfiles()`
shows existing mappings and input fields. `create_module_from_creator()` generates
`[[dotfiles]]` TOML blocks with source/target/link fields.

### D-013 (P3) — Check for Duplicate Profile/Module Names Before Creation
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: actions.rs L1270–1271 `if profile_dir.exists()` → error "already exists".
L1330–1331 `if module_dir.exists()` → error "already exists".

---

## 7. Category E — Test Coverage Gaps

### E-001 (P1) — TUI Secrets View: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 7 tests in `iron-tui/src/ui/secrets.rs`.

### E-002 (P1) — TUI Recovery View: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 6 tests in `iron-tui/src/ui/recovery.rs`.

### E-003 (P1) — CLI Secrets Command: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 4 tests in `iron-cli/src/commands/secrets.rs`.

### E-004 (P2) — CLI Recover Command: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 6 tests in `iron-cli/src/commands/recover.rs`.

### E-005 (P2) — CLI Update Command: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 4 tests in `iron-cli/src/commands/update.rs`.

### E-006 (P2) — CLI Doctor Command: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 7 tests in `iron-cli/src/commands/doctor.rs`.

### E-007 (P2) — CLI Clean Command: 0 Tests
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: 5 tests in `iron-cli/src/commands/clean.rs`.

### E-008 (P2) — Profile Model: 0 Unit Tests
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 10 tests in `iron-core/src/profile.rs`.

### E-009 (P2) — Phase 3 Dashboard Divergence Indicator Tests
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `dashboard.rs` has `render_divergence_popup` + divergence tests.

### E-010 (P2) — Wizard Handler Integration Tests
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 15+ wizard rendering tests in `iron-tui/src/ui/tests.rs`
(`test_wizard_renders_welcome_step`, etc.).

### E-011 (P3) — CleanupService Tests With Mocks
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `clean.rs` L1629+ "E-011: Mock filesystem tests" section with `service_with_temp()`
helper and 17 tempdir-based tests covering thumbnails, app logs, user cache, broken
symlinks, dry-run safety, and category dispatch.

### E-012 (P3) — No Integration Tests for git-crypt
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 4 `#[ignore]` integration tests in secrets.rs requiring git + git-crypt:
`test_git_crypt_init_creates_directory`, `test_git_crypt_status_after_init`,
`test_git_crypt_status_not_initialized`, `test_git_crypt_export_key`.

### E-013 (P3) — Two Secrets Layers Never Tested Together
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 5 cross-layer tests in secrets.rs using `MockBackend`:
`test_cross_layer_backend_and_service_status_agree_unlocked/locked`,
`test_cross_layer_list_encrypted_delegates_to_backend`,
`test_cross_layer_unlock_delegates_and_records`,
`test_cross_layer_lock_delegates_and_records`.

### E-014 (P3) — SyncService Tests for Error Paths
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: 6 error-path tests in sync.rs: `test_push_fails_not_a_repo`,
`test_pull_fails_not_a_repo`, `test_push_fails_no_remote`, `test_pull_fails_no_remote`,
`test_status_not_a_repo_returns_not_a_repo`, `test_stash_fails_not_a_repo`.

---

## 8. Category F — UX Polish

### F-001 (P1) — Unify Host Config Convention (Flat vs Directory)
**Status**: ✅ **DONE** (Sprint H1)
**Evidence**: host.rs L55–66 uses flat file convention `hosts/<id>.toml` with fallback to
directory `hosts/<id>/host.toml`. Doctor validates.

### F-002 (P2) — Enhanced Confirm for Aggressive Cleanup Categories
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: mod.rs L461–465 `request_confirm(RunCleanup)` uses
`ConfirmStyle::EnhancedWarning` when aggressive category selected.

### F-003 (P2) — Record Cleanup Operations in State
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `with_state_manager()` builder at clean.rs L336–339.
`record_operation("cleanup", ...)` at L1104–1114 after execution.

### F-004 (P2) — Doctor Refresh on Navigation
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `navigate()` in mod.rs L430: `if matches!(view, View::Doctor) { self.refresh_current_view(); }`

### F-005 (P2) — Use iron_pacman::clean_cache()/get_orphans() in CleanupService
**Status**: ✅ **DONE** (Sprint H3)
**Evidence**: `PackageManager` trait now includes `get_orphans()` and `clean_cache(keep)`
with default implementations. `DefaultPackageManager` (iron-pacman) implements both.
`DefaultCleanupService` accepts `Arc<dyn PackageManager>` via `with_package_manager()`.
`preview_orphan_packages()`, `execute_package_cache()`, and `execute_orphan_packages()`
delegate to PackageManager when injected, falling back to raw Command otherwise.
Wired at TUI (preview_cleanup, execute_cleanup) and CLI (clean.rs) call sites.

### F-006 (P3) — BrokenSymlinks Category in CleanupService
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `CleanupCategory::BrokenSymlinks` variant in clean.rs with `preview_broken_symlinks()`,
`execute_broken_symlinks()`, `find_broken_symlinks()` recursive helper.
Added to `all()` (9) and `safe()` (7) lists. NOT aggressive.
CLI `--symlinks` flag mapped to `CleanupCategory::BrokenSymlinks`.

### F-007 (P3) — Snapshot Status as Pre-Flight Check
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `check_snapshot_status()` at update.rs L918 checks snapshot age.
Added to `run_preflight_checks()` at L1342.

### F-008 (P3) — Partial Update Detection Pre-Flight Check
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `check_partial_updates()` at update.rs L969 reads last 200 lines of
`/var/log/pacman.log`, searches for `pacman -Sy ` without `-Syu`.
Added to `run_preflight_checks()` at L1346.

### F-009 (P3) — ProfileBuilder Conflict Warnings During Module Selection
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: handlers.rs L819–848 in profile builder space-toggle: bidirectional
conflict checking with `conflicts_with()` for all selected modules.
Shows deduplicated warning via `set_error()` but still allows selection.

### F-010 (P3) — ModuleCreator Add ModuleKind Selection
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: `module_creator_kind_index` field in App struct (mod.rs).
handlers.rs L941–962: field 3 = kind, cycles through 6 variants
(AppConfig/Shell/DesktopComponent/Theme/SystemUtil/DevTools) via l/j/h/k/Left/Right.
actions.rs `create_module_from_creator()` maps index to kind string in TOML.

### F-011 (P3) — Show Guidance When Module List Is Empty
**Status**: ✅ **DONE** (Sprint H2)
**Evidence**: profile_builder.rs renders guidance when `app.modules.is_empty()`:
"Create modules first using [n] from the Modules view, or use `iron module create <name>`"
with instructions for Enter to continue or Esc to go back.

### F-012 (P3) — Detect and Integrate Timeshift/Snapper
**Status**: ✅ **ALREADY DONE** (pre-hardening)
**Evidence**: `snapshot.rs` `detect_backend()` + `create_manager()` auto-detects
timeshift/snapper. Used in all 10+ `create_manager()` call sites across TUI actions.

---

## 9. Requirements Cross-Reference

| FR | Requirement | Status | Gap |
|----|------------|--------|-----|
| FR-1.4 | Auto-detect current host | ✅ IMPLEMENTED (hostname detection) | — |
| FR-1.5 | Warning badge when no snapshot | ✅ IMPLEMENTED (doctor check) | — |
| FR-2.6 | Dormant config storage | ✅ FIXED (B-001 resolved) | — |
| FR-3.5 | Smart merge for overlapping symlinks | ⚠️ STUB | P3 backlog |
| FR-5.3 | Predict dependency conflicts | ✅ IMPLEMENTED | — |
| FR-5.6 | Auto snapshot before update | ✅ IMPLEMENTED | — |
| FR-5.7 | Detect/diff/merge .pacnew | ✅ IMPLEMENTED | — |
| FR-5.9 | 120s timeout on external commands | ✅ IMPLEMENTED (**A-001**) | — |
| FR-5.10 | Track update progress / resume | ✅ IMPLEMENTED | — |
| FR-6.3 | 4-step recovery flow | ✅ IMPLEMENTED (**C-009**) | — |
| FR-6.4 | Post-install verification | ✅ IMPLEMENTED (**C-010**) | — |
| FR-7.2 | Pull applies config changes | ✅ IMPLEMENTED (**D-008**) | — |
| FR-7.4 | Interactive merge on conflict | ⚠️ STUB | P3 backlog |
| FR-8.5 | `iron secrets unlock` | ✅ IMPLEMENTED | — |
| FR-8.6 | `iron secrets link` | ✅ IMPLEMENTED | — |
| FR-9.2 | First-run wizard | ✅ IMPLEMENTED (**B-002**) | — |
| FR-9.4 | Profile builder | ✅ IMPLEMENTED | — |
| FR-10.7 | Report git-crypt status | ✅ IMPLEMENTED | — |
| FR-10.8 | JSON health report | ✅ IMPLEMENTED | — |

**All FR violations resolved.**

---

## 10. Dependency Graph

Remaining tasks and their dependencies:

```
A-001 (SyncService → iron-git) ──→ A-009 (shared instance) ✅
                                ──→ A-010 (secrets lock check) ✅
                                ──→ D-009 (background push/pull) ✅

C-004 (CLI module create) ──→ D-012 (dotfile mapping in creator) ✅
  [DONE]                        [DONE]

F-005 (iron_pacman in clean) ─→ (dependency inversion) ✅

C-009 (full recovery flow) ──→ (packages + services) ✅
```

---

## 11. Sprint Results

### Sprint H1 — 28 tasks ✅

All 28 H1 tasks were completed: B-001, A-002, B-002, B-003, C-001, D-001, D-002,
F-001, E-001, E-002, E-003, A-003, A-005, A-008, B-004, B-005, C-002, C-003, C-004,
C-005, D-003, D-004, D-005, E-004, E-005, E-006, E-007.

**Note**: A-001 (SyncService → iron-git refactor) was not completed. The error mapping
sub-task (A-002) was done instead. A-001 remains the single largest piece of architecture
debt.

### Sprint H2 — 33 tasks (29 completed, 4 open)

Completed in H2: A-004, A-007, C-006, D-006, D-007, D-008, E-008, E-009, E-010,
F-002, F-003, F-004, B-006, C-007, C-008, C-010, D-010, D-011, D-013, E-011, E-012,
E-013, E-014, F-006, F-007, F-008, F-009, F-010, F-011.

Already done (confirmed): A-004 (Sprint 3), A-005 (Sprint 3), F-012 (pre-hardening), A-006 (pre-hardening).

### Sprint H3 — 7 tasks (7 completed)

Completed the final 7 tasks: A-001, A-009, A-010, C-009, D-009, D-012, F-005.

**All 65 hardening tasks are now complete.**

---

## 12. Completion Summary

All 65 hardening tasks are complete (100%). No remaining work.

### Test Coverage Summary (Final)

| Component | Tests | Status |
|-----------|------:|--------|
| iron-core | 905 | ✅ (+4 ignored git-crypt) |
| iron-tui | 445 | ✅ |
| iron-pacman | 101 | ✅ |
| iron-git | 95 | ✅ |
| iron-fs | 88 | ✅ |
| iron-systemd | 69 | ✅ |
| **Total** | **1,703** | **0 failed** |

---

## 13. Already Resolved (Excluded)

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
| S1-P5-NEW-001 | TUI profile activation broken | Sprint 2 |
| S1-P5-NEW-002 | TUI module enable/disable broken | Sprint 2 |
| S1-P7-NEW-002 | DoctorService missing from iron-core | Sprint 4 |
| S1-P7-NEW-003 | CLI clean not using CleanupService | Sprint 4 |
| S1-P7-NEW-004 | TUI Doctor [r] re-run broken | Sprint 4 |
| S1-P7-NEW-006 | CLI doctor not using DoctorService | Sprint 4 |
| S1-P7-NEW-007 | TUI doctor not using DoctorService | Sprint 4 |
| D-P8-001 | Push auto-commit | Sprint 4 |
| D-P8-002 | Pull dirty check | Sprint 4 |
| S1-P6-NEW-001 | TUI update → UpdateService.apply() | Sprint 3 |
| S1-P6-NEW-006 | TUI update path audit logging | Sprint 3 |
| A-006 | TUI update operation recording | Already resolved |
| F-012 | Timeshift/snapper detection | Pre-hardening |

---

## Appendix A: Codebase Metrics (Post-Hardening)

| Crate | Lines | Tests | Ignored | Role |
|-------|------:|------:|--------:|------|
| iron-core | 29,523 | 897 | 4 | Domain models, services, state |
| iron-tui | 17,129 | 445 | 0 | Ratatui TUI (27 views) |
| iron-cli | 7,948 | — | — | Clap CLI |
| iron-git | 2,317 | 95 | 0 | Git/git-crypt |
| iron-fs | 1,969 | 88 | 0 | File operations |
| iron-pacman | 2,574 | 101 | 0 | Pacman/paru/yay |
| iron-systemd | 1,889 | 69 | 0 | Systemd |
| **Total** | **63,349** | **1,695** | **4** | **0 failed, 0 clippy warnings** |

**Growth from hardening**: +2,609 LOC, +128 tests

## Appendix B: Effort Estimation Key

| Size | Typical Scope | Estimated Hours |
|------|--------------|-----------------|
| **S** (Small) | Single function/method change, simple test addition | 1–2h |
| **M** (Medium) | Cross-file refactor, new feature with tests | 3–5h |
| **L** (Large) | Multi-crate change, new async patterns, complex logic | 6–10h |
