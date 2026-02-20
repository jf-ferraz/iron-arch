# Iron Example Configurations

This directory contains example bundles, profiles, modules, and host configurations
to help you get started with Iron.

## Directory Structure

```
iron/
├── bundles/           # Desktop environments
│   ├── hyprland/      # Hyprland Wayland compositor
│   │   ├── bundle.toml
│   │   ├── config/    # Default configs
│   │   └── scripts/   # Setup scripts
│   └── niri/          # Niri scrollable tiling compositor
│       ├── bundle.toml
│       ├── config/
│       └── scripts/
│
├── profiles/          # Configuration presets
│   ├── developer/     # Full development environment
│   │   └── profile.toml
│   └── minimal/       # Clean, basic setup
│       └── profile.toml
│
├── modules/           # Individual configurations
│   ├── nvim-ide/      # Neovim as IDE
│   │   ├── module.toml
│   │   ├── config/    # Neovim configuration
│   │   └── scripts/
│   └── kitty-dev/     # Developer terminal
│       ├── module.toml
│       └── config/    # Kitty configuration
│
└── hosts/             # Machine-specific configs
    └── desktop/       # Example workstation
        └── host.toml
```

## Bundles

Bundles are complete desktop environments. Each bundle includes:
- Core packages for the compositor/DE
- Available profiles
- Post-install hooks
- Service configurations

### Hyprland Bundle
A dynamic tiling Wayland compositor with beautiful animations.
- Default profile: minimal
- Available profiles: minimal, developer, gaming, streamer

### Niri Bundle
A scrollable-tiling compositor with infinite horizontal workspace.
- Default profile: minimal
- Available profiles: minimal, developer, creative

## Profiles

Profiles are configuration presets that determine which modules are enabled.

### Developer Profile
Full development environment including:
- Neovim IDE
- Kitty terminal (dev config)
- Git configuration
- Tmux setup
- Starship prompt

### Minimal Profile
Clean setup with essentials only:
- Basic terminal
- Simple waybar
- Fish shell basics

## Modules

Modules are individual application configurations.

### nvim-ide
Complete Neovim setup with:
- LSP support
- Treesitter highlighting
- Telescope fuzzy finder
- Neo-tree file browser
- Catppuccin theme

### kitty-dev
Developer-focused terminal with:
- JetBrainsMono Nerd Font
- Catppuccin Mocha theme
- Tab powerline style
- Keyboard shortcuts

## Hosts

Host configurations capture machine-specific details.

### desktop (Example)
Example workstation configuration with:
- Hardware catalog (CPU, GPU, monitors)
- Active bundle and installed bundles
- Installation parameters for recovery

## Getting Started

1. **Initialize Iron**:
   ```bash
   iron init
   ```

2. **Install a bundle**:
   ```bash
   iron bundle install hyprland
   ```

3. **Select a profile**:
   ```bash
   iron profile select developer
   ```

4. **Enable additional modules**:
   ```bash
   iron module enable nvim-ide
   ```

## Customizing

Copy any example to create your own:

```bash
# Create a custom bundle based on hyprland
cp -r bundles/hyprland bundles/my-bundle
# Edit bundles/my-bundle/bundle.toml

# Create a custom profile
cp -r profiles/developer profiles/my-profile
# Edit profiles/my-profile/profile.toml

# Create a custom module
mkdir modules/my-module
# Create modules/my-module/module.toml
```

## Best Practices

1. **Keep modules small**: Each module should manage one application's config
2. **Use profiles**: Group related modules instead of enabling them individually
3. **Document dependencies**: Mark which modules depend on or conflict with others
4. **Version configs**: Use git to track your Iron configuration
5. **Test changes**: Use `iron update --dry-run` before applying changes

## System Scan

Run a pre-install system scan to discover existing configurations, package
overlaps, and potential conflicts before Iron touches anything:

```bash
# Run a full system scan (human-readable)
iron scan

# JSON output (for scripting)
iron scan --json
```

The scan checks:
- Existing config files in `~/.config/` and `~/`
- Whether configs are regular files or symlinks (already managed)
- Packages already installed that Iron modules would install
- Potential conflicts between existing configs and Iron modules
- Actionable recommendations (e.g., "Back up ~/.config/nvim before enabling nvim-ide")

The TUI wizard also runs a system scan automatically after first-run setup.

## Health Checks (Doctor)

Diagnose your Iron installation with the built-in doctor:

```bash
# Run all health checks
iron doctor

# JSON output
iron doctor --json
```

The doctor checks symlink integrity, package state, state file consistency,
and snapshot backend availability.

## Secrets Management

Manage encrypted secrets (API keys, tokens, SSH configs) with git-crypt:

```bash
# Check encryption status
iron secrets status

# Unlock secrets after a fresh clone
iron secrets unlock

# Lock secrets before sharing
iron secrets lock

# Symlink decrypted secrets to target locations
iron secrets link

# Add a GPG key to the secrets keyring
iron secrets add-key ABCD1234

# Export symmetric key for backup
iron secrets export-key
iron secrets export-key --output my-key.key
```

## Backup & Recovery

Create backups and recover from failures:

```bash
# Interactive recovery wizard
iron recover

# Generate a standalone install script from current host config
iron recover generate-script

# Create a full backup
iron recover --backup

# Restore from a previous backup
iron recover --restore ./iron-backup-20260219/
```
