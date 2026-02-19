# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build
cargo build                          # Debug build
cargo build --release                # Release build

# Test
cargo test --workspace               # Run all tests
cargo test -p iron-core              # Test single crate
cargo test state::tests::            # Run tests matching pattern
cargo test --workspace -- --nocapture # Show test output

# Lint and Format
cargo fmt --all                      # Format code
cargo fmt --all -- --check           # Check formatting
cargo clippy --workspace -- -D warnings  # Lint with warnings as errors

# Coverage
cargo tarpaulin --workspace          # Generate coverage report
```

## Architecture

Iron is a declarative Arch Linux configuration management system built as a Rust workspace with layered architecture:

```
Presentation: iron-cli (binary) + iron-tui (library)
                    │
Application:  iron-core (services, domain logic, state)
                    │
Infrastructure: iron-fs, iron-pacman, iron-git, iron-systemd
```

### Crate Responsibilities

| Crate | Role |
|-------|------|
| `iron-core` | Domain models, services (Host/Bundle/Profile/Module/Update/Sync/Recovery/Secrets), state management, validation, circuit breaker pattern |
| `iron-cli` | Clap-based CLI with command modules in `src/commands/` |
| `iron-tui` | Ratatui TUI with screens (`src/ui/`), app state (`src/app/`), event handling |
| `iron-fs` | File operations, symlinks, backups, TOML I/O |
| `iron-pacman` | Package management via pacman/paru/yay |
| `iron-git` | Git operations via git2 |
| `iron-systemd` | Systemd service management |

### Domain Model

The configuration hierarchy: HOST → BUNDLE → PROFILE → MODULE

- **Host**: Machine-specific config (hardware, install params)
- **Bundle**: Desktop environment (Hyprland, Niri, KDE) with packages, services, dotfiles
- **Profile**: Dotfile collection that can span bundles
- **Module**: Single application config (nvim, kitty, fish)

### State Management

- Configuration state: Git-tracked TOML files in `bundles/`, `profiles/`, `modules/`, `hosts/`
- Runtime state: Local-only JSON in `.iron/state/` (active host/bundle/profile, enabled modules)
- Dormant state: Inactive bundle configs stored in `dormant/`

### Key Patterns

**Circuit Breaker** (`iron-core/src/resilience/`): External commands (pacman, git, systemctl) use circuit breakers to prevent hangs and enable graceful degradation.

**Trait-based abstractions**: `PackageManager`, `SnapshotManager`, `FileSystem` traits in iron-core with real and mock implementations for testing.

**Service layer** (`iron-core/src/services/`): Business logic exposed via service modules (host, bundle, profile, module, update, sync, recovery, secrets, clean, state).

## Code Style

- Line width: 100 chars (rustfmt.toml)
- Tests: Inline `#[cfg(test)]` modules with mocks via trait abstractions
- Errors: `thiserror` for error types, `anyhow::Result` for application code
- Async: Tokio runtime, but many operations are sync

## Documentation

| Resource | When to Read |
|----------|-------------|
| `docs/project-brief.md` | Vision, scope, deliverables |
| `docs/requirements.md` | System requirements |
| `docs/current.md` | Active state, known issues, priorities |
| `docs/decisions/` | Architecture decision records |
| `docs/legacy/architecture/ARCHITECTURE.md` | Detailed technical architecture (1700+ lines) |
