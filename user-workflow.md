# Iron — User Workflow & UX Specification

> **Purpose**: This document translates Iron's system capabilities into user-facing workflows,
> organized by persona scenarios. It explains what each functionality does, how the user
> interacts with it through the TUI/CLI, and what happens under the hood. It serves as the
> bridge between product design and engineering implementation.
>
> **Audience**: Development team and product/design — technical enough for implementation,
> structured around user experience.
>
> **Companion docs**: `docs/requirements.md` (formal FR/NFR), `docs/use-cases.md` (31 detailed
> use cases), `docs/architecture.md` (system design).

---

## Status Legend

| Marker       | Meaning |
|--------------|---------|
| `[EXISTING]` | Feature is fully specified in requirements.md / use-cases.md and implemented |
| `[STUB]`     | Feature is specified but implementation is partial or placeholder |
| `[NEW]`      | Proposed enhancement originating from brainstorm — not yet in the formal spec |

---

## Personas

| Priority  | Persona              | Description |
|-----------|----------------------|-------------|
| PRIMARY   | **The Newcomer**     | New to Linux/Arch. Knows rolling-release requires careful updates. Wants to rice their system but doesn't know how to manage dotfiles. Demands a clean, performant system. Wants to replicate config across machines. |
| SECONDARY | **The Power User**   | Experienced Arch user with multiple machines. Manages custom bundles/profiles, uses git sync across devices, handles secrets and security modules. |
| TERTIARY  | **The Recovery User** | System broke — failed update, botched bundle switch, or hardware replacement. Needs to restore from snapshot or rebuild from scratch on new hardware. |

---

## Iron's Core Data Hierarchy

Before diving into workflows, understanding Iron's data model is essential:

```
HOST (hardware identity)
 └── BUNDLE (desktop environment: Hyprland, Niri, KDE...)
      └── PROFILE (named module collection: developer, minimal...)
           └── MODULE (single app config: nvim-ide, kitty-dev...)
                └── DOTFILE (symlinked config files)
```

- **One host** = one physical/virtual machine, identified by hostname
- **One active bundle** at a time (others stored dormant)
- **One active profile** per bundle (can switch without reinstalling packages)
- **Modules** can be shared across profiles and toggled independently

**Configuration examples:**

Bundle (`bundles/hyprland/bundle.toml`):
```toml
id = "hyprland"
name = "Hyprland Desktop"
description = "Dynamic tiling Wayland compositor with stunning animations"
bundle_type = "WaylandCompositor"
packages = ["hyprland", "waybar", "wofi", "hyprpaper", "hypridle", "hyprlock",
            "xdg-desktop-portal-hyprland", "wl-clipboard", "cliphist",
            "grim", "slurp", "mako"]
aur_packages = ["hyprshot"]
profiles = ["minimal", "developer", "gaming", "streamer"]
default_profile = "minimal"
conflicts = ["niri", "sway", "kde"]
services = ["pipewire", "pipewire-pulse", "wireplumber"]
post_install = "scripts/setup-hyprland.sh"
```

Profile (`profiles/developer/profile.toml`):
```toml
id = "developer"
name = "Developer"
description = "Full development environment with IDE-like terminal experience"
modules = ["nvim-ide", "kitty-dev", "waybar-dev", "dev-tools",
           "git-config", "tmux-config", "starship-prompt"]
theme = "catppuccin-mocha"
shell = "fish"
```

Module (`modules/nvim-ide/module.toml`):
```toml
id = "nvim-ide"
name = "Neovim IDE"
description = "Neovim configured as a full IDE with LSP, completion, and debugging"
kind = "AppConfig"
packages = ["neovim", "ripgrep", "fd", "lazygit", "nodejs", "npm"]
conflicts = ["vim-minimal"]

[[dotfiles]]
source = "config/nvim"
target = "~/.config/nvim"
link = true
```

Host (`hosts/desktop.toml`):
```toml
id = "desktop"
name = "Desktop Workstation"
installed_bundles = []

[hardware]
cpu = "AMD Ryzen 7 9800X3D 8-Core Processor"
gpu = "AMD/ATI Navi 44 [Radeon RX 9060 XT]"
ram_mb = 31191
chassis = "Desktop"

[[hardware.monitors]]
output = "DP-1"
resolution = "2560x1440"
refresh_rate = 60
scale = 1.0
```

---

# Scenario 1 — The Newcomer

> **User profile**: Never installed Iron. New to Linux/Arch. Knows that rolling-release
> distros require careful package management and that mistakes can break the system. Wants
> to start customizing (ricing) but doesn't know how to manage multiple dotfiles. Demands
> a performant, clean system. Wants to save all configuration so it can be replicated on
> other machines.

The Newcomer's journey is a linear progression through Iron's features. Each phase builds
on the previous one, guiding the user from zero to a fully managed, reproducible system.

---

## Phase 1 — First Launch & Host Setup `[EXISTING]`

> **Trigger**: User installs Arch, clones the Iron repo, and runs `iron go` for the first
> time. No `.iron/state/state.json` exists.
>
> **Maps to**: UC-1, FR-1.1–FR-1.5, FR-9.1
>
> **Service**: `HostService::detect_current()`, `HostService::catalog_hardware()`
>
> **TUI View**: `SetupWizard`

### What happens

Iron detects no existing state and auto-launches the **Setup Wizard** — a 6-step guided
flow that collects the minimum information needed to manage this machine.

### TUI Walkthrough

**Step 1 — Welcome**
```
┌── Welcome to Iron! ────────────────────────┐
│                                             │
│  Iron will help you manage your Arch Linux  │
│  system. This wizard takes ~2 minutes.      │
│                                             │
│  Steps:                                     │
│   1. Welcome          (you are here)        │
│   2. Host detection                         │
│   3. Bundle selection                       │
│   4. Profile selection                      │
│   5. Confirmation                           │
│   6. Complete                               │
│                                             │
│  [Enter] Begin    [q] Quit                  │
└─────────────────────────────────────────────┘
```
Press `Enter` to proceed.

**Step 2 — Host Detection**
- Iron reads the system hostname and runs `HostService::catalog_hardware()`.
- The wizard displays detected specs: CPU, GPU, RAM, chassis type, connected monitors.
- If `hosts/<hostname>.toml` already exists (e.g., cloned repo), Iron matches it. If not,
  a new host entry is created automatically.
- Keys: `Enter` to accept detected host, `e` to edit host name.

**Step 3 — Bundle Selection**
- Lists all available bundles discovered from `bundles/*/bundle.toml`.
- Each entry shows: name, description, package count, profile count.
- Keys: `j`/`k` to navigate, `Enter` to select.

**Step 4 — Profile Selection**
- Lists profiles available for the selected bundle (from `bundle.toml:profiles[]`).
- Default profile is pre-highlighted (from `bundle.toml:default_profile`).
- Keys: `j`/`k` to navigate, `Enter` to select.

**Step 5 — Confirmation**
- Summary screen showing all choices: host, bundle, profile, modules that will be enabled.
- Keys: `Enter` to confirm, `Esc` to go back and change selections.

**Step 6 — Complete**
- Iron writes `state.json` with the selected host, bundle, and profile.
- Activates the bundle: installs packages, enables services, links dotfiles.
- Displays success message with next-step hints.
- Keys: `Enter` to continue to Dashboard.

### What Iron does under the hood

1. `App::init()` fails to load `StateManager` → routes to `SetupWizard` view.
2. `HostService::detect_current()` reads system hostname.
3. `HostService::catalog_hardware()` probes CPU, GPU, RAM, monitors, chassis.
4. `BundleService::discover()` scans `bundles/` directory for `bundle.toml` files.
5. `ProfileService::list_profiles()` loads profiles for the selected bundle.
6. `StateManager::new()` creates `state.json` with initial state.
7. `BundleService::activate()` installs packages, enables systemd services,
   runs `post_install` hook, links dotfiles via stow.
8. `ProfileService::select()` sets active profile, enables its modules.
9. `StateManager` writes a JSONL audit log entry for the entire wizard completion.

### Error paths

| Error | Recovery |
|-------|----------|
| Hardware detection fails | Wizard proceeds with partial info; user can edit manually |
| Bundle activation fails mid-install | `TransactionGuard` rolls back state changes; wizard stays on Step 5 |
| Selected bundle has unresolvable conflicts | Warning dialog; user selects a different bundle |

---

## Phase 1.5 — Host Selection Screen `[NEW]`

> **Enhancement**: Beyond first-run, Iron could show a host selection screen every time it
> opens if multiple hosts are registered. This surfaces the multi-machine model early.
>
> **Proposed TUI**: New view before Dashboard, shown only when multiple hosts exist.

### Proposed TUI Layout

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

- Selecting a host loads its `state.json` context (active bundle, profile, modules).
- Creating a new host triggers the Setup Wizard for that host.
- If only one host exists, this screen is skipped — goes straight to Dashboard.

### Integration points

- Extends `HostService::list_hosts()` to provide selection data.
- `StateManager` would need a `switch_host()` method to change `current_host`.
- Historical scan data `[NEW]` would be scoped per host ID.

---

## Phase 2 — System Scan `[NEW]`

> **Origin**: Brainstorm idea. Currently partially covered by `iron doctor` (FR-10.3 checks
> installed packages) but not as a distinct, user-facing workflow with detailed audit output.
>
> **Proposed Service**: New `ScanService` or extension of `HostService`
>
> **Proposed TUI View**: `SystemScan` (new view, accessible from Dashboard or SystemMaintenance)

### What this feature would do

System Scan is an at-a-glance audit of the relationship between what's physically installed on
the system and what Iron's configuration hierarchy declares should be installed. It answers:

1. **Is my system in sync with my Iron config?**
2. **Are there packages I installed manually that Iron doesn't know about?**
3. **Are there dotfiles not managed by any Iron module?**

### Proposed workflow

1. User navigates to System Scan (proposed hotkey: `n` from Dashboard, or menu item in
   `SystemMaintenance`).
2. Iron runs `HostService::catalog_hardware()` to refresh system specs.
3. Iron queries `PackageManager` for all installed packages (`pacman -Q`).
4. Iron computes the union of all packages defined across the active HOST → BUNDLE → PROFILE
   → MODULE hierarchy.
5. Iron compares installed vs. declared and produces an audit report.

### Proposed audit report output

```
┌── System Scan Report ──────────────────────┐
│                                             │
│  Host: desktop (AMD Ryzen 7 · 32GB RAM)    │
│  Bundle: hyprland   Profile: developer      │
│                                             │
│  ┌─ Package Summary ─────────────────────┐  │
│  │ Total Installed     │          847    │  │
│  │ Managed by Iron     │          143    │  │
│  │ Updates Available   │           12    │  │
│  │ Unmatched           │          704    │  │
│  │ Unlinked            │           23    │  │
│  └─────────────────────┴────────────────┘  │
│                                             │
│  [u] View unmatched  [l] View unlinked      │
│  [d] View details    [Esc] Back             │
└─────────────────────────────────────────────┘
```

**Term definitions:**
- **Managed by Iron**: packages declared in the active bundle + profile + enabled modules.
- **Unmatched**: packages installed on the system but not declared in any Iron config file.
  These are packages the user installed manually outside Iron.
- **Unlinked**: packages that are declared in Iron modules but have no associated `[[dotfiles]]`
  entry — Iron manages the package but not its configuration.
- **Updates Available**: packages with newer versions in the repos (from `checkupdates`).

### Historical scan registry `[NEW]`

Each scan result would be appended to a per-host historical registry:

```json
{
  "host_id": "desktop",
  "scans": [
    {
      "timestamp": "2026-02-19T10:30:00Z",
      "total_installed": 847,
      "managed": 143,
      "updates_available": 12,
      "unmatched": 704,
      "unlinked": 23
    }
  ]
}
```

This enables trend tracking — the user can see how managed packages increase over time as
they bring more of their system under Iron's control.

---

## Phase 3 — Dashboard Orientation `[EXISTING]`

> **Trigger**: After wizard completion (or on subsequent launches), user lands here.
>
> **Maps to**: FR-9.2, FR-9.3
>
> **Service**: `StateManager`, `HostService`, `BundleService`, `UpdateService`
>
> **TUI View**: `Dashboard` (hotkey: `d`)

### What the user sees

The Dashboard is Iron's home screen — a single-pane overview of the entire system state:

```
┌── Iron Dashboard ──────────────────────────────────────────┐
│                                                             │
│  Host: desktop              Bundle: Hyprland Desktop        │
│  Profile: developer         Modules: 7 enabled              │
│                                                             │
│  ┌─ Health ──────────────┐  ┌─ Recent Operations ────────┐ │
│  │ ● State: OK           │  │ Bundle activated: hyprland  │ │
│  │ ● Symlinks: OK        │  │ Profile set: developer      │ │
│  │ ● Packages: OK        │  │ Module enabled: nvim-ide    │ │
│  │ ▲ Snapshot: Missing   │  │ Module enabled: kitty-dev   │ │
│  │ ● Git: Clean          │  └────────────────────────────┘ │
│  └───────────────────────┘                                  │
│                                                             │
│  ┌─ Alerts ──────────────────────────────────────────────┐ │
│  │ ⚠ No snapshot exists for this host (FR-1.5)           │ │
│  │ ⚠ 12 updates available (3 flagged)                    │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  [b]undles  [p]rofiles  [m]odules  [u]pdate  [x]maint     │
│  [y]sync    [s]ettings  [?]help    [q]uit                  │
└─────────────────────────────────────────────────────────────┘
```

### Navigation from Dashboard

| Key | Destination | Purpose |
|-----|-------------|---------|
| `b` | Bundles | Manage desktop environments |
| `p` | Profiles | Switch module collections |
| `m` | Modules | Toggle individual app configs |
| `u` | UpdatePreview | Preview and run system updates |
| `x` | SystemMaintenance | Hub for update, clean, doctor |
| `l` | CleanSystem | Direct access to cleanup |
| `y` | Sync | Git push/pull configuration |
| `s` | Settings | Config, logs, wizard re-entry |
| `w` | SetupWizard | Re-run the setup wizard |
| `?` | Help Overlay | Show all keybindings |
| `Tab` | Next view | Cycle: Dashboard → Bundles → Profiles → Modules → SystemMaintenance → UpdatePreview → Sync → Settings |
| `Shift+Tab` | Previous view | Reverse cycle |
| `q` / `Ctrl+c` | Quit | Exit Iron |

### What Iron does

- `StateManager` loads `state.json` to populate host, bundle, profile, modules.
- Dashboard data is built from cached state — no external commands on load.
- Alert badges are computed: missing snapshot (FR-1.5), pending updates, git dirty status.
- `last_operations[]` from state provides the "Recent Operations" panel.

---

## Phase 4 — Bundle Exploration & Selection `[EXISTING]`

> **Trigger**: Newcomer wants to explore desktop environments or switch their DE.
>
> **Maps to**: UC-4 through UC-7, FR-2.1–FR-2.6
>
> **Service**: `BundleService::discover()`, `BundleService::activate()`,
> `BundleService::deactivate()`, `BundleService::switch()`
>
> **TUI Views**: `Bundles` (hotkey: `b`), `BundleDetail`

### Bundles list view

```
┌── Bundles ─────────────────────────────────────────────┐
│                                                         │
│  > Hyprland Desktop          [ACTIVE]                   │
│    Dynamic tiling Wayland compositor                    │
│    12 packages · 4 profiles · 3 services                │
│                                                         │
│    Niri Desktop              [NOT INSTALLED]            │
│    Scrollable-tiling Wayland compositor                 │
│    12 packages · 3 profiles · 3 services                │
│                                                         │
│  [j/k] Navigate  [Enter] Details  [a] Activate          │
│  [Esc] Back      [?] Help                               │
└─────────────────────────────────────────────────────────┘
```

**Bundle states**: `NOT_INSTALLED` → `DORMANT` ↔ `ACTIVE` (with intermediate:
`ACTIVATING`, `DEACTIVATING`, `FAILED`).

### Bundle detail view

Press `Enter` on a bundle to see its full specification:

```
┌── Hyprland Desktop ────────────────────────────────────┐
│                                                         │
│  Status: ACTIVE                                         │
│  Type: WaylandCompositor                                │
│  Conflicts with: niri, sway, kde                        │
│                                                         │
│  ┌─ Packages (12) ──────────────────────────────────┐  │
│  │ hyprland · waybar · wofi · hyprpaper · hypridle  │  │
│  │ hyprlock · xdg-desktop-portal-hyprland           │  │
│  │ wl-clipboard · cliphist · grim · slurp · mako    │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌─ AUR (1) ────┐  ┌─ Services (3) ────────────────┐  │
│  │ hyprshot      │  │ pipewire · pipewire-pulse     │  │
│  └──────────────┘  │ wireplumber                    │  │
│                     └────────────────────────────────┘  │
│  ┌─ Profiles (4) ───────────────────────────────────┐  │
│  │ minimal · developer · gaming · streamer          │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  [a] Activate/Switch  [Esc] Back                        │
└─────────────────────────────────────────────────────────┘
```

### Bundle switch workflow (UC-4)

When the user activates a different bundle while one is already active:

1. Iron detects an active bundle exists → prompts: "Switch from Hyprland to Niri?"
2. **Pre-switch snapshot** is created `[STUB]` — `SnapshotManager::create()` (timeshift/snapper).
3. Current bundle is **deactivated**: configs moved to `dormant/<bundle_id>/`, services
   disabled, state updated to `DORMANT`.
4. New bundle is **activated**: packages installed `[STUB]`, services enabled, dotfiles symlinked,
   `post_install` hook runs, state updated to `ACTIVE`.
5. If activation fails at any point, `TransactionGuard` rolls back: dormant configs restored,
   previous bundle reactivated.

**Conflict detection** (FR-2.5): Before switching, `BundleService::check_conflicts()` compares
package lists. If bundle A declares `conflicts = ["niri"]` and the user tries to activate `niri`,
a warning is shown. Currently warning-only `[STUB]` — does not hard-block.

### CLI equivalents

```bash
iron bundle list              # List all bundles with status
iron bundle list --all        # Include detailed info
iron bundle status hyprland   # Show specific bundle details
iron bundle install niri      # Install bundle (packages coexist, not activated)
iron bundle switch niri       # Deactivate current, activate target
iron bundle switch niri -y    # Skip confirmation
iron bundle remove niri -y    # Remove bundle entirely
```

---

## Phase 5 — Profile & Module Customization `[EXISTING]`

> **Trigger**: Newcomer wants to customize their environment — choose a different set of tools
> or toggle individual application configs.
>
> **Maps to**: UC-8 through UC-11, FR-3.1–FR-3.7, FR-4.1–FR-4.5
>
> **Services**: `ProfileService`, `ModuleService`
>
> **TUI Views**: `Profiles` (hotkey: `p`), `ProfileDetail`, `Modules` (hotkey: `m`),
> `ModuleDetail`, `ProfileBuilder`, `ModuleCreator`

### Profiles view

```
┌── Profiles ────────────────────────────────────────────┐
│                                                         │
│  > developer         [ACTIVE]                           │
│    Full development environment with IDE-like terminal   │
│    7 modules · theme: catppuccin-mocha · shell: fish    │
│                                                         │
│    minimal                                              │
│    Clean setup with essential configurations only        │
│    3 modules · theme: catppuccin-frappe · shell: fish   │
│                                                         │
│  [j/k] Navigate  [Enter] Details  [a] Activate          │
│  [n] New profile [Esc] Back                             │
└─────────────────────────────────────────────────────────┘
```

### Profile activation (UC-8)

When switching from `developer` to `minimal`:

1. Press `a` on `minimal`.
2. Iron confirms: "Switch to minimal? Modules nvim-ide, waybar-dev, dev-tools, git-config,
   tmux-config, starship-prompt will be disabled."
3. `ProfileService::select("minimal")` is called.
4. State updates: `active_profiles` changes, `active_modules` is recalculated from the new
   profile's module list.
5. Dotfile symlinks are updated — old profile's module configs are unlinked, new profile's
   module configs are linked via stow (FR-3.4) `[PARTIAL]`.
6. **No packages are reinstalled** — profile switching only changes dotfile links and state
   (FR-3.2). Both profiles use packages already installed by the bundle.

### Modules view

```
┌── Modules ─────────────────────────────────────────────┐
│                                                         │
│  > nvim-ide          [ENABLED]   AppConfig              │
│    Neovim IDE with LSP, completion, and debugging        │
│                                                         │
│    kitty-dev         [ENABLED]   AppConfig              │
│    Kitty terminal with developer-focused configuration   │
│                                                         │
│    waybar-dev        [ENABLED]   DesktopComponent       │
│    Waybar with developer-focused layout                  │
│                                                         │
│    ufw               [DISABLED]  SystemUtil              │
│    Uncomplicated Firewall configuration                  │
│                                                         │
│  [j/k] Navigate  [Enter] Details  [e] Toggle enable     │
│  [n] New module  [Esc] Back                             │
└─────────────────────────────────────────────────────────┘
```

### Module enable/disable (UC-10)

Press `e` on a module to toggle:

- **Enable**: `ModuleService::enable(id)` → checks for conflicts
  (e.g., `nvim-ide` conflicts with `vim-minimal`), installs packages `[STUB]`, creates
  dotfile symlinks, updates `active_modules[]` in state.
- **Disable**: `ModuleService::disable(id)` → removes dotfile symlinks, updates state.
  Packages are not uninstalled (other modules or the bundle may depend on them).

### Module conflict resolution (UC-11)

If enabling `nvim-ide` while `vim-minimal` is enabled:

1. Iron detects conflict via `ModuleService::check_conflicts()`.
2. Warning dialog: "nvim-ide conflicts with vim-minimal. Disable vim-minimal first?"
3. Currently **warning-only** `[STUB]` — does not hard-block. User can force with
   `iron module enable nvim-ide --force` via CLI.

### Module kinds

| Kind | Description | Examples |
|------|-------------|---------|
| `AppConfig` | Application-specific dotfiles | nvim-ide, kitty-dev |
| `DevTools` | Developer tool packages | dev-tools |
| `Shell` | Shell configuration | starship-prompt |
| `DesktopComponent` | DE-specific component configs | waybar-dev |
| `SystemUtil` | System-level configuration | ufw, fail2ban |

### Profile Builder

Pressing `n` from Profiles view opens a 3-step visual wizard (FR-3.6):

1. Name and describe the new profile.
2. Browse available modules with checkboxes.
3. Dependency resolution: selecting `nvim-ide` auto-suggests `dev-tools`.
4. Conflict warnings: selecting `kitty-dev` warns if `kitty-minimal` is checked.
5. Save → writes `profiles/<name>/profile.toml`.

Currently shows "Profile Builder coming soon." Create profiles manually via TOML or
`iron profile create <id>`.

### CLI equivalents

```bash
iron profile list                    # List profiles for active bundle
iron profile show developer          # Show profile details
iron profile show developer --effective  # Show with resolved module list
iron profile select minimal          # Switch to minimal profile
iron profile create gaming --name "Gaming"  # Create new profile

iron module list                     # List all modules
iron module list --enabled           # Only enabled modules
iron module list --kind AppConfig    # Filter by kind
iron module show nvim-ide            # Show module details
iron module enable nvim-ide          # Enable module
iron module enable nvim-ide --force  # Enable even with conflicts
iron module disable kitty-dev -y     # Disable without confirmation
```

---

## Phase 6 — First System Update `[EXISTING]`

> **Trigger**: Newcomer sees "12 updates available" alert on Dashboard and navigates to
> the update screen. This is the workflow Iron was fundamentally built to make safe.
>
> **Maps to**: UC-12, UC-13, UC-14, UC-15, FR-5.1–FR-5.10
>
> **Service**: `UpdateService::run_preflight_checks_with_news()`,
> `UpdateService::assess_risk()`, `UpdateService::run_post_update_checks()`
>
> **TUI View**: `UpdatePreview` (hotkey: `u`)

### UpdatePreview screen — Three sections

The update screen has three horizontally-arranged sections. Navigate between them with
`h`/`l` or `Tab`.

**Section 1 — Pre-flight Checks**
```
┌─ Pre-flight Checks ────────────────────────┐
│                                             │
│  ● Disk space: 45GB free (OK)              │
│  ● Partial update: None detected (OK)       │
│  ● AUR staleness: 0 stale packages (OK)     │
│  ● Snapshot: ⚠ No recent snapshot            │
│                                             │
└─────────────────────────────────────────────┘
```

**Section 2 — Arch News**
```
┌─ Arch News ────────────────────────────────┐
│                                             │
│  [NEW] 2026-02-15: PHP 8.4 update          │
│    ⚠ Manual intervention required           │
│                                             │
│  [READ] 2026-02-10: Python 3.13 rebuild    │
│                                             │
│  [j/k] Scroll  [Enter] Read full article    │
│  [a] Acknowledge                            │
└─────────────────────────────────────────────┘
```

**Section 3 — Package List with Risk**
```
┌─ Packages (12 updates) ── Risk: MEDIUM ────┐
│                                             │
│  ▲ linux           6.12.1 → 6.12.2  [HIGH] │
│    systemd         256.1 → 256.2    [HIGH]  │
│    mesa            24.3.1 → 24.3.2  [MED]   │
│    waybar          0.10.3 → 0.10.4  [LOW]   │
│    neovim          0.10.2 → 0.10.3  [LOW]   │
│    ...                                      │
│                                             │
│  [j/k] Scroll  [u] Execute update           │
│  [Esc] Cancel                               │
└─────────────────────────────────────────────┘
```

### Risk scoring logic

Iron computes an aggregate risk level that only escalates (never decreases):

| Trigger | Risk Level |
|---------|------------|
| Baseline (no triggers) | **LOW** — "Safe to update" |
| `linux*` kernel packages (excluding headers) | **MEDIUM** |
| > 100 packages to update | **MEDIUM** |
| `nvidia`, `mesa`, `xorg-server`, `wayland`, `plasma-desktop`, `sddm`, etc. | **MEDIUM** |
| `linux`, `linux-lts`, `systemd`, `glibc`, `gcc`, `grub`, `mkinitcpio` | **HIGH** |
| Flagged AUR packages (out-of-date) | **HIGH** |
| Arch News with `requires_manual = true` | **CRITICAL** |

### Update approval behavior by risk

| Risk | Behavior |
|------|----------|
| **LOW** | One-key confirm: `[u]` executes |
| **MEDIUM** | Confirm dialog: "Review recommended. Proceed? [y/n]" |
| **HIGH** | Confirm dialog: "Attention required. Create snapshot first? [y/n/s]" |
| **CRITICAL** | Typed confirmation: user must type "CONFIRM" to proceed (FR-5.5). Enhanced dialog with per-character validation. |

### Update execution

- **In TUI**: pressing `[u]` triggers the risk-differentiated confirmation dialog
  (see table above), then runs a **real system update** via `pacman -Syu`. The
  confirmation level scales with risk to prevent accidental execution of critical updates.
- **Via CLI**: `iron update` (or `iron update --yes` to skip confirmation).
- `SavedUpdatePlan` persists the planned update so it can be resumed if interrupted.
- `PacmanOutputParser` tracks real-time progress of `pacman -Syu` (FR-5.10) — records which
  packages have been installed, allowing `iron update --resume` to pick up where it left off.

### Post-update checks

After a successful update, Iron automatically runs:

1. **`.pacnew` detection** — `UpdateService::find_config_conflicts()` scans for new
   `.pacnew` and `.pacsave` files (FR-5.7). If found, an alert appears on Dashboard.
2. **Reboot requirement** — checks if kernel or systemd was updated.
3. **Failed services** — queries `systemctl --failed` for broken services.

### `.pacnew` handling (UC-15)

Navigate to `Settings` (`s`) → `ConfigManager` (`c`):

```
┌── Config Conflicts ────────────────────────────────────┐
│                                                         │
│  > /etc/pacman.conf.pacnew                              │
│    Modified: 2026-02-19  Size: 2.4KB                    │
│                                                         │
│    /etc/mkinitcpio.conf.pacnew                          │
│    Modified: 2026-02-19  Size: 1.1KB                    │
│                                                         │
│  [Enter] View diff  [Esc] Back                          │
└─────────────────────────────────────────────────────────┘
```

Currently **hint-only** `[STUB]` — shows the conflicts but interactive diff/merge
(FR-5.7) is not yet implemented. Users must resolve `.pacnew` files manually.

### CLI equivalents

```bash
iron update                    # Run safe system update (full execution)
iron update --dry-run          # Preview only (same as TUI behavior)
iron update --force            # Skip pre-flight checks
iron update --no-snapshot      # Don't create auto-snapshot
iron update --resume           # Resume interrupted update
iron update --status           # Show saved update plan status
iron update --clear-progress   # Clear stale progress data
iron update --yes              # Skip all confirmation prompts
```

---

## Phase 7 — System Cleanup `[EXISTING]`

> **Trigger**: Newcomer wants to keep their system tight and clean — no stale caches,
> orphaned packages, or bloated logs.
>
> **Maps to**: UC-16, UC-17, FR-9.6
>
> **Service**: `CleanupService::preview()`, `CleanupService::execute()`
>
> **TUI Views**: `CleanSystem` (hotkey: `l`), `CleanupPreview`, `CleanupResults`

### CleanSystem — Category selection

```
┌── System Cleanup ──────────────────────────────────────┐
│                                                         │
│  Select categories to clean:                            │
│                                                         │
│  [x] Package Cache      Old package versions (3 latest) │
│  [x] Orphan Packages    Unused dependency packages       │
│  [x] Systemd Journal    System logs (vacuum to 100MB)    │
│  [x] User Cache         ~/.cache files older than 30d    │
│  [x] Thumbnails         ~/.cache/thumbnails              │
│  [x] Application Logs   Old logs in ~/.local/share       │
│  [ ] Browser Cache      Firefox/Chrome cache ⚠ AGGRESSIVE│
│  [ ] Developer Cache    npm/yarn/pip/cargo ⚠ AGGRESSIVE  │
│                                                         │
│  [Space] Toggle  [a] Select all  [Enter] Preview         │
│  [Esc] Back                                             │
└─────────────────────────────────────────────────────────┘
```

**Safe categories** (pre-selected): PackageCache, OrphanPackages, SystemdJournal, UserCache,
Thumbnails, AppLogs.

**Aggressive categories** (opt-in, marked ⚠): BrowserCache, DevCache. These can break
active browser sessions or require re-downloading build dependencies.

### CleanupPreview — Size estimates

Press `Enter` to see what will be cleaned:

```
┌── Cleanup Preview ─────────────────────────────────────┐
│                                                         │
│  Category              Estimated Savings                │
│  ──────────────────────────────────────                 │
│  Package Cache         1.2 GB                           │
│  Orphan Packages       340 MB (5 packages)              │
│  Systemd Journal       89 MB                            │
│  User Cache            456 MB                           │
│  Thumbnails            12 MB                            │
│  Application Logs      67 MB                            │
│  ──────────────────────────────────────                 │
│  Total                 2.16 GB                          │
│                                                         │
│  [c] Execute cleanup   [Esc] Back                       │
└─────────────────────────────────────────────────────────┘
```

### CleanupResults — Freed space report

```
┌── Cleanup Complete ────────────────────────────────────┐
│                                                         │
│  ✓ Package Cache:    1.18 GB freed                      │
│  ✓ Orphan Packages:  340 MB freed (5 removed)           │
│  ✓ Systemd Journal:  89 MB freed                        │
│  ✓ User Cache:       423 MB freed                       │
│  ✓ Thumbnails:       12 MB freed                        │
│  ✓ Application Logs: 55 MB freed                        │
│                                                         │
│  Total freed: 2.10 GB                                   │
│                                                         │
│  [Enter] Back to Dashboard                              │
└─────────────────────────────────────────────────────────┘
```

**Note**: TUI runs cleanup in **dry-run mode only**. Full execution requires
`iron clean --all` or `iron clean --cache --orphans` via CLI.

### CLI equivalents

```bash
iron clean                  # Interactive category selection
iron clean --orphans        # Remove orphan packages only
iron clean --cache          # Clean package cache only
iron clean --symlinks       # Fix broken symlinks
iron clean --all            # All safe categories
```

---

## Phase 8 — Git Sync (Save & Replicate Configuration) `[EXISTING]`

> **Trigger**: Newcomer has finished customizing and wants to save everything to a remote
> repo so they can replicate it on another machine.
>
> **Maps to**: UC-19 through UC-21, FR-7.1–FR-7.5
>
> **Service**: `SyncService::push()`, `SyncService::pull()`, `SyncService::status()`
>
> **TUI View**: `Sync` (hotkey: `y`)

### Sync view

```
┌── Git Sync ────────────────────────────────────────────┐
│                                                         │
│  Branch: main                                           │
│  Remote: origin (git@github.com:user/iron-config.git)   │
│                                                         │
│  Status: ✓ Clean                                        │
│  Commits ahead:  2                                      │
│  Commits behind: 0                                      │
│  Dirty files:    0                                      │
│                                                         │
│  [p] Push    [f] Pull    [s] Refresh status              │
│  [Esc] Back                                             │
└─────────────────────────────────────────────────────────┘
```

### Push workflow (UC-19)

1. Press `p` from Sync view.
2. `SyncService::commit()` stages all changes in the Iron repo (TOML configs, state files,
   dotfiles, scripts).
3. `SyncService::push()` pushes to the configured remote.
4. If the push fails (network, auth), error is displayed with retry option.

### Pull workflow (UC-20)

1. Press `f` from Sync view.
2. `SyncService::pull()` fetches from remote and fast-forward merges.
3. If the pull contains config changes (new modules, profile changes), Iron detects and
   applies them — updating state and re-linking dotfiles as needed.
4. If local and remote have diverged, `SyncStatus::Diverged` is detected (UC-21).

### Sync conflict handling (UC-21) `[STUB]`

When local and remote have diverged:

1. Sync view shows: "Status: ⚠ Diverged (3 conflicts)".
2. Ideal: interactive diff/merge within the TUI (FR-7.4).
3. Current: defers to `git` CLI — user must resolve manually, then retry sync.

### What gets synced

Everything in the Iron repo is git-tracked:
- `bundles/` — Bundle definitions and their dotfile configs
- `profiles/` — Profile definitions
- `modules/` — Module definitions and their dotfile configs
- `hosts/` — Host hardware and install parameter configs
- `dormant/` — Inactive bundle configs (preserved but unlinked)
- `scripts/` — Post-install hooks
- `secrets/` — Encrypted secrets (via git-crypt)
- `state.json` — System state (host, active bundle/profile/modules)

### CLI equivalents

```bash
iron sync status                   # Branch, ahead/behind, dirty files
iron sync push                     # Commit all + push
iron sync push --message "msg"     # Custom commit message
iron sync pull                     # Fetch + merge
iron sync pull --stash             # Stash local changes before pull
```

---

## Phase 9 — System Doctor `[EXISTING / STUB]`

> **Trigger**: Something feels wrong, or the newcomer just wants to verify system health.
>
> **Maps to**: UC-18, FR-10.1–FR-10.8
>
> **Service**: Doctor checks in `iron-cli/src/commands/doctor.rs`
>
> **TUI View**: `Doctor` (from `SystemMaintenance` → `d`) — Implemented:
> renders 7 health checks from `app.state` (host configured, bundle active, profile
> selected, modules enabled, packages synced, git sync, doctor status).
> CLI `iron doctor` is also fully implemented.

### SystemMaintenance hub

Navigate to `SystemMaintenance` (hotkey: `x`):

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

### Doctor checks (10 items)

The `iron doctor` CLI command runs these checks and produces a structured JSON report:

| Check | What it validates | Status values |
|-------|-------------------|---------------|
| **state_file** (FR-10.1) | `state.json` exists and is valid JSON | pass / fail |
| **directories** (FR-10.5) | `modules/`, `profiles/`, `bundles/`, `hosts/` exist | pass / fail |
| **current_host** | Host referenced in state is loadable | pass / fail |
| **git** (FR-10.6) | `.git` exists; warns of uncommitted changes | pass / warn |
| **tools** | Required external tools present (`pacman`, `git`) | pass / fail |
| **packages** (FR-10.3) | All active bundle packages are installed | pass / warn / fail |
| **snapshot** (FR-10.4) | `timeshift` or `snapper` available | pass / warn |
| **secrets** (FR-10.7) | `secrets/` dir exists, `git-crypt` initialized | pass / warn |
| **symlinks** (FR-10.2) | All active module symlinks point to valid targets | pass / warn / fail |
| **services** (NFR-11) | Service availability check for degradation handling | pass / warn |

**Output format** (FR-10.8):
```json
{
  "checks": [
    { "name": "state_file", "status": "pass", "message": "State file valid" },
    { "name": "snapshot", "status": "warn", "message": "No snapshot tool found" }
  ],
  "overall": "warn",
  "timestamp": "2026-02-19T10:30:00Z"
}
```

Exit code 1 if any check is `fail`.

### CLI usage

```bash
iron doctor              # Run all health checks
iron doctor --format json  # Machine-readable output
```

---

# Scenario 2 — The Power User

> **User profile**: Experienced Arch user. Manages two or more machines (desktop + laptop).
> Maintains custom bundles and profiles. Uses git sync for cross-machine consistency. Handles
> secrets (SSH keys, API tokens). Leverages security modules and audit logging.

The Power User doesn't need the wizard flow. They interact with Iron primarily through
keyboard shortcuts and CLI commands, and they understand the data hierarchy deeply enough
to create custom configurations.

---

## Workflow A — Multi-Machine Setup `[EXISTING]`

> **Maps to**: UC-2, UC-30, FR-1.3, FR-7.1–FR-7.5
>
> **Service**: `HostService`, `SyncService`

### Registering a second machine (UC-2)

1. Clone the Iron repo on the new machine: `git clone <repo-url>`.
2. Run `iron init --name "Laptop"`.
3. Iron detects the new hostname, runs `HostService::catalog_hardware()`.
4. Creates `hosts/<hostname>.toml` with detected hardware.
5. Creates a host-specific entry in `state.json`:
   ```json
   { "current_host": "laptop", "active_bundles": {}, ... }
   ```
6. User selects a bundle and profile (via wizard or `iron bundle switch <id>`).

### Daily multi-machine workflow (UC-30)

**Morning on desktop:**
```bash
iron sync pull           # Get latest changes from laptop's last session
iron update              # Safe system update
iron sync push           # Push updated state
```

**Evening on laptop:**
```bash
iron sync pull           # Get desktop's changes
# Work, customize, add modules...
iron sync push           # Push laptop's changes
```

**Conflict-free**: `state.json` stores `current_host` — each machine writes to its own
host section. Bundle/profile/module definitions are shared (they're the same TOML files),
but activation state is per-host.

---

## Workflow B — Bundle Switching (Advanced) `[EXISTING]`

> **Maps to**: UC-4 through UC-7, FR-2.1–FR-2.6
>
> **Service**: `BundleService`

### Installing a second bundle alongside the active one (UC-5)

Power users may want to have multiple DEs installed with only one active:

```bash
iron bundle install niri   # Install Niri packages alongside Hyprland
```

- Packages from both bundles coexist on the system (FR-2.3).
- Only one bundle can be `ACTIVE`. The newly installed bundle starts as `DORMANT`.
- Its dotfile configs are stored in `dormant/niri/` — present in the repo but not
  symlinked to `~/.config`.

### Reactivating a dormant bundle (UC-7)

```
TUI: [b] → Bundles → navigate to Niri [DORMANT] → [a] Activate
```

1. Pre-switch snapshot `[STUB]`.
2. Hyprland deactivated → configs moved to `dormant/hyprland/`.
3. Niri activated → configs moved from `dormant/niri/` to active locations.
4. Services swapped: Hyprland-specific services disabled, Niri services enabled.
5. On failure: `TransactionGuard` restores Hyprland.

### Deactivating without switching (UC-6)

For headless/server use or when no DE is needed:

```
TUI: [b] → Bundles → navigate to active bundle → deactivate option
CLI: iron bundle remove hyprland -y
```

System goes to a bundle-less state. Dashboard shows "Bundle: None".

---

## Workflow C — Custom Profile Building `[EXISTING / STUB]`

> **Maps to**: UC-9, FR-3.1–FR-3.7
>
> **Service**: `ProfileService::create()`

### Creating a custom profile

**Via CLI:**
```bash
iron profile create gaming --name "Gaming"
```
This creates `profiles/gaming/profile.toml` with an empty module list. Edit the TOML
to add modules:

```toml
id = "gaming"
name = "Gaming"
description = "Optimized for gaming with minimal background processes"
modules = ["kitty-dev", "waybar-dev", "starship-prompt"]
theme = "catppuccin-mocha"
shell = "fish"
for_bundle = "hyprland"
```

**Via TUI**: `ProfileBuilder` view — press `n` from Profiles. Provides a
3-step wizard (Name/Description → Module checklist → Preview/Create).

### Sharing modules across profiles (FR-3.3, FR-3.7)

Modules are independent of profiles. The same `kitty-dev` module can appear in both
`developer` and `gaming` profile definitions. When switching profiles, only the
**delta** of modules changes — modules present in both profiles stay enabled.

---

## Workflow D — Secrets Management `[EXISTING]`

> **Maps to**: UC-22 through UC-24, FR-8.1–FR-8.6
>
> **Service**: `SecretsService`
>
> **TUI View**: `Secrets` — Implemented: shows git-crypt status, encrypted
> file list, and action keys (lock/unlock). CLI also fully implemented.

### First-time secrets setup (UC-22)

```bash
iron secrets status      # Check git-crypt status
# Initialize git-crypt if not already:
cd <iron-repo> && git-crypt init
# Add your GPG key:
git-crypt add-gpg-user <your-gpg-id>
```

### Storing secrets

Place sensitive files in the `secrets/` directory:
```
secrets/
├── ssh/
│   ├── id_ed25519
│   └── id_ed25519.pub
├── gpg/
│   └── private-key.asc
└── tokens/
    └── github-token
```

These are encrypted at rest by `git-crypt` and only decrypted when unlocked.

### Daily usage

```bash
iron secrets unlock      # Decrypt secrets (requires GPG key)
iron secrets link        # Symlink secrets to their target locations (~/.ssh, etc.)
iron secrets lock        # Re-encrypt secrets
iron secrets status      # Show encryption state
```

### After cloning on a new machine (UC-23)

```bash
git clone <repo>
iron secrets unlock --key ~/.gnupg/my-key.gpg
iron secrets link
```

---

## Workflow E — Security Hardening `[EXISTING]`

> **Maps to**: UC-29
>
> **TUI View**: `SecurityModules`

Security-focused modules (ufw, fail2ban) are grouped in a dedicated view:

```
┌── Security Modules ────────────────────────────────────┐
│                                                         │
│  > ufw               [DISABLED]  SystemUtil              │
│    Uncomplicated Firewall configuration                  │
│                                                         │
│    fail2ban          [DISABLED]  SystemUtil              │
│    Intrusion prevention system                           │
│                                                         │
│  [e] Toggle enable   [Enter] Details   [Esc] Back        │
└─────────────────────────────────────────────────────────┘
```

Enabling a security module installs its packages and applies its configuration. For example,
enabling `ufw` would install the `ufw` package and apply firewall rules defined in
`modules/ufw/config/`.

---

## Workflow F — Audit & Diagnostics `[EXISTING]`

> **Maps to**: UC-28, UC-31
>
> **TUI Views**: `OperationLog` (from Settings → `o`), `Settings` (hotkey: `s`)

### Operation audit log (UC-28)

Navigate to `Settings` → `OperationLog`:

```
┌── Operation Log ───────────────────────────────────────┐
│                                                         │
│  Filter: [ALL]  (cycle with [f])                        │
│                                                         │
│  2026-02-19 10:30  bundle.activate  hyprland    ✓       │
│  2026-02-19 10:31  profile.select   developer   ✓       │
│  2026-02-19 10:31  module.enable    nvim-ide    ✓       │
│  2026-02-19 10:31  module.enable    kitty-dev   ✓       │
│  2026-02-19 11:00  update.execute   12 packages ✓       │
│  2026-02-19 14:30  sync.push        3 files     ✓       │
│                                                         │
│  [f] Cycle filter  [j/k] Scroll  [Esc] Back             │
└─────────────────────────────────────────────────────────┘
```

Every state-mutating operation is recorded in a JSONL audit log by `StateManager`.
Filter categories cycle through: ALL → bundle → profile → module → update → sync → clean.

### Settings view

```
┌── Settings ────────────────────────────────────────────┐
│                                                         │
│  Iron Configuration                                     │
│  Root: /home/user/iron-config                           │
│  State: /home/user/iron-config/state.json               │
│  Host: desktop                                          │
│                                                         │
│  [o] Operation Log    [c] Config Manager                 │
│  [w] Re-run Wizard    [Esc] Back                         │
└─────────────────────────────────────────────────────────┘
```

---

# Scenario 3 — The Recovery User

> **User profile**: Something went wrong. A failed system update left packages in an
> inconsistent state, a bundle switch broke the desktop, or the user is setting up a
> replacement machine and needs to recreate their entire environment from a backup.

Recovery is Iron's safety net. The system is designed so that no operation is
unrecoverable — every state change is logged, every configuration is git-tracked, and
the system can be rebuilt from the repo alone.

---

## Workflow A — Failed Update Recovery `[EXISTING]`

> **Maps to**: UC-14, FR-5.10
>
> **Service**: `UpdateService`, `SavedUpdatePlan`, `PacmanOutputParser`

### Resuming an interrupted update

If `pacman -Syu` is interrupted (power failure, network drop, crash):

1. On next `iron update`, Iron detects a `SavedUpdatePlan` exists.
2. `PacmanOutputParser` has tracked which packages were successfully installed before
   the interruption.
3. Iron prompts: "Previous update was interrupted. Resume? [y/n]"
4. `iron update --resume` continues from the last successfully installed package.

### CLI commands

```bash
iron update --status          # Show saved update plan
iron update --resume          # Resume from last successful package
iron update --clear-progress  # Discard stale progress data and start fresh
```

### Post-failure diagnostics

After an interrupted update, immediately run:

```bash
iron doctor                   # Check system health
iron update --status          # See what was pending
```

The doctor will flag:
- Broken symlinks (if mid-update packages changed paths)
- Failed services (if updated services can't start)
- Package mismatches (if some but not all packages were updated)

---

## Workflow B — Failed Bundle Switch Rollback `[EXISTING]`

> **Maps to**: UC-26, FR-2.4
>
> **Service**: `BundleService::switch()`, `TransactionGuard`

### Automatic rollback on failure

If bundle activation fails mid-process:

1. `TransactionGuard` detects the failure.
2. State changes are rolled back — `state.json` reverts to the previous bundle.
3. Dormant configs are restored — the old bundle's dotfiles are moved back from
   `dormant/` to their active locations.
4. Services are restored — old bundle's services re-enabled.
5. User sees error dialog with details of what failed and confirmation that the
   rollback was successful.

### Manual recovery

If automatic rollback also fails (rare):

```bash
iron bundle status            # See current bundle state
iron bundle switch <old-id>   # Force switch back to the previous bundle
iron doctor                   # Verify system health
```

---

## Workflow C — Full System Recovery `[EXISTING]`

> **Maps to**: UC-3, UC-25, UC-27, FR-6.1–FR-6.5
>
> **Service**: `RecoveryService::export()`, `RecoveryService::import()`,
> `RecoveryService::generate_install_script()`, `RecoveryService::verify_installation()`

### Export current system state

Before disaster strikes, create a recovery export:

```bash
iron recover --export
```

This produces a JSON file containing:
- All installed packages (official + AUR)
- Enabled systemd services
- Active modules, bundle, and profile
- Host hardware configuration

### Recover on a fresh Arch install (UC-25)

1. Install Arch Linux (base system).
2. Clone the Iron repo: `git clone <repo-url>`.
3. Import the recovery export:
   ```bash
   iron recover --import iron-export-20260219.json
   ```
4. Iron runs a 4-step recovery flow:
   - **Step 1 — Install**: Reinstall all packages from the export.
   - **Step 2 — Bundle**: Activate the recorded bundle, link dotfiles.
   - **Step 3 — Profile**: Select the recorded profile, enable modules.
   - **Step 4 — Verify**: `RecoveryService::verify_installation()` checks drivers,
     services, permissions, symlinks.
5. Target: complete recovery in **< 30 minutes**.

### Generate install script for offline recovery (UC-27)

```bash
iron recover --script
```

Produces a standalone `install.sh` Bash script that can be run without Iron installed.
The script includes:
- `pacman -S` commands for all packages
- `systemctl enable` commands for services
- Symlink creation for dotfiles
- Basic verification steps

### TUI View — `Recovery`

Implemented: status panel showing last backup timestamp, plus action keys for
export, import, and install script generation.

---

## Workflow D — Arch Installation Wizard `[NEW]`

> **Origin**: Brainstorm idea. This would extend Iron beyond post-install configuration
> management into the installation phase itself. Currently **out of scope** per
> `docs/project-brief.md` (Iron assumes Arch is already installed).

### Proposed feature

A future `iron install` command (or TUI wizard) that guides bare-metal Arch installation:

1. **Disk partitioning** — guided partition layout based on host config
   (`host.toml:install_params.partitions[]`).
2. **Base system** — `pacstrap` with packages defined in host config
   (`host.toml:install_params.kernel`, `microcode`, etc.).
3. **Bootloader** — install and configure based on `install_params.bootloader`.
4. **Drivers** — GPU drivers based on detected/declared hardware
   (`install_params.gpu_drivers`).
5. **Post-install** — hand off to standard Iron flow (bundle → profile → modules).

### Relationship to existing features

This builds on `RecoveryService::generate_install_script()`, which already produces a
post-install script from host config. The installation wizard would be the front-end
that guides users through the steps that script automates.

### Implementation notes

- Would live as a separate CLI command: `iron install`.
- Requires running in a live Arch ISO environment.
- Host config (`hosts/<id>/host.toml`) already stores `[install_params]` with bootloader,
  kernel, microcode, filesystem, encryption, and partition definitions.
- Phase 2+ enhancement — not blocking for initial release.

---

# Global Navigation Reference

## TUI Hotkeys (available from any screen)

| Key | View | Description |
|-----|------|-------------|
| `d` | Dashboard | System overview home screen |
| `b` | Bundles | Manage desktop environments |
| `p` | Profiles | Switch module collections |
| `m` | Modules | Toggle individual app configs |
| `x` | SystemMaintenance | Hub: update, cleanup, doctor |
| `u` | UpdatePreview | Preview system updates with risk scores |
| `l` | CleanSystem | System cleanup category selection |
| `y` | Sync | Git push/pull configuration sync |
| `s` | Settings | Config summary, log, wizard re-entry |
| `w` | SetupWizard | Re-enter the setup wizard |
| `?` | Help Overlay | Toggle keybinding reference |
| `q` / `Ctrl+c` | — | Quit Iron |

## Tab Cycle Order

`Dashboard` → `Bundles` → `Profiles` → `Modules` → `SystemMaintenance` → `UpdatePreview`
→ `Sync` → `Settings` → (loop)

Use `Tab` for forward, `Shift+Tab` for reverse.

## List Navigation (vim-style)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `h` / `←` | Previous section (in multi-section views) |
| `l` / `→` | Next section (in multi-section views) |
| `Enter` | Select / open detail view |
| `Esc` | Go back to previous view |
| `r` | Refresh current view data |

## Per-View Keybindings

| View | Key | Action |
|------|-----|--------|
| Bundles | `a` | Activate selected bundle |
| BundleDetail | `a` | Activate this bundle |
| Profiles | `a` | Activate selected profile |
| Profiles | `n` | New profile (ProfileBuilder) |
| ProfileDetail | `a` / `Enter` | Activate this profile |
| Modules | `e` | Toggle enable/disable selected module |
| Modules | `n` | New module (ModuleCreator) |
| ModuleDetail | `e` | Toggle enable/disable this module |
| UpdatePreview | `u` | Execute update (dry-run in TUI) |
| UpdatePreview | `h`/`l` | Navigate between 3 sections |
| CleanSystem | `Space` | Toggle category selection |
| CleanSystem | `a` | Select all categories |
| CleanupPreview | `c` | Execute cleanup (dry-run in TUI) |
| Sync | `p` | Push to remote |
| Sync | `f` | Pull from remote |
| Sync | `s` | Refresh git status |
| OperationLog | `f` | Cycle filter category |
| Settings | `o` | Open Operation Log |
| Settings | `c` | Open Config Manager |
| Settings | `w` | Re-run Setup Wizard |

---

# Feature Integration Matrix

This table maps every workflow to its TUI view(s), CLI command(s), core service(s),
configuration file(s), and implementation status.

| Workflow | TUI View(s) | CLI Command(s) | Core Service(s) | Config Files | Status |
|----------|-------------|----------------|-----------------|--------------|--------|
| First-run wizard | `SetupWizard` | `iron init` | `HostService`, `BundleService`, `ProfileService`, `StateManager` | `hosts/*.toml`, `state.json` | Implemented |
| Host detection | `SetupWizard` (Step 2) | `iron host catalog` | `HostService::catalog_hardware()` | `hosts/*.toml` | Implemented |
| Host management | `Dashboard` | `iron host list/current/select` | `HostService` | `hosts/*.toml` | Implemented |
| Host selection screen | — | — | — | — | `[NEW]` Proposed |
| System scan | — | — | — | — | `[NEW]` Proposed |
| Bundle listing | `Bundles` | `iron bundle list` | `BundleService::discover()` | `bundles/*/bundle.toml` | Implemented |
| Bundle activation | `BundleDetail` | `iron bundle switch <id>` | `BundleService::activate()` | `bundles/*/bundle.toml`, `state.json` | Implemented |
| Bundle switching | `Bundles` | `iron bundle switch <id>` | `BundleService::switch()` | `bundles/*/`, `dormant/*/` | Implemented (snapshot `[STUB]`) |
| Bundle conflict check | `BundleDetail` | — | `BundleService::check_conflicts()` | `bundle.toml:conflicts[]` | Warning-only `[STUB]` |
| Profile listing | `Profiles` | `iron profile list` | `ProfileService::list_profiles()` | `profiles/*/profile.toml` | Implemented |
| Profile switching | `ProfileDetail` | `iron profile select <id>` | `ProfileService::select()` | `profiles/*/profile.toml`, `state.json` | Implemented (stow `[PARTIAL]`) |
| Profile creation | `ProfileBuilder` | `iron profile create <id>` | `ProfileService::create()` | `profiles/*/profile.toml` | TUI implemented (3-step wizard), CLI implemented |
| Module listing | `Modules` | `iron module list` | `ModuleService` | `modules/*/module.toml` | Implemented |
| Module enable/disable | `ModuleDetail` | `iron module enable/disable <id>` | `ModuleService::enable()` / `disable()` | `modules/*/module.toml`, `state.json` | Implemented |
| Module conflict check | `ModuleDetail` | — | `ModuleService::check_conflicts()` | `module.toml:conflicts[]` | Warning-only `[STUB]` |
| Module creation | `ModuleCreator` | — | — | `modules/*/module.toml` | TUI implemented (2-step wizard) |
| Update preview | `UpdatePreview` | `iron update --dry-run` | `UpdateService::run_preflight_checks_with_news()`, `assess_risk()` | — | Implemented |
| Update execution | `UpdatePreview` | `iron update` | `UpdateService`, `PacmanOutputParser` | — | Implemented (risk-differentiated confirmation) |
| Update resume | — | `iron update --resume` | `SavedUpdatePlan` | — | Implemented |
| Post-update checks | — | — (automatic) | `UpdateService::run_post_update_checks()` | — | Implemented |
| `.pacnew` handling | `ConfigManager` | — | `UpdateService::find_config_conflicts()` | — | Hint-only `[STUB]` |
| System cleanup | `CleanSystem`, `CleanupPreview`, `CleanupResults` | `iron clean` | `CleanupService::preview()`, `execute()` | — | TUI dry-run only |
| System doctor | `Doctor` | `iron doctor` | Doctor checks (10 items) | — | TUI implemented (7 checks), CLI implemented |
| Git sync | `Sync` | `iron sync status/push/pull` | `SyncService` | — | Implemented |
| Sync conflict | `Sync` | — | `SyncService::check_conflicts()` | — | `[STUB]` (defers to git CLI) |
| Secrets management | `Secrets` | `iron secrets status/unlock/lock/link` | `SecretsService` | `secrets/` | TUI implemented, CLI implemented |
| Recovery export/import | `Recovery` | `iron recover --export/--import` | `RecoveryService` | Export JSON | TUI implemented, CLI implemented |
| Install script gen | — | `iron recover --script` | `RecoveryService::generate_install_script()` | `install.sh` | Implemented |
| Security modules | `SecurityModules` | `iron module enable ufw` | `ModuleService` | `modules/ufw/`, `modules/fail2ban/` | Implemented |
| Audit log | `OperationLog` | — | `StateManager` (JSONL log) | `.iron/audit.jsonl` | Implemented |
| Shell completions | — | `iron completions <shell>` | — | — | Implemented |
| Arch install wizard | — | — | — | `host.toml:install_params` | `[NEW]` Proposed |

---

# Appendix — All TUI Views

Complete list of views defined in the `View` enum (23 views):

| # | View | Hotkey / Access | Status | Description |
|---|------|----------------|--------|-------------|
| 1 | `Dashboard` | `d` | Implemented | System overview: host, bundle, profile, health, alerts |
| 2 | `SetupWizard` | `w` | Implemented | 6-step guided first-run wizard |
| 3 | `Bundles` | `b` | Implemented | List bundles with state badges |
| 4 | `BundleDetail` | `Enter` from Bundles | Implemented | Bundle packages, services, dotfiles |
| 5 | `Profiles` | `p` | Implemented | List profiles for active bundle |
| 6 | `ProfileDetail` | `Enter` from Profiles | Implemented | Profile modules and metadata |
| 7 | `ProfileBuilder` | `n` from Profiles | Implemented | 3-step visual profile creation wizard |
| 8 | `Modules` | `m` | Implemented | List all modules with enable/disable |
| 9 | `ModuleDetail` | `Enter` from Modules | Implemented | Module packages, dotfiles, conflicts |
| 10 | `ModuleCreator` | `n` from Modules | Implemented | 2-step module creation wizard |
| 11 | `UpdatePreview` | `u` | Implemented | Pre-flight, news, packages with risk |
| 12 | `Sync` | `y` | Implemented | Git status, push, pull |
| 13 | `SystemMaintenance` | `x` | Implemented | Hub: update, cleanup, doctor |
| 14 | `CleanSystem` | `l` | Implemented | Cleanup category selection |
| 15 | `CleanupPreview` | `Enter` from CleanSystem | Implemented | Per-category size estimates |
| 16 | `CleanupResults` | After cleanup execution | Implemented | Freed space report |
| 17 | `SecurityModules` | From Modules | Implemented | Security module list |
| 18 | `Doctor` | `d` from SystemMaintenance | Implemented | 7 health checks from app state |
| 19 | `Secrets` | — | Implemented | git-crypt status, encrypted files, actions |
| 20 | `Recovery` | — | Implemented | Status panel, export/import/generate |
| 21 | `ConfigManager` | `c` from Settings | Implemented | `.pacnew`/`.pacsave` viewer |
| 22 | `OperationLog` | `o` from Settings | Implemented | Audit trail with filter cycling |
| 23 | `Settings` | `s` | Implemented | Config summary, log, wizard re-entry |

---

# Appendix — All CLI Commands

| Command | Subcommands / Flags | Purpose |
|---------|---------------------|---------|
| `iron` (no args) / `iron go` | — | Launch TUI Dashboard |
| `iron init` | `--id`, `--name`, `--force` | Initialize Iron on current host |
| `iron status` | — | System status overview |
| `iron update` | `--dry-run`, `--force`, `--no-snapshot`, `--resume`, `--status`, `--clear-progress`, `-y` | Safe system update |
| `iron bundle` | `list [--all]`, `status [id]`, `install <id> [-y]`, `switch <id> [-y]`, `remove <id> [-y]` | Bundle management |
| `iron profile` | `list [--bundle]`, `show <id> [--effective]`, `select <id>`, `create <id> [--name] [--extends]`, `edit <id>` | Profile management |
| `iron module` | `list [--enabled] [--disabled] [--kind]`, `show <id>`, `enable <id> [--force]`, `disable <id> [-y]` | Module management |
| `iron host` | `list`, `current`, `catalog [--update]`, `select <id>`, `snapshot [--description]` | Host management |
| `iron sync` | `status`, `push [--message]`, `pull [--stash]` | Git sync |
| `iron secrets` | `status`, `unlock [--key]`, `lock`, `link` | Secrets management |
| `iron doctor` | — | System health check (10 checks) |
| `iron clean` | `--orphans`, `--cache`, `--symlinks`, `-a/--all` | System cleanup |
| `iron recover` | `--export`, `--import <file>`, `--script` | Recovery workflow |
| `iron completions` | `<shell>` (bash, zsh, fish) | Generate shell completions |

**Global flags** (all commands): `--root <path>`, `--format <text|json|minimal>`,
`-v/--verbose`, `-q/--quiet`, `--no-color`

---

# Appendix — Proposed Enhancements Summary `[NEW]`

These features originated from the brainstorm session and extend the current specification.
They are incorporated inline in the scenarios above but consolidated here for tracking.

| # | Enhancement | Scenario | Description | Dependency |
|---|-------------|----------|-------------|------------|
| 1 | **System Scan** | S1, Phase 2 | Distinct workflow that audits installed packages vs. Iron-declared packages. Produces report: managed, unmatched, unlinked counts. | New `ScanService` or `HostService` extension |
| 2 | **Historical Scan Registry** | S1, Phase 2 | Each scan result appended to per-host history for trend tracking. | System Scan |
| 3 | **Host Selection Screen** | S1, Phase 1.5 | TUI view shown on launch when multiple hosts exist. List identified hosts with create/select. | `HostService::list_hosts()` |
| 4 | **Arch Installation Wizard** | S3, Workflow D | Guided bare-metal Arch installation from within Iron. Extends recovery/install script generation. | `RecoveryService::generate_install_script()` |
| 5 | **Package Audit Report** | S1, Phase 2 | Detailed output format: Total Installed, Managed by Iron, Updates Available, Unmatched, Unlinked. | System Scan |


