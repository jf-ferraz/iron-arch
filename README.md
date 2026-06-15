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
- **Risk Scoring** - Know before you update (LOW/MEDIUM/HIGH/CRITICAL)
- **Auto-Snapshots** - Timeshift/Snapper integration before changes
- **Instant Rollback** - One command to restore previous state

### User Experience
- **Dashboard TUI** - System health at a glance with ratatui
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

### Crate Structure

```
iron/
├── crates/
│   ├── iron-core/     # Domain models, services, state management (707 tests)
│   ├── iron-cli/      # CLI application with 11 command groups (88 tests)
│   ├── iron-tui/      # TUI dashboard with ratatui (239 tests)
│   ├── iron-fs/       # File system operations (61 tests)
│   ├── iron-pacman/   # Package management (139 tests)
│   ├── iron-git/      # Git operations (95 tests)
│   └── iron-systemd/  # Systemd service management (64 tests)
├── bundles/           # Desktop environment bundles
├── profiles/          # User profiles
├── modules/           # Configuration modules
└── hosts/             # Host-specific configs
```

## CLI Commands

```
iron                         # Launch TUI dashboard
iron init                    # Initialize Iron configuration
iron status                  # Show system status
iron doctor                  # System health check
iron clean                   # System cleanup
iron update [--dry-run]      # Safe system update

iron bundle list             # List available bundles
iron bundle install <id>     # Install a bundle
iron bundle switch <id>      # Switch active bundle

iron profile list            # List available profiles
iron profile select <id>     # Activate a profile
iron profile create <name>   # Create new profile

iron module list             # List all modules
iron module enable <id>      # Enable a module
iron module disable <id>     # Disable a module

iron import hm <path>        # Scaffold modules from a `home-manager build` output
iron import hm <path> --dry-run        # Preview the modules without writing
iron import hm <path> --only kitty,fish # Import selected apps only

iron host list               # List configured hosts
iron host current            # Show current host
iron host catalog            # Catalog hardware

iron sync status             # Show sync status
iron sync push               # Push changes to remote
iron sync pull               # Pull changes from remote

iron secrets status          # Show secrets status
iron secrets unlock          # Decrypt secrets
iron secrets lock            # Encrypt secrets

iron recover                 # Recovery workflow
```

## Documentation

- [Requirements Specification](docs/requirements/REQUIREMENTS-SPEC-v1.0.md) - Full project requirements
- [Architecture Design](docs/architecture/ARCHITECTURE.md) - Technical architecture and design
- [API Reference](docs/architecture/API.md) - Service and type documentation
- [Configuration Reference](docs/architecture/CONFIG.md) - TOML configuration formats
- [Implementation Plan](docs/workflow/IMPLEMENTATION-PLAN.md) - Development phases
- [User Guide](docs/guide/USER-GUIDE.md) - Getting started and usage
- [Contributing](docs/dev/CONTRIBUTING.md) - Developer guide

## Project Status

**Current Version**: 0.1.0
**Phase**: 8/9 Complete (Production Hardening)
**Test Coverage**: 1,533 tests passing (64% line coverage)

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Foundation (Domain Models, Errors, Validation) | ✅ Complete |
| 2 | Infrastructure (FS, Pacman, Git, Systemd) | ✅ Complete |
| 3 | Core Services (8 services implemented) | ✅ Complete |
| 4 | CLI Implementation (11 command groups) | ✅ Complete |
| 5 | TUI Implementation (ratatui + crossterm) | ✅ Complete |
| 6 | CLI Integration Tests (88 tests) | ✅ Complete |
| 7 | NFR Implementation (resilience, logging) | ✅ Complete |
| 8 | Production Hardening (circuit breaker, graceful degradation) | ✅ Complete |
| 9 | Polish & Release | Pending |

## Tech Stack

- **Language**: Rust (Edition 2024)
- **TUI**: Ratatui 0.29 + Crossterm 0.28
- **CLI**: Clap 4.0 with derive macros
- **Config**: TOML (toml 0.8)
- **State**: JSON (serde_json)
- **Git**: git2 0.19
- **Async**: Tokio 1.0

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace

# Run with coverage
cargo tarpaulin --workspace
```

## License

MIT License - See [LICENSE](LICENSE)

---

Built with care for the Arch Linux community.
