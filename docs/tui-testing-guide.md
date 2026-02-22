# TUI Manual Testing Guide

> **Scope**: Exhaustive empirical testing of every TUI workflow from Scenario 1 + Hardening.
> **Environment**: Local terminal via `cargo run -p iron-cli -- -r /home/laraj/Documents/iron go`
> **Date**: 2026-02-20 | **Coverage**: 25 views, 10 workflows, 65 implemented features

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Test Environment Setup](#2-test-environment-setup)
3. [Legend & Conventions](#3-legend--conventions)
4. [Global Navigation Tests](#4-global-navigation-tests)
5. [Workflow 1 — First Launch & Setup Wizard](#5-workflow-1--first-launch--setup-wizard)
6. [Workflow 2 — Dashboard Orientation](#6-workflow-2--dashboard-orientation)
7. [Workflow 3 — Bundle Management](#7-workflow-3--bundle-management)
8. [Workflow 4 — Profile & Module Management](#8-workflow-4--profile--module-management)
9. [Workflow 5 — Profile Builder & Module Creator Wizards](#9-workflow-5--profile-builder--module-creator-wizards)
10. [Workflow 6 — System Update](#10-workflow-6--system-update)
11. [Workflow 7 — System Cleanup](#11-workflow-7--system-cleanup)
12. [Workflow 8 — Git Sync](#12-workflow-8--git-sync)
13. [Workflow 9 — Secrets Management](#13-workflow-9--secrets-management)
14. [Workflow 10 — Recovery & Backup](#14-workflow-10--recovery--backup)
15. [Auxiliary View Tests](#15-auxiliary-view-tests)
16. [Overlay & Dialog Tests](#16-overlay--dialog-tests)
17. [Edge Cases & Error Handling](#17-edge-cases--error-handling)
18. [Test Results Template](#18-test-results-template)

---

## 1. Prerequisites

### System Dependencies

| Dependency | Purpose | Check Command |
|-----------|---------|---------------|
| Rust toolchain | Build the project | `rustc --version` |
| `git` | Sync operations, state tracking | `git --version` |
| `pacman` | Package queries (update, cleanup) | `pacman --version` |
| `paccache` | Package cache cleanup | `which paccache` |
| `git-crypt` | Secrets encryption | `git-crypt --version` |
| `gpg` | GPG key management for secrets | `gpg --version` |
| `systemctl` | Service management | `systemctl --version` |
| Terminal | Min 80×24, recommended 120×40 | Resize before launch |

### Build

```bash
cd /home/laraj/Documents/iron
cargo build -p iron-cli
```

### Verify Test Data Exists

The workspace ships with sample data used for all tests:

| Item | Path | Content |
|------|------|---------|
| Bundles | `bundles/hyprland/`, `bundles/niri/` | 2 Wayland compositor bundles |
| Modules | `modules/` (9 dirs) | nvim-ide, kitty-dev, git-config, tmux-config, starship-prompt, waybar-dev, dev-tools, fail2ban, ufw |
| Profiles | `profiles/developer/`, `profiles/minimal/` | 2 predefined profiles |
| Host | `hosts/desktop.toml` | Desktop workstation config |
| State | `state.json` | Current active state (host=desktop, 2 active modules) |

**Quick verification:**

```bash
ls bundles/*/bundle.toml   # Should show 2 files
ls modules/*/module.toml   # Should show 9 files
ls profiles/*/profile.toml # Should show 2 files
cat state.json             # Should show JSON with current_host, active_modules, etc.
```

---

## 2. Test Environment Setup

### Launch Command

```bash
# Run against the workspace directory (recommended for testing)
cargo run -p iron-cli -- -r . go
```

This reads `state.json`, `bundles/`, `modules/`, `profiles/`, and `hosts/` from the workspace root.

### State Reset (Between Tests)

To reset to a clean state for re-testing first-launch scenarios:

```bash
# Backup current state
cp state.json state.json.bak

# Reset to empty state (triggers Setup Wizard)
echo '{}' > state.json

# Restore after testing
cp state.json.bak state.json
```

### State Reset for Host Selection Test

```bash
# Create a multi-host scenario to trigger HostSelection view
cp hosts/desktop.toml hosts/laptop.toml
sed -i 's/desktop/laptop/g; s/Desktop Workstation/Laptop/g' hosts/laptop.toml

# Clear current_host to trigger selection
jq '.current_host = null' state.json > tmp.json && mv tmp.json state.json

# Clean up after test
rm hosts/laptop.toml
```

---

## 3. Legend & Conventions

### Key Notation

| Notation | Meaning |
|----------|---------|
| `↑` `↓` `←` `→` | Arrow keys |
| `j` `k` | Vim-style up/down (equivalent to arrows in most views) |
| `Enter` | Confirm / select / advance to next step |
| `Esc` | Go back / cancel |
| `Tab` | Cycle forward through views or form fields |
| `Shift+Tab` | Cycle backward through views |
| `Space` | Toggle checkbox / selection |
| `Ctrl+C` | Force quit |

### Visual Indicators

| Indicator | Meaning |
|-----------|---------|
| `[OK]` green | Healthy state |
| `[!!]` yellow | Warning — needs attention |
| `[XX]` red | Error — action required |
| `✓` green footer | Success message (auto-expires 3s) |
| `ℹ` blue footer | Info message (auto-expires 3s) |
| `⚠` yellow footer | Warning message (auto-expires 5s) |
| `✗` red footer | Error message (auto-expires 5s) |
| `█░` progress bar | Module activation progress |

### Test Result Markers

| Marker | Meaning |
|--------|---------|
| ✅ PASS | Behaves as described |
| ❌ FAIL | Unexpected behavior or crash |
| ⚠️ PARTIAL | Works but with cosmetic / non-critical issues |
| ⏭️ SKIP | Cannot test (missing dependency, requires sudo, etc.) |

---

## 4. Global Navigation Tests

> **Goal**: Verify all cross-view navigation, the tab cycle, and global keybindings.
> **Starting view**: Dashboard (default after init)

### T-NAV-001: Tab Cycle (Forward)

| # | Action | Expected View |
|---|--------|---------------|
| 1 | Launch TUI | Dashboard |
| 2 | Press `Tab` | Bundles |
| 3 | Press `Tab` | Profiles |
| 4 | Press `Tab` | Modules |
| 5 | Press `Tab` | SystemMaintenance |
| 6 | Press `Tab` | UpdatePreview |
| 7 | Press `Tab` | Sync |
| 8 | Press `Tab` | Settings |
| 9 | Press `Tab` | Dashboard (wraps around) |

**Verify**: Header shows correct view name and icon at each step.

### T-NAV-002: Tab Cycle (Backward)

| # | Action | Expected View |
|---|--------|---------------|
| 1 | From Dashboard, press `Shift+Tab` | Settings |
| 2 | Press `Shift+Tab` | Sync |
| 3 | Continue pressing `Shift+Tab` | Reverse of T-NAV-001 order |

### T-NAV-003: Direct Navigation Keys

| # | Key | Expected View | Verify |
|---|-----|---------------|--------|
| 1 | `d` | Dashboard | Header shows `[=] Dashboard` |
| 2 | `b` | Bundles | Header shows `[B] Bundles` |
| 3 | `p` | Profiles | Header shows `[P] Profiles` |
| 4 | `m` | Modules | Header shows `[M] Modules` |
| 5 | `x` | SystemMaintenance | Header shows `Maintenance` |
| 6 | `u` | UpdatePreview | Header shows `[U] System Update` |
| 7 | `l` | CleanSystem | Header shows `System Cleanup` |
| 8 | `y` | Sync | Header shows `Git Sync` |
| 9 | `s` | Settings | Header shows `Settings` |
| 10 | `w` | SetupWizard | Header shows `Setup Wizard` |
| 11 | `S` (Shift+s) | Secrets | Header shows `Secrets` |
| 12 | `R` (Shift+r) | Recovery | Header shows `Recovery` |
| 13 | `H` (Shift+h) | HostSelection | Header shows `Host Selection` |

### T-NAV-004: Esc (Back Navigation)

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate `d` → `b` → `Enter` (on a bundle) | BundleDetail view |
| 2 | Press `Esc` | Returns to Bundles |
| 3 | Press `Esc` | Returns to Dashboard |

### T-NAV-005: Help Overlay

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `?` | Centered popup appears: "Help: Dashboard" title |
| 2 | Verify contents | Lists view-specific actions, Navigation section (Tab/Shift+Tab/Esc), Global section (q/?/Ctrl+C) |
| 3 | Verify concept map (Dashboard-only) | Shows HOST → BUNDLE → PROFILE → MODULE tree |
| 4 | Press any key | Help overlay closes |
| 5 | Navigate to Bundles (`b`), press `?` | Shows "Help: Bundles" with bundle-specific keys |

### T-NAV-006: Quit Flow

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `q` | Confirm dialog: "Quit Iron?" |
| 2 | Press `n` or `Esc` | Dialog closes, TUI continues |
| 3 | Press `q` again | Confirm dialog appears |
| 4 | Press `y` or `Enter` | TUI exits cleanly |
| 5 | Re-launch, press `Ctrl+C` | TUI exits immediately (no confirm) |

---

## 5. Workflow 1 — First Launch & Setup Wizard

> **Goal**: Verify the 5-step first-run experience from an empty state.
> **Setup**: Reset state to trigger wizard (see section 2).

```bash
echo '{}' > state.json
cargo run -p iron-cli -- -r . go
```

### T-WIZ-001: Auto-Route to Wizard

| # | Observation | Expected |
|---|-------------|----------|
| 1 | TUI launches | Lands on SetupWizard view (Step 1: Welcome) |
| 2 | Header | Shows `Setup Wizard` |
| 3 | Footer | Shows `[j/k] Select  [Enter] Confirm  [h/l] Navigate` |

### T-WIZ-002: Step 1 — Welcome

| # | Action | Expected |
|---|--------|----------|
| 1 | Read welcome text | Shows Iron introduction and purpose |
| 2 | Press `Enter` | Advances to Step 2: Host Setup |
| 3 | Press `q` or `Esc` | Quits wizard (returns to quit confirmation) |

### T-WIZ-003: Step 2 — Host Setup

| # | Action | Expected |
|---|--------|----------|
| 1 | Observe hostname | Auto-detected from system hostname |
| 2 | Press `e` | Enters edit mode for hostname |
| 3 | Type a custom name (e.g., `test-host`) | Text appears in hostname field, only `[a-z0-9-]` characters accepted |
| 4 | Press `Home` / `End` | Cursor jumps to start / end of text |
| 5 | Press `←` `→` | Cursor moves within text |
| 6 | Press `Backspace` / `Delete` | Deletes char before / after cursor |
| 7 | Press `Enter` | Advances to Step 3: Bundle Selection |
| 8 | Press `Backspace` from step 3 | Returns to Step 2 with hostname preserved |

### T-WIZ-004: Step 3 — Bundle Selection

| # | Action | Expected |
|---|--------|----------|
| 1 | Observe list | Shows 2 bundles: `Hyprland Desktop`, `Niri Desktop` |
| 2 | Each bundle shows | Description, package count, conflict info |
| 3 | Press `j` / `↓` | Highlight moves to next bundle |
| 4 | Press `k` / `↑` | Highlight moves to previous bundle |
| 5 | Press `Enter` | Selects highlighted bundle, advances to Step 4 |
| 6 | Press `Backspace` | Returns to Step 2 |

### T-WIZ-005: Step 4 — Profile Selection

| # | Action | Expected |
|---|--------|----------|
| 1 | Observe list | Shows profiles compatible with selected bundle |
| 2 | Press `j` / `k` | Navigate profiles |
| 3 | Press `Enter` | Selects profile, advances to Step 5: Confirmation |
| 4 | Press `Backspace` | Returns to Step 3 |

### T-WIZ-006: Step 5 — Confirmation & Apply

| # | Action | Expected |
|---|--------|----------|
| 1 | Observe summary | Shows selected host, bundle, and profile |
| 2 | Press `Enter` or `y` | Wizard applies configuration |
| 3 | Observe progress | Status messages show: setting host, activating bundle, applying profile |
| 4 | After completion | Navigates to SystemScan view |
| 5 | Verify `state.json` | Contains `current_host`, `active_bundles`, `active_profiles` entries |

### T-WIZ-007: Post-Wizard System Scan

| # | Action | Expected |
|---|--------|----------|
| 1 | On SystemScan view | Shows scan results (packages, services, symlinks) |
| 2 | Press `r` | Re-runs the system scan |
| 3 | Press `Enter` | Navigates to Dashboard |
| 4 | Press `Esc` | Also navigates back to Dashboard |

**After testing, restore state:**
```bash
cp state.json.bak state.json
```

---

## 6. Workflow 2 — Dashboard Orientation

> **Goal**: Verify the home screen displays all sections correctly and responds to interaction.
> **Starting view**: Dashboard (press `d` from anywhere)

### T-DASH-001: Layout Verification

| # | Section | Location | Expected Content |
|---|---------|----------|-----------------|
| 1 | Header | Top bar | `IRON` badge + `[=] Dashboard` + `Host: desktop | Bundle: <name>` |
| 2 | System Status | Top-left | Health icon (`[OK]`/`[!!]`/`[XX]`), package count, update count |
| 3 | Maintenance | Mid-left | Last update/cleanup timestamps with age-based color coding |
| 4 | Quick Actions | Bottom-left | 3×3 grid: `[b] Bundles [p] Profiles [m] Modules [u] Update [x] Maintain [l] Cleanup [y] Sync [s] Settings [?] Help` |
| 5 | Active Config | Top-right | Active bundle, profile, module progress bar (`█░`), drift status |
| 6 | Notifications | Bottom-right | Pending updates, Arch news, diverged modules, or "All clear" |
| 7 | Footer | Bottom bar | `[q] Quit  [?] Help  [Tab] Navigate` + active config status line |

### T-DASH-002: Maintenance Timestamps Color Coding

| Condition | Expected Color |
|-----------|---------------|
| Last update ≤ 1 day ago | Green |
| Last update ≤ 7 days ago | Yellow |
| Last update > 7 days ago | Red |
| Last cleanup ≤ 7 days ago | Green |
| Last cleanup ≤ 30 days ago | Yellow |
| Last cleanup > 30 days ago | Red |
| No data (null) | Gray / "Never" text |

### T-DASH-003: Module Progress Bar

| # | Observation | Expected |
|---|-------------|----------|
| 1 | Read module progress | Shows `N/M modules active` with filled/empty bar proportional to ratio |
| 2 | Verify against state.json | Count matches `active_modules` length vs total modules discovered |

### T-DASH-004: Divergence Popup

| # | Action | Expected |
|---|--------|----------|
| 1 | Check notifications area | If diverged modules exist: shows count and `[i] Divergence` in footer |
| 2 | Press `i` | Popup appears listing each diverged module |
| 3 | Press `j` / `k` | Navigate diverged module list |
| 4 | Press `r` | Shows guidance: "Restore from remote with `iron sync pull`" |
| 5 | Press `a` | Shows guidance: "Accept local changes with `iron sync push`" |
| 6 | Press `d` | Shows guidance: "View diff with `git diff`" |
| 7 | Press `Esc` | Closes popup |
| 8 | If no divergence | `[i]` key hint absent from footer; `i` press is no-op |

### T-DASH-005: Quick Actions Navigation

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `b` (from Dashboard) | Navigates to Bundles view |
| 2 | Press `Esc` to return | Back to Dashboard |
| 3 | Press each quick action key | Navigates to the correct view |

---

## 7. Workflow 3 — Bundle Management

> **Goal**: Verify bundle listing, detail view, activation, deactivation, and switching.
> **Starting view**: Bundles (press `b`)

### T-BUN-001: Bundle List View

| # | Observation | Expected |
|---|-------------|----------|
| 1 | View loads | Shows list with 2 bundles: `Hyprland Desktop`, `Niri Desktop` |
| 2 | Each item shows | Name, description, state indicator (Active / Inactive / Dormant) |
| 3 | Active bundle | Highlighted or marked differently (green / bold) |
| 4 | Footer | `[j/k] Select  [Enter] Details  [e] Toggle  [Esc] Back  [?] Help` |

### T-BUN-002: Bundle List Navigation

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `j` / `↓` | Selection moves down |
| 2 | Press `k` / `↑` | Selection moves up |
| 3 | Press `Home` | Jumps to first bundle |
| 4 | Press `End` | Jumps to last bundle |
| 5 | Press `Enter` on a bundle | Opens BundleDetail view |

### T-BUN-003: Bundle Detail View

| # | Observation | Expected |
|---|-------------|----------|
| 1 | Header changes | Shows `Bundle Details` |
| 2 | Detail content | Bundle name, description, type (WaylandCompositor), state |
| 3 | Packages section | Lists all packages from `bundle.toml` (e.g., hyprland, waybar, wofi...) |
| 4 | Services section | Lists services (e.g., pipewire, pipewire-pulse, wireplumber) |
| 5 | Conflicts section | Lists conflicting bundles (e.g., niri, sway, kde) |
| 6 | Press `Esc` | Returns to Bundles list |

### T-BUN-004: Bundle Activation

| # | Action | Expected |
|---|--------|----------|
| 1 | In Bundles list, select an **inactive** bundle | Highlight on inactive bundle |
| 2 | Press `a` | Confirm dialog appears: "Switch to bundle '…'?" |
| 3 | Press `n` / `Esc` | Dialog closes, no change |
| 4 | Press `a` again, then `y` / `Enter` | Bundle activation runs |
| 5 | Observe footer | Success message: bundle activated ✓ |
| 6 | Verify state.json | `active_bundles` now contains the selected bundle |

> **⚠️ Note**: Activation calls `pacman` to install packages. If packages are already installed, it completes quickly. If not, it requires sudo privileges.

### T-BUN-005: Bundle Deactivation

| # | Action | Expected |
|---|--------|----------|
| 1 | Open the active bundle's detail view (`Enter`) | BundleDetail shows active bundle |
| 2 | Press `d` | Confirm dialog: "Remove bundle '…'?" |
| 3 | Press `y` / `Enter` | Bundle deactivated |
| 4 | Verify state.json | `active_bundles` is empty or updated |

### T-BUN-006: Bundle Conflict Warning

| # | Action | Expected |
|---|--------|----------|
| 1 | Activate Hyprland bundle | Hyprland is now active |
| 2 | Attempt to activate Niri bundle | Confirm dialog should mention conflict |
| 3 | Confirm activation | Previous bundle deactivated, new bundle activated |

---

## 8. Workflow 4 — Profile & Module Management

> **Goal**: Verify profile/module listing, detail views, activation, and enable/disable.

### T-PROF-001: Profile List View

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `p` (navigate to Profiles) | Shows 2 profiles: `Developer`, `Minimal` |
| 2 | Each profile shows | Name, module count, active/inactive state indicator |
| 3 | Navigate with `j`/`k` | Selection moves between profiles |
| 4 | Press `Enter` | Opens ProfileDetail view |

### T-PROF-002: Profile Detail & Activation

| # | Action | Expected |
|---|--------|----------|
| 1 | In ProfileDetail | Shows profile name, theme, shell, and module list |
| 2 | Press `Enter` or `a` | Activates the profile |
| 3 | Observe result | Success/error message in footer |
| 4 | Return to Profiles list | Active indicator updated |

### T-MOD-001: Module List View

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `m` (navigate to Modules) | Shows 9 modules |
| 2 | Each module shows | Name, kind, enabled/disabled status |
| 3 | Navigate with `j`/`k` | Selection moves between modules |
| 4 | Press `Home` / `End` | Jumps to first / last module |

### T-MOD-002: Module Detail View

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `Enter` on a module | ModuleDetail opens |
| 2 | Observe content | Module name, description, kind, packages, AUR packages, dotfiles, conflicts, dependencies |
| 3 | Conflict info | Shows which modules conflict with this one |
| 4 | Press `Esc` | Returns to Modules list |

### T-MOD-003: Module Enable/Disable Toggle

| # | Action | Expected |
|---|--------|----------|
| 1 | Select a **disabled** module in the list | Highlighted |
| 2 | Press `e` | Confirm dialog: "Enable module '…'?" |
| 3 | Press `y` | Module enabled, state.json `active_modules` updated |
| 4 | Footer shows | Success message ✓ |
| 5 | Select the now **enabled** module | Highlighted |
| 6 | Press `e` | Confirm dialog: "Disable module '…'?" |
| 7 | Press `y` | Module disabled, state.json updated |

### T-MOD-004: Module Toggle from Detail View

| # | Action | Expected |
|---|--------|----------|
| 1 | Open a module's detail (`Enter`) | ModuleDetail view |
| 2 | Press `e` | Same confirm dialog as T-MOD-003 |
| 3 | Confirm | Toggles enable/disable state |

---

## 9. Workflow 5 — Profile Builder & Module Creator Wizards

> **Goal**: Verify the 3-step creation wizards for profiles and modules.

### T-PB-001: Launch Profile Builder

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Profiles (`p`) | Profiles list |
| 2 | Press `n` | ProfileBuilder wizard opens at Step 1 (Name/Description) |
| 3 | Header | Shows `New Profile` |

### T-PB-002: Step 1 — Name & Description

| # | Action | Expected |
|---|--------|----------|
| 1 | Type profile name | Text appears in name field; only `[a-z0-9-]` chars accepted |
| 2 | Try uppercase letters | Silently rejected (not inserted) |
| 3 | Try special characters (`!`, `@`, `_`) | Silently rejected |
| 4 | Press `Tab` | Focus moves to description field |
| 5 | Type description | Free-text accepted |
| 6 | Press `Tab` again | Focus moves back to name field |
| 7 | Press `Enter` with empty name | No-op (name is required) |
| 8 | Press `Enter` with valid name | Advances to Step 2 (Module Selection) |
| 9 | Press `Esc` | Cancels wizard, returns to Profiles list |

### T-PB-003: Step 2 — Module Selection

| # | Action | Expected |
|---|--------|----------|
| 1 | Module list appears | Shows all 9 available modules as checkboxes |
| 2 | Press `j`/`k` | Navigate modules |
| 3 | Press `Space` on a module | Toggles selection (checkbox fills/empties) |
| 4 | Select conflicting modules | Warning message appears: "'X' conflicts with 'Y'" |
| 5 | Select module with dependencies | Tip message: "'X' depends on: Y, Z" |
| 6 | Press `Enter` | Advances to Step 3 (Preview) |
| 7 | Press `Esc` | Returns to Step 1 (name/description preserved) |

### T-PB-004: Step 3 — Preview & Create

| # | Action | Expected |
|---|--------|----------|
| 1 | Preview shows | Profile name, description, selected module list |
| 2 | Press `Enter` | Creates profile directory and `profile.toml` |
| 3 | Observe footer | Success message: profile created ✓ |
| 4 | Auto-navigation | Returns to Profiles list |
| 5 | New profile appears | In the profiles list with correct name |
| 6 | Verify on disk | `profiles/<name>/profile.toml` exists with correct content |
| 7 | Press `Esc` from preview | Returns to Step 2 |

### T-PB-005: Duplicate Profile Name

| # | Action | Expected |
|---|--------|----------|
| 1 | Create a profile with name `developer` (already exists) | Error message in footer: "already exists" |
| 2 | Profile not overwritten | Original `profiles/developer/profile.toml` unchanged |

### T-MC-001: Launch Module Creator

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Modules (`m`) | Modules list |
| 2 | Press `n` | ModuleCreator wizard opens at Step 1 (Metadata) |
| 3 | Header | Shows `New Module` |

### T-MC-002: Step 1 — Module Metadata

| # | Action | Expected |
|---|--------|----------|
| 1 | Focus on ID field | Cursor active in module ID field |
| 2 | Type module ID | Only `[a-z0-9-]` chars accepted |
| 3 | Press `Tab` | Focus cycles through: ID → Description → Packages → Kind |
| 4 | Type description | Free-text accepted |
| 5 | Type packages | Comma-separated values (e.g., `vim,tmux,fish`) |
| 6 | Focus on Kind field | Shows current kind (e.g., `AppConfig`) |
| 7 | Press `←` / `→` or `h` / `l` on Kind | Cycles through 6 variants: AppConfig, Shell, DesktopComponent, Theme, SystemUtil, DevTools |
| 8 | Press `Enter` with empty ID | No-op (ID required) |
| 9 | Press `Enter` with valid ID | Advances to Step 2 (Dotfiles) |
| 10 | Press `Esc` | Cancels wizard, returns to Modules list |

### T-MC-003: Step 2 — Dotfile Mappings

| # | Action | Expected |
|---|--------|----------|
| 1 | Step 2 loads | Shows dotfile source/target input fields |
| 2 | Type source path | e.g., `config/myapp` |
| 3 | Press `Tab` | Focus moves to target field |
| 4 | Type target path | e.g., `~/.config/myapp` |
| 5 | Press `Enter` | Adds the mapping, clears input fields for next pair |
| 6 | Previous mapping visible | Listed above input fields |
| 7 | Add multiple mappings | All listed correctly |
| 8 | Press `Backspace` (with empty fields) | Removes last mapping |
| 9 | Press `Enter` to proceed | Advances to Step 3 (Preview) |
| 10 | Press `Esc` | Returns to Step 1 |

### T-MC-004: Step 3 — Preview & Create

| # | Action | Expected |
|---|--------|----------|
| 1 | Preview shows | Module ID, description, packages, kind, and dotfile mappings |
| 2 | Dotfile section | Shows each source → target mapping |
| 3 | Press `Enter` | Creates module directory and `module.toml` |
| 4 | Observe footer | Success message ✓ |
| 5 | Auto-navigation | Returns to Modules list |
| 6 | New module appears in list | With correct name and kind |
| 7 | Verify `module.toml` on disk | Contains `[[dotfiles]]` blocks with source/target/link fields |
| 8 | Press `Esc` from preview | Returns to Step 2 |

### T-MC-005: Duplicate Module Name

| # | Action | Expected |
|---|--------|----------|
| 1 | Create module with ID `nvim-ide` (already exists) | Error message: "already exists" |

**Cleanup after wizard tests:**
```bash
# Remove test-created profiles/modules
rm -rf profiles/<test-name>/ modules/<test-name>/
```

---

## 10. Workflow 6 — System Update

> **Goal**: Verify the update preview, pre-flight checks, Arch news, risk assessment, and update execution.
> **Starting view**: UpdatePreview (press `u`)

### T-UPD-001: Update Preview Layout

| # | Section | Expected Content |
|---|---------|-----------------|
| 1 | Header | `[U] System Update` |
| 2 | Three sections | Pre-flight Checks, Arch News, Available Packages |
| 3 | Section navigation | `→`/`l` and `←`/`h` cycle between the three sections |
| 4 | Footer | `[u] Update  [h/l] Sections  [r] Refresh  [Esc] Back` |

### T-UPD-002: Pre-flight Checks

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Pre-flight section (first section) | Shows checklist items |
| 2 | Each check shows | ✓ pass or ✗ fail with description |
| 3 | Check items include | Disk space, snapshot age, partial update detection, mirror freshness |
| 4 | If any check fails | `[u] Update` may be blocked (critical blockers prevent update) |

### T-UPD-003: Arch News Section

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to News section (`→`) | Shows recent Arch Linux news items |
| 2 | Navigate items with `j`/`k` | Scrolls through news |
| 3 | Press `a` | Acknowledges selected news item (marking it read) |
| 4 | Press `A` (Shift+a) | Acknowledges all news items |
| 5 | Items requiring manual intervention | Highlighted differently |

### T-UPD-004: Package List Section

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Packages section (`→`) | Shows available package updates |
| 2 | Each package shows | Name, current version → new version, repository |
| 3 | Scroll with `j`/`k` | Navigates package list |

### T-UPD-005: Refresh Updates

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `r` | Refreshes all three sections |
| 2 | Pre-flight checks re-run | Results update in real-time |
| 3 | News re-fetched | May show new items |
| 4 | Packages re-checked | List updates |
| 5 | Footer momentarily | Shows info message during refresh |

### T-UPD-006: Run System Update — Risk-Based Confirmation

**Test varies by risk level detected:**

| Risk Level | Confirmation Style | Visual |
|-----------|-------------------|--------|
| Low / Medium | Simple confirm | Standard Yes/No popup |
| High | Enhanced Warning | Yellow border, `! HIGH RISK !` banner |
| Critical | Typed Confirmation | Red border, `!! CRITICAL !!`, must type `CONFIRM` |

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `u` with pre-flight blockers | Error message: cannot update, blockers exist |
| 2 | Press `u` with all checks passing | Confirm dialog appears (style depends on risk) |
| 3 | Cancel with `n`/`Esc` | Dialog closes, no update |
| 4 | Confirm with `y`/`Enter` (or type `CONFIRM` for critical) | Update executes |
| 5 | During update | Progress indicator shown |
| 6 | After update | Post-update checks run (.pacnew detection, reboot check, failed services) |

> **⚠️ Note**: Running the actual update (`pacman -Syu`) modifies the system. Only proceed if you intend to update.

---

## 11. Workflow 7 — System Cleanup

> **Goal**: Verify category selection, preview, and execution of cleanup operations.
> **Starting view**: CleanSystem (press `l`)

### T-CLN-001: Category Selection View

| # | Observation | Expected |
|---|-------------|----------|
| 1 | View loads | Shows cleanup categories as checkboxes |
| 2 | Categories include | Package Cache, Orphan Packages, Old Logs, Broken Symlinks, etc. (9 total) |
| 3 | Some categories tagged | `[safe]` or `[aggressive]` labels |
| 4 | Footer | `[Space] Toggle  [s] Safe  [a] All  [Enter] Preview  [c] Clean  [Esc] Back` |

### T-CLN-002: Category Navigation & Selection

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `j`/`k` | Navigate between categories |
| 2 | Press `Space` on a category | Toggles checkbox ☑/☐ |
| 3 | Press `s` | Selects only safe categories (pre-checked) |
| 4 | Press `a` | Selects all categories including aggressive |
| 5 | Press `n` | Deselects all categories |

### T-CLN-003: Cleanup Preview

| # | Action | Expected |
|---|--------|----------|
| 1 | Select some categories and press `Enter` | Navigates to CleanupPreview |
| 2 | Preview shows | Scan results per category: files found, space reclaimable |
| 3 | Each category section | Lists specific items (e.g., orphan package names, cache sizes) |
| 4 | Press `Esc` | Returns to CleanSystem (selections preserved) |

### T-CLN-004: Cleanup Execution — Safe Categories

| # | Action | Expected |
|---|--------|----------|
| 1 | Select safe categories only (`s`) | Safe categories checked |
| 2 | Press `c` | Simple confirm dialog: "Run system cleanup? (dry-run mode)" |
| 3 | Press `y` | Cleanup executes |
| 4 | Navigates to CleanupResults | Shows summary: items cleaned per category, total space freed |
| 5 | Press `Esc` from results | Returns to CleanSystem |

### T-CLN-005: Cleanup Execution — Aggressive Categories

| # | Action | Expected |
|---|--------|----------|
| 1 | Select all categories (`a`) | All checked including aggressive |
| 2 | Press `c` | **Enhanced Warning** dialog: yellow border, `! HIGH RISK !` banner |
| 3 | Press `n` | Cancels |
| 4 | Press `c` again, then `y` | Cleanup runs |

### T-CLN-006: Execute from Preview

| # | Action | Expected |
|---|--------|----------|
| 1 | From CleanupPreview | Press `c` |
| 2 | Confirm dialog | Same style as from category view |
| 3 | Confirm | Executes and shows results |

> **⚠️ Note**: Cleanup operations involving `paccache` and orphan removal require sudo privileges.

---

## 12. Workflow 8 — Git Sync

> **Goal**: Verify sync status display, background push/pull, and conflict resolution.
> **Starting view**: Sync (press `y`)
> **Prerequisite**: The workspace must be a git repository with a configured remote.

### T-SYNC-001: Sync View Auto-Refresh

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Sync view (`y`) | Auto-refreshes sync status on entry |
| 2 | Status shows | Branch name, clean/dirty state, ahead/behind counts, last sync time |
| 3 | Footer | `[p] Push  [f] Pull  [s] Status  [Esc] Back` |

### T-SYNC-002: Manual Status Refresh

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `s` | Refreshes git status |
| 2 | If dirty (uncommitted changes) | Shows "dirty" indicator |
| 3 | If clean | Shows "clean" indicator |
| 4 | If ahead of remote | Shows ahead count |
| 5 | If behind remote | Shows behind count |

### T-SYNC-003: Push (Background)

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `p` | Confirm dialog: "Push local changes to remote?" |
| 2 | Press `y` | Push starts in background |
| 3 | Footer shows | Info message: "Pushing changes..." |
| 4 | TUI remains responsive | Can navigate to other views during push |
| 5 | On completion | Success or error message appears in footer |
| 6 | Auto-commit | If dirty files exist, auto-commits before push |

### T-SYNC-004: Pull (Background)

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `f` | Confirm dialog: "Pull remote changes to local?" |
| 2 | Press `y` | Pull starts in background |
| 3 | Footer shows | Info message: "Pulling changes..." |
| 4 | If dirty files exist | Auto-stash before pull, auto-pop after |
| 5 | On completion | Success or error message in footer |
| 6 | Post-pull | Config re-link runs automatically |

### T-SYNC-005: Background Operation Non-Blocking

| # | Action | Expected |
|---|--------|----------|
| 1 | Start a push (`p` → `y`) | Background operation starts |
| 2 | Press `Tab` | Can navigate to other views |
| 3 | Return to Sync | Status shows operation in progress or completed result |
| 4 | Start another push while one is running | Should be prevented (button disabled or error message) |

### T-SYNC-006: Conflict Resolution

> **Prerequisite**: Create a merge conflict scenario (e.g., edit a file on remote that conflicts with local).

| # | Action | Expected |
|---|--------|----------|
| 1 | Pull results in conflicts | Sync view shows conflict list |
| 2 | Press `l` | Resolves all conflicts: keep local version (`git checkout --ours`) |
| 3 | **OR** Press `r` | Resolves all conflicts: keep remote version (`git checkout --theirs`) |
| 4 | After resolution | Conflict list clears, status updates |

### T-SYNC-007: Secrets Lock on Push

| # | Action | Expected |
|---|--------|----------|
| 1 | If git-crypt is initialized and secrets are unlocked | Push should auto-lock secrets before pushing |
| 2 | After push completes | Secrets remain locked |
| 3 | Verify | No plaintext secrets in the push |

---

## 13. Workflow 9 — Secrets Management

> **Goal**: Verify git-crypt initialization, lock/unlock, GPG key management.
> **Starting view**: Secrets (press `S`)
> **Prerequisites**: `git-crypt` and `gpg` installed. A valid GPG key pair.

### T-SEC-001: Secrets View Auto-Refresh

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Secrets (`S`) | Auto-refreshes secrets status on entry |
| 2 | Status shows | git-crypt initialization state, lock/unlock status |
| 3 | Footer | `[i] Init  [u] Unlock  [l] Lock  [Esc] Back` |

### T-SEC-002: Initialize git-crypt

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `i` | Initializes git-crypt in the repository |
| 2 | If already initialized | Error or info message: already initialized |
| 3 | If not a git repo | Error message |
| 4 | On success | Footer shows ✓ message, status updates to "initialized" |

### T-SEC-003: Lock Secrets

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `l` (when unlocked) | Locks (re-encrypts) secrets |
| 2 | On success | Status updates to "locked" ✓ |
| 3 | Verify | `secrets/` directory files are encrypted binary |

### T-SEC-004: Unlock Secrets

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `u` (when locked) | Unlocks (decrypts) secrets |
| 2 | Requires valid GPG key | May prompt for GPG passphrase in terminal |
| 3 | On success | Status updates to "unlocked" ✓ |
| 4 | Verify | `secrets/` directory files are readable plaintext |

### T-SEC-005: Add GPG Key

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `a` | Text input mode activates (field appears for GPG key ID) |
| 2 | Type a GPG key ID | Characters appear in input field |
| 3 | Press `Backspace` | Deletes character |
| 4 | Press `Esc` | Cancels input mode |
| 5 | Press `Enter` with valid key ID | Adds GPG user to git-crypt |
| 6 | On success | Footer shows ✓ message |
| 7 | With invalid key ID | Error message in footer |

### T-SEC-006: Refresh Status

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `r` | Re-checks git-crypt status |
| 2 | Status updates | Reflects current lock/unlock/init state |

---

## 14. Workflow 10 — Recovery & Backup

> **Goal**: Verify state export, import, install script generation, and snapshot creation.
> **Starting view**: Recovery (press `R`)

### T-REC-001: Recovery View Layout

| # | Observation | Expected |
|---|-------------|----------|
| 1 | View loads | Shows recovery options and last backup info |
| 2 | Last backup | Populated from audit log (or "Never" if no backups) |
| 3 | Footer | `[g] install.sh  [e] Export  [s] Snapshot  [Esc] Back` |

### T-REC-002: Export State

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `e` | Exports current state to JSON file |
| 2 | On success | Footer shows ✓ with export file path |
| 3 | Verify file | `iron-export-<timestamp>.json` created in the config directory |
| 4 | Verify content | JSON contains host, bundles, profiles, modules, packages |

### T-REC-003: Generate Install Script

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `g` | Generates a shell install script |
| 2 | On success | Footer shows ✓ with script path |
| 3 | Verify file | `install.sh` exists and is executable (`chmod +x`) |
| 4 | Verify content | Contains `pacman -S` for official packages, `yay -S` for AUR, `systemctl enable` for services |

### T-REC-004: Create Snapshot/Backup

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `s` | Creates a backup via RecoveryService |
| 2 | On success | Footer shows ✓ message |
| 3 | Verify | Backup recorded in audit log, last backup time updates |

### T-REC-005: Import State

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `i` | Text input field appears for file path |
| 2 | Type an export file path | e.g., `./iron-export-20260220-120000.json` |
| 3 | Press `Esc` | Cancels input |
| 4 | Press `Enter` with valid path | Import runs the 6-step flow |
| 5 | Import steps | (1) set host, (2) set bundle, (3) set profile, (4) enable modules, (5) install packages (best-effort), (6) enable services (best-effort) |
| 6 | On success | Footer shows ✓ with summary |
| 7 | With invalid path | Error message: file not found |
| 8 | Verify state.json | Updated with imported configuration |

### T-REC-006: Import Round-Trip

| # | Action | Expected |
|---|--------|----------|
| 1 | Export state (`e`) | Note the filename |
| 2 | Modify state (change active module) | State differs from export |
| 3 | Import the exported file (`i`) | Restores original state |
| 4 | Verify | state.json matches the original export |

---

## 15. Auxiliary View Tests

### T-MAINT-001: System Maintenance Hub

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `x` | Shows 3 cards: Update, Cleanup, Doctor |
| 2 | Press `←`/`h` / `→`/`l` | Navigate between cards (index 0-2) |
| 3 | Press `Enter` on Update card | Navigates to UpdatePreview |
| 4 | Press `Enter` on Cleanup card | Navigates to CleanSystem |
| 5 | Press `Enter` on Doctor card | Navigates to Doctor |
| 6 | Direct keys: `u`, `c`, `d` | Jump to respective views |

### T-DOC-001: Doctor (Health Checks)

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to Doctor (from Maintenance `d` or via shortcuts) | Auto-runs health checks on entry |
| 2 | Shows check results | List of checks with ✓ pass / ✗ fail status |
| 3 | Check categories | Package integrity, service status, symlink health, etc. |
| 4 | Press `r` | Re-runs all health checks |
| 5 | Press `Esc` | Returns to previous view |

### T-SET-001: Settings View

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `s` (navigate to Settings) | Shows 8 configuration items |
| 2 | Navigate with `j`/`k` | Scrolls through settings |
| 3 | Press `Enter` on a setting | Shows edit hint or detail |
| 4 | Press `r` | Refreshes settings from disk |
| 5 | Press `o` | Opens OperationLog view |
| 6 | Press `c` | Opens ConfigManager view |
| 7 | Press `w` | Opens SetupWizard (re-run) |
| 8 | Press `s` | Opens SystemScan view |

### T-LOG-001: Operation Log

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to OperationLog (from Settings `o`) | Shows audit log entries |
| 2 | Each entry shows | Timestamp, operation type, status, details |
| 3 | Press `j`/`k` | Scrolls through entries |
| 4 | Press `f` | Cycles through filters (All, Updates, Cleanup, Sync, etc.) |
| 5 | Filter changes | Only matching entries shown |

### T-CFG-001: Config Manager

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to ConfigManager (from Settings `c`) | Shows .pacnew/.pacsave files |
| 2 | Press `j`/`k` | Navigate file list |
| 3 | Press `Enter` | Shows diff guidance hint |
| 4 | Press `r` | Refreshes file scan |

### T-SCAN-001: System Scan

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to SystemScan (from Settings `s`) | Shows scan results |
| 2 | Scan sections | Packages, services, modules, symlinks, system info |
| 3 | Press `r` | Re-runs system scan (results update) |
| 4 | Press `↑`/`k` `↓`/`j` | Scrolls scan results |
| 5 | Press `Enter` | Navigates to Dashboard |

### T-SEC-MOD-001: Security Modules

| # | Action | Expected |
|---|--------|----------|
| 1 | Navigate to SecurityModules | Shows only security-related modules (e.g., fail2ban, ufw) |
| 2 | Press `j`/`k` | Navigate modules |
| 3 | Press `Enter` or `e` | Toggle enable/disable |
| 4 | Press `i` | Same as toggle (install/enable) |

### T-HOST-001: Host Selection

> **Setup**: Create multi-host scenario (see section 2).

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `H` | Shows available hosts (desktop + any added) |
| 2 | Press `j`/`k` | Navigate host list |
| 3 | Press `Enter` | Selects host and navigates to Dashboard |
| 4 | Header updates | Shows newly selected host name |
| 5 | Press `c` | Opens SetupWizard to create a new host |

---

## 16. Overlay & Dialog Tests

### T-OVL-001: Simple Confirm Dialog

| # | Action | Expected |
|---|--------|----------|
| 1 | Trigger any simple confirm (e.g., `q` to quit) | 7-line centered popup appears |
| 2 | Dialog shows | Action description, Yes/No prompt |
| 3 | Press `y` or `Enter` | Executes action |
| 4 | Press `n` or `Esc` | Cancels, dialog closes |
| 5 | Press any other key | No-op (dialog stays) |

### T-OVL-002: Enhanced Warning Dialog

| # | Action | Expected |
|---|--------|----------|
| 1 | Trigger enhanced warning (e.g., cleanup with aggressive categories) | 10-line popup with yellow border |
| 2 | Banner | Shows `! HIGH RISK !` |
| 3 | Press `y` or `Enter` | Executes action |
| 4 | Press `n` or `Esc` | Cancels |

### T-OVL-003: Typed Confirmation Dialog

| # | Action | Expected |
|---|--------|----------|
| 1 | Trigger typed confirm (e.g., critical-risk system update) | 12-line popup with red border |
| 2 | Banner | Shows `!! CRITICAL !!` |
| 3 | Shows input field | Must type `CONFIRM` to proceed |
| 4 | Type `C-O-N-F-I-R-M` | Each correctly typed letter turns green |
| 5 | Type wrong letter | Letter turns red |
| 6 | Press `Enter` with correct text | Executes action |
| 7 | Press `Enter` with wrong text | No-op (dialog stays) |
| 8 | Press `Esc` | Cancels |

### T-OVL-004: Help Overlay Interaction

| # | Action | Expected |
|---|--------|----------|
| 1 | Press `?` in any view | Centered help popup (60×34) |
| 2 | Title | "Help: <current_view_name>" |
| 3 | Content sections | View Actions (yellow), Navigation, Global |
| 4 | Dashboard-only | Shows "Iron Concepts" (HOST/BUNDLE/PROFILE/MODULE tree) |
| 5 | Press any key | Overlay closes |
| 6 | Underlying view intact | No state change from help overlay |

### T-OVL-005: Status Message Expiration

| # | Action | Expected |
|---|--------|----------|
| 1 | Trigger a success action (e.g., `r` to refresh) | Green ✓ message appears in footer |
| 2 | Wait 3 seconds | Message auto-expires, footer returns to keybinding hints |
| 3 | Trigger an error (e.g., invalid import path) | Red ✗ message appears in footer |
| 4 | Wait 5 seconds | Error message auto-expires |
| 5 | Error takes priority over status | If both exist, error shown first |

---

## 17. Edge Cases & Error Handling

### T-EDGE-001: Empty State

| # | Action | Expected |
|---|--------|----------|
| 1 | Start with empty `state.json` (`{}`) | Routes to SetupWizard |
| 2 | Press `Esc` from wizard to go to Dashboard | Dashboard shows onboarding nudge: `[w] to get started` |
| 3 | Module list empty | Shows "No modules found" or similar empty state |

### T-EDGE-002: Missing bundles directory

| # | Action | Expected |
|---|--------|----------|
| 1 | Rename `bundles/` temporarily | `mv bundles bundles.bak` |
| 2 | Launch TUI | Bundles list is empty, no crash |
| 3 | Setup Wizard Bundle Selection | Shows empty list or helpful message |
| 4 | Restore | `mv bundles.bak bundles` |

### T-EDGE-003: Corrupt state.json

| # | Action | Expected |
|---|--------|----------|
| 1 | Write invalid JSON to state.json | `echo 'not json' > state.json` |
| 2 | Launch TUI | Should handle gracefully (default state or error message) |
| 3 | Restore | `cp state.json.bak state.json` |

### T-EDGE-004: No Git Repository

| # | Action | Expected |
|---|--------|----------|
| 1 | Run from a non-git directory | `cargo run -p iron-cli -- -r /tmp/iron-test go` |
| 2 | Navigate to Sync view | Shows error or "not a git repository" status |
| 3 | Push/Pull | Shows error message, no crash |

### T-EDGE-005: No Internet Connection

| # | Action | Expected |
|---|--------|----------|
| 1 | Disconnect network | |
| 2 | Refresh updates (`u` → `r`) | Error or timeout message (should not hang indefinitely) |
| 3 | Sync push/pull | Error message with network failure details |
| 4 | Arch news fetch | Graceful failure, shows cached or empty |

### T-EDGE-006: Terminal Resize

| # | Action | Expected |
|---|--------|----------|
| 1 | During TUI operation, resize terminal | TUI redraws correctly |
| 2 | Shrink to very small (e.g., 40×12) | Layout degrades gracefully, no crash |
| 3 | Expand to very large (e.g., 200×60) | Layout fills correctly |

### T-EDGE-007: Rapid Key Presses

| # | Action | Expected |
|---|--------|----------|
| 1 | Rapidly press `j` 50 times | Selection doesn't overshoot past last item |
| 2 | Rapidly press `Tab` 20 times | Cycles through views without desync |
| 3 | Rapidly press `r` for refresh | No duplicate operations or race conditions |

### T-EDGE-008: Long-Running Background Operations

| # | Action | Expected |
|---|--------|----------|
| 1 | Start a push to a slow remote | Background operation starts |
| 2 | Navigate away from Sync view | Other views work normally |
| 3 | Return to Sync view | Shows operation in progress |
| 4 | When operation completes | Result message appears regardless of current view |

---

## 18. Test Results Template

Copy this table for recording test results:

```
Test Session: ____________________
Date: ____________________
Terminal: ____________________ (size: ___×___)
Iron version: ____________________

| Test ID | Description | Result | Notes |
|---------|-------------|--------|-------|
| T-NAV-001 | Tab cycle forward | | |
| T-NAV-002 | Tab cycle backward | | |
| T-NAV-003 | Direct navigation keys | | |
| T-NAV-004 | Esc back navigation | | |
| T-NAV-005 | Help overlay | | |
| T-NAV-006 | Quit flow | | |
| T-WIZ-001 | Auto-route to wizard | | |
| T-WIZ-002 | Wizard step 1 (welcome) | | |
| T-WIZ-003 | Wizard step 2 (host) | | |
| T-WIZ-004 | Wizard step 3 (bundle) | | |
| T-WIZ-005 | Wizard step 4 (profile) | | |
| T-WIZ-006 | Wizard step 5 (confirm) | | |
| T-WIZ-007 | Post-wizard scan | | |
| T-DASH-001 | Dashboard layout | | |
| T-DASH-002 | Maintenance timestamps | | |
| T-DASH-003 | Module progress bar | | |
| T-DASH-004 | Divergence popup | | |
| T-DASH-005 | Quick actions navigation | | |
| T-BUN-001 | Bundle list view | | |
| T-BUN-002 | Bundle list navigation | | |
| T-BUN-003 | Bundle detail view | | |
| T-BUN-004 | Bundle activation | | |
| T-BUN-005 | Bundle deactivation | | |
| T-BUN-006 | Bundle conflict warning | | |
| T-PROF-001 | Profile list view | | |
| T-PROF-002 | Profile detail & activation | | |
| T-MOD-001 | Module list view | | |
| T-MOD-002 | Module detail view | | |
| T-MOD-003 | Module enable/disable | | |
| T-MOD-004 | Module toggle from detail | | |
| T-PB-001 | Launch profile builder | | |
| T-PB-002 | PB step 1 (name/desc) | | |
| T-PB-003 | PB step 2 (modules) | | |
| T-PB-004 | PB step 3 (preview/create) | | |
| T-PB-005 | Duplicate profile name | | |
| T-MC-001 | Launch module creator | | |
| T-MC-002 | MC step 1 (metadata) | | |
| T-MC-003 | MC step 2 (dotfiles) | | |
| T-MC-004 | MC step 3 (preview/create) | | |
| T-MC-005 | Duplicate module name | | |
| T-UPD-001 | Update preview layout | | |
| T-UPD-002 | Pre-flight checks | | |
| T-UPD-003 | Arch news section | | |
| T-UPD-004 | Package list section | | |
| T-UPD-005 | Refresh updates | | |
| T-UPD-006 | Run system update | | |
| T-CLN-001 | Category selection view | | |
| T-CLN-002 | Category navigation | | |
| T-CLN-003 | Cleanup preview | | |
| T-CLN-004 | Cleanup execution (safe) | | |
| T-CLN-005 | Cleanup execution (aggressive) | | |
| T-CLN-006 | Execute from preview | | |
| T-SYNC-001 | Sync auto-refresh | | |
| T-SYNC-002 | Manual status refresh | | |
| T-SYNC-003 | Push (background) | | |
| T-SYNC-004 | Pull (background) | | |
| T-SYNC-005 | Background non-blocking | | |
| T-SYNC-006 | Conflict resolution | | |
| T-SYNC-007 | Secrets lock on push | | |
| T-SEC-001 | Secrets auto-refresh | | |
| T-SEC-002 | Initialize git-crypt | | |
| T-SEC-003 | Lock secrets | | |
| T-SEC-004 | Unlock secrets | | |
| T-SEC-005 | Add GPG key | | |
| T-SEC-006 | Refresh secrets status | | |
| T-REC-001 | Recovery view layout | | |
| T-REC-002 | Export state | | |
| T-REC-003 | Generate install script | | |
| T-REC-004 | Create snapshot | | |
| T-REC-005 | Import state | | |
| T-REC-006 | Import round-trip | | |
| T-MAINT-001 | Maintenance hub | | |
| T-DOC-001 | Doctor health checks | | |
| T-SET-001 | Settings view | | |
| T-LOG-001 | Operation log | | |
| T-CFG-001 | Config manager | | |
| T-SCAN-001 | System scan | | |
| T-SEC-MOD-001 | Security modules | | |
| T-HOST-001 | Host selection | | |
| T-OVL-001 | Simple confirm dialog | | |
| T-OVL-002 | Enhanced warning dialog | | |
| T-OVL-003 | Typed confirmation dialog | | |
| T-OVL-004 | Help overlay interaction | | |
| T-OVL-005 | Status message expiration | | |
| T-EDGE-001 | Empty state | | |
| T-EDGE-002 | Missing bundles directory | | |
| T-EDGE-003 | Corrupt state.json | | |
| T-EDGE-004 | No git repository | | |
| T-EDGE-005 | No internet connection | | |
| T-EDGE-006 | Terminal resize | | |
| T-EDGE-007 | Rapid key presses | | |
| T-EDGE-008 | Long-running background ops | | |

Summary:
- Total tests: 78
- Passed: ___
- Failed: ___
- Partial: ___
- Skipped: ___
```
