# Project Brief

## Vision

Iron is a declarative Arch Linux configuration management platform. It transforms system
administration from a complex, error-prone process into an elegant, safe, and user-friendly
experience. Less is more — turning your Arch into Iron.

## Target Users

| Priority | User Type | Goal |
|----------|-----------|------|
| PRIMARY | Developer / Power User | Wants the power of Arch without the maintenance burden |
| SECONDARY | Linux Migrant | Coming from Ubuntu/Fedora, ready for Arch with guardrails |
| TERTIARY | Complete Newcomer | Not the primary target, but UX must remain accessible |

## Problem Statement

Existing Arch configuration tools (dcli, jff-arch-config, arch-config) have good technical bones
but require deep system knowledge to use. They offer no guided onboarding, no risk-scored update
approval workflow, and no safe bundle switching with automatic rollback. Users who make a mistake
during an update or a desktop environment switch have no structured recovery path. Iron directly
addresses this gap: a newcomer should be productive in under five minutes, and an expert should
never have to touch the Arch Wiki for routine maintenance.

## Key Deliverables

1. **Single `iron` binary** — one statically-linked Rust binary, no runtime dependencies beyond
   pacman, systemd, and git.
2. **TUI dashboard with guided wizards** — Ratatui-based interface with first-run wizard,
   bundle/profile selector, and pre-update approval screen.
3. **Safe update workflow with risk scores** — Arch News integration, AUR flagged-package
   detection, and LOW/MEDIUM/HIGH/CRITICAL risk scoring before any update runs.
4. **Git-backed config sync and recovery** — full system state in a git repo; `iron recover`
   restores an identical system in under 30 minutes.

## Success Metrics

- **Learning curve**: newcomer completes basic tasks in ≤ 5 minutes
- **TUI responsiveness**: keypress to screen update in < 100ms
- **Recovery time**: full system restore from scratch in < 30 minutes
- **Deployment footprint**: single binary, zero runtime library dependencies
- **Update safety**: zero unreviewed CRITICAL updates applied (always requires typed confirmation)

## Scope

### In Scope

- Host, bundle, profile, and module management (create, list, switch, enable, disable)
- Safe update workflow: risk scoring, Arch News, AUR flags, approval gates, auto-snapshot
- Git-backed config sync: push, pull, conflict detection, multi-machine support
- Secrets management: SSH keys, GPG keys, API tokens via git-crypt or age
- TUI interface: dashboard, first-run wizard, bundle/profile wizard, profile builder
- CLI interface: all operations available as `iron <subcommand>` for scripting

### Out of Scope

- Non-Arch Linux distributions (no Debian, Fedora, NixOS support)
- Graphical (non-terminal) frontends
- Cloud storage for configurations (users provide their own git remote)

## Constraints

- **Platform**: Arch Linux and derivatives only
- **Language**: Rust (core logic) + Bash (install scripts and hooks)
- **TUI framework**: Ratatui + crossterm
- **Config format**: TOML
- **Required system tools**: pacman, systemd, stow, git
- **Optional system tools**: paru/yay (AUR), timeshift/snapper (snapshots), git-crypt/age (secrets)

## Technical Preferences

- Rust workspace with one binary crate (`iron-cli`) and six library crates
- Trait-based abstractions (`PackageManager`, `FileSystem`, `SnapshotManager`) for testability
- Inline `#[cfg(test)]` modules with mock implementations; no separate test crate
- `thiserror` for library error types; `anyhow::Result` in application code
- Tokio async runtime; most operations are sync except network and long I/O
- Circuit breaker pattern for all external commands (pacman, git, systemctl) — 120s timeout
