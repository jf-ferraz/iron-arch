# Iron User Guide

> **Version**: 1.0.0
> **Last Updated**: 2025-02-12

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Quick Start](#3-quick-start)
4. [Core Concepts](#4-core-concepts)
5. [CLI Reference](#5-cli-reference)
6. [TUI Dashboard](#6-tui-dashboard)
7. [Managing Bundles](#7-managing-bundles)
8. [Managing Profiles](#8-managing-profiles)
9. [Managing Modules](#9-managing-modules)
10. [Safe Updates](#10-safe-updates)
11. [Git Sync](#11-git-sync)
12. [Secrets Management](#12-secrets-management)
13. [Recovery](#13-recovery)
14. [Troubleshooting](#14-troubleshooting)

---

## 1. Introduction

Iron is a declarative configuration management platform for Arch Linux. It allows you to:

- **Manage desktop environments** as switchable "bundles"
- **Organize dotfiles** into reusable "modules" and "profiles"
- **Update safely** with risk assessment and automatic snapshots
- **Sync configurations** across multiple machines using Git
- **Recover quickly** with generated install scripts

### Philosophy

> "Less is More - Turning your Arch into Iron"

Iron believes that Arch Linux should be:
- **Safe**: Updates should never break your system
- **Simple**: Complex operations should feel natural
- **Reproducible**: Your exact setup should work on any machine

---

## 2. Installation

### From AUR (Recommended)

```bash
# Using paru
paru -S iron

# Using yay
yay -S iron
```

### From Source

```bash
git clone https://github.com/laraj/iron.git
cd iron
cargo build --release
sudo cp target/release/iron /usr/local/bin/
```

### Dependencies

Iron requires these system tools:
- `pacman` - Package management
- `git` - Version control
- `git-crypt` (optional) - Secrets encryption
- `timeshift` or `snapper` (optional) - System snapshots

---

## 3. Quick Start

### Initialize Iron

```bash
# Create a new Iron configuration
mkdir -p ~/.config/iron
cd ~/.config/iron
iron init
```

This creates the basic directory structure:
```
~/.config/iron/
├── bundles/           # Desktop environment configs
├── profiles/          # Dotfile collections
├── modules/           # Individual components
├── hosts/             # Machine-specific configs
└── state.json         # Runtime state
```

### Launch TUI

```bash
# Launch the interactive dashboard
iron
```

### First-Time Setup Wizard

On first launch, Iron displays a setup wizard:

1. **Welcome** - Introduction and overview
2. **Host Setup** - Detect or name your machine
3. **Bundle Selection** - Choose your desktop environment
4. **Profile Selection** - Choose your dotfile set
5. **Confirmation** - Review your choices
6. **Complete** - Apply configuration

---

## 4. Core Concepts

### Hierarchy

```
HOST (Your machine)
  └── BUNDLE (Desktop environment)
        └── PROFILE (Dotfile collection)
              └── MODULE (Individual component)
```

### Host

A **Host** represents a physical machine with:
- Hardware specifications (CPU, GPU, monitors)
- Installed bundles
- Active bundle selection

Example: `desktop` (your main workstation), `laptop` (your portable)

### Bundle

A **Bundle** is a complete desktop environment:
- Compositor/WM (Hyprland, Niri, KDE)
- Core packages (bar, launcher, notifications)
- Bundle-specific dotfiles

Only ONE bundle can be **active** at a time. Others are **dormant**.

### Profile

A **Profile** is a collection of modules:
- Groups related configurations
- Can inherit from other profiles
- May be bundle-specific or universal

Examples: `minimal`, `developer`, `gaming`

### Module

A **Module** is the smallest unit:
- Packages to install
- Dotfiles to link
- Pre/post install hooks

Examples: `nvim-ide`, `kitty-dev`, `fish-config`

---

## 5. CLI Reference

### Global Flags

```bash
iron [--json]     # Output as JSON (for scripting)
iron [--quiet]    # Minimal output
iron [--verbose]  # Detailed output
iron [--no-color] # Disable colored output
```

### Core Commands

| Command | Description |
|---------|-------------|
| `iron` | Launch TUI dashboard |
| `iron init` | Initialize Iron in current directory |
| `iron status` | Show system overview |
| `iron doctor` | Run health checks |
| `iron clean` | Clean package cache |
| `iron update` | Safe system update |

### Bundle Commands

```bash
iron bundle list              # List all bundles
iron bundle status [id]       # Show bundle details
iron bundle install <id>      # Install a bundle
iron bundle switch <id>       # Switch to a bundle
iron bundle remove <id>       # Remove a bundle
```

### Profile Commands

```bash
iron profile list             # List all profiles
iron profile show <id>        # Show profile details
iron profile select <id>      # Activate a profile
iron profile create <name>    # Create new profile
```

### Module Commands

```bash
iron module list              # List all modules
iron module show <id>         # Show module details
iron module enable <id>       # Enable a module
iron module disable <id>      # Disable a module
iron module apply             # Apply enabled modules
```

### Host Commands

```bash
iron host list                # List configured hosts
iron host current             # Show current host
iron host catalog             # Detect hardware
iron host snapshot            # Create system snapshot
```

### Sync Commands

```bash
iron sync status              # Show sync status
iron sync push                # Push to remote
iron sync pull                # Pull from remote
```

### Secrets Commands

```bash
iron secrets status           # Show secrets status
iron secrets unlock           # Decrypt secrets
iron secrets lock             # Encrypt secrets
iron secrets link             # Link secrets to locations
```

---

## 6. TUI Dashboard

Launch the TUI with:
```bash
iron
```

### Navigation

| Key | Action |
|-----|--------|
| `Tab` | Cycle through views |
| `↑/↓` | Navigate lists |
| `Enter` | Select item |
| `Esc` | Go back |
| `?` | Show help |
| `q` | Quit |

### Quick Actions

| Key | Action |
|-----|--------|
| `u` | Update system |
| `b` | Bundles view |
| `p` | Profiles view |
| `m` | Modules view |
| `s` | Settings view |
| `r` | Refresh data |

### Dashboard Widgets

- **System Health** - Overall status indicator
- **Active Config** - Current host, bundle, profile
- **Maintenance** - Last update, clean, doctor times
- **Alerts** - Pending updates, warnings

---

## 7. Managing Bundles

### List Bundles

```bash
iron bundle list
```

Output:
```
BUNDLES
  hyprland    Hyprland Desktop       [ACTIVE]
  niri        Niri Compositor        [DORMANT]
  kde         KDE Plasma Desktop     [NOT INSTALLED]
```

### Install a Bundle

```bash
iron bundle install niri
```

This:
1. Installs required packages
2. Sets bundle state to DORMANT
3. Does NOT activate (configs remain unlinked)

### Switch Bundles

```bash
iron bundle switch niri
```

This:
1. Creates a system snapshot
2. Deactivates current bundle (moves to dormant/)
3. Activates new bundle (links dotfiles)
4. Starts required services

### Bundle State Machine

```
NOT_INSTALLED → install() → DORMANT
                              ↓ activate()
DORMANT ←── deactivate() ─── ACTIVE
```

---

## 8. Managing Profiles

### List Profiles

```bash
iron profile list
```

Output:
```
PROFILES
  minimal      Minimal setup          [ACTIVE]
  developer    Full dev environment
  gaming       Gaming optimized
```

### Select Profile

```bash
iron profile select developer
```

This enables all modules defined in the profile.

### Create Profile

```bash
iron profile create my-profile
```

Then edit `profiles/my-profile/profile.toml`:

```toml
id = "my-profile"
name = "My Custom Profile"
description = "Personal configuration"

modules = [
    "nvim-ide",
    "kitty-dev",
    "fish-config",
]

theme = "catppuccin-mocha"
shell = "fish"

# Optional: inherit from another profile
extends = "minimal"
```

---

## 9. Managing Modules

### List Modules

```bash
iron module list
```

### Enable/Disable

```bash
iron module enable nvim-ide
iron module disable vim-minimal
```

### Module Configuration

Create `modules/my-module/module.toml`:

```toml
id = "my-module"
name = "My Module"
kind = "AppConfig"  # AppConfig, Shell, Theme, etc.

packages = ["myapp"]
aur_packages = []

[[dotfiles]]
source = "config/myapp"
target = "~/.config/myapp"
link = true

conflicts = ["other-module"]
depends = ["base-module"]

post_install = "scripts/setup.sh"
```

---

## 10. Safe Updates

### Check for Updates

```bash
iron update --dry-run
```

### Risk Assessment

Iron calculates a risk score based on:
- **Kernel updates** - HIGH risk
- **Systemd updates** - HIGH risk
- **Graphics drivers** - MEDIUM risk
- **Critical packages** - Based on importance
- **Arch News** - Manual intervention items

### Risk Levels

| Level | Action Required |
|-------|-----------------|
| LOW | Auto-approve available |
| MEDIUM | Confirmation required |
| HIGH | Review and confirm |
| CRITICAL | Review Arch News first |

### Update Workflow

```bash
# Preview updates
iron update --dry-run

# Normal update (prompts for approval)
iron update

# Auto-approve LOW risk
iron update --yes

# Force update (skip risk assessment)
iron update --force
```

### Automatic Snapshots

Before any update, Iron creates a snapshot using:
- **Timeshift** (if installed)
- **Snapper** (if installed)

---

## 11. Git Sync

### Setup Remote

```bash
cd ~/.config/iron
git init
git remote add origin git@github.com:you/iron-config.git
```

### Push Changes

```bash
iron sync push
# Or with custom message:
iron sync push -m "Updated kitty config"
```

### Pull Changes

```bash
iron sync pull
```

### Multi-Machine Workflow

1. Configure on Machine A:
   ```bash
   iron bundle switch hyprland
   iron sync push
   ```

2. Apply on Machine B:
   ```bash
   iron sync pull
   iron bundle switch hyprland
   ```

---

## 12. Secrets Management

### Setup git-crypt

```bash
cd ~/.config/iron
git-crypt init
git-crypt add-gpg-user YOUR-GPG-KEY-ID
```

### Add Secrets

Place secrets in `secrets/`:
```
secrets/
├── ssh/
│   ├── id_ed25519
│   └── id_ed25519.pub
├── gpg/
│   └── private-key.asc
└── tokens/
    └── github.token
```

Add to `.gitattributes`:
```
secrets/** filter=git-crypt diff=git-crypt
```

### Unlock on Clone

```bash
git clone git@github.com:you/iron-config.git
cd iron-config
iron secrets unlock
iron secrets link
```

---

## 13. Recovery

### Generate Install Script

```bash
iron recover --generate-script > install.sh
```

This creates a script that:
1. Partitions disks
2. Installs base system
3. Configures bootloader
4. Installs drivers

### Recovery Workflow

1. Boot Arch ISO
2. Clone your Iron repo
3. Run install script
4. Chroot and apply Iron config:
   ```bash
   iron bundle install hyprland
   iron bundle switch hyprland
   iron secrets unlock
   iron secrets link
   ```

### Verify Installation

```bash
iron doctor
```

Checks:
- Required packages installed
- Services running
- Permissions correct
- Dotfiles linked

---

## 14. Troubleshooting

### Common Issues

#### Bundle switch fails

```bash
# Check for conflicts
iron bundle status hyprland

# Force deactivate current
iron bundle deactivate --force

# Try switch again
iron bundle switch hyprland
```

#### Module conflicts

```bash
# Check conflicts
iron module show nvim-ide

# Disable conflicting module first
iron module disable vim-minimal
iron module enable nvim-ide
```

#### Sync conflicts

```bash
# Check status
iron sync status

# Resolve manually
git status
git diff
# Edit conflicts
git add .
iron sync push
```

### Logs

```bash
# View operation log
cat ~/.config/iron/.iron/state/operations.jsonl

# Enable verbose mode
iron --verbose status
```

### Reset State

```bash
# Backup current state
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak

# Reset to clean state
iron init --force
```

---

## Appendix: Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `IRON_ROOT` | Configuration directory | `~/.config/iron` |
| `IRON_LOG` | Log level | `info` |
| `IRON_NO_COLOR` | Disable colors | `false` |
| `IRON_NO_SNAPSHOT` | Skip snapshots | `false` |

---

**Need help?** Open an issue at https://github.com/laraj/iron/issues
