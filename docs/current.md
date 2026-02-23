# Current State

## Status: Phase 2 — Power User Features (COMPLETE)

**Scenario 1 — Newcomer Journey** is **fully implemented** (45/45 tasks).
**Hardening Sprints H1 + H2 + H3** addressed 65 gaps — **65 done, 0 remaining** (100%).
**Phase 0 — Foundation Fixes** — **12/12 tasks implemented** (2026-02-22). ✅
**Phase 1 — Core Experience** — **Sprints 1.1-1.4 implemented** (2026-02-22). ✅
**Phase 2 — Power User Features** — **Sprints 2.1-2.3 implemented** (2026-02-22). ✅

### Phase 0 Results

| Sprint | Focus | Tasks | Status |
|--------|-------|-------|--------|
| **0.1** | UX Quick Wins | 6 tasks (F0-001 → F0-006) | ✅ Done |
| **0.2** | Tech Debt Closure | 6 tasks (F0-007 → F0-012) | ✅ Done |

**Phase 0 key deliverables:**
- `iron` (no args) launches TUI by default (F0-001)
- Dashboard shows sync status + disk space (F0-002)
- Doctor has 13 health checks including disk space (F0-003)
- Getting Started hints for new users (F0-004)
- CLI operations show summary lines + `--explain` flag (F0-005, F0-006)
- All hardening tech debt closed: SyncService executor, CleanupService executor, secrets auto-lock, recovery full import, module creator dotfiles (F0-007 → F0-012)
- `iron clean --dry-run` flag added for safe testing

### Phase 1 — Core Experience Results

| Sprint | Focus | Tasks | Status |
|--------|-------|-------|--------|
| **1.1** | Host as Source of Truth | F1-001 → F1-003 | ✅ Done |
| **1.2** | The Apply Command | F1-005 → F1-010 | ✅ Done |
| **1.3** | Diff & Drift Detection | F1-011 → F1-018 | ✅ Done |
| **1.4** | Template Engine (Stretch) | F1-019 | ✅ Done |

**Phase 1 key deliverables implemented:**
- **Host struct extended** with `bundle`, `profile`, `extra_modules`, `variables` fields (backward-compatible via `#[serde(default)]`)
- **DesiredState resolver** (`resolve_desired_state()`) resolves host.toml → bundle → profile → modules → packages/dotfiles/services with profile inheritance, module dependency resolution, and conflict detection
- **ApplyService** (`iron apply`) computes diff between desired and actual state, produces `ApplyPlan` with ordered actions, executes with rollback tracking
- **DriftService** (`iron diff`) detects package drift (missing/extra), service drift (not enabled), config drift (missing/broken/wrong symlinks)
- **CLI commands**: `iron apply [--dry-run] [--module ID] [-y]` and `iron diff [--adopt] [--correct] [--dry-run] [-y]`
- **TUI views**: Apply view (`[a]` from Dashboard), Drift detail view (`[D]` from Dashboard), Dashboard drift indicator badge
- **Template engine** in iron-fs: `{{variable}}` substitution with whitespace trimming, unknown variable preservation, variable extraction
- **SystemService::is_enabled()** method added to trait with default impl + systemd implementation

### New files created in Phase 1

```
iron-core/src/services/apply.rs     ← DesiredState, ApplyService, ApplyPlan, ApplyAction
iron-core/src/services/drift.rs     ← DriftService, DriftReport, PackageDrift, ServiceDrift, ConfigDrift
iron-cli/src/commands/apply.rs      ← CLI apply command
iron-cli/src/commands/diff.rs       ← CLI diff command
iron-tui/src/ui/apply.rs           ← TUI Apply + Drift views
iron-fs/src/lib.rs (template mod)   ← Template engine
```

**Planning docs:**
- [`docs/product-review-and-roadmap.md`](product-review-and-roadmap.md) — Full product review + 4-phase roadmap
- [`docs/phase0-kanban.md`](phase0-kanban.md) — Phase 0 sprint kanban (✅ complete)
- [`docs/phase0-technical-guide.md`](phase0-technical-guide.md) — Phase 0 implementation guide (✅ complete)
- [`docs/phase1-kanban.md`](phase1-kanban.md) — Phase 1 sprint kanban with acceptance criteria
- [`docs/phase1-technical-guide.md`](phase1-technical-guide.md) — Phase 1 implementation guide with exact code patterns

**User research:**
- [`docs/newcomer-expectations-brainstorm.md`](newcomer-expectations-brainstorm.md) — Newcomer persona analysis
- [`docs/mid-level-user-expectations-brainstorm.md`](mid-level-user-expectations-brainstorm.md) — Mid-level persona analysis

Branch: `feature/tui-enhancement-phase1`.

---

## Codebase Metrics

| Crate | Lines | Tests | Ignored | Role |
|---|---|---|---|---|
| iron-core | ~32,000 | ~930 | 4 | Domain models, services, state |
| iron-tui | ~18,000 | ~460 | 0 | Ratatui TUI (28 views) |
| iron-cli | ~10,000 | ~135 | — | Clap CLI |
| iron-fs | 1,969 | 88 | 0 | File operations |
| iron-git | 2,317 | 95 | 0 | Git/git-crypt |
| iron-pacman | 2,650 | 101 | 0 | Pacman/paru/yay |
| iron-systemd | 1,889 | 69 | 0 | Systemd |
| **Total** | **~69,000** | **2,033** | **4** | **0 failed** |

### Growth Timeline

| Milestone | Tests | LOC | Delta |
|-----------|------:|----:|-------|
| Pre-Scenario 1 | ~1,200 | ~55,000 | baseline |
| Post-Sprint 4 (S1 complete) | 1,567 | ~60,740 | +367 / +5,740 |
| Post-Hardening (H1+H2) | 1,695 | ~63,349 | +128 / +2,609 |
| Post-Hardening (H3 final) | 1,703 | ~64,500 | +8 / +1,151 |
| Post-Phase 1 | ~1,800 | ~66,000 | +97 / +1,500 |
| Post-Phase 2 | 2,033 | ~69,000 | +233 / +3,000 |

---

### Phase 2 — Power User Features Results

| Sprint | Focus | Tasks | Status |
|--------|-------|-------|--------|
| **2.1** | Snapshot & Rollback | F2-001 → F2-008 (8 tasks) | ✅ Done |
| **2.2** | Enhanced CLI Output | F2-009 → F2-014 (6 tasks) | ✅ Done |
| **2.3** | Config Validation & Security | F2-015 → F2-019 (5 tasks) | ✅ Done |

**Phase 2 key deliverables implemented:**
- **SnapshotService** (`iron snapshot create/list/restore/delete/prune`) with JSON storage in `.snapshots/`
- **Auto-snapshot** before destructive operations (apply, update) with auto-prune
- **Per-module rollback** (`iron rollback --module <id>`)
- **TUI Snapshot timeline** view (`[t]` from Dashboard) with list/detail
- **Tree/table/summary CLI output** methods (tree_root/branch/last, table, summary_block)
- **Progress spinners** via indicatif (ProgressReporter with spinner + bar modes)
- **Enhanced error messages** with `suggestion()` method on IronError (10 error types covered)
- **Pre-apply config validation** (`iron validate`) checks TOML, references, paths, conflicts
- **Security level dashboard** (`iron security`) with Basic/Standard/Advanced/Paranoid scoring
- **Module security_points** field for security module tagging
- **Dashboard security indicator** badge in TUI

### New files created in Phase 2

```
iron-core/src/services/snapshot_service.rs  ← SnapshotService, SnapshotRecord, auto-snapshot/prune
iron-core/src/services/security.rs          ← SecurityLevel, SecurityService, SecurityReport
iron-cli/src/commands/snapshot.rs           ← CLI snapshot + rollback commands
iron-cli/src/commands/security.rs           ← CLI security status command
iron-cli/src/commands/validate.rs           ← CLI validate command
iron-cli/src/progress.rs                    ← ProgressReporter (indicatif wrapper)
iron-tui/src/ui/snapshot.rs                 ← TUI Snapshot timeline view
```

**Planning docs:**
- [`docs/phase2-kanban.md`](phase2-kanban.md) — Phase 2 sprint kanban (✅ complete)
- [`docs/phase2-technical-guide.md`](phase2-technical-guide.md) — Phase 2 implementation guide (✅ complete)

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
| **Apply** | `ApplyService` | `DefaultApplyService` (Phase 1) |
| **Drift** | `DriftService` | `DefaultDriftService` (Phase 1) |
| **Snapshot** | `SnapshotService` | `DefaultSnapshotService` (Phase 2) |
| **Security** | `SecurityService` | `DefaultSecurityService` (Phase 2) |

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

### TUI Views (28 total)
Dashboard, SetupWizard, Bundles, BundleDetail, Profiles, ProfileDetail,
Modules, ModuleDetail, UpdatePreview, Sync, Settings, SystemMaintenance,
CleanSystem, CleanupPreview, CleanupResults, SecurityModules, ConfigManager,
OperationLog, Doctor, Secrets, Recovery, ProfileBuilder, ModuleCreator,
SystemScan, HostSelection, Apply, DriftDetail, **Snapshots**

---

## Known Issues & Tech Debt

### Hardening Backlog — ✅ ALL RESOLVED (Phase 0)

| Task | Description | Resolved By |
|------|-------------|-------------|
| **A-001** | SyncService bypasses iron-git (raw `Command::new("git")`) | F0-007 ✅ |
| **F-005** | CleanupService uses raw `Command::new` instead of iron_pacman | F0-008 ✅ |
| **C-009** | import() restores state only, not packages/services/dotfiles | F0-012 ✅ |
| **D-012** | ModuleCreator missing dotfile mapping step | F0-011 ✅ |
| **A-009** | SyncService creates fresh instances per action | F0-007 ✅ |
| **A-010** | Secrets not locked before push | F0-010 ✅ |
| **D-009** | Push/pull blocks TUI thread (no async) | F0-009 ✅ (was already done) |

### Active Issues
- **Integration tests require sudo**: iron-pacman/iron-systemd tests prompt for sudo.
  Use `--dry-run` for clean tests. CI-safe: `cargo test -p iron-core -p iron-tui -p iron-cli -p iron-fs -p iron-git`

### FR Stubs (from requirements, not yet targeted)
- **FR-3.5**: Smart merge for overlapping symlinks
- **FR-7.4**: Interactive merge on sync conflict

---

### Next: Phase 3 — Declarative Convergence & Multi-Machine Readiness

**Phase 3 planning complete** — 22 tasks across 4 sprints.

See:
- [`docs/phase3-kanban.md`](phase3-kanban.md) — Phase 3 sprint kanban (22 tasks)
- [`docs/phase3-technical-guide.md`](phase3-technical-guide.md) — Phase 3 implementation guide
- [`docs/phase2-gap-analysis.md`](phase2-gap-analysis.md) — Gap analysis informing Phase 3
- [`docs/product-review-and-roadmap.md`](product-review-and-roadmap.md) — Full roadmap (Phase 3 + 4)

---

## Recent Changes

- **2026-02-22** — Phase 3 planning complete: kanban + technical guide created (22 tasks across 4 sprints)
- **2026-02-22** — Phase 2 gap analysis + benchmark analysis (dcli/arch-config) completed
- **2026-02-22** — Phase 2 remediation: 10 tasks from reviewer findings implemented
- **2026-02-22** — Phase 2 implementation complete: 19/19 tasks across 3 sprints (snapshot/rollback, CLI output, validation/security)
- **2026-02-22** — Phase 2 planning: kanban + technical guide created (19 tasks across 3 sprints)
- **2026-02-22** — Phase 1 implementation complete: apply.rs, drift.rs, template engine, TUI views, CLI commands
- **2026-02-22** — Phase 1 sprint planning: kanban + technical guide created (22 tasks across 4 sprints)
- **2026-02-22** — Phase 0 all tests passing (905 passed, 4 ignored). `iron clean --dry-run` added.
- **2026-02-22** — Phase 0 Sprint 0.2 implemented (F0-007 → F0-012): tech debt closure
- **2026-02-22** — Phase 0 Sprint 0.1 implemented (F0-001 → F0-006): UX quick wins
- **2026-02-22** — Phase 0 planning: product review, user research, kanban + technical guide created
- **2026-02-21** — Clippy cleanup: 32 warnings fixed, committed as `8761e0b`
- **2026-02-21** — Sprint 4 (6 P3 tasks) completed, committed as `1c74ed1`. **All 45/45 S1 tasks done.**
- **2026-02-20** — Hardening sprint documentation audit: all 65 tasks verified against codebase
- **2026-02-20** — Hardening Sprint H2 (29/33 tasks completed)
- **2026-02-20** — Hardening Sprint H1 (28/28 tasks completed)
- **2026-02-20** — Sprint 3 (16 P2 tasks) completed, committed as `ab81806`
- **2026-02-20** — Sprint 2 (16 P1 tasks) completed, committed as `38c1b72`
- **2026-02-19** — Sprint 1 (8 P0 tasks) completed, committed as `6a28dea`
- **2026-02-18** — Scenario 1 phase analysis docs (phases 1–9) created
- **2026-02-18** — Project scaffolded with Mind Agent Framework

## Workflow State
- **Type**: ENHANCEMENT
- **Descriptor**: enhancement-phase3-sprint3.3
- **Last Agent**: none
- **Remaining Chain**: analyst → architect → developer → tester → reviewer
- **Iteration**: docs/iterations/enhancement-phase3-sprint3.3/
