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
