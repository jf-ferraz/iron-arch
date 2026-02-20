# Current State

## Status: Scenario 1 Complete

**Scenario 1 — Newcomer Journey** is **fully implemented**.
All 45/45 tasks across 4 sprints are done. Branch: `feature/tui-enhancement-phase1`.
HEAD: `8761e0b` (clippy cleanup commit after Sprint 4).

---

## Codebase Metrics

| Crate | Lines | Tests | Role |
|---|---|---|---|
| iron-core | 27,456 | 902 | Domain models, services, state |
| iron-tui | 16,199 | 410 | Ratatui TUI (27 views) |
| iron-cli | 4,854 | 69 | Clap CLI |
| iron-fs | 1,969 | 88 | File operations |
| iron-git | 2,317 | 98 | Git/git-crypt |
| iron-pacman | 2,569 | — | Pacman/paru/yay (needs sudo) |
| iron-systemd | 1,889 | — | Systemd (needs sudo) |
| Integration tests | 3,487 | — | Cross-crate tests |
| **Total** | **~60,740** | **1,567** | **0 failed, 8 ignored** |

## Sprint Summary

### Sprint 4 — Polish (P3) — 6 tasks
| Task | Summary |
|---|---|
| S1-P7-003 | Doctor `[r]` refresh key → `refresh_current_view()` |
| S1-P1.5-005 | Scan history: `rescan_system()` + state persistence |
| S1-P2-001 | `HostSelection` TUI view with auto-detect/manual create |
| S1-P2-002 | Host selection wired into wizard flow (auto-selects single host) |
| S1-P2-003 | `iron host select` CLI (pre-existing) |
| S1-P3-002 | Divergence guidance tooltip popup with `[d]` key |

### Sprint 3 — New Features (P2) — 16 tasks
| Task | Summary |
|---|---|
| S1-P1-003 | Setup wizard "Step X of Y" progress indicator |
| S1-P1.5-001→004,006 | `ScanService` + `ScanReport` + `SystemScan` TUI + wizard wiring + `iron scan` CLI |
| S1-P3-001 | Dashboard divergence indicators (`[!!] N diverged` / `[OK] in sync`) |
| S1-P5-001 | ProfileBuilder TOML persistence |
| S1-P5-002 | ModuleCreator TOML + directory persistence |
| S1-P7-001 | `DoctorService` trait unifying TUI and CLI health checks |
| S1-P8-001 | Sync conflict resolution: `[l]` keep-local / `[r]` keep-remote |
| S1-P9-004 | `SecretsBackend` trait consolidating SecretsService + SecretsManager |
| S1-P9-005 | Secrets audit logging via `StateManager` |
| S1-P9-006 | CLI `secrets add-key`, `secrets export-key`, `recover --backup/--restore` |
| S1-XI-001/002 | Scan integration tests + coverage gate |

### Sprint 2 — Core Gaps (P1) — 16 tasks
| Task | Summary |
|---|---|
| S1-P4-003 | `SystemdServiceAdapter` injection across all 6 TUI/wizard sites |
| S1-P4-001 | Dormant directory lifecycle (`archive_to_dormant`/`restore_from_dormant`) |
| S1-P4-002 | Block activation when conflicts detected |
| S1-P4-005 | `switch()` rollback on failure |
| S1-P4-006 | `resolve_dotfiles_dir()` supporting `dotfiles/` and `config/` |
| S1-P5-003/004 | Profile activation + Module enable/disable via service layer |
| S1-P6-002/003 | Update preflight gating + snapshot integration |
| S1-P7-004 | CLI `iron clean` → `DefaultCleanupService` |
| S1-P8-002/003 | Sync push auto-commit + pull dirty-tree stash handling |
| S1-P9-003 | `list_encrypted()` gitattributes pattern matching |
| S1-X-001/002 | Architecture.md + EXAMPLES.md updates |

### Sprint 1 — Critical Fixes (P0) — 8 tasks
| Task | Summary |
|---|---|
| S1-P1-001/005 | Real PackageManager injection (5 TUI sites + wizard) |
| S1-P1-002 | Corrected 20+ `[STUB]` annotations |
| S1-P6-001 | Risk-differentiated confirmation dialogs |
| S1-P4-004 | `clear_active_bundle()` in deactivate |
| S1-P7-002 | Cleanup `dry_run: true` per spec |
| S1-P9-001/002 | Secrets + Recovery view handlers wired |

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

### Active Issues
- **iron-cli clippy**: 1 `too_many_arguments` warning (8 params, threshold 7). Function signature needs restructuring.
- **Integration tests require sudo**: `cargo test` on iron-pacman/iron-systemd prompts for sudo. Use `cargo test -p iron-core -p iron-tui -p iron-cli -p iron-fs -p iron-git` for CI-safe runs.
- **Deferred keybinds**: `[a] Add GPG key` in Secrets (needs text input widget), `[i] Import` / `[r] Recovery wizard` in Recovery (needs file path input widget).
- **Disk space**: Workspace disk was at 100% (191GB). `cargo clean` freed 14.9GB. Build cache needs regeneration (`cargo build` will be slower first time).

### Lower-Priority Guideline Issues (~80 items)
Each scenario-1 phase guideline doc lists additional discovered issues not tracked in Sprint 1–4:

| Guideline | Extra Issues |
|---|---|
| `scenario-1-phase-1.md` | S1-P1-004 through S1-P1-007 (first-launch detection, wizard tests) |
| `scenario-1-phase-2.md` | S1-P2-004 through S1-P2-006 (host TOML creation, config convention) |
| `scenario-1-phase-3.md` | 6 unnumbered issues (naming, unused fields) |
| `scenario-1-phase-4.md` | B1 through B8 (package handling, dormant edge cases) |
| `scenario-1-phase-5.md` | S1-P5-NEW-001 through -013 (CLI create, validation, templates) |
| `scenario-1-phase-6.md` | S1-P6-NEW-001 through -012 (UpdateService wiring, type unification) |
| `scenario-1-phase-7.md` | Additional cleanup/doctor improvements |
| `scenario-1-phase-8.md` | Sync edge cases |
| `scenario-1-phase-9.md` | `[a]` GPG key input, `[i]` import, `[r]` recovery wizard |

These can form the basis of a "Scenario 1 Hardening" or "Scenario 2" sprint.

---

## Recent Changes

- **2026-02-21** — Clippy cleanup: 32 warnings fixed (collapsible_if, map/flatten, range), committed as `8761e0b`
- **2026-02-21** — Sprint 4 (6 P3 tasks) completed, committed as `1c74ed1`. **All 45/45 tasks done.**
- **2026-02-20** — Sprint 3 (16 P2 tasks) completed, committed as `ab81806`
- **2026-02-20** — Sprint 2 (16 P1 tasks) completed, committed as `38c1b72`
- **2026-02-19** — Sprint 1 (8 P0 tasks) completed, committed as `6a28dea`
- **2026-02-18** — Scenario 1 phase analysis docs (phases 1–9) created
- **2026-02-18** — Project scaffolded with Mind Agent Framework
