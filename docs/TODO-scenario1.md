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
| Secrets TUI | `[STUB]` | **REAL** – git-crypt status, encrypted file list, action keys |
| Recovery TUI | `[STUB]` | **REAL** – status panel, export/import/generate keys |
| System Scan | described in Phase 1.5 | **MISSING** – no scan service or TUI view exists |
| Host Selection | described in Phase 2 | **MISSING** – no multi-host selection UI exists |
| TUI Bundle activate | uses real PM | **BUG** – `App::init()` creates `BundleService` with `NoopPackageManager` |
| TUI System Update | dry-run hinted | **REAL** – calls `pacman -Syu --noconfirm` (not dry-run) |
| Typed confirmation | CRITICAL updates | **MISSING** – all risk levels use same Y/N dialog |
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
    on all 4 `DefaultBundleService::new()` call sites in actions.rs.

- [x] **S1-P1-002** | **P1** | Correct `[STUB]` annotations in user-workflow.md
  - **Why**: 6 features marked `[STUB]` are fully implemented. Misleads contributors.
  - **Action**: Remove `[STUB]` from Doctor, ProfileBuilder, ModuleCreator, Secrets, Recovery
    descriptions. Add accurate "implemented" notes.
  - **Files**: `user-workflow.md`
  - **Test**: Manual review.
  - **Deps**: None
  - **Completed**: 2026-02-19 — Updated 20+ annotations across user-workflow.md. Also corrected
    TUI update execution description (was incorrectly documented as dry-run only).

- [ ] **S1-P1-003** | **P2** | Add progress indicator to Setup Wizard
  - **Why**: user-workflow spec calls for "Step X of 6" progress display.
    Current wizard has steps but no visible progress counter in the TUI.
  - **Action**: Add step counter to `setup_wizard.rs` render function.
  - **Files**: `crates/iron-tui/src/ui/setup_wizard.rs`
  - **Test**: Visual verification; unit test for step count text.
  - **Deps**: None

### Phase 1.5 — System Scan *(NEW FEATURE)*

- [ ] **S1-P1.5-001** | **P2** | Create `ScanService` in iron-core
  - **Why**: user-workflow describes a system scan that detects existing dotfiles,
    packages, and potential conflicts before bundle activation. No code exists.
  - **Action**: New service that scans `$HOME` for known config patterns, reads
    installed packages from pacman, and produces a `ScanReport`.
  - **Files**: `crates/iron-core/src/services/scan.rs`, `crates/iron-core/src/services/mod.rs`
  - **Test**: Unit tests with mock filesystem.
  - **Deps**: iron-fs, iron-pacman

- [ ] **S1-P1.5-002** | **P2** | Create `ScanReport` model
  - **Why**: Need a structured output type for scan results.
  - **Action**: Define `ScanReport` struct with fields: `existing_configs`, `installed_packages`,
    `potential_conflicts`, `recommendations`.
  - **Files**: `crates/iron-core/src/models/scan.rs`, `crates/iron-core/src/models/mod.rs`
  - **Test**: Serialization tests.
  - **Deps**: None

- [ ] **S1-P1.5-003** | **P2** | Create `SystemScan` TUI view
  - **Why**: user-workflow describes a visual scan progress + results screen.
  - **Action**: New view in iron-tui showing scan progress, discovered items,
    conflict warnings, and action recommendations.
  - **Files**: `crates/iron-tui/src/ui/system_scan.rs`, add `SystemScan` to `View` enum
  - **Test**: Render tests.
  - **Deps**: S1-P1.5-001, S1-P1.5-002

- [ ] **S1-P1.5-004** | **P2** | Wire scan into Setup Wizard flow
  - **Why**: Scan should run automatically after initial setup, before bundle activation.
  - **Action**: After wizard completes, transition to SystemScan view, then to Dashboard.
  - **Files**: `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Integration test for wizard → scan → dashboard flow.
  - **Deps**: S1-P1.5-003

- [ ] **S1-P1.5-005** | **P3** | Scan history / re-scan capability
  - **Why**: user-workflow mentions ability to re-run scan from Settings.
  - **Action**: Store scan results in state.json, add re-scan key binding.
  - **Files**: `state.json` schema, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: State persistence test.
  - **Deps**: S1-P1.5-001

- [ ] **S1-P1.5-006** | **P2** | Add `iron scan` CLI command
  - **Why**: CLI parity with TUI scan feature.
  - **Action**: New subcommand that runs ScanService and outputs results.
  - **Files**: `crates/iron-cli/src/commands/scan.rs`, `crates/iron-cli/src/commands/mod.rs`
  - **Test**: CLI integration test.
  - **Deps**: S1-P1.5-001

### Phase 2 — Host Selection

- [ ] **S1-P2-001** | **P3** | Create `HostSelection` TUI view
  - **Why**: user-workflow describes a host picker for multi-machine setups.
    Currently the TUI reads the single host from config; no selection UI exists.
  - **Action**: New view listing discovered host TOML files with preview panel.
  - **Files**: `crates/iron-tui/src/ui/host_selection.rs`, add to `View` enum
  - **Test**: Render test with mock host configs.
  - **Deps**: None

- [ ] **S1-P2-002** | **P3** | Wire host selection into first-launch flow
  - **Why**: If multiple hosts exist, user should pick one before proceeding.
  - **Action**: After setup wizard, if >1 host config found, show HostSelection.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Flow test.
  - **Deps**: S1-P2-001

- [ ] **S1-P2-003** | **P3** | Add `iron host select` CLI command
  - **Why**: CLI parity.
  - **Action**: Interactive or flag-based host selection.
  - **Files**: `crates/iron-cli/src/commands/host.rs`
  - **Test**: CLI integration test.
  - **Deps**: None

### Phase 3 — Dashboard Overview

- [ ] **S1-P3-001** | **P2** | Add divergence indicators to Dashboard
  - **Why**: user-workflow describes visual indicators when configs have drifted
    from their managed state. Dashboard currently shows status but no drift detection.
  - **Action**: Compare current file hashes against last-known state, show warning
    icons next to diverged modules.
  - **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-core/src/services/sync.rs`
  - **Test**: Render test with diverged state; unit test for hash comparison.
  - **Deps**: None

- [ ] **S1-P3-002** | **P3** | Dashboard divergence guidance tooltip
  - **Why**: user-workflow says diverged items should show resolution options.
  - **Action**: On selecting a diverged item, show popup with "restore" / "accept" / "diff" options.
  - **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Handler test for divergence actions.
  - **Deps**: S1-P3-001

### Phase 4 — Bundle Exploration & Activation

- [ ] **S1-P4-001** | **P1** | Implement dormant directory management
  - **Why**: `deactivate()` unlinks symlinks but does NOT move configs to `dormant/`.
    user-workflow describes dormant bundles as archived in the `dormant/` directory.
  - **Action**: On deactivate, move bundle configs to `dormant/<bundle_name>/`.
    On re-activate, move them back.
  - **Files**: `crates/iron-core/src/services/bundle.rs`, `crates/iron-fs/src/lib.rs`
  - **Test**: Integration test: activate → deactivate → verify dormant dir → reactivate.
  - **Deps**: None

- [ ] **S1-P4-002** | **P1** | Block activation when conflicts detected
  - **Why**: `check_conflicts()` returns conflicts but `activate()` proceeds anyway.
    user-workflow says activation should be blocked with resolution options.
  - **Action**: In TUI handler, call `check_conflicts()` before `activate()`.
    If conflicts exist, show conflict resolution dialog instead of proceeding.
  - **Files**: `crates/iron-tui/src/app/actions.rs`, `crates/iron-tui/src/app/handlers.rs`
  - **Test**: Test that activation is blocked when conflicts exist.
  - **Deps**: None

### Phase 5 — Profile & Module Management

- [ ] **S1-P5-001** | **P2** | ProfileBuilder – persist created profiles
  - **Why**: ProfileBuilder wizard renders UI but may not persist to disk.
    Need to verify and ensure TOML is written on "Create" confirmation.
  - **Action**: Verify `handle_profile_builder_input()` calls a service method
    that writes the profile TOML. Add if missing.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-core/src/services/profile.rs`
  - **Test**: Create profile via TUI → verify TOML file exists.
  - **Deps**: None

- [ ] **S1-P5-002** | **P2** | ModuleCreator – persist created modules
  - **Why**: Same as above for modules.
  - **Action**: Verify `handle_module_creator_input()` persists to disk.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-core/src/services/module.rs`
  - **Test**: Create module via TUI → verify TOML + directory structure.
  - **Deps**: None

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

- [ ] **S1-P6-002** | **P1** | **DECISION**: Confirm TUI update behavior
  - **Why**: `run_system_update()` calls `package_manager.upgrade(false)` which
    runs `sudo pacman -Syu --noconfirm`. user-workflow implies previewing first.
    The preview exists (`UpdatePreview` view) but pressing 'u' runs the real update.
  - **Action**: **Option A** *(recommended)*: Keep real updates, ensure
    risk-differentiated confirmation (S1-P6-001) gates the action.
    **Option B**: Add dry-run flag, show diff, require second confirmation.
  - **Files**: `crates/iron-tui/src/app/actions.rs`
  - **Test**: Depends on chosen option.
  - **Deps**: S1-P6-001

- [ ] **S1-P6-003** | **P1** | Pre-update snapshot integration
  - **Why**: user-workflow describes automatic snapshot before CRITICAL updates.
    Code has `TODO: Detect and use timeshift/snapper` comments.
  - **Action**: Detect installed snapshot tool (timeshift/snapper), create snapshot
    before update, store snapshot ID for potential rollback.
  - **Files**: `crates/iron-core/src/services/update.rs`, `crates/iron-core/src/context.rs`
  - **Test**: Mock snapshot tool; verify snapshot created before update proceeds.
  - **Deps**: S1-P6-001

### Phase 7 — Maintenance & Cleanup

- [ ] **S1-P7-001** | **P2** | Doctor TUI ↔ CLI parity check
  - **Why**: Doctor TUI shows 7 checks; CLI `iron doctor` may have different checks.
    user-workflow says both should show identical results.
  - **Action**: Audit both paths, extract shared check logic into iron-core service,
    have both TUI and CLI consume it.
  - **Files**: `crates/iron-core/src/services/doctor.rs`, `crates/iron-tui/src/ui/doctor.rs`,
    `crates/iron-cli/src/commands/doctor.rs`
  - **Test**: Same input → same output from both interfaces.
  - **Deps**: None

### Phase 8 — Sync & Collaboration

- [ ] **S1-P8-001** | **P2** | Sync conflict resolution UI
  - **Why**: user-workflow describes a merge conflict resolution flow in the TUI.
    Current Sync view shows status but conflict resolution is manual (CLI git).
  - **Action**: Add conflict detection to Sync view, show conflicted files with
    options: "keep local" / "keep remote" / "open diff".
  - **Files**: `crates/iron-tui/src/ui/sync.rs`, `crates/iron-git/src/lib.rs`
  - **Test**: Render test with conflict state.
  - **Deps**: None

### Phase 9 — Security & Secrets

*(Secrets and Recovery TUI views are already implemented. Remaining work:)*

- [ ] **S1-P9-001** | **P2** | Secrets — encrypt/decrypt file actions
  - **Why**: Secrets view renders status and file list but action keys may not
    trigger actual git-crypt operations.
  - **Action**: Verify 'e' (encrypt) and 'd' (decrypt) keys actually invoke
    git-crypt lock/unlock. Wire if missing.
  - **Files**: `crates/iron-tui/src/app/handlers.rs`, `crates/iron-core/src/services/secrets.rs`
  - **Test**: Integration test with git-crypt.
  - **Deps**: None

### Cross-Phase — Documentation

- [ ] **S1-X-001** | **P1** | Update architecture.md with scan service
  - **Why**: New ScanService needs to be documented in the architecture.
  - **Action**: Add ScanService to service layer docs, update dependency diagram.
  - **Files**: `docs/architecture.md`
  - **Test**: Review.
  - **Deps**: S1-P1.5-001

- [ ] **S1-X-002** | **P1** | Update EXAMPLES.md with new commands
  - **Why**: New `iron scan`, updated host commands need examples.
  - **Action**: Add scan examples, host selection examples.
  - **Files**: `EXAMPLES.md`
  - **Test**: `iron scan --help` works.
  - **Deps**: S1-P1.5-006

### Cross-Phase — Infrastructure

- [ ] **S1-XI-001** | **P2** | Add `ScanService` to integration test harness
  - **Why**: Ensure scan feature has e2e test coverage.
  - **Action**: Add scan scenarios to existing test infrastructure.
  - **Files**: Test files TBD based on existing test structure.
  - **Test**: `cargo test` passes with scan tests.
  - **Deps**: S1-P1.5-001

- [ ] **S1-XI-002** | **P2** | Coverage gate for new code
  - **Why**: Project has 64% coverage; new features should maintain or improve it.
  - **Action**: Add `#[cfg(test)]` modules to all new files, ensure tarpaulin config
    includes new crate paths.
  - **Files**: `tarpaulin.toml` (if exists), new test modules.
  - **Test**: `cargo tarpaulin` reports ≥64%.
  - **Deps**: All implementation tasks

---

## Execution Order (Recommended Sprints)

### Sprint 1 — Critical Fixes (P0)
| Task | Description | Est |
|---|---|---|
| S1-P1-001 | Inject real PM into BundleService | 1h |
| S1-P6-001 | Risk-differentiated confirmation | 3h |
| S1-P1-002 | Fix [STUB] annotations in docs | 30m |
| **Total** | | **4.5h** |

### Sprint 2 — Core Gaps (P1)
| Task | Description | Est |
|---|---|---|
| S1-P4-001 | Dormant directory management | 3h |
| S1-P4-002 | Block activation on conflicts | 2h |
| S1-P6-002 | Decision: TUI update behavior | 1h |
| S1-P6-003 | Pre-update snapshot integration | 4h |
| S1-X-001 | Update architecture.md | 1h |
| S1-X-002 | Update EXAMPLES.md | 1h |
| **Total** | | **12h** |

### Sprint 3 — New Features (P2)
| Task | Description | Est |
|---|---|---|
| S1-P1-003 | Setup wizard progress indicator | 1h |
| S1-P1.5-001 → 004, 006 | System Scan (full feature) | 12h |
| S1-P3-001 | Dashboard divergence indicators | 3h |
| S1-P5-001 | ProfileBuilder persistence | 2h |
| S1-P5-002 | ModuleCreator persistence | 2h |
| S1-P7-001 | Doctor TUI/CLI parity | 2h |
| S1-P8-001 | Sync conflict resolution | 4h |
| S1-P9-001 | Secrets encrypt/decrypt actions | 2h |
| S1-XI-001 | Scan integration tests | 2h |
| S1-XI-002 | Coverage gate | 1h |
| **Total** | | **31h** |

### Sprint 4 — Polish (P3)
| Task | Description | Est |
|---|---|---|
| S1-P1.5-005 | Scan history / re-scan | 2h |
| S1-P2-001 → 003 | Host Selection (full feature) | 6h |
| S1-P3-002 | Divergence guidance tooltip | 2h |
| **Total** | | **10h** |

---

## Summary

| Priority | Count | Estimated |
|---|---|---|
| P0 (Critical) | 2 | 4h |
| P1 (High) | 8 | 12h |
| P2 (Medium) | 15 | 31h |
| P3 (Low) | 5 | 10h |
| **Total** | **30** | **57h** |

| Phase | Tasks | Focus |
|---|---|---|
| Phase 1 | 3 | PM injection, stubs fix, progress indicator |
| Phase 1.5 | 6 | System Scan (entirely new) |
| Phase 2 | 3 | Host Selection (entirely new) |
| Phase 3 | 2 | Dashboard divergence |
| Phase 4 | 2 | Dormant mgmt, conflict blocking |
| Phase 5 | 2 | Persistence verification |
| Phase 6 | 3 | Confirmation UX, snapshots |
| Phase 7 | 1 | Doctor parity |
| Phase 8 | 1 | Sync conflicts |
| Phase 9 | 1 | Secrets actions |
| Cross-Docs | 2 | Architecture, examples |
| Cross-Infra | 2 | Tests, coverage |
