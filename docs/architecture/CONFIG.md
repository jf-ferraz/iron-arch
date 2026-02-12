# Iron Configuration Reference

> **Version**: 1.0.0
> **Last Updated**: 2025-02-12

---

## Table of Contents

1. [File Formats](#1-file-formats)
2. [Host Configuration](#2-host-configuration)
3. [Bundle Configuration](#3-bundle-configuration)
4. [Profile Configuration](#4-profile-configuration)
5. [Module Configuration](#5-module-configuration)
6. [State Files](#6-state-files)

---

## 1. File Formats

All Iron configuration files use **TOML** format.

### File Locations

| File Type | Location | Git-tracked |
|-----------|----------|-------------|
| Host | `hosts/<id>/host.toml` | Yes |
| Bundle | `bundles/<id>/bundle.toml` | Yes |
| Profile | `profiles/<id>/profile.toml` | Yes |
| Module | `modules/<id>/module.toml` | Yes |
| State | `.iron/state/*.json` | No |

---

## 2. Host Configuration

**Location**: `hosts/<hostname>/host.toml`

### Schema

```toml
# Required fields
id = "desktop"              # Unique identifier (alphanumeric + hyphen)
name = "Desktop Workstation" # Human-readable name

# Optional fields
description = "Primary development machine"

# Hardware specification
[hardware]
cpu = "AMD Ryzen 9 7950X"   # CPU model
gpu = "NVIDIA RTX 4080"     # GPU model
ram_mb = 65536              # RAM in megabytes
chassis = "Desktop"         # Desktop | Laptop | Server | Tablet | Convertible | Unknown

# Monitor configurations (array)
[[hardware.monitors]]
output = "DP-1"             # Output name (from compositor)
resolution = "2560x1440"    # Resolution string
refresh_rate = 165          # Hz (optional)
scale = 1.0                 # Scale factor (optional)

[[hardware.monitors]]
output = "HDMI-1"
resolution = "1920x1080"
refresh_rate = 60

# Bundle configuration
installed_bundles = ["hyprland", "niri"]  # Installed bundle IDs
active_bundle = "hyprland"                 # Currently active bundle ID

# Installation parameters for recovery
[install_params]
bootloader = "SystemdBoot"  # SystemdBoot | Grub | RefindBoot
kernel = "linux"            # Kernel package name
microcode = "amd-ucode"     # Microcode package (optional)
gpu_drivers = ["nvidia", "nvidia-utils"]
filesystem = "btrfs"        # Root filesystem
encrypted = true            # LUKS encryption

# Partition configuration
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

### Chassis Types

| Value | Description |
|-------|-------------|
| `Desktop` | Desktop computer |
| `Laptop` | Laptop/notebook |
| `Server` | Server machine |
| `Tablet` | Tablet device |
| `Convertible` | 2-in-1 convertible |
| `Unknown` | Unknown chassis |

### Bootloader Types

| Value | Description |
|-------|-------------|
| `SystemdBoot` | systemd-boot (recommended for UEFI) |
| `Grub` | GRUB bootloader |
| `RefindBoot` | rEFInd boot manager |

---

## 3. Bundle Configuration

**Location**: `bundles/<id>/bundle.toml`

### Schema

```toml
# Required fields
id = "hyprland"                    # Unique identifier
name = "Hyprland Desktop"          # Human-readable name
bundle_type = "WaylandCompositor"  # Bundle type (see below)

# Optional fields
description = "Dynamic tiling Wayland compositor"

# Package lists
packages = [                       # Official repo packages
    "hyprland",
    "waybar",
    "wofi",
    "hyprpaper",
    "hypridle",
    "hyprlock",
]

aur_packages = [                   # AUR packages
    "hyprshot",
]

# Profile configuration
profiles = ["minimal", "developer", "gaming"]  # Available profiles
default_profile = "minimal"                     # Default profile ID

# Conflict management
conflicts = ["niri", "sway", "kde"]  # Conflicting bundle IDs

# Service management
services = [                         # Systemd services to enable
    "pipewire",
    "pipewire-pulse",
    "wireplumber",
]

# Hooks
post_install = "scripts/setup.sh"    # Post-install script (relative path)
```

### Bundle Types

| Value | Description |
|-------|-------------|
| `WaylandCompositor` | Wayland compositor (Hyprland, Niri, Sway) |
| `DesktopEnvironment` | Full DE (KDE, GNOME, XFCE) |
| `X11WindowManager` | X11 window manager (i3, bspwm) |

### Bundle Directory Structure

```
bundles/hyprland/
├── bundle.toml              # Bundle manifest
├── dotfiles/                # Bundle-specific dotfiles
│   ├── hypr/               # -> ~/.config/hypr/
│   ├── waybar/             # -> ~/.config/waybar/
│   └── wofi/               # -> ~/.config/wofi/
└── scripts/
    └── setup.sh            # Post-install script
```

---

## 4. Profile Configuration

**Location**: `profiles/<id>/profile.toml`

### Schema

```toml
# Required fields
id = "developer"            # Unique identifier
name = "Developer"          # Human-readable name

# Optional fields
description = "Full development environment"

# Module configuration
modules = [                 # Module IDs to include
    "nvim-ide",
    "kitty-dev",
    "waybar-dev",
    "dev-tools",
    "git-config",
    "tmux-config",
    "starship-prompt",
]

# Theme configuration
theme = "catppuccin-mocha"  # Theme identifier

# Shell preference
shell = "fish"              # bash | zsh | fish

# Inheritance
extends = "minimal"         # Parent profile to inherit from

# Bundle compatibility
for_bundle = ""             # Bundle ID (empty = works with any)
```

### Profile Inheritance

When `extends` is specified, the profile inherits:
- All modules from parent (child modules are added)
- Theme (child overrides if specified)
- Shell (child overrides if specified)

Example:
```toml
# profiles/developer/profile.toml
id = "developer"
extends = "minimal"         # Inherits minimal's modules
modules = [                 # These are ADDED to minimal's modules
    "nvim-ide",
    "dev-tools",
]
```

---

## 5. Module Configuration

**Location**: `modules/<id>/module.toml`

### Schema

```toml
# Required fields
id = "nvim-ide"             # Unique identifier
name = "Neovim IDE"         # Human-readable name
kind = "AppConfig"          # Module kind (see below)

# Optional fields
description = "Neovim configured as a full IDE"

# Package lists
packages = [                # Official repo packages
    "neovim",
    "ripgrep",
    "fd",
    "lazygit",
]

aur_packages = []           # AUR packages

# Dotfile mappings
[[dotfiles]]
source = "config/nvim"      # Relative to module directory
target = "~/.config/nvim"   # Target path (~ expanded)
link = true                 # true = symlink, false = copy

# Conflict management
conflicts = ["vim-minimal"] # Conflicting module IDs

# Dependencies
depends = ["git-config"]    # Required module IDs

# Hooks
pre_install = "scripts/pre.sh"   # Pre-install script
post_install = "scripts/post.sh" # Post-install script
```

### Module Kinds

| Value | Description |
|-------|-------------|
| `AppConfig` | Application configuration (nvim, kitty) |
| `Shell` | Shell configuration (bash, zsh, fish) |
| `DesktopComponent` | Desktop component (waybar, rofi) |
| `Theme` | Theme assets (icons, cursors, GTK) |
| `SystemUtil` | System utilities |
| `DevTools` | Development tools |

### Module Directory Structure

```
modules/nvim-ide/
├── module.toml              # Module manifest
├── config/                  # Dotfiles
│   └── nvim/               # -> ~/.config/nvim/
│       ├── init.lua
│       └── lua/
└── scripts/
    └── post.sh             # Post-install script
```

### Dotfile Mapping Examples

```toml
# Link config directory
[[dotfiles]]
source = "config/nvim"
target = "~/.config/nvim"
link = true

# Link single file to home directory
[[dotfiles]]
source = "zshrc"
target = "~/.zshrc"
link = true

# Copy instead of link (for templates)
[[dotfiles]]
source = "templates/gitconfig"
target = "~/.gitconfig"
link = false
```

---

## 6. State Files

State files are stored in `.iron/state/` and are **not** git-tracked.

### Current Host State

**Location**: `.iron/state/current_host.json`

```json
{
    "host_id": "desktop",
    "detected_at": "2025-02-12T10:30:00Z"
}
```

### Active Bundle State

**Location**: `.iron/state/active_bundle.json`

```json
{
    "bundle_id": "hyprland",
    "activated_at": "2025-02-12T10:35:00Z",
    "dotfiles_linked": [
        "~/.config/hypr",
        "~/.config/waybar",
        "~/.config/wofi"
    ]
}
```

### Active Profile State

**Location**: `.iron/state/active_profile.json`

```json
{
    "profile_id": "developer",
    "selected_at": "2025-02-12T10:40:00Z"
}
```

### Enabled Modules State

**Location**: `.iron/state/enabled_modules.json`

```json
{
    "modules": [
        "nvim-ide",
        "kitty-dev",
        "waybar-dev",
        "dev-tools",
        "git-config"
    ],
    "updated_at": "2025-02-12T10:45:00Z"
}
```

### Maintenance State

**Location**: `.iron/state/maintenance.json`

```json
{
    "last_update": "2025-02-10T15:30:00Z",
    "last_clean": "2025-02-05T09:00:00Z",
    "last_doctor": "2025-02-01T18:00:00Z",
    "last_snapshot": "2025-02-10T15:29:00Z",
    "last_sync": "2025-02-12T10:00:00Z"
}
```

### Operations Log

**Location**: `.iron/state/operations.jsonl`

```jsonl
{"op":"bundle_switch","from":"niri","to":"hyprland","timestamp":"2025-02-12T10:35:00Z","status":"success"}
{"op":"profile_select","profile":"developer","timestamp":"2025-02-12T10:40:00Z","status":"success"}
{"op":"module_enable","module":"dev-tools","timestamp":"2025-02-12T10:42:00Z","status":"success"}
{"op":"update","packages":15,"timestamp":"2025-02-10T15:30:00Z","status":"success","risk":"low"}
```

---

## Appendix: TOML Quick Reference

### Basic Types

```toml
# String
name = "value"

# Integer
count = 42

# Float
scale = 1.5

# Boolean
enabled = true

# Array
packages = ["pkg1", "pkg2", "pkg3"]

# Inline table
monitor = { output = "DP-1", resolution = "2560x1440" }
```

### Tables

```toml
# Table
[section]
key = "value"

# Nested table
[section.subsection]
key = "value"

# Array of tables
[[items]]
id = "first"

[[items]]
id = "second"
```

### Best Practices

1. Use lowercase with hyphens for IDs: `nvim-ide`, `waybar-dev`
2. Use descriptive names: `"Neovim IDE"` not `"nvim"`
3. Always include `description` for documentation
4. Keep package lists sorted alphabetically
5. Use relative paths for scripts and dotfiles
