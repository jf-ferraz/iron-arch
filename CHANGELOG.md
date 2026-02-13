# Changelog

All notable changes to Iron will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- TUI launch via `iron go` command
- Additional bundle configurations (GNOME, KDE, Sway)
- Cloud sync options (GitHub gist backup)

---

## [0.1.0] - 2025-02-12

### Phase 7: Polish & Release

#### Documentation
- Comprehensive USER-GUIDE.md with CLI and TUI usage
- CONTRIBUTING.md with development guidelines
- EXAMPLES.md explaining configuration structure
- Complete example configurations for bundles, profiles, modules

#### Distribution
- PKGBUILD for Arch Linux packaging
- PKGBUILD-git for development installs
- GitHub Actions release workflow with multi-target builds
- Shell completion generation (bash, zsh, fish)
- Installation script (`scripts/install.sh`)

#### Quality Assurance
- All 165 tests passing
- Zero clippy warnings
- Security review passed (no injection vulnerabilities)
- Performance validated (< 1ms startup, 2.9MB binary)

---

### Added

#### Phase 1: Foundation
- Workspace setup with 7 crates
- Domain models: Host, Bundle, Profile, Module
- Error system with `IronError` hierarchy
- Validation layer for IDs and paths
- 62 unit tests in iron-core

#### Phase 2: Infrastructure
- **iron-fs**: TOML parsing, symlink management, backup operations
- **iron-pacman**: Package queries, AUR helper detection, risk assessment
- **iron-git**: Repository status, commit, push/pull operations
- **iron-systemd**: Service enable/disable, timer management
- 27 infrastructure tests

#### Phase 3: Core Services
- **StateManager**: JSON state persistence with transactions
- **HostService**: Hardware detection, host management
- **BundleService**: Bundle install/activate/switch
- **ProfileService**: Profile selection, inheritance resolution
- **ModuleService**: Module enable/disable, conflict detection
- **UpdateService**: Risk assessment, Arch News integration
- **SyncService**: Git synchronization
- **SecretsService**: git-crypt integration
- **RecoveryService**: Install script generation

#### Phase 4: CLI Implementation
- 11 command groups implemented
- Output modes: Human, JSON, Verbose, Quiet
- Color support with `--no-color` flag
- Comprehensive help text

Commands:
- `iron init` - Initialize configuration
- `iron status` - System overview
- `iron doctor` - Health checks
- `iron clean` - Cache cleanup
- `iron bundle [list|install|switch|remove]`
- `iron profile [list|select|create]`
- `iron module [list|enable|disable|apply]`
- `iron host [list|current|catalog|snapshot]`
- `iron update [--dry-run|--force|--yes]`
- `iron sync [status|push|pull]`
- `iron secrets [status|unlock|lock|link]`
- `iron recover`

#### Phase 5: TUI Implementation
- Dashboard view with system health
- Bundle management views
- Profile management views
- Module management views
- Update preview with risk display
- Setup wizard (6 steps)
- Interactive actions with confirmations
- Real data integration with iron-pacman
- 22 TUI tests

#### Phase 6: CLI Integration Tests
- 54 CLI integration tests
- Coverage for all command groups
- Output format verification
- Error handling validation

### Technical Details

- **Language**: Rust (Edition 2024)
- **TUI Framework**: Ratatui 0.29 + Crossterm 0.28
- **CLI Framework**: Clap 4.0
- **Config Format**: TOML (toml 0.8)
- **State Format**: JSON (serde_json)
- **Async Runtime**: Tokio 1.0
- **Git Integration**: git2 0.19

### Test Coverage

| Crate | Tests |
|-------|-------|
| iron-core | 62 |
| iron-cli | 54 |
| iron-tui | 22 |
| iron-pacman | 12 |
| iron-git | 9 |
| iron-fs | 3 |
| iron-systemd | 3 |
| **Total** | **165** |

### Architecture Improvements

- Fixed cross-layer coupling (PackageManager trait in iron-core)
- Dependency injection for TUI package manager
- NoopPackageManager for testing
- Clippy warnings fixed (38 → 0)
- API improvements (`&PathBuf` → `&Path`)

---

## Release Notes Format

### Version Header
```
## [X.Y.Z] - YYYY-MM-DD
```

### Change Categories
- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Features to be removed in future
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security-related changes
