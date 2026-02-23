# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current Status

**Phase 0 (Foundation Fixes)** — ✅ Complete (12/12 tasks, 2026-02-22)
**Phase 1 (Core Experience)** — ✅ Complete (Sprint 1.1-1.4, 2026-02-22)
**Phase 2 (Power User Features)** — ✅ Complete (Sprint 2.1-2.3, 19/19 tasks, 2026-02-22)
**Phase 3 (Declarative Convergence)** — PLANNING (Sprint 3.1-3.4, 25 tasks, ~10 weeks)

Phase 2 deliverables implemented:
- F2-001: `SnapshotService` trait + `SnapshotRecord` model (JSON storage in `.snapshots/`)
- F2-002→F2-006: CLI `iron snapshot create/list/restore/delete/prune` + `iron rollback [--module]`
- F2-007: TUI Snapshot timeline view (`[t]` from Dashboard)
- F2-008: Auto-snapshot before destructive operations (apply, update) with auto-prune
- F2-009→F2-011: Tree/table/summary CLI output methods
- F2-012: Progress spinner/bar via indicatif (`ProgressReporter`)
- F2-013: `--explain` mode for shell command visibility
- F2-014: Enhanced error messages with `suggestion()` on `IronError`
- F2-015: Pre-apply config validation (`iron validate`)
- F2-016→F2-018: Security level dashboard (`iron security`, TUI badge, Basic/Standard/Advanced/Paranoid)
- F2-019: Module `security_points` field for security scoring

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
| `iron-core` | Domain models, services (Host/Bundle/Profile/Module/Update/Sync/Recovery/Secrets/Apply/Drift/Snapshot/Security), state management, validation, circuit breaker pattern |
| `iron-cli` | Clap-based CLI with command modules in `src/commands/` |
| `iron-tui` | Ratatui TUI with screens (`src/ui/`), app state (`src/app/`), event handling |
| `iron-fs` | File operations, symlinks, backups, TOML I/O, template engine |
| `iron-pacman` | Package management via pacman/paru/yay |
| `iron-git` | Git operations via git2 |
| `iron-systemd` | Systemd service management |

### Domain Model

The configuration hierarchy: HOST → BUNDLE → PROFILE → MODULE

- **Host**: Machine-specific config (hardware, install params, **declared desired state**: bundle, profile, extra_modules, variables)
- **Bundle**: Desktop environment (Hyprland, Niri, KDE) with packages, services, dotfiles
- **Profile**: Dotfile collection that can span bundles (list of modules, extends inheritance)
- **Module**: Single application config (nvim, kitty, fish) — packages, dotfiles, hooks, dependencies, conflicts

### Key Concepts (Phase 1 + 2)

- **Desired State**: Resolved from host.toml → bundle → profile → modules. Contains all packages, dotfiles, services, variables.
- **Actual State**: Queried from system (pacman, systemctl, readlink, checksums).
- **ApplyPlan**: Diff between desired and actual = list of actions to converge.
- **DriftReport**: Same diff presented as a report for `iron diff`.
- **SnapshotRecord**: Named checkpoint of system state (modules, bundle, packages, checksums). Stored as JSON in `.snapshots/`.
- **SecurityLevel**: Basic/Standard/Advanced/Paranoid based on enabled security modules' `security_points`.

### State Management

- Configuration state: Git-tracked TOML files in `bundles/`, `profiles/`, `modules/`, `hosts/`
- Runtime state: Local-only JSON in `state.json` (active host/bundle/profile, enabled modules)
- Dormant state: Inactive bundle configs stored in `dormant/`

### Key Patterns

**Circuit Breaker** (`iron-core/src/resilience/`): External commands (pacman, git, systemctl) use `CommandExecutor` trait with circuit breakers to prevent hangs and enable graceful degradation. All services use this — no raw `Command::new()` in production paths.

**Trait-based abstractions**: `PackageManager`, `SnapshotManager`, `FileSystem`, `CommandExecutor`, `SystemService`, `SecretsBackend` traits in iron-core with real and mock implementations for testing.

**Service layer** (`iron-core/src/services/`): Business logic exposed via service modules (host, bundle, profile, module, update, sync, recovery, secrets, clean, scan, doctor, state, **apply**, **drift**).

**Builder pattern**: Services constructed with `.with_package_manager()`, `.with_service_manager()`, `.with_executor()` chaining.

**Background execution in TUI**: Long operations (sync push/pull, apply) use `std::thread::spawn` + `mpsc::channel`. Results polled on every `tick()`.

## Code Style

- Edition: 2024 (supports let-chains, if-let chains)
- Line width: 100 chars (rustfmt.toml)
- Tests: Inline `#[cfg(test)]` modules with mocks via trait abstractions
- Errors: `thiserror` for error types, `anyhow::Result` for application code
- New struct fields: Always use `#[serde(default)]` for backward compat. Always update ALL test helpers that construct the struct directly.
- CLI integration tests: Always use `--dry-run` to avoid sudo prompts and TUI launch hangs.
- New TUI `View` variants: Must update ALL 7 exhaustive matches (render dispatch, header, footer×2, help overlay, cycle_forward, cycle_backward, test_view_names).

## Documentation

| Resource | When to Read |
|----------|-------------|
| `docs/current.md` | **Start here** — Active state, known issues, priorities |
| `docs/product-review-and-roadmap.md` | Full product review + 4-phase roadmap |
| `docs/phase3-kanban.md` | **Phase 3 sprint kanban (22 tasks, PLANNING)** |
| `docs/phase3-technical-guide.md` | **Phase 3 implementation guide** |
| `docs/phase2-gap-analysis.md` | Gap analysis: original vision vs current implementation |
| `docs/phase2-kanban.md` | Phase 2 sprint kanban (✅ complete) |
| `docs/phase2-technical-guide.md` | Phase 2 implementation guide (✅ complete) |
| `docs/phase1-kanban.md` | Phase 1 sprint kanban (✅ complete) |
| `docs/phase1-technical-guide.md` | Phase 1 implementation guide (✅ complete) |
| `docs/phase0-kanban.md` | Phase 0 sprint kanban (✅ complete) |
| `docs/phase0-technical-guide.md` | Phase 0 implementation guide (✅ complete) |
| `docs/newcomer-expectations-brainstorm.md` | Newcomer user persona analysis |
| `docs/mid-level-user-expectations-brainstorm.md` | Mid-level user persona analysis |
| `docs/project-brief.md` | Vision, scope, deliverables |
| `docs/requirements.md` | System requirements |