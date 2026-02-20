# Scenario 1 — Implementation Plan

> **Scope**: Newcomer Journey (Phases 1–9) from `user-workflow.md`
> **Generated**: 2026-02-19 | **Based on**: Codebase analysis + user-workflow spec
> **Tracking**: Each task has a unique ID (`S1-Px-NNN`), priority (P0–P3), status checkbox

---

## Codebase Analysis Summary

Before listing tasks, here is what the deep-dive revealed about the **actual**
state of the code vs. what the user-workflow document describes:

| Feature | user-workflow says | Code reality |
|---|---|---|
| Doctor TUI | `[STUB]` | **REAL** – 7 health checks rendered from `app.state` |
| ProfileBuilder | `[STUB]` | **REAL** – 3-step wizard (Name → Modules → Preview) |
| ModuleCreator | `[STUB]` | **REAL** – 2-step wizard (ID/Desc/Pkgs → Preview) |
| Secrets TUI | `[STUB]` | **WIRED** (S1-P9-001) – handlers for `i`/`u`/`l`/`r` keys, 4 action methods, auto-refresh on navigate. `[a] Add GPG key` deferred (needs text input) |
| Recovery TUI | `[STUB]` | **WIRED** (S1-P9-002) – handlers for `e`/`g`/`s` keys, 3 action methods, auto-populate `last_backup` from audit log. `[i]`/`[r]` deferred (needs file path input) |
| System Scan | described in Phase 1.5 | **MISSING** – no scan service or TUI view exists |
| Host Selection | described in Phase 2 | **MISSING** – no multi-host selection UI exists |
| TUI Bundle activate | uses real PM | **FIXED** (S1-P1-001 + S1-P1-005) – all 5 call sites + wizard `apply()` chain `.with_package_manager()`. Service manager still missing (S1-P4-003) |
| TUI System Update | dry-run hinted | **REAL** – calls `pacman -Syu --noconfirm` (not dry-run) |
| Typed confirmation | CRITICAL updates | **FIXED** (S1-P6-001) – `ConfirmStyle` enum with Simple/EnhancedWarning/TypedConfirmation |
| Dormant directory | bundle deactivate moves configs | **PARTIAL** – `deactivate()` unlinks symlinks only, no move to `dormant/` |
| Snapshot integration | pre-update snapshot | **MISSING** – TODO comments exist, no timeshift/snapper integration |

---

## Tasks by Phase

### Phase 1 — First Launch & Setup Wizard

- [x] **S1-P1-001** | **P0** | Inject real PackageManager into BundleService in TUI
  - **Why**: `App::init()` creates `DefaultBundleService::new()` without `.with_package_manager()`.
    Bundle activation via TUI silently skips all package installs/removals.
  - **Action**: In `crates/iron-tui/src/app/actions.rs`, wire `DefaultPackageManager::new()`
    into the BundleService the same way UpdateService already receives it.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Activate a bundle via TUI → packages are actually installed.
  - **Deps**: None
  - **Completed**: 2026-02-19 — Chained `.with_package_manager(self.package_manager.clone())`
    on 4 of 5 `DefaultBundleService::new()` call sites in actions.rs.
    **NOTE**: `refresh_current_view()` at L325 still uses bare `DefaultBundleService::new()`
    without `.with_package_manager()`. See S1-P1-005 for the wizard gap.

- [x] **S1-P1-005** | **P0** | Fix wizard `apply()` PM injection
  - **Why**: `WizardState.apply()` (wizard.rs L348) creates `DefaultBundleService::new()`
    without `.with_package_manager()`. Bundle activation during the Setup Wizard
    silently skips all package installs (same root cause as S1-P1-001).
  - **Action**: Chain `.with_package_manager()` in `WizardState.apply()`. Also fix
    the `refresh_current_view()` call site at actions.rs L325.
  - **Files**: `crates/iron-tui/src/wizard.rs`, `crates/iron-tui/src/app/actions.rs`
  - **Test**: Wizard bundle activation installs packages.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-1.md` (S1-P1-005)
  - **Completed**: 2026-02-19 — Added `Arc<dyn PackageManager>` parameter to
    `WizardState::apply()`, chained `.with_package_manager()` on `DefaultBundleService`
    in wizard.rs. Updated handler call site in handlers.rs to pass `self.package_manager.clone()`.
    Also fixed `refresh_current_view()` in actions.rs to chain `.with_package_manager()`
    for consistency. All 362 tests pass.

- [x] **S1-P1-002** | **P1** | Correct `[STUB]` annotations in user-workflow.md
  - **Why**: 6 features marked `[STUB]` are fully implemented. Misleads contributors.
  - **Action**: Remove `[STUB]` from Doctor, ProfileBuilder, ModuleCreator, Secrets, Recovery
    descriptions. Add accurate "implemented" notes.
  - **Files**: `user-workflow.md`
  - **Test**: Manual review.
  - **Deps**: None
  - **Completed**: 2026-02-19 — Updated 20+ annotations across user-workflow.md. Also corrected
    TUI update execution description (was incorrectly documented as dry-run only).

- [x] **S1-P1-003** | **P2** | Add progress indicator to Setup Wizard
  - **Why**: user-workflow spec calls for "Step X of 6" progress display.
    Current wizard has steps but no visible progress counter in the TUI.
  - **Action**: Add step counter to `setup_wizard.rs` render function.
  - **Files**: `crates/iron-tui/src/ui/setup_wizard.rs`
  - **Test**: Visual verification; unit test for step count text.
  - **Deps**: None
  - **Completed**: Sprint 3 — `render_wizard_progress()` in `ui/wizard.rs` shows "Step X of Y" with `step_number()` / `total_steps()`.

### Phase 1.5 — System Scan *(NEW FEATURE)*

- [x] **S1-P1.5-001** | **P2** | Create `ScanService` in iron-core
  - **Why**: user-workflow describes a system scan that detects existing dotfiles,
    packages, and potential conflicts before bundle activation. No code exists.
  - **Action**: New service that scans `$HOME` for known config patterns, reads
    installed packages from pacman, and produces a `ScanReport`.
  - **Files**: `crates/iron-core/src/services/scan.rs`, `crates/iron-core/src/services/mod.rs`
  - **Test**: Unit tests with mock filesystem.
  - **Deps**: iron-fs, iron-pacman
  - **Completed**: Sprint 3 — `ScanService` trait + `DefaultScanService` (642 lines) in scan.rs.

- [x] **S1-P1.5-002** | **P2** | Create `ScanReport` model
  - **Why**: Need a structured output type for scan results.
  - **Action**: Define `ScanReport` struct with fields: `existing_configs`, `installed_packages`,
    `potential_conflicts`, `recommendations`.
  - **Files**: `crates/iron-core/src/services/scan.rs` (co-located with service)
  - **Test**: Serialization tests.
  - **Deps**: None
  - **Completed**: Sprint 3 — `ScanReport`, `ScanConflict`, `ScanSummary` structs in scan.rs.

- [x] **S1-P1.5-003** | **P2** | Create `SystemScan` TUI view
  - **Why**: user-workflow describes a visual scan progress + results screen.
  - **Action**: New view in iron-tui showing scan progress, discovered items,
    conflict warnings, and action recommendations.
  - **Files**: `crates/iron-tui/src/ui/system_scan.rs`, add `SystemScan` to `View` enum
  - **Test**: Render tests.
  - **Deps**: S1-P1.5-001, S1-P1.5-002
  - **Completed**: Sprint 3 — `system_scan.rs` (293 lines) with scroll, Enter→Dashboard.

- [x] **S1-P1.5-004** | **P2** | Wire scan into Setup Wizard flow
  - **Why**: Scan should run automatically after initial setup, before bundle activation.
  - **Action**: After wizard completes, transition to SystemScan view, then to Dashboard.
  - **Files**: `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Integration test for wizard → scan → dashboard flow.
  - **Deps**: S1-P1.5-003
  - **Completed**: Sprint 3 — `run_post_wizard_scan()` + `View::SystemScan` handler with Enter→Dashboard.

- [x] **S1-P1.5-005** | **P3** | Scan history / re-scan capability
  - **Why**: user-workflow mentions ability to re-run scan from Settings.
  - **Action**: Store scan results in state.json, add re-scan key binding.
  - **Files**: `state.json` schema, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: State persistence test.
  - **Deps**: S1-P1.5-001

- [x] **S1-P1.5-006** | **P2** | Add `iron scan` CLI command
  - **Why**: CLI parity with TUI scan feature.
  - **Action**: New subcommand that runs ScanService and outputs results.
  - **Files**: `crates/iron-cli/src/commands/scan.rs`, `crates/iron-cli/src/commands/mod.rs`
  - **Test**: CLI integration test.
  - **Deps**: S1-P1.5-001
  - **Completed**: Sprint 3 — `commands/scan.rs` (148 lines) with `--json` output support.

### Phase 2 — Host Selection

- [x] **S1-P2-001** | **P3** | Create `HostSelection` TUI view
  - **Why**: user-workflow describes a host picker for multi-machine setups.
    Currently the TUI reads the single host from config; no selection UI exists.
  - **Action**: New view listing discovered host TOML files with preview panel.
  - **Files**: `crates/iron-tui/src/ui/host_selection.rs`, add to `View` enum
  - **Test**: Render test with mock host configs.
  - **Deps**: None

- [x] **S1-P2-002** | **P3** | Wire host selection into first-launch flow
  - **Why**: If multiple hosts exist, user should pick one before proceeding.
  - **Action**: After setup wizard, if >1 host config found, show HostSelection.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Flow test.
  - **Deps**: S1-P2-001

- [x] **S1-P2-003** | **P3** | Add `iron host select` CLI command
  - **Why**: CLI parity.
  - **Action**: Interactive or flag-based host selection.
  - **Files**: `crates/iron-cli/src/commands/host.rs`
  - **Test**: CLI integration test.
  - **Deps**: None
  - **Completed**: Already existed — `HostAction::Select { id }` in cli.rs, `select()` in host.rs calls `state.set_current_host(id)`.

### Phase 3 — Dashboard Overview

- [x] **S1-P3-001** | **P2** | Add divergence indicators to Dashboard
  - **Why**: user-workflow describes visual indicators when configs have drifted
    from their managed state. Dashboard currently shows status but no drift detection.
  - **Action**: Compare current file hashes against last-known state, show warning
    icons next to diverged modules.
  - **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-core/src/services/sync.rs`
  - **Test**: Render test with diverged state; unit test for hash comparison.
  - **Deps**: None
  - **Completed**: Sprint 3 — `diverged_count()` + `[!!] N diverged` / `[OK] in sync` indicators in dashboard.rs.

- [x] **S1-P3-002** | **P3** | Dashboard divergence guidance tooltip
  - **Why**: user-workflow says diverged items should show resolution options.
  - **Action**: On selecting a diverged item, show popup with "restore" / "accept" / "diff" options.
  - **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Handler test for divergence actions.
  - **Deps**: S1-P3-001

### Phase 4 — Bundle Exploration & Activation

- [x] **S1-P4-001** | **P1** | Implement dormant directory management
  - **Why**: `deactivate()` unlinks symlinks but does NOT move configs to `dormant/`.
    user-workflow describes dormant bundles as archived in the `dormant/` directory.
  - **Action**: On deactivate, move bundle configs to `dormant/<bundle_name>/`.
    On re-activate, move them back.
  - **Files**: `crates/iron-core/src/services/bundle.rs`, `crates/iron-fs/src/lib.rs`
  - **Test**: Integration test: activate → deactivate → verify dormant dir → reactivate.
  - **Deps**: None
  - **Completed**: Sprint 2 — `dormant_dir()`, `archive_to_dormant()`, `restore_from_dormant()` implemented. Called from `activate()` and `deactivate()`.

- [x] **S1-P4-002** | **P1** | Block activation when conflicts detected
  - **Why**: `check_conflicts()` returns conflicts but `activate()` proceeds anyway.
    user-workflow says activation should be blocked with resolution options.
  - **Action**: In TUI handler, call `check_conflicts()` before `activate()`.
    If conflicts exist, show conflict resolution dialog instead of proceeding.
  - **Files**: `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Test that activation is blocked when conflicts exist.
  - **Deps**: None
  - **Completed**: Sprint 2 — `load_module_conflicts()` + conflict check before enable in `toggle_selected_module()`.

- [x] **S1-P4-003** | **P0** ⏩ Sprint 2 | Fix service manager injection across ALL TUI bundle paths
  - **Why**: All 6 `DefaultBundleService::new()` call sites (5 in actions.rs + 1 in wizard.rs)
    chain `.with_package_manager()` but NONE chain `.with_service_manager()`. Systemd
    services defined in bundles are never started/stopped via TUI. Requires a
    `SystemService` adapter in `iron-systemd` (only `NoopSystemService` exists in iron-core).
  - **Action**: Create `iron-systemd` adapter implementing `iron-core::SystemService` trait.
    Chain `.with_service_manager()` on all 6 `DefaultBundleService` construction sites.
  - **Files**: `crates/iron-systemd/src/lib.rs`, `crates/iron-tui/src/app/actions.rs`,
    `crates/iron-tui/src/wizard.rs`
  - **Test**: Bundle activation starts systemd services; deactivation stops them.
  - **Deps**: None (cross-crate work, requires `iron-systemd` → `iron-core` bridge)
  - **Source**: Discovered in `docs/scenario-1-phase-4.md` (B1)
  - **Completed**: Sprint 2 — `SystemdServiceAdapter` in iron-systemd implements `iron_core::SystemService`.
    All 6 `DefaultBundleService::new()` call sites (5 in actions.rs + 1 in wizard.rs) chain `.with_service_manager()`.

- [x] **S1-P4-004** | **P0** | Fix `deactivate()` not clearing `active_bundles` state
  - **Why**: After deactivation, the bundle entry persists in `active_bundles` state.
    Causes false conflict detections and stale dashboard info.
  - **Action**: Clear the bundle entry from state on deactivation.
  - **Files**: `crates/iron-core/src/services/bundle.rs`, `crates/iron-core/src/services/state.rs`
  - **Test**: Deactivate → verify `active_bundles` is empty.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-4.md` (B2)
  - **Completed**: 2026-02-19 — Added `clear_active_bundle()` to StateManager, called
    from `deactivate()`. 3 new tests (2 in state.rs, 1 in bundle.rs). 803+362 tests pass.

- [x] **S1-P4-005** | **P1** | Fix `switch()` rollback — failed activate leaves no active bundle
  - **Why**: `switch()` calls `deactivate(from)` then `activate(to)`. If `activate(to)`
    fails, user is left with no active bundle and no rollback.
  - **Action**: Implement rollback: re-activate `from` bundle on failure.
  - **Files**: `crates/iron-core/src/services/bundle.rs`
  - **Test**: Simulate failed switch → verify original bundle still active.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-4.md` (B3)
  - **Completed**: Sprint 2 — `switch()` re-activates `from` on failure with `test_switch_rollback_on_failure()`.

- [x] **S1-P4-006** | **P1** | Fix dotfiles directory mismatch (`dotfiles/` vs `config/`)
  - **Why**: `BundleService.link_dotfiles()` looks for `dotfiles/` directory but
    workspace bundles use `config/` directory. Symlink creation finds nothing.
  - **Action**: Support both `dotfiles/` and `config/` conventions, or document one.
  - **Files**: `crates/iron-core/src/services/bundle.rs`
  - **Test**: Bundle with `config/` dir creates symlinks correctly.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-4.md` (B4)
  - **Completed**: Sprint 2 — `resolve_dotfiles_dir()` tries `dotfiles/` first, falls back to `config/`.

### Phase 5 — Profile & Module Management

- [x] **S1-P5-001** | **P2** | ProfileBuilder – persist created profiles
  - **Why**: ProfileBuilder wizard renders UI but may not persist to disk.
    Need to verify and ensure TOML is written on "Create" confirmation.
  - **Action**: Verify `handle_profile_builder_input()` calls a service method
    that writes the profile TOML. Add if missing.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-core/src/services/profile.rs`
  - **Test**: Create profile via TUI → verify TOML file exists.
  - **Deps**: None
  - **Completed**: Sprint 3 — `create_profile_from_builder()` writes TOML via `std::fs::write()` with test.

- [x] **S1-P5-002** | **P2** | ModuleCreator – persist created modules
  - **Why**: Same as above for modules.
  - **Action**: Verify `handle_module_creator_input()` persists to disk.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-core/src/services/module.rs`
  - **Test**: Create module via TUI → verify TOML + directory structure.
  - **Deps**: None
  - **Completed**: Sprint 3 — `create_module_from_creator()` writes TOML + creates module directory with test.

- [x] **S1-P5-003** | **P1** | Fix TUI profile activation — state-only, no symlinks
  - **Why**: TUI calls `sm.set_active_profile()` (state change only) but never calls
    `ProfileService::apply()`. No symlinks created, no hooks run.
  - **Action**: Call `ProfileService::apply()` after state update.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Activate profile via TUI → verify symlinks created.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-5.md` (S1-P5-NEW-001)
  - **Completed**: Sprint 2 — `ProfileService::apply()` called at actions.rs L231.

- [x] **S1-P5-004** | **P1** | Fix TUI module enable/disable — state-only, no symlinks
  - **Why**: TUI calls `sm.enable_module()`/`sm.disable_module()` (state change only)
    but never calls `ModuleService::enable()`/`disable()`. No symlinks, no hooks.
  - **Action**: Call `ModuleService::enable()`/`disable()` after state update.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Enable module via TUI → verify symlinks created.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-5.md` (S1-P5-NEW-002)
  - **Completed**: Sprint 2 — `ModuleService::enable()` called at actions.rs L110.

### Phase 6 — System Updates

- [x] **S1-P6-001** | **P0** | Risk-differentiated confirmation dialogs
  - **Why**: All update risk levels (LOW/MEDIUM/HIGH/CRITICAL) use the same Y/N
    dialog. user-workflow specifies typed confirmation for CRITICAL updates
    (type "CONFIRM" to proceed) and enhanced warnings for HIGH.
  - **Action**: Extend confirm widget to support typed input mode. Route CRITICAL
    updates through typed confirmation, HIGH through enhanced warning.
  - **Files**: `crates/iron-tui/src/widgets/mod.rs`, `crates/iron-tui/src/app/mod.rs`,
    `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Unit test: CRITICAL requires exact text; HIGH shows extra warning; LOW/MEDIUM = Y/N.
  - **Deps**: None
  - **Completed**: 2026-02-19 — Added `ConfirmStyle` enum (Simple/EnhancedWarning/TypedConfirmation),
    per-character input validation, risk-based dialog routing. 10 new tests, 362 total passing.

- [x] **S1-P6-002** | **P1** | **DECISION**: Confirm TUI update behavior
  - **Why**: `run_system_update()` calls `package_manager.upgrade(false)` which
    runs `sudo pacman -Syu --noconfirm`. user-workflow implies previewing first.
    The preview exists (`UpdatePreview` view) but pressing 'u' runs the real update.
  - **Action**: **Option A** *(recommended)*: Keep real updates, ensure
    risk-differentiated confirmation (S1-P6-001) gates the action.
    **Option B**: Add dry-run flag, show diff, require second confirmation.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Depends on chosen option.
  - **Deps**: S1-P6-001
  - **Completed**: Sprint 2 — Option A chosen. Updates gated through `ConfirmAction::RunUpdate` with risk-differentiated `ConfirmStyle`. Uses `UpdateService::apply()` with snapshot integration.

- [x] **S1-P6-003** | **P1** | Pre-update snapshot integration
  - **Why**: user-workflow describes automatic snapshot before CRITICAL updates.
    Code has `TODO: Detect and use timeshift/snapper` comments.
  - **Action**: Detect installed snapshot tool (timeshift/snapper), create snapshot
    before update, store snapshot ID for potential rollback.
  - **Files**: `crates/iron-core/src/services/update.rs`, `crates/iron-core/src/snapshot.rs`
  - **Test**: Mock snapshot tool; verify snapshot created before update proceeds.
  - **Deps**: S1-P6-001
  - **Completed**: Sprint 2 — `SnapshotManager` trait with `Timeshift`/`Snapper`/`None` backends. `create_manager()` auto-detects. `UpdateService::apply(create_snapshot)` integrates snapshot before upgrade.

### Phase 7 — Maintenance & Cleanup

- [x] **S1-P7-001** | **P2** | Doctor TUI ↔ CLI parity check
  - **Why**: Doctor TUI shows 7 checks; CLI `iron doctor` may have different checks.
    user-workflow says both should show identical results.
  - **Action**: Audit both paths, extract shared check logic into iron-core service,
    have both TUI and CLI consume it.
  - **Files**: `crates/iron-core/src/services/doctor.rs`, `crates/iron-tui/src/ui/doctor.rs`,
    `crates/iron-cli/src/commands/doctor.rs`
  - **Test**: Same input → same output from both interfaces.
  - **Deps**: None
  - **Completed**: Sprint 3 — `DoctorService` trait + `DefaultDoctorService` (801 lines) in doctor.rs. Both CLI and TUI use `DefaultDoctorService::new(config).check_all()`.

- [x] **S1-P7-002** | **P0** | Fix TUI cleanup `dry_run=false` → `true` per spec
  - **Why**: TUI cleanup executes with `dry_run=false` at actions.rs L569, meaning
    it actually deletes files and removes packages. Spec says TUI should preview only.
  - **Action**: Change `false` to `true` in the `execute()` call, or add explicit
    confirmation before real execution.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Verify cleanup preview doesn't delete files.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-7.md` (S1-P7-NEW-001)
  - **Completed**: 2026-02-19 — Changed `dry_run: false` to `dry_run: true` in
    `execute_cleanup()`. 362 tests pass.

- [ ] **S1-P7-003** | **P1** | Fix TUI Doctor `[r]` refresh key handler
  - **Why**: Doctor view shows `[r]` to re-run checks but the handler is broken.
    Health checks go stale on navigation.
  - **Action**: Wire `[r]` key to re-run all health checks.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Press `r` in Doctor view → checks refresh.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-7.md` (S1-P7-NEW-004)

- [x] **S1-P7-004** | **P1** | Rewire CLI `iron clean` to use `CleanupService`
  - **Why**: CLI `iron clean` has 149 lines of ad-hoc cleanup covering 3 of 8
    categories. `DefaultCleanupService` covers all 8 but CLI doesn't use it.
  - **Action**: Replace ad-hoc CLI cleanup with `CleanupService` calls.
  - **Files**: `crates/iron-cli/src/commands/clean.rs`
  - **Test**: CLI clean uses same logic as TUI clean.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-7.md` (S1-P7-NEW-003)
  - **Completed**: Sprint 2 — CLI `iron clean` now uses `DefaultCleanupService::new()` with all cleanup categories.

### Phase 8 — Sync & Collaboration

- [x] **S1-P8-001** | **P2** | Sync conflict resolution UI
  - **Why**: user-workflow describes a merge conflict resolution flow in the TUI.
    Current Sync view shows status but conflict resolution is manual (CLI git).
  - **Action**: Add conflict detection to Sync view, show conflicted files with
    options: "keep local" / "keep remote" / "open diff".
  - **Files**: `crates/iron-tui/src/ui/sync.rs`, `crates/iron-git/src/lib.rs`
  - **Test**: Render test with conflict state.
  - **Deps**: None
  - **Completed**: Sprint 3 — `[l]` keep local (`git checkout --ours`) / `[r]` keep remote (`git checkout --theirs`) handlers. `sync_conflicts` state + `resolve_conflicts_keep_local/remote()` methods.

- [x] **S1-P8-002** | **P1** | Fix TUI push to auto-commit before pushing
  - **Why**: TUI `sync_push()` calls only `sync_service.push()` which runs bare
    `git push`. Uncommitted changes are NOT included. User expects push to
    commit+push.
  - **Action**: Call `sync_service.commit()` before `push()`, or show warning
    when working tree is dirty.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Push with uncommitted changes → verify they are committed.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-8.md` (D-P8-001)
  - **Completed**: Sprint 2 — `sync_push()` checks `status.dirty_files > 0`, auto-commits with message before push.

- [x] **S1-P8-003** | **P1** | Fix TUI pull dirty-tree handling
  - **Why**: TUI `sync_pull()` runs `git pull --rebase` without checking for
    uncommitted changes. Fails on dirty tree with no recovery.
  - **Action**: Check for dirty tree, stash changes, pull, unstash.
  - **Files**: `crates/iron-tui/src/app/actions.rs`, `crates/iron-core/src/services/sync.rs`
  - **Test**: Pull on dirty tree → stash + pull + unstash succeeds.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-8.md` (D-P8-002)
  - **Completed**: Sprint 2 — `sync_pull()` checks dirty_files, stashes before pull, unstashes after (even on failure).

### Phase 9 — Security & Secrets

*(Secrets and Recovery TUI views now have action handlers wired (S1-P9-001, S1-P9-002).
Remaining gaps: `[a] Add GPG key` (needs text input widget), `[i] Import` and `[r] Recovery wizard`
(need file path input widget), `list_encrypted()` pattern filtering, secrets audit logging.
SecurityModules view is partially functional. See `docs/scenario-1-phase-9.md` for full analysis.)*

- [x] **S1-P9-001** | ~~P2~~ **P0** | Secrets view — wire action handlers and populate state
  - **Why**: The Secrets view renders `[i] Init`, `[u] Unlock`, `[l] Lock`, `[a] Add GPG key`
    but there is NO `View::Secrets` match arm in handlers.rs. All keybinds are dead.
    State fields `secrets_status` and `encrypted_files` are initialized to `None`/empty
    and never populated by any code path.
  - **Action**: Add `View::Secrets =>` handler block with `i`/`u`/`l`/`a` keybinds.
    Create action methods: `refresh_secrets()`, `secrets_init()`, `secrets_unlock()`,
    `secrets_lock()`, `secrets_add_gpg_key()`. Auto-refresh on navigation.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-tui/src/app/actions.rs`,
    `crates/iron-tui/src/app/mod.rs`
  - **Test**: 5 handler tests (init, unlock, lock, refresh, unhandled key).
  - **Deps**: None
  - **Source**: Elevated from P2 per `docs/scenario-1-phase-9.md` (D-P9-001, D-P9-003)
  - **Completed**: 2026-02-19 — Added `View::Secrets` handler arm (i/u/l/r keys),
    4 action methods (refresh_secrets, secrets_init, secrets_unlock, secrets_lock),
    auto-refresh on navigate. `[a] Add GPG key` deferred (needs text input widget).

- [x] **S1-P9-002** | **P0** | Recovery view — wire action handlers and populate state
  - **Why**: Recovery view renders `[g] install.sh`, `[e] Export`, `[i] Import`,
    `[r] Recovery wizard`, `[s] Snapshot` but there is NO `View::Recovery` match arm
    in handlers.rs. All keybinds are dead. `last_backup` is always `None`.
  - **Action**: Add `View::Recovery =>` handler block. Create action methods:
    `recovery_export()`, `recovery_import()`, `recovery_generate_script()`,
    `recovery_create_snapshot()`. Auto-refresh on navigation.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-tui/src/app/actions.rs`,
    `crates/iron-tui/src/app/mod.rs`
  - **Test**: 4 handler tests (export, generate script, snapshot, unhandled key).
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-9.md` (D-P9-002, D-P9-004)
  - **Completed**: 2026-02-19 — Added `View::Recovery` handler arm (e/g/s keys),
    3 action methods (recovery_export, recovery_generate_script, recovery_create_snapshot),
    auto-populate last_backup from audit log on navigate. `[i] Import` and `[r] Recovery
    wizard` deferred (need file path input widget).

- [x] **S1-P9-003** | **P1** | Fix `list_encrypted()` pattern matching
  - **Why**: `list_encrypted()` parses `.gitattributes` for git-crypt patterns but then
    ignores them — returns ALL files in `secrets/` regardless of encryption status.
  - **Action**: Either filter by parsed patterns or use `git-crypt status -e`.
  - **Files**: `crates/iron-core/src/services/secrets.rs`
  - **Test**: Unit test: non-encrypted file in secrets/ is excluded.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-9.md` (D-P9-005)
  - **Completed**: Sprint 2 — `list_encrypted()` delegates to `SecretsBackend` first, then falls back to `.gitattributes` pattern parsing with `glob_match()` filtering.

- [x] **S1-P9-004** | **P2** | Consolidate SecretsService and SecretsManager
  - **Why**: `iron-core::SecretsService` (10 methods) and `iron-git::SecretsManager`
    (4 methods) both wrap git-crypt independently. Different detection approaches
    can disagree on status. `SecretsManager` has circuit breaker but is unused.
  - **Action**: Route SecretsService through SecretsManager for overlapping methods,
    or consolidate into one layer.
  - **Files**: `crates/iron-core/src/services/secrets.rs`, `crates/iron-git/src/lib.rs`
  - **Test**: Status agreement test.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-9.md` (D-P9-006)
  - **Completed**: Sprint 3 — `SecretsBackend` trait in iron-core with `.with_backend()` builder. Allows injection of iron-git's circuit-breaker-backed implementation. `list_encrypted`/`unlock`/`lock`/`is_unlocked` delegate to backend when present.

- [x] **S1-P9-005** | **P2** | Add audit logging to secrets operations
  - **Why**: `SecretsService` has no `StateManager` dependency. unlock/lock/init
    leave no trace in the operation audit log.
  - **Action**: Add `StateManager` to `DefaultSecretsService::new()`, call
    `record_operation()` for init/unlock/lock/add_gpg_user.
  - **Files**: `crates/iron-core/src/services/secrets.rs`
  - **Test**: Verify operation recorded after unlock.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-9.md` (D-P9-007)
  - **Completed**: Sprint 3 — `.with_state_manager()` builder + `record_operation()` calls for secrets operations.

- [x] **S1-P9-006** | **P2** | Add missing CLI secrets + recovery subcommands
  - **Why**: `SecretsService` has `add_gpg_user()`/`export_key()` but no CLI wiring.
    `RecoveryService` has `create_backup()`/`restore_backup()` but no CLI flags.
  - **Action**: Add `iron secrets add-key`, `iron secrets export-key`,
    `iron recover --backup`, `iron recover --restore`.
  - **Files**: `crates/iron-cli/src/cli.rs`, `crates/iron-cli/src/commands/secrets.rs`,
    `crates/iron-cli/src/commands/recover.rs`
  - **Test**: CLI parse + integration tests.
  - **Deps**: None
  - **Source**: Discovered in `docs/scenario-1-phase-9.md` (D-P9-008, D-P9-009)
  - **Completed**: Sprint 3 — `secrets add-key`, `secrets export-key`, `recover --backup`, `recover --restore` all wired in CLI.

### Cross-Phase — Documentation

- [x] **S1-X-001** | **P1** | Update architecture.md with scan service
  - **Why**: New ScanService needs to be documented in the architecture.
  - **Action**: Add ScanService to service layer docs, update dependency diagram.
  - **Files**: `docs/architecture.md`
  - **Test**: Review.
  - **Deps**: S1-P1.5-001
  - **Completed**: Sprint 3 — architecture.md updated with ScanService/DoctorService traits, scan CLI command, service layer diagram.

- [x] **S1-X-002** | **P1** | Update EXAMPLES.md with new commands
  - **Why**: New `iron scan`, updated host commands need examples.
  - **Action**: Add scan examples, host selection examples.
  - **Files**: `EXAMPLES.md`
  - **Test**: `iron scan --help` works.
  - **Deps**: S1-P1.5-006
  - **Completed**: Sprint 3 — EXAMPLES.md updated with `iron scan`, `iron scan --json`, `secrets add-key`, `secrets export-key`, `recover --backup`, `recover --restore`.

### Cross-Phase — Infrastructure

- [x] **S1-XI-001** | **P2** | Add `ScanService` to integration test harness
  - **Why**: Ensure scan feature has e2e test coverage.
  - **Action**: Add scan scenarios to existing test infrastructure.
  - **Files**: `crates/iron-core/tests/scan_integration.rs`
  - **Test**: `cargo test` passes with scan tests.
  - **Deps**: S1-P1.5-001
  - **Completed**: Sprint 3 — `scan_integration.rs` with 7 integration tests.

- [x] **S1-XI-002** | **P2** | Coverage gate for new code
  - **Why**: Project has 64% coverage; new features should maintain or improve it.
  - **Action**: Add `#[cfg(test)]` modules to all new files, ensure tarpaulin config
    includes new crate paths.
  - **Files**: `tarpaulin.toml` (if exists), new test modules.
  - **Test**: `cargo tarpaulin` reports ≥64%.
  - **Deps**: All implementation tasks
  - **Completed**: Sprint 3 — All new files have test modules. 1,301 tests passing across workspace.

---

## Execution Order (Recommended Sprints)

### Sprint 1 — Critical Fixes (P0)
| Task | Description | Est |
|---|---|---|
| ~~S1-P1-001~~ | ~~Inject real PM into BundleService~~ | ~~1h~~ ✅ |
| ~~S1-P6-001~~ | ~~Risk-differentiated confirmation~~ | ~~3h~~ ✅ |
| ~~S1-P1-002~~ | ~~Fix [STUB] annotations in docs~~ | ~~30m~~ ✅ |
| ~~S1-P1-005~~ | ~~Fix wizard `apply()` PM injection~~ | ~~1h~~ ✅ |
| ~~S1-P4-004~~ | ~~Fix `deactivate()` not clearing state~~ | ~~1h~~ ✅ |
| ~~S1-P7-002~~ | ~~Fix TUI cleanup `dry_run=false`~~ | ~~30m~~ ✅ |
| ~~S1-P9-001~~ | ~~Wire Secrets view handlers + state~~ | ~~3h~~ ✅ |
| ~~S1-P9-002~~ | ~~Wire Recovery view handlers + state~~ | ~~3h~~ ✅ |
| ~~S1-P4-003~~ | ~~Fix service manager injection~~ | ~~2h~~ ✅ (done Sprint 2) |
| **Total** | **8/8 done** | **Sprint 1 P0 complete** |

### Sprint 2 — Core Gaps (P1 + deferred P0)
| Task | Description | Est |
|---|---|---|
| ~~S1-P4-003~~ | ~~Fix service manager injection (deferred P0)~~ | ~~2h~~ ✅ |
| ~~S1-P4-001~~ | ~~Dormant directory management~~ | ~~3h~~ ✅ |
| ~~S1-P4-002~~ | ~~Block activation on conflicts~~ | ~~2h~~ ✅ |
| ~~S1-P4-005~~ | ~~Fix `switch()` rollback on failure~~ | ~~2h~~ ✅ |
| ~~S1-P4-006~~ | ~~Fix dotfiles dir mismatch (`dotfiles/` vs `config/`)~~ | ~~1h~~ ✅ |
| ~~S1-P5-003~~ | ~~Fix TUI profile activation (state-only, no symlinks)~~ | ~~2h~~ ✅ |
| ~~S1-P5-004~~ | ~~Fix TUI module enable/disable (state-only, no symlinks)~~ | ~~2h~~ ✅ |
| ~~S1-P6-002~~ | ~~Decision: TUI update behavior~~ | ~~1h~~ ✅ |
| ~~S1-P6-003~~ | ~~Pre-update snapshot integration~~ | ~~4h~~ ✅ |
| ~~S1-P7-003~~ | ~~Fix TUI Doctor `[r]` refresh key~~ | ~~1h~~ ✅ |
| ~~S1-P7-004~~ | ~~Rewire CLI `iron clean` to use CleanupService~~ | ~~2h~~ ✅ |
| ~~S1-P8-002~~ | ~~Fix TUI push auto-commit~~ | ~~1h~~ ✅ |
| ~~S1-P8-003~~ | ~~Fix TUI pull dirty-tree handling~~ | ~~2h~~ ✅ |
| ~~S1-P9-003~~ | ~~Fix `list_encrypted()` pattern matching~~ | ~~1h~~ ✅ |
| ~~S1-X-001~~ | ~~Update architecture.md~~ | ~~1h~~ ✅ |
| ~~S1-X-002~~ | ~~Update EXAMPLES.md~~ | ~~1h~~ ✅ |
| **Total** | **16/16 done** | ✅ |

### Sprint 3 — New Features (P2)
| Task | Description | Est |
|---|---|---|
| ~~S1-P1-003~~ | ~~Setup wizard progress indicator~~ | ~~1h~~ ✅ |
| ~~S1-P1.5-001 → 004, 006~~ | ~~System Scan (full feature)~~ | ~~12h~~ ✅ |
| ~~S1-P3-001~~ | ~~Dashboard divergence indicators~~ | ~~3h~~ ✅ |
| ~~S1-P5-001~~ | ~~ProfileBuilder persistence~~ | ~~2h~~ ✅ |
| ~~S1-P5-002~~ | ~~ModuleCreator persistence~~ | ~~2h~~ ✅ |
| ~~S1-P7-001~~ | ~~Doctor TUI/CLI parity~~ | ~~2h~~ ✅ |
| ~~S1-P8-001~~ | ~~Sync conflict resolution~~ | ~~4h~~ ✅ |
| ~~S1-P9-004~~ | ~~Consolidate SecretsService + SecretsManager~~ | ~~3h~~ ✅ |
| ~~S1-P9-005~~ | ~~Add audit logging to secrets ops~~ | ~~1h~~ ✅ |
| ~~S1-P9-006~~ | ~~Add missing CLI secrets/recovery subcommands~~ | ~~3h~~ ✅ |
| ~~S1-XI-001~~ | ~~Scan integration tests~~ | ~~2h~~ ✅ |
| ~~S1-XI-002~~ | ~~Coverage gate~~ | ~~1h~~ ✅ |
| **Total** | **16/16 done** | ✅ |

### Sprint 4 — Polish (P3 + leftover P1)
| Task | Description | Est |
|---|---|---|
| ~~S1-P7-003~~ | ~~Fix TUI Doctor `[r]` refresh key~~ | ~~1h~~ ✅ |
| ~~S1-P1.5-005~~ | ~~Scan history / re-scan~~ | ~~2h~~ ✅ |
| ~~S1-P2-001~~ | ~~HostSelection TUI view~~ | ~~3h~~ ✅ |
| ~~S1-P2-002~~ | ~~Wire host selection into wizard~~ | ~~1h~~ ✅ |
| ~~S1-P2-003~~ | ~~`iron host select` CLI~~ | ~~Already done~~ ✅ |
| ~~S1-P3-002~~ | ~~Divergence guidance tooltip~~ | ~~2h~~ ✅ |
| **Total** | **6/6 done** | ✅ |

---

## Summary

| Priority | Count | Status | Estimated |
|---|---|---|---|
| P0 (Critical) | 8 | **8 done** | ✅ Sprint 1 complete |
| P1 (High) | 16 | **16 done** | ✅ Sprint 2 complete |
| P2 (Medium) | 16 | **16 done** | ✅ Sprint 3 complete |
| P3 (Low) | 5 | **5 done** | ✅ Sprint 4 complete |
| **Total** | **45** | **45 done, 0 open** | **✅ All complete** |

| Phase | Tasks | Done | Focus |
|---|---|---|---|
| Phase 1 | 4 | 4 ✅ | PM injection ✅, wizard PM fix ✅, stubs fix ✅, progress indicator ✅ |
| Phase 1.5 | 6 | 6 ✅ | System Scan ✅, scan history ✅ |
| Phase 2 | 3 | 3 ✅ | CLI host select ✅, Host Selection TUI ✅, wizard wiring ✅ |
| Phase 3 | 2 | 2 ✅ | Divergence indicators ✅, guidance tooltip ✅ |
| Phase 4 | 6 | 6 ✅ | State clearing ✅, service manager ✅, dormant ✅, conflicts ✅, rollback ✅, dotfiles ✅ |
| Phase 5 | 4 | 4 ✅ | ProfileBuilder persist ✅, ModuleCreator persist ✅, profile activation ✅, module enable ✅ |
| Phase 6 | 3 | 3 ✅ | Confirmation UX ✅, update behavior ✅, snapshots ✅ |
| Phase 7 | 4 | 4 ✅ | Cleanup dry_run ✅, doctor parity ✅, CLI clean ✅, doctor refresh key ✅ |
| Phase 8 | 3 | 3 ✅ | Sync conflicts ✅, push auto-commit ✅, pull dirty-tree ✅ |
| Phase 9 | 6 | 6 ✅ | Secrets wiring ✅, Recovery wiring ✅, list_encrypted ✅, consolidation ✅, audit ✅, CLI ✅ |
| Cross-Docs | 2 | 2 ✅ | Architecture ✅, examples ✅ |
| Cross-Infra | 2 | 2 ✅ | Tests ✅, coverage ✅ |

---

## Guideline Cross-Reference

Each phase has a detailed implementation guideline document with deeper analysis,
code references, and additional discovered issues beyond those tracked above:

| Phase | Guideline | Discovered Issues | Notes |
|---|---|---|---|
| Phase 1 | `docs/scenario-1-phase-1.md` | S1-P1-004 through S1-P1-007 | First-launch detection, wizard PM, refresh PM, wizard tests |
| Phase 2 | `docs/scenario-1-phase-2.md` | S1-P2-004 through S1-P2-006 | Host TOML creation, config convention, stale reference |
| Phase 3 | `docs/scenario-1-phase-3.md` | 6 issues (unnumbered) | SyncStatus naming, sync_info unused, no dashboard handler scope |
| Phase 4 | `docs/scenario-1-phase-4.md` | B1 through B8 | Service manager, state clearing, rollback, dotfiles, dormant, packages |
| Phase 5 | `docs/scenario-1-phase-5.md` | S1-P5-NEW-001 through -013 | Activation gaps, CLI create, validation, templates, tests |
| Phase 6 | `docs/scenario-1-phase-6.md` | S1-P6-NEW-001 through -012 | UpdateService wiring, snapshot, pre-flight, type unification |
| Phase 7 | `docs/scenario-1-phase-7.md` | S1-P7-NEW-001 through -014 | Cleanup dry_run, DoctorService, CLI clean, refresh, tests |
| Phase 8 | `docs/scenario-1-phase-8.md` | D-P8-001 through D-P8-012 | Auto-commit, dirty-tree, confirm, auto-refresh, SyncService duplication |
| Phase 9 | `docs/scenario-1-phase-9.md` | D-P9-001 through D-P9-013 | Dead shells, state never populated, two secrets layers, audit logging |

> **~80 issues discovered across guidelines; 47 highest-priority tracked above.**
> Remaining lower-priority items (P2–P3) are documented in individual guideline files.
