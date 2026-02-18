# Iron Walkthrough Guide

> **Version**: 0.1.0
> **Last Updated**: 2025-02-12

A hands-on, step-by-step guide for building, testing, and using all features of Iron.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Prerequisites](#2-prerequisites)
3. [Quick Start](#3-quick-start)
4. [Project Structure Overview](#4-project-structure-overview)
5. [Running Tests](#5-running-tests)
6. [CLI Command Walkthrough](#6-cli-command-walkthrough)
7. [TUI Dashboard Walkthrough](#7-tui-dashboard-walkthrough)
8. [Configuration Examples](#8-configuration-examples)
9. [Development Workflow](#9-development-workflow)
10. [Troubleshooting](#10-troubleshooting)
11. [Next Steps](#11-next-steps)

---

## 1. Introduction

### What This Guide Covers

This walkthrough provides hands-on instructions for:

- Building Iron from source
- Running the complete test suite
- Using all 11 CLI command groups
- Navigating the TUI dashboard
- Creating configuration files (bundles, profiles, modules, hosts)
- Developer workflows for contributing

### Target Audience

- **New users** wanting to understand Iron's features
- **Developers** contributing to the project
- **System administrators** evaluating Iron for their Arch setup

### Time Estimate

- **Quick Start**: 10 minutes
- **Full Walkthrough**: 30-60 minutes

---

## 2. Prerequisites

### Required Software

| Tool | Minimum Version | Check Command |
|------|-----------------|---------------|
| Rust | 1.75+ (Edition 2024) | `rustc --version` |
| Cargo | Latest | `cargo --version` |
| pacman | Any | `pacman --version` |
| git | 2.30+ | `git --version` |

### Installation (Arch Linux)

```bash
# Install Rust via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
# Expected: rustc 1.75.0 or higher

cargo --version
# Expected: cargo 1.75.0 or higher
```

### Optional Dependencies

| Tool | Purpose | Install Command |
|------|---------|-----------------|
| git-crypt | Secrets encryption | `sudo pacman -S git-crypt` |
| timeshift | System snapshots | `sudo pacman -S timeshift` |
| cargo-tarpaulin | Code coverage | `cargo install cargo-tarpaulin` |
| cargo-watch | Auto-rebuild on changes | `cargo install cargo-watch` |

```bash
# Install all optional dependencies
sudo pacman -S git-crypt timeshift
cargo install cargo-tarpaulin cargo-watch
```

---

## 3. Quick Start

### Clone the Repository

```bash
git clone https://github.com/laraj/iron.git
cd iron
```

### Build (Debug Mode)

```bash
cargo build --workspace
```

Expected output:
```
   Compiling iron-core v0.1.0
   Compiling iron-fs v0.1.0
   Compiling iron-pacman v0.1.0
   ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.32s
```

### Build (Release Mode)

```bash
cargo build --release --workspace
```

### Run Your First Command

```bash
# Show help
./target/debug/iron --help
```

Expected output:
```
Iron - Declarative Arch Linux Configuration Management

Usage: iron [OPTIONS] [COMMAND]

Options:
  -r, --root <ROOT>      Iron root directory [default: ~/.config/iron]
  -f, --format <FORMAT>  Output format [default: text] [possible values: text, json, minimal]
  -v, --verbose          Verbose output (show details)
  -q, --quiet            Quiet output (minimal)
      --no-color         No color output
  -h, --help             Print help
  -V, --version          Print version

Commands:
  init        Initialize Iron on this host
  status      Show system status overview
  update      Safe system update with risk assessment
  bundle      Bundle management (desktop environments)
  profile     Profile management (configuration presets)
  module      Module management (config modules)
  host        Host management
  sync        Git sync operations
  secrets     Secrets management (git-crypt)
  doctor      System health check
  clean       System cleanup
  recover     Recovery workflow
  go          Launch TUI dashboard
  completions Generate shell completions
  help        Print this message or the help of the given subcommand(s)
```

### Verify Installation

```bash
# Check version
./target/debug/iron --version
# Expected: iron 0.1.0

# Run doctor check
./target/debug/iron doctor
```

---

## 4. Project Structure Overview

### Crate Architecture

Iron consists of 7 Rust crates:

| Crate | Purpose | Location |
|-------|---------|----------|
| `iron-core` | Domain models and business logic | `crates/iron-core/` |
| `iron-cli` | Command-line interface | `crates/iron-cli/` |
| `iron-tui` | Terminal UI dashboard | `crates/iron-tui/` |
| `iron-fs` | File system operations | `crates/iron-fs/` |
| `iron-pacman` | Pacman/AUR integration | `crates/iron-pacman/` |
| `iron-git` | Git operations | `crates/iron-git/` |
| `iron-systemd` | Systemd service management | `crates/iron-systemd/` |

### Directory Layout

```
iron/
├── crates/
│   ├── iron-core/       # Domain models, business logic
│   ├── iron-cli/        # CLI commands (11 groups)
│   ├── iron-tui/        # TUI dashboard (ratatui)
│   ├── iron-fs/         # File operations
│   ├── iron-pacman/     # Package management
│   ├── iron-git/        # Git operations
│   └── iron-systemd/    # Systemd integration
├── bundles/             # Desktop environment configs
│   ├── hyprland/
│   └── niri/
├── profiles/            # Configuration presets
│   ├── developer/
│   └── minimal/
├── modules/             # Individual components
│   ├── nvim-ide/
│   └── kitty-dev/
├── hosts/               # Machine-specific configs
│   └── desktop/
├── scripts/             # Helper scripts
│   └── lib/
└── docs/                # Documentation
    ├── guide/
    ├── architecture/
    └── requirements/
```

### Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace configuration |
| `crates/iron-cli/src/cli.rs` | CLI command definitions |
| `crates/iron-tui/src/app/handlers.rs` | TUI keyboard handlers |
| `bundles/*/bundle.toml` | Bundle configuration |
| `profiles/*/profile.toml` | Profile configuration |
| `modules/*/module.toml` | Module configuration |
| `hosts/*/host.toml` | Host configuration |

---

## 5. Running Tests

### Run All Tests

```bash
cargo test --workspace
```

Expected output:
```
   Compiling iron-core v0.1.0
   ...
running 54 tests
test result: ok. 54 passed; 0 failed; 0 ignored

running 62 tests
test result: ok. 62 passed; 0 failed; 0 ignored

...

test result: ok. 165 passed; 0 failed; 0 ignored
```

### Per-Crate Test Counts

| Crate | Tests | Command |
|-------|-------|---------|
| iron-core | 54 | `cargo test -p iron-core` |
| iron-cli | 62 | `cargo test -p iron-cli` |
| iron-tui | 22 | `cargo test -p iron-tui` |
| iron-fs | 12 | `cargo test -p iron-fs` |
| iron-pacman | 9 | `cargo test -p iron-pacman` |
| iron-git | 3 | `cargo test -p iron-git` |
| iron-systemd | 3 | `cargo test -p iron-systemd` |
| **Total** | **165** | `cargo test --workspace` |

### Run Specific Test

```bash
# Run tests matching a pattern
cargo test -p iron-core bundle

# Run a specific test by name
cargo test -p iron-cli test_init_command
```

### Run Tests with Output

```bash
# Show stdout for passing tests
cargo test --workspace -- --nocapture
```

### Code Coverage

```bash
# Install tarpaulin if not installed
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html

# View report
xdg-open tarpaulin-report.html
```

Expected coverage: 80%+

### Integration Tests

```bash
# Run integration tests only
cargo test -p iron-cli --test cli_integration
```

---

## 6. CLI Command Walkthrough

Iron provides 11 command groups. Here's a complete walkthrough with copy-pasteable commands.

### Global Flags

```bash
# JSON output (for scripting)
iron --format json status

# Minimal output (IDs only)
iron --format minimal bundle list

# Verbose output
iron --verbose doctor

# Quiet mode
iron --quiet update

# No color
iron --no-color status

# Custom root directory
iron --root /path/to/config status
```

### 6.1 `iron init` - Initialize Configuration

```bash
# Initialize with hostname auto-detection
iron init

# Initialize with custom host ID
iron init --id my-desktop

# Initialize with custom name
iron init --id laptop --name "Work Laptop"

# Force re-initialization
iron init --force
```

Expected output:
```
Initializing Iron configuration...
  Host ID: desktop
  Name: Desktop Workstation
  Directory: ~/.config/iron

Created directories:
  ~/.config/iron/bundles/
  ~/.config/iron/profiles/
  ~/.config/iron/modules/
  ~/.config/iron/hosts/

Iron initialized successfully!
```

### 6.2 `iron status` - System Overview

```bash
iron status
```

Expected output:
```
IRON STATUS

Host:     desktop (Desktop Workstation)
Bundle:   hyprland [ACTIVE]
Profile:  developer

System:
  Packages:   1,842 installed
  Updates:    12 pending
  Last Sync:  2 hours ago
  Health:     GOOD
```

### 6.3 `iron doctor` - Health Checks

```bash
iron doctor
```

Expected output:
```
HEALTH CHECK

[OK] Required packages installed
[OK] Services running
[OK] Dotfiles linked correctly
[OK] Permissions correct
[WARN] 3 orphan packages found

Overall: GOOD (1 warning)

Run 'iron clean --orphans' to remove orphan packages.
```

### 6.4 `iron clean` - Cleanup Operations

```bash
# Remove orphaned packages
iron clean --orphans

# Clear package cache
iron clean --cache

# Remove broken symlinks
iron clean --symlinks

# All cleanup operations
iron clean --all
```

Expected output (all):
```
CLEANUP

Orphan packages:
  Removing: libfoo libbar libbaz
  Removed 3 packages

Package cache:
  Clearing old versions...
  Freed 2.3 GB

Broken symlinks:
  Removing: ~/.config/oldapp
  Removed 1 symlink

Cleanup complete!
```

### 6.5 `iron bundle` - Bundle Management

```bash
# List all bundles
iron bundle list

# List with full details
iron bundle list --all

# Show bundle status
iron bundle status hyprland

# Install a bundle
iron bundle install niri

# Skip confirmation
iron bundle install niri --yes

# Switch to a different bundle
iron bundle switch niri

# Remove a bundle
iron bundle remove kde
```

Expected output (list):
```
BUNDLES

  ID         NAME                    STATUS
  hyprland   Hyprland Desktop        [ACTIVE]
  niri       Niri Compositor         [DORMANT]
  kde        KDE Plasma              [NOT INSTALLED]
```

Expected output (install):
```
Installing bundle: niri

Packages to install:
  niri, waybar, fuzzel, swww, swaylock, mako

AUR packages:
  niri-git

Proceed? [y/N] y

Installing packages...
  [1/6] niri
  [2/6] waybar
  ...

Bundle 'niri' installed successfully.
State: DORMANT (use 'iron bundle switch niri' to activate)
```

### 6.6 `iron profile` - Profile Management

```bash
# List all profiles
iron profile list

# Filter by bundle
iron profile list --bundle hyprland

# Show profile details
iron profile show developer

# Show with inherited modules
iron profile show developer --effective

# Select/activate a profile
iron profile select developer

# Create a new profile
iron profile create my-profile

# Create extending another profile
iron profile create gaming --extends developer

# Edit profile in $EDITOR
iron profile edit my-profile
```

Expected output (show):
```
PROFILE: developer

Name:        Developer
Description: Full development environment with IDE-like terminal experience

Modules (7):
  nvim-ide         Neovim IDE              [ENABLED]
  kitty-dev        Kitty Terminal          [ENABLED]
  waybar-dev       Waybar Config           [ENABLED]
  dev-tools        Development Tools       [ENABLED]
  git-config       Git Configuration       [ENABLED]
  tmux-config      Tmux Configuration      [ENABLED]
  starship-prompt  Starship Prompt         [ENABLED]

Theme: catppuccin-mocha
Shell: fish
```

### 6.7 `iron module` - Module Management

```bash
# List all modules
iron module list

# List enabled only
iron module list --enabled

# List disabled only
iron module list --disabled

# Filter by kind
iron module list --kind AppConfig

# Show module details
iron module show nvim-ide

# Enable a module
iron module enable nvim-ide

# Force enable (skip conflict check)
iron module enable nvim-ide --force

# Disable a module
iron module disable vim-minimal

# Skip confirmation
iron module disable vim-minimal --yes
```

Expected output (list):
```
MODULES

  ID              NAME              KIND        STATUS
  nvim-ide        Neovim IDE        AppConfig   [ENABLED]
  kitty-dev       Kitty Terminal    AppConfig   [ENABLED]
  vim-minimal     Vim Minimal       AppConfig   [DISABLED]
  fish-config     Fish Shell        Shell       [ENABLED]
  starship-prompt Starship          Shell       [ENABLED]
```

Expected output (enable):
```
Enabling module: nvim-ide

Packages to install:
  neovim, ripgrep, fd, lazygit, nodejs, npm

Dotfiles to link:
  config/nvim -> ~/.config/nvim

Conflicts: vim-minimal (currently disabled)

Proceed? [y/N] y

Installing packages...
Linking dotfiles...
Running post_install hook...

Module 'nvim-ide' enabled successfully.
```

### 6.8 `iron host` - Host Management

```bash
# List configured hosts
iron host list

# Show current host info
iron host current

# Catalog current hardware
iron host catalog

# Update existing host config
iron host catalog --update

# Select active host
iron host select laptop

# Create system snapshot
iron host snapshot

# With description
iron host snapshot --description "Before major update"
```

Expected output (current):
```
CURRENT HOST: desktop

Name:        Desktop Workstation
Description: Primary development machine

Hardware:
  CPU:  AMD Ryzen 9 7950X
  GPU:  NVIDIA RTX 4080
  RAM:  64 GB

Monitors:
  DP-1:   2560x1440 @ 165Hz (scale 1.0)
  HDMI-1: 1920x1080 @ 60Hz (scale 1.0)

Bundles:
  Active:    hyprland
  Installed: hyprland, niri
```

Expected output (snapshot):
```
Creating system snapshot...

Snapshot created:
  ID:          2025-02-12_14-30-45
  Description: Before major update
  Tool:        timeshift
  Size:        4.2 GB

Snapshot stored successfully.
```

### 6.9 `iron update` - Safe System Updates

```bash
# Preview updates (dry run)
iron update --dry-run

# Normal update (with confirmation)
iron update

# Skip risk assessment
iron update --force

# Skip snapshot creation
iron update --no-snapshot
```

Expected output (dry-run):
```
UPDATE PREVIEW

Pending updates: 12 packages

HIGH RISK:
  linux               6.17.1-1 -> 6.18.9-1
  nvidia-dkms         560.35.03-1 -> 565.57.01-1

MEDIUM RISK:
  systemd             256.4-1 -> 256.5-1

LOW RISK:
  neovim              0.10.0-1 -> 0.10.2-1
  ripgrep             14.1.0-1 -> 14.1.1-1
  ... (7 more)

Risk Assessment: HIGH
  Kernel update detected
  NVIDIA driver update detected

Recommendation: Create snapshot before proceeding.

Use 'iron update' to apply (will prompt for confirmation).
```

Expected output (update):
```
SYSTEM UPDATE

Risk Level: HIGH

Updates:
  linux               6.17.1-1 -> 6.18.9-1
  nvidia-dkms         560.35.03-1 -> 565.57.01-1
  ...

Creating snapshot before update...
  Snapshot: 2025-02-12_14-35-00

Proceed with update? [y/N] y

Updating packages...
  [1/12] linux
  [2/12] nvidia-dkms
  ...

Update complete!
  Updated: 12 packages
  Snapshot: 2025-02-12_14-35-00 (rollback available)
```

### 6.10 `iron sync` - Git Synchronization

```bash
# Show sync status
iron sync status

# Push local changes
iron sync push

# Push with custom message
iron sync push --message "Updated nvim config"

# Pull changes from remote
iron sync pull

# Pull with stash
iron sync pull --stash
```

Expected output (status):
```
SYNC STATUS

Remote:  origin (git@github.com:laraj/iron-config.git)
Branch:  main

Local changes:
  Modified: modules/nvim-ide/config/nvim/init.lua
  Added:    modules/kitty-dev/config/kitty/theme.conf

Status: 2 uncommitted changes
Last push: 2 hours ago
Last pull: 1 day ago
```

Expected output (push):
```
Pushing changes...

Committing:
  modules/nvim-ide/config/nvim/init.lua
  modules/kitty-dev/config/kitty/theme.conf

Commit: "Updated nvim config"
Pushing to origin/main...

Sync complete!
```

### 6.11 `iron secrets` - Secrets Management

```bash
# Show secrets status
iron secrets status

# Unlock encrypted secrets
iron secrets unlock

# Unlock with specific key
iron secrets unlock --key ~/.gnupg/mykey.gpg

# Lock secrets before push
iron secrets lock

# Link secrets to proper locations
iron secrets link
```

Expected output (status):
```
SECRETS STATUS

Encryption: git-crypt
Status:     LOCKED

Secret files:
  secrets/ssh/id_ed25519          [ENCRYPTED]
  secrets/gpg/private-key.asc     [ENCRYPTED]
  secrets/tokens/github.token     [ENCRYPTED]

Use 'iron secrets unlock' to decrypt.
```

Expected output (unlock):
```
Unlocking secrets...

GPG key: laraj@example.com
Decrypting...

Secrets unlocked:
  secrets/ssh/id_ed25519
  secrets/gpg/private-key.asc
  secrets/tokens/github.token

Use 'iron secrets link' to symlink to proper locations.
```

### 6.12 `iron recover` - Recovery Workflow

```bash
# Export current state
iron recover --export

# Import from file
iron recover --import state-backup.json

# Generate install script
iron recover --script
```

Expected output (script):
```
Generating install script...

Script written to: install-desktop.sh

This script will:
  1. Partition /dev/nvme0n1
  2. Format as btrfs (encrypted)
  3. Install base system
  4. Configure systemd-boot
  5. Install AMD microcode
  6. Install NVIDIA drivers

Review the script before running!
```

---

## 7. TUI Dashboard Walkthrough

### Launching the TUI

```bash
# Launch dashboard (no arguments)
iron

# Or explicitly
iron go
```

### Views

| View | Access Key | Description |
|------|------------|-------------|
| Dashboard | `d` | System overview, health, alerts |
| Bundles | `b` | List and manage bundles |
| Profiles | `p` | List and manage profiles |
| Modules | `m` | List and manage modules |
| Update | `u` | Preview and apply updates |
| Settings | `s` | Configuration options |

### Navigation Keys

| Key | Action |
|-----|--------|
| `Tab` | Cycle to next view |
| `Shift+Tab` | Cycle to previous view |
| `d` | Go to Dashboard |
| `b` | Go to Bundles |
| `p` | Go to Profiles |
| `m` | Go to Modules |
| `u` | Go to Update Preview |
| `s` | Go to Settings |

### List Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous item |
| `↓` / `j` | Select next item |
| `Enter` | View details / activate |
| `Home` | Jump to first item |
| `End` | Jump to last item |
| `Esc` | Go back |

### Actions

| Key | Action |
|-----|--------|
| `a` | Activate selected bundle/profile |
| `e` | Enable/disable selected module |
| `r` | Refresh current view |
| `?` | Show help overlay |
| `q` | Quit |
| `Ctrl+c` | Force quit |

### Setup Wizard

On first launch, the TUI displays a setup wizard:

**Step 1: Welcome**
- Press `Enter` to continue
- Press `q` to quit

**Step 2: Host Setup**
- Press `e` to edit host ID
- Type your host identifier
- Press `Enter` to confirm
- Press `Enter` to continue

**Step 3: Bundle Selection**
- Use `↑`/`↓` or `j`/`k` to navigate
- Press `Enter` to select and continue

**Step 4: Profile Selection**
- Use `↑`/`↓` or `j`/`k` to navigate
- Press `Enter` to select and continue

**Step 5: Confirmation**
- Review your selections
- Press `Enter` or `y` to apply
- Press `Esc` to go back

**Step 6: Complete**
- Press `Enter` to enter dashboard

### Update View

In the Update Preview view:

| Key | Action |
|-----|--------|
| `r` | Refresh update list |
| `u` / `Enter` | Start update (with confirmation) |
| `Esc` | Return to dashboard |

### Profile Detail View

| Key | Action |
|-----|--------|
| `a` / `Enter` | Activate this profile |
| `Esc` | Return to profile list |

---

## 8. Configuration Examples

### Bundle Configuration (`bundle.toml`)

Location: `bundles/<bundle-id>/bundle.toml`

```toml
# Hyprland Bundle - Wayland Compositor
# A complete desktop environment based on Hyprland

id = "hyprland"
name = "Hyprland Desktop"
description = "Dynamic tiling Wayland compositor with stunning animations"
bundle_type = "WaylandCompositor"

# Core packages for this bundle
packages = [
    "hyprland",
    "waybar",
    "wofi",
    "hyprpaper",
    "hypridle",
    "hyprlock",
    "xdg-desktop-portal-hyprland",
    "wl-clipboard",
    "cliphist",
    "grim",
    "slurp",
    "mako",
]

# AUR packages
aur_packages = [
    "hyprshot",
]

# Available profiles for this bundle
profiles = ["minimal", "developer", "gaming", "streamer"]
default_profile = "minimal"

# Conflicts with other bundles
conflicts = ["niri", "sway", "kde"]

# Services to enable
services = ["pipewire", "pipewire-pulse", "wireplumber"]

# Post-install hook
post_install = "scripts/setup-hyprland.sh"
```

### Profile Configuration (`profile.toml`)

Location: `profiles/<profile-id>/profile.toml`

```toml
# Developer Profile
# Optimized for software development workflows

id = "developer"
name = "Developer"
description = "Full development environment with IDE-like terminal experience"

# Modules included in this profile
modules = [
    "nvim-ide",
    "kitty-dev",
    "waybar-dev",
    "dev-tools",
    "git-config",
    "tmux-config",
    "starship-prompt",
]

# Theme
theme = "catppuccin-mocha"

# Shell preference
shell = "fish"

# Works with any bundle (empty = universal)
for_bundle = ""
```

### Module Configuration (`module.toml`)

Location: `modules/<module-id>/module.toml`

```toml
# Neovim IDE Module
# Full IDE experience in the terminal

id = "nvim-ide"
name = "Neovim IDE"
description = "Neovim configured as a full IDE with LSP, completion, and debugging"
kind = "AppConfig"

# Packages
packages = [
    "neovim",
    "ripgrep",
    "fd",
    "lazygit",
    "nodejs",
    "npm",
]

aur_packages = []

# Dotfiles to link
[[dotfiles]]
source = "config/nvim"
target = "~/.config/nvim"
link = true

# Conflicts
conflicts = ["vim-minimal"]

# Dependencies
depends = []

# Hooks
post_install = "scripts/setup-nvim.sh"
```

### Host Configuration (`host.toml`)

Location: `hosts/<host-id>/host.toml`

```toml
# Desktop Host Configuration
# Main development workstation

id = "desktop"
name = "Desktop Workstation"
description = "Primary development machine"

[hardware]
cpu = "AMD Ryzen 9 7950X"
gpu = "NVIDIA RTX 4080"
ram_mb = 65536
chassis = "Desktop"

[[hardware.monitors]]
output = "DP-1"
resolution = "2560x1440"
refresh_rate = 165
scale = 1.0

[[hardware.monitors]]
output = "HDMI-1"
resolution = "1920x1080"
refresh_rate = 60
scale = 1.0

# Installed bundles
installed_bundles = ["hyprland", "niri"]
active_bundle = "hyprland"

# Installation parameters (for recovery)
[install_params]
bootloader = "SystemdBoot"
kernel = "linux"
microcode = "amd-ucode"
gpu_drivers = ["nvidia", "nvidia-utils", "nvidia-settings"]
filesystem = "btrfs"
encrypted = true

[[install_params.partitions]]
device = "/dev/nvme0n1p1"
mount_point = "/boot"
filesystem = "vfat"
size = "512M"

[[install_params.partitions]]
device = "/dev/nvme0n1p2"
mount_point = "/"
filesystem = "btrfs"
size = "remaining"
```

---

## 9. Development Workflow

### Setting Up Development Environment

```bash
# Clone repository
git clone https://github.com/laraj/iron.git
cd iron

# Build in debug mode
cargo build --workspace

# Run tests
cargo test --workspace
```

### Making Changes

```bash
# Create a new branch
git checkout -b feature/my-feature

# Make your changes
$EDITOR crates/iron-core/src/lib.rs

# Format code
cargo fmt --all

# Run lints
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

### Using cargo-watch (Auto-Rebuild)

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-run tests on changes
cargo watch -x "test --workspace"

# Auto-build on changes
cargo watch -x "build --workspace"

# Auto-run clippy on changes
cargo watch -x "clippy --workspace -- -D warnings"
```

### Pre-Commit Checklist

```bash
# Format
cargo fmt --all

# Lint (must pass with no warnings)
cargo clippy --workspace -- -D warnings

# Test (all must pass)
cargo test --workspace

# Build release (verify it compiles)
cargo build --release --workspace
```

### Running a Single Crate

```bash
# Test single crate
cargo test -p iron-core

# Build single crate
cargo build -p iron-cli

# Run with arguments
cargo run -p iron-cli -- status
cargo run -p iron-cli -- bundle list
```

---

## 10. Troubleshooting

### Common Build Errors

#### Missing Rust toolchain

```
error: could not find `rustc` in PATH
```

**Solution:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Outdated Rust version

```
error: package `iron-core v0.1.0` cannot be built because it requires rust 1.75 or newer
```

**Solution:**
```bash
rustup update stable
```

#### Missing system libraries

```
error: linking with `cc` failed
```

**Solution:**
```bash
sudo pacman -S base-devel
```

### Common Runtime Errors

#### Permission denied

```
Error: Permission denied (os error 13)
```

**Solution:**
```bash
# Check file permissions
ls -la ~/.config/iron/

# Fix ownership
sudo chown -R $USER:$USER ~/.config/iron/
```

#### Config file parse error

```
Error: Failed to parse bundle.toml: invalid TOML
```

**Solution:**
```bash
# Validate TOML syntax
cargo install toml-validator
toml-validator bundles/hyprland/bundle.toml
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `IRON_ROOT` | Configuration directory | `~/.config/iron` |
| `IRON_LOG` | Log level (`debug`, `info`, `warn`, `error`) | `info` |
| `IRON_NO_COLOR` | Disable colored output | `false` |
| `IRON_NO_SNAPSHOT` | Skip automatic snapshots | `false` |
| `RUST_BACKTRACE` | Show backtrace on panic | `0` |

```bash
# Enable debug logging
IRON_LOG=debug iron status

# Show full backtrace on errors
RUST_BACKTRACE=1 iron bundle install hyprland
```

### Debugging Tips

```bash
# Verbose mode for detailed output
iron --verbose status

# Check current state
cat ~/.config/iron/state.json | jq

# View operation log
cat ~/.config/iron/.iron/state/operations.jsonl

# Reset state (backup first!)
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak
iron init --force
```

---

## 11. Next Steps

### Further Reading

| Document | Description |
|----------|-------------|
| [USER-GUIDE.md](USER-GUIDE.md) | Complete user documentation |
| [ARCHITECTURE.md](../architecture/ARCHITECTURE.md) | System architecture |
| [API.md](../architecture/API.md) | Internal API documentation |
| [CONFIG.md](../architecture/CONFIG.md) | Configuration reference |
| [CONTRIBUTING.md](../dev/CONTRIBUTING.md) | Contribution guidelines |

### Getting Help

- **Issues**: [https://github.com/laraj/iron/issues](https://github.com/laraj/iron/issues)
- **Discussions**: [https://github.com/laraj/iron/discussions](https://github.com/laraj/iron/discussions)

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and lints
5. Submit a pull request

See [CONTRIBUTING.md](../dev/CONTRIBUTING.md) for detailed guidelines.

---

*Last updated: 2025-02-12 | Iron v0.1.0 | 165 tests passing*
