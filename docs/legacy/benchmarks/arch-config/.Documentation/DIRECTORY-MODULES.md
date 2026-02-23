# Directory Modules Guide

## Overview

Directory modules are an advanced module format in dcli that provides better organization for complex modules with multiple package files, post-install scripts, and configuration files (dotfiles).

## Directory Structure

A directory module follows this structure:

```
modules/module-name/
├── module.yaml                 # Module manifest (YAML)
├── module.lua                  # OR Module manifest (Lua) - takes precedence
├── packages.yaml               # Optional: Package files (auto-discovered)
├── packages-*.yaml             # Optional: Additional package files
├── scripts/                    # Optional: Post-install scripts
│   ├── setup.sh
│   └── config.sh
└── dotfiles/                   # Optional: Configuration files
    └── ...
```

> **Note:** If both `module.lua` and `module.yaml` exist, `module.lua` takes precedence.

### Nested Modules

Modules can be organized in nested directories (up to 3 levels supported, 2 recommended):

```
modules/
├── hyprland-dots/
│   ├── hyprland/
│   │   ├── module.yaml
│   │   └── packages.yaml
│   └── i3/
│       ├── module.yaml
│       └── packages.yaml
└── development/
    ├── python/
    │   ├── module.yaml
    │   ├── packages-core.yaml
    │   └── packages-optional.yaml
    └── rust/
        ├── module.yaml
        └── packages.yaml
```

## Module Manifest (module.yaml or module.lua)

The module manifest file defines the module's metadata and configuration. You can use either:

- **`module.yaml`** - Static YAML configuration (simpler)
- **`module.lua`** - Dynamic Lua configuration (more powerful)

If both exist, `module.lua` takes precedence.

### YAML Manifest (module.yaml)

The `module.yaml` file defines static module configuration.

### Fields

```yaml
description: "Brief description of what this module provides"

# Optional: Modules that conflict with this one
conflicts:
  - conflicting-module-1
  - conflicting-module-2

# Optional: Post-install hook script (relative to module directory)
post_install_hook: scripts/setup.sh

# Optional: Explicit list of package files to load (empty = auto-discover)
package_files:
  - packages-core.yaml
  - packages-optional.yaml
```

### Field Descriptions

- **`description`** (string): Human-readable description of the module
- **`conflicts`** (array): List of module names that cannot be enabled simultaneously
- **`post_install_hook`** (string): Path to a script that runs after package installation (relative to module root)
- **`package_files`** (array): Explicit list of package files to load. If empty or omitted, all `*.yaml` files (except `module.yaml`) are auto-discovered

### Lua Manifest (module.lua)

For dynamic configuration, use `module.lua` instead of `module.yaml`. This allows conditional package file selection based on hardware, hostname, or system state.

```lua
-- module.lua
local hostname = dcli.system.hostname()
local memory_mb = dcli.system.memory_total_mb()

-- Dynamically select package files
local package_files = { "packages-core.yaml" }

if memory_mb >= 16000 then
    table.insert(package_files, "packages-heavy.yaml")
end

if dcli.hardware.chassis_type() == "desktop" then
    table.insert(package_files, "packages-desktop.yaml")
end

return {
    description = string.format("Dev tools for %s (%d MB RAM)", hostname, memory_mb),
    package_files = package_files,
    conflicts = { "minimal" },
    post_install_hook = "scripts/setup.sh",
    hook_behavior = "once",
    dotfiles_sync = (hostname == "workstation"),
}
```

**When to use module.lua:**
- Different machines need different package file subsets
- Description should include runtime system information
- Dotfiles sync should be conditional
- Hook behavior depends on environment

See [LUA-MODULES.md](LUA-MODULES.md) for complete Lua API documentation.

## Package Files

### Auto-Discovery Mode

If `package_files` is empty or omitted, dcli will automatically discover all `*.yaml` files in the module directory (excluding `module.yaml`).

**Example structure:**
```
gaming/
├── module.yaml           # Manifest (no package_files specified)
├── packages.yaml         # Auto-discovered
├── steam.yaml            # Auto-discovered
└── emulators.yaml        # Auto-discovered
```

### Explicit Mode

Specify exactly which package files to load:

```yaml
# module.yaml
description: Gaming packages
package_files:
  - packages-core.yaml
  - packages-steam.yaml
  # packages-optional.yaml won't be loaded
```

### Package File Format

Package files use the same format as legacy modules:

```yaml
description: Core gaming packages

packages:
  - steam
  - wine
  - lutris
  
  # Advanced: Package with version constraint
  - name: vulkan-radeon
    version: ">=24.0.0"
  
  # Flatpak support
  - flatpak:com.valvesoftware.Steam

exclude: []  # Only used in host files
```

## Post-Install Hooks

Post-install hooks are bash scripts that run after packages are installed during `dcli sync`.

### Features

- **Location**: Must be in the `scripts/` subdirectory
- **Execution**: Runs with `sudo bash <script>`
- **Tracking**: Tracked by SHA256 hash - only re-runs if script content changes
- **State File**: `~/.config/arch-config/state/hooks-executed.yaml`

### Example Hook

**module.yaml:**
```yaml
description: Python development environment
post_install_hook: scripts/setup-python.sh
```

**scripts/setup-python.sh:**
```bash
#!/bin/bash
set -e

echo "Setting up Python development environment..."

# Create virtualenv directory
mkdir -p ~/.virtualenvs

# Install global pip packages
pip install --upgrade pip setuptools wheel

# Configure poetry
poetry config virtualenvs.in-project true

echo "Python setup complete!"
```

### Hook Execution Behavior

- Hooks run **only once** after initial installation
- If you modify the script, it will run again on next `dcli sync`
- Hooks are skipped with `dcli sync --no-hooks`
- Tracked per-module in state file

## Module Conflicts

Prevent incompatible modules from being enabled simultaneously.

### Example: Hyprland-dots

```yaml
# hyprland/module.yaml
description: Hyprland wayland compositor
conflicts:
  - window-managers/i3
  - window-managers/openbox
packages:
  - hyprland
  - waybar
  - rofi-wayland
```

```yaml
# i3/module.yaml
description: i3 tiling window manager
conflicts:
  - window-managers/hyprland
  - window-managers/sway
packages:
  - i3-wm
  - i3status
  - rofi
```

### Conflict Resolution

When enabling a module with conflicts:

```bash
$ dcli module enable window-managers/hyprland
Warning: Module 'window-managers/i3' conflicts with 'window-managers/hyprland'
Do you want to disable 'window-managers/i3'? (y/n)
```

## Dotfiles Support

### Features

```
module-name/
├── module.yaml
├── packages.yaml
└── dotfiles/
    ├── hypr/
    ├── kitty/
    ├── fish/
    ├── fastfetch/
```

Dotfiles will be symlinked or copied to the user's home/$USER/.config/ directory during sync.

## Complete Example: Development Module

### Directory Structure

```
packages/modules/development/python/
├── module.yaml
├── packages-core.yaml
├── packages-data-science.yaml
└── scripts/
    └── setup-python.sh
```

### module.yaml

```yaml
description: Python development tools and environment

package_files:
  - packages-core.yaml
  - packages-data-science.yaml

conflicts:
  - development/ruby

post_install_hook: scripts/setup-python.sh
```

### packages-core.yaml

```yaml
description: Core Python development packages

packages:
  - python
  - python-pip
  - python-setuptools
  - python-virtualenv
  - python-poetry
  - ipython
  - black
  - mypy
  - pytest
```

### packages-data-science.yaml

```yaml
description: Python data science packages

packages:
  - python-numpy
  - python-pandas
  - python-matplotlib
  - jupyter-notebook
```

### scripts/setup-python.sh

```bash
#!/bin/bash
set -e

echo "Configuring Python development environment..."

# Create virtualenv wrapper directory
mkdir -p ~/.virtualenvs

# Configure poetry
poetry config virtualenvs.in-project true

# Configure pip
pip config set global.break-system-packages true

echo "Python development environment configured!"
```

## Usage Commands

### List All Modules

```bash
dcli module list
```

Shows all modules (legacy and directory) with their status.

### Enable a Module

```bash
# Short name (auto-resolves)
dcli module enable python

# Full path
dcli module enable development/python

# Interactive mode
dcli module enable
```

### Disable a Module

```bash
dcli module disable development/python
```

### Validate Modules

```bash
dcli validate
```

Checks all module structures for errors.

### Sync Packages

```bash
# Install packages and run hooks
dcli sync

# Skip hooks
dcli sync --no-hooks

# Preview changes
dcli sync --dry-run
```

## Validation Rules

Directory modules are validated for:

1. **Module Structure**
   - `module.yaml` must exist
   - Module directory must be valid

2. **Description**
   - Warning if empty

3. **Package Files**
   - All files in `package_files` must exist
   - Warning if no package files found
   - Warning if package files are empty
   - Error for duplicate packages within module

4. **Post-Install Hooks**
   - Hook file must exist if specified
   - Hook must be a file (not directory)
   - Warning if `scripts/` exists but no hook configured

5. **Dotfiles**
   - Warning that dotfiles support is not yet implemented

6. **Naming Conflicts**
   - Error if both `module-name.yaml` and `module-name/` exist

## Legacy vs Directory Modules

| Feature | Legacy Module | Directory Module |
|---------|---------------|------------------|
| **Structure** | Single `.yaml` file | Directory with multiple files |
| **Package Files** | One file only | Multiple files (auto-discovered or explicit) |
| **Scripts** | External path only | Native `scripts/` subdirectory |
| **Hooks** | Absolute path required | Relative path from module root |
| **Dotfiles** | Not supported | `dotfiles/` directory (planned) |
| **Organization** | Flat | Hierarchical/nested |
| **Best For** | Simple modules | Complex modules with many packages |

## Best Practices

1. **Use Directory Modules When:**
   - Module has >20 packages (split into logical files)
   - Post-install configuration is needed
   - Future dotfiles support is desired
   - Logical grouping of packages makes sense

2. **Use Legacy Modules When:**
   - Module has <20 packages
   - No post-install scripts needed
   - Simple, straightforward package list

3. **Organization Tips:**
   - Split packages by category (core, optional, extras)
   - Use descriptive names for package files
   - Keep scripts small and focused
   - Document what post-install hooks do

4. **Naming Conventions:**
   - Use kebab-case for module names (`window-managers`, not `window_managers`)
   - Use descriptive categories for nested modules
   - Limit nesting to 2 levels for maintainability

5. **Hook Scripts:**
   - Always use `#!/bin/bash` and `set -e`
   - Print status messages for visibility
   - Make scripts idempotent (safe to run multiple times)
   - Test scripts independently before adding to module

## Migration from Legacy Modules

To convert a legacy module to directory format:

1. **Create directory:**
   ```bash
   mkdir -p modules/module-name
   ```

2. **Move and split the legacy file:**
   ```bash
   # Copy original
   cp modules/module-name.yaml modules/module-name/packages.yaml
   
   # Optionally split into multiple files
   # Edit packages.yaml, create packages-optional.yaml, etc.
   ```

3. **Create module.yaml:**
   ```yaml
   description: "Copy from original description field"
   conflicts: []  # Copy from original if exists
   post_install_hook: ""  # Add if needed
   package_files: []  # Leave empty for auto-discovery
   ```

4. **Remove old fields from package files:**
   - Remove `conflicts` field from `packages.yaml` (now in `module.yaml`)
   - Remove `post_install_hook` field from `packages.yaml` (now in `module.yaml`)

5. **Add scripts if needed:**
   ```bash
   mkdir -p modules/module-name/scripts
   # Create your setup scripts
   ```

6. **Validate:**
   ```bash
   dcli validate
   ```

7. **Test:**
   ```bash
   dcli sync --dry-run
   ```

8. **Remove legacy file:**
   ```bash
   rm modules/module-name.yaml
   ```

## Troubleshooting

### Module Not Found

```
Error: Module 'my-module' not found
```

**Solutions:**
- Verify `module.yaml` exists in the module directory
- Check module path in `config.yaml` matches directory structure
- Run `dcli module list` to see all available modules

### Package File Not Found

```
Error: Package file specified in manifest not found: packages-extra.yaml
```

**Solutions:**
- Verify file exists in module directory
- Check spelling in `package_files` list
- Use auto-discovery by removing `package_files` field

### Hook Script Not Found

```
Error: post_install_hook script not found: scripts/setup.sh
```

**Solutions:**
- Verify script exists at specified path (relative to module root)
- Check file permissions (`chmod +x scripts/setup.sh`)
- Verify path uses forward slashes, not backslashes

### Duplicate Package Error

```
Error: Duplicate package across files: steam
```

**Solutions:**
- Remove duplicate package from one of the package files
- Each package should only appear once within a module

## File Locations

- **Config Root**: `~/.config/arch-config/` (or `$ARCH_CONFIG_DIR`)
- **Modules Directory**: `~/.config/arch-config/modules/`
- **Main Config**: `~/.config/arch-config/config.yaml`
- **Hook State**: `~/.config/arch-config/state/hooks-executed.yaml`

## Advanced Features

### Version Constraints

Pin packages to specific versions or ranges:

```yaml
packages:
  # Exact version
  - name: hyprland
    version: "0.52.1-6"
  
  # Minimum version
  - name: git
    version: ">=2.40.0"
  
  # Maximum version
  - name: python
    version: "<3.12"
```

### Flatpak Packages

Include Flatpak applications:

```yaml
packages:
  # Simple flatpak syntax
  - flatpak:com.spotify.Client
  
  # Object syntax
  - name: com.valvesoftware.Steam
    type: flatpak
```

### Package Exclusions (Host Files Only)

Host files can exclude packages from base/modules:

```yaml
# packages/hosts/laptop.yaml
packages:
  - tlp
  - powertop

exclude:
  - desktop-package
  - nvidia-drivers
```

## Future Enhancements

- Full dotfiles support with symlinking/copying
- Module dependencies (require other modules)
- Module versioning
- Module templates
- Remote module repositories
- Per-module flatpak scope
- AUR package support in modules
- Pre-install hooks
- Module update notifications
