# Scenario 1 — Phase 7: Maintenance & Cleanup

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P7-001 from `docs/TODO-scenario1.md` + System Cleanup (user-workflow Phase 7)
> **Phase**: System Cleanup categories, preview/execute flow, Doctor TUI ↔ CLI parity
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-core, iron-tui, iron-cli, iron-pacman

---

## Table of Contents

1. [Phase 7 Architecture Overview](#1-phase-7-architecture-overview)
2. [System Cleanup — Deep Dive](#2-system-cleanup--deep-dive)
3. [Task S1-P7-001 — Doctor TUI ↔ CLI Parity Check](#3-task-s1-p7-001)
4. [Discovered Issues — Outside Original Phase 7 Scope](#4-discovered-issues)
5. [Integration Map](#5-integration-map)
6. [Test Coverage Analysis](#6-test-coverage-analysis)

---

## 1. Phase 7 Architecture Overview

### What Phase 7 Covers

Phase 7 spans two distinct subsystems:

1. **System Cleanup** — An 8-category cleanup workflow with preview (size estimates),
   execution (actual file/package removal), and a 3-view TUI flow
   (`CleanSystem` → `CleanupPreview` → `CleanupResults`).
2. **System Doctor** — Health diagnostics that should produce identical results whether
   invoked from TUI or CLI. Currently the two paths are completely independent with
   almost no overlap.

Both are accessible from the `SystemMaintenance` hub (`x` hotkey):

```
┌── System Maintenance ──────────────────────────────────┐
│                                                         │
│  [u] System Update    Preview and run updates            │
│  [c] System Cleanup   Clean caches and orphans           │
│  [d] System Doctor    Health diagnostics                 │
│                                                         │
│  [Esc] Back                                             │
└─────────────────────────────────────────────────────────┘
```

### Key Components

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| CleanupCategory enum | `services/clean.rs` L24 | 16 | 8 cleanup categories |
| CleanupPreview struct | `services/clean.rs` L126 | 12 | Size estimates per category |
| CleanupResult struct | `services/clean.rs` L148 | 15 | Per-category execution result |
| CleanupSummary struct | `services/clean.rs` L200 | 12 | Aggregated results |
| CleanupService trait | `services/clean.rs` L225 | 30 | preview(), execute(), helpers |
| DefaultCleanupService | `services/clean.rs` L261 | 750 | Full impl with 8+8 methods |
| TUI CleanSystem view | `ui/clean.rs` L22 | 220 | Category selection UI |
| TUI CleanupPreview view | `ui/clean.rs` L239 | 70 | Pre-execution detail |
| TUI CleanupResults view | `ui/clean.rs` L312 | 60 | Post-execution report |
| TUI cleanup handlers | `handlers.rs` L152 | 70 | Space/a/s/n/Enter/c keybinds |
| TUI cleanup actions | `actions.rs` L519 | 80 | toggle, preview, execute |
| TUI cleanup state | `mod.rs` L105 | 8 | categories, previews, summary |
| CLI clean command | `commands/clean.rs` | 149 | 3 categories, advisory only |
| CLI doctor command | `commands/doctor.rs` | 561 | 10 health checks + JSON |
| TUI doctor view | `ui/doctor.rs` | 161 | 7 health checks from app state |
| iron-pacman helpers | `lib.rs` L782–835 | 55 | clean_cache(), get_orphans() |

### System Cleanup Data Flow

```
User presses 'c' from Dashboard/SystemMaintenance
    │
    ▼
View::CleanSystem                                   [handlers.rs L152]
    │  Shows 8 categories, 6 pre-selected (safe)
    │  Space → toggle, s → safe, a → all, n → none
    │
    ├─ Enter → preview_cleanup()                    [actions.rs L534]
    │    │  DefaultCleanupService::new()
    │    │  service.preview(&selected_categories)
    │    │    ├─ preview_package_cache()    → scan /var/cache/pacman/pkg
    │    │    ├─ preview_orphan_packages()  → pacman -Qtdq
    │    │    ├─ preview_systemd_journal()  → journalctl --disk-usage
    │    │    ├─ preview_user_cache()       → count ~/.cache files >30d
    │    │    ├─ preview_thumbnails()       → count ~/.cache/thumbnails
    │    │    ├─ preview_app_logs()         → count ~/.local/share *.log
    │    │    ├─ preview_browser_cache()    → scan firefox/chrome cache
    │    │    └─ preview_dev_cache()        → scan npm/yarn/pip/cargo/go
    │    └─ Navigate to View::CleanupPreview
    │
    └─ 'c' → request_confirm(RunCleanup)            [handlers.rs L190]
              │  ConfirmStyle::Simple (always — no risk differentiation)
              │
              ▼
         execute_cleanup()                           [actions.rs L557]
              │  DefaultCleanupService::new()
              │  service.execute(&categories, true)   ← ✅ dry_run=true (S1-P7-002)
              │    ├─ execute_package_cache()   → sudo paccache -rk3
              │    ├─ execute_orphan_packages() → sudo pacman -Rns --noconfirm
              │    ├─ execute_systemd_journal() → sudo journalctl --vacuum-size=100M
              │    ├─ execute_user_cache()      → rm old files in ~/.cache
              │    ├─ execute_thumbnails()      → rm ~/.cache/thumbnails/*
              │    ├─ execute_app_logs()        → rm old *.log in ~/.local/share
              │    ├─ execute_browser_cache()   → rm browser cache dirs
              │    └─ execute_dev_cache()       → rm npm/yarn/pip/cargo/go caches
              └─ Navigate to View::CleanupResults
```

---

## 2. System Cleanup — Deep Dive

### 2.1 CleanupService (iron-core) — FULLY IMPLEMENTED

The core service at `crates/iron-core/src/services/clean.rs` (1,492 lines, 22 tests) is
the most complete subsystem in Phase 7. All 8 categories from the spec are implemented.

**Category enum** at L24:

| Category | Safe? | Description | Preview method | Execute method |
|----------|-------|-------------|----------------|----------------|
| PackageCache | ✅ | Old package versions (keeps 3) | Scan `/var/cache/pacman/pkg` | `sudo paccache -rk3` |
| OrphanPackages | ✅ | Unused dependency packages | `pacman -Qtdq` + 50MB/pkg estimate | `sudo pacman -Rns --noconfirm` |
| SystemdJournal | ✅ | System logs (vacuum to 100MB) | `journalctl --disk-usage` | `sudo journalctl --vacuum-size=100M` |
| UserCache | ✅ | `~/.cache` files older than 30d | Count old files + sizes | Direct `remove_old_files()` |
| Thumbnails | ✅ | `~/.cache/thumbnails` | Count files + sizes | `remove_directory_contents()` |
| AppLogs | ✅ | `~/.local/share` old `.log` files | Count old log files | `remove_log_files()` |
| BrowserCache | ⚠️ | Firefox + Chrome/Chromium | Scan cache dirs | `remove_directory_contents()` |
| DevCache | ⚠️ | npm/yarn/pip/cargo/go | Scan 5 cache dirs | `remove_directory_contents()` per dir |

**Service trait** at L225:

```rust
pub trait CleanupService: Send + Sync {
    fn preview(&self, categories: &[CleanupCategory]) -> Vec<CleanupPreview>;
    fn execute(&self, categories: &[CleanupCategory], dry_run: bool) -> CleanupSummary;
    fn preview_safe(&self) -> Vec<CleanupPreview>;    // default: safe() categories
    fn preview_all(&self) -> Vec<CleanupPreview>;     // default: all() categories
    fn execute_safe(&self, dry_run: bool) -> CleanupSummary;
    fn total_space(&self, categories: &[CleanupCategory]) -> u64;
}
```

**DefaultCleanupService** at L261 — configurable with:
- `cache_max_age_days` (default: 30)
- `journal_max_size_mb` (default: 100)
- `package_cache_keep` (default: 3)

All execute methods have full `dry_run` support — when `dry_run=true`, they return
`[DRY RUN] Would run: ...` messages with estimated sizes without touching the filesystem.

**Helper functions** at L1009–1202: `format_bytes()`, `count_files_and_size()`,
`count_old_files()`, `count_log_files()`, `remove_old_files()`, `remove_log_files()`,
`remove_directory_contents()`, `parse_journal_size()`, `parse_journal_freed()`,
`parse_size_string()`.

### 2.2 TUI Cleanup Views — FULLY IMPLEMENTED

Three views match the spec exactly:

**CleanSystem** (`ui/clean.rs` L22–237):
- 4-panel layout: Header, Categories (table), Summary, Help
- Category table with checkbox [x]/[ ], name, space estimate, details
- Aggressive categories marked with ⚠ in yellow
- Summary shows selected count, total reclaimable, aggressive warning
- Help bar: `[Space] Toggle  [a] All  [s] Safe  [n] None  [Enter] Preview  [c] Clean`

**CleanupPreview** (`ui/clean.rs` L239–306):
- Filtered list of selected categories with bullet points
- Shows name, space estimate, item count, and details per category
- Warnings section at bottom for aggressive categories
- Only keybind: `[c]` to execute (via confirm dialog)

**CleanupResults** (`ui/clean.rs` L312–370):
- Per-category results with ✓/✗ icon
- Success: name + space freed + items cleaned
- Failure: name + error message
- Color-coded (green/red)

**TUI handlers** (`handlers.rs` L152–213):
- CleanSystem: Space (toggle), s (safe), a (all), n (none), Enter (preview), c (clean), j/k (nav)
- CleanupPreview: c (execute)
- CleanupResults: read-only (Esc back handled globally)

**TUI state** (`mod.rs` L105–111):
- `cleanup_categories: Vec<CleanupCategory>` — initialized with `safe()` at L314
- `cleanup_previews: Vec<CleanupPreview>` — populated by `preview_cleanup()`
- `cleanup_summary: Option<CleanupSummary>` — populated by `execute_cleanup()`
- `cleanup_preview_mode: bool` — true until execution

**TUI App helpers** (`mod.rs` L748–809):
- `toggle_cleanup_category()`, `is_cleanup_category_selected()`
- `cleanup_total_space()`, `cleanup_preview_for()`
- `has_cleanup_results()`, `clear_cleanup_state()`, `reset_cleanup_view()`
- `select_safe_cleanup_categories()`, `select_all_cleanup_categories()`, `deselect_all_cleanup_categories()`

### 2.3 CLI Clean Command — DIVERGED FROM CORE SERVICE

The CLI `iron clean` at `crates/iron-cli/src/commands/clean.rs` (149 lines, 0 tests) is
a **completely independent implementation** that does NOT use `DefaultCleanupService`.

**What the CLI does:**

| Subcommand | What it actually does | Uses core service? |
|------------|----------------------|-------------------|
| `--orphans` | Runs `pacman -Qtdq`, **lists** orphans, suggests `sudo pacman -Rns` | ❌ No |
| `--cache` | Counts files in `/var/cache/pacman/pkg`, suggests `paccache -r` | ❌ No |
| `--symlinks` | Finds broken symlinks, **actually removes** them | ❌ No |
| `--all` | Runs all three above | ❌ No |

**Critical gaps:**

1. **Only 3 of 8 categories** — missing SystemdJournal, UserCache, Thumbnails, AppLogs,
   BrowserCache, DevCache.
2. **Advisory-only** for orphans and cache — prints suggestions instead of executing.
   Only symlinks are actually cleaned.
3. **No preview/estimate flow** — no size estimation at all.
4. **No `--dry-run` flag** — despite the core service supporting it.
5. **Duplicates iron-pacman** — CLI's `clean_orphans()` calls `pacman -Qtdq` directly,
   same as `iron_pacman::get_orphans()`. CLI's `clean_cache()` counts files manually,
   similar to `iron_pacman::clean_cache()`.

**Spec mismatch:**

The user-workflow spec lists:
```bash
iron clean                  # Interactive category selection
iron clean --orphans        # Remove orphan packages only
iron clean --cache          # Clean package cache only
iron clean --symlinks       # Fix broken symlinks
iron clean --all            # All safe categories
```

The current CLI matches the flags but not the behavior — `--orphans` and `--cache` should
actually clean (per spec), not just suggest.

---

## 3. Task S1-P7-001

### Doctor TUI ↔ CLI Parity Check

> **Priority**: P2
> **TODO Entry**: "Doctor TUI shows 7 checks; CLI `iron doctor` may have different checks.
> user-workflow says both should show identical results."

### Current State: No Shared Service

There is **no `DoctorService`** in `iron-core/src/services/`. The services module
(`services/mod.rs`) exports: `bundle`, `clean`, `host`, `module`, `profile`, `recovery`,
`secrets`, `state`, `sync`, `update` — but no `doctor`.

Both the CLI and TUI implement their health checks independently.

### CLI Doctor — 10 Checks (commands/doctor.rs, 561 lines, 0 tests)

| # | Check | FR | Method | What it does |
|---|-------|----|--------|--------------|
| 1 | State file valid | FR-10.1 | Read + `serde_json::from_str` | Validates `state.json` |
| 2 | Directory structure | FR-10.5 | `path.exists()` | modules/, profiles/, bundles/, hosts/ |
| 3 | Current host | — | `HostService::load_host()` | Verifies host config loadable |
| 4 | Git status | FR-10.6 | `git status --porcelain` | .git exists, uncommitted changes |
| 5 | External tools | — | `which pacman && which git` | Required tools present |
| 6 | Package installation | FR-10.3 | `pacman -Q` per pkg | All bundle packages installed |
| 7 | Snapshot backend | FR-10.4 | `which timeshift/snapper` | Snapshot tool available |
| 8 | Secrets status | FR-10.7 | Check `.git-crypt/keys/` | git-crypt configured |
| 9 | Symlink integrity | FR-10.2 | `read_link()` + `exists()` | All module symlinks valid |
| 10 | Service availability | NFR-11 | `ServiceAvailability::check()` | Degradation handling |

**Local types** (L26–52): `CheckStatus` (Pass/Warn/Fail), `HealthReport`, `HealthCheck`.
These are defined locally — not reusable by TUI.

**JSON output** (L525–546): Full FR-10.8 compliance — `HealthReport` with `checks`,
`overall`, `timestamp`. Triggered by `--format json` or `output.is_json()`.

**Exit behavior**: Exit code 1 if any check is `fail` (L556).

### TUI Doctor — 7 Checks (ui/doctor.rs, 161 lines, 0 tests)

| # | Check | Method | What it does |
|---|-------|--------|--------------|
| 1 | Host configured | `app.current_host.is_some()` | State check (no disk I/O) |
| 2 | Active bundle | `app.active_bundle.is_some()` | State check |
| 3 | Modules discovered | `!app.modules.is_empty()` | State check |
| 4 | System updates | `app.pending_update_count()` | State check |
| 5 | Arch news | `app.arch_news.iter().filter(requires_manual)` | State check |
| 6 | Active profile | `app.active_profile.is_some()` | State check |
| 7 | Snapshot backend | `app.snapshot_backend` match | Enum check |

**No runtime probes** — every check reads pre-loaded `App` state. No shell commands,
no disk scanning, no package verification.

**UI layout**: Title bar (3 lines) + Checks area (min) + Footer (3 lines with `[r] Re-run`
and `[Esc] Back`).

**Output format**: Text with `[OK]`/`[!!]`/`[ ]` icons, color-coded (green/yellow/pink/overlay).

### Parity Matrix

| Check | CLI | TUI | Notes |
|-------|-----|-----|-------|
| State file valid | ✅ (disk read + parse) | ❌ | |
| Directory structure | ✅ (exists check) | ❌ | |
| Host configured | ✅ (service load) | ✅ (state check) | Different methods |
| Active bundle | ❌ (checked via packages) | ✅ | |
| Modules discovered | ❌ | ✅ | |
| System updates | ❌ | ✅ | |
| Arch news | ❌ | ✅ | |
| Active profile | ❌ | ✅ | |
| Git status | ✅ (git porcelain) | ❌ | |
| External tools | ✅ (which) | ❌ | |
| Package installation | ✅ (pacman -Q) | ❌ | |
| Snapshot backend | ✅ (which) | ✅ (enum) | Different methods |
| Secrets status | ✅ (git-crypt check) | ❌ | |
| Symlink integrity | ✅ (readlink) | ❌ | |
| Service availability | ✅ (ServiceAvailability) | ❌ | |

**Summary**: 15 unique checks across both paths. Only 2 overlap (host, snapshot), and
even those use different validation methods. **Parity is 2/15 (13%)**.

### Broken TUI `[r]` Re-run Key

The TUI Doctor footer renders `[r] Re-run` (at `ui/doctor.rs` L53–55), but there is
**no `View::Doctor` match arm** in the main key handler dispatch at `handlers.rs`.
The `View::Doctor` only appears in:
- `esc_destination()` → Dashboard (L666)
- `tab_destination()` → Dashboard (L692)
- Navigation targets from SystemMaintenance (L226, L247)

Pressing `r` in the Doctor view does nothing.

### Recommended Fix

Create a shared `DoctorService` in `iron-core`:

**Step 1**: Create `crates/iron-core/src/services/doctor.rs`

```rust
pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

pub enum CheckStatus { Pass, Warn, Fail }

pub struct HealthReport {
    pub checks: Vec<HealthCheck>,
    pub overall: CheckStatus,
    pub timestamp: String,
}

pub trait DoctorService: Send + Sync {
    fn run_all_checks(&self) -> HealthReport;
}
```

**Step 2**: Move CLI's 10 checks + TUI's 5 unique checks into `DefaultDoctorService`,
parameterized by what data is available (some checks need runtime probes, others can
use pre-loaded state).

**Step 3**: CLI consumes `DoctorService::run_all_checks()` and renders text/JSON.

**Step 4**: TUI consumes `DoctorService::run_all_checks()` at navigation time,
stores results in `App`, and `build_health_checks()` reads the stored results.

**Step 5**: Add `View::Doctor` match arm in handlers:
- `r` → re-run checks (call `DoctorService::run_all_checks()` again)

### Implementation Complexity

| Step | Effort | Files |
|------|--------|-------|
| Create DoctorService trait + types | Small | `services/doctor.rs`, `services/mod.rs` |
| Move CLI 10 checks into service | Medium | `services/doctor.rs`, `commands/doctor.rs` |
| Add TUI 5 unique checks to service | Small | `services/doctor.rs` |
| Rewire CLI to consume service | Small | `commands/doctor.rs` |
| Rewire TUI to consume service | Medium | `actions.rs`, `mod.rs`, `ui/doctor.rs` |
| Fix `[r]` handler | Trivial | `handlers.rs` |
| Add tests | Medium | `services/doctor.rs` |

---

## 4. Discovered Issues — Outside Original Phase 7 Scope

---

### 4.1 ✅ RESOLVED: TUI Cleanup dry_run=false (S1-P7-002)

> **Fixed 2026-02-19**: Changed `false` to `true` in `execute_cleanup()`. TUI now
> runs cleanup in dry-run mode per spec. 371 tests pass.

**Severity**: ~~High~~ Resolved | **File**: `actions.rs` L580

~~The `execute_cleanup()` method passes `dry_run: false`~~ → Now passes `true`.

---

### 4.2 CLI Clean Does Not Use CleanupService

**Severity**: Medium | **Files**: `commands/clean.rs`, `services/clean.rs`

The CLI `iron clean` is a 149-line ad-hoc implementation that:
1. Only covers 3 of 8 categories
2. Is advisory-only for orphans and cache (prints commands to run)
3. Does not use `DefaultCleanupService` at all
4. Has no preview/estimate flow
5. Has no dry-run support

The core `DefaultCleanupService` (1,492 lines) has everything needed. The CLI should
be rewritten to consume it, matching how the TUI does.

---

### 4.3 iron-pacman Has Unused Cleanup Functions

**Severity**: Low | **Files**: `iron-pacman/lib.rs` L782–835

`iron-pacman` exports:
- `clean_cache(keep_versions: u32)` — runs `paccache -rk{N}`
- `get_orphans()` → `Vec<String>` — runs `pacman -Qtdq`
- `is_cached(package, version)` — checks `/var/cache/pacman/pkg`

None of these are used by `DefaultCleanupService` or the CLI `clean` command.
Both duplicate the same `pacman`/`paccache` calls via direct `std::process::Command`.

**Recommended**: Either `DefaultCleanupService` should use `iron_pacman::clean_cache()`
and `iron_pacman::get_orphans()`, or if that creates an unwanted dependency,
document the duplication.

---

### 4.4 Cleanup Confirmation Always Uses Simple Style

**Severity**: Low | **File**: `mod.rs` L370–378

The `request_confirm()` routing for `RunCleanup`:

```rust
_ => ConfirmStyle::Simple,  // catches RunCleanup (and all others)
```

Aggressive cleanup categories (BrowserCache, DevCache) arguably warrant at least
`ConfirmStyle::EnhancedWarning` since they can break active sessions or require
lengthy re-downloads. Currently there's no risk differentiation for cleanup.

---

### 4.5 No Cleanup Operation Recording in State

**Severity**: Low | **File**: `actions.rs` L557–590

`execute_cleanup()` does not call `state_manager.record_operation()` or
`state_manager.update_maintenance("cleanup")`. Dashboard shows "Last Cleanup: never"
even after running cleanup from TUI.

---

### 4.6 CLI Missing `--journal`, `--user-cache`, `--thumbnails`, `--logs`, `--browser`, `--dev` Flags

**Severity**: Medium | **File**: `commands/clean.rs`, CLI definition in `cli.rs`

The CLI only has `--orphans`, `--cache`, `--symlinks`, `--all`. The spec mentions
`iron clean --all` for "all safe categories" — but the 5 other safe categories
(journal, user cache, thumbnails, app logs) and 2 aggressive categories have no
individual CLI flags.

---

### 4.7 Spec Mentions `iron clean --symlinks` but Service Has No Symlink Category

**Severity**: Low | **Files**: `services/clean.rs`, `commands/clean.rs`

The `CleanupCategory` enum has 8 categories — none of them is "symlinks".
The CLI `--symlinks` flag is handled by a completely separate `clean_symlinks()`
function that uses `ModuleService::discover()` to find broken symlinks.

This is a spec/implementation mismatch: either add a `BrokenSymlinks` category
to `CleanupService`, or document that symlink cleanup is a separate operation.

---

### 4.8 TUI Doctor Checks Are Stale (No Refresh on Navigation)

**Severity**: Low | **File**: `ui/doctor.rs`

The TUI doctor view renders checks from `App` state that was loaded at startup.
If the user fixes an issue (e.g., activates a bundle, acknowledges news) and then
navigates to Doctor, the checks still reflect startup state. There's no `refresh_doctor()`
action that re-runs checks.

Combined with the broken `[r]` key (Issue 3), the Doctor view shows permanently stale data.

---

## 5. Integration Map

### Cleanup Flow — Current Architecture

```
                     ┌─── iron-core ──────────────────────────────────┐
                     │                                                 │
                     │  DefaultCleanupService                          │
                     │    ├─ 8 preview_*() methods (scan/estimate)     │
                     │    └─ 8 execute_*() methods (clean/dry-run)     │
                     │                                                 │
                     │  CleanupCategory enum (8 variants)              │
                     │  CleanupPreview / CleanupResult / CleanupSummary│
                     │                                                 │
                     └─────────┬───────────────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │ USED BY        │                │ NOT USED BY
              ▼                │                ▼
     ┌─── iron-tui ───┐       │       ┌─── iron-cli ───┐
     │                 │       │       │                 │
     │ preview_cleanup │       │       │ clean_orphans() │ ← ad-hoc
     │   → service     │       │       │ clean_cache()   │ ← ad-hoc
     │     .preview()  │       │       │ clean_symlinks()│ ← ad-hoc
     │                 │       │       │                 │
     │ execute_cleanup │       │       │ NO CALLS to     │
     │   → service     │       │       │ CleanupService  │
     │     .execute()  │       │       │                 │
     │   dry_run=true  │       │       └─────────────────┘
     │   ✅ per spec   │       │       ┌─── iron-pacman ──┐
     └─────────────────┘       │       │                   │
                               │       │ clean_cache() ◄── NOT USED
                               │       │ get_orphans() ◄── NOT USED
                               │       │ is_cached()   ◄── NOT USED
                               │       └───────────────────┘
                               │
                    ┌──────────┘
                    │  DUPLICATED LOGIC
                    │
     iron-core execute_package_cache()  →  paccache -rk3
     iron-pacman::clean_cache()         →  paccache -rk{N}
                    │
     iron-core execute_orphan_packages()→  pacman -Qtdq + -Rns
     iron-pacman::get_orphans()         →  pacman -Qtdq
     CLI clean_orphans()                →  pacman -Qtdq
```

### Doctor Flow — Current Architecture

```
     ┌─── iron-cli ────────────────────┐    ┌─── iron-tui ──────────────┐
     │                                  │    │                            │
     │  commands/doctor.rs              │    │  ui/doctor.rs              │
     │                                  │    │                            │
     │  Local types:                    │    │  build_health_checks()     │
     │    CheckStatus (Pass/Warn/Fail)  │    │    reads App state fields  │
     │    HealthReport                  │    │                            │
     │    HealthCheck                   │    │  7 checks:                 │
     │                                  │    │    host, bundle, modules,  │
     │  10 checks (runtime probes):     │    │    updates, news, profile, │
     │    state_file, directories,      │    │    snapshot                │
     │    host, git, tools, packages,   │    │                            │
     │    snapshot, secrets, symlinks,   │    │  No runtime probes        │
     │    services                      │    │  No shared types           │
     │                                  │    │  No JSON output            │
     │  FR-10.8 JSON output ✅          │    │  [r] Re-run ❌ BROKEN     │
     │                                  │    │                            │
     └────────────┬─────────────────────┘    └─────────┬──────────────────┘
                  │                                     │
                  │         NO SHARED SERVICE            │
                  │      ┌──────────────────┐            │
                  └──────┤  ❌ MISSING       ├───────────┘
                         │  DoctorService    │
                         │  services/doctor.rs│
                         └──────────────────┘
```

### Proposed Architecture

```
     ┌─── iron-core ──────────────────────────────────────────────┐
     │                                                             │
     │  services/doctor.rs (NEW)                                   │
     │                                                             │
     │  pub struct HealthCheck { name, status, message }           │
     │  pub enum CheckStatus { Pass, Warn, Fail }                 │
     │  pub struct HealthReport { checks, overall, timestamp }     │
     │                                                             │
     │  pub trait DoctorService: Send + Sync {                     │
     │    fn run_all_checks(&self) -> HealthReport;                │
     │    fn run_check(&self, name: &str) -> Option<HealthCheck>;  │
     │  }                                                          │
     │                                                             │
     │  DefaultDoctorService {                                     │
     │    state_manager, root_path, ...                            │
     │    15 unified checks (all from CLI + TUI)                   │
     │  }                                                          │
     │                                                             │
     └─────────────────────┬───────────────────────────────────────┘
                           │
              ┌────────────┼────────────────┐
              ▼                              ▼
     ┌─── iron-cli ───┐            ┌─── iron-tui ──────────────────┐
     │                 │            │                                │
     │ doctor_service  │            │ On navigate to View::Doctor:   │
     │  .run_all()     │            │   doctor_service.run_all()     │
     │  → HealthReport │            │   → store in App.doctor_report │
     │  → text / JSON  │            │   → render from stored report  │
     │                 │            │                                │
     └─────────────────┘            │ On 'r' key:                    │
                                    │   re-run doctor_service        │
                                    │   → refresh App.doctor_report  │
                                    └────────────────────────────────┘
```

---

## 6. Test Coverage Analysis

### Existing Test Counts

| File | #[test] | Coverage Notes |
|------|---------|----------------|
| `services/clean.rs` | **22** | Categories, formatting, parsing, results, summary, construction |
| `ui/clean.rs` | **3** | Render-no-panic only (no action/toggle tests) |
| `ui/doctor.rs` | **0** | No tests |
| `commands/clean.rs` | **0** | No tests |
| `commands/doctor.rs` | **0** | No tests |

**Total Phase 7–related: 25 tests**

### What's Well Tested

1. **CleanupCategory** — enum methods (safe/aggressive/all, name/description/id)
2. **format_bytes / parse_size_string / parse_journal_size / parse_journal_freed** — formatting/parsing
3. **CleanupResult / CleanupSummary** — creation, aggregation, space formatting
4. **DefaultCleanupService** — construction, settings, empty category handling

### What's Untested

1. **All execute methods** — never tested (would need mocked filesystem/commands)
2. **All preview methods** — never tested (need real or mocked `/var/cache`, `~/.cache`, etc.)
3. **TUI cleanup actions** — toggle, preview, execute never tested
4. **TUI cleanup handlers** — keybind dispatch never tested
5. **CLI clean** — zero tests
6. **CLI doctor** — zero tests
7. **TUI doctor** — zero tests (neither render nor `build_health_checks()`)

### Tests Needed for Cleanup Fixes

| Test | For Issue | Description |
|------|-----------|-------------|
| `test_tui_executes_dry_run` | 4.1 | TUI `execute_cleanup()` should pass `dry_run=true` |
| `test_cli_uses_cleanup_service` | 4.2 | CLI `iron clean` consumes `DefaultCleanupService` |
| `test_cleanup_all_8_categories_in_cli` | 4.2 | All categories accessible via CLI flags |
| `test_aggressive_cleanup_enhanced_confirm` | 4.4 | Aggressive categories get `EnhancedWarning` |
| `test_cleanup_records_operation` | 4.5 | State updated after cleanup |

### Tests Needed for Doctor (S1-P7-001)

| Test | Description |
|------|-------------|
| `test_doctor_service_runs_all_checks` | All 15 checks produce HealthReport |
| `test_doctor_service_pass_scenario` | All healthy → overall Pass |
| `test_doctor_service_warn_scenario` | Some warnings → overall Warn |
| `test_doctor_service_fail_scenario` | Any failure → overall Fail |
| `test_cli_doctor_uses_service` | CLI consumes DoctorService |
| `test_tui_doctor_uses_service` | TUI consumes DoctorService |
| `test_doctor_json_output` | FR-10.8 JSON matches schema |
| `test_doctor_rerun_key` | `r` key re-runs checks |

---

## Appendix A: Key File Reference

| File | Lines | Purpose |
|------|-------|---------|
| `crates/iron-core/src/services/clean.rs` | 1,492 | CleanupService: 8-category preview + execute + dry-run |
| `crates/iron-core/src/services/mod.rs` | ~30 | Service re-exports (no doctor module) |
| `crates/iron-core/src/availability.rs` | ~350 | ServiceAvailability::check() used by CLI doctor |
| `crates/iron-tui/src/ui/clean.rs` | 416 | 3 cleanup views (CleanSystem, Preview, Results) |
| `crates/iron-tui/src/ui/doctor.rs` | 161 | 7-check doctor view from App state |
| `crates/iron-tui/src/app/mod.rs` | 809 | Cleanup state fields, View enum, App helpers |
| `crates/iron-tui/src/app/handlers.rs` | 1,590 | CleanSystem/Preview/Results handlers, no Doctor handler |
| `crates/iron-tui/src/app/actions.rs` | 1,511 | preview_cleanup(), execute_cleanup() |
| `crates/iron-cli/src/commands/clean.rs` | 149 | Ad-hoc CLI: orphans, cache (advisory), symlinks |
| `crates/iron-cli/src/commands/doctor.rs` | 561 | 10 runtime health checks + JSON output |
| `crates/iron-pacman/src/lib.rs` | 1,830 | clean_cache(), get_orphans() (unused) |

## Appendix B: Summary of Actions Required

### Task Status

| Task | Status | Action |
|------|--------|--------|
| **S1-P7-001** | ❌ OPEN | Extract shared DoctorService, rewire CLI + TUI, fix `[r]` key |

### New Tasks to File

| ID | Priority | Title | Category |
|----|----------|-------|----------|
| S1-P7-NEW-001 | ~~**P0**~~ ✅ | ~~Fix TUI cleanup `dry_run=false` → `true` per spec~~ | ✅ Completed (S1-P7-002) |
| S1-P7-NEW-002 | **P1** | Create `DoctorService` in iron-core with 15 unified checks | Feature (S1-P7-001) |
| S1-P7-NEW-003 | **P1** | Rewire CLI `iron clean` to use `DefaultCleanupService` | Feature (Issue 4.2) |
| S1-P7-NEW-004 | **P1** | Fix TUI Doctor `[r]` re-run key handler | Bug (Issue 3) |
| S1-P7-NEW-005 | **P2** | Add missing CLI clean flags (journal, cache, logs, etc.) | Feature (Issue 4.6) |
| S1-P7-NEW-006 | **P2** | Rewire CLI doctor to consume `DoctorService` | Feature (S1-P7-001) |
| S1-P7-NEW-007 | **P2** | Rewire TUI doctor to consume `DoctorService` | Feature (S1-P7-001) |
| S1-P7-NEW-008 | **P2** | Add enhanced confirm for aggressive cleanup categories | UX (Issue 4.4) |
| S1-P7-NEW-009 | **P3** | Record cleanup operation in state | Feature (Issue 4.5) |
| S1-P7-NEW-010 | **P3** | Use `iron_pacman::clean_cache()`/`get_orphans()` in service | Cleanup (Issue 4.3) |
| S1-P7-NEW-011 | **P3** | Add `BrokenSymlinks` category to CleanupService or document split | Design (Issue 4.7) |
| S1-P7-NEW-012 | **P2** | Add cleanup service tests (preview + execute with mocks) | Testing |
| S1-P7-NEW-013 | **P2** | Add CLI clean + CLI doctor tests | Testing |
| S1-P7-NEW-014 | **P3** | Add TUI doctor refresh on navigation | UX (Issue 4.8) |

### Implementation Order

```
S1-P7-NEW-001 (dry_run fix) ◄── ✅ DONE (2026-02-19)

S1-P7-NEW-002 (DoctorService)
    ├─► S1-P7-NEW-006 (CLI doctor rewire)
    ├─► S1-P7-NEW-007 (TUI doctor rewire)
    │       └─► S1-P7-NEW-004 (fix [r] key)
    │       └─► S1-P7-NEW-014 (refresh on nav)
    └─► S1-P7-NEW-013 (doctor tests)

S1-P7-NEW-003 (CLI clean rewire)
    ├─► S1-P7-NEW-005 (add CLI flags)
    ├─► S1-P7-NEW-010 (iron-pacman integration)
    └─► S1-P7-NEW-012 (cleanup tests)

S1-P7-NEW-008 (aggressive confirm) ── independent
S1-P7-NEW-009 (state recording)    ── independent
S1-P7-NEW-011 (symlinks decision)  ── independent
```

### Quick Win — ✅ dry_run Fix (Completed)

> **Completed 2026-02-19 (S1-P7-002)**. The one-character change was applied:
> `service.execute(&self.cleanup_categories, true)` — TUI is now spec-compliant.

~~The highest-severity finding is the spec violation where TUI executes cleanup
with `dry_run=false`.~~ Users who want actual
cleanup should use `iron clean --all` via CLI, per the spec.
