# Architecture

## System Overview

Iron is a Rust workspace organized into three layers: a **Presentation layer** (iron-cli binary and
iron-tui library) that handles user interaction, an **Application layer** (iron-core) that owns all
domain logic and services, and an **Infrastructure layer** (iron-fs, iron-pacman, iron-git,
iron-systemd) that wraps external system tools.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              IRON SYSTEM                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  PRESENTATION                                                                │
│  ┌─────────────────┐              ┌─────────────────┐                        │
│  │    iron-cli     │              │    iron-tui     │                        │
│  │  Commands       │              │  Dashboard      │                        │
│  │  Arguments      │              │  Wizards        │                        │
│  │  Output         │              │  Navigation     │                        │
│  └────────┬────────┘              └────────┬────────┘                        │
│           │                               │                                  │
│           ▼                               ▼                                  │
│  APPLICATION                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  iron-core                                                           │    │
│  │  Host Service · Bundle Service · Profile Service · Module Service   │    │
│  │  Update Service · Recovery Service · Sync Service · Secrets Service │    │
│  │  State Manager · Circuit Breaker · Validation                       │    │
│  └──────────────────────────────────┬──────────────────────────────────┘    │
│                                     │                                        │
│                                     ▼                                        │
│  INFRASTRUCTURE                                                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │  iron-fs    │ │iron-pacman  │ │  iron-git   │ │iron-systemd │           │
│  │  Symlinks   │ │  Packages   │ │  Commits    │ │  Services   │           │
│  │  Backups    │ │  AUR        │ │  Push/Pull  │ │  Timers     │           │
│  │  TOML I/O   │ │  Updates    │ │  Diff       │ │  Units      │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                                              │
│  EXTERNAL SYSTEMS                                                            │
│  pacman · git · systemd · timeshift/snapper · Arch News RSS                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Design Principles

| Principle | Description | Implementation |
|-----------|-------------|----------------|
| **Separation of Concerns** | Each crate has a single responsibility | Domain logic in iron-core; I/O in infra crates |
| **Dependency Inversion** | Core does not depend on infrastructure | Traits defined in iron-core, implemented in infra crates |
| **Fail-Safe Defaults** | Operations are non-destructive by default | Dry-run mode; snapshots before any destructive change |
| **Offline-First** | All core features work without a network connection | Git sync is optional; local state is the source of truth |
| **Progressive Disclosure** | Simple for beginners, powerful for experts | TUI wizards for newcomers; full CLI for power users |

---

## Component Map

| Crate | Type | Responsibility | Key Dependencies |
|-------|------|----------------|-----------------|
| `iron-cli` | Binary | CLI argument parsing, command dispatch, output formatting | iron-core, iron-tui, clap |
| `iron-tui` | Library | TUI rendering, event loop, screens, wizards | iron-core, ratatui, crossterm |
| `iron-core` | Library | Domain models, services (Host/Bundle/Profile/Module/Update/Sync/Recovery/Secrets/Clean), state management, circuit breaker, validation | (no infra deps) |
| `iron-fs` | Library | File operations, symlink management, backups, TOML I/O | iron-core |
| `iron-pacman` | Library | Package management (pacman + AUR), update risk assessment, Arch News | iron-core |
| `iron-git` | Library | Git operations via git2 (commit, push, pull, diff, status) | iron-core |
| `iron-systemd` | Library | Systemd service/timer management (enable, disable, status) | iron-core |

### Dependency Graph

```
                          ┌─────────────┐
                          │  iron-cli   │
                          │   (bin)     │
                          └──────┬──────┘
                                 │
                ┌────────────────┼────────────────┐
                │                │                │
                ▼                ▼                ▼
         ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
         │  iron-tui   │  │  iron-core  │  │   clap      │
         │   (lib)     │  │   (lib)     │  │  (extern)   │
         └──────┬──────┘  └──────┬──────┘  └─────────────┘
                │                │
                ▼                └───────────┐
         ┌─────────────┐                    │
         │  ratatui    │              ┌─────────────┐
         │  (extern)   │              │   iron-fs   │
         └─────────────┘              └──────┬──────┘
                                             │
                        ┌────────────────────┼────────────────────┐
                        ▼                    ▼                    ▼
                 ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
                 │iron-pacman  │      │  iron-git   │      │iron-systemd │
                 │   (lib)     │      │   (lib)     │      │   (lib)     │
                 └─────────────┘      └─────────────┘      └─────────────┘
```

---

## Data Model

### Configuration Hierarchy

```
HOST (hardware + install params)
  └── BUNDLE (desktop environment: Hyprland, Niri, KDE)
        └── PROFILE (dotfile collection: minimal, developer, gaming)
              └── MODULE (single app config: nvim, kitty, fish)
                    └── DOTFILE (source → target symlink)
```

### Entity Relationships

| Entity | Contains | Cardinality | Constraint |
|--------|----------|-------------|------------|
| Host | Bundles | 1:N | Only 1 bundle active at a time |
| Bundle | Profiles | 1:N | User selects active profile |
| Profile | Modules | N:M | Modules can be shared across profiles |
| Module | Dotfiles + Packages + Hooks | 1:N | Atomic config unit |

### Three-Layer State System

| Layer | Location | Format | Notes |
|-------|----------|--------|-------|
| **Config State** | `bundles/`, `profiles/`, `modules/`, `hosts/` | TOML | Git-tracked; shared across machines |
| **Runtime State** | `.iron/state/` | JSON | Local-only; active host/bundle/profile, enabled modules, audit log |
| **Dormant State** | `dormant/` | Files | Git-tracked; inactive bundle configs stored unlinked |
| **Secrets State** | `secrets/` | Encrypted files | Git-tracked; encrypted with git-crypt or age |

### Bundle State Machine

```
NOT_INSTALLED
      │ install()
      ▼
  DORMANT ◄──────────────────── ACTIVE
      │                             │
      │ activate()       deactivate()│
      ▼                             ▼
  ACTIVATING                  DEACTIVATING
      │                             │
  success │ fail               success │
      ▼    ▼                        ▼
  ACTIVE  FAILED ──rollback()──► DORMANT
```

### Module State Machine

```
DISABLED
    │ enable()
    ▼
INSTALLING
    │
    ├── success ──► ENABLED ──── disable() ──► DISABLED
    ├── fail    ──► FAILED  ──── retry()   ──► DISABLED
    └── conflict ─► CONFLICTED ─ resolve() ──► DISABLED
```

---

## API Contracts

### CLI Command Surface

```
iron init                    # First-run wizard
iron status                  # Show active host/bundle/profile/modules
iron doctor                  # Run health checks; output JSON report

iron bundle list             # List all bundles and states
iron bundle switch <id>      # Switch active bundle (snapshot + rollback)
iron bundle activate <id>    # Activate a dormant bundle
iron bundle deactivate <id>  # Move bundle to dormant

iron profile list            # List profiles for active bundle
iron profile switch <id>     # Switch active profile
iron profile create          # Launch TUI profile builder
iron profile show <id>       # Show modules in profile

iron module list             # List all modules with status
iron module enable <id>      # Enable a module
iron module disable <id>     # Disable a module

iron update                  # Check updates, show risk score, prompt for approval
iron update --dry-run        # Show what would change without applying

iron sync push [--message]   # Commit all state and push to remote
iron sync pull               # Pull from remote and apply
iron sync status             # Show sync status

iron recover                 # Start 4-step recovery wizard
iron recover generate-script # Output install.sh from host config

iron secrets unlock          # Decrypt secrets after fresh clone
iron secrets link            # Symlink decrypted secrets to target locations
iron secrets status          # Show encryption status

iron clean                   # Remove orphaned symlinks and stale state
```

### Service Layer (iron-core)

```rust
pub trait HostService {
    fn detect_current(&self) -> Result<Host>;
    fn catalog_hardware(&self) -> Result<HardwareSpec>;
    fn list_hosts(&self) -> Result<Vec<Host>>;
    fn save_host(&self, host: &Host) -> Result<()>;
}

pub trait BundleService {
    fn list_bundles(&self) -> Result<Vec<Bundle>>;
    fn get_active(&self) -> Result<Option<Bundle>>;
    fn activate(&self, id: &str) -> Result<ActivationResult>;
    fn deactivate(&self, id: &str) -> Result<()>;
    fn switch(&self, from: &str, to: &str) -> Result<SwitchResult>;
}

pub trait ProfileService {
    fn list_profiles(&self) -> Result<Vec<Profile>>;
    fn get_active(&self) -> Result<Option<Profile>>;
    fn select(&self, id: &str) -> Result<()>;
    fn create(&self, profile: Profile) -> Result<()>;
    fn get_effective_modules(&self, id: &str) -> Result<Vec<Module>>;
}

pub trait UpdateService {
    fn check_updates(&self) -> Result<UpdateCheck>;
    fn calculate_risk(&self, updates: &[PackageUpdate]) -> Result<RiskAssessment>;
    fn fetch_arch_news(&self) -> Result<Vec<NewsItem>>;
    fn execute_update(&self, approved: bool) -> Result<UpdateResult>;
}

pub trait SyncService {
    fn status(&self) -> Result<SyncStatus>;
    fn push(&self, message: Option<&str>) -> Result<()>;
    fn pull(&self) -> Result<PullResult>;
}

pub trait RecoveryService {
    fn generate_install_script(&self, host: &Host) -> Result<String>;
    fn export_state(&self) -> Result<StateExport>;
    fn verify_installation(&self) -> Result<VerificationResult>;
}
```

---

## Key Decisions

### Decision: Rust Workspace

- **Choice**: Single binary (`iron-cli`) composed of six library crates in a Cargo workspace.
- **Rationale**: Crate boundaries enforce layering (core cannot import infra), enable parallel
  compilation, and isolate test surfaces. Single binary output means no runtime dependency management.
- **Rejected**: Go — less expressive type system, error handling less rigorous; Python — too slow
  for TUI responsiveness target, tends toward monolithic scripts.
- **Consequences**: Steeper onboarding for contributors; faster, safer builds and strong type
  guarantees at crate boundaries.

### Decision: Ratatui for TUI

- **Choice**: Ratatui with crossterm backend for the terminal UI.
- **Rationale**: Mature, actively maintained, keyboard-first design, and well-tested in production
  Rust applications. Crossterm provides cross-platform terminal control.
- **Rejected**: egui — not terminal-native; tview — Go only; cursive — less active maintenance.
- **Consequences**: Terminal-only UI (no mouse-first interactions); responsive by default due to
  immediate-mode rendering.

### Decision: TOML for Configuration

- **Choice**: All configuration files use TOML.
- **Rationale**: Human-readable, unambiguous indentation rules (unlike YAML), native Rust support
  via `toml` crate, and no arbitrary code execution risk.
- **Rejected**: YAML — significant footguns (type coercion, indentation errors); JSON — no
  comments; Lua — arbitrary code execution risk in config files.
- **Consequences**: Slightly more verbose than YAML for nested structures; tooling support is
  excellent.

### Decision: Trait-Based Abstractions

- **Choice**: `PackageManager`, `FileSystem`, and `SnapshotManager` traits defined in iron-core,
  with real implementations in infra crates and mock implementations for tests.
- **Rationale**: Enables unit testing of all service logic without requiring real pacman, git, or
  systemctl on the test machine. Decouples iron-core from specific tool implementations.
- **Rejected**: Direct function calls — untestable without real system tools; dependency injection
  containers — overengineered for this scale.
- **Consequences**: All service tests run without system tools; adding a new backend (e.g., flatpak)
  requires only a new trait implementation.

### Decision: Circuit Breaker for External Commands

- **Choice**: All calls to pacman, git, and systemctl go through a circuit breaker in
  `iron-core/src/resilience/` with a 120-second timeout and `RetryableError` on timeout.
- **Rationale**: External tools can hang indefinitely (network timeouts, user prompts, lock files).
  The circuit breaker prevents Iron from hanging and enables graceful degradation.
- **Rejected**: Raw `std::process::Command` — no timeout support; ad-hoc timeout per call site —
  inconsistent and hard to test.
- **Consequences**: Consistent timeout behavior across all external commands; some operations that
  legitimately take > 120s (large updates) require user-visible progress and chunked execution.

### Decision: Git-Backed Configuration State

- **Choice**: All configuration state (bundles, profiles, modules, hosts, dormant configs) is
  stored as TOML files in a git repository. Runtime state (`.iron/state/`) is local-only and
  gitignored.
- **Rationale**: Git provides versioning, conflict detection, multi-machine sync, and a recovery
  path without requiring a dedicated server or cloud service. Users own their data.
- **Rejected**: SQLite — not human-readable or diff-friendly; custom sync protocol — unnecessary
  infrastructure; cloud-hosted state — privacy and availability concerns.
- **Consequences**: Config history is always available via `git log`; recovery is `git clone` +
  `iron recover`; conflicts are resolved with standard git tooling.

---

## Boundaries

- **In scope**: Arch Linux and derivatives, x86_64 and aarch64 hardware, Wayland and X11 display
  servers, terminal (CLI + TUI) presentation.
- **Out of scope**: Other Linux distributions, graphical (non-terminal) frontends, cloud-hosted
  configuration storage.
- **Extension points**:
  - Additional package manager backends (e.g., flatpak) via `PackageManager` trait
  - Additional encryption backends (age vs. git-crypt) via `SecretsBackend` trait
  - Additional snapshot backends (timeshift vs. snapper) via `SnapshotManager` trait
