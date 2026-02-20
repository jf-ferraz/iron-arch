# Current State

## Active Work

**Scenario 1 — Newcomer Journey** implementation in progress.
Sprint 1 (Critical Fixes / P0) is **complete**. Sprint 2 (Core Gaps / P1) is next.

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

## Test Counts (2026-02-19)

| Crate | Tests |
|---|---|
| iron-core | 803 |
| iron-tui | 371 |
| iron-cli (acceptance) | 88 |
| iron-git | 95 |
| iron-fs | 101 |
| iron-pacman | 64 |
| iron-systemd | 39 |
| iron-cli (other) | 31 + 23 + 24 + 11 |
| **Total** | **1,723** (all passing) |

## Known Issues

- **S1-P4-003** (deferred P0): No `with_service_manager()` on any TUI `DefaultBundleService` call site. `iron-systemd` does not implement `iron-core::SystemService` trait — only `NoopSystemService` exists. Deferred to Sprint 2.
- **Pre-existing clippy**: 19 `collapsible_if` warnings in `iron-core/src/snapshot.rs`. 3 pre-existing warnings in `iron-tui` (OR pattern range, map/flatten, useless vec).
- **Deferred keybinds**: `[a] Add GPG key` in Secrets view (needs text input widget), `[i] Import` / `[r] Recovery wizard` in Recovery view (needs file path input widget).

## Next Priority: Sprint 2

Sprint 2 tackles P1 tasks + 1 deferred P0. See `docs/TODO-scenario1.md` for full plan.
Key tasks: service manager adapter, dormant directory mgmt, conflict blocking,
profile/module activation fixes, snapshot integration, sync improvements.
Estimated: ~28h across 16 tasks.

## Recent Changes

- **2026-02-19** — Sprint 1 (8 P0/P1 tasks) completed
- **2026-02-18** — Scenario 1 phase analysis docs (phases 1–9) created
- **2026-02-18** — Project scaffolded with Mind Agent Framework
