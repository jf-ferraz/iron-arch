# Current State

## Active Work

**Scenario 1 — Newcomer Journey** implementation in progress.
Sprint 1 (P0) and Sprint 2 (P1 + deferred P0) are **complete**. Sprint 3 (P2 — New Features) is next.

## Sprint 2 Completed (2026-02-20)

| Task | Summary |
|---|---|
| S1-P4-003 | `SystemdServiceAdapter` in iron-systemd bridging `ServiceManager` → `SystemService`; all 6 TUI/wizard sites chain `.with_service_manager()` |
| S1-P4-001 | Dormant directory management — `archive_to_dormant()`, `restore_from_dormant()`, `move_dir()` with rename-first/copy-delete fallback |
| S1-P4-002 | Block activation when `check_conflicts()` returns active conflicting bundles |
| S1-P4-005 | `switch()` rollback — re-activates original bundle if `activate(to)` fails |
| S1-P4-006 | `resolve_dotfiles_dir()` — supports both `dotfiles/` (legacy) and `config/` (current) |
| S1-P5-003 | Profile activation via `ProfileService::apply()` (replaces `sm.set_active_profile()`) in TUI + wizard |
| S1-P5-004 | Module enable/disable via `ModuleService::enable()`/`disable()` (replaces bare `sm.enable_module()`) |
| S1-P6-002 | Update preflight gating (blocks on `blockers`) + `UpdateService::apply(create_snapshot)` integration |
| S1-P6-003 | `SnapshotManager` blanket impl for `Box<dyn SnapshotManager>`; all 7 `NoopManager` sites → `create_manager()` |
| S1-P7-003 | `View::Doctor` arm in `refresh_current_view()` — re-runs health checks on `r` key |
| S1-P7-004 | Complete rewrite of `iron clean` CLI to use `DefaultCleanupService` (`preview()` + `execute()`) |
| S1-P8-002 | Sync push auto-commits dirty changes before push (`iron: auto-commit N change(s)`) |
| S1-P8-003 | Sync pull stashes dirty tree before pull, restores after (handles both success and failure) |
| S1-P9-003 | `list_encrypted()` filters by `.gitattributes` patterns using `glob_match()` with gitattributes-correct basename matching |
| S1-X-001 | Error handling audit — 0 unwraps in new production code, 0 compiler warnings |
| S1-X-002 | Doc comments pass — all public functions documented |

## Sprint 1 Completed (2026-02-19)

| Task | Summary |
|---|---|
| S1-P1-001 | Injected real `PackageManager` into 4/5 `DefaultBundleService` call sites in TUI actions |
| S1-P1-005 | Fixed wizard `apply()` PM injection + `refresh_current_view()` (5th call site) |
| S1-P1-002 | Corrected 20+ `[STUB]` annotations across `user-workflow.md` |
| S1-P6-001 | Added `ConfirmStyle` enum with risk-differentiated dialogs (Simple/Enhanced/TypedConfirmation) |
| S1-P4-004 | Added `clear_active_bundle()` to `StateManager`, called from `deactivate()` |
| S1-P7-002 | Changed TUI cleanup from `dry_run: false` to `dry_run: true` |
| S1-P9-001 | Wired Secrets view: `View::Secrets` handler arm (i/u/l/r), 4 action methods, auto-refresh |
| S1-P9-002 | Wired Recovery view: `View::Recovery` handler arm (e/g/s), 3 action methods, last_backup from audit |

## Test Counts (2026-02-20)

| Crate | Tests |
|---|---|
| iron-core | 815 |
| iron-tui | 371 |
| iron-cli (acceptance) | 88 |
| iron-git | 101 |
| iron-fs | 95 |
| iron-pacman | — |
| iron-systemd | 69 |
| **Total** | **1,539 lib + 23 doc** (all passing) |

## Known Issues

- **Pre-existing clippy**: 4 warnings in iron-cli (collapsible_if in update.rs, field never read / unused var in test fixtures). Not in Sprint 2 modified files.
- **Integration tests require sudo**: `cargo test` (full) hangs on pacman integration tests. Use `cargo test --lib` for CI-safe runs.
- **Deferred keybinds**: `[a] Add GPG key` in Secrets view (needs text input widget), `[i] Import` / `[r] Recovery wizard` in Recovery view (needs file path input widget).

## Next Priority: Sprint 3

Sprint 3 tackles P2 tasks (new features). See `docs/TODO-scenario1.md` for full plan.
Key tasks: System Scan service + TUI view, Host Selection UI, Dashboard divergence,
DoctorService unification, progress indicator, persistence verification.
Estimated: ~36h across 16 tasks.

## Architecture Notes for Sprint 3

### Dependency Injection Pattern
- `App` holds `package_manager: Arc<dyn PackageManager>` and `service_manager: Arc<dyn SystemService>`
- Both injected via `App::new()`. All `DefaultBundleService` sites chain both `.with_package_manager()` and `.with_service_manager()`
- `WizardState::apply()` takes both as parameters
- CLI wires `iron_pacman::DefaultPackageManager` + `iron_systemd::SystemdServiceAdapter::user()`

### Snapshot Integration
- `iron_core::snapshot::create_manager()` returns `Box<dyn SnapshotManager>` (auto-detects timeshift/snapper/noop)
- Blanket impl allows `Box<dyn SnapshotManager>` wherever `S: SnapshotManager` is required
- Used in: `DefaultUpdateService`, `DefaultRecoveryService`, `refresh_updates()`, `run_post_update_checks()`, `refresh_config_conflicts()`

### Bundle Lifecycle
```
NotInstalled → activate() → Active
Active → deactivate() → Dormant (configs in dormant/<id>/)
Dormant → activate() → Active (configs restored from dormant/<id>/)
Active(A) → switch(A, B) → Active(B)  [rollback to A on failure]
```

### Glob Matching (gitattributes)
- Patterns without `/` → match basename only (e.g., `*.enc` matches `secrets/config.enc`)
- `*` → matches any character except `/`
- `**` → matches any character including `/`

## Recent Changes

- **2026-02-20** — Sprint 2 (16 P1 tasks) completed, committed as `38c1b72`
- **2026-02-19** — Sprint 1 (8 P0/P1 tasks) completed
- **2026-02-18** — Scenario 1 phase analysis docs (phases 1–9) created
- **2026-02-18** — Project scaffolded with Mind Agent Framework
