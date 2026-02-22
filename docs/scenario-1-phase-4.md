# Scenario 1 — Phase 4: Bundle Exploration & Activation

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P4-001, S1-P4-002 from `docs/TODO-scenario1.md`
> **Phase**: Bundle Exploration & Activation
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-core, iron-tui, iron-cli, iron-fs and integration boundaries

---

## Table of Contents

1. [Phase 4 Architecture Overview](#1-phase-4-architecture-overview)
2. [Task S1-P4-001 — Implement Dormant Directory Management](#2-task-s1-p4-001)
3. [Task S1-P4-002 — Block Activation When Conflicts Detected](#3-task-s1-p4-002)
4. [Discovered Issues — Outside Phase 4 Scope](#4-discovered-issues)
5. [Integration Map](#5-integration-map)
6. [Test Coverage Analysis](#6-test-coverage-analysis)

---

## 1. Phase 4 Architecture Overview

### The Bundle Model

Iron's data hierarchy places **Bundles** at the top level — each bundle represents a
complete desktop environment (e.g., Hyprland, Niri) with packages, dotfiles, services,
and a set of profiles:

```
HOST (machine identity)
  └─ BUNDLE (desktop environment: hyprland, niri, etc.)
       ├─ Packages (pacman + AUR)
       ├─ Dotfiles (symlinked to ~/.config/*)
       ├─ Services (systemd units: pipewire, etc.)
       ├─ Scripts (post_install hooks)
       └─ Profiles (curated module sets: developer, minimal, etc.)
           └─ Modules (individual config units)
```

### Bundle Struct

Defined in `crates/iron-core/src/bundle.rs` (L8–L40):

```rust
pub struct Bundle {
    pub id: String,               // "hyprland", "niri"
    pub name: String,             // "Hyprland Desktop"
    pub description: Option<String>,
    pub bundle_type: BundleType,  // WaylandCompositor | DesktopEnvironment | X11WindowManager
    pub packages: Vec<String>,    // ["hyprland", "waybar", "wofi", ...]
    pub aur_packages: Vec<String>,// ["hyprshot"]
    pub profiles: Vec<String>,    // ["minimal", "developer", "gaming", "streamer"]
    pub default_profile: Option<String>,
    pub conflicts: Vec<String>,   // ["niri", "sway", "kde"]
    pub services: Vec<String>,    // ["pipewire", "pipewire-pulse", "wireplumber"]
    pub post_install: Option<String>, // "scripts/setup-hyprland.sh"
}
```

**Note**: No `dotfiles` field — dotfile discovery is **convention-based**: the service
walks `bundles/<id>/dotfiles/` to find files. Actual workspace bundles use `config/`
instead (e.g., `bundles/hyprland/config/hypr/hyprland.conf`). There's a mismatch
between the convention the service expects (`dotfiles/`) and the real directory name
(`config/`). See [Discovered Issues](#4-discovered-issues).

### BundleState Enum

Defined in `crates/iron-core/src/bundle.rs` (L56–L66):

```rust
pub enum BundleState {
    NotInstalled,  // Not installed
    Dormant,       // "Installed but not active (configs in dormant/)" ← comment
    Active,        // Installed and active (configs linked)
}
```

The `Dormant` variant exists with a comment saying "configs in dormant/" — but no code
actually moves configs to `dormant/`. The spec additionally describes intermediate states
(`Activating`, `Deactivating`, `Failed`) that do not exist in the enum.

### BundleService Trait

Defined at `crates/iron-core/src/services/bundle.rs` (L16–L40):

| Method | Line | Purpose |
|--------|------|---------|
| `discover()` | L18 | Scan `bundles/` directory for bundle.toml files |
| `load(id)` | L21 | Parse a single bundle by ID |
| `active()` | L24 | Return currently active bundle via state |
| `activate(id)` | L27 | Full activation: packages → dotfiles → services → state |
| `deactivate(id)` | L30 | Deactivation: services → unlink dotfiles → clear state ✅ (S1-P4-004) |
| `switch(from, to)` | L33 | Sequential deactivate + activate |
| `state(id)` | L36 | Determine bundle's state (Active/Dormant/NotInstalled) |
| `check_conflicts(id)` | L39 | Return IDs of conflicting bundles that are currently Active |

### Current Activation Flow

```
User presses 'a' on Bundles/BundleDetail view
  │
  ├─ handlers.rs L404: self.activate_selected_bundle()
  │
  ├─ actions.rs L187-194: activate_selected_bundle()
  │    ├─ Gets selected bundle
  │    ├─ NO conflict check ← GAP (S1-P4-002)
  │    └─ request_confirm(ConfirmAction::SwitchBundle(id))
  │
  ├─ User confirms (Y)
  │    └─ execute_confirm_action() → switch_bundle(id)
  │
  └─ actions.rs L418-448: switch_bundle()
       ├─ Creates DefaultBundleService::new()
       │    .with_package_manager() ← OK
       │    (missing .with_service_manager()) ← BUG
       │
       ├─ If active bundle exists:
       │    └─ bundle_service.deactivate(current.id)
       │         ├─ disable_services() → NoopSystemService (silent no-op)
       │         ├─ unlink_dotfiles() → removes symlinks, restores .iron-backup
       │         ├─ clear_active_bundle(host_id) → removes from state.json ✅ (S1-P4-004)
       │         └─ NO move to dormant/ ← GAP (S1-P4-001)
       │
       └─ bundle_service.activate(bundle_id)
            ├─ install_packages() → real PM
            ├─ link_dotfiles() → creates symlinks
            ├─ enable_services() → NoopSystemService (silent no-op)
            └─ set_active_bundle(host, id) → overwrites previous in state
```

### CLI vs TUI Parity Gap

The CLI correctly gates on conflicts. The TUI does not:

| Check | CLI `install` | CLI `switch` | TUI `activate` |
|-------|--------------|-------------|-----------------|
| `check_conflicts()` | **YES** (L212–219) | **YES** (L276–283) | **NO** |
| Blocks on conflict | Returns early | Returns early | Proceeds anyway |
| Confirm dialog | y/N prompt | y/N prompt | `ConfirmStyle::Simple` |
| Service manager | Injected | Injected | Missing (Noop) |

---

## 2. Task S1-P4-001

### Implement Dormant Directory Management

**Priority**: P1 | **Status**: Not started | **Deps**: None

### 2.1 Problem Statement

The user-workflow spec states:

> Current bundle is **deactivated**: configs moved to `dormant/<bundle_id>/`, services
> disabled, state updated to `DORMANT`.

But `deactivate()` at `crates/iron-core/src/services/bundle.rs` L320–342 only:
1. Disables services
2. Unlinks dotfiles (removes symlinks, restores `.iron-backup` files)

It does **not**:
- Move any config files to `dormant/`
- ~~Update state to clear the active bundle~~ → ✅ **FIXED** (S1-P4-004): `clear_active_bundle()` added
- Record the dormant state anywhere

The `dormant/` directory at workspace root exists but is empty.

### 2.2 What "Dormant" Actually Means

The current `state()` method (L352–380) uses a **heuristic** to detect dormancy:

```rust
fn state(&self, id: &str) -> IronResult<BundleState> {
    // If active_bundles[host] == id → Active
    // If any dotfile symlinks still exist → Dormant (heuristic)
    // Otherwise → NotInstalled
}
```

This heuristic is **broken**: after `unlink_dotfiles()` removes all symlinks, the bundle
goes from Active → NotInstalled, skipping Dormant entirely. The spec says Dormant means
"configs archived in dormant/", not "some symlinks happen to exist."

### 2.3 Proposed Implementation

**Core principle**: Dormant = bundle's dotfiles/config directory is preserved in
`dormant/<bundle_id>/` so it can be restored on re-activation without re-download
or re-generation.

#### 2.3.1 Add `clear_active_bundle()` to StateManager

**File**: `crates/iron-core/src/services/state.rs`
**Insert after**: `set_active_bundle()` (L176–188)

```rust
/// Clear active bundle for a host (deactivation)
pub fn clear_active_bundle(&self, host_id: &str) -> IronResult<()> {
    {
        let mut state = self.state.lock().unwrap();
        state.active_bundles.remove(host_id);
    }
    self.persist()?;
    self.audit(
        "clear_active_bundle",
        OperationStatus::Success,
        Some(host_id.to_string()),
    )
}
```

Without this, standalone deactivation (e.g., `iron bundle remove`) leaves stale
`active_bundles` entries in `state.json`.

#### 2.3.2 Add `move_directory()` to iron-fs

**File**: `crates/iron-fs/src/lib.rs`
**Insert after**: `copy_dir_recursive()` (~L395)

```rust
/// Move a directory to a new location (copy + remove source).
///
/// Falls back to copy+delete if source and target are on different filesystems
/// (std::fs::rename would fail cross-device).
pub fn move_directory(src: &Path, dst: &Path) -> IronResult<()> {
    // Try rename first (fast, atomic on same filesystem)
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Fall back to copy + remove (cross-filesystem)
    copy_dir_recursive(src, dst)?;
    fs::remove_dir_all(src).map_err(|e| FsError::BackupFailed {
        path: src.to_path_buf(),
        message: format!("Failed to remove source after copy: {}", e),
    })?;

    Ok(())
}
```

The existing `copy_dir_recursive()` (L395–420) is private — it should either be made
`pub` or the new `move_directory()` wraps it. The `rename` attempt handles the common
case (same filesystem) efficiently.

#### 2.3.3 Modify `deactivate()` in BundleService

**File**: `crates/iron-core/src/services/bundle.rs`
**Replace**: L320–342

Current flow: disable services → unlink dotfiles

New flow:
1. Disable services
2. Unlink dotfiles (symlinks from `~/.config/` back to bundle)
3. **Move** `bundles/<id>/config/` → `dormant/<id>/config/` (preserving content)
4. **Move** `bundles/<id>/scripts/` → `dormant/<id>/scripts/` (preserving hooks)
5. Copy `bundles/<id>/bundle.toml` → `dormant/<id>/bundle.toml` (metadata reference)
6. Clear active bundle in state

```rust
fn deactivate(&self, id: &str) -> IronResult<()> {
    let bundle = self.load(id)?;
    let host_id = self.current_host()?;

    // Verify bundle is actually active
    if let Some(active_id) = self.state_manager.active_bundle(&host_id) {
        if active_id != id {
            return Err(StateError::BundleNotInstalled { id: id.to_string() }.into());
        }
    } else {
        return Err(StateError::BundleNotInstalled { id: id.to_string() }.into());
    }

    // 1. Disable services
    self.disable_services(&bundle)?;

    // 2. Unlink dotfiles
    self.unlink_dotfiles(&bundle)?;

    // 3. Move configs to dormant/
    self.archive_to_dormant(id)?;

    // 4. Clear active bundle from state
    self.state_manager.clear_active_bundle(&host_id)?;

    Ok(())
}
```

New private helper:

```rust
/// Archive bundle configs to dormant/<id>/
fn archive_to_dormant(&self, id: &str) -> IronResult<()> {
    let bundle_dir = self.bundle_dir(id);
    let dormant_dir = self.bundles_dir.parent()
        .unwrap_or(&self.bundles_dir)
        .join("dormant")
        .join(id);

    fs::create_dir_all(&dormant_dir).ok();

    // Move config and scripts directories
    for subdir in &["config", "scripts"] {
        let src = bundle_dir.join(subdir);
        let dst = dormant_dir.join(subdir);
        if src.exists() {
            iron_fs::move_directory(&src, &dst)?;
        }
    }

    // Copy bundle.toml as metadata reference
    let toml_src = bundle_dir.join("bundle.toml");
    let toml_dst = dormant_dir.join("bundle.toml");
    if toml_src.exists() {
        fs::copy(&toml_src, &toml_dst).ok();
    }

    Ok(())
}
```

#### 2.3.4 Modify `activate()` to Restore from Dormant

**File**: `crates/iron-core/src/services/bundle.rs`
**Insert into**: `activate()` (L284–318), before `install_packages()`

```rust
fn activate(&self, id: &str) -> IronResult<()> {
    let bundle = self.load(id)?;
    let host_id = self.current_host()?;

    // Check if already active
    if let Some(active_id) = self.state_manager.active_bundle(&host_id) {
        if active_id == id {
            return Err(StateError::BundleAlreadyActive { id: id.to_string() }.into());
        }
        self.deactivate(&active_id)?;
    }

    // Restore from dormant if available
    self.restore_from_dormant(id)?;

    // Install packages
    self.install_packages(&bundle)?;
    // Link dotfiles
    self.link_dotfiles(&bundle)?;
    // Enable services
    self.enable_services(&bundle)?;
    // Update state
    self.state_manager.set_active_bundle(&host_id, id)?;

    Ok(())
}
```

New private helper:

```rust
/// Restore bundle configs from dormant/<id>/ back to bundles/<id>/
fn restore_from_dormant(&self, id: &str) -> IronResult<()> {
    let dormant_dir = self.bundles_dir.parent()
        .unwrap_or(&self.bundles_dir)
        .join("dormant")
        .join(id);

    if !dormant_dir.exists() {
        return Ok(()); // Nothing to restore, fresh install
    }

    let bundle_dir = self.bundle_dir(id);

    for subdir in &["config", "scripts"] {
        let src = dormant_dir.join(subdir);
        let dst = bundle_dir.join(subdir);
        if src.exists() && !dst.exists() {
            iron_fs::move_directory(&src, &dst)?;
        }
    }

    // Clean up empty dormant directory
    if dormant_dir.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
        fs::remove_dir_all(&dormant_dir).ok();
    }

    Ok(())
}
```

#### 2.3.5 Update `state()` to Use Dormant Directory

**File**: `crates/iron-core/src/services/bundle.rs`
**Replace**: L352–380

Current heuristic (broken): checks for leftover symlinks
New logic: checks if `dormant/<id>/` exists

```rust
fn state(&self, id: &str) -> IronResult<BundleState> {
    let _ = self.load(id)?; // Verify bundle exists
    let host_id = self.current_host()?;

    // Active = listed in active_bundles
    if let Some(active_id) = self.state_manager.active_bundle(&host_id)
        && active_id == id
    {
        return Ok(BundleState::Active);
    }

    // Dormant = has configs in dormant/<id>/
    let dormant_dir = self.bundles_dir.parent()
        .unwrap_or(&self.bundles_dir)
        .join("dormant")
        .join(id);

    if dormant_dir.exists() {
        return Ok(BundleState::Dormant);
    }

    Ok(BundleState::NotInstalled)
}
```

#### 2.3.6 Update TUI Bundle Views

**File**: `crates/iron-tui/src/ui/bundles.rs`

**render_bundles() — tri-state badge** (replace L42–43 logic):

```rust
let (status, status_style) = if is_active {
    ("●", Style::default().fg(theme::GREEN))
} else if is_dormant {
    ("◐", Style::default().fg(theme::YELLOW))
} else {
    ("○", Style::default().fg(theme::OVERLAY))
};
```

This requires the render function to know each bundle's state. Currently it only checks
`app.active_bundle`. Options:
- **Option A**: Add `pub bundle_states: HashMap<String, BundleState>` to `App` struct, populate on
  view entry. Cleanest but adds a field.
- **Option B**: Derive inline — `is_dormant = dormant_dir.exists()`. Breaks separation
  (render code doing filesystem checks).

**Recommendation**: Option A — add `bundle_states` field, populated in `load_bundles()`.

**render_bundle_detail() — show packages, services, conflicts** (L68–140):

The detail view currently shows only ID, description, type, status, and profiles. The
spec mock-up shows packages, AUR packages, services, and conflicts sections. These are
available on the `Bundle` struct but not rendered.

Add after profiles section:

```rust
// Packages
lines.push(Line::from(Span::styled(
    format!("Packages ({}):", bundle.packages.len()),
    Style::default().fg(theme::YELLOW).bold(),
)));
for pkg in &bundle.packages {
    lines.push(Line::from(format!("  - {}", pkg)));
}

// Services
if !bundle.services.is_empty() {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Services ({}):", bundle.services.len()),
        Style::default().fg(theme::YELLOW).bold(),
    )));
    for svc in &bundle.services {
        lines.push(Line::from(format!("  - {}", svc)));
    }
}

// Conflicts
if !bundle.conflicts.is_empty() {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Conflicts with:",
        Style::default().fg(theme::RED).bold(),
    )));
    for conflict in &bundle.conflicts {
        lines.push(Line::from(Span::styled(
            format!("  ⚠ {}", conflict),
            Style::default().fg(theme::RED),
        )));
    }
}
```

### 2.4 File Change Summary

| File | Change | Complexity |
|------|--------|------------|
| `crates/iron-core/src/services/state.rs` | Add `clear_active_bundle()` | Low |
| `crates/iron-fs/src/lib.rs` | Add `move_directory()` (pub) | Low |
| `crates/iron-core/src/services/bundle.rs` | Modify `deactivate()`, `activate()`, `state()` + add `archive_to_dormant()`, `restore_from_dormant()` | Medium |
| `crates/iron-tui/src/ui/bundles.rs` | Tri-state badge in list, expanded detail view | Medium |
| `crates/iron-tui/src/app/mod.rs` | Add `bundle_states: HashMap<String, BundleState>` | Low |
| `crates/iron-tui/src/app/actions.rs` | Populate `bundle_states` on bundle load | Low |

---

## 3. Task S1-P4-002

### Block Activation When Conflicts Detected

**Priority**: P1 | **Status**: Not started | **Deps**: None

### 3.1 Problem Statement

`activate_selected_bundle()` at `crates/iron-tui/src/app/actions.rs` L187–194 goes straight
to the confirmation dialog without checking conflicts:

```rust
pub fn activate_selected_bundle(&mut self) {
    if self.view != View::Bundles && self.view != View::BundleDetail { return; }
    if let Some(bundle) = self.selected_bundle() {
        let bundle_id = bundle.id.clone();
        self.request_confirm(ConfirmAction::SwitchBundle(bundle_id));  // ← NO conflict check
    }
}
```

Compare with `toggle_selected_module()` (L149–179) which **does** check
`self.module_conflicts` and shows an error if conflicts exist:

```rust
// Module path (correct pattern):
if !self.module_conflicts.is_empty() {
    self.set_error(format!("Cannot enable '{}': conflicts with {}.", ...));
    return;
}
```

The CLI path correctly blocks: both `install()` (L212–219) and `switch()` (L276–283) in
`crates/iron-cli/src/commands/bundle.rs` call `check_conflicts()` and return early if
non-empty.

### 3.2 Existing Module Conflict Pattern

The module conflict system is the template for bundles:

```
User presses 'e' on Modules view
  │
  ├─ handlers.rs: navigate to ModuleDetail triggers load_module_conflicts()  ← PRELOAD
  │    └─ actions.rs L177: calls ModuleService::check_conflicts()
  │       └─ Stores result in self.module_conflicts: Vec<String>
  │
  ├─ handlers.rs: user sees conflict warning in detail view                 ← DISPLAY
  │
  └─ actions.rs L149: toggle_selected_module()
       ├─ if !self.module_conflicts.is_empty()
       │    └─ self.set_error("Cannot enable...") → BLOCKS                  ← GATE
       └─ else
            └─ request_confirm(ConfirmAction::EnableModule(id))             ← PROCEED
```

### 3.3 Proposed Implementation

Follow the module conflict pattern exactly:

#### 3.3.1 Add `bundle_conflicts` Field to App

**File**: `crates/iron-tui/src/app/mod.rs`
**Insert after**: `module_conflicts` field (~L130)

```rust
// -------------------------------------------------------------------------
// Bundle Conflict State (Phase 4)
// -------------------------------------------------------------------------
/// Conflicts for the currently-selected bundle (populated on BundleDetail nav)
pub bundle_conflicts: Vec<String>,
```

Initialize in `Default` impl as `bundle_conflicts: Vec::new()`.

#### 3.3.2 Add `load_bundle_conflicts()` Method

**File**: `crates/iron-tui/src/app/actions.rs`
**Insert after**: `load_module_conflicts()` (~L197)

```rust
/// Load conflict data for the currently selected bundle into `self.bundle_conflicts`.
pub fn load_bundle_conflicts(&mut self) {
    self.bundle_conflicts.clear();
    if let Some(bundle) = self.selected_bundle() {
        let bundle_id = bundle.id.clone();
        if let Some(ref sm) = self.state_manager {
            let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
                .with_package_manager(self.package_manager.clone());
            if let Ok(conflicts) = bundle_service.check_conflicts(&bundle_id) {
                self.bundle_conflicts = conflicts;
            }
        }
    }
}
```

#### 3.3.3 Call `load_bundle_conflicts()` on Navigation

**File**: `crates/iron-tui/src/app/handlers.rs`

When the user navigates to `BundleDetail`, load conflicts. Currently, navigation to
`BundleDetail` happens via `select_item()` at L762:

```rust
View::Bundles if !self.bundles.is_empty() => {
    self.navigate(View::BundleDetail);
}
```

Similar to how `navigate_to_module_detail()` triggers `load_module_conflicts()`, we need
to trigger `load_bundle_conflicts()` after navigating to BundleDetail.

**Option A**: Add a hook in `navigate()` for BundleDetail.
**Option B**: Call it in `select_item()` for the Bundles case.

**Recommendation**: Option B (lighter touch):

```rust
View::Bundles if !self.bundles.is_empty() => {
    self.load_bundle_conflicts();
    self.navigate(View::BundleDetail);
}
```

#### 3.3.4 Gate `activate_selected_bundle()` on Conflicts

**File**: `crates/iron-tui/src/app/actions.rs`
**Replace**: `activate_selected_bundle()` (L187–194)

```rust
pub fn activate_selected_bundle(&mut self) {
    if self.view != View::Bundles && self.view != View::BundleDetail {
        return;
    }
    if let Some(bundle) = self.selected_bundle() {
        let bundle_id = bundle.id.clone();

        // Check for conflicts before allowing activation
        if !self.bundle_conflicts.is_empty() {
            let conflict_names = self.bundle_conflicts.join(", ");
            self.set_error(format!(
                "Cannot activate '{}': conflicts with active bundle(s): {}. \
                 Deactivate conflicting bundle first.",
                bundle_id, conflict_names
            ));
            return;
        }

        self.request_confirm(ConfirmAction::SwitchBundle(bundle_id));
    }
}
```

#### 3.3.5 Show Conflict Warning in Bundle Detail

**File**: `crates/iron-tui/src/ui/bundles.rs`
**Add to**: `render_bundle_detail()`, after the conflicts section

When `app.bundle_conflicts` is non-empty, render a prominent warning:

```rust
if !app.bundle_conflicts.is_empty() {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "⚠ BLOCKED: Active conflict detected",
        Style::default().fg(theme::RED).bold(),
    )));
    for conflict in &app.bundle_conflicts {
        lines.push(Line::from(Span::styled(
            format!("  Bundle '{}' is currently active", conflict),
            Style::default().fg(theme::RED),
        )));
    }
    lines.push(Line::from(Span::styled(
        "  Deactivate conflicting bundle before activation",
        Style::default().fg(theme::SUBTEXT),
    )));
}
```

#### 3.3.6 Update Detail Footer Actions

The current footer at L128:

```rust
"[Esc] Back  [Enter] Activate"
```

Should be context-sensitive:

```rust
let footer = if !app.bundle_conflicts.is_empty() {
    "[Esc] Back  [a] Blocked (conflict)"
} else if is_active {
    "[Esc] Back"
} else {
    "[Esc] Back  [a] Activate"
};
```

### 3.4 Confirm Dialog Enhancement (Optional)

Currently `request_confirm()` uses `ConfirmStyle::Simple` for all bundle actions. For
bundle switches (which deactivate one DE and activate another — a significant operation),
consider using `ConfirmStyle::EnhancedWarning`:

```rust
// In request_confirm():
ConfirmAction::SwitchBundle(_) => ConfirmStyle::EnhancedWarning,
```

This would show the prominent yellow warning border from Phase 6's risk-differentiated
dialogs. A bundle switch affects packages, services, and dotfiles — it's at least a
"High" risk operation.

### 3.5 File Change Summary

| File | Change | Complexity |
|------|--------|------------|
| `crates/iron-tui/src/app/mod.rs` | Add `bundle_conflicts: Vec<String>` field | Low |
| `crates/iron-tui/src/app/actions.rs` | Add `load_bundle_conflicts()`, gate `activate_selected_bundle()` | Low |
| `crates/iron-tui/src/app/handlers.rs` | Call `load_bundle_conflicts()` on BundleDetail nav | Low |
| `crates/iron-tui/src/ui/bundles.rs` | Conflict warning in detail, context-sensitive footer | Medium |
| `crates/iron-tui/src/app/mod.rs` | (Optional) EnhancedWarning for SwitchBundle | Low |

---

## 4. Discovered Issues — Outside Phase 4 Scope

These issues were found during analysis but are not part of S1-P4-001 or S1-P4-002.

### 4.1 `switch_bundle()` Missing `with_service_manager()` (BUG)

**File**: `crates/iron-tui/src/app/actions.rs` L420–421

```rust
let bundle_service = DefaultBundleService::new(&self.config_dir, sm.clone())
    .with_package_manager(self.package_manager.clone());
    // ← Missing: .with_service_manager(...)
```

The `DefaultBundleService` uses `NoopSystemService` when no service manager is chained.
This means `enable_services()` and `disable_services()` during TUI-initiated bundle
switches are silent no-ops — systemd services (pipewire, etc.) are never started or
stopped.

Same bug exists in `RemoveBundle` handling at L78–80.

**Fix**: The TUI needs a `service_manager: Arc<dyn SystemService>` field on `App`,
similar to `package_manager`, passed through from `iron-cli::main()`.

**Tracking**: Should be filed as a new P0 bug task since it's equivalent to the S1-P1-001
PackageManager injection bug.

### 4.2 `deactivate()` Never Clears State (BUG)

**File**: `crates/iron-core/src/services/bundle.rs` L320–342

Confirmed: `deactivate()` never calls `state_manager.clear_active_bundle()` or any
equivalent. After deactivation:
- `active_bundles["desktop"]` still points to the old bundle ID
- `state()` method returns `Active` because the state entry exists
- Only overwritten when another `activate()` runs

This bug cascades: `check_conflicts()` at L382–400 queries `state()` to determine if
conflicting bundles are Active. A deactivated bundle reports as Active, causing false
conflict detections.

**Fix**: Part of S1-P4-001 (adding `clear_active_bundle()` call).

### 4.3 No Rollback in `switch()` (DESIGN GAP)

**File**: `crates/iron-core/src/services/bundle.rs` L344–350

```rust
fn switch(&self, from: &str, to: &str) -> IronResult<()> {
    self.deactivate(from)?;  // ← If this succeeds...
    self.activate(to)?;      // ← ...and this fails → broken state
    Ok(())
}
```

If `activate(to)` fails after `deactivate(from)` succeeds, the system is left with no
active bundle and no recovery path. The spec describes a `TransactionGuard` that rolls
back on failure.

**Recommendation**: Wrap in a transaction-like pattern:

```rust
fn switch(&self, from: &str, to: &str) -> IronResult<()> {
    self.deactivate(from)?;
    match self.activate(to) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Attempt rollback
            let _ = self.activate(from);
            Err(e)
        }
    }
}
```

Full `TransactionGuard` is out of Phase 4 scope but should be tracked.

### 4.4 Dotfiles Directory Convention Mismatch

The `link_dotfiles()` and `unlink_dotfiles()` methods at L123–193 walk
`bundles/<id>/dotfiles/` and symlink to `~/.{relative}`.

But actual workspace bundles use `config/` not `dotfiles/`:
- `bundles/hyprland/config/hypr/hyprland.conf`
- `bundles/niri/config/...`

This means `link_dotfiles()` currently finds nothing to link — the `dotfiles/` directory
doesn't exist. Either:
- **Option A**: Rename `config/` to `dotfiles/` in workspace bundles
- **Option B**: Change the service to look for `config/` instead of `dotfiles/`
- **Option C**: Make the directory name configurable in `bundle.toml`

This is likely why bundle activation appears to work but doesn't actually link anything.
Should be filed as a separate bug.

### 4.5 Bundle Detail View Missing Information

**File**: `crates/iron-tui/src/ui/bundles.rs` L68–140

The detail view currently renders:
- ✅ ID, description, type
- ✅ Status (Active/Inactive only — no Dormant)
- ✅ Profiles list
- ❌ Packages (not shown)
- ❌ AUR packages (not shown)
- ❌ Services (not shown)
- ❌ Conflicts (not shown)
- ❌ Post-install hook info (not shown)

The spec mock-up shows all of these. While rendering more data is straightforward (all
fields are on the `Bundle` struct), it's technically a separate enhancement.

### 4.6 No `DeactivateBundle` Trigger in TUI

There's no way to deactivate a bundle from the TUI without switching to another. The
`ConfirmAction` enum has `RemoveBundle` but there's no key binding or UI path that
triggers it from the Bundles/BundleDetail view. The `'a'` key only triggers
`SwitchBundle`. A separate `'D'` (Shift+D) or `'r'` key for deactivation should
be considered.

### 4.7 `RemoveBundle` Calls `deactivate()` Not `remove()`

**File**: `crates/iron-tui/src/app/actions.rs` L77–90

`ConfirmAction::RemoveBundle` calls `bundle_service.deactivate()` — which just unlinks.
A proper "remove" would additionally remove packages. The `remove_packages()` helper
exists (L110–120) but is marked `#[allow(dead_code)]` — it's never called anywhere.

---

## 5. Integration Map

### 5.1 Dormant Directory Data Flow (S1-P4-001)

```
DEACTIVATION:
                                                           ┌─────────────────┐
handlers.rs                  actions.rs                    │ iron-core        │
  │                            │                           │ bundle service   │
  │  'a' on new bundle         │                           │                  │
  │ ──────────────────────────>│ switch_bundle(id)         │                  │
  │                            │ ─────────────────────────>│ deactivate(old)  │
  │                            │                           │   │              │
  │                            │                           │   ├─ disable_services()
  │                            │                           │   ├─ unlink_dotfiles()
  │                            │                           │   ├─ archive_to_dormant() ─── NEW
  │                            │                           │   │   └─ iron_fs::move_directory()
  │                            │                           │   │       bundles/old/config/ → dormant/old/config/
  │                            │                           │   └─ clear_active_bundle() ── NEW
  │                            │                           │                  │
  │                            │                           │ activate(new)    │
  │                            │                           │   ├─ restore_from_dormant() ── NEW
  │                            │                           │   │   └─ iron_fs::move_directory()
  │                            │                           │   │       dormant/new/config/ → bundles/new/config/
  │                            │                           │   ├─ install_packages()
  │                            │                           │   ├─ link_dotfiles()
  │                            │                           │   ├─ enable_services()
  │                            │                           │   └─ set_active_bundle()
  │                            │                           └─────────────────┘
  │                            │
  │                            │  reload_bundles()
  │<───────────────────────────│  update bundle_states     ── NEW
  │                            │
```

### 5.2 Conflict Gating Flow (S1-P4-002)

```
ACTIVATION ATTEMPT:

User navigates to BundleDetail              User presses 'a' to activate
  │                                           │
  ├─ select_item() in handlers.rs             ├─ activate_selected_bundle()
  │   └─ load_bundle_conflicts()   ── NEW     │   │
  │       └─ BundleService::check_conflicts() │   ├─ if bundle_conflicts.is_empty()
  │           └─ returns Vec<String>           │   │   └─ request_confirm(SwitchBundle)
  │               stored in App.bundle_conflicts│  └─ else
  │                                           │       └─ set_error("Conflicts with...")
  │                                           │           → activation blocked
  │
  ├─ render_bundle_detail()
  │   └─ if !app.bundle_conflicts.is_empty()
  │       └─ render conflict warning  ── NEW
  │
```

### 5.3 State Transition Diagram

```
                     discover()
                        │
                        ▼
              ┌──────────────────┐
              │  NotInstalled    │
              │  (no state entry,│
              │   no dormant/)   │
              └────────┬─────────┘
                       │ activate()
                       │ install_packages()
                       │ link_dotfiles()
                       │ set_active_bundle()
                       ▼
              ┌──────────────────┐
     ┌───────│     Active       │───────┐
     │       │  (state entry,   │       │
     │       │   symlinks live) │       │
     │       └──────────────────┘       │
     │                                  │
     │ deactivate()                     │ switch(from, to)
     │ unlink, archive_to_dormant(),    │ = deactivate(from)
     │ clear_active_bundle()            │ + activate(to)
     │                                  │
     ▼                                  │
┌──────────────┐                        │
│   Dormant    │────────────────────────┘
│ (dormant/<id>/    re-activate:
│  has configs,     restore_from_dormant()
│  no state entry)  + full activate()
└──────────────┘
```

---

## 6. Test Coverage Analysis

### 6.1 Existing Bundle Tests

#### iron-core — `services/bundle.rs` (L405–641)

| Test | Line | What It Covers |
|------|------|----------------|
| `test_discover_bundles` | L442 | Discover finds created bundles |
| `test_load_bundle` | L453 | Load parses TOML correctly |
| `test_bundle_not_found` | L463 | Load returns error for missing |
| `test_bundle_state_not_installed` | L471 | Default state is NotInstalled |
| `test_active_bundle_none` | L481 | No active bundle returns None |
| `test_activate_bundle` | L489 | Activate sets state to Active |
| `test_activate_already_active_bundle` | L501 | Activate same bundle errors |
| `test_activate_switches_from_previous` | L513 | Activate auto-deactivates previous |
| `test_deactivate_not_active_bundle` | L527 | Deactivate non-active errors |
| `test_switch_bundles` | L537 | Switch changes active bundle |
| `test_bundle_state_active` | L557 | State returns Active after activate |
| `test_check_conflicts_empty` | L567 | No conflicts when none active |
| `test_check_conflicts_with_active` | L606 | Detects conflict with active bundle |
| `test_discover_empty_dir` | L619 | Empty dir returns empty vec |
| `test_bundle_service_new` | L626 | Service constructor works |
| `test_activate_nonexistent_bundle` | L635 | Activate missing bundle errors |

**16 tests total.** None test dormant directory operations or rollback.

#### iron-tui — `ui/tests.rs`

| Test | Line | What It Covers |
|------|------|----------------|
| `test_bundles_renders_list` | L290 | List renders bundle names |
| `test_bundles_shows_active_indicator` | L306 | Active bundle has indicator |
| `test_bundles_shows_descriptions` | L323 | Descriptions are displayed |
| `test_bundles_empty_list` | L338 | Empty state shows guidance |
| `test_bundle_detail_renders_info` | L357 | Detail shows ID, type, status |
| `test_bundle_detail_shows_profiles` | L373 | Detail shows profile list |
| `test_bundle_detail_no_selection` | L388 | No-selection shows fallback |
| `test_bundles_renders_at_various_sizes` | L959 | Responsive at different sizes |

**8 tests total.** None test Dormant badge or conflict warnings in UI.

#### iron-tui — `handlers.rs` tests

| Test | What It Covers |
|------|----------------|
| `test_b_navigates_to_bundles` | 'b' key goes to Bundles view |
| `test_enter_opens_bundle_detail` | Enter on bundle goes to BundleDetail |

**2 tests total.** No test for 'a' key activation or conflict blocking.

### 6.2 Tests Needed for S1-P4-001

#### iron-core — StateManager

| Test | Purpose |
|------|---------|
| `test_clear_active_bundle` | Verify `clear_active_bundle()` removes entry |
| `test_clear_active_bundle_no_host` | Clear when host not in map is no-op |
| `test_clear_active_bundle_persists` | Verify change is written to state.json |

#### iron-fs

| Test | Purpose |
|------|---------|
| `test_move_directory_same_fs` | Move via rename on same filesystem |
| `test_move_directory_creates_dst` | Target directory parent is created |
| `test_move_directory_removes_src` | Source is deleted after move |
| `test_move_directory_existing_dst` | Error or merge when target exists |

#### iron-core — BundleService

| Test | Purpose |
|------|---------|
| `test_deactivate_archives_to_dormant` | Deactivation creates dormant/<id>/ |
| `test_deactivate_clears_state` | active_bundles no longer contains ID |
| `test_activate_restores_from_dormant` | Activation moves dormant configs back |
| `test_activate_fresh_no_dormant` | Fresh activate works without dormant/ |
| `test_state_dormant_after_deactivate` | state() returns Dormant post-deactivate |
| `test_state_not_installed_no_dormant` | state() returns NotInstalled when no dormant |
| `test_switch_creates_dormant_for_old` | Old bundle archived, new activated |
| `test_switch_rollback_on_failure` | (future) Rollback when activate fails |

#### iron-tui — UI

| Test | Purpose |
|------|---------|
| `test_bundles_shows_dormant_indicator` | Dormant bundle shows `◐` badge |
| `test_bundle_detail_dormant_status` | Detail view shows "Dormant" not "Inactive" |

### 6.3 Tests Needed for S1-P4-002

#### iron-tui — Actions

| Test | Purpose |
|------|---------|
| `test_activate_blocked_with_conflicts` | Error message set, no confirm dialog |
| `test_activate_proceeds_without_conflicts` | Confirm dialog shown when clean |
| `test_load_bundle_conflicts_populates` | Conflicts loaded from service |
| `test_load_bundle_conflicts_clears_previous` | Old conflicts cleared on new load |

#### iron-tui — Handlers

| Test | Purpose |
|------|---------|
| `test_enter_bundle_detail_loads_conflicts` | Navigating to detail triggers load |
| `test_a_key_blocked_with_conflicts` | 'a' shows error when conflicts exist |

#### iron-tui — UI

| Test | Purpose |
|------|---------|
| `test_bundle_detail_shows_conflict_warning` | Warning rendered when conflicts exist |
| `test_bundle_detail_no_conflict_warning` | No warning when clean |
| `test_bundle_detail_footer_blocked` | Footer shows "Blocked" when conflicts |

### 6.4 Test Count Summary

| Area | Existing | New (S1-P4-001) | New (S1-P4-002) | Total |
|------|----------|------------------|------------------|-------|
| iron-core StateManager | 0 | 3 | 0 | 3 |
| iron-fs | 0 | 4 | 0 | 4 |
| iron-core BundleService | 16 | 8 | 0 | 24 |
| iron-tui Actions | 0 | 0 | 4 | 4 |
| iron-tui Handlers | 2 | 0 | 2 | 4 |
| iron-tui UI | 8 | 2 | 3 | 13 |
| **Total** | **26** | **17** | **9** | **52** |

---

## Appendix A — Key File Reference

| File | Lines | Purpose |
|------|-------|---------|
| `crates/iron-core/src/bundle.rs` | 226 | Bundle struct, BundleType, BundleState enums |
| `crates/iron-core/src/services/bundle.rs` | 641 | BundleService trait + DefaultBundleService impl + 16 tests |
| `crates/iron-core/src/services/state.rs` | 1947 | StateManager — `active_bundle()`, `set_active_bundle()` (no clear) |
| `crates/iron-core/src/state.rs` | 1485 | IronState — `active_bundles: HashMap<String, String>` |
| `crates/iron-tui/src/app/actions.rs` | 1511 | `activate_selected_bundle()`, `switch_bundle()`, `load_module_conflicts()` |
| `crates/iron-tui/src/app/handlers.rs` | 1590 | Key 'a' → `activate_selected_bundle()`, Enter → `select_item()` |
| `crates/iron-tui/src/app/mod.rs` | 809 | App struct, ConfirmAction enum, `request_confirm()` |
| `crates/iron-tui/src/ui/bundles.rs` | ~155 | `render_bundles()`, `render_bundle_detail()` |
| `crates/iron-cli/src/commands/bundle.rs` | 342 | CLI install/switch with conflict checks |
| `crates/iron-fs/src/lib.rs` | 1970 | `copy_dir_recursive()` (private), `move_directory()` needed |
| `bundles/hyprland/bundle.toml` | 42 | Hyprland config: 12 pkgs, conflicts = [niri, sway, kde] |
| `bundles/niri/bundle.toml` | 42 | Niri config: 12 pkgs, conflicts = [hyprland, sway, kde] |

## Appendix B — Discovered Bug Summary

| ID | Severity | Description | Location |
|----|----------|-------------|----------|
| B1 | **P0** | `switch_bundle()` missing `.with_service_manager()` — services never managed via TUI | actions.rs L420 |
| B2 | **P0** | `deactivate()` never clears `active_bundles` state — stale entry persists | bundle.rs L320 |
| B3 | **P1** | `switch()` has no rollback — failed activate leaves no bundle active | bundle.rs L344 |
| B4 | **P1** | Dotfiles directory mismatch — service expects `dotfiles/`, bundles have `config/` | bundle.rs L130 |
| B5 | **P2** | `state()` dormant heuristic is broken — uses leftover symlinks, not dormant directory | bundle.rs L352 |
| B6 | **P2** | `RemoveBundle` calls `deactivate()` not `remove()` — packages never cleaned | actions.rs L78 |
| B7 | **P3** | No TUI path to deactivate without switching — `RemoveBundle` unreachable | mod.rs L237 |
| B8 | **P3** | Bundle detail missing packages/services/conflicts sections | bundles.rs L68 |
