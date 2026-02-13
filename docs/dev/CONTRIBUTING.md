# Contributing to Iron

Thank you for your interest in contributing to Iron! This guide will help you get started.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Project Structure](#project-structure)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Pull Request Process](#pull-request-process)
8. [Release Process](#release-process)

---

## Code of Conduct

Please be respectful and constructive in all interactions. We're building software for the Arch Linux community - let's keep the same spirit of helpfulness and technical excellence.

---

## Getting Started

### Prerequisites

- **Rust**: 1.75+ (uses Edition 2024)
- **Arch Linux**: Primary development platform
- **Git**: Version control
- **Clippy**: Linting (`rustup component add clippy`)
- **Rustfmt**: Formatting (`rustup component add rustfmt`)

### Optional Tools

- **cargo-tarpaulin**: Code coverage
- **cargo-watch**: Auto-rebuild on changes
- **timeshift/snapper**: For testing snapshot features

---

## Development Setup

### Clone Repository

```bash
git clone https://github.com/laraj/iron.git
cd iron
```

### Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p iron-core
```

### Run Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p iron-cli

# With output
cargo test --workspace -- --nocapture
```

### Run Lints

```bash
# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace -- -D warnings
```

### Run Application

```bash
# TUI
cargo run -p iron-cli

# CLI commands
cargo run -p iron-cli -- status
cargo run -p iron-cli -- bundle list
```

---

## Project Structure

```
iron/
├── crates/
│   ├── iron-core/       # Domain models, services, state
│   │   ├── src/
│   │   │   ├── models/  # Domain entities
│   │   │   ├── services/# Business logic
│   │   │   ├── state/   # State management
│   │   │   └── error.rs # Error types
│   │   └── Cargo.toml
│   │
│   ├── iron-cli/        # CLI application
│   │   ├── src/
│   │   │   ├── commands/# Command implementations
│   │   │   ├── output/  # Output formatting
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   │
│   ├── iron-tui/        # TUI application
│   │   ├── src/
│   │   │   ├── app.rs   # App state
│   │   │   ├── ui.rs    # Rendering
│   │   │   ├── widgets/ # UI components
│   │   │   └── wizard.rs# Setup wizard
│   │   └── Cargo.toml
│   │
│   ├── iron-fs/         # File operations
│   ├── iron-pacman/     # Package management
│   ├── iron-git/        # Git operations
│   └── iron-systemd/    # Systemd integration
│
├── docs/
│   ├── architecture/    # Technical docs
│   ├── guide/           # User guide
│   ├── dev/             # Developer docs
│   ├── requirements/    # Requirements spec
│   └── workflow/        # Implementation plan
│
├── bundles/             # Example bundles
├── profiles/            # Example profiles
├── modules/             # Example modules
└── hosts/               # Example hosts
```

### Crate Dependencies

```
iron-cli (binary)
    ├── iron-core
    ├── iron-tui
    └── clap

iron-tui (library)
    ├── iron-core
    ├── iron-pacman
    └── ratatui

iron-core (library)
    ├── iron-fs
    ├── iron-pacman
    ├── iron-git
    └── iron-systemd

iron-fs, iron-pacman, iron-git, iron-systemd (infrastructure)
    └── (minimal dependencies)
```

---

## Coding Standards

### Rust Style

- Follow **rustfmt** defaults (configured in `rustfmt.toml`)
- Follow **clippy** recommendations
- Use `thiserror` for error types
- Use `serde` for serialization

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Crates | `iron-*` | `iron-core` |
| Modules | `snake_case` | `bundle_service` |
| Types | `PascalCase` | `BundleState` |
| Functions | `snake_case` | `activate_bundle` |
| Constants | `SCREAMING_SNAKE` | `DEFAULT_TIMEOUT` |

### Documentation

- All public items must have doc comments
- Include examples in doc comments where helpful
- Use `///` for item docs, `//!` for module docs

```rust
/// Activates the specified bundle.
///
/// # Arguments
///
/// * `id` - The bundle identifier
///
/// # Returns
///
/// Returns `Ok(ActivationResult)` on success.
///
/// # Errors
///
/// Returns `Err(StateError::BundleNotFound)` if bundle doesn't exist.
pub fn activate(&self, id: &str) -> Result<ActivationResult> {
    // ...
}
```

### Error Handling

- Use `Result<T, IronError>` for fallible operations
- Provide context with error messages
- Use `anyhow` for prototyping, `thiserror` for library code

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("Bundle not found: {id}")]
    NotFound { id: String },

    #[error("Bundle already active: {id}")]
    AlreadyActive { id: String },

    #[error("Conflict detected: {0}")]
    Conflict(String),
}
```

---

## Testing

### Test Structure

```
crates/iron-*/
├── src/
│   └── lib.rs        # Unit tests inline
└── tests/
    └── integration.rs # Integration tests
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_activation() {
        // Arrange
        let service = BundleService::new(/* ... */);

        // Act
        let result = service.activate("hyprland");

        // Assert
        assert!(result.is_ok());
    }
}
```

### Test Categories

| Type | Location | Purpose |
|------|----------|---------|
| Unit | `src/*.rs` | Test individual functions |
| Integration | `tests/` | Test crate interactions |
| CLI | `iron-cli/tests/` | Test CLI commands |

### Running Tests

```bash
# All tests
cargo test --workspace

# With coverage
cargo tarpaulin --workspace --out Html

# Specific test
cargo test test_bundle_activation

# Integration tests only
cargo test --test integration
```

### Test Coverage Target

- **iron-core**: 80%+
- **iron-cli**: 70%+ (integration tests)
- **iron-tui**: 60%+ (UI tests are harder)
- **Infrastructure crates**: 70%+

---

## Pull Request Process

### Before Submitting

1. **Create an issue** first for significant changes
2. **Fork** the repository
3. **Create a branch**: `git checkout -b feature/my-feature`
4. **Make changes** following coding standards
5. **Add tests** for new functionality
6. **Run checks**:
   ```bash
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

### PR Requirements

- [ ] Descriptive title and description
- [ ] Tests pass locally
- [ ] No clippy warnings
- [ ] Formatted with rustfmt
- [ ] Documentation updated if needed
- [ ] Changelog entry if user-facing

### PR Template

```markdown
## Description

Brief description of changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing

Describe how you tested the changes.

## Checklist

- [ ] Tests pass
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Changelog updated
```

### Review Process

1. Automated checks run (CI)
2. Maintainer reviews code
3. Address feedback
4. Maintainer merges

---

## Release Process

### Version Numbers

We use semantic versioning: `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

### Release Checklist

1. Update version in `Cargo.toml` (workspace)
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v0.2.0`
4. Push tag: `git push origin v0.2.0`
5. CI builds and publishes to AUR

---

## Architecture Overview

### Layer Architecture

```
┌─────────────────────────────────────┐
│         Presentation Layer          │
│      (iron-cli, iron-tui)           │
├─────────────────────────────────────┤
│         Application Layer           │
│         (iron-core services)        │
├─────────────────────────────────────┤
│       Infrastructure Layer          │
│  (iron-fs, iron-pacman, iron-git)   │
└─────────────────────────────────────┘
```

### Key Principles

1. **Separation of Concerns**: Each crate has one responsibility
2. **Dependency Inversion**: Core defines traits, infra implements
3. **Fail-Safe Defaults**: Non-destructive by default
4. **Offline-First**: Core features work without network

For detailed architecture, see [ARCHITECTURE.md](../architecture/ARCHITECTURE.md).

---

## Getting Help

- **Issues**: https://github.com/laraj/iron/issues
- **Discussions**: https://github.com/laraj/iron/discussions

---

Thank you for contributing to Iron!
