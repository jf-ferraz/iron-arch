# Scenario 1 — Phase 1: First Launch & Setup Wizard

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P1-001, S1-P1-002, S1-P1-003 from `docs/TODO-scenario1.md`
> **Phase**: First Launch & Setup Wizard
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-tui, iron-core, and integration boundaries

---

## Table of Contents

1. [Phase 1 Architecture Overview](#1-phase-1-architecture-overview)
2. [Task S1-P1-001 — Inject Real PackageManager into BundleService](#2-task-s1-p1-001)
3. [Task S1-P1-002 — Correct STUB Annotations](#3-task-s1-p1-002)
4. [Task S1-P1-003 — Setup Wizard Progress Indicator](#4-task-s1-p1-003)
5. [Discovered Issues — Outside Phase 1 Scope](#5-discovered-issues)
6. [Integration Map](#6-integration-map)
7. [Test Coverage Analysis](#7-test-coverage-analysis)

---

## 1. Phase 1 Architecture Overview

### System Flow: First Launch → Setup Wizard → Dashboard

```
CLI Binary (iron-cli)
  │
  │  Creates Arc<DefaultPackageManager> (real pacman wrapper)
  │
  ▼
iron_tui::run_with_config(config_dir, package_manager)     [lib.rs:36]
  │
  │  Constructs App::new(config_dir, package_manager)       [mod.rs:268]
  │  Calls app.init()                                       [actions.rs:12]
  │
  ▼
App::init()
  │
  ├─ StateManager::new(config_dir)                          [state.rs:103]
  │    │
  │    ├─ state.json EXISTS and valid  → Ok(loaded_state)
  │    ├─ state.json MISSING           → Ok(IronState::default())
  │    └─ state.json EXISTS but corrupt → Err(StateError::Corrupted)
  │
  ├─ Ok(sm) with current_host == Some(_) → Load bundles → Dashboard
  ├─ Ok(sm) with current_host == None    → Dashboard (unconfigured) ← GAP
  └─ Err(_)                              → View::SetupWizard + init_wizard()
```

### Key Components

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| `WizardState` | `crates/iron-tui/src/wizard.rs` | L69–90 | State machine: step, selections, errors |
| `WizardStep` enum | `crates/iron-tui/src/wizard.rs` | L52–66 | `Welcome → HostSetup → BundleSelection → ProfileSelection → Confirmation → Complete` |
| `render_setup_wizard()` | `crates/iron-tui/src/ui/wizard.rs` | L10–39 | Top-level render: progress bar + content + nav hints |
| `render_wizard_progress()` | `crates/iron-tui/src/ui/wizard.rs` | L42–63 | Progress bar: `[━━━━─────]  Step N of 5` |
| `handle_wizard_key()` | `crates/iron-tui/src/app/handlers.rs` | L425–510 | Step-specific keyboard dispatch |
| `init_wizard()` | `crates/iron-tui/src/app/actions.rs` | L362–368 | Initializes wizard state, detects host, loads bundles/profiles |
| `WizardState::apply()` | `crates/iron-tui/src/wizard.rs` | L324–369 | Commits wizard result: creates state, activates bundle, sets profile |
| `DefaultBundleService` | `crates/iron-core/src/services/bundle.rs` | L42–69 | Bundle discovery, activation, deactivation |
| `StateManager` | `crates/iron-core/src/services/state.rs` | L90–120 | Persistence layer for `state.json` |
| `App::init()` | `crates/iron-tui/src/app/actions.rs` | L12–64 | Application bootstrap and wizard trigger logic |

### Wizard Step Progression

```
Welcome (1)  →  HostSetup (2)  →  BundleSelection (3)  →  ProfileSelection (4)  →  Confirmation (5)  →  Complete (6)
   │                 │                    │                        │                       │                   │
   │  No input       │  detect_host()     │  load_bundles()        │  load_profiles()      │  apply()          │  init()
   │  required       │  text edit mode    │  j/k selection         │  j/k selection        │  commits state    │  reinit app
   │                 │  host_id required  │  requires ≥1 bundle    │  optional             │  Y/Enter          │  → Dashboard
```

### Data Flow During `apply()`

```
WizardState::apply(config_dir)
  │
  ├─ StateManager::new(config_dir)              → creates/loads state.json
  ├─ state_manager.set_current_host(host_id)    → persists to state.json
  ├─ DefaultBundleService::new(config_dir, sm)  → ⚠ NoopPackageManager (BUG)
  │     └─ bundle_service.activate(bundle_id)
  │           ├─ install_packages()             → NoopPackageManager.install() = no-op
  │           ├─ link_dotfiles()                → symlinks via stow
  │           └─ enable_services()              → NoopSystemService = no-op
  └─ state_manager.set_active_profile(host, profile)
```

---

## 2. Task S1-P1-001

### Inject Real PackageManager into BundleService in TUI

| Field | Value |
|-------|-------|
| **ID** | S1-P1-001 |
| **Priority** | P0 (Critical) |
| **Status** | ✅ Completed (2026-02-19) |
| **Risk** | High — silent data loss (packages never installed) |

### 2.1 Problem Analysis

The `DefaultBundleService` follows a builder pattern where `new()` creates the service
with `NoopPackageManager` and `NoopSystemService` by default. Callers must chain
`.with_package_manager(pm)` to inject a real implementation:

```rust
// crates/iron-core/src/services/bundle.rs:62-69
pub fn new(iron_root: &Path, state_manager: StateManager) -> Self {
    Self {
        bundles_dir: iron_root.join("bundles"),
        state_manager,
        package_manager: Arc::new(NoopPackageManager),    // ← default: no-op
        service_manager: Arc::new(NoopSystemService),     // ← default: no-op
    }
}
```

The `NoopPackageManager` silently succeeds on all operations — `install()`, `remove()`,
`upgrade()` all return `Ok(())` without touching the system. This means any
`BundleService` created without explicit PM injection will appear to work correctly
but will never actually install or remove packages.

### 2.2 Affected Call Sites (Pre-Fix)

There were **6 total** `DefaultBundleService::new()` call sites across the TUI codebase:

| # | Location | Method | Had PM? | Impact |
|---|----------|--------|---------|--------|
| 1 | `actions.rs:27` | `init()` — load bundles from active state | **NO** → **FIXED** | Discovery-only (low impact) |
| 2 | `actions.rs:49` | `init()` — fallback bundle load | **NO** → **FIXED** | Discovery-only (low impact) |
| 3 | `actions.rs:82` | `execute_confirm_action()` — RemoveBundle | **NO** → **FIXED** | `deactivate()` skips package removal (medium) |
| 4 | `actions.rs:421` | `switch_bundle()` | **NO** → **FIXED** | `activate()` skips package install (HIGH) |
| 5 | `actions.rs:325` | `refresh_current_view()` — Bundles | **NO** → **UNFIXED** | Discovery-only (low impact, overlooked) |
| 6 | `wizard.rs:348` | `WizardState::apply()` — first-time activate | **NO** → **UNFIXED** | `activate()` skips package install (HIGH) |

### 2.3 What Was Fixed

Four call sites in `actions.rs` were updated to chain `.with_package_manager()`:

```rust
// Pattern applied at sites 1, 2, 3, 4:
let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
    .with_package_manager(self.package_manager.clone());
```

This works because `App` stores `package_manager: Arc<dyn PackageManager>` (injected
at construction via `App::new()`), and `Arc::clone()` is cheap (reference count increment).

### 2.4 Remaining Gaps (Discovered During Analysis)

**Gap A — `refresh_current_view()` at actions.rs:325**:

```rust
// Current code (still missing PM):
View::Bundles | View::BundleDetail => {
    if let Some(ref sm) = self.state_manager {
        let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
        //                 ← missing .with_package_manager()
        self.bundles = bundle_service.discover().unwrap_or_default();
    }
}
```

**Impact**: Low. `discover()` only reads TOML files from disk — it doesn't call
`install_packages()`. The PM is unused during discovery. However, for consistency and
to prevent future bugs if `discover()` ever gains PM-dependent behavior, this should
be fixed.

**Gap B — `WizardState::apply()` at wizard.rs:348**:

```rust
// Current code (still missing PM):
let bundle_service = DefaultBundleService::new(config_dir, state_manager.clone());
if let Err(e) = bundle_service.activate(bundle_id) { ... }
```

**Impact**: HIGH. This is invoked during the first-time setup wizard when the user
selects a bundle and confirms. `activate()` calls `install_packages()` which uses
`NoopPackageManager`. The user's selected bundle packages are **never installed**.

**Root cause**: `WizardState` is a standalone struct that doesn't have access to
`App.package_manager`. It creates its own `DefaultBundleService` internally.

**Recommended fix** (for a future sprint or follow-up):

```rust
// Option A: Pass PM as parameter to apply()
pub fn apply(
    &mut self,
    config_dir: &Path,
    package_manager: Arc<dyn PackageManager>,
) -> Result<(), String> {
    // ...
    let bundle_service = DefaultBundleService::new(config_dir, state_manager.clone())
        .with_package_manager(package_manager);
    // ...
}

// Then in handlers.rs, Confirmation step:
WizardStep::Confirmation => match key.code {
    KeyCode::Enter | KeyCode::Char('y') => {
        match self.wizard.apply(&self.config_dir, self.package_manager.clone()) {
```

**Trade-off**: This changes the `apply()` signature, which requires updating all
call sites and tests. The `WizardState` deliberately avoids owning an `Arc<dyn PackageManager>`
to keep the wizard testable without mock setup.

**Alternatively** — have `App::init()` (called right after `apply()` succeeds in the
handler) also do a `bundle_service.activate()` with the real PM. But this would
double-activate and potentially cause idempotency issues.

The safest immediate fix is **Option A** — adding the PM parameter. This task was
overlooked in the Sprint 1 fix because the analysis focused on `actions.rs` call sites,
not the `wizard.rs` call site.

### 2.5 Verification

After the Sprint 1 fix, the following call sites correctly inject the PM:

```
✅ actions.rs:27   — init() load from state
✅ actions.rs:49   — init() fallback load
✅ actions.rs:82   — RemoveBundle
✅ actions.rs:421  — switch_bundle()
⚠  actions.rs:325  — refresh_current_view() [low impact, discovery-only]
❌ wizard.rs:348   — apply() first-time activation [HIGH impact]
```

### 2.6 Test Impact

The existing tests pass because `App::default()` uses `NoopPackageManager` by
construction. The fix doesn't change behavior for test scenarios — it only affects
real usage where a `DefaultPackageManager` (pacman wrapper) is injected by the CLI.

To properly test this, an integration test would need to:
1. Create an `App` with a mock `PackageManager` that records calls
2. Trigger bundle activation via TUI key events
3. Assert that `install()` was called with the expected packages

No such test exists currently. The fix was verified by code inspection.

---

## 3. Task S1-P1-002

### Correct `[STUB]` Annotations in user-workflow.md

| Field | Value |
|-------|-------|
| **ID** | S1-P1-002 |
| **Priority** | P1 (High) |
| **Status** | ✅ Completed (2026-02-19) |
| **Risk** | Low — documentation accuracy |

### 3.1 Problem Analysis

The `user-workflow.md` document contained `[STUB]` annotations on 6 features that were
discovered during deep codebase analysis to be **fully implemented**. These annotations
misled contributors into thinking the features needed to be built.

### 3.2 Feature Verification Results

| Feature | Annotation | Actual Code | Evidence |
|---------|-----------|-------------|----------|
| **Doctor TUI** | `[STUB]`: "System Doctor coming soon" | 147-line view rendering 7 health checks | `crates/iron-tui/src/ui/doctor.rs` — reads from `app.state`, renders host/bundle/profile/modules/packages/sync/doctor status |
| **ProfileBuilder** | `[STUB]` | 236-line 3-step wizard | `crates/iron-tui/src/ui/profile_builder.rs` — Step 0: Name/Desc, Step 1: Module checklist with cursor, Step 2: Preview/Create |
| **ModuleCreator** | `[STUB]` | 198-line 2-step wizard | `crates/iron-tui/src/ui/module_creator.rs` — Step 0: ID/Desc/Packages fields, Step 1: Preview/Create |
| **Secrets TUI** | `[STUB]`: "no dedicated TUI screen" | 120-line view | `crates/iron-tui/src/ui/secrets.rs` — git-crypt status, encrypted file list, lock/unlock action keys |
| **Recovery TUI** | `[STUB]`: "CLI-only" | 134-line view | `crates/iron-tui/src/ui/recovery.rs` — status panel, last backup timestamp, export/import/generate keys |
| **TUI Update** | Implied dry-run only | Real `pacman -Syu --noconfirm` | `actions.rs:450` — `self.package_manager.upgrade(false)` calls real pacman |

### 3.3 Annotations Corrected

20+ `[STUB]` annotations were updated across `user-workflow.md`:

- **Feature descriptions** (6 features): Removed `[STUB]`, added "Implemented" with specifics
- **Feature integration matrix** (7 rows): Updated status column from `TUI [STUB]` to `TUI implemented`
- **Appendix — All TUI Views** (5 rows): Changed status from `[STUB]` to `Implemented`
- **Per-view keybinding table** (2 rows): Removed `[STUB]` from ProfileBuilder and ModuleCreator
- **Update execution section**: Changed from "dry-run mode only" to "real system update with risk-differentiated confirmation"
- **CRITICAL risk row**: Changed from `[STUB]` to "Implemented: type CONFIRM to proceed"

### 3.4 Annotations Intentionally Kept

Some `[STUB]` annotations were **correctly left in place** where the feature is genuinely
incomplete:

| Annotation | Location | Reason kept |
|-----------|----------|-------------|
| `[STUB]` on bundle install packages | Phase 4 desc | Wizard `apply()` still uses NoopPM (Gap B above) |
| `[STUB]` on pre-switch snapshot | Phase 4 desc | Timeshift/snapper integration is TODO |
| `[STUB]` on conflict blocking | Phase 4 desc | check_conflicts() returns but doesn't block |
| `[STUB]` on `.pacnew` handling | Phase 7 desc | Hint-only, no interactive resolution |
| `[STUB]` on sync conflict resolution | Phase 8 desc | Defers to git CLI |

### 3.5 Verification

Manual review of `grep -n '\[STUB\]' user-workflow.md` confirms remaining annotations
are all genuinely incomplete features, not false negatives.

---

## 4. Task S1-P1-003

### Add Progress Indicator to Setup Wizard

| Field | Value |
|-------|-------|
| **ID** | S1-P1-003 |
| **Priority** | P2 (Medium) |
| **Status** | ❌ Not Started |
| **Risk** | Low — UI enhancement |

### 4.1 Current State Analysis

**Finding**: A progress indicator **already exists**. The TODO task was based on the
user-workflow spec calling for "Step X of 6", but the codebase already implements this.

#### Existing Implementation

File: `crates/iron-tui/src/ui/wizard.rs`, lines 42–63:

```rust
fn render_wizard_progress(frame: &mut Frame, area: Rect, wizard: &WizardState) {
    let step_num = wizard.step_number();    // 1-indexed (1–6)
    let total = wizard.total_steps();       // Returns 5

    let progress_text = format!("Step {} of {}", step_num.min(total), total);

    // Visual progress bar
    let filled = "━".repeat(step_num.min(total) * 4);
    let empty = "─".repeat((total - step_num.min(total)) * 4);
    let progress_bar = format!("[{}{}]", filled, empty);

    let text = vec![Line::from(vec![
        Span::raw("  "),
        Span::styled(&progress_bar, Style::default().fg(theme::MAUVE)),
        Span::raw("  "),
        Span::styled(progress_text, Style::default().fg(theme::SUBTEXT)),
    ])];

    let para = Paragraph::new(text);
    frame.render_widget(para, area);
}
```

This renders:
```
  [━━━━━━━━────────────]  Step 2 of 5
```

#### Supporting Wizard Logic

File: `crates/iron-tui/src/wizard.rs`:

```rust
// L121-L130
pub fn step_number(&self) -> usize {
    match self.step {
        WizardStep::Welcome => 1,
        WizardStep::HostSetup => 2,
        WizardStep::BundleSelection => 3,
        WizardStep::ProfileSelection => 4,
        WizardStep::Confirmation => 5,
        WizardStep::Complete => 6,
    }
}

// L133-L135
pub fn total_steps(&self) -> usize {
    5 // Complete is step 6 but we show "of 5" since Complete is the result screen
}
```

#### Existing Test Coverage

File: `crates/iron-tui/src/ui/tests.rs`:

| Test | What it verifies |
|------|-----------------|
| `test_wizard_renders_progress_indicator` | Checks "Step 1 of 5" appears in rendered output |
| `test_wizard_progress_updates_per_step` | Iterates all steps, verifies "Step N of 5" for each |

### 4.2 Gap Analysis: What's Missing?

Despite the progress indicator existing, there are potential improvements the
user-workflow spec might intend:

| Aspect | Current State | user-workflow Spec | Gap? |
|--------|--------------|-------------------|------|
| Step counter text | "Step N of 5" | "Step X of 6" | **Minor**: spec says 6 steps, code counts 5 (Complete excluded). Cosmetic choice. |
| Visual bar | `━` filled / `─` empty | Any visual indicator | ✅ Met |
| Step name label | Not shown | Not explicitly required | Could add "Host Setup", "Bundle Selection" etc. |
| Percentage | Not shown | Not explicitly required | Could add "40%" |
| Color coding | MAUVE for bar, SUBTEXT for text | Not specified | ✅ Reasonable |

### 4.3 Recommended Action: Enhance (Not Create)

Since the progress indicator already exists with test coverage, this task should be
**reclassified** from "Add progress indicator" to "Enhance progress indicator" with
optional improvements:

#### Enhancement A — Add Step Name Label (Recommended)

```rust
// Proposed change to render_wizard_progress():
let step_name = match wizard.step {
    WizardStep::Welcome => "Welcome",
    WizardStep::HostSetup => "Host Setup",
    WizardStep::BundleSelection => "Bundle Selection",
    WizardStep::ProfileSelection => "Profile Selection",
    WizardStep::Confirmation => "Confirmation",
    WizardStep::Complete => "Complete",
};

let progress_text = format!("Step {} of {} — {}", step_num.min(total), total, step_name);
```

This would render:
```
  [━━━━━━━━────────────]  Step 2 of 5 — Host Setup
```

**Effort**: ~15 minutes. Change 1 line in `wizard.rs` render, update 2 tests.

**Files touched**:
- `crates/iron-tui/src/ui/wizard.rs` — modify `render_wizard_progress()`
- `crates/iron-tui/src/ui/tests.rs` — update `test_wizard_renders_progress_indicator`
  and `test_wizard_progress_updates_per_step` to check for step names

#### Enhancement B — Colored Step Completion (Optional)

Add per-step colored indicators (checkmarks for completed steps):

```
  ✓ Welcome  ✓ Host  ● Bundle  ○ Profile  ○ Confirm
```

**Effort**: ~1 hour. New rendering logic, new tests.

**Files touched**: Same as Enhancement A.

#### Enhancement C — Step Count Alignment (Trivial)

Change `total_steps()` from 5 to 6 if the spec requires "Step X of 6":

```rust
pub fn total_steps(&self) -> usize {
    6 // Include Complete step in count
}
```

**Risk**: This changes the progress bar fill ratio. At step 5 (Confirmation), the bar
would show 5/6 filled instead of 5/5. Might feel incomplete. Current behavior (5/5 at
Confirmation, then Complete page) is arguably better UX.

### 4.4 Implementation Plan (if proceeding with Enhancement A)

```
1. Modify render_wizard_progress() in crates/iron-tui/src/ui/wizard.rs:
   - Add step_name match expression
   - Append " — {step_name}" to progress_text format string
   - No structural changes to layout or progress bar

2. Update tests in crates/iron-tui/src/ui/tests.rs:
   - test_wizard_renders_progress_indicator: assert "Welcome" appears
   - test_wizard_progress_updates_per_step: assert step names per step

3. Verify:
   - cargo test -p iron-tui
   - Visual check with cargo run (if possible)
```

### 4.5 Decision Required

Given the progress indicator already exists and works correctly with test coverage,
the team should decide:

- **Option 1**: Close S1-P1-003 as "Already Implemented" — the user-workflow spec is met
- **Option 2**: Implement Enhancement A (step name labels) — 15 min effort
- **Option 3**: Implement Enhancement A + B (names + colored indicators) — 1.5 hour effort

---

## 5. Discovered Issues — Outside Phase 1 Scope

During the deep dive, several issues were discovered that are **not part of the Phase 1
task list** but directly relate to Phase 1 (Setup Wizard) functionality. These should be
tracked for future sprints.

### 5.1 First-Launch Detection Gap

**Severity**: Medium
**Location**: `crates/iron-tui/src/app/actions.rs:14–42`

**Problem**: `App::init()` triggers the Setup Wizard only when `StateManager::new()` returns
`Err`. But `StateManager::new()` succeeds with `IronState::default()` when `state.json`
is missing. This means:

| Scenario | StateManager::new() | current_host | Result | Expected |
|----------|---------------------|-------------|--------|----------|
| Fresh install, no state.json | `Ok(default)` | `None` | Dashboard (empty) | SetupWizard |
| Configured system | `Ok(loaded)` | `Some("desktop")` | Dashboard (populated) | Dashboard |
| Corrupted state.json | `Err(Corrupted)` | N/A | SetupWizard | SetupWizard |

**On a true first launch**, the user sees an empty Dashboard with no host, no bundle,
no profile. They must manually press `w` to enter the Setup Wizard.

**Recommended fix** (not in Phase 1 scope):

```rust
// In App::init(), after Ok(sm):
if sm.current_host().is_none() {
    // Unconfigured state — treat as first launch
    self.view = View::SetupWizard;
    self.init_wizard();
    self.state_manager = Some(sm);
    return Ok(());
}
```

**Suggested task ID**: `S1-P1-004` | **P1** | Fix first-launch detection

### 5.2 Wizard `apply()` Missing PackageManager (Gap B from S1-P1-001)

**Severity**: High
**Location**: `crates/iron-tui/src/wizard.rs:348`

This is the most impactful remaining gap from Phase 1. See [Section 2.4](#24-remaining-gaps-discovered-during-analysis) above.

**Suggested task ID**: `S1-P1-005` | **P0** | Fix wizard apply() PM injection

### 5.3 `refresh_current_view()` Missing PM (Gap A from S1-P1-001)

**Severity**: Low
**Location**: `crates/iron-tui/src/app/actions.rs:325`

Discovery-only codepath. See [Section 2.4](#24-remaining-gaps-discovered-during-analysis).

**Suggested task ID**: `S1-P1-006` | **P3** | Add PM to refresh_current_view

### 5.4 No Integration Tests for Wizard Key Handling

**Severity**: Low
**Location**: `crates/iron-tui/src/app/handlers.rs`

The `handle_wizard_key()` function at lines 425–510 has **no dedicated tests** in
the `handlers.rs` test module. Wizard logic is tested in `wizard.rs` (24 unit tests)
and rendering is tested in `ui/tests.rs` (14 tests), but the handler integration
layer — where key events flow through `handle_key()` → `handle_wizard_key()` → state
mutation — is untested.

**Suggested task ID**: `S1-P1-007` | **P2** | Add wizard handler integration tests

---

## 6. Integration Map

### 6.1 Crate Dependencies (Phase 1 Touch Points)

```
iron-cli (binary)
  │
  ├─ iron-tui (presentation)
  │    ├─ app/mod.rs         App state, ConfirmStyle, ConfirmAction
  │    ├─ app/actions.rs     init(), init_wizard(), switch_bundle(), execute_confirm_action()
  │    ├─ app/handlers.rs    handle_key(), handle_wizard_key(), confirm dialog routing
  │    ├─ wizard.rs          WizardState, WizardStep, apply()
  │    ├─ ui/wizard.rs       render_setup_wizard(), render_wizard_progress()
  │    ├─ ui/tests.rs        Wizard render tests
  │    ├─ widgets/mod.rs     render_confirm_dialog() (3 risk styles)
  │    └─ lib.rs             run_with_config() — entry point
  │
  ├─ iron-core (application)
  │    ├─ services/bundle.rs  DefaultBundleService, activate(), deactivate(), discover()
  │    ├─ services/state.rs   StateManager, IronState persistence
  │    ├─ state.rs            IronState struct, Default impl
  │    ├─ packages.rs         RiskLevel enum, PackageManager trait, NoopPackageManager
  │    └─ system_service.rs   SystemService trait, NoopSystemService
  │
  └─ iron-pacman (infrastructure)
       └─ lib.rs              DefaultPackageManager — real pacman wrapper
```

### 6.2 Data Flow: Package Manager Injection

```
iron-cli main()
  │
  │  let pm = Arc::new(DefaultPackageManager::new());  // real pacman
  │
  ▼
iron_tui::run_with_config(config_dir, pm)
  │
  ▼
App::new(config_dir, pm)
  │
  │  self.package_manager = pm  // stored in App
  │
  ├─► init() → BundleService::new().with_package_manager(self.pm.clone())  ✅
  ├─► switch_bundle() → BundleService::new().with_package_manager(self.pm.clone())  ✅
  ├─► execute_confirm_action(RemoveBundle) → BundleService::new().with_package_manager(self.pm.clone())  ✅
  ├─► refresh_current_view(Bundles) → BundleService::new()  ⚠ (no PM, low impact)
  └─► wizard.apply(config_dir) → BundleService::new()  ❌ (no PM, HIGH impact)
       │
       └─ WizardState has no reference to App.package_manager
```

### 6.3 Confirmation Dialog Flow

```
User triggers action (e.g., press 'u' on UpdatePreview)
  │
  ▼
request_confirm(ConfirmAction::RunUpdate)             [mod.rs:354-368]
  │
  ├─ Reads self.update_risk (RiskLevel)
  ├─ Maps to ConfirmStyle:
  │    Critical → TypedConfirmation
  │    High     → EnhancedWarning
  │    Low/Med  → Simple
  ├─ Clears confirm_typed_input
  └─ Sets show_confirm = true
  │
  ▼
render_confirm_dialog()                                [widgets/mod.rs:530-710]
  │
  ├─ TypedConfirmation → 52×12 red popup, "Type CONFIRM:", per-char validation
  ├─ EnhancedWarning   → 48×10 yellow popup, "HIGH RISK", Y/N
  └─ Simple            → 40×7 standard popup, Y/N
  │
  ▼
handle_key() — confirm branch                          [handlers.rs:31-76]
  │
  ├─ TypedConfirmation:
  │    Char(c) → push to confirm_typed_input
  │    Backspace → pop from confirm_typed_input
  │    Enter → if input == "CONFIRM" → execute_confirm_action()
  │    Esc → cancel, clear input
  │
  └─ EnhancedWarning / Simple:
       Y/Enter → execute_confirm_action()
       N/Esc → cancel
```

---

## 7. Test Coverage Analysis

### 7.1 Phase 1 Test Inventory

| Area | File | Test Count | Coverage |
|------|------|-----------|----------|
| Wizard state machine | `wizard.rs` | 24 | Step progression, selection, text input, bounds |
| Wizard rendering | `ui/tests.rs` | 14 | All 6 steps rendered, progress indicator, sizes |
| Wizard key handling | `handlers.rs` | 0 | ❌ No dedicated tests |
| Confirm dialog (Simple) | `handlers.rs` | 4 | Y/N/Enter/Esc |
| Confirm dialog (Risk) | `handlers.rs` | 10 | Low/High/Critical mapping, typed input, backspace, reject, cancel |
| BundleService | `bundle.rs` | ~30 | discover, activate, deactivate, conflicts |
| StateManager | `state.rs` | ~50 | CRUD, persistence, host/bundle/profile ops |
| App actions | `actions.rs` | ~10 | init, refresh, update |
| **Total Phase 1** | | **~142** | |

### 7.2 Coverage Gaps

| Gap | Impact | Recommendation |
|-----|--------|---------------|
| No `handle_wizard_key()` integration tests | Medium — key dispatch untested | Add 10-15 tests covering each step's key handling |
| No PM injection verification test | High — bug could regress silently | Add mock PM test for bundle activation |
| No first-launch detection test | Medium — new users hit empty dashboard | Add test: no state.json → wizard shown |
| No `wizard.apply()` success flow test in handlers | Low — covered by wizard.rs unit tests | Nice-to-have but not blocking |

### 7.3 Existing Test Patterns (Reference for New Tests)

Handler tests use this pattern:

```rust
#[test]
fn test_example() {
    let mut app = App::default();           // NoopPackageManager
    app.view = View::SomeView;              // Set up state
    app.some_data = vec![...];              // Populate data

    app.handle_key(create_key_event(KeyCode::Char('x')));  // Simulate key

    assert_eq!(app.view, View::ExpectedView);  // Verify result
    assert!(app.some_flag);
}
```

Helper function:

```rust
fn create_key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
```

Render tests use this pattern:

```rust
#[test]
fn test_wizard_renders_step() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::default();
    app.view = View::SetupWizard;
    app.wizard.step = WizardStep::SomeStep;

    terminal.draw(|frame| render(frame, &app)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(content.contains("Expected Text"));
}
```

---

## Appendix A — File Reference

| File | Lines | Last Modified | Phase 1 Relevance |
|------|-------|-------------|-------------------|
| `crates/iron-tui/src/wizard.rs` | 836 | Sprint 1 | Wizard state machine |
| `crates/iron-tui/src/ui/wizard.rs` | 400 | Unchanged | Wizard rendering (progress indicator lives here) |
| `crates/iron-tui/src/ui/tests.rs` | ~1300 | Unchanged | Wizard render tests |
| `crates/iron-tui/src/app/mod.rs` | 787 | Sprint 1 | `ConfirmStyle`, `ConfirmAction`, `App` state |
| `crates/iron-tui/src/app/actions.rs` | 1511 | Sprint 1 | `init()`, `init_wizard()`, `switch_bundle()` |
| `crates/iron-tui/src/app/handlers.rs` | 1590 | Sprint 1 | Key handling, confirm dialog routing |
| `crates/iron-tui/src/widgets/mod.rs` | 981 | Sprint 1 | `render_confirm_dialog()` (3 styles) |
| `crates/iron-tui/src/lib.rs` | 633 | Unchanged | TUI entry point |
| `crates/iron-core/src/services/bundle.rs` | 641 | Unchanged | `DefaultBundleService` |
| `crates/iron-core/src/services/state.rs` | 1947 | Unchanged | `StateManager` |
| `crates/iron-core/src/state.rs` | 1485 | Unchanged | `IronState` |
| `crates/iron-core/src/packages.rs` | 820 | Unchanged | `RiskLevel`, `PackageManager` trait |

## Appendix B — New Tasks Discovered

| ID | Priority | Title | Origin |
|----|---------|-------|--------|
| S1-P1-004 | P1 | Fix first-launch detection (current_host == None → wizard) | Section 5.1 |
| S1-P1-005 | P0 | Fix wizard apply() PM injection | Section 5.2 |
| S1-P1-006 | P3 | Add PM to refresh_current_view() for consistency | Section 5.3 |
| S1-P1-007 | P2 | Add wizard handler integration tests | Section 5.4 |
