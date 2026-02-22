# Current State

## Status: Phase 0 — Foundation Fixes (IMPLEMENTED)

**Scenario 1 — Newcomer Journey** is **fully implemented** (45/45 tasks).
**Hardening Sprints H1 + H2 + H3** addressed 65 gaps — **65 done, 0 remaining** (100%).
**Phase 0 — Foundation Fixes** — **12/12 tasks implemented** (2026-02-22).

### Phase 0 Results

| Sprint | Focus | Tasks | Status |
|--------|-------|-------|--------|
| **0.1** | UX Quick Wins | 6 tasks (F0-001 → F0-006) | ✅ Done |
| **0.2** | Tech Debt Closure | 6 tasks (F0-007 → F0-012) | ✅ Done |

### Next: Phase 1 — Core Experience (Apply/Diff/Drift/Templates)

Phase 1 sprint-level planning should begin now. See [`docs/product-review-and-roadmap.md`](product-review-and-roadmap.md) §11 for the roadmap-level tasks (F1-001 → F1-022).

**Planning docs:**
- [`docs/product-review-and-roadmap.md`](product-review-and-roadmap.md) — Full product review + 4-phase roadmap
- [`docs/phase0-kanban.md`](phase0-kanban.md) — Sprint kanban board with acceptance criteria
- [`docs/phase0-technical-guide.md`](phase0-technical-guide.md) — Implementation guide with exact file/line references

**User research:**
- [`docs/newcomer-expectations-brainstorm.md`](newcomer-expectations-brainstorm.md) — Newcomer persona analysis
- [`docs/mid-level-user-expectations-brainstorm.md`](mid-level-user-expectations-brainstorm.md) — Mid-level persona analysis

Branch: `feature/tui-enhancement-phase1`.

---

## Codebase Metrics

| Crate | Lines | Tests | Ignored | Role |
|---|---|---|---|---|
| iron-core | 29,900 | 905 | 4 | Domain models, services, state |
| iron-tui | 17,500 | 445 | 0 | Ratatui TUI (27 views) |
| iron-cli | 8,000 | — | — | Clap CLI |
| iron-fs | 1,969 | 88 | 0 | File operations |
| iron-git | 2,317 | 95 | 0 | Git/git-crypt |
| iron-pacman | 2,650 | 101 | 0 | Pacman/paru/yay |
| iron-systemd | 1,889 | 69 | 0 | Systemd |
| **Total** | **~64,500** | **1,703** | **4** | **0 failed** |

### Growth Timeline

| Milestone | Tests | LOC | Delta |
|-----------|------:|----:|-------|
| Pre-Scenario 1 | ~1,200 | ~55,000 | baseline |
| Post-Sprint 4 (S1 complete) | 1,567 | ~60,740 | +367 / +5,740 |
| Post-Hardening (H1+H2) | 1,695 | ~63,349 | +128 / +2,609 |
| Post-Hardening (H3 final) | 1,703 | ~64,500 | +8 / +1,151 |

---

## Sprint Summary

### Hardening Sprint H3 — 7 tasks ✅

Final 7 tasks: A-001 (SyncService → CommandExecutor), A-009 (persistent SyncService),
A-010 (pre-push secrets lock), C-009 (full recovery import), D-009 (background push/pull),
D-012 (module creator dotfiles), F-005 (PackageManager in CleanupService).

### Hardening Sprint H2 — 33 tasks ✅

Completed: A-004, A-007, B-006, C-004, C-006, C-007, C-008, C-010, D-006, D-007,
D-008, D-010, D-011, D-013, E-008, E-009, E-010, E-011, E-012, E-013, E-014,
F-002, F-003, F-004, F-006, F-007, F-008, F-009, F-010, F-011.

### Hardening Sprint H1 — 28 tasks ✅

All 28 completed: A-002, A-003, A-005, A-008, B-001, B-002, B-003, B-004, B-005,
C-001, C-005, D-001, D-002, D-003, D-004, D-005, E-001, E-002, E-003, E-004,
E-005, E-006, E-007, F-001.

### Sprint 4 — Polish (P3) — 6 tasks ✅
Doctor refresh key, scan history, HostSelection TUI, wizard host wiring,
CLI host select, divergence guidance tooltip.

### Sprint 3 — New Features (P2) — 16 tasks ✅
ScanService, SystemScan TUI, dashboard divergence, ProfileBuilder/ModuleCreator
persistence, DoctorService, sync conflict resolution, SecretsBackend consolidation,
secrets audit logging, CLI secrets/recover commands.

### Sprint 2 — Core Gaps (P1) — 16 tasks ✅
SystemdServiceAdapter injection, dormant lifecycle, conflict blocking, switch rollback,
dotfiles dir resolution, profile activation, module enable/disable, update preflight,
snapshot integration, CLI clean, sync push/pull, list_encrypted.

### Sprint 1 — Critical Fixes (P0) — 8 tasks ✅
PackageManager injection (5 TUI sites + wizard), stub annotations, risk-differentiated
confirms, clear_active_bundle, cleanup dry_run, secrets + recovery handler wiring.

---

## Architecture Reference

### Dependency Injection Pattern
- `App` holds `package_manager: Arc<dyn PackageManager>` and `service_manager: Arc<dyn SystemService>`
- Both injected via `App::new()`. All `DefaultBundleService` sites chain `.with_package_manager()` and `.with_service_manager()`
- `WizardState::apply()` takes both as parameters
- CLI wires `iron_pacman::DefaultPackageManager` + `iron_systemd::SystemdServiceAdapter::user()`

### Service Layer (`iron-core/src/services/`)
| Service | Trait | Implementation |
|---|---|---|
| Bundle | `BundleService` | `DefaultBundleService` |
| Host | `HostService` | `DefaultHostService` |
| Profile | `ProfileService` | `DefaultProfileService` |
| Module | `ModuleService` | `DefaultModuleService` |
| Update | `UpdateService` | `DefaultUpdateService<S: SnapshotManager>` |
| Cleanup | `CleanupService` | `DefaultCleanupService` |
| Sync | `SyncService` | `DefaultSyncService` |
| Scan | `ScanService` | `DefaultScanService` |
| Doctor | `DoctorService` | `DefaultDoctorService` |
| Secrets | `SecretsService` | (with `SecretsBackend` trait for iron-git) |
| Recovery | `RecoveryService` | `DefaultRecoveryService` |
| State | `StateManager` | (direct struct, handles JSON persistence) |

### Snapshot Integration
- `iron_core::snapshot::create_manager()` returns `Box<dyn SnapshotManager>` (auto-detects timeshift/snapper/noop)
- Blanket impl lets `Box<dyn SnapshotManager>` work wherever `S: SnapshotManager` is required

### Bundle Lifecycle
```
NotInstalled → activate() → Active
Active → deactivate() → Dormant (configs in dormant/<id>/)
Dormant → activate() → Active (configs restored)
Active(A) → switch(A, B) → Active(B)  [rollback to A on failure]
```

### TUI Views (27 total)
Dashboard, SetupWizard, Bundles, BundleDetail, Profiles, ProfileDetail,
Modules, ModuleDetail, UpdatePreview, Sync, Settings, SystemMaintenance,
CleanSystem, CleanupPreview, CleanupResults, SecurityModules, ConfigManager,
OperationLog, Doctor, Secrets, Recovery, ProfileBuilder, ModuleCreator,
SystemScan, HostSelection

---

## Known Issues & Tech Debt

### Remaining Hardening Backlog (7 tasks)

| Task | Description | Priority | Blocker |
|------|-------------|----------|---------|
| **A-001** | SyncService bypasses iron-git (16× raw `Command::new("git")`) | P1 | — |
| **F-005** | CleanupService uses 6× raw `Command::new` instead of iron_pacman | P2 | dep inversion needed |
| **C-009** | import() restores state only, not packages/services/dotfiles (FR-6.3) | P3 | — |
| **D-012** | ModuleCreator missing dotfile mapping step | P3 | — |
| **A-009** | SyncService creates fresh instances per action | P3 | A-001 |
| **A-010** | Secrets not locked before push | P3 | A-001 |
| **D-009** | Push/pull blocks TUI thread (no async) | P3 | A-001 |

> Full task details: [`docs/scenario-1-hardening.md`](scenario-1-hardening.md)

### Active Issues
- **Clippy**: 2 pre-existing warnings — `collapsible_if` in wizard.rs, `too_many_arguments` in update.rs.
  Zero warnings from hardening changes.
- **Integration tests require sudo**: iron-pacman/iron-systemd tests prompt for sudo.
  CI-safe: `cargo test -p iron-core -p iron-tui -p iron-cli -p iron-fs -p iron-git`
- **FR-5.9 violation**: SyncService has no timeout on git operations (A-001).
- **FR-6.3 partial**: Recovery import is state-only, not full 4-step flow (C-009).

### FR Stubs (from requirements, not yet targeted)
- **FR-3.5**: Smart merge for overlapping symlinks
- **FR-7.4**: Interactive merge on sync conflict

---

## Recent Changes

- **2026-02-20** — Hardening sprint documentation audit: all 65 tasks verified against codebase
- **2026-02-20** — Hardening Sprint H2 (29/33 tasks completed)
- **2026-02-20** — Hardening Sprint H1 (28/28 tasks completed)
- **2026-02-21** — Clippy cleanup: 32 warnings fixed, committed as `8761e0b`
- **2026-02-21** — Sprint 4 (6 P3 tasks) completed, committed as `1c74ed1`. **All 45/45 S1 tasks done.**
- **2026-02-20** — Sprint 3 (16 P2 tasks) completed, committed as `ab81806`
- **2026-02-20** — Sprint 2 (16 P1 tasks) completed, committed as `38c1b72`
- **2026-02-19** — Sprint 1 (8 P0 tasks) completed, committed as `6a28dea`
- **2026-02-18** — Scenario 1 phase analysis docs (phases 1–9) created
- **2026-02-18** — Project scaffolded with Mind Agent Framework
