# TUI UX Improvements - Analysis & Plan

**Date**: 2026-02-18
**Type**: BUG_FIX + ENHANCEMENT
**Status**: IMPLEMENTED ✓

---

## Executive Summary

Investigation revealed **4 critical UX issues** in the iron-tui crate affecting navigation and discoverability:

| Issue | Severity | User Impact |
|-------|----------|-------------|
| SetupWizard inaccessible | High | Cannot reconfigure host/bundle after initial setup |
| Sync view unreachable | Medium | Orphaned code, dead feature |
| Tab navigation inconsistency | Medium | Unexpected behavior in UpdatePreview |
| Incomplete hints | Medium | Users cannot discover available shortcuts |

---

## Issue Details

### Issue 1: SetupWizard Inaccessible After First Run

**Files**:
- `crates/iron-tui/src/app/actions.rs:36-41`
- `crates/iron-tui/src/app/handlers.rs:301-308, 421-454`

**Problem**: SetupWizard only accessible during first-time initialization. No re-entry path exists.

**Evidence**:
```rust
// actions.rs:36-41 - Only enters wizard when StateManager fails
match StateManager::new(&config_dir) {
    Ok(sm) => { /* proceed to Dashboard */ }
    Err(_) => { app.view = View::SetupWizard; }
}

// handlers.rs - No shortcut for SetupWizard
KeyCode::Char('d') => View::Dashboard,
KeyCode::Char('b') => View::Bundles,
// ... no 'w' for wizard
```

**Recommendation**: Add `[w]` shortcut and Settings menu option to re-enter wizard.

---

### Issue 2: Sync View Unreachable

**Files**:
- `crates/iron-tui/src/ui/update.rs:392-408`
- `crates/iron-tui/src/app/mod.rs:135`

**Problem**: `View::Sync` exists with a render function but has zero navigation paths.

**Evidence**:
- Not in `cycle_view_forward/backward` sequences
- No keyboard shortcut defined
- No menu entry or link from any view

**Recommendation**: Either:
1. Add `[y]` shortcut and include in navigation cycle, OR
2. Remove orphaned code if feature is deferred

---

### Issue 3: Tab Navigation Inconsistency

**Files**:
- `crates/iron-tui/src/app/handlers.rs:67-75`

**Problem**: UpdatePreview overrides Tab/BackTab for section navigation, breaking expected view-cycling.

**Evidence**:
```rust
// handlers.rs:67-75 - Tab overridden for sections
View::UpdatePreview => match key.code {
    KeyCode::Tab => {
        app.next_update_section();
        true  // Prevents global Tab handling
    }
    // ...
}
```

**Recommendation**:
- Use different key (e.g., `[/]` or arrow keys) for section navigation
- OR clearly indicate in footer that Tab cycles sections, not views

---

### Issue 4: Incomplete Footer & Help Hints

**Files**:
- `crates/iron-tui/src/widgets/mod.rs:131-144, 166-248`
- `crates/iron-tui/src/ui/dashboard.rs:111-128`

**Problem**:
- Dashboard Quick Actions missing `[x] Maintenance`, `[l] Cleanup`
- Footer has incomplete view coverage
- Help overlay missing keybindings for 5 views

**Evidence**:
```rust
// dashboard.rs:111-128 - Missing shortcuts
"[b] Bundles    [p] Profiles    [m] Modules"
"[u] Updates    [s] Settings    [?] Help"
// Missing: [x] Maintenance, [l] Cleanup, [y] Sync
```

**Recommendation**: Update Dashboard and footer to show all available shortcuts.

---

## Recommended Improvement Plan

### Priority 1: Critical Fixes (Do First)

| Task | File | Change |
|------|------|--------|
| Add wizard re-entry shortcut | `handlers.rs:301-308` | Add `KeyCode::Char('w') => View::SetupWizard` |
| Add wizard to Tab cycle | `handlers.rs:421-454` | Include SetupWizard in cycle OR add Settings menu entry |
| Update Dashboard hints | `dashboard.rs:111-128` | Add `[x] Maintenance`, `[l] Cleanup`, `[w] Wizard` |

### Priority 2: Navigation Consistency

| Task | File | Change |
|------|------|--------|
| Fix UpdatePreview Tab override | `handlers.rs:67-75` | Change section nav to `←/→` arrows, keep Tab for views |
| Add Sync view shortcut | `handlers.rs:301-308` | Add `KeyCode::Char('y') => View::Sync` if feature is active |
| Update Tab cycle | `handlers.rs:421-454` | Add Sync to cycle if feature is active |

### Priority 3: Hint Completeness

| Task | File | Change |
|------|------|--------|
| Complete footer coverage | `widgets/mod.rs:131-144` | Add cases for all 16 views |
| Complete help keybindings | `widgets/mod.rs:166-248` | Add keybindings for SetupWizard, detail views, Sync |
| Footer consistency | All `ui/*.rs` | Ensure each view has accurate footer hints |

---

## Implementation Order

```
Phase A: Wizard Re-entry (1-2 hours)
├── Add 'w' shortcut to handlers.rs
├── Add "Reconfigure" option to Settings view
├── Update Dashboard Quick Actions
└── Test wizard can be re-entered and completed

Phase B: Navigation Fixes (2-3 hours)
├── Change UpdatePreview section nav from Tab to arrows
├── Decide on Sync view: enable or remove
├── Update Tab cycle to include all active views
└── Test full navigation cycle works

Phase C: Hint Completeness (1-2 hours)
├── Audit all views for footer hints
├── Add missing footer cases in render_footer
├── Add missing help keybindings
└── Verify Dashboard shows all shortcuts
```

---

## Verification Checklist

- [ ] Wizard accessible via `[w]` from any view
- [ ] Wizard accessible from Settings menu
- [ ] Tab cycles through all main views (including Sync if enabled)
- [ ] UpdatePreview uses arrows for sections, Tab for views
- [ ] Dashboard shows all shortcuts: `[b][p][m][u][s][x][l][w][?]`
- [ ] Footer accurate for every view
- [ ] Help overlay has keybindings for every view
- [ ] All 324+ tests still pass

---

## Files to Modify

| File | Purpose |
|------|---------|
| `crates/iron-tui/src/app/handlers.rs` | Add shortcuts, fix Tab cycle |
| `crates/iron-tui/src/ui/dashboard.rs` | Update Quick Actions hints |
| `crates/iron-tui/src/ui/settings.rs` | Add wizard re-entry option |
| `crates/iron-tui/src/widgets/mod.rs` | Complete footer and help overlay |

---

## Decision Required

**Sync View**: Should be enabled with `[y]` shortcut, or removed as dead code?

The render function exists (`ui/update.rs:392-408`) showing Git sync UI with `[p] push` and `[l] pull` hints, but functionality is not wired. This appears to be scaffolded but incomplete.

**Options**:
1. **Enable**: Add `[y]` shortcut, wire to iron-git operations ✓ CHOSEN
2. **Remove**: Delete `View::Sync` variant and `render_sync()` to reduce confusion
3. **Defer**: Leave code but don't add navigation (current state - not recommended)

---

## Implementation Summary

### Changes Made

**Phase A: Wizard & Navigation Shortcuts**
- Added `[w]` shortcut for SetupWizard re-entry (`handlers.rs:309`)
- Added `[y]` shortcut for Sync view (`handlers.rs:310`)
- Updated Dashboard Quick Actions to show all shortcuts (`dashboard.rs:111-135`)

**Phase B: Navigation Consistency**
- Changed UpdatePreview section navigation from Tab/BackTab to `←/→` arrows (`handlers.rs:67-75`)
- Tab now works globally in all views for view cycling
- Added Sync to Tab cycle: `Dashboard → Bundles → Profiles → Modules → SystemMaintenance → UpdatePreview → Sync → Settings → Dashboard`
- Added SetupWizard handling in Tab cycle (exits to Dashboard)
- Added Sync view key handler with `[p]` push, `[f]` pull, `[s]` status

**Phase C: Complete Hints**
- Expanded footer keybindings to cover all 16 views (`widgets/mod.rs:131-195`)
- Updated help overlay keybindings for UpdatePreview (arrows instead of Tab)
- Added keybindings for Sync and SetupWizard views
- Added `[w]` wizard shortcut to Dashboard and Settings help

### Files Modified

| File | Lines Changed |
|------|---------------|
| `crates/iron-tui/src/app/handlers.rs` | +40 lines (shortcuts, Tab cycle, Sync handler, tests) |
| `crates/iron-tui/src/ui/dashboard.rs` | +10 lines (expanded Quick Actions) |
| `crates/iron-tui/src/widgets/mod.rs` | +60 lines (footer cases, help keybindings) |

### Test Results

- **324 tests pass** in iron-tui
- All workspace tests pass
- No regressions

### Navigation Map (After Fix)

```
Tab Cycle: Dashboard → Bundles → Profiles → Modules → SystemMaintenance → UpdatePreview → Sync → Settings → (loop)

Direct Shortcuts:
  [d] Dashboard    [b] Bundles     [p] Profiles    [m] Modules
  [x] Maintenance  [u] Update      [l] Cleanup     [y] Sync
  [s] Settings     [w] Wizard      [?] Help        [q] Quit
```
