# Iron

> **Less is More - Turning your Arch into Iron**

A declarative Arch Linux configuration management platform that brings elegance, safety, and simplicity to system administration.

## Vision

Iron empowers users of all experience levels to:
- Manage system configurations with confidence
- Update safely with proactive breaking-change detection
- Reproduce their exact system on any machine
- Switch between desktop environments seamlessly

## Features

### Core Capabilities
- **Declarative Configuration** - Define your system in TOML, Iron makes it happen
- **Bundle System** - Manage desktop environments (Hyprland, Niri, KDE) as switchable units
- **Profile System** - Create and switch between dotfile collections
- **Module System** - Organize configs into reusable, shareable components

### Safety First
- **Breaking Change Detection** - Arch News, AUR flags, dependency conflicts
- **Risk Scoring** - Know before you update (LOW/MEDIUM/HIGH)
- **Auto-Snapshots** - Timeshift/Snapper integration before changes
- **Instant Rollback** - One command to restore previous state

### User Experience
- **Dashboard TUI** - System health at a glance
- **Guided Wizards** - First-time setup, profile creation, bundle switching
- **Newcomer Friendly** - 5-minute learning curve
- **Power User Ready** - Full CLI for scripting and automation

### Multi-Machine
- **Git-Based Sync** - Same repo, multiple machines
- **Host Profiles** - Machine-specific configurations
- **Recovery Ready** - Full system restore in under 30 minutes

## Quick Start

```bash
# Clone your Iron repo (or initialize new)
git clone <your-iron-repo> ~/.config/iron
cd ~/.config/iron

# First-time setup
iron init

# Launch TUI dashboard
iron

# Or use CLI commands
iron status              # System overview
iron update              # Safe system update
iron bundle switch hypr  # Switch desktop environment
iron profile select dev  # Change dotfile profile
iron sync push           # Backup to git
```

## Architecture

```
HOST (Hardware + System Config)
  └── BUNDLE (Desktop Environment)
        └── PROFILE (Dotfile Collection)
              └── MODULE (Individual Component)
```

## Documentation

- [Requirements Specification](docs/requirements/REQUIREMENTS-SPEC-v1.0.md)
- [Architecture Design](docs/architecture/) (coming soon)
- [User Guide](docs/guide/) (coming soon)
- [Developer Guide](docs/dev/) (coming soon)

## Project Status

**Phase**: Requirements Complete, Architecture Design Next

See [REQUIREMENTS-SPEC-v1.0.md](docs/requirements/REQUIREMENTS-SPEC-v1.0.md) for full specification.

## Tech Stack

- **Language**: Rust
- **TUI**: Ratatui
- **Config**: TOML
- **Operations**: Bash scripts
- **Sync**: Git

## License

MIT License - See [LICENSE](LICENSE)

---

Built with care for the Arch Linux community.
