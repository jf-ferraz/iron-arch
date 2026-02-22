# Use Cases

Iron TUI — Screen-by-Screen Walkthroughs

This document is the single-source-of-truth for user scenarios. It covers 31 use cases across 9
categories: first run, bundle management, profile & module management, system updates, maintenance,
Git sync, secrets, recovery, and power-user workflows.

**Template** (every use case follows this structure):

```
### UC-N: Title
**Scenario**        — when and who
**Preconditions**   — what must already exist
**TUI Walkthrough** — numbered steps with exact keys and screen names
**What Iron does**  — services called, state mutated, files written
**Success state**   — what the user sees when it works
**Error paths**     — what can fail and how the TUI recovers
```

**Navigation reference** (global hotkeys from any screen):

| Key | Destination |
|-----|-------------|
| `d` | Dashboard |
| `b` | Bundles |
| `p` | Profiles |
| `m` | Modules |
| `x` | SystemMaintenance |
| `u` | UpdatePreview |
| `l` | CleanSystem |
| `s` | Settings |
| `w` | SetupWizard (re-enter) |
| `y` | Sync |
| `Tab` / `Shift+Tab` | Cycle views forward / backward |
| `j`/`k` or `↑`/`↓` | List navigation |
| `Enter` | Select / open detail |
| `Esc` | Go back to previous view |
| `r` | Refresh current view |
| `?` | Toggle help overlay |
| `q` / `Ctrl+c` | Quit |

---

## Category 1 — First Run & Host Setup

### UC-1: First-time setup on a fresh Arch install

**Scenario**: A user installs Arch Linux, clones the Iron repository, and runs `iron go` for
the first time. No `.iron/state/state.json` exists yet. Iron auto-launches the Setup Wizard.

**Preconditions**:
- Iron binary is installed and in PATH.
- Iron repository is cloned (bundles/, profiles/, and modules/ directories exist).
- No `.iron/` runtime state directory exists.

**TUI Walkthrough**:
1. Run `iron go` — `App::init()` cannot create a `StateManager`; TUI navigates to
   **SetupWizard**.
2. **Step 1 — Welcome**: Box displays "Welcome to Iron!" and lists the wizard steps.
   Press `[Enter]` to begin.
3. **Step 2 — Host Setup**: The system hostname is detected and pre-fills the Host ID field.
   - To accept: press `[Enter]` to continue.
   - To customise: press `[e]` to enter edit mode, type the desired ID, press `[Enter]` to
     confirm the input, then `[Enter]` again to advance to the next step.
   - Press `[Backspace]`/`[Esc]` to return to Welcome.
4. **Step 3 — Bundle Selection**: Lists bundles discovered in `bundles/`. Use `[j]`/`[k]`
   to select a desktop environment. Press `[Enter]` to continue.
5. **Step 4 — Profile Selection**: Lists profiles from `profiles/`. Use `[j]`/`[k]` to
   choose a starting profile. Press `[Enter]` to continue.
6. **Step 5 — Confirmation**: Displays Host ID, Bundle, and Profile choices.
   Press `[Enter]` or `[y]` to apply. Press `[Backspace]`/`[Esc]` to go back.
7. **Step 6 — Complete**: Green success banner. Press `[Enter]` to go to the **Dashboard**.

**What Iron does**:
- `WizardState::detect_host()` reads `/etc/hostname`.
- `WizardState::load_bundles()` scans `<config>/bundles/*/bundle.toml`.
- `WizardState::load_profiles()` scans `<config>/profiles/*/profile.toml`.
- On confirm: `WizardState::apply()` writes `.iron/state/state.json` with host, active
  bundle, and active profile, then `App::init()` re-runs to load full application state.
- View transitions: `SetupWizard` → `Dashboard`.

**Success state**:
- Dashboard shows the configured host ID, active bundle badge, and active profile.
- Status bar: "Setup complete! Welcome to Iron."

**Error paths**:
- *No bundles found*: Step 3 shows "No bundles found. Create bundles in your config
  directory." The wizard cannot advance; user must press `[q]` and create bundle files.
- *No profiles found*: Same guard on Step 4.
- *Empty Host ID*: `wizard.can_proceed()` returns false; `[Enter]` on HostSetup does nothing
  until a non-empty ID is present.
- *wizard.apply() fails* (e.g., disk write error): Step 5 shows "Error: <message>" in red.
  The wizard stays on the Confirmation step.

---

### UC-2: Registering a second machine in an existing repo

**Scenario**: The user already runs Iron on a desktop. They clone the same repo onto a laptop
with a different hostname. Iron has no state for the laptop and treats it as first-time setup.

**Preconditions**:
- Iron repo is already set up for the desktop (bundles and profiles exist).
- Repo is cloned onto the laptop; binary is built.
- No `.iron/state/state.json` matching the laptop hostname exists.

**TUI Walkthrough**:
1. Run `iron go` on the laptop — wizard launches (same flow as UC-1).
2. **Step 2 — Host Setup**: Laptop hostname is detected (e.g., `laptop`). Keep or customise.
3. **Step 3 — Bundle Selection**: Pick a laptop-appropriate bundle (e.g., `niri` instead of
   `hyprland`).
4. **Step 4 — Profile Selection**: Pick a lighter profile (e.g., `minimal`).
5. **Step 5 — Confirmation**: Review and apply.
6. Dashboard loads with the laptop's host entry.

**What Iron does**:
- Creates a new host entry in `state.json` keyed by the laptop hostname — isolated from the
  desktop's entry (FR-1.3, FR-7.5).
- Existing desktop state is not modified.

**Success state**:
- Dashboard shows the laptop host ID, chosen bundle, and profile.
- Both host entries coexist in `state.json`.

**Error paths**:
- *Hostname already registered*: The wizard still launches (no state file match). The
  existing entry would be overwritten on confirm. Verify the Host ID on Step 2 first.
- *Repo not up-to-date*: If bundles/profiles weren't pushed from the desktop, the lists in
  Steps 3–4 may be empty. Run `git pull` before launching Iron.

---

### UC-3: Restoring Iron on a replacement machine

**Scenario**: The user's machine breaks. They have a new Arch install and want to restore
their full setup from the Iron repo.

**Preconditions**:
- Iron repo is available (GitHub/GitLab or local backup).
- The previous host ID is known.
- AUR helper (paru/yay) is available if AUR packages are needed.

**TUI Walkthrough**:
1. Install Arch, install Iron binary, clone the repo. Run `iron go` — wizard launches.
2. **Step 2 — Host Setup**: Enter the same host ID as the old machine to restore that host's
   configuration.
3. **Steps 3–5**: Select the same bundle and profile as before, or adjust for new hardware.
4. After wizard completes, press `[y]` → **Sync** view.
5. Press `[s]` to refresh sync status, then `[f]` to pull any upstream changes.
6. Return to Dashboard (`[d]`) and press `[u]` to run a full system update to install
   all packages.
7. Unlock secrets after the pull (see UC-23).

**What Iron does**:
- Wizard reconstructs state.json from scratch for the host.
- `DefaultSyncService::pull()` applies config differences from the remote.
- Package installation runs via `UpdateService` / pacman during the update step.

**Success state**:
- System returned to the last synced state.
- All dotfiles are symlinked; packages are installed.

**Error paths**:
- *Old host ID not known*: Inspect the repo's `hosts/` directory or open `state.json` to
  find the previous entry.
- *Secrets locked*: SSH/GPG keys are encrypted post-clone. See UC-23.
- *AUR helper absent*: AUR package installation stubs with a warning but does not block
  state restoration.

---

## Category 2 — Bundle Management / Desktop Environments

### UC-4: Switching active desktop environment

**Scenario**: The user has Hyprland active and wants to switch to Niri. Both bundles exist.

**Preconditions**:
- At least two bundles in `<config>/bundles/`.
- A bundle is currently active.
- `StateManager` is initialised (state.json exists).

**TUI Walkthrough**:
1. Press `[b]` → **Bundles**.
2. Use `[j]`/`[k]` to navigate to `niri`.
3. Press `[Enter]` → **BundleDetail** (shows packages, services, dotfiles).
4. Press `[a]` to activate. Confirmation dialog appears.
5. Press `[y]` or `[Enter]` to confirm; `[n]` or `[Esc]` to cancel.
6. Status bar: "Switched to bundle: niri".

   Shortcut: from the **Bundles** list, navigate to `niri` and press `[a]` directly without
   opening BundleDetail.

**What Iron does**:
- `ConfirmAction::SwitchBundle("niri")` is dispatched to `App::execute_confirm_action()`.
- `App::switch_bundle()` calls `DefaultBundleService::deactivate(&current.id)` — unlinks
  dotfiles, writes dormant config to `dormant/`.
- Then `DefaultBundleService::activate("niri")` — installs packages (placeholder per
  `install_packages()`), links Niri dotfiles, updates state.
- `StateManager` records the operation in the audit log.
- `app.bundles`, `app.active_bundle`, `app.active_modules` are refreshed.

**Success state**:
- **Bundles** view shows `niri` with `[ACTIVE]`; previous bundle shows `[DORMANT]`.
- **Dashboard** reflects the new active bundle.

**Error paths**:
- *Deactivate current bundle fails*: `set_error("Failed to deactivate current bundle: …")`.
  Switch is aborted; the original bundle remains active.
- *Activate new bundle fails*: `set_error("Failed to activate bundle: …")`. The original was
  already deactivated; state is inconsistent. See UC-26 for rollback.
- *No state manager*: `set_error("No state manager available")`.
- *Conflict detected* (FR-2.5): `BundleService::check_conflicts()` is defined but conflict
  blocking in the TUI is a stub — see FR-2.5.

---

### UC-5: Installing a second bundle alongside the active one

**Scenario**: The user wants Niri installed (packages on disk) alongside Hyprland without
making Niri active yet.

**Preconditions**:
- Active bundle exists (e.g., Hyprland).
- A second `bundle.toml` exists in `bundles/`.

**TUI Walkthrough**:
1. Press `[b]` → **Bundles**. Navigate to `niri`, press `[Enter]` → **BundleDetail**.
2. There is no dedicated "install only, don't activate" key in the TUI.
   *Use the CLI*: `iron bundle install niri`.
3. After CLI install, press `[r]` in **Bundles** to refresh — `niri` shows `[INSTALLED]`.

**What Iron does**:
- `BundleService::activate()` installs packages (placeholder stub) and links configs.
  A distinct "install-only" path is not yet surfaced in the TUI.

**Success state**:
- Niri packages are on disk; Hyprland remains `[ACTIVE]`.

**Error paths**:
- *TUI gap*: "Install without activate" is not a TUI action. The full switch (UC-4) or the
  CLI must be used.
- *Package conflicts* (FR-2.5): `check_conflicts()` warns before install proceeds.

---

### UC-6: Deactivating the current bundle (going dormant)

**Scenario**: The user wants to temporarily remove all active desktop configs without
uninstalling packages.

**Preconditions**: An active bundle exists.

**TUI Walkthrough**:
1. Press `[b]` → **Bundles**. Navigate to the active bundle and press `[Enter]`.
2. There is no standalone "deactivate" key in the current TUI. Two options:
   - Switch to any other bundle (UC-4) — the current bundle is deactivated as the first step.
   - Use the CLI: `iron bundle deactivate <id>`.
3. After deactivation, press `[r]` to refresh — bundle shows `[DORMANT]`.

**What Iron does**:
- `BundleService::deactivate()` unlinks dotfiles and moves configs to `dormant/<bundle-id>/`.
- `StateManager` records the deactivation.

**Success state**:
- Active bundle field on Dashboard reads `(none)`.
- Bundle shows `[DORMANT]` in the list.

**Error paths**:
- *TUI gap*: `ConfirmAction::RemoveBundle` is defined in the code but logs "Bundle removal
  not yet implemented". Standalone deactivation requires the CLI for now.

---

### UC-7: Reactivating a dormant bundle

**Scenario**: The user previously switched from Hyprland to Niri. Hyprland is in `dormant/`.
Now they want Hyprland back.

**Preconditions**:
- Hyprland bundle is in `dormant/` state.
- Hyprland packages are still installed on disk.

**TUI Walkthrough**:
1. Press `[b]` → **Bundles**. The dormant bundle is visible in the list.
2. Navigate to `hyprland`. Press `[a]` (or `[Enter]` then `[a]`).
3. Confirm the switch (`[y]`/`[Enter]`).
4. Iron deactivates Niri, restores Hyprland configs from `dormant/`, and re-links dotfiles.

**What Iron does**:
- Same `switch_bundle()` path as UC-4.
- `BundleService::activate()` reads configs from `dormant/hyprland/` and re-links them.

**Success state**:
- Hyprland shows `[ACTIVE]`; Niri shows `[DORMANT]`.

**Error paths**:
- *Dormant config missing*: If `dormant/hyprland/` files were deleted, activation falls back
  to the `bundles/hyprland/bundle.toml` definition. Error shown if TOML is also missing.

---

## Category 3 — Profile & Module Management

### UC-8: Switching from minimal to developer profile

**Scenario**: The user has been running the `minimal` profile. They need full dev tooling and
want to switch to the `developer` profile.

**Preconditions**:
- `developer` profile exists in `<config>/profiles/developer/profile.toml`.
- A host and active bundle are set; `StateManager` is initialised.

**TUI Walkthrough**:
1. Press `[p]` → **Profiles**.
2. Use `[j]`/`[k]` to navigate to `developer`.
3. Press `[Enter]` → **ProfileDetail** (lists modules included in the profile).
4. Press `[a]` or `[Enter]` to activate the profile.
5. Status bar: "Activated profile: developer".

**What Iron does**:
- `App::activate_selected_profile()` calls `StateManager::set_active_profile(host_id,
  "developer")`.
- `IronState.active_profiles[host_id]` is updated and written to `state.json`.
- Audit log records the profile switch.
- Full dotfile re-linking (FR-3.4) requires `ProfileService` integration (partial).

**Success state**:
- Dashboard shows `developer` as the active profile.
- Settings view shows `developer` under "Active Profile".

**Error paths**:
- *No StateManager or no current host*: `activate_selected_profile()` is a no-op (no error
  shown). Verify state via Settings (`[s]`) or re-run the wizard (`[w]`).
- *Profile TOML missing or invalid*: `Profile::load()` fails; profile does not appear in the
  list. Check TOML syntax.

---

### UC-9: Building a custom profile from scratch

**Scenario**: The user wants a bespoke profile combining nvim + kitty + fish but not the
full developer suite.

**Preconditions**: Desired modules exist in `<config>/modules/`.

**TUI Walkthrough**:
1. In-TUI profile creation is not yet implemented (FR-3.6 — visual profile builder stub).
2. *Workaround*:
   a. Manually create `<config>/profiles/custom/profile.toml` with the desired module IDs.
   b. Press `[p]` and `[r]` in the TUI to refresh — the new profile appears.
   c. Press `[Enter]` on it and `[a]` to activate.

**What Iron does**:
- `App::load_profiles()` rescans `profiles/*/profile.toml` on `[r]`.
- `StateManager::set_active_profile()` records the selection.

**Success state**:
- Custom profile appears in **Profiles** and can be activated.

**Error paths**:
- *TOML syntax error*: `Profile::load()` fails; profile is skipped silently. Validate the
  file against the profile schema in `docs/architecture.md`.
- *FR-3.6 gap*: Visual profile builder in TUI is not yet implemented.

---

### UC-10: Enabling or disabling a single module

**Scenario**: The user wants to activate the `nvim-ide` module without changing anything else.

**Preconditions**:
- `nvim-ide` module exists in `<config>/modules/nvim-ide/module.toml`.
- `StateManager` is initialised.

**TUI Walkthrough**:
1. Press `[m]` → **Modules**.
2. Use `[j]`/`[k]` to navigate to `nvim-ide`.
3. Press `[e]` to toggle. A confirmation dialog appears: "Enable module 'nvim-ide'?" (or
   "Disable" if already active).
4. Press `[y]`/`[Enter]` to confirm.
5. Status bar: "Enabled module: nvim-ide".

   Alternative: press `[Enter]` → **ModuleDetail** to review packages and dotfiles, then
   press `[e]` to toggle.

**What Iron does**:
- `toggle_selected_module()` checks `app.active_modules` to determine current state.
- Dispatches `ConfirmAction::EnableModule("nvim-ide")` or `DisableModule`.
- On confirm: `StateManager::enable_module("nvim-ide")` or `disable_module()`.
- `state.json` is updated; `app.active_modules` refreshed from `sm.active_modules()`.
- Audit log entry is written.

**Success state**:
- Module shows `[ENABLED]` badge in the **Modules** list.
- Dashboard enabled-module count increments.

**Error paths**:
- *StateManager error*: `set_error("Failed to enable module: …")` shown in red; state
  unchanged.
- *Module file missing*: `Module::load()` fails silently; module doesn't appear in list.
- *No StateManager*: `execute_confirm_action()` is a no-op for enable/disable.

---

### UC-11: Resolving a module conflict

**Scenario**: The user tries to enable `kitty-minimal` but it conflicts with the already-
enabled `kitty-dev` — both target `~/.config/kitty/kitty.conf`.

**Preconditions**:
- `kitty-dev` is enabled.
- `kitty-minimal` is in the modules directory with an overlapping dotfile path.

**TUI Walkthrough**:
1. Press `[m]`, navigate to `kitty-minimal`, press `[e]` to toggle.
2. Confirmation dialog appears. `ModuleService::check_conflicts("kitty-minimal")` may detect
   the collision before enabling (FR-4.3).
3. If a conflict warning is shown: press `[Esc]` to cancel.
4. Navigate to `kitty-dev` and press `[e]` to disable it; confirm with `[y]`.
5. Navigate back to `kitty-minimal` and press `[e]` to enable — no conflict this time.

**What Iron does**:
- `ModuleService::check_conflicts()` returns `["kitty-dev"]` if dotfile paths overlap.
- Conflict blocking in the TUI is currently a stub (FR-4.3). The enable action proceeds
  without hard blocking — a warning is surfaced but the operation is not stopped.
- In a full implementation, the confirm dialog would list conflicting modules and require
  the user to resolve before proceeding.

**Success state**:
- `kitty-minimal` is `[ENABLED]`; `kitty-dev` is `[DISABLED]`.
- No duplicate symlink in `~/.config/kitty/`.

**Error paths**:
- *Symlink collision at enable time*: The new symlink overwrites the old one. A warning is
  shown but the operation succeeds (current behaviour).
- *FR-4.3 gap*: Conflict detection does not yet block the enable action.

---

## Category 4 — System Updates & Safety

### UC-12: Standard safe system update (LOW/MEDIUM risk)

**Scenario**: The user wants to update Arch. There are 15 routine package updates with no
kernel, bootloader, or glibc changes (LOW risk).

**Preconditions**:
- Pacman database is accessible.
- Pre-flight checks pass.

**TUI Walkthrough**:
1. Press `[u]` → **UpdatePreview**.
2. Press `[r]` to refresh — checks for updates, fetches Arch News, runs pre-flight checks.
3. Three sections are shown (navigate with `[→]`/`[←]` or `[l]`/`[h]`; items with `[j]`/`[k]`):
   - **Pre-flight Checks** (default): disk space, partial-update detection, AUR staleness.
   - **News**: unacknowledged Arch News items.
   - **Packages**: pending package update list.
4. If news items are present, press `[a]` to acknowledge the selected item, or `[A]` to
   acknowledge all. Unacknowledged blocking news prevents the update.
5. When pre-flight passes and news is clear, press `[u]` to initiate. Confirmation dialog
   appears.
6. Press `[y]`/`[Enter]` to confirm.
7. Post-update checks run automatically: `.pacnew` detection, reboot requirement, failed
   services.

> **Note**: The TUI runs updates in dry-run mode — `run_system_update()` is a stub (see
> `App::run_system_update()`: "dry-run mode"). Full package installation requires the CLI:
> `iron update run`. TUI integration is planned per FR-5.

**What Iron does**:
- `DefaultUpdateService::run_preflight_checks_with_news(&news_items)` → `PreflightResult`.
- `iron_core::assess_risk(&pending_updates, &arch_news)` → `RiskLevel`.
- On confirm: `App::run_system_update()` → `DefaultUpdateService::run_post_update_checks()`.
- `post_update_result` is populated; warning shown if issues found.

**Success state**:
- Status bar: "System update started (dry-run mode)".
- Post-update panel shows any issues (reboot required, config conflicts, failed services).

**Error paths**:
- *Pre-flight fails*: `can_proceed_with_update()` = false; `[u]` shows "Cannot update -
  resolve pre-flight issues first".
- *Blocking news*: `has_critical_news()` = true; `[u]` blocked until news acknowledged.
- *Package manager unavailable*: `refresh_updates()` sets error; update list is empty.

---

### UC-13: Handling a HIGH or CRITICAL risk update

**Scenario**: A kernel update is pending. Iron detects CRITICAL risk (linux + glibc).

**Preconditions**: `linux` or `glibc` appears in `pending_updates`.

**TUI Walkthrough**:
1. Press `[u]` → **UpdatePreview**, `[r]` to refresh.
2. Risk badge shows `[CRITICAL]` (red).
3. Navigate to the **Packages** section (`[→]`/`[→]`). Locate `linux` and `glibc`.
4. Check the **Pre-flight** section — may flag "reboot required after update".
5. Navigate to **News**, review kernel-related advisories. Press `[a]` per item or `[A]` to
   acknowledge all.
6. Once `can_proceed_with_update()` is true, press `[u]`. Confirmation dialog appears.
7. Press `[y]`/`[Enter]`.
   (FR-5.5 specifies CRITICAL requires a typed confirmation string — currently not
   implemented; the standard `[y]` dialog is used.)

**What Iron does**:
- `assess_risk()` returns `RiskLevel::Critical` when `linux`, `glibc`, `systemd`, `nvidia`,
  or `nvidia-dkms` appear in the update list.
- `check_reboot_required()` sets `app.reboot_required = true`.
- Post-update: `post_update_result.reboot_required = true`; warning shown.

**Success state**:
- Update proceeds; warning: "Post-update: Reboot required (2 packages), 1 config conflict".

**Error paths**:
- *Blocking news not acknowledged*: `[u]` shows "Cannot update - resolve pre-flight issues
  first" until news is cleared.
- *Typed confirmation not enforced* (FR-5.5 partial): Only `[y]` is required regardless of
  risk level in the current TUI.

---

### UC-14: Resuming an update interrupted mid-flight

**Scenario**: Power was lost mid-update after 6 of 15 packages installed. Iron resumes from
the last checkpoint on next boot.

**Preconditions**:
- `SavedUpdatePlan` is persisted in `state.json` from the interrupted run.
- `PacmanOutputParser` was tracking per-package progress (FR-5.10).

**TUI Walkthrough**:
1. Run `iron go` — Dashboard loads. A warning indicates a saved update plan.
2. Press `[u]` → **UpdatePreview**. Pre-flight section shows saved plan: completed packages
   and remaining packages.
3. Press `[u]` to resume. Confirmation dialog.
4. Press `[y]`/`[Enter]` — update resumes from the last successful package.

> **Note**: Full resume support is defined in FR-5.10. `SavedUpdatePlan`, `UpdatePhase`,
> and `UpdateProgress` types exist in `iron_core::state`. The TUI rendering of resume state
> and the resume execution loop are partially implemented.

**What Iron does**:
- `StateManager` reads `UpdateProgress` from `state.json`.
- `PacmanOutputParser::parse_line()` tracks `(X/N) upgrading <package>` lines.
- Resume skips packages listed in `CompletedPackage` and continues from the checkpoint.

**Success state**:
- Remaining packages install; `state.json` clears the `SavedUpdatePlan`.

**Error paths**:
- *Broken package state*: A partial install may leave the package database inconsistent.
  Iron warns; manual `pacman -Syu --needed` may be required.
- *Stub limitation*: Full resume execution via TUI is not yet complete (FR-5.10 partial).

---

### UC-15: Reviewing and handling .pacnew files post-update

**Scenario**: After a system update, pacman created `.pacnew` files for `/etc/sudoers` and
`/etc/ssh/sshd_config`.

**Preconditions**:
- A system update has been run.
- `run_post_update_checks()` populated `post_update_result`.

**TUI Walkthrough**:
1. After update, UpdatePreview shows warning: "Post-update: 2 config conflicts
   (.pacnew/.pacsave)".
2. Press `[s]` → **Settings**.
3. Press `[c]` → **ConfigManager**.
4. List shows all `.pacnew` / `.pacsave` conflict file paths.
5. Use `[j]`/`[k]` to navigate to an entry.
6. Press `[Enter]` — TUI shows: "Diff viewer: use 'diff' command on the files shown".
7. Press `[r]` to re-scan for config conflicts.
8. Merge in a terminal using `diff`, `vimdiff`, or `pacdiff`. After merging, press `[r]` to
   confirm the conflict is resolved.

**What Iron does**:
- `App::refresh_config_conflicts()` calls `DefaultUpdateService::find_config_conflicts()` —
  scans the filesystem for `.pacnew` / `.pacsave` files.
- Results stored in `post_update_result.config_conflicts`.
- Diff viewing is hint-only; no external editor is launched from the TUI (FR-5.7 stub).

**Success state**:
- After merging, `[r]` in ConfigManager shows "No configuration conflicts found".

**Error paths**:
- *Files deleted without merging*: Iron no longer sees the conflict on re-scan. User is
  responsible for ensuring the new config was applied.
- *FR-5.7 gap*: Interactive diff/merge in TUI not yet implemented; use `pacdiff`.

---

## Category 5 — System Maintenance

### UC-16: Safe system cleanup (cache, orphans, logs)

**Scenario**: The user wants to reclaim disk space by cleaning package cache, orphan packages,
systemd journal, user cache, thumbnails, and app logs.

**Preconditions**: Iron TUI is running; user has sudo for pacman.

**TUI Walkthrough**:
1. Press `[x]` → **SystemMaintenance**.
2. Press `[c]` (or navigate with `[h]`/`[l]` to the Clean card and press `[Enter]`) →
   **CleanSystem**.
3. Safe categories are pre-selected: PackageCache, OrphanPackages, SystemdJournal, UserCache,
   Thumbnails, AppLogs. Aggressive categories (BrowserCache, DevCache) are not selected.
4. Use `[j]`/`[k]` to navigate; `[Space]` to toggle individual categories.
   - `[s]` selects all safe categories (resets to safe set).
   - `[n]` deselects all.
5. Press `[Enter]` to preview — **CleanupPreview** shows per-category size estimates.
6. Review the total reclaimable space. Press `[c]` to execute, or `[Esc]` to go back.
7. Confirmation dialog: press `[y]`/`[Enter]`.
8. **CleanupResults** shows freed space and item counts.

> **Note**: The TUI runs cleanup in dry-run mode (`service.execute(&categories, dry_run=true)`
> in `App::execute_cleanup()`). Actual deletion requires the CLI: `iron clean run`.

**What Iron does**:
- `CleanupCategory::safe()` pre-selects 6 categories.
- `DefaultCleanupService::preview(&categories)` → `Vec<CleanupPreview>` with
  `space_reclaimable` per category.
- `DefaultCleanupService::execute(&categories, true)` → `CleanupSummary`.
- Navigation: CleanSystem → CleanupPreview → CleanupResults.

**Success state**:
- **CleanupResults**: "Cleanup complete: X.X GB freed from N items" (dry-run figures).
- `cleanup_summary` is populated in app state.

**Error paths**:
- *No categories selected*: `[c]` shows "No categories selected". Select at least one.
- *Preview scan fails* (permission denied): `set_warning` with error; space estimates show 0.
- *Partial failure* (`summary.failed > 0`): "Cleanup completed with N errors: X freed".

---

### UC-17: Aggressive cleanup including dev and browser caches

**Scenario**: A power user wants a deep clean including npm/yarn/pip/cargo build caches and
browser caches to reclaim several GB.

**Preconditions**: User understands browser session data and build caches will be removed.

**TUI Walkthrough**:
1. Press `[l]` → **CleanSystem** (or `[x]` → `[c]`).
2. Press `[a]` — selects all 8 categories including BrowserCache and DevCache. Warning shown:
   "Selected all categories (including aggressive)".
3. Press `[Enter]` → **CleanupPreview**. Aggressive categories are flagged visually.
4. Review totals. Press `[c]` to execute.
5. Confirm dialog, press `[y]`/`[Enter]`.
6. **CleanupResults**.

**What Iron does**:
- `select_all_cleanup_categories()` sets `cleanup_categories` to all 8 values.
- `set_warning("Selected all categories (including aggressive)")` fires immediately.
- `CleanupCategory::is_aggressive()` = true for BrowserCache and DevCache.

**Success state**:
- **CleanupResults** reports totals across all 8 categories (dry-run in TUI).

**Error paths**:
- *Browser running*: Cache files may be locked; those items appear in `summary.failed`.
- *Build artefacts in use*: Same partial failure; accessible items are cleaned.
- *Accidental aggressive select*: Press `[s]` to revert to safe-only before executing.

---

### UC-18: Running system health diagnostics

**Scenario**: The user suspects broken symlinks or missing packages.

**Preconditions**: Iron TUI is running.

**TUI Walkthrough**:
1. Press `[x]` → **SystemMaintenance**.
2. Navigate with `[h]`/`[l]` to the Doctor card, or press `[d]`.
3. TUI shows info: "System Doctor coming soon".

> **Current status**: System Doctor is a stub (handlers.rs lines 184, 205). The planned
> feature (FR-6.4) will validate symlinks, verify installed packages, and check service health.
> Use the Settings → OperationLog path (`[s]` → `[o]`) and ConfigManager (`[s]` → `[c]`) for
> manual triage in the interim.

**What Iron does**:
- (Planned — FR-6.4) Validate all active module symlinks exist and point to correct targets.
- (Planned) Verify all bundle packages are installed via `pacman -Q`.
- (Planned) Check enabled systemd services are active.
- (Planned) Report a health summary on the Dashboard.

**Success state**:
- (Planned) Doctor screen lists all checks: ✓ or ✗ per item with actionable remediation.
- *Currently*: The info message "System Doctor coming soon" is the only output.

**Error paths**:
- *Currently*: No checks are performed; only an info message is shown.

---

## Category 6 — Git Sync

### UC-19: Pushing local config changes to remote

**Scenario**: The user modified their Neovim config, enabled a new module, and switched
profiles. They want to persist these changes to the remote Git repo.

**Preconditions**:
- Iron repo is a Git repository with a remote configured.
- `DefaultSyncService` can reach the remote.

**TUI Walkthrough**:
1. Press `[y]` → **Sync**.
2. Press `[s]` to refresh status — shows branch, commits ahead/behind, dirty file count.
3. If `status = Dirty` or `Ahead`: press `[p]` to push.
4. Iron commits pending changes and pushes to the remote branch.
5. Status bar: "Changes pushed successfully".
6. Press `[s]` — `SyncInfo.status` should resolve to `UpToDate`.

**What Iron does**:
- `DefaultSyncService::status()` runs `git status` + `git rev-list` → `SyncInfo`.
- `DefaultSyncService::push()` calls `git push` via `Command::new("git")`.
- On success: `refresh_sync_status()` is called automatically.
- Audit log records the sync operation.

**Success state**:
- **Sync** view: `SyncStatus::UpToDate`, `commits_ahead = 0`.
- Status bar: "Changes pushed successfully".

**Error paths**:
- *Remote unreachable*: `set_error("Push failed: …")`; status unchanged.
- *Auth failure* (SSH key not loaded): Git returns non-zero; error shown. Load SSH key with
  `ssh-add` in a terminal.
- *No state manager*: `set_error("No state manager available")`.

---

### UC-20: Pulling config changes to a second machine

**Scenario**: Desktop was updated and pushed. Laptop needs to pull those changes.

**Preconditions**:
- Laptop has the repo cloned.
- Remote has commits the laptop hasn't seen (`SyncStatus::Behind`).

**TUI Walkthrough**:
1. Press `[y]` → **Sync**, `[s]` to refresh. Status shows `Behind` (N commits).
2. Press `[f]` to pull — `DefaultSyncService::pull()` runs `git pull`.
3. Status bar: "Changes pulled successfully".
4. Press `[r]` on the relevant view (or restart Iron) to reload state from updated TOML files.

**What Iron does**:
- `DefaultSyncService::pull()` runs `git pull` in the repo root.
- Iron does not auto-apply config changes post-pull; a manual `[r]` or restart is needed.

**Success state**:
- **Sync** shows `UpToDate` after pull.
- New bundle/profile/module TOMLs visible after refresh.

**Error paths**:
- *Dirty working tree*: `SyncStatus::Dirty`; pull may fail. Use `SyncService::stash()` via
  CLI to stash before pulling.
- *Fast-forward not possible* (diverged): `SyncStatus::Diverged`. See UC-21.
- *Network error*: `set_error("Pull failed: …")`.

---

### UC-21: Handling a sync conflict

**Scenario**: `bundles/hyprland/bundle.toml` was edited on both machines without syncing
first. `git pull` reports a merge conflict.

**Preconditions**: `SyncStatus::Diverged` — both local and remote have new commits.

**TUI Walkthrough**:
1. Press `[y]` → **Sync**, `[s]` — status shows `Diverged`.
2. Press `[f]` (pull) — fails with a conflict error message.
3. Status bar: "Pull failed: merge conflict in bundles/hyprland/bundle.toml".
4. Resolve in a terminal:
   ```
   git pull          # shows conflict markers
   git mergetool     # or manually edit
   git add bundles/hyprland/bundle.toml
   git commit -m "merge: resolve hyprland bundle conflict"
   ```
5. Return to TUI, `[s]` — status resolves to `UpToDate` or `Ahead`.
6. Press `[p]` to push the merge commit.

**What Iron does**:
- `SyncService::check_conflicts()` can detect conflicts before pull.
- FR-7.4 (interactive merge) is a stub. The TUI surfaces the error but defers resolution to
  the Git CLI.

**Success state**:
- `SyncStatus::UpToDate` after the merge commit is pushed.

**Error paths**:
- *Conflict in state.json*: Shouldn't occur (host-keyed). If it does, the JSON must be
  manually repaired.
- *FR-7.4 gap*: In-TUI conflict resolution not yet implemented.

---

## Category 7 — Secrets Management

> **TUI status**: Secrets management does not yet have a dedicated TUI screen. The
> `SecretsService` trait and `DefaultSecretsService` exist in
> `iron-core/src/services/secrets.rs`. All workflows below use the CLI
> (`iron secrets …`) until a secrets screen is added.

### UC-22: Setting up secrets for the first time (git-crypt)

**Scenario**: The user has SSH private keys and API tokens to store encrypted in the repo.

**Preconditions**:
- `git-crypt` is installed; GPG is set up with a key; Iron repo is a Git repo.

**TUI Walkthrough** (CLI path — no dedicated TUI screen yet):
1. `iron secrets init` — initialises git-crypt in the repo root.
2. `iron secrets add-gpg-user <GPG-KEY-ID>` — adds the GPG key as authorised decryptor.
3. Place secrets in the `secrets/` directory.
4. `.gitattributes` is updated by git-crypt to mark encrypted paths.
5. `iron sync push` — commit and push secrets encrypted.

**What Iron does**:
- `DefaultSecretsService::init()` runs `git-crypt init`.
- `DefaultSecretsService::add_gpg_user(key_id)` runs `git-crypt add-gpg-user`.
- `DefaultSecretsService::status()` returns `SecretsStatus::Unlocked` after init.

**Success state**:
- `SecretsStatus::Unlocked`. Secrets files are encrypted for others, decrypted for key holder.

**Error paths**:
- *git-crypt not installed*: `status()` returns `SecretsStatus::NotAvailable`.
- *GPG key not found*: `add_gpg_user()` fails with `ServiceError`. Check with `gpg --list-keys`.
- *Already initialised*: `init()` is idempotent; git-crypt warns but does not fail.

---

### UC-23: Unlocking secrets after a fresh clone

**Scenario**: The user cloned the repo on a new machine. Secrets are visible but encrypted.

**Preconditions**:
- GPG private key available (`gpg --list-secret-keys`).
- Repo was initialised with git-crypt; this user's GPG key was added.

**TUI Walkthrough** (CLI path — no dedicated TUI screen yet):
1. `iron secrets status` → shows `SecretsStatus::Locked`.
2. `iron secrets unlock` — git-crypt decrypts using the local GPG key.
3. `iron secrets status` → shows `SecretsStatus::Unlocked`.
4. SSH key symlinks restore after profile activation.

**What Iron does**:
- `DefaultSecretsService::unlock(None)` runs `git-crypt unlock`.
- `unlock(Some(key_path))` uses a symmetric key file instead of GPG.

**Success state**: `SecretsStatus::Unlocked`; SSH keys and API tokens accessible.

**Error paths**:
- *GPG key absent*: `unlock()` fails. Import key first: `gpg --import <key-file>`.
- *Key not authorised*: Another authorised user must run `add_gpg_user()` first.

---

### UC-24: Adding a new SSH key or API token

**Scenario**: The user generated a new SSH key for a new service and wants Iron to manage it.

**Preconditions**: Secrets are unlocked (`SecretsStatus::Unlocked`).

**TUI Walkthrough** (CLI path — no dedicated TUI screen yet):
1. Copy the key to `<iron-repo>/secrets/ssh/<service>/id_ed25519`.
2. Update the relevant `module.toml` with the symlink mapping.
3. `iron secrets list-encrypted` — verify the file is listed.
4. `iron sync push` — commit and push encrypted.

**What Iron does**:
- `DefaultSecretsService::is_encrypted(file)` checks the file is in git-crypt scope.
- `DefaultSecretsService::list_encrypted()` lists all encrypted files.

**Success state**: Key committed encrypted; symlinked to `~/.ssh/<service>/id_ed25519`.

**Error paths**:
- *File not encrypted*: `.gitattributes` pattern doesn't cover the path. Add the pattern and
  verify with `git-crypt status`.

---

## Category 8 — Recovery

### UC-25: Full system recovery from a fresh Arch install

**Scenario**: Machine died. New Arch install, Iron repo on GitHub.

**Preconditions**:
- Arch base system installed; Iron binary available; repo URL known.

**TUI Walkthrough**:
1. `git clone <iron-repo-url>`, then `iron go` — wizard launches. Follow UC-1/UC-3 flow.
2. After wizard, press `[y]` → **Sync**, `[f]` to pull latest.
3. Press `[m]` → **Modules** — enable required modules.
4. Press `[u]` → **UpdatePreview** — run a full system update to install all packages.
5. Unlock secrets (UC-23).
6. Reboot to load new kernel/modules.

**What Iron does**:
- `RecoveryService::import(&export)` restores full state from a `RecoveryExport` JSON file
  pre-generated with `iron recover export`.
- `RecoveryService::generate_install_script()` can produce a shell script for automation
  (UC-27).

**Success state**:
- System restored to last synced state; packages installed; dotfiles symlinked.

**Error paths**:
- *AUR helper absent*: Install `paru` manually before Iron runs.
- *Secrets locked*: Unlock after pull (UC-23).
- *FR-6.5*: Recovery under 30 minutes depends on network and package count. Iron tracks
  elapsed time in `RecoveryExport` but does not enforce the time limit.

---

### UC-26: Rolling back after a failed bundle switch

**Scenario**: Bundle activation for KDE failed mid-way. System is in an inconsistent state.

**Preconditions**: A partial bundle switch was attempted; `TransactionGuard` may have
auto-rolled back.

**TUI Walkthrough**:
1. TUI shows error after the failed switch: "Failed to activate bundle: …".
2. If no bundle is active, press `[b]` → **Bundles**.
3. Navigate to the last-known good bundle (Hyprland) and press `[a]`.
4. Confirm — Iron calls `BundleService::activate("hyprland")` from the dormant state.
5. Alternatively, use the CLI: `iron recover rollback` if a snapshot is available.

**What Iron does**:
- `StateManager` uses `TransactionGuard`: on drop without commit, auto-rollback is attempted
  (see `TransactionGuard::drop()`). The pre-transaction `Transaction.snapshot` is used.
- `BundleService::activate()` from dormant state restores the previous bundle.

**Success state**:
- Hyprland is `[ACTIVE]`; system is consistent; audit log records the recovery.

**Error paths**:
- *Dormant config also corrupted*: Manual repair needed. Check `dormant/hyprland/` for
  missing files.
- *Transaction auto-rollback fails*: `state.json` may need manual repair using
  `Transaction.snapshot` stored in state.
- *FR-2.4 gap*: Pre-switch timeshift/snapper snapshot is defined in FR-2.4 but is a stub.

---

### UC-27: Generating an install script for offline recovery

**Scenario**: The user is preparing for a reinstall and wants a shell script that reproduces
their setup without network access to the Iron binary.

**Preconditions**: Current system state is synced; `RecoveryService` accessible via CLI.

**TUI Walkthrough** (CLI path — no dedicated TUI screen yet):
1. `iron recover export` — writes `RecoveryExport` JSON to `~/.iron/recovery.json`.
2. `iron recover generate-script --include-packages --include-aur --include-services
   --include-modules --include-bundle --aur-helper paru` → `install.sh`.
3. Copy `install.sh` to a USB drive or private Gist.
4. On the fresh install: `bash install.sh`.

**What Iron does**:
- `DefaultRecoveryService::export()` collects: `active_bundle`, `active_profile`,
  `active_modules`, `packages`, `aur_packages`, `services`.
- `generate_install_script(&InstallScriptOptions)` renders a bash script from the export.
- `save_export(path)` writes the JSON file.

**Success state**: `install.sh` installs all packages and enables all services.

**Error paths**:
- *Package list stale*: Packages installed outside Iron may be missing. Run
  `iron state sync-packages` first.
- *AUR helper not specified*: Defaults to `yay`; override with `--aur-helper paru`.

---

## Category 9 — Power User & Advanced

### UC-28: Reviewing the operation audit log

**Scenario**: The user wants to know what Iron did to the system over the past week.

**Preconditions**: Iron has been used; audit log has entries in `state.json`.

**TUI Walkthrough**:
1. Press `[s]` → **Settings**.
2. Press `[o]` → **OperationLog**.
3. Use `[j]`/`[k]` to scroll operations (newest first).
4. Press `[f]` to cycle filter modes: All → Errors → Module → Bundle → Sync → All.
   Status bar: "Filter: Errors" (or whichever is active).
5. Press `[Esc]` to return to Settings.

**What Iron does**:
- `StateManager` writes each operation to `audit.log` (JSONL, max 1000 entries;
  `MAX_AUDIT_ENTRIES`).
- `AuditEntry` fields: `timestamp`, `operation`, `status`, `host_id`, `details`.
- `OperationFilter::cycle_operation_filter()` iterates `OperationFilter::all()` variants.
- Log view reads `state_manager.state().last_operations`.

**Success state**:
- All recent Iron operations visible with timestamps and status.
- Filter narrows to the desired operation type.

**Error paths**:
- *Empty log*: "No operations recorded" if `last_operations` is empty.
- *Log truncated*: Only the most recent 1000 entries are kept. Older entries remain in
  `.iron/state/audit.log` (JSONL).

---

### UC-29: Security hardening a fresh system

**Scenario**: The user has a fresh Hyprland setup and wants to activate UFW, auditd, and
AppArmor security modules.

**Preconditions**:
- Security modules (`ufw`, `auditd`, `apparmor`) exist in `<config>/modules/`.
- User has sudo for systemctl and pacman.

**TUI Walkthrough**:
1. Press `[m]` → **Modules**. The **SecurityModules** view filters modules whose IDs match:
   `security`, `firewall`, `audit`, `ufw`, `firewalld`, `fail2ban`, `auditd`, `apparmor`,
   `selinux`, `clamav`.
2. Use `[j]`/`[k]` to navigate to `ufw`. Press `[e]` or `[Enter]` to toggle.
3. Confirm: `[y]`/`[Enter]`. Status bar: "Enabled module: ufw".
4. Repeat for `auditd` and `apparmor`.
5. Press `[r]` to refresh — all three show `[ENABLED]`.
6. Press `[i]` on a module as an alias for enable (same as `[e]` in current implementation).

**What Iron does**:
- `toggle_selected_module()` → `ConfirmAction::EnableModule("ufw")`.
- `StateManager::enable_module("ufw")` updates `state.json`.
- `ModuleService::enable()` links dotfiles and runs `post_install` hooks (e.g.,
  `systemctl enable --now ufw`) — hook execution is defined in FR-4.4.

**Success state**:
- ufw, auditd, and apparmor show `[ENABLED]` in SecurityModules.

**Error paths**:
- *Package not installed*: `ModuleService::enable()` fails if the binary is absent. Install
  the package first: `sudo pacman -S ufw`.
- *Hook execution partial*: FR-4.4 pre/post hooks may not fire automatically in all TUI
  paths. Verify with `systemctl status ufw`.

---

### UC-30: Multi-machine workflow (desktop + laptop daily)

**Scenario**: Desktop (Hyprland, developer profile) and laptop (Niri, minimal profile) are
maintained in the same repo. User syncs daily.

**Preconditions**: Both hosts registered; remote configured.

**TUI Walkthrough** (two-machine daily loop):
1. Make config changes — enable a module, update a dotfile.
2. Press `[y]` → **Sync**, `[s]` to check status.
3. If `Dirty`: press `[p]` to commit and push.

**TUI Walkthrough — Laptop (morning)**:
1. Press `[y]` → **Sync**, `[s]` to check.
2. Status shows `Behind` (N new commits from desktop).
3. Press `[f]` to pull.
4. Press `[r]` (or restart Iron) to reload state from updated TOML files.
5. Press `[m]` → **Modules**, `[r]` to refresh — new modules appear if any were added.
6. Enable any new modules relevant to the laptop profile.

**What Iron does**:
- Host-keyed state means desktop and laptop configs are isolated in `state.json` by host ID.
- Shared bundle/profile/module TOMLs are version-controlled and pulled together.
- Per-machine state (active bundle, enabled modules) is preserved independently.

**Success state**:
- Laptop has all shared config updates from the desktop.
- Laptop-specific bundle (Niri) and profile (minimal) remain unchanged.

**Error paths**:
- *Dirty laptop state blocks pull*: Stash changes via CLI before pulling.
- *Desktop-only module on laptop*: Module files are shared; enabling on the laptop is
  possible but may lack required hardware. Check `module.toml` for dependency annotations.
- *Simultaneous edits*: See UC-21 for conflict resolution.

---

### UC-31: Diagnosing and recovering from stale/broken state

**Scenario**: Iron behaves oddly — active bundle doesn't match what's linked in `~/.config`,
or a module shows `[ENABLED]` but its dotfiles are missing.

**Preconditions**: `state.json` exists but may have diverged from actual filesystem state.

**TUI Walkthrough**:
1. Press `[x]` → **SystemMaintenance**, press `[d]` — shows "System Doctor coming soon"
   (stub). Manual triage:
2. Press `[s]` → **Settings**, press `[r]` to refresh state from StateManager.
   Review active host, bundle, profile, and module count.
3. Press `[o]` → **OperationLog** — look for recent failures. Press `[f]` to filter by
   Errors.
4. Press `[Esc]`, press `[c]` → **ConfigManager** — scan for `.pacnew` conflicts.
5. If a bundle shows wrong state:
   a. Press `[b]` → **Bundles**, navigate to the stale bundle, press `[a]` to re-activate.
   b. `BundleService::activate()` re-links all dotfiles.
6. If a module symlink is broken:
   a. Press `[m]`, navigate to the module, press `[e]` to disable (confirm), then `[e]`
      again to re-enable. This re-triggers `StateManager::enable_module()`.
7. If `state.json` is corrupted: use the CLI `iron state reset --host <id>` to rebuild the
   host entry from on-disk TOML files.

**What Iron does**:
- `App::refresh_current_view()` reloads bundles/profiles/modules from disk on `[r]`.
- `App::init()` re-reads `StateManager` on Dashboard refresh.
- Re-activating a bundle calls `BundleService::activate()`, re-linking all dotfiles.
- Disable + re-enable cycle calls `StateManager::disable_module()` + `enable_module()`.

**Success state**:
- `state.json` reflects actual filesystem state.
- Dashboard, Bundles, Modules all show consistent status.

**Error paths**:
- *state.json unparseable*: `StateManager::new()` fails; TUI opens Setup Wizard. Use
  `iron state repair` (CLI) or restore from a backup.
- *Dotfile target missing from repo*: `BundleService::link_dotfiles()` errors. Check
  `bundle.toml` and ensure source files exist.
- *FR-6.4 gap*: Automated diagnostics via TUI not yet implemented. Manual triage using
  Settings, OperationLog, and ConfigManager is the current workaround.

---

## Appendix A — View Reference

| View | Key | Description |
|------|-----|-------------|
| Dashboard | `d` | System overview: host, bundle, profile, health |
| Bundles | `b` | List and activate desktop environment bundles |
| BundleDetail | `Enter` from Bundles | Bundle packages, services, dotfiles; `[a]` activate |
| Profiles | `p` | List and activate dotfile profiles |
| ProfileDetail | `Enter` from Profiles | Profile modules; `[a]`/`[Enter]` activate |
| Modules | `m` | List and toggle individual modules; `[e]` enable/disable |
| ModuleDetail | `Enter` from Modules | Module packages, dotfiles; `[e]` enable/disable |
| UpdatePreview | `u` | Pre-flight checks, Arch News, packages; `[u]` update |
| Sync | `y` | Git status; `[p]` push, `[f]` pull, `[s]` refresh |
| SystemMaintenance | `x` | Hub: Update `[u]`, Clean `[c]`, Doctor `[d]` |
| CleanSystem | `l` | Category select: `[Space]` toggle, `[s]`/`[a]`/`[n]`, `[Enter]` preview |
| CleanupPreview | `Enter` from CleanSystem | Per-category estimates; `[c]` execute |
| CleanupResults | `[c]` from CleanupPreview | Freed space report; read-only |
| SecurityModules | sub-screen | Security module list; `[e]`/`[Enter]` toggle, `[i]` install |
| ConfigManager | `[c]` from Settings | `.pacnew`/`.pacsave` conflicts; `[r]` refresh |
| OperationLog | `[o]` from Settings | Audit trail; `[f]` cycle filter |
| Settings | `s` | Config summary; `[o]` log, `[c]` config, `[w]` wizard |
| SetupWizard | `w` | 6-step setup: Welcome→Host→Bundle→Profile→Confirm→Complete |

## Appendix B — Service Reference

| Service | Key methods | Implementation status |
|---------|-------------|----------------------|
| `DefaultBundleService` | `discover()`, `activate()`, `deactivate()`, `switch()`, `check_conflicts()` | Implemented; `install_packages()` is a stub |
| `DefaultModuleService` | `enable()`, `disable()`, `check_conflicts()`, `list_enabled()` | Implemented; conflict blocking is a stub (FR-4.3) |
| `DefaultUpdateService` | `run_preflight_checks_with_news()`, `run_post_update_checks()`, `find_config_conflicts()` | Implemented; TUI forces dry-run mode |
| `DefaultCleanupService` | `preview()`, `execute()` | Implemented; TUI forces `dry_run=true` |
| `DefaultSyncService` | `status()`, `push()`, `pull()`, `commit()`, `check_conflicts()`, `stash()` | Implemented via `git` subprocess |
| `DefaultSecretsService` | `status()`, `init()`, `unlock()`, `lock()`, `add_gpg_user()`, `export_key()` | Implemented; no TUI screen yet |
| `DefaultRecoveryService` | `export()`, `import()`, `generate_install_script()`, `create_backup()` | Implemented; no TUI screen yet |
| `StateManager` | `enable_module()`, `disable_module()`, `set_active_profile()`, `acknowledge_news()` | Implemented; file-locking + JSONL audit log |
