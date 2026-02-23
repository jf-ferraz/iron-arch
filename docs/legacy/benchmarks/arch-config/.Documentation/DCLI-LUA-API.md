# dcli Lua API Reference

Complete reference for all Lua APIs available in dcli modules and host configurations.

## Quick Reference

| Namespace | Purpose | Example |
|-----------|---------|---------|
| `dcli.hardware` | Hardware detection | `dcli.hardware.has_nvidia()` |
| `dcli.system` | System information | `dcli.system.hostname()` |
| `dcli.package` | Package queries | `dcli.package.is_installed("vim")` |
| `dcli.file` | File operations (sandboxed) | `dcli.file.exists("/path")` |
| `dcli.env` | Environment variables | `dcli.env.home()` |
| `dcli.util` | Utility functions | `dcli.util.contains(table, value)` |
| `dcli.log` | Logging | `dcli.log.info("message")` |
| `dcli.service` | Systemd service detection | `dcli.service.is_active("sshd")` |
| `dcli.power` | Power management | `dcli.power.on_battery()` |
| `dcli.security` | Security features | `dcli.security.has_secureboot()` |
| `dcli.desktop` | Desktop environment | `dcli.desktop.is_wayland()` |
| `dcli.boot` | Boot configuration | `dcli.boot.is_uefi()` |
| `dcli.network` | Network detection | `dcli.network.has_wifi()` |
| `dcli.audio` | Audio system | `dcli.audio.has_pipewire()` |
| `dcli.storage` | Storage devices | `dcli.storage.has_ssd()` |

---

## dcli.hardware

Hardware detection functions for conditional configuration.

### Functions

#### `cpu_vendor()`

Returns the CPU manufacturer.

**Returns:** `string` - `"intel"`, `"amd"`, or `"unknown"`

**Example:**
```lua
local cpu = dcli.hardware.cpu_vendor()
if cpu == "intel" then
    table.insert(packages, "intel-ucode")
elseif cpu == "amd" then
    table.insert(packages, "amd-ucode")
end
```

---

#### `gpu_vendors()`

Returns an array of all detected GPU vendors.

**Returns:** `table` - Array of strings like `{"nvidia", "amd"}` or `{"intel"}`

**Example:**
```lua
local gpus = dcli.hardware.gpu_vendors()
for _, vendor in ipairs(gpus) do
    dcli.log.info("Found GPU: " .. vendor)
end

-- Check for multi-GPU setup
if #gpus > 1 then
    dcli.log.warn("Multi-GPU system detected")
end
```

---

#### `has_nvidia()`

Check if an NVIDIA GPU is present.

**Returns:** `boolean`

**Example:**
```lua
if dcli.hardware.has_nvidia() then
    table.insert(packages, "nvidia")
    table.insert(packages, "nvidia-utils")
    table.insert(packages, "nvidia-settings")
end
```

---

#### `has_amd_gpu()`

Check if an AMD GPU is present.

**Returns:** `boolean`

**Example:**
```lua
if dcli.hardware.has_amd_gpu() then
    table.insert(packages, "mesa")
    table.insert(packages, "vulkan-radeon")
    table.insert(packages, "libva-mesa-driver")
end
```

---

#### `has_intel_gpu()`

Check if an Intel integrated GPU is present.

**Returns:** `boolean`

**Example:**
```lua
if dcli.hardware.has_intel_gpu() then
    table.insert(packages, "mesa")
    table.insert(packages, "vulkan-intel")
    table.insert(packages, "intel-media-driver")
end
```

---

#### `is_laptop()`

Check if the system is a laptop.

**Returns:** `boolean`

**Example:**
```lua
if dcli.hardware.is_laptop() then
    table.insert(packages, "tlp")
    table.insert(packages, "powertop")
    table.insert(services.enabled, "tlp.service")
end
```

---

#### `has_battery()`

Check if a battery is present in the system.

**Returns:** `boolean`

**Example:**
```lua
if dcli.hardware.has_battery() then
    dcli.log.info("Battery detected - enabling power management")
    table.insert(packages, "acpi")
end
```

---

#### `chassis_type()`

Get the system chassis type.

**Returns:** `string` - `"desktop"`, `"laptop"`, `"server"`, `"tablet"`, or `"unknown"`

**Example:**
```lua
local chassis = dcli.hardware.chassis_type()

if chassis == "server" then
    -- Skip GUI packages on servers
    dcli.log.info("Server detected - skipping GUI packages")
elseif chassis == "laptop" then
    table.insert(packages, "laptop-mode-tools")
end
```

---

## dcli.system

System information and configuration queries.

### Functions

#### `hostname()`

Get the system hostname.

**Returns:** `string`

**Example:**
```lua
local hostname = dcli.system.hostname()

if hostname == "workstation" then
    table.insert(enabled_modules, "development")
elseif hostname == "gaming-pc" then
    table.insert(enabled_modules, "gaming")
end
```

---

#### `kernel_version()`

Get the running kernel version.

**Returns:** `string` - Example: `"6.7.1-arch1-1"`

**Example:**
```lua
local kernel = dcli.system.kernel_version()
dcli.log.info("Running kernel: " .. kernel)

-- Check kernel version
if dcli.util.version_gte(kernel, "6.0") then
    dcli.log.info("Modern kernel (6.0+) detected")
end
```

---

#### `arch()`

Get the system architecture.

**Returns:** `string` - `"x86_64"`, `"aarch64"`, etc.

**Example:**
```lua
local arch = dcli.system.arch()

if arch == "x86_64" then
    -- x86-specific packages
    table.insert(packages, "lib32-mesa")
end
```

---

#### `os()`

Get the operating system.

**Returns:** `string` - `"linux"`, `"macos"`, `"windows"`

**Example:**
```lua
if dcli.system.os() == "linux" then
    dcli.log.info("Running on Linux")
end
```

---

#### `distro()`

Get the Linux distribution ID.

**Returns:** `string` - `"arch"`, `"endeavouros"`, `"manjaro"`, etc.

**Example:**
```lua
local distro = dcli.system.distro()

if distro == "endeavouros" then
    dcli.log.info("EndeavourOS detected")
    table.insert(packages, "eos-update-notifier")
end
```

---

#### `distro_name()`

Get the full distribution name.

**Returns:** `string` - Example: `"Arch Linux"`, `"EndeavourOS"`

**Example:**
```lua
local distro_name = dcli.system.distro_name()
dcli.log.info("Distribution: " .. distro_name)
```

---

#### `distro_version()`

Get the distribution version.

**Returns:** `string` - Version string or `"rolling"` for rolling release distros

**Example:**
```lua
local version = dcli.system.distro_version()
if version == "rolling" then
    dcli.log.info("Rolling release distribution")
end
```

---

#### `memory_total_mb()`

Get total system RAM in megabytes.

**Returns:** `number` - RAM in MB

**Example:**
```lua
local memory_mb = dcli.system.memory_total_mb()
local memory_gb = math.floor(memory_mb / 1024)

dcli.log.info(string.format("System has %d GB RAM", memory_gb))

if memory_mb >= 32000 then
    -- High RAM - enable heavy packages
    table.insert(packages, "flatpak:org.blender.Blender")
    table.insert(enabled_modules, "virtualization")
end
```

---

#### `cpu_cores()`

Get the number of CPU cores.

**Returns:** `number`

**Example:**
```lua
local cores = dcli.system.cpu_cores()

dcli.log.info(string.format("CPU cores: %d", cores))

if cores >= 8 then
    -- Many cores - enable parallel builds
    dcli.log.info("High core count - enabling parallel compilation")
end
```

---

## dcli.package

Query installed packages and availability.

### Functions

#### `is_installed(name)`

Check if a pacman package is installed.

**Parameters:**
- `name` (string) - Package name

**Returns:** `boolean`

**Example:**
```lua
if not dcli.package.is_installed("docker") then
    table.insert(packages, "docker")
    table.insert(packages, "docker-compose")
end

-- Check for conflicts
if dcli.package.is_installed("pulseaudio") then
    dcli.log.warn("PulseAudio detected - consider migrating to PipeWire")
end
```

---

#### `version(name)`

Get the version of an installed package.

**Parameters:**
- `name` (string) - Package name

**Returns:** `string` or `nil` - Version string if installed, `nil` otherwise

**Example:**
```lua
local kernel_ver = dcli.package.version("linux")
if kernel_ver then
    dcli.log.info("Kernel version: " .. kernel_ver)
end
```

---

#### `is_available(name)`

Check if a package is available in the repositories.

**Parameters:**
- `name` (string) - Package name

**Returns:** `boolean`

**Example:**
```lua
if dcli.package.is_available("neovim-nightly") then
    table.insert(packages, "neovim-nightly")
else
    table.insert(packages, "neovim")
end
```

---

#### `repo(name)`

Get the repository a package comes from.

**Parameters:**
- `name` (string) - Package name

**Returns:** `string` or `nil` - Repository name (`"core"`, `"extra"`, `"multilib"`, etc.) or `nil`

**Example:**
```lua
local repo = dcli.package.repo("firefox")
if repo then
    dcli.log.info("Firefox is from: " .. repo)
end
```

---

#### `is_foreign(name)`

Check if a package is from AUR or manually installed.

**Parameters:**
- `name` (string) - Package name

**Returns:** `boolean` - `true` if from AUR or manual install

**Example:**
```lua
if dcli.package.is_foreign("yay") then
    dcli.log.info("yay is from AUR")
end
```

---

#### `list_installed()`

Get a list of all installed packages.

**Returns:** `table` - Array of package names

**Example:**
```lua
local installed = dcli.package.list_installed()
dcli.log.info(string.format("Total packages installed: %d", #installed))
```

---

#### `list_explicit()`

Get a list of explicitly installed packages.

**Returns:** `table` - Array of package names

**Example:**
```lua
local explicit = dcli.package.list_explicit()
dcli.log.info(string.format("Explicitly installed: %d packages", #explicit))
```

---

#### `flatpak_installed(id)`

Check if a Flatpak application is installed.

**Parameters:**
- `id` (string) - Flatpak application ID

**Returns:** `boolean`

**Example:**
```lua
if not dcli.package.flatpak_installed("com.spotify.Client") then
    table.insert(packages, "flatpak:com.spotify.Client")
end
```

---

#### `flatpak_version(id)`

Get the version of an installed Flatpak.

**Parameters:**
- `id` (string) - Flatpak application ID

**Returns:** `string` or `nil`

**Example:**
```lua
local version = dcli.package.flatpak_version("com.spotify.Client")
if version then
    dcli.log.info("Spotify version: " .. version)
end
```

---

#### `aur_available(name)`

Check if a package is available in the AUR.

**Parameters:**
- `name` (string) - Package name

**Returns:** `boolean`

**Example:**
```lua
if dcli.package.aur_available("vscodium-bin") then
    table.insert(packages, "vscodium-bin")
end
```

---

## dcli.file

File system operations (sandboxed for security).

**Security Note:** File read operations are restricted to safe paths:
- `/sys/` - Hardware information
- `/proc/` - Process and kernel info
- `/etc/os-release` - Distribution info
- `/etc/hostname` - System hostname
- `/etc/machine-id` - Machine identifier

### Functions

#### `exists(path)`

Check if a file or directory exists.

**Parameters:**
- `path` (string) - File path

**Returns:** `boolean`

**Example:**
```lua
if dcli.file.exists("/etc/docker/daemon.json") then
    dcli.log.info("Docker already configured")
end
```

---

#### `is_file(path)`

Check if a path is a file.

**Parameters:**
- `path` (string) - File path

**Returns:** `boolean`

**Example:**
```lua
if dcli.file.is_file("/etc/pacman.conf") then
    dcli.log.info("pacman.conf exists")
end
```

---

#### `is_dir(path)`

Check if a path is a directory.

**Parameters:**
- `path` (string) - Directory path

**Returns:** `boolean`

**Example:**
```lua
if dcli.file.is_dir("/sys/class/power_supply") then
    dcli.log.info("Power supply directory exists")
end
```

---

#### `read(path)`

Read a file's contents (sandboxed paths only).

**Parameters:**
- `path` (string) - File path (must be in safe list)

**Returns:** `string` or `nil` - File contents or `nil` if not found/denied

**Example:**
```lua
local os_release = dcli.file.read("/etc/os-release")
if os_release and os_release:match("EndeavourOS") then
    dcli.log.info("Running on EndeavourOS")
end
```

---

#### `read_lines(path)`

Read a file as an array of lines (sandboxed paths only).

**Parameters:**
- `path` (string) - File path (must be in safe list)

**Returns:** `table` or `nil` - Array of lines or `nil`

**Example:**
```lua
local meminfo = dcli.file.read_lines("/proc/meminfo")
if meminfo then
    for _, line in ipairs(meminfo) do
        if line:match("^MemTotal:") then
            dcli.log.info("Memory: " .. line)
            break
        end
    end
end
```

---

## dcli.env

Environment variables and XDG directories.

### Functions

#### `get(name)`

Get an environment variable value.

**Parameters:**
- `name` (string) - Variable name

**Returns:** `string` or `nil`

**Example:**
```lua
local editor = dcli.env.get("EDITOR")
if editor then
    dcli.log.info("Default editor: " .. editor)
end

if dcli.env.get("WAYLAND_DISPLAY") then
    dcli.log.info("Wayland session detected")
    table.insert(packages, "wl-clipboard")
end
```

---

#### `home()`

Get the user's home directory.

**Returns:** `string` - Home directory path

**Example:**
```lua
local home = dcli.env.home()
local config_path = home .. "/.config/nvim"

if dcli.file.exists(config_path) then
    dcli.log.info("Neovim config exists")
end
```

---

#### `user()`

Get the current username.

**Returns:** `string`

**Example:**
```lua
local user = dcli.env.user()
dcli.log.info("Setting up for user: " .. user)
```

---

#### `config_dir()`

Get the XDG config directory.

**Returns:** `string` - Usually `~/.config`

**Example:**
```lua
local config_dir = dcli.env.config_dir()
dcli.log.info("Config directory: " .. config_dir)
```

---

#### `data_dir()`

Get the XDG data directory.

**Returns:** `string` - Usually `~/.local/share`

**Example:**
```lua
local data_dir = dcli.env.data_dir()
```

---

#### `cache_dir()`

Get the XDG cache directory.

**Returns:** `string` - Usually `~/.cache`

**Example:**
```lua
local cache_dir = dcli.env.cache_dir()
```

---

#### `shell()`

Get the user's default shell.

**Returns:** `string` - Shell path

**Example:**
```lua
local shell = dcli.env.shell()
if shell:match("fish$") then
    dcli.log.info("Using fish shell")
    table.insert(packages, "fisher")
end
```

---

## dcli.util

Utility functions for common operations.

### Functions

#### `contains(table, value)`

Check if an array contains a value.

**Parameters:**
- `table` (table) - Array to search
- `value` (any) - Value to find

**Returns:** `boolean`

**Example:**
```lua
local packages = {"git", "neovim", "htop"}

if dcli.util.contains(packages, "neovim") then
    dcli.log.info("Neovim is in the list")
end
```

---

#### `keys(table)`

Get an array of table keys.

**Parameters:**
- `table` (table) - Input table

**Returns:** `table` - Array of keys

**Example:**
```lua
local config = {host = "desktop", ram = 16}
local keys = dcli.util.keys(config)
-- keys = {"host", "ram"}
```

---

#### `values(table)`

Get an array of table values.

**Parameters:**
- `table` (table) - Input table

**Returns:** `table` - Array of values

**Example:**
```lua
local config = {host = "desktop", ram = 16}
local values = dcli.util.values(config)
-- values = {"desktop", 16}
```

---

#### `merge(t1, t2)`

Merge two tables (t2 values override t1).

**Parameters:**
- `t1` (table) - First table
- `t2` (table) - Second table (takes precedence)

**Returns:** `table` - New merged table

**Example:**
```lua
local base = {editor = "vim", shell = "bash"}
local custom = {editor = "nvim"}
local merged = dcli.util.merge(base, custom)
-- merged = {editor = "nvim", shell = "bash"}
```

---

#### `extend(target, source)`

Append source array items to target array (modifies target).

**Parameters:**
- `target` (table) - Target array (modified in-place)
- `source` (table) - Source array

**Returns:** `table` - The modified target table

**Example:**
```lua
local packages = {"git", "vim"}
dcli.util.extend(packages, {"htop", "ripgrep"})
-- packages = {"git", "vim", "htop", "ripgrep"}
```

---

#### `split(str, delim)`

Split a string by delimiter.

**Parameters:**
- `str` (string) - String to split
- `delim` (string) - Delimiter

**Returns:** `table` - Array of strings

**Example:**
```lua
local path = "/usr/local/bin"
local parts = dcli.util.split(path, "/")
-- parts = {"", "usr", "local", "bin"}
```

---

#### `trim(str)`

Remove leading and trailing whitespace.

**Parameters:**
- `str` (string) - String to trim

**Returns:** `string`

**Example:**
```lua
local clean = dcli.util.trim("  hello  ")
-- clean = "hello"
```

---

#### `starts_with(str, prefix)`

Check if a string starts with a prefix.

**Parameters:**
- `str` (string) - String to check
- `prefix` (string) - Prefix to find

**Returns:** `boolean`

**Example:**
```lua
if dcli.util.starts_with(package_name, "lib32-") then
    dcli.log.info("32-bit library package")
end
```

---

#### `ends_with(str, suffix)`

Check if a string ends with a suffix.

**Parameters:**
- `str` (string) - String to check
- `suffix` (string) - Suffix to find

**Returns:** `boolean`

**Example:**
```lua
if dcli.util.ends_with(filename, ".yaml") then
    dcli.log.info("YAML file detected")
end
```

---

#### `version_compare(v1, v2)`

Compare two version strings.

**Parameters:**
- `v1` (string) - First version
- `v2` (string) - Second version

**Returns:** `number` - `-1` if v1 < v2, `0` if equal, `1` if v1 > v2

**Example:**
```lua
local result = dcli.util.version_compare("6.7.1", "6.0.0")
-- result = 1 (6.7.1 > 6.0.0)
```

---

#### `version_gte(v1, v2)`

Check if v1 >= v2.

**Parameters:**
- `v1` (string) - First version
- `v2` (string) - Second version

**Returns:** `boolean`

**Example:**
```lua
local kernel = dcli.package.version("linux") or "0"
if dcli.util.version_gte(kernel, "6.0") then
    dcli.log.info("Kernel 6.0 or newer")
end
```

---

#### `version_gt(v1, v2)`

Check if v1 > v2.

**Returns:** `boolean`

---

#### `version_lte(v1, v2)`

Check if v1 <= v2.

**Returns:** `boolean`

---

#### `version_lt(v1, v2)`

Check if v1 < v2.

**Returns:** `boolean`

---

## dcli.log

Logging functions for debugging and information.

### Functions

#### `info(msg)`

Log an informational message.

**Parameters:**
- `msg` (string) - Message to log

**Example:**
```lua
dcli.log.info("Loading gaming module...")
```

---

#### `warn(msg)`

Log a warning message.

**Parameters:**
- `msg` (string) - Warning message

**Example:**
```lua
dcli.log.warn("NVIDIA detected but nouveau might conflict")
```

---

#### `debug(msg)`

Log a debug message (visible with `RUST_LOG=debug`).

**Parameters:**
- `msg` (string) - Debug message

**Example:**
```lua
dcli.log.debug("Package count: " .. #packages)
```

---

#### `error(msg)`

Log an error message.

**Parameters:**
- `msg` (string) - Error message

**Example:**
```lua
dcli.log.error("Failed to detect GPU")
```

---

## Complete Examples

### Hardware-Aware Module

```lua
-- modules/hardware.lua
local packages = {}
local services = {enabled = {}, disabled = {}}

-- CPU microcode
local cpu = dcli.hardware.cpu_vendor()
if cpu == "intel" then
    table.insert(packages, "intel-ucode")
elseif cpu == "amd" then
    table.insert(packages, "amd-ucode")
end

-- GPU drivers
if dcli.hardware.has_nvidia() then
    dcli.log.info("NVIDIA GPU detected")
    dcli.util.extend(packages, {
        "nvidia",
        "nvidia-utils",
        "nvidia-settings",
        "lib32-nvidia-utils",
    })
elseif dcli.hardware.has_amd_gpu() then
    dcli.log.info("AMD GPU detected")
    dcli.util.extend(packages, {
        "mesa",
        "vulkan-radeon",
        "lib32-vulkan-radeon",
        "libva-mesa-driver",
    })
end

-- Laptop power management
if dcli.hardware.is_laptop() then
    dcli.log.info("Laptop detected")
    dcli.util.extend(packages, {"tlp", "powertop", "acpi"})
    table.insert(services.enabled, "tlp.service")
    table.insert(services.disabled, "power-profiles-daemon.service")
end

return {
    description = "Hardware drivers (auto-detected)",
    packages = packages,
    services = services,
}
```

### Dynamic Host Configuration

```lua
-- config.lua
local hostname = dcli.system.hostname()
local memory_mb = dcli.system.memory_total_mb()
local is_laptop = dcli.hardware.is_laptop()

local enabled_modules = {"base", "hardware"}

-- Add modules based on RAM
if memory_mb >= 16000 then
    table.insert(enabled_modules, "development")
end

if memory_mb >= 32000 then
    table.insert(enabled_modules, "virtualization")
end

-- Host-specific modules
local host_modules = {
    workstation = {"docker", "kubernetes"},
    ["gaming-pc"] = {"gaming", "streaming"},
    laptop = {"office"},
}

if host_modules[hostname] then
    dcli.util.extend(enabled_modules, host_modules[hostname])
end

return {
    host = hostname,
    description = string.format("%s - %d GB RAM", hostname, math.floor(memory_mb / 1024)),
    enabled_modules = enabled_modules,
    module_processing = is_laptop and "sequential" or "parallel",
    flatpak_scope = "user",
}
```

### Conditional Package Selection

```lua
-- modules/desktop.lua
local packages = {}

-- Check for existing packages
if not dcli.package.is_installed("firefox") then
    table.insert(packages, "firefox")
end

-- Wayland vs X11
if dcli.env.get("WAYLAND_DISPLAY") then
    dcli.log.info("Wayland session - adding wl-clipboard")
    table.insert(packages, "wl-clipboard")
else
    dcli.log.info("X11 session - adding xclip")
    table.insert(packages, "xclip")
end

-- Different apps based on chassis
local chassis = dcli.hardware.chassis_type()
if chassis == "laptop" then
    -- Lighter apps for laptop
    table.insert(packages, "flatpak:org.gnome.Evince")
elseif chassis == "desktop" then
    -- Full suite for desktop
    dcli.util.extend(packages, {
        "flatpak:org.gimp.GIMP",
        "flatpak:org.blender.Blender",
        "flatpak:com.spotify.Client",
    })
end

return {
    description = "Desktop applications",
    packages = packages,
}
```

---

## Best Practices

1. **Always check return values:**
   ```lua
   local version = dcli.package.version("linux")
   if version then
       -- Safe to use version
   end
   ```

2. **Use logging for debugging:**
   ```lua
   dcli.log.info("Detected " .. #packages .. " packages")
   dcli.log.debug("CPU vendor: " .. dcli.hardware.cpu_vendor())
   ```

3. **Handle edge cases:**
   ```lua
   local hostname = dcli.system.hostname() or "unknown"
   ```

4. **Use utility functions:**
   ```lua
   -- Instead of manual loops
   if dcli.util.contains(enabled_modules, "docker") then
       table.insert(services.enabled, "docker.service")
   end
   ```

5. **Keep file reads sandboxed:**
   ```lua
   -- This works (safe path)
   local os_release = dcli.file.read("/etc/os-release")
   
   -- This fails (not in safe list)
   local passwd = dcli.file.read("/etc/passwd")  -- Access denied
   ```

---

## dcli.service

Systemd service detection and management queries.

### Functions

#### `is_enabled(name)`

Check if a systemd service is enabled.

**Parameters:**
- `name` (`string`) - Service name (e.g., `"sshd.service"` or `"sshd"`)

**Returns:** `boolean`

**Example:**
```lua
if dcli.service.is_enabled("docker.service") then
    dcli.log.info("Docker is enabled")
end
```

---

#### `is_active(name)`

Check if a systemd service is currently active/running.

**Parameters:**
- `name` (`string`) - Service name

**Returns:** `boolean`

**Example:**
```lua
if dcli.service.is_active("NetworkManager") then
    -- Network Manager is running
    table.insert(packages, "nm-connection-editor")
end
```

---

#### `is_running(name)`

Alias for `is_active()`.

**Parameters:**
- `name` (`string`) - Service name

**Returns:** `boolean`

---

#### `exists(name)`

Check if a service unit file exists.

**Parameters:**
- `name` (`string`) - Service name

**Returns:** `boolean`

**Example:**
```lua
if dcli.service.exists("bluetooth.service") then
    -- Bluetooth service is available
end
```

---

#### `status(name)`

Get the current status of a service.

**Parameters:**
- `name` (`string`) - Service name

**Returns:** `string` - One of: `"active"`, `"inactive"`, `"failed"`, `"activating"`, `"deactivating"`, `"unknown"`

**Example:**
```lua
local status = dcli.service.status("sshd")
if status == "failed" then
    dcli.log.warn("SSH service has failed")
end
```

---

#### `list_enabled()`

Get a list of all enabled services.

**Returns:** `table` - Array of enabled service names

**Example:**
```lua
local enabled = dcli.service.list_enabled()
for _, svc in ipairs(enabled) do
    dcli.log.info("Enabled: " .. svc)
end
```

---

#### `list_active()`

Get a list of all currently active services.

**Returns:** `table` - Array of active service names

---

#### `list_failed()`

Get a list of all failed services.

**Returns:** `table` - Array of failed service names

**Example:**
```lua
local failed = dcli.service.list_failed()
if #failed > 0 then
    dcli.log.warn("Failed services detected: " .. table.concat(failed, ", "))
end
```

---

#### `is_user_service(name)`

Check if a user service (systemd --user) is active.

**Parameters:**
- `name` (`string`) - Service name

**Returns:** `boolean`

---

## dcli.power

Power management and battery status detection.

### Functions

#### `on_battery()`

Check if the system is currently running on battery power.

**Returns:** `boolean`

**Example:**
```lua
if dcli.power.on_battery() then
    -- Enable power saving features
    table.insert(packages, "tlp")
    table.insert(packages, "powertop")
end
```

---

#### `on_ac()`

Check if the system is connected to AC power.

**Returns:** `boolean`

---

#### `battery_percent()`

Get current battery charge level.

**Returns:** `number | nil` - Battery percentage (0-100) or `nil` if no battery

**Example:**
```lua
local battery = dcli.power.battery_percent()
if battery and battery < 20 then
    dcli.log.warn("Low battery: " .. battery .. "%")
end
```

---

#### `battery_status()`

Get current battery charging status.

**Returns:** `string` - One of: `"charging"`, `"discharging"`, `"full"`, `"unknown"`

**Example:**
```lua
local status = dcli.power.battery_status()
if status == "charging" then
    dcli.log.info("Battery is charging")
end
```

---

#### `has_suspend()`

Check if system supports suspend to RAM.

**Returns:** `boolean`

---

#### `has_hibernate()`

Check if system supports hibernate to disk.

**Returns:** `boolean`

**Example:**
```lua
if dcli.power.has_hibernate() then
    -- Ensure swap is configured for hibernation
    dcli.log.info("Hibernation is supported")
end
```

---

#### `cpu_governor()`

Get the current CPU frequency scaling governor.

**Returns:** `string` - Governor name like `"performance"`, `"powersave"`, `"schedutil"`, or `"unknown"`

**Example:**
```lua
local governor = dcli.power.cpu_governor()
if governor == "powersave" then
    dcli.log.info("CPU is in power-saving mode")
end
```

---

#### `available_governors()`

Get list of available CPU governors.

**Returns:** `table` - Array of governor names

**Example:**
```lua
local governors = dcli.power.available_governors()
if dcli.util.contains(governors, "performance") then
    -- Can switch to performance mode
end
```

---

#### `supports_turbo()`

Check if CPU supports turbo boost / boost technology.

**Returns:** `boolean`

---

#### `turbo_enabled()`

Check if turbo boost is currently enabled.

**Returns:** `boolean`

---

## dcli.security

Security features and hardening detection.

### Functions

#### `has_selinux()`

Check if SELinux is available on the system.

**Returns:** `boolean`

---

#### `selinux_enabled()`

Check if SELinux is currently enabled and enforcing.

**Returns:** `boolean`

**Example:**
```lua
if dcli.security.selinux_enabled() then
    table.insert(packages, "selinux-python")
    table.insert(packages, "selinux-policy")
end
```

---

#### `has_apparmor()`

Check if AppArmor is available.

**Returns:** `boolean`

---

#### `apparmor_enabled()`

Check if AppArmor is currently enabled.

**Returns:** `boolean`

**Example:**
```lua
if dcli.security.apparmor_enabled() then
    table.insert(packages, "apparmor")
    table.insert(services.enabled, "apparmor.service")
end
```

---

#### `has_secureboot()`

Check if Secure Boot is supported by the system.

**Returns:** `boolean`

---

#### `secureboot_enabled()`

Check if Secure Boot is currently enabled.

**Returns:** `boolean`

**Example:**
```lua
if dcli.security.secureboot_enabled() then
    dcli.log.info("Secure Boot is enabled")
    -- Use signed bootloader and kernel
    table.insert(packages, "shim-signed")
end
```

---

#### `has_tpm()`

Check if a TPM (Trusted Platform Module) is present.

**Returns:** `boolean`

---

#### `tpm_version()`

Get TPM version.

**Returns:** `string | nil` - `"1.2"`, `"2.0"`, or `nil` if no TPM

**Example:**
```lua
local tpm = dcli.security.tpm_version()
if tpm == "2.0" then
    table.insert(packages, "tpm2-tools")
    table.insert(packages, "tpm2-tss")
end
```

---

#### `firewall_active()`

Check if any firewall is currently active.

**Returns:** `boolean`

---

#### `firewall_type()`

Detect which firewall system is in use.

**Returns:** `string` - One of: `"ufw"`, `"firewalld"`, `"iptables"`, `"nftables"`, `"none"`

**Example:**
```lua
local fw = dcli.security.firewall_type()
if fw == "ufw" then
    table.insert(packages, "ufw")
    table.insert(services.enabled, "ufw.service")
elseif fw == "firewalld" then
    table.insert(packages, "firewalld")
end
```

---

#### `has_luks()`

Check if any LUKS encrypted partitions are in use.

**Returns:** `boolean`

**Example:**
```lua
if dcli.security.has_luks() then
    -- System uses encryption
    table.insert(packages, "cryptsetup")
end
```

---

#### `kernel_lockdown()`

Get kernel lockdown mode status.

**Returns:** `string` - One of: `"none"`, `"integrity"`, `"confidentiality"`

---

## dcli.desktop

Desktop environment and display server detection.

### Functions

#### `environment()`

Detect the desktop environment.

**Returns:** `string` - Desktop environment name like `"kde"`, `"gnome"`, `"xfce"`, `"hyprland"`, `"sway"`, `"i3"`, or `"unknown"`

**Example:**
```lua
local de = dcli.desktop.environment()
if de == "kde" then
    table.insert(packages, "plasma-meta")
    table.insert(packages, "kde-applications-meta")
elseif de == "gnome" then
    table.insert(packages, "gnome")
    table.insert(packages, "gnome-extra")
elseif de == "hyprland" then
    table.insert(packages, "hyprland")
    table.insert(packages, "waybar")
end
```

---

#### `display_server()`

Detect the display server in use.

**Returns:** `string` - `"wayland"`, `"x11"`, or `"unknown"`

---

#### `is_wayland()`

Check if running on Wayland.

**Returns:** `boolean`

**Example:**
```lua
if dcli.desktop.is_wayland() then
    -- Use Wayland-specific tools
    table.insert(packages, "wl-clipboard")
    table.insert(packages, "xdg-desktop-portal-wlr")
end
```

---

#### `is_x11()`

Check if running on X11.

**Returns:** `boolean`

**Example:**
```lua
if dcli.desktop.is_x11() then
    table.insert(packages, "xclip")
    table.insert(packages, "xdotool")
end
```

---

#### `window_manager()`

Detect the window manager.

**Returns:** `string` - Window manager name like `"kwin"`, `"mutter"`, `"i3"`, `"sway"`, `"hyprland"`, or `"unknown"`

---

#### `session_type()`

Get the session type.

**Returns:** `string` - `"x11"`, `"wayland"`, `"tty"`, or `"unknown"`

---

#### `has_display()`

Check if a display server is available.

**Returns:** `boolean`

---

#### `compositor()`

Detect the compositor in use.

**Returns:** `string | nil` - Compositor name like `"picom"`, `"kwin_wayland"`, `"Hyprland"`, or `nil`

**Example:**
```lua
local comp = dcli.desktop.compositor()
if comp == "picom" then
    table.insert(packages, "picom")
end
```

---

#### `theme()`

Get the current desktop theme.

**Returns:** `string | nil` - Theme name or `nil`

---

#### `icon_theme()`

Get the current icon theme.

**Returns:** `string | nil` - Icon theme name or `nil`

---

#### `screen_resolution()`

Get the current screen resolution.

**Returns:** `string | nil` - Resolution like `"1920x1080"` or `nil`

---

## dcli.boot

Bootloader and boot configuration detection.

### Functions

#### `bootloader()`

Detect which bootloader is being used.

**Returns:** `string` - Bootloader name: `"grub"`, `"systemd-boot"`, `"refind"`, `"lilo"`, `"syslinux"`, or `"unknown"`

**Example:**
```lua
local bootloader = dcli.boot.bootloader()
if bootloader == "grub" then
    table.insert(packages, "grub")
    table.insert(packages, "os-prober")
elseif bootloader == "systemd-boot" then
    table.insert(packages, "systemd")
end
```

---

#### `is_uefi()`

Check if system booted in UEFI mode.

**Returns:** `boolean`

**Example:**
```lua
if dcli.boot.is_uefi() then
    table.insert(packages, "efibootmgr")
else
    -- BIOS/Legacy boot
    dcli.log.info("System uses BIOS boot")
end
```

---

#### `is_bios()`

Check if system booted in BIOS/Legacy mode.

**Returns:** `boolean`

---

#### `init_system()`

Detect the init system.

**Returns:** `string` - Init system name: `"systemd"`, `"openrc"`, `"runit"`, `"s6"`, `"sysvinit"`, or `"unknown"`

**Example:**
```lua
local init = dcli.boot.init_system()
if init == "systemd" then
    -- Can use systemd features
elseif init == "openrc" then
    table.insert(packages, "openrc")
end
```

---

#### `kernel_params()`

Get all kernel boot parameters.

**Returns:** `table` - Array of kernel parameters

**Example:**
```lua
local params = dcli.boot.kernel_params()
for _, param in ipairs(params) do
    dcli.log.debug("Kernel param: " .. param)
end
```

---

#### `has_kernel_param(param)`

Check if a specific kernel parameter is set.

**Parameters:**
- `param` (`string`) - Parameter name (e.g., `"quiet"`, `"splash"`)

**Returns:** `boolean`

**Example:**
```lua
if dcli.boot.has_kernel_param("quiet") then
    dcli.log.info("Kernel boots in quiet mode")
end

if dcli.boot.has_kernel_param("nvidia-drm.modeset") then
    dcli.log.info("NVIDIA DRM modesetting enabled")
end
```

---

#### `efi_vars_supported()`

Check if EFI variables are accessible.

**Returns:** `boolean`

---

#### `boot_id()`

Get unique boot session ID.

**Returns:** `string` - Boot ID or `"unknown"`

---

## dcli.network

Network hardware and connectivity detection.

### Functions

#### `has_wifi()`

Check if WiFi hardware is present.

**Returns:** `boolean`

**Example:**
```lua
if dcli.network.has_wifi() then
    table.insert(packages, "wireless_tools")
    table.insert(packages, "wpa_supplicant")
    table.insert(packages, "iwd")
end
```

---

#### `has_ethernet()`

Check if Ethernet hardware is present.

**Returns:** `boolean`

---

#### `has_bluetooth()`

Check if Bluetooth hardware is present.

**Returns:** `boolean`

**Example:**
```lua
if dcli.network.has_bluetooth() then
    table.insert(packages, "bluez")
    table.insert(packages, "bluez-utils")
    table.insert(services.enabled, "bluetooth.service")
end
```

---

#### `is_connected()`

Check if system has network connectivity.

**Returns:** `boolean`

---

#### `connection_type()`

Get the type of active network connection.

**Returns:** `string` - `"wifi"`, `"ethernet"`, `"none"`, or `"unknown"`

**Example:**
```lua
local conn = dcli.network.connection_type()
if conn == "wifi" then
    dcli.log.info("Connected via WiFi")
elseif conn == "ethernet" then
    dcli.log.info("Connected via Ethernet")
end
```

---

#### `list_interfaces()`

Get list of all network interfaces.

**Returns:** `table` - Array of interface names

**Example:**
```lua
local interfaces = dcli.network.list_interfaces()
for _, iface in ipairs(interfaces) do
    dcli.log.info("Interface: " .. iface)
end
```

---

#### `active_interface()`

Get the currently active network interface.

**Returns:** `string | nil` - Interface name or `nil`

---

#### `interface_type(name)`

Get the type of a network interface.

**Parameters:**
- `name` (`string`) - Interface name (e.g., `"eth0"`, `"wlan0"`)

**Returns:** `string` - `"wifi"`, `"ethernet"`, `"loopback"`, `"bridge"`, `"virtual"`, or `"unknown"`

---

#### `interface_up(name)`

Check if an interface is up.

**Parameters:**
- `name` (`string`) - Interface name

**Returns:** `boolean`

---

#### `has_ipv6()`

Check if IPv6 is enabled and available.

**Returns:** `boolean`

**Example:**
```lua
if dcli.network.has_ipv6() then
    dcli.log.info("IPv6 is enabled")
end
```

---

#### `hostname()`

Get the system hostname.

**Returns:** `string` - Hostname

---

## dcli.audio

Audio system detection and configuration.

### Functions

#### `server()`

Detect which audio server is running.

**Returns:** `string` - `"pulseaudio"`, `"pipewire"`, `"jack"`, `"alsa"`, or `"none"`

**Example:**
```lua
local audio = dcli.audio.server()
if audio == "pipewire" then
    table.insert(packages, "pipewire")
    table.insert(packages, "pipewire-pulse")
    table.insert(packages, "pipewire-alsa")
    table.insert(packages, "wireplumber")
elseif audio == "pulseaudio" then
    table.insert(packages, "pulseaudio")
    table.insert(packages, "pavucontrol")
end
```

---

#### `has_pulseaudio()`

Check if PulseAudio is running.

**Returns:** `boolean`

---

#### `has_pipewire()`

Check if PipeWire is running.

**Returns:** `boolean`

**Example:**
```lua
if dcli.audio.has_pipewire() then
    -- PipeWire-specific configuration
    table.insert(packages, "pipewire-jack")
    table.insert(packages, "qpwgraph")
end
```

---

#### `has_jack()`

Check if JACK audio server is running.

**Returns:** `boolean`

---

#### `has_alsa()`

Check if ALSA is available.

**Returns:** `boolean`

---

#### `list_cards()`

Get list of sound cards.

**Returns:** `table` - Array of sound card names

**Example:**
```lua
local cards = dcli.audio.list_cards()
for _, card in ipairs(cards) do
    dcli.log.info("Sound card: " .. card)
end
```

---

#### `default_sink()`

Get the default audio output device.

**Returns:** `string | nil` - Sink name or `nil`

---

#### `default_source()`

Get the default audio input device.

**Returns:** `string | nil` - Source name or `nil`

---

#### `bluetooth_available()`

Check if Bluetooth audio is available.

**Returns:** `boolean`

**Example:**
```lua
if dcli.audio.bluetooth_available() then
    table.insert(packages, "bluez")
    if dcli.audio.has_pipewire() then
        table.insert(packages, "libspa-bluetooth")
    end
end
```

---

## dcli.storage

Storage device and filesystem detection.

### Functions

#### `has_ssd()`

Check if system has any SSD drives.

**Returns:** `boolean`

**Example:**
```lua
if dcli.storage.has_ssd() then
    -- Enable SSD optimizations
    table.insert(packages, "util-linux")  -- for fstrim
    table.insert(services.enabled, "fstrim.timer")
end
```

---

#### `has_hdd()`

Check if system has any HDD (spinning disk) drives.

**Returns:** `boolean`

---

#### `has_nvme()`

Check if system has any NVMe drives.

**Returns:** `boolean`

**Example:**
```lua
if dcli.storage.has_nvme() then
    table.insert(packages, "nvme-cli")
end
```

---

#### `list_disks()`

Get list of all disk devices.

**Returns:** `table` - Array of disk names (e.g., `{"sda", "nvme0n1"}`)

**Example:**
```lua
local disks = dcli.storage.list_disks()
for _, disk in ipairs(disks) do
    local type = dcli.storage.disk_type(disk)
    dcli.log.info(disk .. " is " .. type)
end
```

---

#### `disk_type(name)`

Get the type of a disk device.

**Parameters:**
- `name` (`string`) - Disk name (e.g., `"sda"`, `"nvme0n1"`)

**Returns:** `string` - `"ssd"`, `"hdd"`, `"nvme"`, or `"unknown"`

---

#### `disk_size(name)`

Get disk size in bytes.

**Parameters:**
- `name` (`string`) - Disk name

**Returns:** `number | nil` - Size in bytes or `nil`

**Example:**
```lua
local size = dcli.storage.disk_size("sda")
if size then
    local gb = size / (1024 * 1024 * 1024)
    dcli.log.info("Disk size: " .. string.format("%.2f GB", gb))
end
```

---

#### `filesystem(path)`

Get filesystem type for a path.

**Parameters:**
- `path` (`string`) - Mount point or path (e.g., `"/"`, `"/home"`)

**Returns:** `string | nil` - Filesystem type like `"ext4"`, `"btrfs"`, `"xfs"`, or `nil`

**Example:**
```lua
local fs = dcli.storage.filesystem("/")
if fs == "btrfs" then
    table.insert(packages, "btrfs-progs")
    table.insert(packages, "snapper")
elseif fs == "ext4" then
    table.insert(packages, "e2fsprogs")
end
```

---

#### `mount_point(device)`

Get mount point for a device.

**Parameters:**
- `device` (`string`) - Device name (e.g., `"sda1"` or `"/dev/sda1"`)

**Returns:** `string | nil` - Mount point or `nil`

---

#### `is_mounted(device)`

Check if a device is currently mounted.

**Parameters:**
- `device` (`string`) - Device name

**Returns:** `boolean`

---

#### `free_space(path)`

Get free space in bytes for a path.

**Parameters:**
- `path` (`string`) - Path to check

**Returns:** `number | nil` - Free space in bytes or `nil`

**Example:**
```lua
local free = dcli.storage.free_space("/")
if free and free < (10 * 1024 * 1024 * 1024) then  -- Less than 10GB
    dcli.log.warn("Low disk space on root partition")
end
```

---

#### `total_space(path)`

Get total space in bytes for a path.

**Parameters:**
- `path` (`string`) - Path to check

**Returns:** `number | nil` - Total space in bytes or `nil`

---

#### `has_swap()`

Check if swap is enabled.

**Returns:** `boolean`

**Example:**
```lua
if not dcli.storage.has_swap() then
    dcli.log.warn("No swap configured")
end
```

---

#### `swap_size()`

Get total swap size in bytes.

**Returns:** `number | nil` - Swap size in bytes or `nil`

---

## Related Documentation

- [LUA-MODULES.md](LUA-MODULES.md) - Lua module format and examples
- [LUA-HOSTS.md](LUA-HOSTS.md) - Lua host configuration guide
- [README.md](README.md) - Main dcli documentation
