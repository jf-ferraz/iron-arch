# Scenario 1 — Phase 2: Host Selection

## Implementation Guideline (Deep Dive)

> **Scope**: Tasks S1-P2-001, S1-P2-002, S1-P2-003 from `docs/TODO-scenario1.md`
> **Phase**: Host Selection
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-tui, iron-core, iron-cli and integration boundaries

---

## Table of Contents

1. [Phase 2 Architecture Overview](#1-phase-2-architecture-overview)
2. [Task S1-P2-001 — Create HostSelection TUI View](#2-task-s1-p2-001)
3. [Task S1-P2-002 — Wire Host Selection into First-Launch Flow](#3-task-s1-p2-002)
4. [Task S1-P2-003 — Add `iron host select` CLI Enhancement](#4-task-s1-p2-003)
5. [Discovered Issues — Outside Phase 2 Scope](#5-discovered-issues)
6. [Integration Map](#6-integration-map)
7. [Test Coverage Analysis](#7-test-coverage-analysis)

---

## 1. Phase 2 Architecture Overview

### The Host Model

Iron manages multiple machines via a **Host** abstraction. Each host is a physical or
virtual machine with unique hardware, bundles, and profiles. The data hierarchy is:

```
HOST (machine identity)
  ├─ BUNDLE (desktop environment: hyprland, niri, etc.)
  │    ├─ Packages
  │    ├─ Dotfiles (symlinked via stow)
  │    └─ Services (systemd units)
  ├─ PROFILE (curated module set: developer, minimal, etc.)
  │    └─ MODULES (individual config units: nvim-ide, tmux-config, etc.)
  └─ State (active_bundle, active_profile, active_modules)
```

State is **keyed per host** — `active_bundles: HashMap<String, String>` maps `host_id → bundle_id`,
and `active_profiles: HashMap<String, String>` maps `host_id → profile_id`.

### How Hosts Exist Today

**Two coexisting conventions** for host config files:

| Convention | Path | Used By | Fields |
|-----------|------|---------|--------|
| **Flat file** | `hosts/{id}.toml` | `DefaultHostService` | Basic: id, name, hardware, installed_bundles |
| **Directory** | `hosts/{id}/host.toml` | `Host::load()` | Rich: + description, install_params, active_bundle |

The `DefaultHostService::list_hosts()` scans **only flat `*.toml` files** in `hosts/` — it
does NOT descend into subdirectories. This means `hosts/desktop/host.toml` is invisible to
the service's discovery function.

**Current workspace** has both:
- `hosts/desktop.toml` (flat — visible to `list_hosts()`)
- `hosts/desktop/host.toml` (directory — invisible to `list_hosts()`)

This duality is a source of potential bugs and should be unified, but is outside Phase 2 scope.

### Current Host Selection Flow

```
User launches iron (CLI or TUI)
  │
  ├─ TUI path: App::init()
  │    ├─ StateManager::new() → reads state.json
  │    ├─ sm.current_host() → returns Option<String>
  │    │    ├─ Some("desktop") → load bundles for "desktop" → Dashboard
  │    │    └─ None → Dashboard (empty state, no wizard) ← GAP
  │    └─ Err → SetupWizard → Wizard HostSetup = TEXT INPUT (not selection)
  │
  └─ CLI path: `iron host select <id>`
       ├─ id is REQUIRED (not optional)
       ├─ load_host(id) → verify exists
       └─ set_current_host(id) → persist to state.json
```

**No host selection UI exists anywhere.** The wizard uses text input for a host ID string.
The CLI requires the exact host ID as an argument. Neither path presents a list of
discovered hosts for the user to choose from.

### Key Components

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| `Host` struct | `crates/iron-core/src/host.rs` | L8–30 | Host data model (id, name, hardware, bundles) |
| `HardwareSpec` | `crates/iron-core/src/host.rs` | L33–49 | CPU, GPU, RAM, monitors, chassis |
| `ChassisType` | `crates/iron-core/src/host.rs` | L66–73 | Desktop, Laptop, Server, Tablet, Convertible, Unknown |
| `HostService` trait | `crates/iron-core/src/services/host.rs` | L12–39 | 9 methods: detect*, load*, save*, list*, find*, create* |
| `DefaultHostService` | `crates/iron-core/src/services/host.rs` | L42–320 | Real implementation scanning `hosts/*.toml` |
| `StateManager` | `crates/iron-core/src/services/state.rs` | L90–170 | `current_host()`, `set_current_host()`, `active_bundle(host_id)` |
| `IronState` | `crates/iron-core/src/state.rs` | L149–175 | `current_host: Option<String>`, per-host HashMaps |
| `View` enum | `crates/iron-tui/src/app/mod.rs` | L183–226 | 23 variants (no `HostSelection`) |
| `App` struct | `crates/iron-tui/src/app/mod.rs` | L36–178 | `current_host: Option<String>`, no host list field |
| `WizardState` | `crates/iron-tui/src/wizard.rs` | L69–90 | `host_id: String`, `detect_host()` — text-based |
| `HostAction` enum | `crates/iron-cli/src/cli.rs` | L332–358 | `Select { id: String }` — required argument |
| `host::select()` | `crates/iron-cli/src/commands/host.rs` | L267–280 | Direct ID lookup, no interactive mode |

---

## 2. Task S1-P2-001

### Create `HostSelection` TUI View

| Field | Value |
|-------|-------|
| **ID** | S1-P2-001 |
| **Priority** | P3 (Low) |
| **Status** | ❌ Not Started |
| **Estimated Effort** | 4–5 hours |

### 2.1 Requirements (from user-workflow.md)

The spec describes a host picker view at `user-workflow.md:220–249`:

```
┌── Identified Hosts ────────────────────────┐
│                                             │
│  > [●] desktop    AMD Ryzen 7 · RX 9060 XT │
│    [ ] laptop     Intel i7 · Intel Iris     │
│    [ ] server     AMD EPYC · No GPU         │
│                                             │
│  [Enter] Select    [c] Create new    [q] Quit│
└─────────────────────────────────────────────┘
```

Key behaviors:
- List all discovered hosts from `hosts/*.toml`
- Show current host as `●`, others as `○`
- Display hardware summary (CPU + GPU abbreviated)
- Up/down navigation, Enter to select
- `[c]` to create a new host (triggers wizard)
- If only 1 host exists, skip this view entirely

### 2.2 View System Pattern — How to Add a New View

Adding a new view requires changes to **5 locations** across 3 files. This pattern is
consistent across all 23 existing views.

#### Step 1: Add `View::HostSelection` variant

File: `crates/iron-tui/src/app/mod.rs`, line 226 (after `ModuleCreator`):

```rust
/// Host selection for multi-machine setups
HostSelection,
```

#### Step 2: Add `App` state fields for the host list

File: `crates/iron-tui/src/app/mod.rs`, inside `pub struct App` (after `current_host` at L48):

```rust
/// Discovered hosts for HostSelection view
pub discovered_hosts: Vec<iron_core::host::Host>,
```

And in the `Default` impl (after `current_host: None` at L286):

```rust
discovered_hosts: Vec::new(),
```

#### Step 3: Create the render module

File: `crates/iron-tui/src/ui/host_selection.rs` (new file):

**Template**: The Bundles list view at `crates/iron-tui/src/ui/bundles.rs` (144 lines) is
the closest template — it shows a list of items with active/inactive markers, selection
highlighting, and a detail block.

The proposed layout:

```
┌── Host Selection ──────────────────────────────────────┐
│  Select a host to activate:                             │
│                                                         │
│  ● desktop — Desktop Workstation                        │
│    AMD Ryzen 7 9800X3D · RX 9060 XT · 30 GB · Desktop  │
│                                                         │
│  ○ laptop — Development Laptop                          │
│    Intel i7-1370P · Intel Iris · 16 GB · Laptop         │
│                                                         │
│  [Enter] Select  [c] Create new  [Esc] Back             │
└─────────────────────────────────────────────────────────┘
```

Implementation details:

```rust
// crates/iron-tui/src/ui/host_selection.rs

use crate::app::App;
use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

pub fn render_host_selection(frame: &mut Frame, area: Rect, app: &App) {
    let block = theme::themed_block("Host Selection", theme::MAUVE);

    if app.discovered_hosts.is_empty() {
        // Empty state: "No hosts found. [c] to create one."
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No hosts configured.", ...)),
            Line::from("Press [c] to create a new host, or [w] for setup wizard."),
        ])
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app.discovered_hosts.iter().enumerate().map(|(i, host)| {
        let is_current = app.current_host.as_ref() == Some(&host.id);
        let marker = if is_current { "●" } else { "○" };

        // Build hardware summary: "CPU · GPU · RAM · Chassis"
        let hw = &host.hardware;
        let cpu_short = hw.cpu.as_deref().unwrap_or("Unknown CPU")
            .split_whitespace().take(4).collect::<Vec<_>>().join(" ");
        let gpu_short = hw.gpu.as_deref().unwrap_or("No GPU")
            .rsplit(']').next().unwrap_or("Unknown GPU").trim();
        let ram = hw.ram_mb.map(|r| format!("{} GB", r / 1024)).unwrap_or_default();
        let chassis = hw.chassis.as_ref().map(|c| format!("{:?}", c)).unwrap_or_default();

        let line1 = format!("{} {} — {}", marker, host.id, host.name);
        let line2 = format!("  {} · {} · {} · {}", cpu_short, gpu_short, ram, chassis);

        let style = if i == app.selected_index { theme::selected() } else { theme::unselected() };

        ListItem::new(vec![
            Line::styled(line1, style),
            Line::styled(line2, Style::default().fg(theme::OVERLAY)),
        ])
    }).collect();

    let list = List::new(items).block(block).highlight_symbol("▸ ");
    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}
```

#### Step 4: Register in the render dispatch

File: `crates/iron-tui/src/ui/mod.rs`:

1. Add module declaration (after `mod wizard;` at L23):
   ```rust
   mod host_selection;
   ```

2. Add re-export (after `pub use wizard::render_setup_wizard;` at L48):
   ```rust
   pub use host_selection::render_host_selection;
   ```

3. Add match arm in `render()` (after `View::SetupWizard =>` at L72):
   ```rust
   View::HostSelection => render_host_selection(frame, layout[1], app),
   ```

#### Step 5: Register in the handler dispatch

File: `crates/iron-tui/src/app/handlers.rs`:

1. Add view-specific key handler in the `match self.view` block (at L97):
   ```rust
   View::HostSelection => match key.code {
       KeyCode::Enter => {
           if let Some(host) = app.discovered_hosts.get(app.selected_index) {
               if let Some(ref sm) = self.state_manager {
                   let _ = sm.set_current_host(&host.id);
                   let _ = self.init(); // Reload state for new host
                   self.set_status(format!("Switched to host '{}'", host.id));
               }
           }
           true
       }
       KeyCode::Char('c') => {
           self.view = View::SetupWizard;
           self.init_wizard();
           true
       }
       _ => false,
   },
   ```

2. Add to `cycle_view_forward()` (at L665):
   ```rust
   View::HostSelection => View::Dashboard,
   ```

3. Add to `cycle_view_backward()` (at L698):
   ```rust
   View::HostSelection => View::Dashboard,
   ```

4. Add to `current_list_len()` (at L724):
   ```rust
   View::HostSelection => self.discovered_hosts.len(),
   ```

### 2.3 Host Discovery Integration

The TUI currently does **not** use `HostService` at all. The wizard does its own hostname
detection via `detect_host()` (reads `/etc/hostname` or `$HOSTNAME`). To populate the
HostSelection view, a new action method is needed.

File: `crates/iron-tui/src/app/actions.rs`:

```rust
/// Load discovered hosts from disk
pub fn load_hosts(&mut self) {
    use iron_core::services::host::{DefaultHostService, HostService};
    let host_service = DefaultHostService::new(&self.config_dir);
    self.discovered_hosts = host_service.list_hosts().unwrap_or_default();
}
```

**Dependency check**: `iron-tui` already depends on `iron-core` (see `Cargo.toml`), and
`iron-core` re-exports `DefaultHostService` and `HostService` from
`crates/iron-core/src/services/mod.rs`. No new crate dependencies needed.

**Import**: Add `use iron_core::services::host::{DefaultHostService, HostService};` to
`actions.rs` (or use it inline).

This should be called:
1. On `init()` — after state loads successfully, before view decision
2. When navigating to `View::HostSelection` (via keybinding or flow redirect)
3. When returning from setup wizard (hosts may have changed)

### 2.4 Navigation Keybinding

The user-workflow spec says HostSelection should be accessible. A global keybinding needs
to be added.

File: `crates/iron-tui/src/app/handlers.rs`, in the general key handling block (at L380):

```rust
KeyCode::Char('H') => {
    self.load_hosts();
    self.navigate(View::HostSelection);
}
```

`Shift+H` is currently unused. This follows the existing pattern where lowercase letters
navigate to primary views and uppercase to secondary views (`S` → Secrets, `R` → Recovery).

### 2.5 Complexity Analysis

| Aspect | Complexity | Notes |
|--------|-----------|-------|
| View enum + fields | Trivial | 3 lines added to `mod.rs` |
| Render function | Low | ~80 lines, template from `bundles.rs` |
| Handler dispatch | Low | ~20 lines, standard pattern |
| host discovery | Low | 5 lines, existing `HostService::list_hosts()` |
| Cycle/nav registration | Trivial | 6 match arms to add |
| Tests | Medium | Render tests + handler tests (see Section 7) |
| **Total** | **Low-Medium** | Straightforward new view, all infrastructure exists |

### 2.6 Files Changed Summary

| File | Change Type | Lines |
|------|------------|-------|
| `crates/iron-tui/src/ui/host_selection.rs` | **NEW** | ~80–100 |
| `crates/iron-tui/src/app/mod.rs` | Edit | +5 (View variant, App field, Default) |
| `crates/iron-tui/src/ui/mod.rs` | Edit | +3 (mod, pub use, render match arm) |
| `crates/iron-tui/src/app/handlers.rs` | Edit | +25 (view handler, cycle, list_len, keybinding) |
| `crates/iron-tui/src/app/actions.rs` | Edit | +8 (load_hosts method) |
| `crates/iron-tui/src/ui/tests.rs` | Edit | +40 (render tests) |

---

## 3. Task S1-P2-002

### Wire Host Selection into First-Launch Flow

| Field | Value |
|-------|-------|
| **ID** | S1-P2-002 |
| **Priority** | P3 (Low) |
| **Status** | ❌ Not Started |
| **Estimated Effort** | 1.5–2 hours |
| **Dependency** | S1-P2-001 |

### 3.1 Requirements

Users with multiple host configs should see the HostSelection view:
1. **On first launch** — after wizard completes, if >1 host exists
2. **On subsequent launches** — if >1 host exists AND `current_host` is not set

When only 1 host exists, it should be auto-selected and the view skipped.

### 3.2 Insertion Points

There are **three** code locations where host selection logic should integrate:

#### Point A: After Wizard Completion (handlers.rs)

File: `crates/iron-tui/src/app/handlers.rs`, lines 500–508:

```rust
// CURRENT CODE:
WizardStep::Confirmation => match key.code {
    KeyCode::Enter | KeyCode::Char('y') => {
        if let Ok(()) = self.wizard.apply(&self.config_dir) {
            let _ = self.init();
            self.view = View::Dashboard;     // ← INSERTION POINT
            self.set_status("Setup complete! Welcome to Iron.");
        }
    }
```

**Proposed change**:

```rust
WizardStep::Confirmation => match key.code {
    KeyCode::Enter | KeyCode::Char('y') => {
        if let Ok(()) = self.wizard.apply(&self.config_dir) {
            let _ = self.init();
            self.load_hosts();
            if self.discovered_hosts.len() > 1 {
                self.view = View::HostSelection;
                self.set_status("Setup complete! Select your host.");
            } else {
                self.view = View::Dashboard;
                self.set_status("Setup complete! Welcome to Iron.");
            }
        }
    }
```

**Complexity**: Low. The only risk is that `load_hosts()` might return 0 hosts if the
wizard didn't save a host TOML file. This is possible because `wizard.apply()` calls
`set_current_host(host_id)` (persists to `state.json`) but does NOT call
`host_service.create_from_current()` or `host_service.save_host()` — it never creates a
`hosts/{id}.toml` file.

**Implication**: After the wizard, `list_hosts()` may return 0 or only pre-existing hosts.
The host the user just configured via the wizard may not appear in `list_hosts()`. This is
a **design gap** — see Section 5.1.

#### Point B: On App::init() (actions.rs)

File: `crates/iron-tui/src/app/actions.rs`, lines 14–36:

```rust
// CURRENT CODE in Ok(sm) branch:
self.current_host = sm.current_host();
// ...load bundles, profiles...
self.state_manager = Some(sm);
```

**Proposed change** — after loading state but before proceeding to dashboard:

```rust
Ok(sm) => {
    self.current_host = sm.current_host();
    self.state_manager = Some(sm);

    // Check if host selection is needed
    self.load_hosts();
    if self.current_host.is_none() && self.discovered_hosts.len() > 1 {
        self.view = View::HostSelection;
        return Ok(());
    }
    // Auto-select if only one host exists and none is set
    if self.current_host.is_none() && self.discovered_hosts.len() == 1 {
        if let Some(ref sm) = self.state_manager {
            let host_id = &self.discovered_hosts[0].id;
            let _ = sm.set_current_host(host_id);
            self.current_host = Some(host_id.clone());
        }
    }

    // Continue with normal loading...
    if let Some(ref host_id) = self.current_host {
        // ...existing bundle/profile loading code...
    }
}
```

**Sequence**:
1. State loads OK
2. `current_host` is `None` (fresh/reset state)
3. Scan `hosts/` directory → N hosts found
4. If N > 1 → show `HostSelection`
5. If N == 1 → auto-select that host
6. If N == 0 → proceed to Dashboard (or show wizard, per Phase 1 gap S1-P1-004)

#### Point C: On Host Selection Confirmation (handlers.rs)

When the user selects a host in the HostSelection view:

```rust
View::HostSelection => match key.code {
    KeyCode::Enter => {
        if let Some(host) = self.discovered_hosts.get(self.selected_index) {
            if let Some(ref sm) = self.state_manager {
                let _ = sm.set_current_host(&host.id);
            }
            let _ = self.init(); // Reload everything for new host
            self.view = View::Dashboard;
            self.set_status(format!("Host '{}' activated.", host.id));
        }
        true
    }
```

After selecting a host, `init()` is called to reload bundles/profiles/modules for the
newly selected host. This is the same pattern used after wizard completion.

### 3.3 State Reload After Host Switch

When switching hosts, the following state must be reloaded:

| State | Source | Keyed by Host | Action |
|-------|--------|--------------|--------|
| `current_host` | `state.json` | N/A | `sm.set_current_host(id)` |
| `active_bundle` | `state.json` | `active_bundles[host_id]` | `sm.active_bundle(host_id)` |
| `active_profile` | `state.json` | `active_profiles[host_id]` | `sm.active_profile(host_id)` |
| `bundles` | `bundles/*.toml` | No (shared) | `bundle_service.discover()` |
| `profiles` | `profiles/*/profile.toml` | No (shared) | `load_profiles()` |
| `modules` | `modules/*/module.toml` | No (shared) | `load_modules()` |
| `active_modules` | `state.json` | Global (not per-host) | `sm.active_modules()` |

**Key observation**: Bundles, profiles, and modules are **shared** across hosts (same
directories), but the **active selections** are per-host. Calling `self.init()` handles
all of this correctly because it reads from `sm.current_host()` → `sm.active_bundle(host_id)`
→ etc.

### 3.4 Edge Cases

| Case | Current Behavior | Expected Behavior | Requires |
|------|-----------------|-------------------|----------|
| 0 hosts configured | Dashboard (empty) | Show wizard or prompt | S1-P1-004 |
| 1 host configured | Dashboard | Skip selection, auto-set | S1-P2-002 Point B |
| 1 host, already set | Dashboard | No change needed | — |
| N hosts, none set | Dashboard (empty) | Show HostSelection | S1-P2-002 Point B |
| N hosts, one set | Dashboard (normal) | Dashboard (normal) | — |
| Host deleted after set | Dashboard (stale) | Host not found handling | NEW (see 5.2) |

### 3.5 Decision: When to Show HostSelection

The user-workflow spec says: "shown only when multiple hosts exist." Two interpretations:

**Option A — Show on every launch** (if >1 host):
- Always shows HostSelection before Dashboard
- User must select/confirm every time
- More intrusive, but clearer context

**Option B — Show only when `current_host` is None** (recommended):
- Only shows HostSelection when no host is set
- Once set, goes directly to Dashboard
- Less intrusive, can still access via `Shift+H`

**Recommendation**: Option B. It matches the spec text "surfaces the multi-machine model
early" without being annoying on repeat launches. The keybinding `Shift+H` handles the
re-selection use case.

### 3.6 Files Changed Summary

| File | Change Type | Lines |
|------|------------|-------|
| `crates/iron-tui/src/app/actions.rs` | Edit | +15 (init() host check, auto-select) |
| `crates/iron-tui/src/app/handlers.rs` | Edit | +8 (wizard completion redirect) |
| `crates/iron-tui/src/app/actions.rs` | Edit (tests) | +20 (flow tests) |

---

## 4. Task S1-P2-003

### Add Interactive `iron host select` CLI Command

| Field | Value |
|-------|-------|
| **ID** | S1-P2-003 |
| **Priority** | P3 (Low) |
| **Status** | ❌ Not Started |
| **Estimated Effort** | 1.5–2 hours |

### 4.1 Current Implementation

The CLI already has a `host select` command at `crates/iron-cli/src/commands/host.rs:267–280`:

```rust
fn select(ctx: &AppContext, id: &str) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();
    let host = host_service.load_host(id)?;
    output.info(&format!("Selecting host: {}", host.name));
    ctx.state.set_current_host(id)?;
    output.success(&format!("Switched to host '{}'", id));
    Ok(())
}
```

And the Clap definition at `crates/iron-cli/src/cli.rs:352–355`:

```rust
/// Select active host
Select {
    /// Host ID
    id: String,
},
```

**Current usage**: `iron host select desktop` — requires exact host ID.
**Desired usage**: `iron host select` (no arg) → interactive list, OR `iron host select desktop`.

### 4.2 Change A — Make `id` Optional

File: `crates/iron-cli/src/cli.rs`, line 354:

```rust
// BEFORE:
Select {
    /// Host ID
    id: String,
},

// AFTER:
Select {
    /// Host ID (omit for interactive selection)
    id: Option<String>,
},
```

### 4.3 Change B — Update Dispatch

File: `crates/iron-cli/src/commands/host.rs`, line 36:

```rust
// BEFORE:
HostAction::Select { id } => select(ctx, &id),

// AFTER:
HostAction::Select { id } => {
    match id {
        Some(id) => select(ctx, &id),
        None => select_interactive(ctx),
    }
}
```

### 4.4 Change C — Add Interactive Selection Function

File: `crates/iron-cli/src/commands/host.rs` (new function):

```rust
/// Interactive host selection
fn select_interactive(ctx: &AppContext) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();
    let hosts = host_service.list_hosts()?;

    if hosts.is_empty() {
        output.warning("No hosts configured");
        output.info("Run 'iron init' to configure a host");
        return Ok(());
    }

    if hosts.len() == 1 {
        let host = &hosts[0];
        output.info(&format!("Only one host found: {} ({})", host.id, host.name));
        ctx.state.set_current_host(&host.id)?;
        output.success(&format!("Selected host '{}'", host.id));
        return Ok(());
    }

    let current = ctx.current_host();

    output.header("Select a host");
    for (i, host) in hosts.iter().enumerate() {
        let is_current = current.as_ref() == Some(&host.id);
        let marker = if is_current { "●" } else { "○" };
        let hw_summary = host.hardware.cpu.as_deref().unwrap_or("Unknown");
        println!("  {} [{}] {} — {} ({})", marker, i + 1, host.id, host.name, hw_summary);
    }

    println!();
    print!("Enter number (1-{}): ", hosts.len());
    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let choice: usize = input.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;

    if choice < 1 || choice > hosts.len() {
        anyhow::bail!("Selection out of range");
    }

    let selected = &hosts[choice - 1];
    ctx.state.set_current_host(&selected.id)?;
    output.success(&format!("Switched to host '{}' ({})", selected.id, selected.name));

    Ok(())
}
```

### 4.5 Alternative: Use `dialoguer` Crate

The current CLI does NOT depend on `dialoguer` (checked `Cargo.toml`). Adding it would
provide a polished arrow-key selection UI:

```rust
use dialoguer::Select;

fn select_interactive(ctx: &AppContext) -> Result<()> {
    let hosts = ctx.host_service().list_hosts()?;
    let items: Vec<String> = hosts.iter()
        .map(|h| format!("{} — {}", h.id, h.name))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a host")
        .items(&items)
        .default(0)
        .interact()?;

    ctx.state.set_current_host(&hosts[selection].id)?;
    Ok(())
}
```

**Trade-off**: Adding `dialoguer` (~50KB) as a dependency for one feature may not be
justified. The manual stdin approach works and requires no new dependencies.

**Recommendation**: Start with stdin approach. Consider `dialoguer` if more interactive
CLI prompts are needed in the future across the codebase.

### 4.6 Change D — Update CLI Test

File: `crates/iron-cli/src/cli.rs`, update existing test (at L715):

```rust
// CURRENT:
#[test]
fn test_cli_host_select() {
    let cli = Cli::try_parse_from(["iron", "host", "select", "desktop"]).unwrap();
    match cli.command {
        Some(Commands::Host { action: HostAction::Select { id } }) => {
            assert_eq!(id, "desktop");
        }
        _ => panic!("Expected host select"),
    }
}

// NEW (additional test):
#[test]
fn test_cli_host_select_interactive() {
    let cli = Cli::try_parse_from(["iron", "host", "select"]).unwrap();
    match cli.command {
        Some(Commands::Host { action: HostAction::Select { id } }) => {
            assert!(id.is_none());
        }
        _ => panic!("Expected host select with no id"),
    }
}
```

### 4.7 Complexity Analysis

| Aspect | Complexity | Notes |
|--------|-----------|-------|
| Clap argument change | Trivial | `String` → `Option<String>` |
| Dispatch update | Trivial | Match on `Some`/`None` |
| Interactive function | Low | ~40 lines, stdin-based |
| Test updates | Low | 1 existing test update + 1 new test |
| **Total** | **Low** | No new dependencies, standard pattern |

### 4.8 Files Changed Summary

| File | Change Type | Lines |
|------|------------|-------|
| `crates/iron-cli/src/cli.rs` | Edit | +1 (`String` → `Option<String>`) |
| `crates/iron-cli/src/commands/host.rs` | Edit | +45 (dispatch update, `select_interactive()`) |
| `crates/iron-cli/src/cli.rs` | Edit (tests) | +12 (parse test update + new test) |

---

## 5. Discovered Issues — Outside Phase 2 Scope

### 5.1 Wizard Does Not Create Host TOML File

**Severity**: Medium
**Location**: `crates/iron-tui/src/wizard.rs:324–369` (`apply()`)

**Problem**: When the wizard completes, `apply()` calls
`state_manager.set_current_host(&self.host_id)` — this writes the host ID string to
`state.json`. However, it does NOT create a corresponding `hosts/{id}.toml` file.

**Impact**: After first-time wizard:
- `state.json` has `current_host: "desktop"`
- `hosts/desktop.toml` does NOT exist (unless it was pre-populated)
- `DefaultHostService::list_hosts()` returns 0 hosts ← contradicts state
- `host_service.load_host("desktop")` fails with `HostNotFound`
- HostSelection view (S1-P2-001) would show empty list

**Recommended fix**:

```rust
// In wizard.apply(), after set_current_host():
let host_service = DefaultHostService::new(config_dir);
if host_service.load_host(&self.host_id).is_err() {
    // Host doesn't exist on disk — create it from current hardware
    let _ = host_service.create_from_current(&self.host_id, &self.host_id);
}
```

**Suggested task ID**: `S1-P2-004` | **P1** | Wizard apply() should create host TOML

### 5.2 Dual Host Config Convention (Flat vs Directory)

**Severity**: Low
**Location**: `crates/iron-core/src/services/host.rs` and `crates/iron-core/src/host.rs`

**Problem**: Two conventions exist:

| Convention | Path | Discovery | Used by |
|-----------|------|-----------|---------|
| Flat | `hosts/{id}.toml` | `list_hosts()` ✅ | `DefaultHostService` |
| Directory | `hosts/{id}/host.toml` | NOT discovered ❌ | `Host::load()` |

The workspace has **both** `hosts/desktop.toml` AND `hosts/desktop/host.toml` with
different data. `DefaultHostService::list_hosts()` only finds the flat file.

**Impact**: If a user creates hosts using the directory convention (following the
`hosts/desktop/host.toml` example), `list_hosts()` won't find them, and the
HostSelection view will show an incomplete list.

**Recommended fix** (one of):
- **A**: Extend `list_hosts()` to also scan `hosts/*/host.toml` directories
- **B**: Standardize on flat `hosts/{id}.toml` — document and migrate
- **C**: Standardize on directory `hosts/{id}/host.toml` — update `DefaultHostService`

**Suggested task ID**: `S1-P2-005` | **P2** | Unify host config convention (flat vs dir)

### 5.3 Host Deletion / Stale Reference

**Severity**: Low
**Location**: `crates/iron-tui/src/app/actions.rs` (`init()`)

**Problem**: If `state.json` references `current_host: "laptop"` but
`hosts/laptop.toml` was deleted, `init()` will:
1. Set `self.current_host = Some("laptop")`
2. Try to load active bundle for "laptop" — succeeds (from state), but bundle
   doesn't exist on disk → `discover()` returns empty or partial list
3. No error shown to user

**Recommended**: Add a validation check in `init()` — if `current_host` is set but
`host_service.load_host(id)` fails, clear it and prompt for re-selection.

**Suggested task ID**: `S1-P2-006` | **P3** | Handle stale host reference in state

### 5.4 `switch_host()` Method Not Needed

The user-workflow spec mentions `StateManager` needing a `switch_host()` method. However,
`set_current_host()` already does exactly this — it sets the current host ID and persists.
There's no additional state to clear on switch because `init()` is called afterward and
reloads everything from scratch. No new method is needed.

### 5.5 Wizard HostSetup Does Not List Existing Hosts

**Severity**: Low
**Location**: `crates/iron-tui/src/wizard.rs` (`detect_host()`)

**Problem**: The wizard's HostSetup step auto-detects the system hostname and presents
it as a text input. It does NOT scan `hosts/` for existing host configs. If the user
has pre-existing hosts, they have no way to see or select them from the wizard.

**Impact**: Low for Phase 2 — the HostSelection view (S1-P2-001) handles this use case.
But it means the wizard and HostSelection are disconnected:

- Wizard creates a new host (text input) → might create duplicates
- HostSelection shows existing hosts → no way to create new ones inline
  (except `[c]` which redirects back to the wizard)

This creates a slight UX loop. Acceptable for now.

---

## 6. Integration Map

### 6.1 Crate Dependencies (Phase 2 Touch Points)

```
iron-cli (binary)
  │
  ├─ iron-core (application)
  │    ├─ host.rs                  Host struct, HardwareSpec, ChassisType
  │    ├─ services/host.rs         HostService trait, DefaultHostService (list, load, save)
  │    ├─ services/state.rs        StateManager (current_host, set_current_host)
  │    └─ state.rs                 IronState (current_host: Option<String>)
  │
  ├─ iron-tui (presentation)                        NEW / MODIFIED
  │    ├─ app/mod.rs               View::HostSelection (new), discovered_hosts field (new)
  │    ├─ app/actions.rs           load_hosts() (new), init() host check (modified)
  │    ├─ app/handlers.rs          HostSelection key handler (new), wizard redirect (modified)
  │    ├─ ui/host_selection.rs     render_host_selection() (new file)
  │    ├─ ui/mod.rs                module + render dispatch (modified)
  │    └─ ui/tests.rs              render tests (modified)
  │
  └─ iron-cli
       ├─ cli.rs                   HostAction::Select { id: Option<String> } (modified)
       └─ commands/host.rs         select_interactive() (new), dispatch (modified)
```

### 6.2 Data Flow: Host Selection → State Reload

```
User navigates to HostSelection (Shift+H or flow redirect)
  │
  ├─ load_hosts()
  │    └─ DefaultHostService::new(&config_dir).list_hosts()
  │         └─ Scans hosts/*.toml → Vec<Host>
  │
  ▼
User presses Enter on a host
  │
  ├─ state_manager.set_current_host(&host.id)
  │    └─ Persists to state.json: { "current_host": "laptop" }
  │
  ├─ self.init()
  │    ├─ sm.current_host() → Some("laptop")
  │    ├─ sm.active_bundle("laptop") → Some("hyprland") or None
  │    ├─ bundle_service.discover() → Vec<Bundle>
  │    ├─ load_profiles()
  │    └─ load_modules()
  │
  └─ self.view = View::Dashboard
```

### 6.3 CLI Select Flow

```
`iron host select`   (no args)
  │
  ├─ host_service.list_hosts() → Vec<Host>
  ├─ Print numbered list
  ├─ Read stdin → number
  ├─ state.set_current_host(&hosts[n].id)
  └─ Print success

`iron host select desktop`   (with args)
  │
  ├─ host_service.load_host("desktop") → verify exists
  ├─ state.set_current_host("desktop")
  └─ Print success
```

### 6.4 First-Launch Integration

```
App::init()
  │
  ├─ StateManager::new()
  │    ├─ Ok(sm), current_host: Some("desktop") → Dashboard (normal)
  │    │
  │    ├─ Ok(sm), current_host: None
  │    │    ├─ load_hosts() → N hosts
  │    │    ├─ N == 0 → Dashboard (or wizard, Phase 1 fix)
  │    │    ├─ N == 1 → auto-select → Dashboard
  │    │    └─ N > 1 → HostSelection view
  │    │
  │    └─ Err → SetupWizard
  │              └─ Wizard completes → apply()
  │                   ├─ load_hosts() → N hosts
  │                   ├─ N > 1 → HostSelection
  │                   └─ N ≤ 1 → Dashboard
```

---

## 7. Test Coverage Analysis

### 7.1 Existing Host-Related Tests

| Area | File | Test Count | Coverage |
|------|------|-----------|----------|
| `DefaultHostService` | `crates/iron-core/src/services/host.rs` | 13 | save, load, list, find, detect, overwrite |
| `StateManager.set_current_host` | `crates/iron-core/src/services/state.rs` | 5+ | set, get, persistence |
| `Host::load/save` | `crates/iron-core/src/host.rs` | ~4 | TOML round-trip |
| Wizard HostSetup render | `crates/iron-tui/src/ui/tests.rs` | 2 | step renders, edit mode |
| CLI `host list/select` parse | `crates/iron-cli/src/cli.rs` | 2 | Clap parsing |
| **Total existing** | | **~26** | |

### 7.2 Required New Tests

#### S1-P2-001 — HostSelection View Tests

| Test | Type | What to verify |
|------|------|---------------|
| `test_host_selection_renders_empty` | Render | "No hosts configured" message |
| `test_host_selection_renders_hosts` | Render | Host names, hardware summary visible |
| `test_host_selection_marks_current` | Render | `●` on current host, `○` on others |
| `test_host_selection_highlights_selected` | Render | Selection highlighting |
| `test_host_selection_enter_sets_host` | Handler | Enter on host → `set_current_host()` |
| `test_host_selection_enter_navigates_dashboard` | Handler | After select → `View::Dashboard` |
| `test_host_selection_c_opens_wizard` | Handler | `[c]` → `View::SetupWizard` |
| `test_host_selection_escape_goes_back` | Handler | Esc → previous view |
| `test_host_selection_list_len` | Handler | `current_list_len()` returns correct count |
| `test_cycle_includes_host_selection` | Handler | Tab cycles through HostSelection |

#### S1-P2-002 — First-Launch Flow Tests

| Test | Type | What to verify |
|------|------|---------------|
| `test_init_no_host_multiple_hosts_shows_selection` | Action | `current_host: None`, 2+ hosts → `View::HostSelection` |
| `test_init_no_host_single_host_auto_selects` | Action | `current_host: None`, 1 host → auto-set, Dashboard |
| `test_init_no_hosts_no_redirect` | Action | `current_host: None`, 0 hosts → Dashboard (no redirect) |
| `test_init_host_set_goes_to_dashboard` | Action | `current_host: Some`, any hosts → Dashboard (normal) |
| `test_wizard_complete_multiple_hosts_redirect` | Handler | Wizard done, >1 host → `View::HostSelection` |

#### S1-P2-003 — CLI Tests

| Test | Type | What to verify |
|------|------|---------------|
| `test_cli_host_select_no_arg` | Parse | `iron host select` → `id: None` |
| `test_cli_host_select_with_arg` | Parse | `iron host select desktop` → `id: Some("desktop")` |
| `test_select_interactive_single_host` | Integration | 1 host → auto-selects |
| `test_select_interactive_no_hosts` | Integration | 0 hosts → warning message |
| `test_select_existing_host` | Integration | With arg → sets current_host |

### 7.3 Test Pattern Reference

**Handler tests** (from `crates/iron-tui/src/app/handlers.rs`):

```rust
#[test]
fn test_host_selection_enter_sets_host() {
    let mut app = App::default();
    app.view = View::HostSelection;
    app.discovered_hosts = vec![
        Host {
            id: "desktop".to_string(),
            name: "Desktop".to_string(),
            ..default_test_host()
        },
        Host {
            id: "laptop".to_string(),
            name: "Laptop".to_string(),
            ..default_test_host()
        },
    ];
    app.selected_index = 1; // Select "laptop"
    // Note: state_manager needed for set_current_host
    // Test may need tempdir + StateManager::new()

    app.handle_key(create_key_event(KeyCode::Enter));

    assert_eq!(app.view, View::Dashboard);
    assert_eq!(app.current_host, Some("laptop".to_string()));
}
```

**Render tests** (from `crates/iron-tui/src/ui/tests.rs`):

```rust
#[test]
fn test_host_selection_renders_hosts() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::default();
    app.view = View::HostSelection;
    app.discovered_hosts = vec![
        Host {
            id: "desktop".to_string(),
            name: "Desktop Workstation".to_string(),
            hardware: HardwareSpec {
                cpu: Some("AMD Ryzen 7".to_string()),
                ..Default::default()
            },
            ..default_test_host()
        },
    ];

    terminal.draw(|frame| render(frame, &app)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(content.contains("desktop"));
    assert!(content.contains("Desktop Workstation"));
}
```

---

## Appendix A — File Reference

| File | Lines | Phase 2 Relevance |
|------|-------|-------------------|
| `crates/iron-core/src/host.rs` | 340 | Host struct, HardwareSpec, Host::load/save |
| `crates/iron-core/src/services/host.rs` | 604 | HostService trait, DefaultHostService, list_hosts() |
| `crates/iron-core/src/services/state.rs` | 1947 | StateManager, set_current_host(), current_host() |
| `crates/iron-core/src/state.rs` | 1485 | IronState, per-host active_bundles/profiles |
| `crates/iron-tui/src/app/mod.rs` | 809 | View enum (23 variants), App struct, navigate() |
| `crates/iron-tui/src/app/actions.rs` | 1511 | init(), init_wizard(), load_hosts() (new) |
| `crates/iron-tui/src/app/handlers.rs` | 1590 | handle_key(), handle_wizard_key(), cycle_view |
| `crates/iron-tui/src/ui/mod.rs` | 114 | render() dispatch, module declarations |
| `crates/iron-tui/src/ui/bundles.rs` | 144 | Template for list-with-detail view |
| `crates/iron-tui/src/ui/wizard.rs` | 400 | render_wizard_host_setup() reference |
| `crates/iron-tui/src/wizard.rs` | 836 | WizardState, detect_host(), apply() |
| `crates/iron-cli/src/cli.rs` | 840 | HostAction enum, Clap definitions |
| `crates/iron-cli/src/commands/host.rs` | 311 | select(), list(), host command implementations |
| `crates/iron-cli/src/context.rs` | 120 | AppContext, host_service(), current_host() |
| `hosts/desktop.toml` | 22 | Flat host config (visible to list_hosts) |
| `hosts/desktop/host.toml` | 52 | Directory host config (invisible to list_hosts) |

## Appendix B — New Tasks Discovered

| ID | Priority | Title | Origin |
|----|---------|-------|--------|
| S1-P2-004 | P1 | Wizard apply() should create host TOML file | Section 5.1 |
| S1-P2-005 | P2 | Unify host config convention (flat vs directory) | Section 5.2 |
| S1-P2-006 | P3 | Handle stale host reference in state.json | Section 5.3 |

## Appendix C — Implementation Order

```
S1-P2-004 (P1) ←── Must be done FIRST or HostSelection will show empty list
    │               after wizard. Wizard must create hosts/{id}.toml.
    │
    ▼
S1-P2-001 (P3) ←── Create the HostSelection view.
    │               Standalone, can be tested in isolation.
    │
    ├──▶ S1-P2-002 (P3) ← Wire into first-launch flow.
    │                      Depends on the view existing.
    │
    └──▶ S1-P2-003 (P3) ← CLI enhancement.
                           Independent of TUI, can be done in parallel.
```

**Critical path**: S1-P2-004 → S1-P2-001 → S1-P2-002
**Parallel stream**: S1-P2-003 (independent)
**Total estimated effort**: 8–10 hours (including discovered task S1-P2-004)
