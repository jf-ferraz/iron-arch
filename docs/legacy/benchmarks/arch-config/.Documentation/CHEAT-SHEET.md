# DCLI Cheat Sheet

Quick reference guide for all dcli commands and configuration structure.

---

## Core Commands

### `dcli init`
Initialize arch-config directory structure
- `dcli init` - Create new config structure
- `dcli init -b` / `dcli init --bd` - Bootstrap from BlackDon's config

### `dcli sync`
Sync packages to match configuration
- `--dry-run` - Preview changes without applying
- `--prune` - Remove packages not in configuration
- `--force` - Skip confirmation prompts
- `--no-backup` - Skip automatic backup
- `--no-hooks` - Skip post-install hooks
- `--force-dotfiles` - Force re-sync dotfiles

### `dcli install <package>`
Install package with pacman and add to host config

### `dcli remove <package>`
Remove package with pacman (doesn't remove from config)

### `dcli status`
Show current configuration and sync status
- `--json` - Output in JSON format

### `dcli update`
Update system (respects version constraints)
- `--no-backup` - Skip automatic backup

---

## Module Management

### `dcli module list`
List all available modules with status and package counts
- `--json` - Output in JSON format

### `dcli module enable [<module>]`
Enable a module (interactive if no name provided)
- Detects and prompts for conflicts
- Uses fzf for interactive selection
- `--json` - Output in JSON format

### `dcli module disable [<module>]`
Disable a module (interactive if no name provided)
- Uses fzf for interactive selection
- `--json` - Output in JSON format
added a commands cheat sheet. 
### `dcli module run-hook [<module>]`
Run a module's post-install hook
- Interactive selection if no name provided

---

## Hook Management

### `dcli hooks list`
List all hooks and execution status
- `--json` - Output in JSON format

### `dcli hooks reset <module>`
Reset hook to "not run" state (will run on next sync)

### `dcli hooks skip <module>`
Skip a hook permanently

### `dcli hooks run <module>`
Manually run a module's post-install hook

---

## Git Repository

### `dcli repo init`
Initialize git repository for arch-config
- Interactive setup for GitHub, GitLab, or custom

### `dcli repo clone`
Clone existing arch-config repository
- Auto-detects hostname
- Creates host-specific config

### `dcli repo push`
Commit and push changes to remote
- Prompts for commit message

### `dcli repo pull`
Pull updates from remote

### `dcli repo status`
Show repository status and remote URL

---

## Backup/Snapshot

### `dcli backup`
Create a backup snapshot (timeshift or snapper)

### `dcli backup list`
List all backup snapshots

### `dcli backup delete <snapshot>`
Delete a snapshot by ID

### `dcli backup check`
Check backup configuration and status

### `dcli restore [<snapshot>]`
Restore from backup (interactive if no ID provided)

---

## Package Management

### `dcli merge`
Add unmanaged installed packages to system-packages.yaml
- `--dry-run` - Preview without creating file

### `dcli find <package>`
Find where a package is defined in arch-config
- `--json` - Output in JSON format

### `dcli search`
Interactive TUI search for packages (requires fzf and paru)
- Multi-select with TAB
- Shows package info in preview

---

## Validation & Migration

### `dcli validate`
Validate arch-config structure and modules
- `--check-packages` - Verify packages exist in repos (slower)
- `--json` - Output in JSON format

### `dcli migrate`
Migrate from old structure to new structure
- `--dry-run` - Show migration plan without executing
- Creates backup before migration

### `dcli self-update`
Update dcli from git repository
- Auto-detects repository location
- Builds and installs to `/usr/local/bin/dcli`

---

## Global Flags

- `-j, --json` - Output in JSON format (supported by most commands)

---

# Arch-Config Structure

## Directory Layout

```
~/.config/arch-config/              # Main config directory
├── config.yaml                      # Pointer file (contains host name only)
├── hosts/
│   ├── {hostname}.yaml             # Full host configuration
│   └── shared/                      # Optional: shared configs
│       └── common.yaml
├── modules/
│   ├── base.yaml                   # Base packages (all systems)
│   ├── example.yaml                # Example module template
│   └── {category}/                 # Optional: categorized modules
│       └── {module-name}.yaml
├── scripts/                         # Post-install hook scripts
└── state/                           # Auto-generated (git-ignored)
    ├── packages.yaml               # Managed packages state
    ├── hooks.yaml                  # Hook execution status
    └── .gitignore
```

## Configuration Files

### `config.yaml` (Pointer File)
```yaml
# dcli configuration pointer
host: {hostname}
```

### `hosts/{hostname}.yaml` (Full Configuration)
```yaml
host: laptop
description: Work Laptop Configuration

# Import shared configurations
import:
  - hosts/shared/common.yaml

# Enabled modules
enabled_modules:
  - development/python
  - window-managers/hyprland

# Host-specific packages
packages:
  - vim
  - git
  - flatpak:com.spotify.Client

# Exclude packages from base or modules
exclude:
  - nvidia-drivers

# Settings
flatpak_scope: user              # or "system"
auto_prune: false                # or true
backup_tool: timeshift           # or "snapper"
snapper_config: root             # if using snapper
```

### `modules/base.yaml` (Base Packages)
```yaml
description: Base system packages

packages:
  - base
  - base-devel
  - linux
  - linux-firmware
  - git
  - vim
```

### `modules/declared-packages.yaml` (Manually Installed Packages)
Auto-created by `dcli install` and `dcli search` commands.

```yaml
description: Packages installed via dcli install or dcli search commands

packages:
  - neovim
  - htop
  - tmux
```

### Module File Format (Legacy - Single YAML)
```yaml
description: Module description
packages:
  - package1
  - package2
conflicts:
  - conflicting-module
post_install_hook: scripts/hook.sh
hook_behavior: ask|once|always|skip
```

### Module Format (Directory-Based)
```
modules/{module-name}/
├── module.yaml              # Manifest
├── packages.yaml            # Main packages
├── packages-optional.yaml   # Optional packages
└── scripts/
    └── setup.sh             # Post-install hook
```

---

## Key Concepts

### Pointer Model
- `config.yaml` is minimal (just points to host)
- Actual config lives in `hosts/{hostname}.yaml`
- Allows easy host switching and multi-host management

### Module System
- `modules/base.yaml` - Installed on all systems
- Other modules are opt-in via `enabled_modules`
- Modules can conflict with each other
- Post-install hooks run after package installation

### State Directory
- Git-ignored (not version controlled)
- Tracks installed packages and hook execution
- Hooks tracked by SHA256 hash (re-run if script changes)

### Package Sources
- Pacman: `package-name`
- AUR: `package-name` (auto-detected)
- Flatpak: `flatpak:app.id.Name`

### Backup Integration
- Auto-creates snapshots before major operations
- Supports timeshift or snapper
- Configure with `backup_tool` in host config

---

## Common Workflows

### Initial Setup
```bash
dcli init                    # Create new config
# or
dcli init -b                 # Bootstrap from BlackDon's config

dcli module enable hyprland  # Enable modules
dcli sync                    # Install packages
```

### Daily Usage
```bash
dcli install neovim          # Install and track package
dcli sync                    # Sync configuration
dcli update                  # Update system
dcli status                  # Check current state
```

### Multi-Host Management
```bash
dcli repo init               # Initialize git repo
dcli repo push               # Push changes

# On another machine:
dcli repo clone              # Clone config
dcli sync                    # Apply configuration
```

### Migration from Old Structure
```bash
dcli migrate --dry-run       # Preview migration
dcli migrate                 # Perform migration
```

---

## Tips

- Use `--dry-run` to preview changes before applying
- Enable `auto_prune: true` for automatic cleanup
- Use `dcli validate` before syncing to catch errors
- Create shared configs in `hosts/shared/` for common settings
- Use categories in modules for better organization
- Run `dcli self-update` periodically to get latest features
