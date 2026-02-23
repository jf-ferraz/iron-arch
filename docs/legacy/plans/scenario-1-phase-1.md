# Scenario 1 — Phase 1 Implementation Guideline

> **Scope**: First Launch & Setup Wizard (Phase 1 from TODO-scenario1.md)
> **Document Type**: Deep-dive implementation guideline with codebase analysis
> **Generated**: 2026-02-19

---

## Executive Summary

Phase 1 covers three tasks: injecting a real PackageManager into BundleService (S1-P1-001), correcting documentation annotations (S1-P1-002), and adding a Setup Wizard progress indicator (S1-P1-003). Two tasks are marked completed; one is pending. This guideline provides an in-depth codebase analysis and implementation details for each task, including residual gaps and verification steps.

---

## Codebase Architecture Context

### Service Layer (iron-core)

```
iron-core/src/services/
├── bundle.rs      # BundleService, DefaultBundleService
├── state.rs       # StateManager
├── host.rs        # HostService
├── profile.rs     # ProfileService
├── module.rs      # ModuleService
└── mod.rs         # Service exports
```

**Key pattern**: `DefaultBundleService::new(path, state_manager)` returns a service with `NoopPackageManager` and `NoopSystemService`. Callers must chain `.with_package_manager(pm)` before using `activate()` or `deactivate()`. The `discover()` method does not require a package manager.

### TUI Layer (iron-tui)

```
iron-tui/src/
├── app/
│   ├── mod.rs       # App state, View enum
│   ├── actions.rs   # execute_confirm_action, switch_bundle, init, etc.
│   └── handlers.rs  # handle_key, handle_wizard_key
├── wizard.rs        # WizardState, WizardStep, apply()
├── ui/
│   ├── wizard.rs    # render_setup_wizard, render_wizard_progress
│   └── mod.rs       # View dispatch to render_* functions
└── lib.rs           # run_with_config(root, package_manager)
```

**Entry point**: `iron go` creates `Arc<DefaultPackageManager>` and passes it to `iron_tui::run_with_config(root, package_manager)`. The `App` struct holds `package_manager: Arc<dyn PackageManager>`.

### Setup Wizard Flow

1. **Trigger**: `App::init()` fails to load `StateManager` (no `.iron/state/state.json`) → `view = View::SetupWizard`, `init_wizard()` called.
2. **Steps**: Welcome → HostSetup → BundleSelection → ProfileSelection → Confirmation → Complete.
3. **Apply**: On Enter at Confirmation, `wizard.apply(&config_dir)` creates StateManager, sets host, activates bundle, sets profile.
4. **Post-apply**: `init()` re-runs, navigates to Dashboard.

---

## Task S1-P1-001 — Inject Real PackageManager into BundleService

### Status

**Completed** (per TODO-scenario1.md). Four call sites in `actions.rs` were updated.

### What Was Done

All `DefaultBundleService::new()` usages in `crates/iron-tui/src/app/actions.rs` that perform activation or deactivation now chain `.with_package_manager(self.package_manager.clone())`:

| Location | Purpose | PM Injected |
|----------|---------|-------------|
| `init()` L24-26 | Load active bundle for current host | ✓ |
| `init()` L49-50 | Load bundles when empty | ✓ |
| `execute_confirm_action` RemoveBundle L79-80 | Deactivate bundle | ✓ |
| `switch_bundle()` L412-414 | Activate new bundle | ✓ |

### Residual Gaps (Verification Required)

#### 1. Setup Wizard Completion Path

**File**: `crates/iron-tui/src/wizard.rs` (L346-352)

```rust
// In WizardState::apply()
if let Some(bundle_id) = self.selected_bundle() {
    let bundle_service = DefaultBundleService::new(config_dir, state_manager.clone());
    if let Err(e) = bundle_service.activate(bundle_id) {
```

**Issue**: `apply()` constructs `DefaultBundleService` without `.with_package_manager()`. First-time users completing the wizard will **not** have packages installed during bundle activation.

**Recommended fix**:

- Add `apply_with_package_manager(&mut self, config_dir: &Path, pm: Arc<dyn PackageManager>) -> Result<(), String>` to `WizardState`, or extend `apply()` with an optional PM parameter.
- Update `handlers.rs` `handle_wizard_key` (L497) to call the new method with `self.package_manager.clone()`.

**Impact**: Critical for first-run experience. New users completing the wizard expect packages to be installed.

#### 2. refresh_current_view (Bundles)

**File**: `crates/iron-tui/src/app/actions.rs` (L321-328)

```rust
View::Bundles | View::BundleDetail => {
    if let Some(ref sm) = self.state_manager {
        let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone());
        self.bundles = bundle_service.discover().unwrap_or_default();
```

**Issue**: No `.with_package_manager()`. For `discover()` this is acceptable (no package ops). For consistency and future-proofing, consider chaining `.with_package_manager(self.package_manager.clone())`.

**Priority**: Low (cosmetic/consistency).

### Implementation Steps (if Fixing Wizard Path)

1. **Add method to `wizard.rs`**:

   ```rust
   pub fn apply_with_package_manager(
       &mut self,
       config_dir: &Path,
       pm: std::sync::Arc<dyn iron_core::PackageManager>,
   ) -> Result<(), String>
   ```

2. **Implement activation branch** with `DefaultBundleService::new(config_dir, state_manager.clone()).with_package_manager(pm)`.

3. **Update `handlers.rs`** (L497): replace `self.wizard.apply(&self.config_dir)` with `self.wizard.apply_with_package_manager(&self.config_dir, self.package_manager.clone())`.

4. **Tests**: Ensure `test_init_wizard_loads_bundles` and a new test `test_wizard_apply_uses_package_manager` (with mock PM) pass.

---

## Task S1-P1-002 — Correct [STUB] Annotations in user-workflow.md

### Status

**Completed** (per TODO-scenario1.md).

### Scope

The user-workflow.md document marked several features as `[STUB]` when they were actually implemented. Corrections included:

- Doctor TUI — 7 health checks rendered
- ProfileBuilder — 3-step wizard (Name → Modules → Preview)
- ModuleCreator — 2-step wizard
- Secrets TUI — git-crypt status, encrypted file list, actions
- Recovery TUI — status panel, export/import/generate keys
- TUI update execution — clarified as real `pacman -Syu` (not dry-run only)

### Verification

- Manually review `user-workflow.md` for remaining incorrect `[STUB]` annotations.
- Cross-check Feature Integration Matrix and Appendix against current TUI/CLI implementation.

---

## Task S1-P1-003 — Add Progress Indicator to Setup Wizard

### Status

**Pending** (per TODO-scenario1.md).

### Current State

A progress indicator **already exists** in `crates/iron-tui/src/ui/wizard.rs`:

- **Function**: `render_wizard_progress(frame, area, wizard)` (L39-59)
- **Rendered**:
  - Progress bar: `[━━━━━━━━━━━━][────────────]`
  - Text: `"Step X of Y"` using `wizard.step_number()` and `wizard.total_steps()`

### Inconsistency Identified

**File**: `crates/iron-tui/src/wizard.rs`

```rust
// L126-139: step_number() returns 1–6
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

// L138-140: total_steps() returns 5
pub fn total_steps(&self) -> usize {
    5
}
```

**Consequence**: On the Complete step, `step_number() = 6` and `total_steps() = 5`. The progress uses `step_num.min(total)`, so it displays "Step 5 of 5" on Complete instead of "Step 6 of 6". The user-workflow specifies a 6-step flow.

### user-workflow Spec

```
Step 1 — Welcome
Step 2 — Host detection
Step 3 — Bundle selection
Step 4 — Profile selection
Step 5 — Confirmation
Step 6 — Complete
```

### Recommended Implementation

#### Option A — Align with 6 Steps (Preferred)

1. Change `total_steps()` in `crates/iron-tui/src/wizard.rs`:

   ```rust
   pub fn total_steps(&self) -> usize {
       6
   }
   ```

2. Update progress rendering to use `step_number()` and `total_steps()` without clamping (remove `.min(total)` if it causes layout issues).

3. Verify progress bar width calculation in `ui/wizard.rs`:

   ```rust
   let filled = "━".repeat(step_num.min(total) * 4);
   let empty = "─".repeat((total - step_num.min(total)) * 4);
   ```

   With `total = 6`, this remains valid; Complete will show "Step 6 of 6" and a full bar.

#### Option B — Keep 5 Steps (Confirmation as Last “Setup” Step)

If the design treats Complete as a final screen rather than a step:

- Document that "Step 5 of 5" is the last setup step and Complete is a summary screen.
- Ensure Welcome text matches: e.g., "5 steps" vs "6 steps" in the wizard copy.

### Unit Tests

**File**: `crates/iron-tui/src/ui/tests.rs`

- `test_wizard_renders_progress_indicator` — already exists; verify it checks for "Step" text.
- Add or extend test for step count:

  ```rust
  #[test]
  fn test_wizard_progress_step_count() {
      let mut app = create_app_with_wizard();
      app.wizard.step = WizardStep::Complete;
      // Render and assert buffer contains "Step 6 of 6" (or "Step 5 of 5" per design)
  }
  ```

### Files to Modify

| File | Changes |
|------|---------|
| `crates/iron-tui/src/wizard.rs` | `total_steps()` return 6 |
| `crates/iron-tui/src/ui/wizard.rs` | Adjust progress text/bar if needed |
| `crates/iron-tui/src/ui/tests.rs` | Step count assertion |

---

## Integration Map: Phase 1 Dependencies

```
┌─────────────────────────────────────────────────────────────────┐
│                         iron go (CLI)                            │
│  Creates DefaultPackageManager → iron_tui::run_with_config()     │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                            App                                   │
│  package_manager: Arc<dyn PackageManager>                        │
│  view: View::SetupWizard (when no state.json)                    │
└─────────────────────────────────────────────────────────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌──────────────┐   ┌─────────────────┐   ┌──────────────────────┐
│ actions.rs   │   │ handlers.rs     │   │ wizard.rs            │
│ init()       │   │ handle_wizard_  │   │ WizardState::apply() │
│ switch_      │   │   key()         │   │   → BundleService    │
│   bundle()   │   │   → wizard.     │   │     .activate()      │
│              │   │     apply()     │   │     (no PM!)         │
└──────────────┘   └─────────────────┘   └──────────────────────┘
        │                    │                    │
        └────────────────────┴────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    DefaultBundleService                          │
│  discover()  — no PM needed                                      │
│  activate()  — requires real PackageManager                      │
│  deactivate() — requires real PackageManager                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Testing Strategy

### S1-P1-001

- **Regression**: Activate a bundle via TUI (Bundles → [a]) and confirm packages are installed (use a mock PM that records install calls).
- **First-run**: Run wizard with no state, complete all steps, verify bundle activation installs packages (mock or real PM).
- **Existing**: `cargo test -p iron-tui actions::tests::` — ensure no regressions.

### S1-P1-002

- Manual review only.

### S1-P1-003

- `cargo test -p iron-tui ui::tests::test_wizard_renders_progress_indicator`
- `cargo test -p iron-tui ui::tests::test_wizard_progress_updates_per_step`
- Add explicit assertion for "Step X of Y" at each step.

---

## Execution Order

1. **Verify S1-P1-001**: Confirm wizard path uses PackageManager; implement wizard PM injection if needed.
2. **S1-P1-003**: Fix `total_steps()` and progress display to match the 6-step spec.
3. **S1-P1-002**: Optional spot-check of user-workflow.md.

---

## Summary

| Task | Status | Effort | Risk |
|------|--------|--------|------|
| S1-P1-001 | Done (with wizard gap) | 1–2h if fixing wizard | High (first-run UX) |
| S1-P1-002 | Done | — | Low |
| S1-P1-003 | Pending | ~1h | Low |

**Priority**: Address the wizard PackageManager gap (S1-P1-001 verification) before or alongside S1-P1-003, as it directly affects first-time users.
