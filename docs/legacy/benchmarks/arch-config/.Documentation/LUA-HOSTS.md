# Lua Host Configuration Guide

## Overview

Lua host files allow you to write dynamic, conditional system configurations that adapt to the machine they run on. Instead of maintaining separate YAML files for each host, you can use a single Lua configuration that detects hardware, checks system resources, and configures itself accordingly.

**Use Lua host files when you need:**

- **Dynamic module selection** - Enable modules based on hardware, RAM, or CPU cores
- **Hardware-aware configuration** - Auto-detect GPUs, laptops, chassis type
- **Single config for multiple machines** - One file that works everywhere
- **Conditional packages** - Install packages only if certain conditions are met
- **Runtime information** - Include system stats in descriptions

## Quick Start

### Basic Lua Host File

Create a `.lua` file in your hosts directory:

```lua
-- ~/.config/arch-config/hosts/myhost.lua

return {
    host = "myhost",
    description = "My workstation",
    
    enabled_modules = {
        "base",
        "development",
        "gaming",
    },
    
    packages = {
        "firefox",
        "neovim",
        "flatpak:com.spotify.Client",
    },
    
    services = {
        enabled = { "docker.service" },
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
}
```

Reference it from `config.yaml`:

```yaml
host: myhost
```

### Dynamic Configuration

The real power comes from using the dcli APIs:

```lua
-- ~/.config/arch-config/hosts/workstation.lua

local hostname = dcli.system.hostname()
local memory_mb = dcli.system.memory_total_mb()
local is_laptop = dcli.hardware.is_laptop()

-- Start with base modules
local enabled_modules = { "base", "cli-tools" }

-- Add GPU drivers based on hardware
if dcli.hardware.has_nvidia() then
    table.insert(enabled_modules, "nvidia-drivers")
elseif dcli.hardware.has_amd_gpu() then
    table.insert(enabled_modules, "amd-drivers")
end

-- Add development tools if we have enough RAM
if memory_mb >= 16000 then
    table.insert(enabled_modules, "development")
    table.insert(enabled_modules, "docker")
end

-- Laptop-specific modules
if is_laptop then
    table.insert(enabled_modules, "laptop-power")
    table.insert(enabled_modules, "bluetooth")
end

return {
    host = hostname,
    description = string.format(
        "%s - %s with %d GB RAM",
        hostname,
        is_laptop and "Laptop" or "Desktop",
        math.floor(memory_mb / 1024)
    ),
    
    enabled_modules = enabled_modules,
    
    packages = { "firefox", "neovim" },
    
    flatpak_scope = "user",
    aur_helper = "paru",
}
```

## Configuration Structure

A Lua host file must return a table with at least a `host` field:

```lua
return {
    -- ═══════════════════════════════════════════════════════════════
    -- REQUIRED
    -- ═══════════════════════════════════════════════════════════════
    
    host = "hostname",  -- Must match your system hostname
    
    -- ═══════════════════════════════════════════════════════════════
    -- IDENTITY & ORGANIZATION
    -- ═══════════════════════════════════════════════════════════════
    
    -- Human-readable description
    description = "My workstation configuration",
    
    -- Import shared config files (can be .yaml or .lua)
    import = {
        "hosts/shared/common.yaml",
        "hosts/shared/development.lua",
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- MODULES & PACKAGES
    -- ═══════════════════════════════════════════════════════════════
    
    -- Modules to enable
    enabled_modules = {
        "base",
        "development",
        "window-managers/hyprland",
    },
    
    -- Host-specific packages
    packages = {
        "firefox",                              -- Simple pacman package
        "flatpak:com.spotify.Client",           -- Flatpak with prefix
        { name = "discord", type = "flatpak" }, -- Table format
    },
    
    -- Packages to exclude (from modules or base)
    exclude = {
        "vim",      -- Using neovim instead
        "nano",     -- Don't need it
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- SERVICES
    -- ═══════════════════════════════════════════════════════════════
    
    services = {
        enabled = {
            "docker.service",
            "sshd.service",
            "bluetooth.service",
        },
        disabled = {
            "cups.service",
            "avahi-daemon.service",
        },
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- DEFAULT APPLICATIONS
    -- ═══════════════════════════════════════════════════════════════
    
    default_apps = {
        scope = "system",  -- "system" or "user"
        
        -- High-level categories
        browser = "firefox",
        text_editor = "code",
        file_manager = "thunar",
        terminal = "kitty",
        video_player = "vlc",
        audio_player = "rhythmbox",
        image_viewer = "feh",
        pdf_viewer = "zathura",
        
        -- Custom MIME type mappings
        mime_types = {
            ["application/pdf"] = "zathura",
            ["text/plain"] = "nvim",
            ["image/png"] = "feh",
        },
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- UPDATE HOOKS
    -- ═══════════════════════════════════════════════════════════════
    
    update_hooks = {
        pre_update = "scripts/pre-update.sh",
        post_update = "scripts/post-update.sh",
        behavior = "ask",  -- "ask", "always", "once", "skip", "never"
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- BACKUP SETTINGS
    -- ═══════════════════════════════════════════════════════════════
    
    config_backups = {
        enabled = true,
        max_backups = 5,
    },
    
    system_backups = {
        enabled = true,
        backup_on_sync = true,
        backup_on_update = true,
        tool = "timeshift",       -- "timeshift" or "snapper"
        snapper_config = "root",  -- Only for snapper
    },
    
    -- ═══════════════════════════════════════════════════════════════
    -- SYSTEM SETTINGS
    -- ═══════════════════════════════════════════════════════════════
    
    flatpak_scope = "user",           -- "user" or "system"
    auto_prune = false,               -- Auto-remove unused packages
    module_processing = "parallel",   -- "parallel" or "sequential"
    strict_package_order = false,     -- Install packages one-by-one
    editor = "nvim",                  -- Config file editor
    aur_helper = "paru",              -- AUR helper (paru, yay, etc.)
}
```

## File Locations & Priority

### Config File Priority

dcli checks for configuration files in this order:

1. `~/.config/arch-config/config.lua` (preferred)
2. `~/.config/arch-config/config.yaml` (fallback)

### Host File Priority

When loading host files, dcli checks:

1. `~/.config/arch-config/hosts/{hostname}.lua` (preferred)
2. `~/.config/arch-config/hosts/{hostname}.yaml` (fallback)
3. `~/.config/arch-config/packages/hosts/{hostname}.lua` (legacy location)
4. `~/.config/arch-config/packages/hosts/{hostname}.yaml` (legacy location)

### Directory Structure

```
~/.config/arch-config/
├── config.lua              # Main config (or config.yaml)
├── hosts/
│   ├── workstation.lua     # Host-specific config
│   ├── laptop.lua
│   ├── gaming-pc.lua
│   └── shared/
│       ├── common.lua      # Shared imports
│       └── development.lua
└── modules/
    ├── base.lua
    ├── gaming.lua
    └── development/
        └── module.lua
```

## Available APIs

All dcli APIs are available in host configuration files. See [LUA-MODULES.md](LUA-MODULES.md) for complete API documentation.

### Quick Reference

| Namespace | Key Functions |
|-----------|---------------|
| `dcli.hardware` | `cpu_vendor()`, `has_nvidia()`, `has_amd_gpu()`, `is_laptop()`, `chassis_type()` |
| `dcli.system` | `hostname()`, `memory_total_mb()`, `cpu_cores()`, `distro()`, `arch()` |
| `dcli.package` | `is_installed(name)`, `version(name)`, `flatpak_installed(id)` |
| `dcli.env` | `home()`, `user()`, `config_dir()`, `get(var)` |
| `dcli.file` | `exists(path)`, `is_file(path)`, `is_dir(path)` |
| `dcli.util` | `contains(t, v)`, `extend(t1, t2)`, `merge(t1, t2)` |
| `dcli.log` | `info(msg)`, `warn(msg)`, `debug(msg)`, `error(msg)` |

## Examples

### Universal Config File

A single `config.lua` that works on all your machines:

```lua
-- ~/.config/arch-config/config.lua

local hostname = dcli.system.hostname()
local memory_mb = dcli.system.memory_total_mb()
local cpu_cores = dcli.system.cpu_cores()
local is_laptop = dcli.hardware.is_laptop()

dcli.log.info(string.format("Configuring %s (%d MB RAM, %d cores)", 
    hostname, memory_mb, cpu_cores))

-- ═══════════════════════════════════════════════════════════════════
-- MODULE SELECTION
-- ═══════════════════════════════════════════════════════════════════

local enabled_modules = {
    "base",
    "cli-tools",
}

-- GPU drivers
if dcli.hardware.has_nvidia() then
    table.insert(enabled_modules, "nvidia-drivers")
    dcli.log.info("NVIDIA GPU detected")
elseif dcli.hardware.has_amd_gpu() then
    table.insert(enabled_modules, "amd-drivers")
    dcli.log.info("AMD GPU detected")
elseif dcli.hardware.has_intel_gpu() then
    table.insert(enabled_modules, "intel-graphics")
end

-- Laptop-specific
if is_laptop then
    table.insert(enabled_modules, "laptop-power")
    table.insert(enabled_modules, "bluetooth")
end

-- Resource-based modules
if memory_mb >= 8000 then
    table.insert(enabled_modules, "development")
end

if memory_mb >= 16000 then
    table.insert(enabled_modules, "docker")
end

if memory_mb >= 32000 and cpu_cores >= 8 then
    table.insert(enabled_modules, "virtualization")
end

-- Host-specific modules
local host_modules = {
    workstation = { "databases", "kubernetes" },
    ["gaming-pc"] = { "gaming", "streaming" },
    laptop = { "office" },
    server = { "server", "monitoring" },
}

if host_modules[hostname] then
    dcli.util.extend(enabled_modules, host_modules[hostname])
end

-- ═══════════════════════════════════════════════════════════════════
-- SERVICES
-- ═══════════════════════════════════════════════════════════════════

local services = {
    enabled = { "sshd.service" },
    disabled = {},
}

-- Docker service if module enabled
if dcli.util.contains(enabled_modules, "docker") then
    table.insert(services.enabled, "docker.service")
end

-- Laptop services
if is_laptop then
    table.insert(services.enabled, "tlp.service")
    table.insert(services.enabled, "bluetooth.service")
    table.insert(services.disabled, "power-profiles-daemon.service")
end

-- ═══════════════════════════════════════════════════════════════════
-- PACKAGES
-- ═══════════════════════════════════════════════════════════════════

local packages = {
    "firefox",
    "neovim",
    "git",
}

-- Add Flatpak apps on desktop systems
if not is_laptop or memory_mb >= 16000 then
    table.insert(packages, "flatpak:com.spotify.Client")
    table.insert(packages, "flatpak:com.discordapp.Discord")
end

-- ═══════════════════════════════════════════════════════════════════
-- RETURN CONFIGURATION
-- ═══════════════════════════════════════════════════════════════════

return {
    host = hostname,
    description = string.format(
        "%s - %s (%d GB RAM, %d cores)",
        hostname,
        is_laptop and "Laptop" or "Desktop",
        math.floor(memory_mb / 1024),
        cpu_cores
    ),
    
    enabled_modules = enabled_modules,
    packages = packages,
    services = services,
    
    default_apps = {
        browser = "firefox",
        terminal = is_laptop and "alacritty" or "kitty",
        text_editor = "code",
        file_manager = "thunar",
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
    module_processing = is_laptop and "sequential" or "parallel",
    
    system_backups = {
        enabled = true,
        tool = "timeshift",
        backup_on_sync = true,
    },
}
```

### Workstation Configuration

```lua
-- ~/.config/arch-config/hosts/workstation.lua

local memory_mb = dcli.system.memory_total_mb()

return {
    host = "workstation",
    description = string.format("Development workstation (%d GB RAM)", 
        math.floor(memory_mb / 1024)),
    
    enabled_modules = {
        "base",
        "development",
        "docker",
        "kubernetes",
        "databases",
        "nvidia-drivers",
    },
    
    packages = {
        -- Browsers
        "firefox",
        "chromium",
        
        -- Development
        "code",
        "neovim",
        "lazygit",
        
        -- Communication
        "flatpak:com.slack.Slack",
        "flatpak:com.discordapp.Discord",
        
        -- Media
        "flatpak:com.spotify.Client",
    },
    
    services = {
        enabled = {
            "docker.service",
            "sshd.service",
            "postgresql.service",
        },
    },
    
    default_apps = {
        scope = "system",
        browser = "firefox",
        terminal = "kitty",
        text_editor = "code",
        file_manager = "thunar",
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
    module_processing = "parallel",
    
    config_backups = {
        enabled = true,
        max_backups = 10,
    },
    
    system_backups = {
        enabled = true,
        tool = "timeshift",
        backup_on_sync = true,
        backup_on_update = true,
    },
}
```

### Gaming PC Configuration

```lua
-- ~/.config/arch-config/hosts/gaming-pc.lua

local has_nvidia = dcli.hardware.has_nvidia()

return {
    host = "gaming-pc",
    description = "Gaming rig" .. (has_nvidia and " (NVIDIA)" or ""),
    
    enabled_modules = {
        "base",
        "gaming",
        has_nvidia and "nvidia-drivers" or "amd-drivers",
    },
    
    packages = {
        -- Gaming
        "steam",
        "lutris",
        "gamemode",
        "mangohud",
        
        -- Communication
        "flatpak:com.discordapp.Discord",
        
        -- Streaming (if NVIDIA for NVENC)
        has_nvidia and "obs-studio" or nil,
        
        -- Game launchers
        "flatpak:com.heroicgameslauncher.hgl",
    },
    
    services = {
        enabled = {
            "gamemode.service",
            has_nvidia and "nvidia-persistenced.service" or nil,
        },
    },
    
    default_apps = {
        browser = "firefox",
        terminal = "kitty",
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
}
```

### Laptop Configuration

```lua
-- ~/.config/arch-config/hosts/laptop.lua

local memory_mb = dcli.system.memory_total_mb()
local battery_present = dcli.hardware.has_battery()

dcli.log.info(string.format("Laptop with %d MB RAM, battery: %s", 
    memory_mb, tostring(battery_present)))

-- Lighter module set for laptop
local enabled_modules = {
    "base",
    "laptop-power",
    "bluetooth",
    "wifi",
}

-- Only add heavy modules if we have enough RAM
if memory_mb >= 16000 then
    table.insert(enabled_modules, "development")
end

-- GPU detection
if dcli.hardware.has_nvidia() then
    table.insert(enabled_modules, "nvidia-optimus")
elseif dcli.hardware.has_amd_gpu() then
    table.insert(enabled_modules, "amd-drivers")
end

return {
    host = "laptop",
    description = string.format("Portable laptop (%d GB)", 
        math.floor(memory_mb / 1024)),
    
    enabled_modules = enabled_modules,
    
    packages = {
        "firefox",
        "neovim",
        "flatpak:com.spotify.Client",
    },
    
    services = {
        enabled = {
            "tlp.service",
            "bluetooth.service",
            "NetworkManager.service",
        },
        disabled = {
            "power-profiles-daemon.service",
        },
    },
    
    default_apps = {
        browser = "firefox",
        terminal = "alacritty",  -- Lighter than kitty
        text_editor = "code",
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
    
    -- Sequential processing saves battery
    module_processing = "sequential",
    
    system_backups = {
        enabled = battery_present,  -- Only if on AC power typically
        tool = "timeshift",
        backup_on_sync = false,     -- Don't backup on battery
        backup_on_update = true,
    },
}
```

### Server Configuration

```lua
-- ~/.config/arch-config/hosts/server.lua

local memory_mb = dcli.system.memory_total_mb()
local cpu_cores = dcli.system.cpu_cores()

return {
    host = "server",
    description = string.format("Server (%d cores, %d GB RAM)", 
        cpu_cores, math.floor(memory_mb / 1024)),
    
    enabled_modules = {
        "base",
        "server",
        "docker",
        "monitoring",
    },
    
    packages = {
        -- Server essentials
        "nginx",
        "postgresql",
        "redis",
        
        -- Security
        "fail2ban",
        "ufw",
        
        -- Monitoring
        "htop",
        "iotop",
        "nethogs",
    },
    
    -- No GUI packages
    exclude = {
        "firefox",
        "chromium",
        "code",
    },
    
    services = {
        enabled = {
            "sshd.service",
            "docker.service",
            "nginx.service",
            "postgresql.service",
            "redis.service",
            "fail2ban.service",
        },
        disabled = {
            "bluetooth.service",
            "cups.service",
        },
    },
    
    -- No default apps (headless)
    default_apps = {},
    
    flatpak_scope = "system",
    aur_helper = "paru",
    module_processing = "sequential",
    
    system_backups = {
        enabled = true,
        tool = "snapper",
        snapper_config = "root",
        backup_on_sync = true,
        backup_on_update = true,
    },
}
```

### Using Imports for Shared Configuration

**Shared common config:**

```lua
-- ~/.config/arch-config/hosts/shared/common.lua

-- This file is imported by other configs
-- It returns partial configuration to be merged

return {
    -- Common packages for all hosts
    packages = {
        "base-devel",
        "git",
        "neovim",
        "htop",
        "ripgrep",
        "fd",
        "bat",
        "exa",
    },
    
    -- Common services
    services = {
        enabled = { "sshd.service" },
    },
    
    -- Common settings
    flatpak_scope = "user",
    aur_helper = "paru",
    
    config_backups = {
        enabled = true,
        max_backups = 5,
    },
}
```

**Host file that imports shared config:**

```lua
-- ~/.config/arch-config/hosts/myhost.lua

return {
    host = "myhost",
    description = "My host with shared config",
    
    -- Import shared configuration
    import = {
        "hosts/shared/common.lua",
    },
    
    -- Host-specific additions
    enabled_modules = {
        "base",
        "development",
    },
    
    -- These packages are added to imported packages
    packages = {
        "firefox",
        "code",
    },
}
```

## Using config.lua as a Pointer

You can use a minimal `config.lua` that just detects the hostname:

```lua
-- ~/.config/arch-config/config.lua

-- Automatically use the correct host file based on hostname
return {
    host = dcli.system.hostname(),
}
```

This will load `hosts/{hostname}.lua` (or `.yaml`) automatically.

## Helper Functions

Define reusable functions for cleaner configuration:

```lua
-- ~/.config/arch-config/config.lua

-- Helper: Add packages if condition is true
local function add_if(packages, condition, ...)
    if condition then
        for _, pkg in ipairs({...}) do
            table.insert(packages, pkg)
        end
    end
end

-- Helper: Create a package group
local function gaming_packages()
    return {
        "steam",
        "lutris", 
        "gamemode",
        "mangohud",
    }
end

local function dev_packages()
    return {
        "git",
        "neovim",
        "code",
        "docker",
    }
end

-- Build package list
local packages = { "firefox", "kitty" }

add_if(packages, dcli.hardware.chassis_type() ~= "server", 
    "flatpak:com.spotify.Client",
    "flatpak:com.discordapp.Discord")

if dcli.system.hostname() == "gaming-pc" then
    dcli.util.extend(packages, gaming_packages())
end

if dcli.system.memory_total_mb() >= 16000 then
    dcli.util.extend(packages, dev_packages())
end

return {
    host = dcli.system.hostname(),
    packages = packages,
    -- ...
}
```

## Validation

Validate your Lua configuration files:

```bash
# Validate all configs and modules
dcli validate

# Check config loading
dcli config show
```

### Common Errors

**Missing host field:**
```
Error: Lua config must have a 'host' field
HINT: Add: host = "your-hostname"
```

**Invalid enum value:**
```
Error: Invalid flatpak_scope: 'invalid'
HINT: Valid values are: 'user', 'system'
```

**Syntax error:**
```
Error: [string "config.lua"]:15: '}' expected near 'packages'
HINT: Missing closing brace '}'
```

**Typo in field name:**
```
Warning: Unknown field 'enabeld_modules'. Did you mean 'enabled_modules'?
```

## Migration from YAML

Converting a YAML host file to Lua:

**YAML:**
```yaml
host: workstation
description: My workstation

enabled_modules:
  - base
  - development
  - docker

packages:
  - firefox
  - neovim
  - flatpak:com.spotify.Client

services:
  enabled:
    - docker.service
    - sshd.service

flatpak_scope: user
aur_helper: paru
```

**Lua equivalent:**
```lua
return {
    host = "workstation",
    description = "My workstation",
    
    enabled_modules = {
        "base",
        "development", 
        "docker",
    },
    
    packages = {
        "firefox",
        "neovim",
        "flatpak:com.spotify.Client",
    },
    
    services = {
        enabled = {
            "docker.service",
            "sshd.service",
        },
    },
    
    flatpak_scope = "user",
    aur_helper = "paru",
}
```

Then add conditional logic as needed!

## Best Practices

1. **Start simple** - Begin with a direct YAML-to-Lua conversion, then add conditions

2. **Use descriptive logging** - Help debug configuration issues
   ```lua
   dcli.log.info("Loading config for " .. hostname)
   dcli.log.debug("Memory: " .. memory_mb .. " MB")
   ```

3. **Handle edge cases** - What if hardware detection fails?
   ```lua
   local hostname = dcli.system.hostname() or "unknown"
   ```

4. **Test with dry-run** - Verify before applying
   ```bash
   dcli sync --dry-run
   ```

5. **Keep it readable** - Use comments and organize sections

6. **Use functions for reusability** - Extract common patterns

7. **Version control your config** - Track changes with git

## Related Documentation

- [LUA-MODULES.md](LUA-MODULES.md) - Lua module format and full API reference
- [DIRECTORY-MODULES.md](DIRECTORY-MODULES.md) - Directory-based modules
- [README.md](README.md) - Main dcli documentation
