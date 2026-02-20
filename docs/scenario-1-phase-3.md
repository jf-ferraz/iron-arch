# Scenario 1 — Phase 3: Dashboard Overview

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P3-001, S1-P3-002 from `docs/TODO-scenario1.md`
> **Phase**: Dashboard Overview — Divergence Detection & Guidance
> **Generated**: 2026-02-20
> **Based on**: Deep codebase analysis across iron-tui, iron-core, iron-fs, and dependency boundaries

---

## Table of Contents

1. [Phase 3 Architecture Overview](#1-phase-3-architecture-overview)
2. [Task S1-P3-001 — Add Divergence Indicators to Dashboard](#2-task-s1-p3-001)
3. [Task S1-P3-002 — Dashboard Divergence Guidance Tooltip](#3-task-s1-p3-002)
4. [Discovered Issues — Outside Phase 3 Scope](#4-discovered-issues)
5. [Integration Map](#5-integration-map)
6. [Test Coverage Analysis](#6-test-coverage-analysis)

---

## 1. Phase 3 Architecture Overview

### What the Dashboard Is Today

The Dashboard (`crates/iron-tui/src/ui/dashboard.rs`, 367 lines) is Iron's home screen — a
read-only overview with **5 panels** rendered in a 2-column layout (58%/42%):

```
┌── Iron Dashboard ────────────────────────────────────────────────────────┐
│                                                                          │
│  LEFT (58%)                          │  RIGHT (42%)                      │
│  ┌─ System Status ───────────────┐   │  ┌─ Active Configuration ─────┐  │
│  │ [OK] Healthy                  │   │  │ Bundle: hyprland           │  │
│  │ Packages: 1234 installed      │   │  │ Profile: developer         │  │
│  │ Updates: [OK] up to date      │   │  │ Modules: ████░░ 5/8       │  │
│  └───────────────────────────────┘   │  │ Pending: 0 updates        │  │
│  ┌─ Maintenance ─────────────────┐   │  └────────────────────────────┘  │
│  │ Last Update:  2 days ago      │   │  ┌─ Notifications ────────────┐  │
│  │ Last Cleanup: 5 days ago      │   │  │ [!] 3 updates available   │  │
│  └───────────────────────────────┘   │  │ [i] 1 Arch news item      │  │
│  ┌─ Quick Actions ───────────────┐   │  │                            │  │
│  │ [b] Bundles  [p] Profiles     │   │  └────────────────────────────┘  │
│  │ [u] Update   [x] Maintain     │   │                                  │
│  │ [y] Sync     [s] Settings     │   │                                  │
│  └───────────────────────────────┘   │                                  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Dashboard Render Architecture

```
render_dashboard(frame, area, app)                     [dashboard.rs:31]
  │
  ├─ Layout::Horizontal [58%, 42%] with margin(1)
  │    ├─ left_layout:  Vertical [Len(7), Len(6), Min(7)]
  │    └─ right_layout: Vertical [Len(9), Min(5)]
  │
  ├─ render_system_status(frame, left[0], app)         [dashboard.rs:71]
  │    └─ app.system_health() → HealthStatus → [OK]/[!!]/[XX]
  │       app.package_count() → installed count
  │       app.pending_update_count() → update count
  │
  ├─ render_quick_stats(frame, left[1], app)           [dashboard.rs:118]
  │    └─ app.state_manager → maintenance → last_update, last_clean
  │       Color-coded by age: green ≤1d, yellow ≤7d, red >7d
  │
  ├─ render_quick_actions(frame, left[2])              [dashboard.rs:167]
  │    └─ Static keyboard shortcut grid (no data deps)
  │
  ├─ render_active_config(frame, right[0], app)        [dashboard.rs:205]
  │    └─ app.active_bundle → bundle ID
  │       app.active_profile → profile name
  │       app.enabled_module_count() / app.modules.len() → progress bar
  │       app.pending_update_count() → pending count
  │
  └─ render_alerts(frame, right[1], app)               [dashboard.rs:275]
       └─ app.pending_update_count() → update alert
          app.arch_news → requires_manual news count
          Fallback: onboarding nudge OR "[OK] All clear"
```

### Dashboard Data Sources (App Fields)

| App Field | Type | Dashboard Usage |
|-----------|------|----------------|
| `system_health()` | `fn() → HealthStatus` | System status icons |
| `package_count()` | `fn() → usize` | Installed package display |
| `pending_update_count()` | `fn() → usize` | Update badges, alerts |
| `state_manager` | `Option<StateManager>` | Maintenance timestamps |
| `active_bundle` | `Option<Bundle>` | Active config display |
| `active_profile` | `Option<String>` | Active config display |
| `modules` | `Vec<Module>` | Module count, **future divergence source** |
| `active_modules` | `Vec<String>` | Enabled module count |
| `arch_news` | `Vec<ArchNewsItem>` | News alert count |
| `sync_info` | `Option<SyncInfo>` | **Currently unused by dashboard** |

### Dashboard Handler Architecture

**Critical finding**: The Dashboard has **no view-specific key handlers**. All keypresses
fall through to the general handler in `handlers.rs:370-420`:

```rust
// handlers.rs — General key handling (all views)
KeyCode::Char('j') => self.select_next(),       // Does nothing on Dashboard
KeyCode::Char('k') => self.select_previous(),   // Does nothing on Dashboard
KeyCode::Enter     => self.select_item(),        // Does nothing on Dashboard
KeyCode::Char('r') => self.refresh_current_view(),
```

The `select_item()` function (`handlers.rs:762-774`) matches on specific views but has
no branch for `View::Dashboard` — it falls through to the `_ => {}` catch-all.

### What Phase 3 Must Add

Phase 3 introduces **two new capabilities** the dashboard completely lacks:

1. **Divergence detection** — determine which managed modules have drifted from their
   expected state (file content changed, symlink broken, or git working tree dirty)
2. **Divergence guidance** — present users with actionable resolution options when
   they select a diverged item

Both require **new infrastructure** that does not exist anywhere in the codebase today.

---

## 2. Task S1-P3-001

### Add Divergence Indicators to Dashboard

> **ID**: S1-P3-001 | **Priority**: P2 | **Status**: Not started
> **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-core/src/services/sync.rs`
> **Depends**: None

### 2.1 What the Spec Says

From `user-workflow.md` (L334–410), the Dashboard should show:

```
┌─ Health ──────────────┐
│ ● State: OK           │
│ ● Symlinks: OK        │
│ ● Packages: OK        │
│ ▲ Snapshot: Missing   │
│ ● Git: Clean          │
└───────────────────────┘
```

And from `TODO-scenario1.md`:

> "Compare current file hashes against last-known state, show warning icons
> next to diverged modules."

The dashboard should indicate when managed config files have drifted from
their expected state after module activation.

### 2.2 Current State Analysis — Zero Infrastructure Exists

**No file content tracking exists anywhere in the codebase.** Here is what each layer
provides today and what is missing:

| Layer | What Exists | What's Missing |
|-------|-------------|----------------|
| `iron-fs` symlink module | `SymlinkStatus` (Valid/WrongTarget/NotSymlink/Missing) | Content change detection |
| `services/module.rs` | `status()` checks symlink targets | File hash recording on `enable()` |
| `services/sync.rs` | Git-level `SyncStatus` (Ahead/Behind/Diverged/Dirty) | Per-file drift detection |
| `state.rs` (`IronState`) | `active_modules: Vec<String>` | `file_hashes: HashMap` field |
| `Cargo.toml` workspace | No hash crate | Would need `sha2 = "0.10"` |
| `App` struct | No divergence fields | Would need divergence state |

**Key insight — the symlink paradox**: Iron manages dotfiles via **symlinks**. When a
symlink is created by `iron-fs::symlink::create()`, the source and target are the
**same file** (the target is a symlink pointing back to the source in the config repo).
So "divergence" doesn't mean "target differs from source" — it means:

1. **Symlink broken** — target is no longer a symlink, or points elsewhere
   → Already detectable via `SymlinkStatus::WrongTarget` / `NotSymlink` / `Missing`

2. **Content modified** — user edited the config file in place (edits flow through
   the symlink to the repo file, making the git working tree dirty)
   → Detectable via `git status --porcelain` on the config directory

3. **Manual override** — user replaced the symlink with a regular file
   → Already detectable via `SymlinkStatus::NotSymlink`

### 2.3 Architecture Decision: Git-Based vs Hash-Based Detection

Two approaches exist. This section analyzes both:

#### Option A: Custom SHA-256 Hashing

```
Module enable()
  │
  ├─ Create symlinks (existing)
  ├─ Record sha256(file_content) for each dotfile  ← NEW
  └─ Store hashes in IronState.file_hashes         ← NEW

Divergence check
  │
  ├─ For each active module:
  │    ├─ For each dotfile mapping:
  │    │    ├─ sha256(current_content) vs stored hash
  │    │    └─ If mismatch → mark diverged
  └─ Return DivergenceReport
```

**Requires**:
- New crate dependency: `sha2 = "0.10"` in workspace `[dependencies]` and `iron-core/Cargo.toml`
- New `IronState` field: `file_hashes: HashMap<String, HashMap<String, String>>`
  (module_id → file_path → sha256_hex)
- Hash recording in `ModuleService::enable()` (`services/module.rs:199-240`)
- Hash comparison in new `DivergenceService`
- State migration (existing `state.json` files lack the field; `#[serde(default)]` handles this)

**Pros**: Precise per-file detection, works even without git, detects external modifications.
**Cons**: New dependency, state schema change, must re-hash on every check, doesn't leverage
existing git infrastructure.

#### Option B: Git Working Tree Status (Recommended)

```
Divergence check
  │
  ├─ git status --porcelain -- <module_dotfile_paths>
  │    └─ For each active module, check its source files
  │
  ├─ iron-fs::symlink::status() for each dotfile target
  │    └─ Detects broken/wrong symlinks
  │
  └─ Combine: file_changes (git) + link_breaks (symlink) → DivergenceReport
```

**Requires**:
- No new crate dependency (uses existing `std::process::Command` for git, existing `iron-fs`)
- No state schema change (git IS the source of truth)
- New `DivergenceService` in iron-core using `git status --porcelain -- <paths>`
- Dotfile path resolution using `Module::target_paths()` and `DotfileMapping::source`

**Pros**: Zero new dependencies, leverages existing git infrastructure, no state migration,
inherently tracks the Iron repo as source of truth, already has `SyncService` that runs git.
**Cons**: Requires a git repo (but Iron already requires this), slightly slower for many files.

**Recommendation**: **Option B** — It aligns with Iron's existing architecture where the git
repo IS the configuration store. The `SyncService` already runs `git status`, and the symlink
module already checks link integrity. Combining both existing capabilities creates a complete
divergence picture without introducing new dependencies or state schema changes.

### 2.4 Detailed Implementation Plan

#### Step 1: Create `DivergenceInfo` Types in iron-core

**File**: `crates/iron-core/src/services/sync.rs` (extend existing file)

```rust
/// Divergence status for a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDivergence {
    /// Module ID
    pub module_id: String,
    /// Module display name
    pub module_name: String,
    /// Files with content changes (git dirty)
    pub modified_files: Vec<String>,
    /// Symlinks that are broken or wrong-target
    pub broken_links: Vec<BrokenLink>,
    /// Overall status
    pub status: DivergenceStatus,
}

/// A broken or wrong symlink
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenLink {
    /// Expected target path
    pub expected: String,
    /// What was found
    pub actual: LinkState,
}

/// What was found at a symlink location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkState {
    /// Symlink exists but points elsewhere
    WrongTarget(String),
    /// Path exists but is not a symlink
    NotSymlink,
    /// Path does not exist
    Missing,
}

/// Divergence severity for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceStatus {
    /// All files match expected state
    Clean,
    /// Files modified but links intact (user edited configs)
    Modified,
    /// Symlinks broken — module partially broken
    Broken,
}

/// Full divergence report across all active modules
#[derive(Debug, Clone, Default)]
pub struct DivergenceReport {
    /// Per-module divergence info
    pub modules: Vec<ModuleDivergence>,
    /// Timestamp of this check
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

impl DivergenceReport {
    /// Count of modules with any divergence
    pub fn diverged_count(&self) -> usize {
        self.modules.iter()
            .filter(|m| m.status != DivergenceStatus::Clean)
            .count()
    }

    /// Whether any module has broken links
    pub fn has_broken(&self) -> bool {
        self.modules.iter().any(|m| m.status == DivergenceStatus::Broken)
    }
}
```

**Placement rationale**: These types belong in `sync.rs` because divergence detection is
conceptually part of the sync domain — it answers "is my local state in sync with the
managed state?" The TODO also lists `sync.rs` as a target file.

Alternatively, a new `crates/iron-core/src/services/divergence.rs` could house these types
and the service. This is cleaner for separation of concerns but adds a new module. The
implementer should choose based on whether sync.rs is already getting large (717 lines,
mostly tests — the actual service code is ~200 lines, so it can absorb more).

#### Step 2: Create `DivergenceService`

**File**: `crates/iron-core/src/services/sync.rs` or new `divergence.rs`

```rust
impl DefaultSyncService {
    /// Check divergence for all active modules
    pub fn check_divergence(
        &self,
        modules: &[Module],
        active_module_ids: &[String],
    ) -> IronResult<DivergenceReport> {
        let mut report = DivergenceReport {
            modules: Vec::new(),
            checked_at: Utc::now(),
        };

        // Get git-dirty files in one call
        let dirty_files = self.get_dirty_files()?;

        for module in modules {
            if !active_module_ids.contains(&module.id) {
                continue;
            }

            let mut divergence = ModuleDivergence {
                module_id: module.id.clone(),
                module_name: module.name.clone(),
                modified_files: Vec::new(),
                broken_links: Vec::new(),
                status: DivergenceStatus::Clean,
            };

            for dotfile in &module.dotfiles {
                // Check 1: Is the source file dirty in git?
                if dirty_files.contains(&dotfile.source) {
                    divergence.modified_files.push(dotfile.source.clone());
                }

                // Check 2: Is the symlink intact?
                // (uses iron-fs::symlink::status)
                // ... symlink status check ...
            }

            // Determine overall status
            if !divergence.broken_links.is_empty() {
                divergence.status = DivergenceStatus::Broken;
            } else if !divergence.modified_files.is_empty() {
                divergence.status = DivergenceStatus::Modified;
            }

            report.modules.push(divergence);
        }

        Ok(report)
    }

    /// Get list of dirty (modified/untracked) files relative to repo root
    fn get_dirty_files(&self) -> IronResult<Vec<String>> {
        let output = self.git(&["status", "--porcelain"])?;
        let files: Vec<String> = output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l[3..].to_string())  // Strip status prefix "XY "
            .collect();
        Ok(files)
    }
}
```

**Integration with existing code**: `DefaultSyncService` already has a `git()` helper
method (`sync.rs:97`) that runs git commands in the repo root. The new `check_divergence()`
method reuses this infrastructure.

**Module dotfile paths**: `Module::dotfiles` is `Vec<DotfileMapping>` where each mapping has:
- `source: String` — relative path within module directory (e.g., `config/hypr/hyprland.conf`)
- `target: String` — absolute path with `~` expansion (e.g., `~/.config/hypr/hyprland.conf`)

The source paths need to be resolved relative to the module directory to match git paths.
`ModuleService::resolve_dotfiles()` (`services/module.rs:148-159`) handles this resolution.

#### Step 3: Add App State for Divergence

**File**: `crates/iron-tui/src/app/mod.rs`

Add to the `App` struct, in a new section after the existing Sync State block:

```rust
// -------------------------------------------------------------------------
// Divergence State (Phase 3: Dashboard Overview)
// -------------------------------------------------------------------------
/// Cached divergence report from last check
pub divergence_report: Option<DivergenceReport>,
/// Whether to show the divergence guidance popup
pub show_divergence_popup: bool,
/// Selected index within divergence popup module list
pub divergence_selected: usize,
```

And in `Default` impl:

```rust
divergence_report: None,
show_divergence_popup: false,
divergence_selected: 0,
```

Add a convenience method:

```rust
/// Count of modules with divergence issues
pub fn diverged_module_count(&self) -> usize {
    self.divergence_report
        .as_ref()
        .map(|r| r.diverged_count())
        .unwrap_or(0)
}
```

#### Step 4: Trigger Divergence Check

**File**: `crates/iron-tui/src/app/actions.rs`

Divergence should be checked:
1. On `App::init()` after loading modules — inexpensive if few modules are active
2. On `refresh_current_view()` when on the Dashboard
3. After module enable/disable operations

```rust
// In init(), after loading modules and active_modules:
fn check_divergence(&mut self) {
    if let Some(ref sm) = self.state_manager {
        let sync_service = DefaultSyncService::new(
            &self.config_dir,
            sm.clone(),
        );
        match sync_service.check_divergence(&self.modules, &self.active_modules) {
            Ok(report) => self.divergence_report = Some(report),
            Err(_) => self.divergence_report = None, // Silently degrade
        }
    }
}
```

**Error handling**: Divergence check should **never** crash the dashboard. If the
config dir is not a git repo, or if modules have no dotfiles, it should silently
return an empty report. The `git status` call may fail on non-git setups — this
should produce `DivergenceReport::default()`, not an error dialog.

#### Step 5: Render Divergence Indicators on Dashboard

**File**: `crates/iron-tui/src/ui/dashboard.rs`

There are **three natural insertion points** for divergence indicators:

##### 5a. System Status Panel — Health Line

Currently `render_system_status()` (`dashboard.rs:71-115`) shows:
```
[OK] Healthy  All systems operational
```

**Enhancement**: Factor divergence into the `HealthStatus` computation. Currently
`system_health()` in `app/mod.rs:468-478` only considers `update_risk`. It should
also consider divergence:

```rust
pub fn system_health(&self) -> HealthStatus {
    // Existing: check update risk
    if matches!(self.update_risk, RiskLevel::Critical) {
        return HealthStatus::Error;
    }
    // New: check divergence
    if let Some(ref report) = self.divergence_report {
        if report.has_broken() {
            return HealthStatus::Error;
        }
        if report.diverged_count() > 0 {
            return HealthStatus::Warning;
        }
    }
    // Existing: fall through
    match self.update_risk {
        RiskLevel::High => HealthStatus::Warning,
        _ if self.pending_update_count() > 0 => HealthStatus::Warning,
        _ => HealthStatus::Ok,
    }
}
```

##### 5b. Active Configuration Panel — Module Line

Currently `render_active_config()` (`dashboard.rs:205-272`) shows:
```
Modules   ████░░ 5/8
```

**Enhancement**: Add a divergence count / warning suffix:

```
Modules   ████░░ 5/8  ▲ 2 diverged
```

Implementation: After the existing module progress line at L261-268, add:

```rust
if app.diverged_module_count() > 0 {
    content.push(Line::from(vec![
        Span::styled("  Diverged  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            format!("{} modules", app.diverged_module_count()),
            Style::default().fg(theme::YELLOW).bold(),
        ),
        Span::styled(
            "  press [i] for details",
            Style::default().fg(theme::OVERLAY),
        ),
    ]));
}
```

**Note**: The Active Configuration panel height is currently `Constraint::Length(9)`.
Adding the divergence line may require increasing this to `Length(11)` or switching
to a `Min(9)` constraint to accommodate variable content.

##### 5c. Notifications Panel — Divergence Alert

Currently `render_alerts()` (`dashboard.rs:275-367`) shows update alerts and news.
This is the **primary insertion point** — it follows the existing alert pattern:

```rust
// After news_count check, before the all-clear fallback:
let diverged = app.diverged_module_count();
if diverged > 0 {
    if updates > 0 || news_count > 0 {
        content.push(Line::from(""));
    }

    let (icon, color) = if app.divergence_report
        .as_ref()
        .map(|r| r.has_broken())
        .unwrap_or(false)
    {
        ("[X]", theme::RED)
    } else {
        ("[!]", theme::YELLOW)
    };

    content.push(Line::from(vec![
        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
        Span::styled(
            format!("{} module{} diverged from managed state",
                diverged,
                if diverged == 1 { "" } else { "s" }),
            Style::default().fg(color),
        ),
    ]));
    content.push(Line::from(vec![
        Span::styled("      Press ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[i]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" to view divergence details", Style::default().fg(theme::SUBTEXT)),
    ]));
}
```

**Note**: The `has_alerts` variable check (`dashboard.rs:309`) currently only considers
`updates > 0`. It must be updated to also include `diverged > 0`:

```rust
let has_alerts = updates > 0 || diverged > 0;
```

Otherwise the "All clear" message renders even when modules are diverged.

##### 5d. Quick Actions Panel — New Keybinding Hint

Currently `render_quick_actions()` shows 3 rows of shortcuts. Add a 4th row or
integrate into existing rows when divergence is detected. Since the panel uses
`Constraint::Min(7)`, it can grow.

### 2.5 Dashboard Visual Impact (After Implementation)

```
┌── Iron Dashboard ────────────────────────────────────────────────────────┐
│                                                                          │
│  ┌─ System Status ───────────────┐   ┌─ Active Configuration ────────┐  │
│  │ [!!] Attention                │   │ Bundle: hyprland              │  │
│  │   Updates pending + drift     │   │ Profile: developer            │  │
│  │ Packages: 1234 installed      │   │ Modules: ████░░ 5/8          │  │
│  │ Updates:  3 available         │   │ Diverged: 2 modules           │  │
│  └───────────────────────────────┘   │ Pending: 3 updates           │  │
│  ┌─ Maintenance ─────────────────┐   └───────────────────────────────┘  │
│  │ Last Update:  2 days ago      │   ┌─ Notifications ───────────────┐  │
│  │ Last Cleanup: 5 days ago      │   │ [!] 3 package updates         │  │
│  └───────────────────────────────┘   │ [!] 2 modules diverged        │  │
│  ┌─ Quick Actions ───────────────┐   │     Press [i] for details     │  │
│  │ [b] Bundles  [p] Profiles     │   └───────────────────────────────┘  │
│  │ [u] Update   [x] Maintain     │                                      │
│  │ [y] Sync     [s] Settings     │                                      │
│  └───────────────────────────────┘                                      │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2.6 Files Modified (Summary)

| File | Change | Lines Affected |
|------|--------|---------------|
| `crates/iron-core/src/services/sync.rs` | Add `ModuleDivergence`, `DivergenceReport`, `DivergenceStatus` types; add `check_divergence()` and `get_dirty_files()` methods to `DefaultSyncService` | New types (~80 lines), new methods (~60 lines) |
| `crates/iron-tui/src/app/mod.rs` | Add `divergence_report`, `show_divergence_popup`, `divergence_selected` fields; add `diverged_module_count()` method; update `system_health()` to factor in divergence | ~20 new lines, ~10 modified lines |
| `crates/iron-tui/src/app/actions.rs` | Add `check_divergence()` method; call it from `init()` and `refresh_current_view()` | ~20 new lines, ~5 modified lines |
| `crates/iron-tui/src/ui/dashboard.rs` | Add divergence line in `render_active_config()`; add divergence alert in `render_alerts()`; update `has_alerts` check | ~30 new lines, ~3 modified lines |
| `crates/iron-core/src/services/mod.rs` | Re-export new divergence types (if separate module) | ~3 lines |

### 2.7 Edge Cases

1. **No git repo** — `config_dir` is not a git repository. `git status` fails.
   → `check_divergence()` should return `Ok(DivergenceReport::default())` (empty).
   No divergence indicators shown.

2. **No active modules** — `active_modules` is empty.
   → Loop exits immediately, empty report. No divergence indicators.

3. **Module with no dotfiles** — A module that only installs packages, no dotfile mappings.
   → No symlinks to check, no git paths to check. Automatically clean.

4. **Freshly enabled module** — Module just enabled, git working tree is clean.
   → Report shows `DivergenceStatus::Clean`. No indicator.

5. **Config directory has untracked files** — `git status` shows `??` prefix.
   → Untracked files should NOT count as divergence (they're new files, not drift
   from managed state). Filter to `M`/`D`/`R` status prefixes only.

6. **Symlink exists but source deleted from repo** — File removed from module dir
   but symlink still present in home.
   → Git shows as deleted (`D`), symlink shows `Missing` source → `DivergenceStatus::Broken`.

7. **Large number of modules** — Running `git status --porcelain` once and matching
   against module paths is O(n×m) where n = dirty files, m = module dotfile paths.
   For typical setups (10-50 managed files), this is negligible.

---

## 3. Task S1-P3-002

### Dashboard Divergence Guidance Tooltip

> **ID**: S1-P3-002 | **Priority**: P3 | **Status**: Not started
> **Files**: `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-tui/src/app/handlers.rs`
> **Depends**: S1-P3-001

### 3.1 What the Spec Says

From `TODO-scenario1.md`:

> "On selecting a diverged item, show popup with 'restore' / 'accept' / 'diff' options."

This is a **modal popup overlay** — a new dialog type showing divergence details
for a specific module with actionable resolution options.

### 3.2 Existing Overlay System

Iron already has 3 overlay types rendered in `ui/mod.rs:95-107`:

```rust
// Render overlays (from ui/mod.rs)
if app.show_help {
    render_help_overlay(frame, area, app);       // widgets/mod.rs:413-503
}
if app.show_confirm {
    render_confirm_dialog(frame, area, app);     // widgets/mod.rs:506-693
}
if app.progress.is_some() {
    render_progress_dialog(frame, area, app);    // widgets/mod.rs:870-886
}
```

The pattern for overlays is consistent:

```rust
fn render_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(width_pct, height_pct, area);  // widgets/mod.rs:820-825
    frame.render_widget(Clear, popup_area);                        // Clear background
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Title")
        .border_style(Style::default().fg(theme::MAUVE));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    // Render content in `inner`
}
```

### 3.3 Divergence Popup Design

```
┌── Divergence Details ────────────────────────────────────────┐
│                                                               │
│  2 modules have diverged from their managed state:            │
│                                                               │
│  ▸ nvim-ide                                              [!]  │
│    Modified: config/nvim/init.lua                             │
│    Modified: config/nvim/lua/plugins.lua                      │
│                                                               │
│    kitty-dev                                             [!]  │
│    Broken link: ~/.config/kitty/kitty.conf → (missing)        │
│                                                               │
│  ─────────────────────────────────────────────────────────── │
│  Actions for selected module:                                 │
│                                                               │
│  [r] Restore — reset files to managed state (git checkout)    │
│  [a] Accept — commit current changes as new managed state     │
│  [d] Diff — view changes in detail                            │
│                                                               │
│  [j/k] Navigate  [Esc] Close                                 │
└───────────────────────────────────────────────────────────────┘
```

### 3.4 Detailed Implementation Plan

#### Step 1: Add Popup Activation

**File**: `crates/iron-tui/src/app/handlers.rs`

Add a Dashboard-specific handler within the general key dispatch. The current code
at `handlers.rs:370-420` handles all views uniformly. Add a Dashboard-specific block:

```rust
// Before the general navigation block, add:
if self.view == View::Dashboard {
    match key.code {
        // Divergence popup
        KeyCode::Char('i') if self.diverged_module_count() > 0 => {
            self.show_divergence_popup = true;
            self.divergence_selected = 0;
            return;
        }
        _ => {}
    }
}
```

**Why `i`?** — The `[i]` key is not mapped in the general handler and aligns with
the "info/inspect" convention suggested by the notification hint text `"Press [i] for details"`.
It doesn't conflict with any existing Dashboard keybinding.

#### Step 2: Add Popup Navigation Handlers

**File**: `crates/iron-tui/src/app/handlers.rs`

Popup overlays intercept keys before the general handler. Add in `handle_key_event()`
before existing overlay checks:

```rust
// In handle_key_event(), add before show_help check:
if self.show_divergence_popup {
    match key.code {
        KeyCode::Esc => {
            self.show_divergence_popup = false;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = self.divergence_report
                .as_ref()
                .map(|r| r.diverged_count().saturating_sub(1))
                .unwrap_or(0);
            if self.divergence_selected < max {
                self.divergence_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            self.divergence_selected = self.divergence_selected.saturating_sub(1);
        }
        KeyCode::Char('r') => {
            self.restore_diverged_module();
        }
        KeyCode::Char('a') => {
            self.accept_diverged_module();
        }
        KeyCode::Char('d') => {
            self.diff_diverged_module();
        }
        _ => {}
    }
    return; // Consume all input while popup is open
}
```

#### Step 3: Implement Resolution Actions

**File**: `crates/iron-tui/src/app/actions.rs`

Each action maps to a git or filesystem operation:

```rust
/// Restore diverged module to managed state
pub fn restore_diverged_module(&mut self) {
    if let Some(module) = self.selected_diverged_module() {
        // For modified files: git checkout -- <paths>
        // For broken links: re-run symlink creation
        // Show confirm dialog first (destructive action)
        self.confirm_action = Some(ConfirmAction::RestoreModule(module.module_id.clone()));
        self.confirm_style = ConfirmStyle::EnhancedWarning;
        self.show_confirm = true;
        self.show_divergence_popup = false;
    }
}

/// Accept current state as new managed state
pub fn accept_diverged_module(&mut self) {
    if let Some(module) = self.selected_diverged_module() {
        // For modified files: git add + commit
        // Use existing SyncService::commit()
        // ...
    }
}

/// Show diff for diverged module
pub fn diff_diverged_module(&mut self) {
    if let Some(module) = self.selected_diverged_module() {
        // Run git diff -- <paths> and capture output
        // Could show in a new overlay or status message
        // ...
    }
}

/// Get the currently selected diverged module from the report
fn selected_diverged_module(&self) -> Option<&ModuleDivergence> {
    self.divergence_report.as_ref().and_then(|report| {
        report.modules.iter()
            .filter(|m| m.status != DivergenceStatus::Clean)
            .nth(self.divergence_selected)
    })
}
```

**Confirm dialog integration**: The `restore` action should use the existing
risk-differentiated confirm dialog system (implemented in S1-P6-001). Since restore
is a destructive action (overwrites user changes), it warrants `ConfirmStyle::EnhancedWarning`.

**ConfirmAction extension**: The `ConfirmAction` enum (defined in `app/mod.rs`) needs
a new variant:

```rust
pub enum ConfirmAction {
    // ... existing variants ...
    RestoreModule(String),  // module_id
    AcceptModule(String),   // module_id
}
```

#### Step 4: Render the Popup

**File**: `crates/iron-tui/src/ui/dashboard.rs` (or `crates/iron-tui/src/widgets/mod.rs`)

Add a new render function following the overlay pattern:

```rust
/// Render divergence details popup
pub fn render_divergence_popup(frame: &mut Frame, area: Rect, app: &App) {
    let report = match &app.divergence_report {
        Some(r) => r,
        None => return,
    };

    let diverged: Vec<_> = report.modules.iter()
        .filter(|m| m.status != DivergenceStatus::Clean)
        .collect();

    if diverged.is_empty() {
        return;
    }

    // Size: 60% width, dynamic height based on content (capped)
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Divergence Details ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(theme::YELLOW));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} module{} diverged from managed state:",
                diverged.len(),
                if diverged.len() == 1 { " has" } else { "s have" }),
            Style::default().fg(theme::TEXT),
        )),
        Line::from(""),
    ];

    for (i, module) in diverged.iter().enumerate() {
        let is_selected = i == app.divergence_selected;
        let prefix = if is_selected { "  ▸ " } else { "    " };
        let style = if is_selected {
            Style::default().fg(theme::MAUVE).bold()
        } else {
            Style::default().fg(theme::TEXT)
        };

        let status_badge = match module.status {
            DivergenceStatus::Modified => Span::styled(" [!]", Style::default().fg(theme::YELLOW)),
            DivergenceStatus::Broken => Span::styled(" [X]", Style::default().fg(theme::RED)),
            DivergenceStatus::Clean => unreachable!(),
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(&module.module_name, style),
            status_badge,
        ]));

        // Show file details for selected module
        if is_selected {
            for file in &module.modified_files {
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled("Modified: ", Style::default().fg(theme::SUBTEXT)),
                    Span::styled(file, Style::default().fg(theme::YELLOW)),
                ]));
            }
            for link in &module.broken_links {
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled("Broken:   ", Style::default().fg(theme::SUBTEXT)),
                    Span::styled(
                        format!("{}", link.expected),
                        Style::default().fg(theme::RED),
                    ),
                ]));
            }
        }
    }

    // Action hints
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────────",
        Style::default().fg(theme::OVERLAY),
    )));
    lines.push(Line::from(vec![
        Span::styled("  [r]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Restore  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[a]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Accept  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[d]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Diff", Style::default().fg(theme::SUBTEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [j/k]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Navigate  ", Style::default().fg(theme::SUBTEXT)),
        Span::styled("[Esc]", Style::default().fg(theme::MAUVE).bold()),
        Span::styled(" Close", Style::default().fg(theme::SUBTEXT)),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}
```

#### Step 5: Wire Popup into Overlay Rendering

**File**: `crates/iron-tui/src/ui/mod.rs`

Add the divergence popup rendering after existing overlays:

```rust
// After progress dialog check (~L107):
if app.show_divergence_popup {
    dashboard::render_divergence_popup(frame, area, app);
}
```

Import: `render_divergence_popup` needs to be `pub` in `dashboard.rs`.

### 3.5 Files Modified (Summary)

| File | Change | Lines Affected |
|------|--------|---------------|
| `crates/iron-tui/src/app/mod.rs` | Add `ConfirmAction::RestoreModule`, `AcceptModule` variants; add `show_divergence_popup`, `divergence_selected` fields (if not done in S1-P3-001) | ~10 lines |
| `crates/iron-tui/src/app/handlers.rs` | Add divergence popup key intercept (Esc, j/k, r/a/d); add Dashboard `[i]` key for popup activation | ~40 lines |
| `crates/iron-tui/src/app/actions.rs` | Add `restore_diverged_module()`, `accept_diverged_module()`, `diff_diverged_module()`, `selected_diverged_module()` | ~60 lines |
| `crates/iron-tui/src/ui/dashboard.rs` | Add `render_divergence_popup()` function | ~80 lines |
| `crates/iron-tui/src/ui/mod.rs` | Add popup render call in overlay section | ~4 lines |

### 3.6 Action Implementation Details

#### Restore (`r` key)

**Git operation**: `git checkout -- <source_paths>` for modified files.
**Symlink operation**: `iron_fs::symlink::create()` for broken links.
**Risk level**: Destructive — permanently discards user edits. Must show
`ConfirmStyle::EnhancedWarning` dialog before executing.

```
User presses [r]
  → show_divergence_popup = false
  → show_confirm = true, confirm_style = EnhancedWarning
  → confirm_action = RestoreModule(module_id)
  → User confirms → execute git checkout + re-link
  → Refresh divergence check
```

#### Accept (`a` key)

**Git operation**: `git add <source_paths> && git commit -m "Accept changes to <module>"`
**Risk level**: Non-destructive (preserves changes as new truth). Could use
`ConfirmStyle::Simple` dialog.

```
User presses [a]
  → show_divergence_popup = false
  → show_confirm = true, confirm_style = Simple
  → confirm_action = AcceptModule(module_id)
  → User confirms → git add + commit
  → Refresh divergence check (module becomes Clean)
```

#### Diff (`d` key)

**Git operation**: `git diff -- <source_paths>`
**Risk level**: Read-only, no confirmation needed.

This is the least straightforward action — displaying a multi-line diff in a TUI
requires either:
- A new overlay/view with scrolling (significant effort)
- Piping to an external tool like `delta` or `less` (breaks TUI session)
- Showing a summary (e.g., "+5 -3 lines in file.lua") with a hint to use
  `iron diff <module>` in CLI

**Recommendation**: For Phase 3, implement the summary approach (show file names
and line count changes). A full diff viewer can be added in a later phase.

### 3.7 Edge Cases

1. **No diverged modules when popup opens** — If divergence is resolved between
   the last check and popup open, show "No divergence detected" and auto-close.

2. **Restore fails** — git checkout returns error (file locked, permissions).
   → Show `error_message` on the dashboard, keep module as diverged.

3. **Accept on broken link** — Can't `git add` a missing file.
   → Accept should only be offered for `DivergenceStatus::Modified` modules.
   For `Broken` modules, only Restore is valid.

4. **Multiple file changes in one module** — Restore/Accept should cover ALL
   diverged files in the module, not individual files. Per-file granularity
   is a future enhancement.

5. **Concurrent external edits** — User edits a file while the popup is open.
   → Report is stale but benign. Refresh on popup close or on action completion.

---

## 4. Discovered Issues — Outside Phase 3 Scope

### 4.1 SyncStatus::Diverged Naming Confusion

**Finding**: `SyncStatus::Diverged` (`sync.rs:24`) means "git remote has commits
ahead AND behind" — it does NOT mean file content drift. The TODO task S1-P3-001
lists `sync.rs` as a target file and uses the word "divergence" for file drift.

**Risk**: Developers may confuse `SyncStatus::Diverged` (git branch divergence)
with `DivergenceStatus::Modified` (file content drift). The type names must be
clearly distinct.

**Recommendation**: Use `DivergenceStatus` / `ModuleDivergence` / `DivergenceReport`
for file-level drift, keeping `SyncStatus` exclusively for git branch-level status.
Add doc comments explicitly noting the distinction.

**Task**: Could be addressed during S1-P3-001 implementation via doc comments.

### 4.2 `sync_info` Field Unused by Dashboard

**Finding**: `App.sync_info: Option<SyncInfo>` exists (`app/mod.rs`) and is populated
from `SyncService::status()`, but the Dashboard **never reads it**. The System Status
panel could show git sync state (clean/dirty/ahead/behind) alongside the new divergence
indicators.

**Recommendation**: Consider adding a "Git: Clean" / "Git: 3 uncommitted" line to
`render_system_status()`, similar to the spec's Health panel mock-up. This is small
scope and could piggyback on S1-P3-001.

### 4.3 No Dashboard Key Handler Scope

**Finding**: The `handle_key_event()` function handles all views in a single match
block. Dashboard has no dedicated handler — all keys fall through to the global
navigation shortcuts.

**Impact for Phase 3**: The `[i]` key for divergence info must be carefully guarded
with `if self.view == View::Dashboard` to avoid conflicts with other views.

**Recommendation**: Consider refactoring key handling to per-view dispatch functions
(similar to how `handle_wizard_key()` exists for the wizard). This would benefit
multiple phases but is a refactoring task outside Phase 3 scope.

### 4.4 Module `status()` Only Checks Symlinks

**Finding**: `ModuleService::status()` (`services/module.rs:289-314`) determines
`ModuleState` (NotInstalled/Installed/Partial) by checking symlink presence. It does
NOT check content freshness.

**Impact**: Even after Phase 3, `ModuleState::Installed` does not imply "in sync."
A module can be `Installed` (all symlinks valid) but `Diverged` (files modified).
These are orthogonal dimensions that should be displayed independently.

### 4.5 No `last_divergence_check` Timestamp

**Finding**: `MaintenanceState` (`state.rs:197-204`) tracks timestamps for update,
clean, doctor, snapshot, and sync — but not for divergence checks.

**Impact**: The dashboard cannot show "Last drift check: 5 minutes ago" without
adding this field. Since Option B (git-based) doesn't require state persistence,
this is lower priority, but the `DivergenceReport.checked_at` field provides an
in-memory equivalent.

**Recommendation**: Add `last_divergence_check: Option<DateTime<Utc>>` to
`MaintenanceState` if persistent tracking is desired. Otherwise, use
`DivergenceReport.checked_at` for in-session display only.

### 4.6 Potential New Task: `iron drift` CLI Command

**Finding**: The TODO defines TUI-only divergence indicators. There is no equivalent
CLI command. Following the pattern established by other features (update, clean, sync),
a CLI counterpart would be expected:

```
$ iron drift
Checking module divergence...

nvim-ide [!] Modified
  config/nvim/init.lua (modified)
  config/nvim/lua/plugins.lua (modified)

kitty-dev [X] Broken
  ~/.config/kitty/kitty.conf → symlink missing

2 of 5 active modules diverged.

$ iron drift --restore nvim-ide
Restoring nvim-ide to managed state... done.
```

**Recommendation**: Create a new task `S1-P3-003` for CLI parity. Low priority (P3)
since the TUI is the primary interface.

---

## 5. Integration Map

### Data Flow: Divergence Check → Dashboard Render

```
App::init() / refresh_current_view()
  │
  ├─ check_divergence()                               [actions.rs - NEW]
  │    │
  │    ├─ DefaultSyncService::new(config_dir, sm)
  │    │
  │    └─ sync_service.check_divergence(&modules, &active_modules)
  │         │                                          [sync.rs - NEW]
  │         ├─ self.git(&["status", "--porcelain"])
  │         │    └─ Returns: Vec<String> of dirty file paths
  │         │
  │         ├─ For each active module:
  │         │    ├─ Match dotfile.source against dirty_files
  │         │    ├─ iron_fs::symlink::status(target, source)
  │         │    └─ Build ModuleDivergence
  │         │
  │         └─ Return DivergenceReport
  │
  └─ self.divergence_report = Some(report)
       │
       ▼
render_dashboard(frame, area, app)                     [dashboard.rs]
  │
  ├─ render_system_status()
  │    └─ app.system_health() → now considers divergence
  │
  ├─ render_active_config()
  │    └─ app.diverged_module_count() → "N diverged" line
  │
  └─ render_alerts()
       └─ diverged > 0 → "[!] N modules diverged" alert
            └─ "Press [i] for details"
```

### Data Flow: Popup Interaction → Resolution

```
User presses [i] on Dashboard (with diverged modules)
  │
  ├─ app.show_divergence_popup = true                  [handlers.rs]
  │
  ├─ render_divergence_popup(frame, area, app)         [dashboard.rs - NEW]
  │    └─ Reads app.divergence_report
  │       Highlights app.divergence_selected module
  │       Shows file details for selected module
  │
  ├─ User presses [r] (Restore)
  │    ├─ restore_diverged_module()                     [actions.rs - NEW]
  │    │    ├─ Sets ConfirmAction::RestoreModule
  │    │    ├─ Opens EnhancedWarning confirm dialog
  │    │    └─ On confirm:
  │    │         ├─ git checkout -- <paths>             (via SyncService)
  │    │         ├─ iron_fs::symlink::create()          (for broken links)
  │    │         └─ check_divergence()                  (refresh report)
  │    └─ show_divergence_popup = false
  │
  ├─ User presses [a] (Accept)
  │    ├─ accept_diverged_module()                      [actions.rs - NEW]
  │    │    ├─ Sets ConfirmAction::AcceptModule
  │    │    ├─ Opens Simple confirm dialog
  │    │    └─ On confirm:
  │    │         ├─ git add <paths>                     (via SyncService)
  │    │         ├─ git commit -m "..."                 (via SyncService)
  │    │         └─ check_divergence()                  (refresh report)
  │    └─ show_divergence_popup = false
  │
  └─ User presses [Esc]
       └─ show_divergence_popup = false
```

### Cross-Crate Dependencies

```
iron-tui (presentation)
  │
  ├─ uses: DivergenceReport, ModuleDivergence, DivergenceStatus
  │        from iron-core::services::sync (or divergence)
  │
  ├─ uses: Module, DotfileMapping
  │        from iron-core::module
  │
  └─ calls: DefaultSyncService::check_divergence()
            from iron-core::services::sync

iron-core (application logic)
  │
  ├─ uses: iron_fs::symlink::status()
  │        for symlink integrity checks
  │
  └─ uses: std::process::Command (git)
           via existing SyncService::git() helper

iron-fs (infrastructure)
  │
  └─ provides: symlink::status() → SymlinkStatus
               Already exists, no changes needed
```

---

## 6. Test Coverage Analysis

### Existing Dashboard Tests

**File**: `crates/iron-tui/src/ui/tests.rs` (L155-290)

| Test | What it checks |
|------|---------------|
| `test_dashboard_renders_health_ok` | `buffer_contains("Healthy")` |
| `test_dashboard_renders_health_warning` | `buffer_contains("Attention")` |
| `test_dashboard_renders_health_error` | `buffer_contains("Critical")` |
| `test_dashboard_shows_package_count` | `buffer_contains("150"), buffer_contains("installed")` |
| `test_dashboard_shows_active_configuration` | `buffer_contains("hyprland"), buffer_contains("developer")` |
| `test_dashboard_shows_quick_actions` | `buffer_contains("Bundles"), buffer_contains("Profiles")` |
| `test_dashboard_shows_pending_updates_alert` | `buffer_contains("5 package updates")` |
| `test_dashboard_shows_no_alerts_when_empty` | `buffer_contains("get started")` |

All 8 tests use the `render_dashboard() → buffer_contains()` pattern.
None test divergence (it doesn't exist yet).

### Tests Needed for S1-P3-001

| Test | Setup | Assert |
|------|-------|--------|
| `test_dashboard_shows_divergence_alert` | Set `app.divergence_report = Some(report_with_2_modified)` | `buffer_contains("2 modules diverged")` |
| `test_dashboard_no_divergence_when_clean` | Set `app.divergence_report = Some(empty_report)` | `buffer_contains("All clear")` |
| `test_dashboard_health_warning_on_divergence` | Set divergence report with modified modules | `buffer_contains("Attention")` |
| `test_dashboard_health_error_on_broken` | Set divergence report with broken links | `buffer_contains("Critical")` |
| `test_dashboard_shows_diverged_count_in_config` | Set divergence report | `buffer_contains("diverged")` |
| `test_divergence_report_counts` | Create report with mixed clean/diverged | Assert `diverged_count() == N` |
| `test_divergence_report_empty` | Create empty report | Assert `diverged_count() == 0`, `has_broken() == false` |
| `test_check_divergence_no_git_repo` | Config dir without `.git` | Returns empty report, no panic |
| `test_check_divergence_no_active_modules` | Empty `active_modules` | Returns empty report |
| `test_check_divergence_detects_modified` | Create git repo, modify a managed file | Report shows `DivergenceStatus::Modified` |

### Tests Needed for S1-P3-002

| Test | Setup | Assert |
|------|-------|--------|
| `test_divergence_popup_renders` | Set divergence report + `show_divergence_popup = true` | `buffer_contains("Divergence Details")` |
| `test_divergence_popup_shows_module_list` | Set 2 diverged modules | `buffer_contains("nvim-ide"), buffer_contains("kitty-dev")` |
| `test_divergence_popup_navigation` | Send `j` key while popup open | `app.divergence_selected` increments |
| `test_divergence_popup_esc_closes` | Send Esc while popup open | `app.show_divergence_popup == false` |
| `test_divergence_popup_restore_opens_confirm` | Send `r` while popup open | `app.show_confirm == true`, `app.confirm_action == RestoreModule(..)` |
| `test_divergence_popup_accept_only_for_modified` | Select a Broken module, press `a` | No action (Accept not valid for broken links) |
| `test_dashboard_i_key_opens_popup` | On Dashboard with divergence, send `i` | `app.show_divergence_popup == true` |
| `test_dashboard_i_key_no_op_without_divergence` | On Dashboard without divergence, send `i` | `app.show_divergence_popup == false` |

### Test Implementation Pattern

Follow the existing pattern from dashboard tests:

```rust
#[test]
fn test_dashboard_shows_divergence_alert() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();

    // Set up divergence report with 2 modified modules
    app.divergence_report = Some(DivergenceReport {
        modules: vec![
            ModuleDivergence {
                module_id: "nvim-ide".to_string(),
                module_name: "nvim-ide".to_string(),
                modified_files: vec!["config/nvim/init.lua".to_string()],
                broken_links: vec![],
                status: DivergenceStatus::Modified,
            },
            ModuleDivergence {
                module_id: "kitty-dev".to_string(),
                module_name: "kitty-dev".to_string(),
                modified_files: vec![],
                broken_links: vec![BrokenLink {
                    expected: "~/.config/kitty/kitty.conf".to_string(),
                    actual: LinkState::Missing,
                }],
                status: DivergenceStatus::Broken,
            },
        ],
        checked_at: Utc::now(),
    });

    terminal
        .draw(|f| {
            render_dashboard(f, f.area(), &app);
        })
        .unwrap();

    assert!(buffer_contains(&terminal, "diverged"));
}
```

For handler tests, follow the existing pattern:

```rust
#[test]
fn test_dashboard_i_key_opens_divergence_popup() {
    let mut app = App::default();
    app.view = View::Dashboard;
    app.divergence_report = Some(/* report with diverged modules */);

    app.handle_key_event(create_key_event(KeyCode::Char('i')));

    assert!(app.show_divergence_popup);
    assert_eq!(app.divergence_selected, 0);
}
```

### Estimated Test Count

| Area | New Tests |
|------|-----------|
| Divergence types (`DivergenceReport`, `ModuleDivergence`) | 3 |
| `check_divergence()` service method | 5 |
| Dashboard render with divergence | 5 |
| Handler: popup activation, navigation, actions | 8 |
| Popup render | 3 |
| **Total** | **~24 new tests** |

---

## Appendix A: Key Code Locations Reference

| Component | File | Lines | Notes |
|-----------|------|-------|-------|
| Dashboard render | `crates/iron-tui/src/ui/dashboard.rs` | L31-62 | Main layout dispatcher |
| System Status panel | `crates/iron-tui/src/ui/dashboard.rs` | L71-115 | Health icons, package/update counts |
| Maintenance panel | `crates/iron-tui/src/ui/dashboard.rs` | L118-164 | Timestamp age coloring |
| Quick Actions panel | `crates/iron-tui/src/ui/dashboard.rs` | L167-202 | Static key grid |
| Active Config panel | `crates/iron-tui/src/ui/dashboard.rs` | L205-272 | Bundle/profile/modules |
| Notifications panel | `crates/iron-tui/src/ui/dashboard.rs` | L275-367 | Alerts, news, all-clear |
| `App` struct | `crates/iron-tui/src/app/mod.rs` | L36-140 | All app state fields |
| `HealthStatus` enum | `crates/iron-tui/src/app/mod.rs` | L263-269 | Ok/Warning/Error |
| `system_health()` | `crates/iron-tui/src/app/mod.rs` | L468-478 | Health computation |
| General key handler | `crates/iron-tui/src/app/handlers.rs` | L370-420 | All-view keys |
| `select_item()` | `crates/iron-tui/src/app/handlers.rs` | L762-774 | View-specific Enter |
| Overlay rendering | `crates/iron-tui/src/ui/mod.rs` | L95-107 | help, confirm, progress |
| `centered_rect()` | `crates/iron-tui/src/widgets/mod.rs` | L820-825 | Popup positioning |
| `render_confirm_dialog()` | `crates/iron-tui/src/widgets/mod.rs` | L506-693 | 3 risk styles |
| `SyncStatus` enum | `crates/iron-core/src/services/sync.rs` | L14-29 | Git-level status |
| `SyncInfo` struct | `crates/iron-core/src/services/sync.rs` | L32-49 | Branch, commits, dirty |
| `SyncService` trait | `crates/iron-core/src/services/sync.rs` | L52-72 | Git operations |
| `DefaultSyncService::git()` | `crates/iron-core/src/services/sync.rs` | L97 | Git command helper |
| `Module` struct | `crates/iron-core/src/module.rs` | L8-42 | Module data model |
| `DotfileMapping` | `crates/iron-core/src/module.rs` | L64-72 | source→target mapping |
| `ModuleState` enum | `crates/iron-core/src/module.rs` | L75-84 | Install status |
| `ModuleService::enable()` | `crates/iron-core/src/services/module.rs` | L199-240 | No hash recording |
| `ModuleService::status()` | `crates/iron-core/src/services/module.rs` | L289-314 | Symlink-only check |
| `resolve_dotfiles()` | `crates/iron-core/src/services/module.rs` | L148-159 | Path resolution |
| `IronState` | `crates/iron-core/src/state.rs` | L148-181 | Global state |
| `MaintenanceState` | `crates/iron-core/src/state.rs` | L197-204 | Timestamps |
| `SymlinkStatus` enum | `crates/iron-fs/src/lib.rs` | L84-94 | Link health |
| `symlink::status()` | `crates/iron-fs/src/lib.rs` | L165-185 | Link check |
| `symlink::create()` | `crates/iron-fs/src/lib.rs` | L99-141 | Link creation |
| Workspace Cargo.toml | `Cargo.toml` | L1-60 | No hash crate |

## Appendix B: Implementation Order

The recommended implementation sequence:

```
1. Add DivergenceReport types to iron-core (sync.rs or new divergence.rs)
   └─ Pure data types, no dependencies on other changes
   └─ Unit tests for type construction and counting methods

2. Implement check_divergence() in DefaultSyncService
   └─ Depends on: step 1 (types)
   └─ Uses existing git() helper + iron-fs::symlink::status()
   └─ Unit tests with mock git output

3. Add divergence fields to App struct
   └─ Depends on: step 1 (DivergenceReport import)
   └─ Add fields + Default impl + convenience methods

4. Add check_divergence() call to actions.rs
   └─ Depends on: steps 2 + 3
   └─ Wire into init() and refresh_current_view()

5. Render divergence indicators on Dashboard
   └─ Depends on: step 3 (App fields)
   └─ Modify render_alerts(), render_active_config(), system_health()
   └─ Render tests

6. Implement divergence popup render function
   └─ Depends on: step 3 (App fields)
   └─ render_divergence_popup() following overlay pattern

7. Add popup handler and resolution actions
   └─ Depends on: steps 4 + 6
   └─ Key handlers, confirm dialog integration
   └─ Handler tests

8. Wire popup into overlay rendering (ui/mod.rs)
   └─ Depends on: step 6
   └─ One-line addition
```

Steps 1 and 3 can be done in parallel. Steps 5 and 6 can also be parallelized.
