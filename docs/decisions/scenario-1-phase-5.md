# Scenario 1 — Phase 5: Profile & Module Management

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P5-001, S1-P5-002 from `docs/TODO-scenario1.md`
> **Phase**: Profile & Module Management — Creation Wizards, Activation, Enable/Disable
> **Generated**: 2026-02-20
> **Based on**: Deep codebase analysis across iron-tui, iron-core, iron-cli, and integration boundaries

---

## Table of Contents

1. [Phase 5 Architecture Overview](#1-phase-5-architecture-overview)
2. [Task S1-P5-001 — ProfileBuilder Persist Created Profiles](#2-task-s1-p5-001)
3. [Task S1-P5-002 — ModuleCreator Persist Created Modules](#3-task-s1-p5-002)
4. [Discovered Issues — Outside Original Phase 5 Scope](#4-discovered-issues)
5. [Integration Map](#5-integration-map)
6. [Test Coverage Analysis](#6-test-coverage-analysis)

---

## 1. Phase 5 Architecture Overview

### The Profile & Module Model

Profiles and Modules sit in the middle of Iron's data hierarchy:

```
HOST (machine identity)
  └─ BUNDLE (desktop environment)
       └─ PROFILE (curated module set: developer, minimal, etc.)
            └─ MODULE (individual config unit: nvim-ide, kitty-dev, etc.)
                 └─ DOTFILE (symlinked config file)
```

A **Profile** is a named collection of modules. Activating a profile should enable all its
modules (create symlinks, run hooks). A **Module** is a single application's configuration:
it owns a set of dotfile mappings, packages, hooks, and metadata.

### Profile Struct

Defined in `crates/iron-core/src/profile.rs` (L8–L24):

```rust
pub struct Profile {
    pub id: String,                  // "developer", "minimal"
    pub name: String,                // "Developer Environment"
    pub description: Option<String>,
    pub modules: Vec<String>,        // ["nvim-ide", "kitty-dev", "waybar-dev"]
    pub theme: Option<String>,       // "catppuccin-mocha"
    pub shell: Option<String>,       // "fish"
    pub extends: Option<String>,     // "base" — inheritance
    pub for_bundle: Option<String>,  // "hyprland" — bundle restriction
}
```

**Note**: The Profile model has `load()` (L49) and `save()` (L56) methods, but no unit
tests — the file has **zero `#[cfg(test)]` blocks**, unlike Module (13 tests) and most
other models.

### ProfileState Enum

```rust
pub enum ProfileState {
    Inactive,   // No modules from this profile are enabled
    Active,     // All modules enabled
    Partial,    // Some modules enabled
}
```

### Module Struct

Defined in `crates/iron-core/src/module.rs` (L19–L51):

```rust
pub struct Module {
    pub id: String,                       // "nvim-ide"
    pub name: String,                     // "Neovim IDE"
    pub description: Option<String>,
    pub kind: ModuleKind,                 // AppConfig | Shell | DesktopComponent | ...
    pub packages: Vec<String>,            // ["neovim", "tree-sitter"]
    pub aur_packages: Vec<String>,        // ["neovim-git"]
    pub dotfiles: Vec<DotfileMapping>,    // source → target symlinks
    pub conflicts: Vec<String>,           // ["vim-minimal"]
    pub depends: Vec<String>,             // ["base"]
    pub pre_install: Option<String>,      // hook script path
    pub post_install: Option<String>,     // hook script path
}
```

### ModuleKind Enum

```rust
pub enum ModuleKind {
    AppConfig,          // Application-specific dotfiles
    Shell,              // Shell configuration
    DesktopComponent,   // DE-specific component configs
    Theme,              // Visual themes
    SystemUtil,         // System-level configuration
    DevTools,           // Developer tool packages
}
```

### DotfileMapping Struct

```rust
pub struct DotfileMapping {
    pub source: String,  // "init.lua" (relative to module dir)
    pub target: String,  // "~/.config/nvim/init.lua" (absolute, ~ expanded)
    pub link: Option<bool>,
}
```

### Service Layer Architecture

Two services manage profiles and modules at the application layer:

```
┌─ ProfileService Trait ────────────────────────────────────────────────────┐
│  discover()         → Vec<Profile>       Scan profiles/ directory         │
│  load(id)           → Profile            Parse single profile.toml        │
│  active()           → Option<Profile>    Currently active via state       │
│  apply(id)          → ()                 Enable profile's modules ★       │
│  unapply(id)        → ()                 Disable profile's modules        │
│  state(id)          → ProfileState       Active/Partial/Inactive          │
│  resolve_inheritance(id) → Vec<String>   Build parent chain               │
│  effective_modules(id)   → Vec<String>   All modules incl. inherited      │
│  for_bundle(id)     → Vec<Profile>       Profiles compatible with bundle  │
└───────────────────────────────────────────────────────────────────────────┘

┌─ ModuleService Trait ─────────────────────────────────────────────────────┐
│  discover()         → Vec<Module>        Scan modules/ directory          │
│  load(id)           → Module             Parse single module.toml         │
│  enable(id)         → ()                 Conflicts → hooks → symlinks ★   │
│  disable(id)        → ()                 Unlink → state update            │
│  check_conflicts(id)→ Vec<String>        Explicit + dotfile conflicts     │
│  status(id)         → ModuleState        Installed/Partial/NotInstalled   │
│  list_enabled()     → Vec<Module>        All currently enabled modules    │
│  list_available()   → Vec<Module>        All discovered modules           │
└───────────────────────────────────────────────────────────────────────────┘
```

★ = The **correct** implementation path that the TUI should use.

### Key Insight: Service Layer vs TUI Actions

The services contain the complete, correct business logic:

- `ProfileService::apply()` (`services/profile.rs` L144) enables each profile module via
  `module_service.enable()` and sets the active profile in state.
- `ModuleService::enable()` (`services/module.rs` L199) runs conflict checks, pre-install
  hooks, creates symlinks for each dotfile, updates state, and runs post-install hooks.

However, the TUI actions bypass the service layer entirely (see [Discovered Issues](#4-discovered-issues)).

### TUI Views for Phase 5

| View Enum | File | Purpose |
|-----------|------|---------|
| `View::Profiles` | `ui/profiles.rs` | Profile list with [ACTIVE] badges |
| `View::ProfileDetail` | `ui/profiles.rs` | Single profile details |
| `View::ProfileBuilder` | `ui/profile_builder.rs` | 3-step creation wizard |
| `View::Modules` | `ui/modules.rs` | Module list with [ENABLED] badges |
| `View::ModuleDetail` | `ui/modules.rs` | Single module details |
| `View::ModuleCreator` | `ui/module_creator.rs` | 2-step creation wizard |

---

## 2. Task S1-P5-001

### ProfileBuilder — Persist Created Profiles

> **TODO Entry**: "ProfileBuilder wizard renders UI but may not persist to disk.
> Need to verify and ensure TOML is written on 'Create' confirmation."
>
> **Priority**: P2
> **Files listed**: `handlers.rs`, `services/profile.rs`

### Finding: ✅ PERSISTENCE CONFIRMED — Task Can Be Closed

The ProfileBuilder **does persist to disk**. The full call chain is implemented and
functional. Below is the complete trace from keypress to filesystem write.

### Complete Call Chain

```
User presses 'n' from Profiles view
    │
    ▼
handlers.rs L410–415: match on 'n' key
    │  View::Profiles → self.open_profile_builder()
    │
    ▼
actions.rs L710–718: open_profile_builder()
    │  Reset all wizard state:
    │    profile_builder_step = 0
    │    profile_builder_name = ""
    │    profile_builder_description = ""
    │    profile_builder_selected_modules = []
    │    profile_builder_module_cursor = 0
    │    profile_builder_editing = true
    │    profile_builder_editing_desc = false
    │  Navigate to View::ProfileBuilder
    │
    ▼
handlers.rs L85–88: View::ProfileBuilder dispatch
    │  self.handle_profile_builder_key(key)
    │
    ▼
handlers.rs L517–594: handle_profile_builder_key()
    │
    ├─ Step 0 — Name & Description (L519–564)
    │    Tab: toggle between name/description fields
    │    Char input: append to active field
    │    Backspace: delete last char
    │    Enter: validate name non-empty, advance to step 1
    │
    ├─ Step 1 — Module Selection (L566–586)
    │    j/k: move cursor through module list
    │    Space: toggle module checkbox
    │    Enter: advance to step 2
    │    Esc: go back to step 0
    │
    └─ Step 2 — Preview & Create (L588–594)
         Enter: → self.create_profile_from_builder()  ★
         Esc: go back to step 1
```

### The Persistence Function

`actions.rs` L722–757 — `create_profile_from_builder()`:

```rust
pub fn create_profile_from_builder(&mut self) {
    let name = self.profile_builder_name.trim().to_string();
    if name.is_empty() {
        self.set_error("Profile name cannot be empty");
        return;
    }

    // 1. Create directory: profiles/<name>/
    let profile_dir = self.config_dir.join("profiles").join(&name);
    std::fs::create_dir_all(&profile_dir)?;

    // 2. Generate TOML via template
    let toml_content = iron_core::templates::profile_toml(
        &name, desc_opt, &module_ids
    );

    // 3. Write to disk: profiles/<name>/profile.toml
    let profile_path = profile_dir.join("profile.toml");
    std::fs::write(&profile_path, toml_content)?;

    // 4. Reload all profiles (clear + re-scan)
    self.profiles.clear();
    self.load_profiles();

    // 5. Navigate back to Profiles view
    self.set_status(format!("Created profile: {}", name));
    self.navigate(View::Profiles);
}
```

### Template Used

`templates.rs` L80–130 — `profile_toml()` generates a fully documented TOML file:

```toml
# Profile: developer
# ─────────────────────────────────────────────────────────────────────
# Iron profile configuration. A profile is a curated collection of
# modules. Activate a profile to enable all its modules at once.
# ─────────────────────────────────────────────────────────────────────

id = "developer"
name = "developer"
description = "Full development environment"

modules = [
  "nvim-ide",
  "kitty-dev",
  "waybar-dev",
]

# extends = ""
# for_bundle = ""
```

### UI Rendering

`ui/profile_builder.rs` (213 lines) renders the 3-step wizard:

| Function | Line | Step | What It Renders |
|----------|------|------|-----------------|
| `render_profile_builder()` | L10 | — | Top-level dispatcher (step → sub-render) |
| `render_step_name()` | L82 | 0 | Name + Description text input fields |
| `render_step_modules()` | L139 | 1 | Checkbox list of available modules |
| `render_step_preview()` | L180 | 2 | Preview card + "Press [Enter] to create" hint |

### App State Fields

`app/mod.rs` L143–157 — wizard state lives directly on the App struct:

```rust
// Profile Builder state
pub profile_builder_step: usize,
pub profile_builder_name: String,
pub profile_builder_description: String,
pub profile_builder_selected_modules: Vec<String>,
pub profile_builder_module_cursor: usize,
pub profile_builder_editing: bool,
pub profile_builder_editing_desc: bool,
```

### What's Missing from Spec (Not Blocking Closure)

The `user-workflow.md` Phase 5 spec describes additional wizard features that are **not
implemented** but are enhancement-level concerns, not persistence bugs:

1. **Dependency Resolution** (spec step 3): "selecting `nvim-ide` auto-suggests `dev-tools`"
   — not implemented. The module checkbox list is flat with no dependency awareness.

2. **Conflict Warnings** (spec step 4): "selecting `kitty-dev` warns if `kitty-minimal` is
   checked" — not implemented. No conflict checking during wizard selection.

3. **Validation**: No duplicate profile name check (existing dir overwrite is possible).

4. **Profile ID sanitisation**: The TUI uses `name` as the directory name directly.
   The CLI (`commands/profile.rs` L233) validates: `id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')`.
   The TUI does not perform this validation — spaces or special characters in the name
   will create directories with those characters.

### Recommendation

**Close S1-P5-001.** Persistence works end-to-end. File the following as separate
enhancement tasks:

| New Task | Priority | Description |
|----------|----------|-------------|
| S1-P5-003 | P3 | ProfileBuilder: add dependency auto-suggestion |
| S1-P5-004 | P3 | ProfileBuilder: add conflict warnings during selection |
| S1-P5-005 | P2 | ProfileBuilder: validate/sanitise profile ID |
| S1-P5-006 | P3 | ProfileBuilder: check for duplicate profile names |

---

## 3. Task S1-P5-002

### ModuleCreator — Persist Created Modules

> **TODO Entry**: "Same as above for modules."
>
> **Priority**: P2
> **Files listed**: `handlers.rs`, `services/module.rs`

### Finding: ✅ PERSISTENCE CONFIRMED — Task Can Be Closed

The ModuleCreator **does persist to disk**. The full call chain is implemented and
functional. Below is the complete trace.

### Complete Call Chain

```
User presses 'n' from Modules view
    │
    ▼
handlers.rs L410–415: match on 'n' key
    │  View::Modules → self.open_module_creator()
    │
    ▼
actions.rs L761–767: open_module_creator()
    │  Reset all wizard state:
    │    module_creator_step = 0
    │    module_creator_name = ""
    │    module_creator_description = ""
    │    module_creator_packages = ""
    │    module_creator_active_field = 0
    │  Navigate to View::ModuleCreator
    │
    ▼
handlers.rs L91–94: View::ModuleCreator dispatch
    │  self.handle_module_creator_key(key)
    │
    ▼
handlers.rs L598–644: handle_module_creator_key()
    │
    ├─ Step 0 — Details Entry (L600–635)
    │    Tab: cycle through 3 fields (ID, Description, Packages)
    │    Char input: append to active field
    │    Backspace: delete last char
    │    Enter: validate ID non-empty, advance to step 1
    │
    └─ Step 1 — Preview & Create (L637–644)
         Enter: → self.create_module_from_creator()  ★
         Esc: go back to step 0
```

### The Persistence Function

`actions.rs` L771–809 — `create_module_from_creator()`:

```rust
pub fn create_module_from_creator(&mut self) {
    let id = self.module_creator_name.trim().to_string();
    if id.is_empty() {
        self.set_error("Module ID cannot be empty");
        return;
    }

    // 1. Create directory: modules/<id>/
    let module_dir = self.config_dir.join("modules").join(&id);
    std::fs::create_dir_all(&module_dir)?;

    // 2. Parse comma-separated packages
    let pkgs: Vec<&str> = self.module_creator_packages
        .split(',').map(|s| s.trim()).filter(|s| !s.is_empty())
        .collect();

    // 3. Generate TOML via template
    let toml_content = iron_core::templates::module_toml(&id, desc_opt, &pkgs);

    // 4. Write to disk: modules/<id>/module.toml
    let module_path = module_dir.join("module.toml");
    std::fs::write(&module_path, toml_content)?;

    // 5. Reload all modules (clear + re-scan)
    self.modules.clear();
    self.load_modules();

    // 6. Navigate back to Modules view
    self.set_status(format!("Created module: {}", id));
    self.navigate(View::Modules);
}
```

### Template Used

`templates.rs` L13–76 — `module_toml()` generates a fully documented TOML with all fields:

```toml
# Module: nvim-ide
# ─────────────────────────────────────────────────────────────────────
# Iron module configuration. A module represents a single application's
# configuration. Modules can be independently enabled/disabled.
# ─────────────────────────────────────────────────────────────────────

id = "nvim-ide"
description = "Neovim IDE with LSP"
kind = "utility"

packages = [
  "neovim",
  "tree-sitter",
]

aur_packages = []
dotfiles = []
depends = []
conflicts = []
```

### UI Rendering

`ui/module_creator.rs` (186 lines) renders the 2-step wizard:

| Function | Line | Step | What It Renders |
|----------|------|------|-----------------|
| `render_module_creator()` | L10 | — | Top-level dispatcher |
| `render_step_details()` | L69 | 0 | Three text fields: ID, Description, Packages |
| `render_step_preview()` | L129 | 1 | Preview card + "Press [Enter] to create" hint |

### App State Fields

`app/mod.rs` L162–168:

```rust
// Module Creator state
pub module_creator_step: usize,
pub module_creator_name: String,
pub module_creator_description: String,
pub module_creator_packages: String,
pub module_creator_active_field: usize,
```

### What's Missing from Spec (Not Blocking Closure)

1. **ModuleKind Selection**: Template hardcodes `kind = "utility"`. The wizard has no
   field for selecting a kind. The Module struct supports 6 kinds (AppConfig, Shell,
   DesktopComponent, Theme, SystemUtil, DevTools).

2. **Dotfile Mapping Configuration**: The wizard only collects ID, description, and packages.
   There is no UI for defining dotfile source→target mappings. The template outputs
   `dotfiles = []` — users must manually edit the TOML afterwards.

3. **Conflict / Depends Fields**: No UI for specifying module conflicts or dependencies.
   Template outputs `conflicts = []` and `depends = []`.

4. **AUR Package Separation**: Packages are entered as a single comma-separated string.
   No distinction between regular `packages` and `aur_packages`.

5. **Module ID Validation**: Same issue as ProfileBuilder — no character validation.
   The CLI does not have a `module create` command at all (see Discovered Issues).

6. **Duplicate Module Check**: No check if `modules/<id>/` already exists before creation.
   Existing module.toml would be overwritten silently.

### Recommendation

**Close S1-P5-002.** Persistence works end-to-end. File the following as separate
enhancement tasks:

| New Task | Priority | Description |
|----------|----------|-------------|
| S1-P5-007 | P3 | ModuleCreator: add ModuleKind selection step |
| S1-P5-008 | P2 | ModuleCreator: add dotfile mapping configuration |
| S1-P5-009 | P3 | ModuleCreator: add conflict/depends fields |
| S1-P5-010 | P2 | ModuleCreator: validate/sanitise module ID |
| S1-P5-011 | P3 | ModuleCreator: check for duplicate module IDs |

---

## 4. Discovered Issues — Outside Original Phase 5 Scope

Analysis of Phase 5 code uncovered **three medium-severity bugs** and several
inconsistencies that were not captured in the original TODO.

---

### 4.1 BUG: TUI Profile Activation is State-Only

**Severity**: Medium | **File**: `actions.rs` L198–215

`activate_selected_profile()` only writes state — it never calls `ProfileService::apply()`:

```rust
// CURRENT (buggy):
pub fn activate_selected_profile(&mut self) {
    let profile = self.selected_profile()?.id.clone();
    if let (Some(sm), Some(host_id)) = (&self.state_manager, &self.current_host) {
        sm.set_active_profile(host_id, &profile)?;      // ← State-only!
        self.active_profile = Some(profile.clone());
    }
}
```

**What should happen** (per spec UC-8 and `ProfileService::apply()` at L144):

1. Resolve `effective_modules()` for the target profile (including inherited modules)
2. For each module: `module_service.enable()` → conflict check → hooks → symlinks → state
3. Update `set_active_profile()` in state

**Impact**: When a user activates a profile via TUI, the status text says "Activated profile:
developer" but **no dotfiles are symlinked** and **no hooks run**. The profile appears active
in the UI but the filesystem is unchanged.

**Contrast with CLI**: `commands/profile.rs` L210 (`select()` function) correctly calls
`profile_service.apply(id)` which performs the full enable sequence.

**Recommended Fix**:  
Inject `ProfileService` into the TUI `App` (similar to how the CLI creates one via
`ctx.profile_service()`) and call `profile_service.apply(id)` instead of
`sm.set_active_profile()`. Also handle `unapply()` for the previously active profile.

---

### 4.2 BUG: TUI Module Enable/Disable is State-Only

**Severity**: Medium | **File**: `actions.rs` L96–121

The TUI confirm-action handler for `EnableModule` and `DisableModule` only calls
`StateManager` methods — never `ModuleService`:

```rust
// CURRENT (buggy):
ConfirmAction::EnableModule(ref id) => {
    sm.enable_module(id)?;           // ← State-only! No symlinks, no hooks.
    self.active_modules = sm.active_modules();
}
ConfirmAction::DisableModule(ref id) => {
    sm.disable_module(id)?;          // ← State-only! No symlink removal.
    self.active_modules = sm.active_modules();
}
```

**What should happen** (per `ModuleService::enable()` at L199):

1. `check_conflicts()` — explicit conflicts + dotfile target overlaps
2. Run `pre_install` hook script
3. `create_symlink()` for each dotfile mapping
4. `state_manager.enable_module()`
5. Run `post_install` hook script

**What should happen** for disable (`ModuleService::disable()` at L247):

1. `remove_symlink()` for each dotfile mapping
2. `state_manager.disable_module()`

**Impact**: Module toggle via TUI changes the state badge from [DISABLED] to [ENABLED]
(and vice versa) but **no dotfiles are symlinked/unlinked** and **no hooks execute**.

**Contrast with CLI**: `commands/module.rs` L261 (`enable()` function) correctly calls
`module_service.enable(id)` with full conflict check, symlinks, and hooks.

**Recommended Fix**:  
Inject `ModuleService` into the TUI `App` and replace `sm.enable_module(id)` with
`module_service.enable(id)` (and `disable` equivalently). Surface conflict check failures
as error messages in the status bar.

---

### 4.3 CLI Missing `module create` Command

**Severity**: Low | **File**: `commands/module.rs`

The CLI's `ModuleAction` enum supports only four subcommands:

```rust
pub enum ModuleAction {
    List { enabled, disabled, kind },
    Show { id },
    Enable { id, force },
    Disable { id, yes },
    // No Create variant!
}
```

The `user-workflow.md` spec lists `iron module create` as a CLI equivalent, but it does not
exist. Users can only create modules via the TUI wizard or by manually writing TOML.

**Contrast with profiles**: The CLI has `ProfileAction::Create { id, name, extends }` at
`commands/profile.rs` L36 and a full `create()` function at L222.

**Recommended Fix**:  
Add `ModuleAction::Create { id, description, kind, packages }` mirroring the TUI wizard
fields. Use `iron_core::templates::module_toml()` for consistency.

---

### 4.4 Inconsistent Profile Creation Between CLI and TUI

**Severity**: Low | **Files**: `commands/profile.rs` L222–260, `actions.rs` L722–757

The CLI `profile create` command generates profile TOML **inline** with a minimal format:

```rust
// CLI (commands/profile.rs L243-252)
let mut content = format!(
    r#"id = "{}"
name = "{}"
"#, id, profile_name
);
content.push_str("\nmodules = []\n");
```

The TUI ProfileBuilder uses `iron_core::templates::profile_toml()` which generates a
**richly documented TOML** with section headers, comments on every field, and commented-out
optional fields.

**Impact**: Profiles created via CLI have bare-minimum TOML (no comments, no optional
fields shown). Profiles created via TUI have full documentation. A user creating via CLI
then opening the file finds no guidance on available options (`extends`, `for_bundle`,
`theme`, `shell`).

**Recommended Fix**:  
Refactor CLI `profile create` to use `iron_core::templates::profile_toml()` like the TUI
does. This also ensures any future template changes apply to both paths.

---

### 4.5 Profile Model Has Zero Unit Tests

**Severity**: Low | **File**: `crates/iron-core/src/profile.rs` (75 lines)

The `Profile` struct has `load()` and `save()` methods but no `#[cfg(test)]` block.
Compare with `module.rs` which has 13 tests including `load_save_roundtrip`, field
validation, and edge cases.

**Recommended Tests**:

| Test | Purpose |
|------|---------|
| `test_profile_load_save_roundtrip` | Write → load → compare all fields |
| `test_profile_load_missing_file` | Verify error on nonexistent path |
| `test_profile_load_invalid_toml` | Verify error on malformed TOML |
| `test_profile_all_modules_basic` | Verify `all_modules()` returns direct modules |
| `test_profile_optional_fields` | Extends, for_bundle, theme, shell default to None |

---

### 4.6 ProfileBuilder Step 1 Has No Module Availability Guard

**Severity**: Low | **File**: `handlers.rs` L566–586

In Step 1, the module checkbox list is populated from `self.modules` which is loaded at
startup via `load_modules()`. If no modules exist (empty `modules/` directory), the user
sees an empty list with no explanation and cannot proceed meaningfully.

**Recommended Fix**: If `self.modules.is_empty()`, show an informational message like
"No modules found. Create modules first (Esc → m → n)." instead of an empty checklist.

---

## 5. Integration Map

### Data Flow: Creation Path

```
                   ┌─────────────┐
                   │   TUI App   │
                   └──────┬──────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
  ┌───────────────┐ ┌──────────┐ ┌──────────────┐
  │ProfileBuilder │ │ Handler  │ │ModuleCreator │
  │   (render)    │ │(keypress)│ │   (render)   │
  │profile_builder│ │handlers  │ │module_creator│
  │  .rs (213 ln) │ │.rs L517+ │ │ .rs (186 ln) │
  └───────┬───────┘ └────┬─────┘ └──────┬───────┘
          │              │               │
          │  ┌───────────┴───────────┐   │
          │  │                       │   │
          ▼  ▼                       ▼   ▼
  ┌─────────────────┐       ┌────────────────────┐
  │create_profile_  │       │create_module_from_ │
  │from_builder()   │       │creator()           │
  │actions.rs L722  │       │actions.rs L771     │
  └────────┬────────┘       └────────┬───────────┘
           │                         │
           ▼                         ▼
  ┌─────────────────┐       ┌─────────────────┐
  │ profile_toml()  │       │ module_toml()   │
  │templates.rs L80 │       │templates.rs L13 │
  └────────┬────────┘       └────────┬────────┘
           │                         │
           ▼                         ▼
  ┌──────────────────────────────────────────┐
  │           std::fs::write()               │
  │  profiles/<name>/profile.toml            │
  │  modules/<id>/module.toml                │
  └──────────────────────────────────────────┘
           │                         │
           ▼                         ▼
  ┌──────────────────────────────────────────┐
  │   load_profiles() / load_modules()       │
  │   actions.rs L384 / L401                 │
  │   Re-scan directories, rebuild Vec       │
  └──────────────────────────────────────────┘
```

### Data Flow: Activation/Enable Path (Current vs Expected)

```
CURRENT (TUI — State-Only):

  User presses 'a' on profile          User confirms Enable Module
         │                                      │
         ▼                                      ▼
  activate_selected_profile()        execute_confirm_action()
  actions.rs L198                    actions.rs L96
         │                                      │
         ▼                                      ▼
  sm.set_active_profile()            sm.enable_module()
  [State JSON updated]               [State JSON updated]
  [NO symlinks, NO hooks]            [NO symlinks, NO hooks]


EXPECTED (CLI Path — Full Service):

  iron profile select <id>            iron module enable <id>
         │                                      │
         ▼                                      ▼
  profile_service.apply()            module_service.enable()
  services/profile.rs L144           services/module.rs L199
         │                                      │
         ▼                                      ▼
  ┌─ For each module:─────┐          ┌─ Full enable sequence:──┐
  │  module_service.enable │          │  check_conflicts()      │
  │  (conflict check,     │          │  pre_install hook       │
  │   hooks, symlinks,    │          │  create_symlink() ×N    │
  │   state update)       │          │  state.enable_module()  │
  └────────┬───────────────┘          │  post_install hook     │
           ▼                          └─────────┬──────────────┘
  sm.set_active_profile()                       │
                                                ▼
                                     [Symlinks created, hooks run,
                                      state updated]
```

### CLI vs TUI Parity Matrix

| Operation | CLI | TUI | Parity |
|-----------|-----|-----|--------|
| Create profile | ✅ `profile create` (inline TOML) | ✅ ProfileBuilder (template TOML) | ⚠️ Different TOML format |
| List profiles | ✅ `profile list` | ✅ Profiles view | ✅ |
| Show profile | ✅ `profile show` (+ `--effective`) | ✅ ProfileDetail view | ✅ |
| Activate profile | ✅ `profile select` → `apply()` | ❌ State-only (Bug 4.1) | ❌ |
| Edit profile | ✅ `profile edit` (opens $EDITOR) | ❌ Not implemented | — |
| Create module | ❌ Not implemented (Bug 4.3) | ✅ ModuleCreator (template TOML) | ❌ |
| List modules | ✅ `module list` (+ filters) | ✅ Modules view | ✅ |
| Show module | ✅ `module show` | ✅ ModuleDetail view | ✅ |
| Enable module | ✅ `module enable` → `enable()` | ❌ State-only (Bug 4.2) | ❌ |
| Disable module | ✅ `module disable` → `disable()` | ❌ State-only (Bug 4.2) | ❌ |

---

## 6. Test Coverage Analysis

### Existing Test Counts

| Component | File | Tests | Coverage Notes |
|-----------|------|-------|----------------|
| Profile model | `profile.rs` | **0** | No `#[cfg(test)]` block at all |
| Module model | `module.rs` | 13 | Good: load/save roundtrip, fields, edge cases |
| ProfileService | `services/profile.rs` | 11 | discover, load, apply, unapply, state, inheritance |
| ModuleService | `services/module.rs` | 14 | discover, load, enable, disable, conflicts, status |
| Templates | `templates.rs` | 4 | profile_toml, module_toml output validation |
| TUI actions | `actions.rs` | Tests in `mod tests` | Covers some action functions |
| TUI handlers | `handlers.rs` | — | No dedicated handler unit tests |

### Tests Needed for Bug Fixes

If the bugs in Section 4 are fixed, the following tests should be added:

**For Bug 4.1 — TUI Profile Activation**:

| Test | Description |
|------|-------------|
| `test_activate_profile_calls_apply` | Verify `ProfileService::apply()` is invoked, not just state |
| `test_activate_profile_enables_modules` | After activation, modules should be `Installed` |
| `test_activate_profile_creates_symlinks` | Dotfile symlinks should exist post-activation |
| `test_activate_profile_unapplies_previous` | Previous profile's modules should be disabled |

**For Bug 4.2 — TUI Module Enable/Disable**:

| Test | Description |
|------|-------------|
| `test_enable_module_calls_service` | Verify `ModuleService::enable()` is invoked |
| `test_enable_module_creates_symlinks` | Dotfile symlinks should exist after enable |
| `test_enable_module_runs_hooks` | Pre/post install hooks should execute |
| `test_enable_module_checks_conflicts` | Conflicting module should produce error |
| `test_disable_module_removes_symlinks` | Dotfile symlinks should be removed |

**For Bug 4.3 — CLI Module Create**:

| Test | Description |
|------|-------------|
| `test_cli_module_create` | `iron module create foo` writes `modules/foo/module.toml` |
| `test_cli_module_create_with_packages` | Packages list appears in TOML |
| `test_cli_module_create_duplicate` | Error if module dir already exists |

**For Issue 4.5 — Profile Model Tests**:

| Test | Description |
|------|-------------|
| `test_profile_load_save_roundtrip` | Write then load, all fields match |
| `test_profile_load_missing_file` | Appropriate error variant returned |
| `test_profile_load_invalid_toml` | Parse error with meaningful message |
| `test_profile_optional_fields_none` | Extends/for_bundle/theme/shell default to None |
| `test_profile_all_modules` | `all_modules()` returns correct list |

### Test File Locations

| Component | Where Tests Should Live |
|-----------|------------------------|
| Profile model tests | `crates/iron-core/src/profile.rs` — add `#[cfg(test)] mod tests` |
| Profile activation TUI | `crates/iron-tui/src/app/actions.rs` — extend existing `mod tests` |
| Module enable/disable TUI | `crates/iron-tui/src/app/actions.rs` — extend existing `mod tests` |
| CLI module create | `crates/iron-cli/src/commands/module.rs` or integration tests |

---

## Appendix A: Key File Reference

| File | Lines | Purpose |
|------|-------|---------|
| `crates/iron-core/src/profile.rs` | 75 | Profile struct, ProfileState, load/save |
| `crates/iron-core/src/module.rs` | 317 | Module struct, ModuleKind, DotfileMapping, 13 tests |
| `crates/iron-core/src/services/profile.rs` | 577 | ProfileService trait + impl, 11 tests |
| `crates/iron-core/src/services/module.rs` | 665 | ModuleService trait + impl, 14 tests |
| `crates/iron-core/src/templates.rs` | 213 | TOML generators: module_toml, profile_toml, bundle_toml |
| `crates/iron-tui/src/app/mod.rs` | 809 | App struct with builder/creator state fields |
| `crates/iron-tui/src/app/actions.rs` | 1511 | All TUI actions incl. create/activate/enable |
| `crates/iron-tui/src/app/handlers.rs` | 1590 | All TUI key handlers incl. wizard steps |
| `crates/iron-tui/src/ui/profile_builder.rs` | 213 | ProfileBuilder 3-step wizard render |
| `crates/iron-tui/src/ui/module_creator.rs` | 186 | ModuleCreator 2-step wizard render |
| `crates/iron-tui/src/ui/profiles.rs` | — | Profiles list + detail views |
| `crates/iron-tui/src/ui/modules.rs` | — | Modules list + detail views |
| `crates/iron-cli/src/commands/profile.rs` | 293 | CLI: list, show, select, create, edit |
| `crates/iron-cli/src/commands/module.rs` | 300 | CLI: list, show, enable, disable (no create) |

## Appendix B: Summary of Actions Required

### Close These Tasks

| Task | Reason |
|------|--------|
| **S1-P5-001** | ProfileBuilder persists to disk via `std::fs::write()`. Confirmed working. |
| **S1-P5-002** | ModuleCreator persists to disk via `std::fs::write()`. Confirmed working. |

### New Tasks to File

| ID | Priority | Title | Category |
|----|----------|-------|----------|
| S1-P5-NEW-001 | **P1** | Fix TUI profile activation — call `ProfileService::apply()` | Bug fix |
| S1-P5-NEW-002 | **P1** | Fix TUI module enable/disable — call `ModuleService::enable()`/`disable()` | Bug fix |
| S1-P5-NEW-003 | **P2** | Add CLI `module create` command | Feature gap |
| S1-P5-NEW-004 | **P2** | Unify CLI profile creation to use `templates::profile_toml()` | Consistency |
| S1-P5-NEW-005 | **P2** | Validate/sanitise profile and module IDs in TUI wizards | Input validation |
| S1-P5-NEW-006 | **P2** | Add dotfile mapping configuration to ModuleCreator wizard | Feature gap |
| S1-P5-NEW-007 | **P2** | Add Profile model unit tests (0 tests currently) | Test coverage |
| S1-P5-NEW-008 | **P3** | ProfileBuilder: dependency auto-suggestion in module selection | Enhancement |
| S1-P5-NEW-009 | **P3** | ProfileBuilder: conflict warnings during module selection | Enhancement |
| S1-P5-NEW-010 | **P3** | ModuleCreator: add ModuleKind selection | Enhancement |
| S1-P5-NEW-011 | **P3** | ModuleCreator: add conflict/depends fields | Enhancement |
| S1-P5-NEW-012 | **P3** | Check for duplicate profile/module names before creation | Enhancement |
| S1-P5-NEW-013 | **P3** | Show guidance when module list is empty in ProfileBuilder | UX |
