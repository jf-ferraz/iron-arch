# Lua Modules Guide

## Overview

Lua modules provide a powerful alternative to YAML for creating dynamic, conditional package configurations. While YAML modules are static declarations, Lua modules can execute logic to determine what packages, services, and hooks to include.

**Use Lua modules when you need:**

- 🔀 **Conditional logic** - Install packages based on hardware, hostname, or system state
- 🔧 **Functions** - Reusable package group definitions and helpers
- 🎯 **Dynamic configuration** - Generate package lists programmatically
- 🖥️ **Hardware detection** - Auto-configure GPU drivers, laptop power management, etc.

## Quick Start

### Basic Lua Module

Create a file with `.lua` extension in your modules directory:

```lua
-- ~/.config/arch-config/modules/gaming.lua

return {
    description = "Gaming packages",
    packages = {
        "steam",
        "lutris",
        "wine",
        "gamemode",
    },
}
```

Enable and sync like any other module:

```bash
dcli module enable gaming
dcli sync
```

### Hardware-Conditional Module

```lua
-- gpu-drivers.lua
local packages = {}

-- Add CPU microcode based on vendor
if dcli.hardware.cpu_vendor() == "intel" then
    table.insert(packages, "intel-ucode")
elseif dcli.hardware.cpu_vendor() == "amd" then
    table.insert(packages, "amd-ucode")
end

-- Add GPU drivers based on detected hardware
if dcli.hardware.has_nvidia() then
    table.insert(packages, "nvidia")
    table.insert(packages, "nvidia-utils")
    table.insert(packages, "nvidia-settings")
elseif dcli.hardware.has_amd_gpu() then
    table.insert(packages, "mesa")
    table.insert(packages, "vulkan-radeon")
    table.insert(packages, "libva-mesa-driver")
end

return {
    description = "GPU drivers (auto-detected)",
    packages = packages,
}
```

## Module Structure

A Lua module must return a table. The only required field is `packages`, but you can include any of the following:

```lua
return {
    -- Required: List of packages to install
    packages = { "pkg1", "pkg2" },
    
    -- Optional: Human-readable description
    description = "My module description",
    
    -- Optional: Modules that conflict with this one
    conflicts = { "other-module", "another-module" },
    
    -- Optional: Services to enable/disable
    services = {
        enabled = { "docker.service", "sshd.service" },
        disabled = { "cups.service" },
    },
    
    -- Optional: Hook scripts (relative to module location)
    pre_install_hook = "scripts/pre-setup.sh",
    post_install_hook = "scripts/post-setup.sh",
    
    -- Optional: Hook behavior ("ask", "once", "always", "never")
    hook_behavior = "ask",           -- Default for both hooks
    pre_hook_behavior = "once",      -- Override for pre-hook
    post_hook_behavior = "always",   -- Override for post-hook
    
    -- Optional: Arbitrary metadata (stored as JSON)
    metadata = {
        author = "your-name",
        version = "1.0",
        tags = { "gaming", "steam" },
    },
}
```

### Package Entry Formats

Packages can be specified in several formats:

```lua
packages = {
    -- Simple string (pacman package)
    "firefox",
    "git",
    
    -- Flatpak with prefix syntax
    "flatpak:com.spotify.Client",
    "flatpak:org.videolan.VLC",
    
    -- Table format with explicit type
    { name = "com.discordapp.Discord", type = "flatpak" },
    { name = "steam", type = "pacman" },  -- explicit pacman (default)
}
```

## Available APIs

### dcli.hardware

Hardware detection functions for conditional configuration.

| Function | Returns | Description |
|----------|---------|-------------|
| `cpu_vendor()` | `"intel"`, `"amd"`, `"unknown"` | CPU manufacturer |
| `gpu_vendors()` | `{"nvidia", "amd", "intel"}` | Array of all detected GPU vendors |
| `has_nvidia()` | `boolean` | NVIDIA GPU present |
| `has_amd_gpu()` | `boolean` | AMD GPU present |
| `has_intel_gpu()` | `boolean` | Intel integrated GPU present |
| `is_laptop()` | `boolean` | Running on a laptop |
| `has_battery()` | `boolean` | Battery present in system |
| `chassis_type()` | `string` | `"desktop"`, `"laptop"`, `"server"`, `"tablet"`, `"unknown"` |

**Example:**

```lua
if dcli.hardware.is_laptop() then
    dcli.log.info("Laptop detected - adding power management")
    table.insert(packages, "tlp")
    table.insert(packages, "powertop")
end

local gpus = dcli.hardware.gpu_vendors()
for _, vendor in ipairs(gpus) do
    dcli.log.info("Found GPU: " .. vendor)
end
```

### dcli.system

System information functions.

| Function | Returns | Description |
|----------|---------|-------------|
| `hostname()` | `string` | System hostname |
| `kernel_version()` | `string` | Running kernel version (e.g., `"6.7.1-arch1-1"`) |
| `arch()` | `string` | System architecture (`"x86_64"`, `"aarch64"`, etc.) |
| `os()` | `string` | Operating system (`"linux"`, `"macos"`, `"windows"`) |
| `distro()` | `string` | Distribution ID (`"arch"`, `"endeavouros"`, `"manjaro"`) |
| `distro_name()` | `string` | Distribution name (`"Arch Linux"`, `"EndeavourOS"`) |
| `distro_version()` | `string` | Distribution version or `"rolling"` for Arch |
| `memory_total_mb()` | `number` | Total system RAM in megabytes |
| `cpu_cores()` | `number` | Number of CPU cores |

**Example:**

```lua
local hostname = dcli.system.hostname()

if hostname == "workstation" then
    -- Add development packages for workstation
    table.insert(packages, "docker")
    table.insert(packages, "kubectl")
elseif hostname == "gaming-pc" then
    -- Add gaming packages
    table.insert(packages, "steam")
    table.insert(packages, "discord")
end

-- Check system resources
if dcli.system.memory_total_mb() >= 16000 then
    dcli.log.info("High memory system - enabling heavy packages")
    table.insert(packages, "flatpak:org.blender.Blender")
end

-- Check distribution
if dcli.system.distro() == "endeavouros" then
    dcli.log.info("EndeavourOS detected")
end
```

### dcli.file

File system operations (sandboxed for security).

| Function | Returns | Description |
|----------|---------|-------------|
| `exists(path)` | `boolean` | Check if a file or directory exists |
| `is_file(path)` | `boolean` | Check if path is a file |
| `is_dir(path)` | `boolean` | Check if path is a directory |
| `read(path)` | `string` or `nil` | Read file contents (sandboxed paths only) |
| `read_lines(path)` | `table` or `nil` | Read file as array of lines |

**Sandboxed Paths:** For security, `read()` and `read_lines()` only work with:
- `/sys/` - System hardware information
- `/proc/` - Process and kernel information
- `/etc/os-release` - Distribution information
- `/etc/hostname` - System hostname
- `/etc/machine-id` - Machine identifier

**Example:**

```lua
-- Check if a config file exists
if dcli.file.exists("/etc/docker/daemon.json") then
    dcli.log.info("Docker already configured")
end

-- Read OS release info
local os_release = dcli.file.read("/etc/os-release")
if os_release and os_release:match("EndeavourOS") then
    dcli.log.info("Running on EndeavourOS")
end

-- Read and process lines
local meminfo = dcli.file.read_lines("/proc/meminfo")
if meminfo then
    for _, line in ipairs(meminfo) do
        if line:match("^MemTotal:") then
            dcli.log.info("Memory: " .. line)
        end
    end
end
```

### dcli.log

Logging functions for debugging and information.

| Function | Description |
|----------|-------------|
| `info(msg)` | Log an informational message |
| `warn(msg)` | Log a warning message |
| `error(msg)` | Log an error message |
| `debug(msg)` | Log a debug message (visible with `RUST_LOG=debug`) |

**Example:**

```lua
dcli.log.info("Loading gaming module...")
dcli.log.warn("NVIDIA detected but nouveau might conflict")
dcli.log.debug("Package count: " .. #packages)
```

### dcli.env

Environment variable and XDG directory helpers.

| Function | Returns | Description |
|----------|---------|-------------|
| `get(name)` | `string` or `nil` | Get environment variable value |
| `home()` | `string` | User's home directory |
| `user()` | `string` | Current username |
| `config_dir()` | `string` | XDG config directory (`~/.config`) |
| `data_dir()` | `string` | XDG data directory (`~/.local/share`) |
| `cache_dir()` | `string` | XDG cache directory (`~/.cache`) |
| `shell()` | `string` | User's default shell |

**Example:**

```lua
local user = dcli.env.user()
local home = dcli.env.home()

dcli.log.info("Setting up for user: " .. user)

-- Check for existing configuration
if dcli.file.exists(dcli.env.config_dir() .. "/nvim") then
    dcli.log.info("Neovim config already exists")
end

-- Check for specific environment
if dcli.env.get("WAYLAND_DISPLAY") then
    dcli.log.info("Wayland session detected")
    table.insert(packages, "wl-clipboard")
else
    table.insert(packages, "xclip")
end
```

### dcli.package

Package query functions for checking installed packages.

| Function | Returns | Description |
|----------|---------|-------------|
| `is_installed(name)` | `boolean` | Check if pacman package is installed |
| `version(name)` | `string` or `nil` | Get installed package version |
| `is_available(name)` | `boolean` | Check if package exists in repos |
| `repo(name)` | `string` or `nil` | Get package repository (core, extra, etc.) |
| `is_foreign(name)` | `boolean` | Check if package is from AUR/manual |
| `depends_on(name)` | `table` | Get package dependencies |
| `required_by(name)` | `table` | Get reverse dependencies |
| `list_installed()` | `table` | List all installed packages |
| `list_explicit()` | `table` | List explicitly installed packages |
| `flatpak_installed(id)` | `boolean` | Check if Flatpak app is installed |
| `flatpak_version(id)` | `string` or `nil` | Get Flatpak app version |
| `aur_available(name)` | `boolean` | Check if package exists in AUR |

**Example:**

```lua
-- Only add packages if not already installed
if not dcli.package.is_installed("docker") then
    table.insert(packages, "docker")
    table.insert(packages, "docker-compose")
end

-- Check for conflicting packages
if dcli.package.is_installed("pulseaudio") then
    dcli.log.warn("PulseAudio installed - consider migrating to PipeWire")
end

-- Version-based decisions
local kernel_ver = dcli.package.version("linux")
if kernel_ver then
    dcli.log.info("Running kernel: " .. kernel_ver)
end

-- Check Flatpak
if not dcli.package.flatpak_installed("com.spotify.Client") then
    table.insert(packages, "flatpak:com.spotify.Client")
end
```

### dcli.util

Utility functions for common operations.

| Function | Returns | Description |
|----------|---------|-------------|
| `contains(table, value)` | `boolean` | Check if array contains value |
| `keys(table)` | `table` | Get array of table keys |
| `values(table)` | `table` | Get array of table values |
| `merge(t1, t2)` | `table` | Merge tables (t2 overrides t1) |
| `extend(target, source)` | `table` | Append source items to target array |
| `split(str, delim)` | `table` | Split string by delimiter |
| `trim(str)` | `string` | Remove leading/trailing whitespace |
| `starts_with(str, prefix)` | `boolean` | Check string prefix |
| `ends_with(str, suffix)` | `boolean` | Check string suffix |
| `version_compare(v1, v2)` | `number` | Compare versions: -1, 0, or 1 |
| `version_gte(v1, v2)` | `boolean` | v1 >= v2 |
| `version_gt(v1, v2)` | `boolean` | v1 > v2 |
| `version_lte(v1, v2)` | `boolean` | v1 <= v2 |
| `version_lt(v1, v2)` | `boolean` | v1 < v2 |

**Example:**

```lua
-- Check if value exists in array
local my_packages = { "git", "neovim", "htop" }
if dcli.util.contains(my_packages, "neovim") then
    dcli.log.info("Neovim is in the list")
end

-- Merge package lists
local base = { "git", "curl", "wget" }
local extra = { "htop", "btop" }
local all = dcli.util.merge(base, extra)

-- Extend an existing array
dcli.util.extend(packages, { "pkg1", "pkg2", "pkg3" })

-- Version comparison
local kernel = dcli.package.version("linux") or "0"
if dcli.util.version_gte(kernel, "6.0") then
    dcli.log.info("Modern kernel detected")
end

-- String utilities
local path = "/home/user/.config/nvim"
if dcli.util.ends_with(path, "nvim") then
    dcli.log.info("Neovim config path")
end
```

## Examples

### GPU Drivers Module

Automatically detect and install appropriate GPU drivers:

```lua
-- gpu-drivers.lua
local packages = {}
local description_parts = {}

-- CPU microcode
local cpu = dcli.hardware.cpu_vendor()
if cpu == "intel" then
    table.insert(packages, "intel-ucode")
    table.insert(description_parts, "Intel CPU")
elseif cpu == "amd" then
    table.insert(packages, "amd-ucode")
    table.insert(description_parts, "AMD CPU")
end

-- NVIDIA GPU
if dcli.hardware.has_nvidia() then
    dcli.log.info("NVIDIA GPU detected")
    table.insert(description_parts, "NVIDIA GPU")
    
    -- Proprietary drivers
    table.insert(packages, "nvidia")
    table.insert(packages, "nvidia-utils")
    table.insert(packages, "nvidia-settings")
    table.insert(packages, "lib32-nvidia-utils")
    
    -- CUDA for compute
    -- table.insert(packages, "cuda")
end

-- AMD GPU
if dcli.hardware.has_amd_gpu() then
    dcli.log.info("AMD GPU detected")
    table.insert(description_parts, "AMD GPU")
    
    table.insert(packages, "mesa")
    table.insert(packages, "vulkan-radeon")
    table.insert(packages, "lib32-vulkan-radeon")
    table.insert(packages, "libva-mesa-driver")
end

-- Intel GPU (integrated)
if dcli.hardware.has_intel_gpu() then
    dcli.log.info("Intel GPU detected")
    table.insert(description_parts, "Intel GPU")
    
    table.insert(packages, "mesa")
    table.insert(packages, "vulkan-intel")
    table.insert(packages, "intel-media-driver")
end

-- Build description
local description = "Hardware drivers"
if #description_parts > 0 then
    description = description .. " (" .. table.concat(description_parts, ", ") .. ")"
end

return {
    description = description,
    packages = packages,
}
```

### Laptop Power Management

Only install power management on laptops:

```lua
-- laptop.lua
if not dcli.hardware.is_laptop() then
    dcli.log.info("Not a laptop - skipping power management")
    return {
        description = "Laptop power management (skipped - desktop detected)",
        packages = {},
    }
end

dcli.log.info("Laptop detected - configuring power management")

return {
    description = "Laptop power management",
    
    packages = {
        "tlp",
        "tlp-rdw",
        "powertop",
        "acpi",
        "acpid",
    },
    
    services = {
        enabled = {
            "tlp.service",
            "acpid.service",
        },
        disabled = {
            -- Conflicts with TLP
            "power-profiles-daemon.service",
        },
    },
    
    post_install_hook = "scripts/setup-tlp.sh",
    hook_behavior = "once",
}
```

### Host-Specific Configuration

Different packages for different machines:

```lua
-- host-config.lua
local hostname = dcli.system.hostname()
local packages = {}
local services = { enabled = {}, disabled = {} }
local description = "Host-specific packages"

-- Common packages for all hosts
local common = {
    "base-devel",
    "git",
    "neovim",
    "htop",
    "ripgrep",
}

for _, pkg in ipairs(common) do
    table.insert(packages, pkg)
end

-- Host-specific configuration
if hostname == "workstation" then
    description = "Workstation development environment"
    
    -- Development tools
    table.insert(packages, "docker")
    table.insert(packages, "docker-compose")
    table.insert(packages, "kubectl")
    table.insert(packages, "helm")
    table.insert(packages, "terraform")
    
    -- Enable services
    table.insert(services.enabled, "docker.service")
    
elseif hostname == "gaming" then
    description = "Gaming PC setup"
    
    table.insert(packages, "steam")
    table.insert(packages, "discord")
    table.insert(packages, "gamemode")
    table.insert(packages, "mangohud")
    table.insert(packages, "flatpak:com.heroicgameslauncher.hgl")
    
elseif hostname == "laptop" then
    description = "Laptop portable setup"
    
    table.insert(packages, "tlp")
    table.insert(packages, "networkmanager")
    table.insert(packages, "bluez")
    table.insert(packages, "bluez-utils")
    
    table.insert(services.enabled, "tlp.service")
    table.insert(services.enabled, "bluetooth.service")
    table.insert(services.enabled, "NetworkManager.service")
    
elseif hostname == "server" then
    description = "Server configuration"
    
    table.insert(packages, "nginx")
    table.insert(packages, "postgresql")
    table.insert(packages, "redis")
    table.insert(packages, "fail2ban")
    
    table.insert(services.enabled, "nginx.service")
    table.insert(services.enabled, "postgresql.service")
    table.insert(services.enabled, "redis.service")
    table.insert(services.enabled, "fail2ban.service")
    table.insert(services.enabled, "sshd.service")
    
else
    dcli.log.warn("Unknown hostname: " .. hostname)
    description = "Generic host configuration"
end

return {
    description = description,
    packages = packages,
    services = services,
}
```

### Development Environment with Functions

Use functions for reusable package groups:

```lua
-- development.lua

-- Helper function to add a group of packages
local function add_packages(target, source)
    for _, pkg in ipairs(source) do
        table.insert(target, pkg)
    end
end

-- Package groups
local function python_packages()
    return {
        "python",
        "python-pip",
        "python-virtualenv",
        "python-poetry",
        "ipython",
        "python-black",
        "python-pylint",
    }
end

local function rust_packages()
    return {
        "rustup",
        "rust-analyzer",
        "cargo-watch",
        "cargo-edit",
    }
end

local function javascript_packages()
    return {
        "nodejs",
        "npm",
        "yarn",
        "typescript",
    }
end

local function docker_packages()
    return {
        "docker",
        "docker-compose",
        "docker-buildx",
    }
end

-- Build package list
local packages = {
    -- Core development tools
    "base-devel",
    "git",
    "git-lfs",
    "neovim",
    "tmux",
    "lazygit",
}

-- Always include these
add_packages(packages, python_packages())
add_packages(packages, rust_packages())

-- Conditionally include based on hostname
local hostname = dcli.system.hostname()
if hostname == "workstation" then
    add_packages(packages, javascript_packages())
    add_packages(packages, docker_packages())
end

-- Services
local services = { enabled = {}, disabled = {} }
if hostname == "workstation" then
    table.insert(services.enabled, "docker.service")
end

return {
    description = "Development environment",
    packages = packages,
    services = services,
    post_install_hook = "scripts/setup-dev.sh",
    hook_behavior = "once",
    metadata = {
        languages = { "python", "rust", "javascript" },
        author = "your-name",
    },
}
```

### Conditional Flatpak Applications

```lua
-- apps.lua
local packages = {}

-- Terminal applications (pacman)
table.insert(packages, "kitty")
table.insert(packages, "fish")
table.insert(packages, "starship")

-- GUI applications based on chassis type
local chassis = dcli.hardware.chassis_type()

if chassis == "desktop" then
    -- Full desktop applications
    table.insert(packages, "flatpak:com.spotify.Client")
    table.insert(packages, "flatpak:com.discordapp.Discord")
    table.insert(packages, "flatpak:com.slack.Slack")
    table.insert(packages, "flatpak:org.videolan.VLC")
    table.insert(packages, "flatpak:org.gimp.GIMP")
    table.insert(packages, "flatpak:org.blender.Blender")
    
elseif chassis == "laptop" then
    -- Lighter set for laptop
    table.insert(packages, "flatpak:com.spotify.Client")
    table.insert(packages, "flatpak:org.videolan.VLC")
    table.insert(packages, "flatpak:org.gnome.Evince")  -- PDF viewer
    
elseif chassis == "server" then
    -- Minimal/no GUI apps for server
    dcli.log.info("Server detected - skipping GUI applications")
end

return {
    description = "Desktop applications (" .. chassis .. ")",
    packages = packages,
}
```

### Gaming Module with Detection

```lua
-- gaming.lua
local packages = {}
local services = { enabled = {}, disabled = {} }

-- Check if we should even install gaming packages
if dcli.hardware.chassis_type() == "server" then
    dcli.log.info("Server detected - skipping gaming packages")
    return {
        description = "Gaming (skipped - server)",
        packages = {},
    }
end

-- Core gaming packages
local core_gaming = {
    "steam",
    "gamemode",
    "lib32-gamemode",
    "mangohud",
    "lib32-mangohud",
}

for _, pkg in ipairs(core_gaming) do
    table.insert(packages, pkg)
end

-- Wine and Proton support
table.insert(packages, "wine")
table.insert(packages, "wine-mono")
table.insert(packages, "wine-gecko")
table.insert(packages, "winetricks")

-- Lutris and dependencies
table.insert(packages, "lutris")

-- Flatpak gaming apps
table.insert(packages, "flatpak:com.heroicgameslauncher.hgl")
table.insert(packages, "flatpak:com.discordapp.Discord")

-- GPU-specific gaming optimizations
if dcli.hardware.has_nvidia() then
    dcli.log.info("Adding NVIDIA gaming optimizations")
    table.insert(packages, "nvidia-utils")
    table.insert(packages, "lib32-nvidia-utils")
    -- Enable persistence daemon for faster game launches
    table.insert(services.enabled, "nvidia-persistenced.service")
    
elseif dcli.hardware.has_amd_gpu() then
    dcli.log.info("Adding AMD gaming optimizations")
    table.insert(packages, "vulkan-radeon")
    table.insert(packages, "lib32-vulkan-radeon")
    -- AMD doesn't need special services
end

-- Enable gamemode service
table.insert(services.enabled, "gamemode.service")

return {
    description = "Gaming setup with " .. dcli.hardware.cpu_vendor():upper() .. " CPU",
    packages = packages,
    services = services,
    post_install_hook = "scripts/setup-gaming.sh",
    hook_behavior = "ask",
}
```

## Security & Sandboxing

Lua modules run in a sandboxed environment for security. The following restrictions apply:

### Disabled Functions

These standard Lua functions are disabled to prevent arbitrary code execution:

- `os` - No access to `os.execute()`, `os.remove()`, etc.
- `io` - No access to `io.open()`, `io.popen()`, etc.
- `loadfile` - Cannot load arbitrary Lua files
- `dofile` - Cannot execute arbitrary Lua files
- `load` - Cannot dynamically compile Lua code

### File Access Restrictions

The `dcli.file.read()` and `dcli.file.read_lines()` functions only work with specific safe paths:

- `/sys/` - Hardware information
- `/proc/` - Process and kernel info
- `/etc/os-release` - Distribution info
- `/etc/hostname` - System hostname
- `/etc/machine-id` - Machine identifier

Attempting to read other paths will result in an error:

```lua
-- This will fail with "Access denied"
local content = dcli.file.read("/etc/passwd")
```

### Safe Functions

The following standard Lua libraries remain available:

- `string` - String manipulation
- `table` - Table operations
- `math` - Mathematical functions
- `utf8` - UTF-8 support
- `pairs`, `ipairs` - Iteration
- `type`, `tostring`, `tonumber` - Type conversion
- `print` - Basic output (appears in logs)

## Debugging Lua Modules

### Enable Debug Logging

```bash
RUST_LOG=debug dcli module list
RUST_LOG=debug dcli sync --dry-run
```

### Add Debug Output

```lua
dcli.log.debug("Loading module...")
dcli.log.debug("CPU vendor: " .. dcli.hardware.cpu_vendor())
dcli.log.debug("GPU vendors: " .. table.concat(dcli.hardware.gpu_vendors(), ", "))
dcli.log.debug("Hostname: " .. dcli.system.hostname())
dcli.log.debug("Package count: " .. tostring(#packages))
```

### Validate Modules

```bash
dcli validate
```

This will check all modules (YAML and Lua) for errors.

### Common Errors

The `dcli validate` command provides detailed error messages with hints:

**Syntax Error:**
```
✗ Invalid Lua module: my-module
    ✗ [string "my-module.lua"]:5: '}' expected (to close '{' at line 3) near 'packages'
      Line: 5
      HINT: Missing closing brace '}'. Check that all tables are properly closed.
```

**Invalid Field Value:**
```
✗ Invalid Lua module: my-module
    ✗ Invalid hook_behavior: 'invalid_value'
      HINT: Valid values are: 'ask', 'once', 'always', 'skip', 'never'
```

**Missing Hook Script:**
```
✗ Invalid Lua module: my-module
    ✗ post_install_hook script not found: scripts/setup.sh
      HINT: Expected at: "/home/user/.config/arch-config/modules/scripts/setup.sh"
```

**Field Name Typos:**
```
⚠ Unknown field 'packges'. Did you mean 'packages'?
⚠ Unknown field 'descripton'. Did you mean 'description'?
```

**Invalid Structure:**
```
✗ services.enabled must be a table/array
      HINT: Use: services = { enabled = { "svc.service" } }
```

**Access Denied:**
```
Error: Access denied: /etc/passwd is not in safe path list
```
Fix: Only read from allowed paths (`/sys/`, `/proc/`, `/etc/os-release`, etc.)

**Nil Value:**
```
Error: attempt to index a nil value (global 'dcli')
      HINT: Make sure you're using the correct dcli.* API (e.g., dcli.hardware.cpu_vendor())
```

## Lua vs YAML: When to Use Each

| Scenario | Recommended | Reason |
|----------|-------------|--------|
| Simple package list | YAML | Easier to read and maintain |
| Static configuration | YAML | No logic needed |
| Hardware-dependent packages | **Lua** | Can detect and respond to hardware |
| Host-specific configs | **Lua** | Can check hostname |
| Conditional services | **Lua** | Can enable/disable based on conditions |
| Shared across machines | **Lua** | Same file works differently per machine |
| Complex package groups | **Lua** | Functions help organize |
| First-time users | YAML | Lower learning curve |

## Migration from YAML to Lua

Converting a YAML module to Lua is straightforward:

**YAML:**
```yaml
description: Gaming packages
packages:
  - steam
  - lutris
  - wine
conflicts:
  - minimal
post_install_hook: scripts/setup-gaming.sh
hook_behavior: ask
```

**Lua equivalent:**
```lua
return {
    description = "Gaming packages",
    packages = {
        "steam",
        "lutris",
        "wine",
    },
    conflicts = { "minimal" },
    post_install_hook = "scripts/setup-gaming.sh",
    hook_behavior = "ask",
}
```

Then add conditional logic as needed:

```lua
local packages = { "steam", "lutris", "wine" }

if dcli.hardware.has_nvidia() then
    table.insert(packages, "nvidia-utils")
end

return {
    description = "Gaming packages",
    packages = packages,
    conflicts = { "minimal" },
    post_install_hook = "scripts/setup-gaming.sh",
    hook_behavior = "ask",
}
```

## Directory Lua Modules

For directory modules, you can use `module.lua` instead of `module.yaml` for the manifest file. This allows dynamic configuration of which package files to load.

### Structure

```
modules/my-module/
├── module.lua              # Lua manifest (instead of module.yaml)
├── packages-core.yaml      # Always loaded
├── packages-heavy.yaml     # Conditionally loaded
├── packages-desktop.yaml   # Conditionally loaded
├── scripts/
│   └── setup.sh
└── dotfiles/
    └── nvim/
```

### module.lua Manifest

The `module.lua` file returns a manifest configuration table:

```lua
-- module.lua
local hostname = dcli.system.hostname()
local memory_mb = dcli.system.memory_total_mb()
local chassis = dcli.hardware.chassis_type()

-- Dynamically select package files
local package_files = { "packages-core.yaml" }

if memory_mb >= 16000 then
    table.insert(package_files, "packages-heavy.yaml")
end

if chassis == "desktop" then
    table.insert(package_files, "packages-desktop.yaml")
end

return {
    description = string.format("Dev tools for %s (%d MB RAM)", hostname, memory_mb),
    
    -- Dynamic package file selection
    package_files = package_files,
    
    conflicts = { "minimal" },
    
    post_install_hook = "scripts/setup.sh",
    hook_behavior = "once",
    
    -- Conditional dotfiles
    dotfiles_sync = (hostname == "workstation"),
    dotfiles = {
        { source = "nvim", target = "~/.config/nvim" },
    },
}
```

### Manifest Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | `string` | Module description |
| `package_files` | `table` | List of YAML package files to load |
| `conflicts` | `table` | Conflicting module names |
| `pre_install_hook` | `string` | Pre-install script path |
| `post_install_hook` | `string` | Post-install script path |
| `hook_behavior` | `string` | `"ask"`, `"once"`, `"always"`, `"skip"` |
| `pre_hook_behavior` | `string` | Override for pre-hook |
| `post_hook_behavior` | `string` | Override for post-hook |
| `dotfiles_sync` | `boolean` | Auto-sync dotfiles/ to ~/.config/ |
| `dotfiles` | `table` | Explicit dotfile mappings |

### Use Cases

**1. Memory-based package selection:**
```lua
local package_files = { "packages-core.yaml" }
if dcli.system.memory_total_mb() >= 32000 then
    table.insert(package_files, "packages-heavy-ide.yaml")
end
```

**2. Architecture-specific packages:**
```lua
local package_files = { "packages-base.yaml" }
if dcli.system.arch() == "x86_64" then
    table.insert(package_files, "packages-x86.yaml")
end
```

**3. Host-specific configurations:**
```lua
local hostname = dcli.system.hostname()
local package_files = { "packages-core.yaml" }

if hostname == "workstation" then
    table.insert(package_files, "packages-workstation.yaml")
elseif hostname == "laptop" then
    table.insert(package_files, "packages-laptop.yaml")
end
```

### module.lua vs module.yaml

| Feature | module.yaml | module.lua |
|---------|-------------|------------|
| Package file list | Static | Dynamic |
| Description | Static | Can include runtime info |
| Dotfiles sync | Static boolean | Conditional |
| Complexity | Simple | More powerful |

**Use `module.lua` when:**
- You need conditional package file selection
- Different machines need different subsets of packages
- Description should include system information
- Dotfiles should only sync on certain hosts

**Use `module.yaml` when:**
- Configuration is static
- Simple module with fixed package files
- No conditional logic needed

## Best Practices

1. **Start with YAML, convert to Lua when needed** - Don't over-engineer simple modules

2. **Use descriptive logging** - Help future you understand what the module is doing
   ```lua
   dcli.log.info("Detected " .. #gpus .. " GPU(s)")
   ```

3. **Handle edge cases** - What if no GPU is detected? What if hostname is unknown?
   ```lua
   if #packages == 0 then
       dcli.log.warn("No packages selected - check hardware detection")
   end
   ```

4. **Keep modules focused** - One module per concern (GPU drivers, power management, etc.)

5. **Use functions for reusability** - Extract common package groups into functions

6. **Document your modules** - Use comments and meaningful descriptions
   ```lua
   -- This module auto-detects GPU hardware and installs appropriate drivers
   -- Supports: NVIDIA (proprietary), AMD (mesa), Intel (mesa)
   ```

7. **Test on multiple machines** - If using hardware detection, verify on different hardware

8. **Use dry-run first** - `dcli sync --dry-run` to see what would be installed

## File Locations

- **Modules Directory:** `~/.config/arch-config/modules/`
- **Module file:** `~/.config/arch-config/modules/my-module.lua`
- **Hook scripts:** `~/.config/arch-config/modules/scripts/` (relative to module)

## Related Documentation

- [DIRECTORY-MODULES.md](DIRECTORY-MODULES.md) - Directory-based module format
- [README.md](README.md) - Main dcli documentation
- [SERVICES.md](SERVICES.md) - Services management