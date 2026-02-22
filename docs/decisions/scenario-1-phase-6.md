# Scenario 1 — Phase 6: System Updates

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P6-001, S1-P6-002, S1-P6-003 from `docs/TODO-scenario1.md`
> **Phase**: System Updates — Risk Assessment, Confirmation UX, Snapshot Integration
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-core, iron-tui, iron-cli, iron-pacman, and integration boundaries

---

## Table of Contents

1. [Phase 6 Architecture Overview](#1-phase-6-architecture-overview)
2. [Task S1-P6-001 — Risk-Differentiated Confirmation Dialogs (COMPLETED)](#2-task-s1-p6-001)
3. [Task S1-P6-002 — Confirm TUI Update Behavior (DECISION)](#3-task-s1-p6-002)
4. [Task S1-P6-003 — Pre-Update Snapshot Integration](#4-task-s1-p6-003)
5. [Discovered Issues — Outside Original Phase 6 Scope](#5-discovered-issues)
6. [Integration Map](#6-integration-map)
7. [Test Coverage Analysis](#7-test-coverage-analysis)

---

## 1. Phase 6 Architecture Overview

### What Phase 6 Covers

Phase 6 is the update workflow — the feature Iron was fundamentally built to make safe.
It encompasses: pre-flight checks, risk assessment, Arch news integration, confirmation
UX, update execution, progress tracking, snapshot integration, and post-update checks.

### The Update Data Flow

```
User presses 'u' from Dashboard
    │
    ▼
Navigate to View::UpdatePreview
    │
    ▼
refresh_updates()                             [actions.rs L218]
    ├─ package_manager.check_updates()         → pending_updates
    ├─ package_manager.fetch_news()            → arch_news
    ├─ DefaultUpdateService::run_preflight_checks_with_news()
    │    ├─ check_network()                    → curl archlinux.org
    │    ├─ check_disk_space()                 → df /var (min 2GB)
    │    ├─ check_battery()                    → /sys/class/power_supply
    │    ├─ check_pacman_lock()                → /var/lib/pacman/db.lck
    │    ├─ check_time_sync()                  → timedatectl NTPSynchronized
    │    └─ check_news()                       → cross-ref with state acks
    └─ assess_risk()                           → update_risk (Low..Critical)

User presses 'u' from UpdatePreview
    │
    ▼
can_proceed_with_update()?                    [mod.rs L523]
    ├─ No → "Cannot update - resolve pre-flight issues first"
    └─ Yes → request_confirm(RunUpdate)

request_confirm(RunUpdate)                     [mod.rs L370]
    │  match update_risk:
    │    Critical → ConfirmStyle::TypedConfirmation  ("CONFIRM")
    │    High     → ConfirmStyle::EnhancedWarning    (Y/N + warning)
    │    _        → ConfirmStyle::Simple             (Y/N)
    │
    ▼
User confirms
    │
    ▼
execute_confirm_action() → run_system_update() [actions.rs L450]
    │
    ├─ package_manager.upgrade(false)          ← {aur_helper} -Syu --noconfirm
    │    (NO snapshot, NO progress tracking, NO UpdateService)
    │
    └─ run_post_update_checks()                [actions.rs L479]
         ├─ find_config_conflicts()            → .pacnew/.pacsave
         ├─ check_reboot_required()            → kernel/systemd/glibc
         └─ find_failed_services()             → systemctl --failed
```

### Key Components

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| UpdateService trait + impl | `services/update.rs` | 3,440 | Full update logic with progress tracking |
| RiskLevel enum | `packages.rs` L12 | 25 | Low/Medium/High/Critical (TUI uses this) |
| UpdateRisk enum | `services/update.rs` L146 | 15 | Low/Medium/High/Critical (service uses this) |
| assess_risk() | `packages.rs` L232 | 80 | Global risk from packages + news |
| PacmanOutputParser | `services/update.rs` L63 | 78 | Streaming pacman stdout parser |
| PreflightResult | `services/update.rs` L250 | 100 | Pre-flight check results |
| PostUpdateResult | `services/update.rs` L368 | 60 | .pacnew + reboot + failed services |
| SavedUpdatePlan | `state.rs` L56 | 18 | Persisted plan for resume |
| UpdateProgress | `state.rs` L77 | 20 | Progress tracking with phases |
| SnapshotManager trait | `snapshot.rs` L62 | 20 | create/list/delete/restore |
| TimeshiftManager | `snapshot.rs` L104 | 100 | Timeshift integration |
| SnapperManager | `snapshot.rs` L208 | 100 | Snapper integration |
| DefaultPackageManager | `iron-pacman/lib.rs` | 1,830 | check_updates, upgrade, fetch_news |
| TUI UpdatePreview view | `ui/update.rs` | 472 | 4-section render |
| TUI actions | `app/actions.rs` | 1,511 | refresh_updates, run_system_update |
| CLI update command | `commands/update.rs` | 519 | Full CLI with all flags |

### Update Preview TUI Layout

```
┌── System Update ──────────────────────────────────────────────────────────┐
│ Updates: 12 package(s)  │  Risk: ⚠ Review recommended                    │
│ [r] Refresh  [u] Update  [Tab] Section  [Esc] Back                       │
├── Pre-flight Checks ─────────────────────────────────────────────────────┤
│ ✓ Network - Network connectivity OK                                      │
│ ✓ Disk Space - 45GB available on /var                                    │
│ ✓ Power - On AC power                                                    │
│ ✓ Pacman Lock - Pacman database is available                             │
│ ✓ Time Sync - System time is synchronized                                │
│ ✓ Arch News - All news acknowledged                                      │
├── Arch News (all acknowledged) ──────────────────────────────────────────┤
│ ✓ All news acknowledged                                                  │
├── Packages (12) ─────────────────────────────────────────────────────────┤
│ ! linux           6.12.1 → 6.12.2  (kernel — red)                       │
│ ! systemd         256.1 → 256.2    (system-critical — peach)             │
│ ~ mesa            24.3.1 → 24.3.2  (graphics — yellow)                  │
│   waybar          0.10.3 → 0.10.4  (normal — text)                      │
│   neovim          0.10.2 → 0.10.3  (normal — text)                      │
└──────────────────────────────────────────────────────────────────────────┘
```

Rendered by `render_update_preview()` at `ui/update.rs` L71 with four sub-functions:
- `render_header_section()` L110 — summary, reboot warning, key hints
- `render_preflight_section()` L180 — check list with ✓/⚠/✗/○ indicators
- `render_news_section()` L219 — unacknowledged items, [a]/[A] acknowledgment
- `render_packages_section()` L308 — risk-colored list, scrollbar, max 50 shown

Section navigation via `h/l` or `Tab` cycles between `PreflightChecks`, `News`, `Packages`
(`UpdateSection` enum). Item navigation `j/k` within each section.

---

## 2. Task S1-P6-001

### Risk-Differentiated Confirmation Dialogs — COMPLETED

> **Status**: ✅ Completed 2026-02-19
> **TODO Entry**: "All update risk levels use the same Y/N dialog. user-workflow specifies
> typed confirmation for CRITICAL and enhanced warnings for HIGH."

### What Was Implemented

**`ConfirmStyle` enum** at `app/mod.rs` L253:

```rust
pub enum ConfirmStyle {
    Simple,              // Y/N (Low/Medium risk)
    EnhancedWarning,     // Y/N with prominent risk display (High)
    TypedConfirmation,   // Must type "CONFIRM" (Critical)
}
```

**Risk → Style routing** at `app/mod.rs` L370:

```rust
ConfirmAction::RunUpdate => match self.update_risk {
    RiskLevel::Critical => ConfirmStyle::TypedConfirmation,
    RiskLevel::High     => ConfirmStyle::EnhancedWarning,
    _                   => ConfirmStyle::Simple,
},
```

**Handler logic** at `handlers.rs` L30–74:

- **TypedConfirmation** (L33–53): Accumulates chars into `confirm_typed_input`, Backspace
  deletes, Enter checks `== "CONFIRM"` (exact match), Esc cancels. Non-matching Enter is
  silently ignored (no dismiss).
- **EnhancedWarning / Simple** (L54–73): `y` or Enter → execute, `n` or Esc → cancel.

**10 dedicated tests** at `handlers.rs` L1245+:
- Typed confirmation correct/wrong/escape/backspace
- Enhanced warning Y/N/Esc
- Simple Y/N
- Risk routing (Critical→Typed, High→Enhanced, Low→Simple)

### Verification: ✅ Complete

The implementation matches the spec exactly. No further action needed.

---

## 3. Task S1-P6-002

### DECISION: Confirm TUI Update Behavior

> **Priority**: P1
> **TODO Entry**: "`run_system_update()` calls `package_manager.upgrade(false)` which runs
> `sudo pacman -Syu --noconfirm`. user-workflow implies previewing first."
>
> **Options**: (A) Keep real updates with risk-differentiated confirmation gating.
> (B) Add dry-run flag, show diff, require second confirmation.

### Current Implementation

**TUI path** — `run_system_update()` at `actions.rs` L450–478:

```rust
pub fn run_system_update(&mut self) {
    self.set_info("Running system update...");
    let package_names: Vec<String> = self.pending_updates
        .iter().map(|p| p.name.clone()).collect();

    // Execute the real upgrade (preview=false → actually install)
    match self.package_manager.upgrade(false) { // ← runs {aur_helper} -Syu --noconfirm
        Ok(_preview) => {
            self.set_status("System update completed successfully");
        }
        Err(e) => {
            self.set_error(format!("System update failed: {}", e));
            return;
        }
    }

    self.run_post_update_checks(&package_names);
}
```

This calls `DefaultPackageManager::upgrade(false)` at `iron-pacman/lib.rs` L429–459:

```rust
fn upgrade(&self, preview: bool) -> IronResult<UpdatePreview> {
    let updates = self.check_updates()?;
    let news = fetch_arch_news()?;
    let (risk_level, risk_reasons) = assess_risk(&updates, &news);

    let preview_result = UpdatePreview { ... };

    if !preview && !self.dry_run {
        let cmd = self.aur_helper.command();
        Command::new(cmd)
            .args(["-Syu", "--noconfirm"])
            .status()?;
    }

    Ok(preview_result)
}
```

**CLI path** — `commands/update.rs` L48–300:
Uses `update_service.apply_with_progress()` which goes through `DefaultUpdateService`:
- Creates snapshot if appropriate
- Runs `sudo pacman -Syu --noconfirm` with streaming output parser
- Tracks progress via `UpdateProgress` persisted to state JSON
- Supports `--resume` for interrupted updates

### Analysis: Why This Matters

The TUI and CLI paths are **fundamentally different**:

| Aspect | TUI (`run_system_update`) | CLI (`apply_with_progress`) |
|--------|---------------------------|------------------------------|
| Entry point | `PackageManager::upgrade()` | `UpdateService::apply_with_progress()` |
| Snapshot | ❌ Never created | ✅ Created if recommended |
| Progress tracking | ❌ None | ✅ Full (phase, packages, resume) |
| Resume support | ❌ None | ✅ `--resume` flag |
| Pre-flight checks | ✅ Gates the `[u]` button | ❌ None |
| News check | ✅ Blocks if `requires_manual` | ❌ None |
| Post-update checks | ✅ Full (.pacnew, reboot, services) | ⚠️ Basic (.pacnew count only) |
| AUR helper | ✅ Uses detected helper | ❌ Uses raw `pacman` |

**Neither path has everything.** The ideal path would combine:
- TUI's pre-flight checks + news gating
- CLI's progress tracking + snapshot + resume

### Recommendation

**Option A is correct** (keep real updates with risk-differentiated confirmation) but
the implementation needs work:

1. **Replace `package_manager.upgrade(false)` with `update_service.apply_with_progress()`**
   in the TUI path. This gives progress tracking, snapshot creation, and resume for free.
2. The TUI already has pre-flight gating and news blocking — those are correctly placed
   before the confirm dialog.
3. The `ConfirmStyle` routing (S1-P6-001) already prevents accidental execution.

This task is really about fixing the **service bypass** — the TUI should use the same
code path as the CLI for actually executing the update. This also directly enables
S1-P6-003 (snapshot integration).

### What Changes Are Needed

**Primary change**: In `actions.rs`, replace `run_system_update()` to use
`DefaultUpdateService::apply_with_progress()`:

```
run_system_update() [CURRENT]
    └─ package_manager.upgrade(false)  ← bypasses everything

run_system_update() [PROPOSED]
    ├─ Construct DefaultUpdateService with real SnapshotManager
    ├─ Build UpdatePlan from self.pending_updates
    ├─ update_service.apply_with_progress(plan, create_snapshot, callback)
    └─ run_post_update_checks(...)
```

**Dependencies**:
- Need to convert `Vec<PackageUpdate>` (packages.rs) → `UpdatePlan` (services/update.rs).
  These use different `PackageUpdate` types (see [Issue 5.1](#51-dual-packageupdate-types)).
- Need to construct a real `SnapshotManager` (not `NoopManager`) from
  `self.snapshot_backend` which is already detected at startup.

**Files to modify**:
- `crates/iron-tui/src/app/actions.rs` — `run_system_update()` and `refresh_updates()`
- Possibly `crates/iron-core/src/services/update.rs` — add conversion helpers

---

## 4. Task S1-P6-003

### Pre-Update Snapshot Integration

> **Priority**: P1
> **TODO Entry**: "user-workflow describes automatic snapshot before CRITICAL updates.
> Code has TODO: Detect and use timeshift/snapper comments."

### Current State of Snapshot Code

The snapshot infrastructure is **fully implemented** in `crates/iron-core/src/snapshot.rs`
(1,095 lines):

| Component | Line | Status |
|-----------|------|--------|
| `SnapshotBackend` enum | L12 | ✅ Timeshift, Snapper, None |
| `SnapshotManager` trait | L62 | ✅ create, list, delete, restore, is_available |
| `detect_backend()` | L84 | ✅ `which timeshift` then `which snapper` |
| `create_manager()` | L105 | ✅ Returns boxed TimeshiftManager/SnapperManager/NoopManager |
| `TimeshiftManager` | L110 | ✅ Full: create, list, delete, restore with CLI args |
| `SnapperManager` | L208 | ✅ Full: create, list, delete, restore via `snapper` |
| `NoopManager` | ~L900 | ✅ No-op for testing |

The `DefaultUpdateService` is **generic over `S: SnapshotManager`** and correctly uses it:

- `apply()` at L1169: `if create_snapshot { self.snapshot_manager.create("pre-update")? }`
- `apply_with_progress()` at L1309: Creates snapshot, stores ID in `UpdateProgress`
- `resume()` at L1376: Does NOT create new snapshot for resume (correct)

### The Problem: TUI Always Uses NoopManager

The TUI's `refresh_updates()` at `actions.rs` L241 constructs:

```rust
let update_service = DefaultUpdateService::new(sm.clone(), NoopManager);
```

And `run_system_update()` doesn't use `DefaultUpdateService` at all — it calls
`package_manager.upgrade(false)` directly.

Meanwhile, the `App` struct **already detects the snapshot backend** at startup:

```rust
// In App::new() at mod.rs L341
snapshot_backend: iron_core::snapshot::detect_backend(),
```

This `snapshot_backend` field (type `SnapshotBackend`) is stored but **never used** to
construct a real `SnapshotManager` for updates. It's only consumed by the Recovery/Doctor
TUI views for display purposes.

### What Exists vs What's Missing

```
✅ IMPLEMENTED (iron-core):
   SnapshotManager trait + Timeshift/Snapper/Noop implementations
   DefaultUpdateService<S> generic over SnapshotManager
   detect_backend() and create_manager()
   apply_with_progress() creates snapshot + stores ID
   Snapshot ID in UpdateProgress for rollback reference

❌ NOT WIRED (iron-tui):
   App.snapshot_backend is detected but unused for updates
   refresh_updates() always passes NoopManager
   run_system_update() bypasses DefaultUpdateService entirely
   No "Create snapshot?" prompt for HIGH risk updates
   No UI to show "Snapshot: timeshift-2026..." after creation
```

### Recommended Fix

This fix is **entirely dependent on S1-P6-002** — once `run_system_update()` uses
`DefaultUpdateService::apply_with_progress()`, snapshot integration comes free:

1. In `run_system_update()`, construct the `DefaultUpdateService` with a real snapshot
   manager based on `self.snapshot_backend`:

   ```rust
   let snapshot_mgr = iron_core::snapshot::create_manager();
   let update_service = DefaultUpdateService::new(sm.clone(), snapshot_mgr);
   ```

2. Decide whether to create a snapshot based on risk level:

   ```rust
   let create_snapshot = match self.update_risk {
       RiskLevel::Critical | RiskLevel::High => true,
       _ => false,
   } && self.snapshot_backend != SnapshotBackend::None;
   ```

3. Call `update_service.apply_with_progress(&plan, create_snapshot, Some(&callback))`.

4. After successful snapshot, show status: "Snapshot created: {id}".

5. In `refresh_updates()`, replace `NoopManager` with the real manager (for pre-flight
   display of snapshot availability).

**No new code needs to be written in iron-core** — the entire snapshot pipeline exists.
The work is purely wiring it in the TUI.

### Spec Requirements vs Implementation

| Spec Requirement | Status |
|------------------|--------|
| Detect timeshift/snapper automatically | ✅ `detect_backend()` |
| Create snapshot before CRITICAL updates | ❌ TUI never creates snapshots |
| Create snapshot before HIGH updates when recommended | ❌ Same |
| Store snapshot ID for potential rollback | ✅ In `UpdateProgress.snapshot_id` |
| Show snapshot status in pre-flight checks | ❌ Not a pre-flight check currently |
| HIGH risk: "Create snapshot first? [y/n/s]" prompt | ❌ Not implemented |
| CLI `--no-snapshot` flag to skip | ✅ CLI has this flag |

---

## 5. Discovered Issues — Outside Original Phase 6 Scope

---

### 5.1 Dual PackageUpdate Types

**Severity**: Low (design inconsistency) | **Files**: `packages.rs` L61, `services/update.rs` L158

Two `PackageUpdate` structs exist with different fields:

**`packages.rs` (used by TUI + iron-pacman):**
```rust
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub is_aur: bool,
    pub is_flagged: bool,
    pub repository: String,
}
```

**`services/update.rs` (used by UpdateService):**
```rust
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub risk: UpdateRisk,
    pub risk_reason: Option<String>,
}
```

The first has AUR/flag info but no risk. The second has risk but no AUR info.
Converting between them requires mapping one to the other. This will need a conversion
function or a unified type when S1-P6-002 is implemented.

---

### 5.2 Dual Risk Enums

**Severity**: Low (design inconsistency) | **Files**: `packages.rs` L12, `services/update.rs` L146

- `RiskLevel` (packages.rs): `Low, Medium, High, Critical` — used by TUI, `assess_risk()`
- `UpdateRisk` (services/update.rs): `Low, Medium, High, Critical` — used by `UpdatePlan`, `PackageUpdate` (service version)

Same four variants, different types. Both derive `PartialOrd`, `Ord`, `Serialize`.
The TUI stores `RiskLevel` in `App.update_risk`; the service uses `UpdateRisk` internally.
These should be unified or a `From` impl added.

---

### 5.3 TUI and CLI Have Complementary but Incomplete Pre-flight Coverage

**Severity**: Medium | **Files**: `actions.rs`, `commands/update.rs`

| Pre-flight Feature | TUI | CLI |
|--------------------|-----|-----|
| Network check | ✅ | ❌ |
| Disk space check | ✅ | ❌ |
| Battery check | ✅ | ❌ |
| Pacman lock check | ✅ | ❌ |
| Time sync check | ✅ | ❌ |
| Arch news check | ✅ | ❌ |
| Risk-differentiated confirm | ✅ | ⚠️ (basic Y/N or `--force`) |
| Progress tracking | ❌ | ✅ |
| Snapshot creation | ❌ | ✅ |
| Resume support | ❌ | ✅ |

The CLI's `commands/update.rs` calls `update_service.check()` and `apply_with_progress()`
but **never runs pre-flight checks**. A user running `iron update` via CLI could update
with no network, locked pacman, low battery, or unacknowledged critical news.

**Recommended Fix**: Add `update_service.run_preflight_checks_with_news()` to the CLI
flow, gating execution on `can_proceed_with_news()` unless `--force` is used.

---

### 5.4 UpdatePlan.news_items Always Empty from check()

**Severity**: Low | **File**: `services/update.rs` L1130

The `UpdateService::check()` implementation returns:
```rust
news_items: vec![], // TODO: Integrate with Arch News parser
```

News is fetched separately via `package_manager.fetch_news()` in the TUI and never
reaches the `UpdatePlan`. This TODO has been worked around (TUI handles news independently)
but represents incomplete integration.

---

### 5.5 run_system_update() Does Not Record Operation in State

**Severity**: Low | **File**: `actions.rs` L450–478

The TUI's `run_system_update()` calls `package_manager.upgrade()` directly.
The `DefaultUpdateService::apply()` records operations via `state_manager.record_operation()`
and calls `state_manager.update_maintenance("update")`. The TUI path does neither.

This means:
- Dashboard's "Last Update: X days ago" is not updated after TUI updates
- Operation log has no entry for TUI-initiated updates
- `iron doctor` cannot verify last update time

---

### 5.6 upgrade() Runs check_updates() and fetch_news() Redundantly

**Severity**: Low | **File**: `iron-pacman/lib.rs` L429

`DefaultPackageManager::upgrade()` calls `check_updates()` and `fetch_arch_news()` again
to build an `UpdatePreview` return value, even though the TUI already has this data from
`refresh_updates()`. This means pressing `[u]` → confirm → execute triggers:

1. `check_updates()` (already done in `refresh_updates()`)
2. `fetch_arch_news()` (already done in `refresh_updates()`)
3. `{aur_helper} -Syu --noconfirm` (the actual update)

The first two are redundant HTTP calls that slow down the update start.

---

### 5.7 CLI Uses pacman, TUI Uses AUR Helper

**Severity**: Medium | **Files**: `services/update.rs` L1174, `iron-pacman/lib.rs` L444

- **CLI path** (`DefaultUpdateService::apply()`): `sudo pacman -Syu --noconfirm`
- **TUI path** (`DefaultPackageManager::upgrade()`): `{aur_helper} -Syu --noconfirm`

The CLI uses raw `pacman`, so AUR packages are **never updated** via CLI.
The TUI uses the detected AUR helper (paru/yay), so AUR packages ARE updated via TUI.

If S1-P6-002 switches TUI to use `DefaultUpdateService::apply_with_progress()`, AUR
updates would regress unless the service is updated to use the AUR helper.

---

### 5.8 Missing Pre-flight Checks from Spec

**Severity**: Low | **File**: `services/update.rs`

The `user-workflow.md` spec describes these pre-flight checks that don't exist:

| Spec Check | Status |
|------------|--------|
| Partial update detection | ❌ Not implemented |
| AUR staleness | ❌ Not a pre-flight check (is assessed in `assess_risk()` via `is_flagged`) |
| Snapshot status | ❌ Not a pre-flight check (spec says "Snapshot: ⚠ No recent snapshot") |

The existing 6 checks (network, disk, battery, pacman lock, time sync, news) cover the
most critical scenarios. The missing checks are informational rather than blocking.

---

## 6. Integration Map

### Complete Update Flow — Current vs Proposed

```
CURRENT TUI FLOW:
                                                              
  Dashboard  ──'u'──►  UpdatePreview  ──'r'──►  refresh_updates()
                            │                        │
                            │                        ├─ PM.check_updates()
                            │                        ├─ PM.fetch_news()
                            │                        ├─ UpdateService(NoopMgr)
                            │                        │    .run_preflight_checks_with_news()
                            │                        └─ assess_risk()
                            │
                         'u' key
                            │
                            ▼
                    can_proceed_with_update()? ──No──► Warning
                            │
                           Yes
                            │
                            ▼
                    request_confirm(RunUpdate)
                            │
                            ▼
                    ConfirmStyle routing
                    (Simple / Enhanced / Typed)
                            │
                         confirm
                            │
                            ▼
                    run_system_update()
                            │
                    PM.upgrade(false) ◄─── {aur_helper} -Syu --noconfirm
                            │               (NO snapshot, NO progress, NO state record)
                            │
                            ▼
                    run_post_update_checks()
                            │
                    ┌───────┼───────┐
                    │       │       │
                .pacnew  reboot  failed
                detect  require  services


PROPOSED TUI FLOW (after S1-P6-002 + S1-P6-003):
                                                              
  Dashboard  ──'u'──►  UpdatePreview  ──'r'──►  refresh_updates()
                            │                        │
                            │                        ├─ PM.check_updates()
                            │                        ├─ PM.fetch_news()
                            │                        ├─ UpdateService(RealMgr) ◄── NEW
                            │                        │    .run_preflight_checks_with_news()
                            │                        └─ assess_risk()
                            │
                         'u' key
                            │
                            ▼
                    can_proceed_with_update()? ──No──► Warning
                            │
                           Yes
                            │
                            ▼
                    request_confirm(RunUpdate)
                            │
                            ▼
                    ConfirmStyle routing
                            │
                         confirm
                            │
                            ▼
                    run_system_update()  ◄── REWRITTEN
                            │
                    ┌───────┴──────────────────────────────────┐
                    │  1. Build UpdatePlan from pending_updates │
                    │  2. Determine create_snapshot             │
                    │  3. UpdateService(RealMgr)                │
                    │       .apply_with_progress(               │
                    │           plan,                           │
                    │           create_snapshot,                │
                    │           progress_callback               │
                    │       )                                   │
                    │  4. state_manager.update_maintenance()    │
                    │  5. run_post_update_checks()              │
                    └──────────────────────────────────────────┘
```

### CLI vs TUI Parity Matrix (Current)

| Feature | CLI (`iron update`) | TUI (`[u]`) | Notes |
|---------|---------------------|-------------|-------|
| Check for updates | ✅ `check()` | ✅ `check_updates()` | Different code paths |
| Pre-flight checks | ❌ | ✅ 6 checks | CLI should add |
| News fetching | ❌ | ✅ RSS + ack | CLI should add |
| Risk assessment | ✅ `UpdateRisk` | ✅ `RiskLevel` | Different enums |
| Risk-keyed confirm | ⚠️ basic Y/N | ✅ 3 styles | S1-P6-001 done |
| Dry-run mode | ✅ `--dry-run` | ❌ | TUI preview is implicit |
| Execution | ✅ `apply_with_progress` | ❌ `PM.upgrade()` | S1-P6-002 |
| Snapshot creation | ✅ if recommended | ❌ NoopManager | S1-P6-003 |
| Progress tracking | ✅ Full | ❌ None | Fixed by S1-P6-002 |
| Resume | ✅ `--resume` | ❌ | Fixed by S1-P6-002 |
| Post-update checks | ⚠️ pacnew count | ✅ Full | CLI should add |
| Operation recording | ✅ | ❌ | Fixed by S1-P6-002 |
| AUR helper | ❌ raw pacman | ✅ detected helper | Issue 5.7 |
| `--force` flag | ✅ | ❌ (no equivalent) | — |
| `--status` flag | ✅ | ❌ | — |
| `--clear-progress` | ✅ | ❌ | — |

---

## 7. Test Coverage Analysis

### Existing Test Counts

| File | #[test] | Coverage Notes |
|------|---------|----------------|
| `services/update.rs` | **106** | Parser events, risk assessment, interrupted update, progress, preflight, post-update, reboot |
| `iron-pacman/lib.rs` | **84** | Risk levels, assess_risk, AUR helpers, parsing, news RSS keywords |
| `snapshot.rs` | **49** | Backend detection, create/list/delete, noop manager |
| `packages.rs` | **38** | assess_risk() combinations, risk ordering, package structs |
| `handlers.rs` | **58** | Confirm dialog styles, typed/enhanced/simple, view navigation |
| `actions.rs` | **40** | Bundle/module/profile/cleanup actions |
| `mod.rs` | **0** | No tests (App struct helper methods untested) |
| `commands/update.rs` | **0** | No tests (CLI update command untested) |

**Total Phase 6–related: 375 tests**

### What's Well Tested

1. **PacmanOutputParser** — comprehensive: multilingual, error lines, package tracking
2. **Risk assessment** — both `assess_risk()` (packages.rs) and `assess_package_risk()` (services/update.rs)
3. **Pre-flight check structs** — result building, can_proceed, blockers/warnings
4. **Post-update detection** — .pacnew/.pacsave patterns, reboot packages
5. **Confirmation dialogs** — all 3 styles with edge cases (S1-P6-001)
6. **Snapshot managers** — create/list/delete for both timeshift and snapper

### Tests Needed for S1-P6-002 (TUI Service Integration)

| Test | Description |
|------|-------------|
| `test_run_system_update_uses_service` | Verify `apply_with_progress()` is called, not `upgrade()` |
| `test_run_system_update_creates_snapshot` | With High risk + backend available → snapshot created |
| `test_run_system_update_no_snapshot_noop` | With backend=None → no snapshot error |
| `test_run_system_update_records_operation` | `update_maintenance("update")` called on success |
| `test_run_system_update_progress_callback` | Progress callback fires during update |

### Tests Needed for S1-P6-003 (Snapshot Wiring)

| Test | Description |
|------|-------------|
| `test_refresh_updates_real_snapshot_manager` | `refresh_updates()` uses detected backend, not NoopManager |
| `test_snapshot_backend_wired_to_service` | `App.snapshot_backend` flows through to `DefaultUpdateService` |
| `test_snapshot_on_critical_risk` | Critical risk → `create_snapshot=true` |
| `test_snapshot_on_high_risk` | High risk → `create_snapshot=true` |
| `test_no_snapshot_on_low_risk` | Low risk → `create_snapshot=false` |

### Tests Needed for Discovered Issues

| Test | For Issue | Description |
|------|-----------|-------------|
| `test_cli_runs_preflight_checks` | 5.3 | CLI `iron update` calls preflight before proceeding |
| `test_aur_packages_updated` | 5.7 | Service uses AUR helper, not raw pacman |
| `test_update_records_maintenance` | 5.5 | Dashboard "Last Update" refreshed after TUI update |

### Test File Locations

| Component | Where Tests Should Live |
|-----------|------------------------|
| TUI update actions | `crates/iron-tui/src/app/actions.rs` — extend `mod tests` |
| CLI update command | `crates/iron-cli/src/commands/update.rs` — add `mod tests` |
| Risk enum unification | `crates/iron-core/src/packages.rs` — extend `mod tests` |
| Snapshot wiring | `crates/iron-tui/src/app/actions.rs` — mock snapshot manager |

---

## Appendix A: Key File Reference

| File | Lines | Purpose |
|------|-------|---------|
| `crates/iron-core/src/services/update.rs` | 3,440 | UpdateService trait+impl, pre-flight, post-update, progress |
| `crates/iron-core/src/packages.rs` | 820 | RiskLevel, PackageManager trait, assess_risk(), PackageUpdate |
| `crates/iron-core/src/snapshot.rs` | 1,095 | SnapshotManager trait, Timeshift/Snapper impl, detect_backend |
| `crates/iron-core/src/state.rs` | 1,485 | SavedUpdatePlan, UpdateProgress, NewsAcknowledgment |
| `crates/iron-pacman/src/lib.rs` | 1,830 | DefaultPackageManager, upgrade(), fetch_arch_news() |
| `crates/iron-tui/src/app/mod.rs` | 809 | App struct, ConfirmStyle/ConfirmAction, request_confirm |
| `crates/iron-tui/src/app/actions.rs` | 1,511 | refresh_updates, run_system_update, run_post_update_checks |
| `crates/iron-tui/src/app/handlers.rs` | 1,590 | UpdatePreview keybindings, confirm dialog handling |
| `crates/iron-tui/src/ui/update.rs` | 472 | UpdatePreview 4-section render |
| `crates/iron-cli/src/commands/update.rs` | 519 | CLI: --dry-run, --force, --resume, --status, --yes |

## Appendix B: Summary of Actions Required

### Task Status

| Task | Status | Action |
|------|--------|--------|
| **S1-P6-001** | ✅ COMPLETED | No further work needed. 3 ConfirmStyle variants, risk routing, 10 tests. |
| **S1-P6-002** | ❌ OPEN | Rewrite `run_system_update()` to use `DefaultUpdateService::apply_with_progress()`. |
| **S1-P6-003** | ❌ OPEN | Wire `App.snapshot_backend` → real `SnapshotManager` in update path. Depends on S1-P6-002. |

### New Tasks to File

| ID | Priority | Title | Category |
|----|----------|-------|----------|
| S1-P6-NEW-001 | **P1** | Replace `PM.upgrade()` with `UpdateService.apply_with_progress()` in TUI | Bug fix (S1-P6-002) |
| S1-P6-NEW-002 | **P1** | Wire snapshot_backend to real SnapshotManager in TUI update path | Feature (S1-P6-003) |
| S1-P6-NEW-003 | **P1** | Add pre-flight checks to CLI `iron update` | Feature gap (Issue 5.3) |
| S1-P6-NEW-004 | **P2** | Unify RiskLevel and UpdateRisk enums | Consistency (Issue 5.2) |
| S1-P6-NEW-005 | **P2** | Unify PackageUpdate types (packages.rs vs services/update.rs) | Consistency (Issue 5.1) |
| S1-P6-NEW-006 | **P2** | Add `record_operation()` + `update_maintenance()` to TUI update path | Bug fix (Issue 5.5) |
| S1-P6-NEW-007 | **P2** | Use AUR helper in DefaultUpdateService instead of raw pacman | Feature (Issue 5.7) |
| S1-P6-NEW-008 | **P3** | Add snapshot status as pre-flight check | Enhancement (Issue 5.8) |
| S1-P6-NEW-009 | **P3** | Eliminate redundant check_updates() + fetch_news() in upgrade() | Performance (Issue 5.6) |
| S1-P6-NEW-010 | **P3** | Resolve TODO in check() — integrate news into UpdatePlan | Cleanup (Issue 5.4) |
| S1-P6-NEW-011 | **P2** | Add CLI update command tests | Test coverage |
| S1-P6-NEW-012 | **P3** | Add partial update detection pre-flight check | Enhancement (Issue 5.8) |

### Implementation Order

```
S1-P6-NEW-005 (Unify PackageUpdate)  ─┐
S1-P6-NEW-004 (Unify Risk enums)     ─┤
                                       ├─► S1-P6-NEW-001 (TUI uses UpdateService)
S1-P6-NEW-007 (AUR in service)       ─┤         │
                                       │         ▼
                                       │  S1-P6-NEW-002 (Snapshot wiring) ← S1-P6-003
                                       │         │
                                       │         ▼
                                       └─ S1-P6-NEW-006 (Operation recording)
                                                 │
                                                 ▼
                                          S1-P6-NEW-003 (CLI pre-flights)
```

The unification tasks (004, 005, 007) are prerequisites for cleanly wiring
the TUI to use `DefaultUpdateService`. The snapshot wiring (002) follows naturally
once the TUI uses the correct service layer. The CLI pre-flight addition (003) is
independently actionable.
