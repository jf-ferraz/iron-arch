# Iron Technical Architecture

> **Document Status**: FINAL
> **Version**: 1.1.0
> **Created**: 2025-02-12
> **Last Updated**: 2026-02-13
> **Author**: Architecture Design Session
> **Reviewed By**: Expert Panel (Fowler, Nygard)

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Crate Architecture](#2-crate-architecture)
3. [Domain Model](#3-domain-model)
4. [State Management](#4-state-management)
5. [Data Flow](#5-data-flow)
6. [CLI Architecture](#6-cli-architecture)
7. [TUI Architecture](#7-tui-architecture)
8. [External Integrations](#8-external-integrations)
9. [File System Layout](#9-file-system-layout)
10. [Security Model](#10-security-model)
11. [Error Handling](#11-error-handling)
12. [Testing Strategy](#12-testing-strategy)

---

## 1. System Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              IRON SYSTEM                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      PRESENTATION LAYER                              │    │
│  │  ┌─────────────────┐              ┌─────────────────┐               │    │
│  │  │    iron-cli     │              │    iron-tui     │               │    │
│  │  │  (CLI Binary)   │              │  (TUI Binary)   │               │    │
│  │  │                 │              │                 │               │    │
│  │  │ • Commands      │              │ • Dashboard     │               │    │
│  │  │ • Arguments     │              │ • Wizards       │               │    │
│  │  │ • Output        │              │ • Navigation    │               │    │
│  │  └────────┬────────┘              └────────┬────────┘               │    │
│  └───────────┼─────────────────────────────────┼───────────────────────┘    │
│              │                                 │                             │
│              ▼                                 ▼                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                       APPLICATION LAYER                              │    │
│  │  ┌─────────────────────────────────────────────────────────────┐    │    │
│  │  │                      iron-core                               │    │    │
│  │  │                                                              │    │    │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │    │    │
│  │  │  │   Host   │ │  Bundle  │ │ Profile  │ │  Module  │        │    │    │
│  │  │  │ Service  │ │ Service  │ │ Service  │ │ Service  │        │    │    │
│  │  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │    │    │
│  │  │                                                              │    │    │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │    │    │
│  │  │  │  Update  │ │ Recovery │ │   Sync   │ │ Secrets  │        │    │    │
│  │  │  │ Service  │ │ Service  │ │ Service  │ │ Service  │        │    │    │
│  │  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │    │    │
│  │  │                                                              │    │    │
│  │  │  ┌────────────────────────────────────────────────────┐     │    │    │
│  │  │  │              State Manager                          │     │    │    │
│  │  │  └────────────────────────────────────────────────────┘     │    │    │
│  │  └──────────────────────────────────────────────────────────────┘    │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     INFRASTRUCTURE LAYER                             │    │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐    │    │
│  │  │  iron-fs    │ │iron-pacman  │ │  iron-git   │ │iron-systemd │    │    │
│  │  │             │ │             │ │             │ │             │    │    │
│  │  │ • Symlinks  │ │ • Packages  │ │ • Commits   │ │ • Services  │    │    │
│  │  │ • Backups   │ │ • AUR       │ │ • Push/Pull │ │ • Timers    │    │    │
│  │  │ • TOML I/O  │ │ • Updates   │ │ • Diff      │ │ • Units     │    │    │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘    │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                       EXTERNAL SYSTEMS                               │    │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐ ┌──────────────┐    │    │
│  │  │ pacman │ │  git   │ │systemd │ │timeshift │ │  Arch News   │    │    │
│  │  │        │ │        │ │        │ │/snapper  │ │  RSS Feed    │    │    │
│  │  └────────┘ └────────┘ └────────┘ └──────────┘ └──────────────┘    │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Principles

| Principle | Description | Implementation |
|-----------|-------------|----------------|
| **Separation of Concerns** | Each crate has a single responsibility | Domain logic in core, I/O in infra crates |
| **Dependency Inversion** | Core doesn't depend on infra | Traits defined in core, implemented in infra |
| **Fail-Safe Defaults** | Operations are non-destructive by default | Dry-run mode, snapshots before changes |
| **Offline-First** | All core features work without network | Git sync optional, local state primary |
| **Progressive Disclosure** | Simple for beginners, powerful for experts | TUI wizards for novices, CLI for power users |

---

## 2. Crate Architecture

### 2.1 Dependency Graph

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
                    │    ┌───────────┴───────────┐
                    │    │                       │
                    ▼    ▼                       ▼
             ┌─────────────┐              ┌─────────────┐
             │  ratatui    │              │   iron-fs   │
             │  (extern)   │              │   (lib)     │
             └─────────────┘              └──────┬──────┘
                                                 │
                    ┌────────────────────────────┴────────────────────────────┐
                    │                            │                            │
                    ▼                            ▼                            ▼
             ┌─────────────┐              ┌─────────────┐              ┌─────────────┐
             │iron-pacman  │              │  iron-git   │              │iron-systemd │
             │   (lib)     │              │   (lib)     │              │   (lib)     │
             └─────────────┘              └─────────────┘              └─────────────┘
```

### 2.2 Crate Responsibilities

| Crate | Type | Responsibility |
|-------|------|----------------|
| `iron-cli` | Binary | CLI argument parsing, command dispatch, output formatting |
| `iron-tui` | Library | TUI rendering, event handling, widget management |
| `iron-core` | Library | Domain logic, services, state management, validation |
| `iron-fs` | Library | File operations, symlinks, backups, TOML parsing |
| `iron-pacman` | Library | Package management, AUR, updates, risk assessment |
| `iron-git` | Library | Git operations, sync, secrets encryption |
| `iron-systemd` | Library | Service management, timers, unit files |

### 2.3 Crate Interfaces

#### iron-core Public API

```rust
// Services
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

pub trait ModuleService {
    fn list_modules(&self) -> Result<Vec<Module>>;
    fn get_status(&self, id: &str) -> Result<ModuleState>;
    fn enable(&self, id: &str) -> Result<()>;
    fn disable(&self, id: &str) -> Result<()>;
    fn check_conflicts(&self, ids: &[&str]) -> Result<Vec<Conflict>>;
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

## 3. Domain Model

### 3.1 Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           IRON DOMAIN MODEL                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐         1:N         ┌─────────────────┐                │
│  │      HOST       │◄────────────────────│     BUNDLE      │                │
│  ├─────────────────┤                     ├─────────────────┤                │
│  │ id: String      │                     │ id: String      │                │
│  │ name: String    │                     │ name: String    │                │
│  │ hardware: Spec  │                     │ type: BundleType│                │
│  │ install_params  │                     │ packages: Vec   │                │
│  │ active_bundle   │─────────────────────│ profiles: Vec   │────┐           │
│  │ installed_bundles                     │ conflicts: Vec  │    │           │
│  └─────────────────┘                     │ services: Vec   │    │           │
│                                          │ state: State    │    │           │
│                                          └────────┬────────┘    │           │
│                                                   │              │           │
│                                               1:N │              │           │
│                                                   ▼              │           │
│  ┌─────────────────┐         N:M         ┌─────────────────┐    │ 1:N       │
│  │     MODULE      │◄────────────────────│    PROFILE      │◄───┘           │
│  ├─────────────────┤                     ├─────────────────┤                │
│  │ id: String      │                     │ id: String      │                │
│  │ name: String    │                     │ name: String    │                │
│  │ kind: ModuleKind│                     │ modules: Vec    │                │
│  │ packages: Vec   │                     │ theme: String   │                │
│  │ dotfiles: Vec   │                     │ shell: String   │                │
│  │ conflicts: Vec  │                     │ extends: Option │                │
│  │ depends: Vec    │                     │ for_bundle: Opt │                │
│  │ hooks: Hooks    │                     │ state: State    │                │
│  │ state: State    │                     └─────────────────┘                │
│  └─────────────────┘                                                        │
│           │                                                                  │
│       1:N │                                                                  │
│           ▼                                                                  │
│  ┌─────────────────┐                                                        │
│  │    DOTFILE      │                                                        │
│  ├─────────────────┤                                                        │
│  │ source: Path    │                                                        │
│  │ target: Path    │                                                        │
│  │ link: bool      │                                                        │
│  │ state: State    │                                                        │
│  └─────────────────┘                                                        │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 State Machines

#### Bundle State Machine

```
                                    ┌─────────────────┐
                                    │  NOT_INSTALLED  │
                                    └────────┬────────┘
                                             │
                                             │ install()
                                             ▼
                         ┌──────────────────────────────────────┐
                         │                                      │
              activate() │                                      │ (first install)
                         │                                      │
                         ▼                                      ▼
                  ┌─────────────┐                        ┌─────────────┐
     ┌───────────│   DORMANT   │                        │   ACTIVE    │───────────┐
     │           └──────┬──────┘                        └──────┬──────┘           │
     │                  │                                      │                  │
     │           activate()                             deactivate()              │
     │                  │                                      │                  │
     │                  ▼                                      ▼                  │
     │           ┌─────────────┐                        ┌─────────────┐           │
     │           │ ACTIVATING  │                        │DEACTIVATING │           │
     │           └──────┬──────┘                        └──────┬──────┘           │
     │                  │                                      │                  │
     │         success/ │ \fail                       success/ │ \fail            │
     │                  │  \                                   │  \               │
     │                  ▼   ▼                                  ▼   ▼              │
     │           ┌─────────────┐  ┌────────┐           ┌─────────────┐            │
     │           │   ACTIVE    │  │ FAILED │           │   DORMANT   │            │
     │           └─────────────┘  └────┬───┘           └─────────────┘            │
     │                                 │                                          │
     │                          retry()/rollback()                                │
     │                                 │                                          │
     │                                 ▼                                          │
     │                          ┌─────────────┐                                   │
     │                          │   DORMANT   │                                   │
     │                          └─────────────┘                                   │
     │                                                                            │
     │                  │ uninstall()                          │ uninstall()      │
     │                  ▼                                      ▼                  │
     │           ┌─────────────────┐                   ┌─────────────────┐        │
     │           │  NOT_INSTALLED  │                   │  NOT_INSTALLED  │        │
     │           └─────────────────┘                   └─────────────────┘        │
     │                                                                            │
     └─────────────────────── switch(other) ──────────────────────────────────────┘
                                    │
                                    ▼
                             (triggers DEACTIVATING on current,
                              then ACTIVATING on target)
```

**Transitional States:**

| State | Description | Duration | Recovery |
|-------|-------------|----------|----------|
| ACTIVATING | Bundle configs being linked, services starting | < 30s | Auto-rollback on failure |
| DEACTIVATING | Bundle configs being unlinked, services stopping | < 30s | Manual intervention |
| FAILED | Activation failed mid-process | Until retry/rollback | Rollback to DORMANT or retry |

#### Module State Machine

```
              ┌─────────────────┐
              │    DISABLED     │
              └────────┬────────┘
                       │
                       │ enable()
                       ▼
              ┌─────────────────┐
        ┌─────│   INSTALLING    │─────┐
        │     └─────────────────┘     │
        │              │              │
   fail │              │ success      │ conflict
        ▼              ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│   FAILED    │ │   ENABLED   │ │ CONFLICTED  │
└─────────────┘ └──────┬──────┘ └─────────────┘
        │              │              │
        │              │ disable()    │ resolve()
        │              ▼              │
        │       ┌─────────────┐       │
        └──────►│   DISABLED  │◄──────┘
                └─────────────┘
```

### 3.3 Value Objects

```rust
/// Risk level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Hardware chassis type
#[derive(Debug, Clone, Copy)]
pub enum ChassisType {
    Desktop,
    Laptop,
    Server,
    Tablet,
    Convertible,
    Unknown,
}

/// Bundle type classification
#[derive(Debug, Clone, Copy)]
pub enum BundleType {
    WaylandCompositor,
    DesktopEnvironment,
    X11WindowManager,
}

/// Module kind classification
#[derive(Debug, Clone, Copy)]
pub enum ModuleKind {
    AppConfig,
    Shell,
    DesktopComponent,
    Theme,
    SystemUtil,
    DevTools,
}

/// Operation result
#[derive(Debug)]
pub enum OperationResult<T> {
    Success(T),
    PartialSuccess { result: T, warnings: Vec<Warning> },
    Failure { error: Error, rollback_performed: bool },
}
```

---

## 4. State Management

### 4.1 State Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         STATE MANAGEMENT                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      CONFIGURATION STATE                             │    │
│  │                    (Git-tracked, TOML files)                         │    │
│  │                                                                      │    │
│  │  bundles/           profiles/           modules/           hosts/    │    │
│  │  ├── hyprland/      ├── developer/      ├── nvim-ide/     ├── desktop/   │
│  │  │   └── bundle.toml│   └── profile.toml│   └── module.toml│   └── host.toml
│  │  └── niri/          └── minimal/        └── kitty-dev/    └── laptop/│    │
│  │      └── bundle.toml    └── profile.toml    └── module.toml    └── host.toml
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                       RUNTIME STATE                                  │    │
│  │                (Local only, .iron/state/ directory)                  │    │
│  │                                                                      │    │
│  │  .iron/state/                                                        │    │
│  │  ├── current_host.json      # Which host is active                  │    │
│  │  ├── active_bundle.json     # Which bundle is linked                │    │
│  │  ├── active_profile.json    # Which profile is applied             │    │
│  │  ├── enabled_modules.json   # List of enabled module IDs           │    │
│  │  ├── maintenance.json       # Last update/clean/doctor timestamps  │    │
│  │  └── operations.jsonl       # Audit log of all operations          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                       DORMANT STATE                                  │    │
│  │               (Git-tracked, inactive bundle configs)                 │    │
│  │                                                                      │    │
│  │  dormant/                                                            │    │
│  │  └── niri/                  # Niri configs when Hyprland is active  │    │
│  │      ├── .config/niri/                                              │    │
│  │      ├── .config/waybar/                                            │    │
│  │      └── .config/fuzzel/                                            │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                       SECRETS STATE                                  │    │
│  │               (Git-tracked, encrypted with git-crypt)               │    │
│  │                                                                      │    │
│  │  secrets/                   # Encrypted at rest                     │    │
│  │  ├── ssh/                   # SSH keys                              │    │
│  │  ├── gpg/                   # GPG keys                              │    │
│  │  └── tokens/                # API tokens                            │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 State Operations

```rust
/// State manager for all Iron state
pub struct StateManager {
    root: PathBuf,
    config_state: ConfigState,
    runtime_state: RuntimeState,
}

impl StateManager {
    /// Load all state from disk
    pub fn load(root: &Path) -> Result<Self>;

    /// Save runtime state (called after each operation)
    pub fn save_runtime(&self) -> Result<()>;

    /// Get current host
    pub fn current_host(&self) -> Option<&Host>;

    /// Get active bundle for current host
    pub fn active_bundle(&self) -> Option<&Bundle>;

    /// Get active profile for current host
    pub fn active_profile(&self) -> Option<&Profile>;

    /// Get all enabled modules
    pub fn enabled_modules(&self) -> &[String];

    /// Record an operation for audit log
    pub fn record_operation(&mut self, op: Operation) -> Result<()>;

    /// Transaction support for atomic operations
    pub fn begin_transaction(&mut self) -> Transaction;
}

/// Transaction for atomic state changes
pub struct Transaction<'a> {
    manager: &'a mut StateManager,
    changes: Vec<StateChange>,
    committed: bool,
}

impl Transaction<'_> {
    pub fn set_active_bundle(&mut self, id: &str);
    pub fn set_active_profile(&mut self, id: &str);
    pub fn enable_module(&mut self, id: &str);
    pub fn disable_module(&mut self, id: &str);
    pub fn commit(self) -> Result<()>;
    pub fn rollback(self);
}
```

---

## 5. Data Flow

### 5.1 Bundle Switch Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        BUNDLE SWITCH DATA FLOW                               │
│                    iron bundle switch niri -> hyprland                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User                CLI              Core              FS            System │
│   │                   │                │                │                │   │
│   │  switch hyprland  │                │                │                │   │
│   │──────────────────►│                │                │                │   │
│   │                   │                │                │                │   │
│   │                   │  validate()    │                │                │   │
│   │                   │───────────────►│                │                │   │
│   │                   │                │                │                │   │
│   │                   │  ◄─ conflicts? │                │                │   │
│   │                   │◄───────────────│                │                │   │
│   │                   │                │                │                │   │
│   │  ◄─ confirm?      │                │                │                │   │
│   │◄──────────────────│                │                │                │   │
│   │                   │                │                │                │   │
│   │  yes ────────────►│                │                │                │   │
│   │                   │                │                │                │   │
│   │                   │  begin_txn()   │                │                │   │
│   │                   │───────────────►│                │                │   │
│   │                   │                │                │                │   │
│   │                   │                │ create_snapshot│                │   │
│   │                   │                │───────────────────────────────►│   │
│   │                   │                │                │                │   │
│   │                   │                │ unlink_dotfiles│                │   │
│   │                   │                │───────────────►│                │   │
│   │                   │                │                │                │   │
│   │                   │                │ move_to_dormant│                │   │
│   │                   │                │───────────────►│                │   │
│   │                   │                │                │                │   │
│   │                   │                │ link_new_bundle│                │   │
│   │                   │                │───────────────►│                │   │
│   │                   │                │                │                │   │
│   │                   │                │ update_state() │                │   │
│   │                   │                │───────────────►│                │   │
│   │                   │                │                │                │   │
│   │                   │  commit()      │                │                │   │
│   │                   │───────────────►│                │                │   │
│   │                   │                │                │                │   │
│   │  ◄─ success       │                │                │                │   │
│   │◄──────────────────│                │                │                │   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Update Flow with Risk Assessment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         UPDATE DATA FLOW                                     │
│                           iron update                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User        CLI         Core        Pacman       News         Snapshot     │
│   │           │           │            │           │              │          │
│   │  update   │           │            │           │              │          │
│   │──────────►│           │            │           │              │          │
│   │           │           │            │           │              │          │
│   │           │ ──────────────────────────────────────────────────────┐     │
│   │           │    Parallel: check_updates() + fetch_news()           │     │
│   │           │ ◄─────────────────────────────────────────────────────┘     │
│   │           │           │            │           │              │          │
│   │           │           │ checkupdates           │              │          │
│   │           │           │───────────►│           │              │          │
│   │           │           │            │           │              │          │
│   │           │           │ ◄──updates─│           │              │          │
│   │           │           │◄───────────│           │              │          │
│   │           │           │            │           │              │          │
│   │           │           │            │ fetch RSS │              │          │
│   │           │           │────────────────────────►              │          │
│   │           │           │            │           │              │          │
│   │           │           │            │ ◄─ news ──│              │          │
│   │           │           │◄───────────────────────│              │          │
│   │           │           │            │           │              │          │
│   │           │           │            │           │              │          │
│   │           │ assess_risk()         │           │              │          │
│   │           │──────────►│            │           │              │          │
│   │           │           │            │           │              │          │
│   │           │  ◄─ RiskAssessment    │           │              │          │
│   │           │◄──────────│            │           │              │          │
│   │           │           │            │           │              │          │
│   │ ◄─ preview (TUI)     │            │           │              │          │
│   │◄──────────│           │            │           │              │          │
│   │           │           │            │           │              │          │
│   │  approve ─►           │            │           │              │          │
│   │           │           │            │           │              │          │
│   │           │           │            │           │ create       │          │
│   │           │           │────────────────────────────────────────►         │
│   │           │           │            │           │              │          │
│   │           │           │ pacman -Syu│           │              │          │
│   │           │           │───────────►│           │              │          │
│   │           │           │            │           │              │          │
│   │           │           │ ◄─ result ─│           │              │          │
│   │           │           │◄───────────│           │              │          │
│   │           │           │            │           │              │          │
│   │  ◄─ result│           │            │           │              │          │
│   │◄──────────│           │            │           │              │          │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 5.3 First-Time Setup Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      FIRST-TIME SETUP FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐                                                           │
│   │   START     │                                                           │
│   └──────┬──────┘                                                           │
│          │                                                                   │
│          ▼                                                                   │
│   ┌─────────────┐     No      ┌─────────────┐                              │
│   │ State exists?├────────────►│ Show Welcome │                              │
│   └──────┬──────┘              │   Wizard     │                              │
│          │ Yes                 └──────┬──────┘                              │
│          │                            │                                      │
│          ▼                            ▼                                      │
│   ┌─────────────┐              ┌─────────────┐                              │
│   │   Normal    │              │   Detect    │                              │
│   │  Dashboard  │              │  Hardware   │                              │
│   └─────────────┘              └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │  Create     │                              │
│                                │ Host Config │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │  Select     │                              │
│                                │  Bundle     │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │  Select     │                              │
│                                │  Profile    │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │  Install    │                              │
│                                │  Packages   │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │   Apply     │                              │
│                                │  Dotfiles   │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │  Run Post   │                              │
│                                │   Hooks     │                              │
│                                └──────┬──────┘                              │
│                                       │                                      │
│                                       ▼                                      │
│                                ┌─────────────┐                              │
│                                │   Success   │                              │
│                                │  Dashboard  │                              │
│                                └─────────────┘                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. CLI Architecture

### 6.1 Command Structure

```
iron
├── (no command)          → Launch TUI dashboard
├── init                  → Initialize Iron configuration
├── status                → Show system status
├── update                → Safe system update
│   ├── --dry-run         → Preview only
│   ├── --force           → Skip risk assessment
│   └── --yes             → Auto-approve LOW risk
├── doctor                → System health check
├── clean                 → System cleanup
├── recover               → Recovery workflow
├── go                    → Launch TUI
│
├── bundle
│   ├── list              → List available bundles
│   ├── status [id]       → Show bundle status
│   ├── install <id>      → Install a bundle
│   ├── switch <id>       → Switch active bundle
│   └── remove <id>       → Remove a bundle
│
├── profile
│   ├── list              → List available profiles
│   ├── show <id>         → Show profile details
│   ├── select <id>       → Activate a profile
│   ├── create <name>     → Create new profile
│   └── edit <id>         → Edit existing profile
│
├── module
│   ├── list              → List all modules
│   ├── show <id>         → Show module details
│   ├── enable <id>       → Enable a module
│   └── disable <id>      → Disable a module
│
├── host
│   ├── list              → List configured hosts
│   ├── current           → Show current host
│   ├── catalog           → Catalog hardware
│   ├── select <id>       → Select active host
│   └── snapshot          → Create system snapshot
│
├── sync
│   ├── status            → Show sync status
│   ├── push              → Push changes to remote
│   └── pull              → Pull changes from remote
│
└── secrets
    ├── status            → Show secrets status
    ├── unlock            → Decrypt secrets
    ├── lock              → Encrypt secrets
    └── link              → Link secrets to locations
```

### 6.2 Output Formatting

```rust
/// Output format options
pub enum OutputFormat {
    /// Human-readable colored output
    Human,
    /// JSON for scripting
    Json,
    /// Minimal output for piping
    Quiet,
}

/// Status output structure
pub struct StatusOutput {
    pub host: HostStatus,
    pub bundle: BundleStatus,
    pub profile: ProfileStatus,
    pub modules: Vec<ModuleStatus>,
    pub maintenance: MaintenanceStatus,
    pub warnings: Vec<Warning>,
}

impl StatusOutput {
    pub fn render(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Human => self.render_human(),
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap(),
            OutputFormat::Quiet => self.render_minimal(),
        }
    }
}
```

---

## 7. TUI Architecture

### 7.1 View Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TUI VIEW HIERARCHY                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  App                                                                         │
│  ├── Dashboard (Home)                                                       │
│  │   ├── SystemHealthWidget                                                 │
│  │   ├── ActiveConfigWidget                                                 │
│  │   ├── MaintenanceWidget                                                  │
│  │   ├── AlertsWidget                                                       │
│  │   └── QuickActionsWidget                                                 │
│  │                                                                          │
│  ├── Bundles                                                                │
│  │   ├── BundleListWidget                                                   │
│  │   ├── BundleDetailWidget                                                 │
│  │   └── BundleSwitchConfirmDialog                                          │
│  │                                                                          │
│  ├── Profiles                                                               │
│  │   ├── ProfileListWidget                                                  │
│  │   ├── ProfileDetailWidget                                                │
│  │   └── ProfileBuilderWizard                                               │
│  │       ├── NameStep                                                       │
│  │       ├── ModuleSelectionStep                                            │
│  │       ├── ThemeStep                                                      │
│  │       └── ConfirmStep                                                    │
│  │                                                                          │
│  ├── Modules                                                                │
│  │   ├── ModuleListWidget                                                   │
│  │   ├── ModuleDetailWidget                                                 │
│  │   └── ModuleToggleWidget                                                 │
│  │                                                                          │
│  ├── Updates                                                                │
│  │   ├── UpdatePreviewWidget                                                │
│  │   ├── RiskScoreWidget                                                    │
│  │   ├── PackageListWidget                                                  │
│  │   ├── NewsAlertWidget                                                    │
│  │   └── ApprovalDialog                                                     │
│  │                                                                          │
│  ├── Settings                                                               │
│  │   ├── HostSettingsWidget                                                 │
│  │   ├── SyncSettingsWidget                                                 │
│  │   └── PreferencesWidget                                                  │
│  │                                                                          │
│  └── Wizards                                                                │
│      ├── SetupWizard                                                        │
│      │   ├── WelcomeStep                                                    │
│      │   ├── HardwareDetectionStep                                          │
│      │   ├── BundleSelectionStep                                            │
│      │   ├── ProfileSelectionStep                                           │
│      │   └── CompletionStep                                                 │
│      │                                                                      │
│      └── RecoveryWizard                                                     │
│          ├── HostSelectionStep                                              │
│          ├── BundleInstallStep                                              │
│          ├── ProfileApplyStep                                               │
│          └── VerificationStep                                               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Dashboard Layout

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  IRON                                                      Desktop │ Hyprland │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  ┌─ System Health ──────────┐  ┌─ Active Configuration ────────────────────┐ │
│  │                          │  │                                           │ │
│  │  ● System OK             │  │  Host:     desktop                        │ │
│  │  ● 245 packages managed  │  │  Bundle:   Hyprland (active)              │ │
│  │  ● No conflicts          │  │  Profile:  Developer                      │ │
│  │  ● Snapshot: 2h ago      │  │  Modules:  12 enabled                     │ │
│  │                          │  │  Theme:    Catppuccin Mocha               │ │
│  └──────────────────────────┘  └───────────────────────────────────────────┘ │
│                                                                               │
│  ┌─ Maintenance ────────────┐  ┌─ Alerts ──────────────────────────────────┐ │
│  │                          │  │                                           │ │
│  │  Last Update:  3 days    │  │  ⚠ 15 updates available                  │ │
│  │  Last Clean:   1 week    │  │  ⚠ 1 Arch News item requires attention   │ │
│  │  Last Doctor:  2 weeks   │  │  ● No conflicts detected                 │ │
│  │  Last Sync:    1 hour    │  │                                           │ │
│  │                          │  │                                           │ │
│  └──────────────────────────┘  └───────────────────────────────────────────┘ │
│                                                                               │
│  ┌─ Quick Actions ──────────────────────────────────────────────────────────┐ │
│  │                                                                          │ │
│  │  [U] Update System    [B] Bundles    [P] Profiles    [M] Modules        │ │
│  │  [D] Doctor           [C] Clean      [S] Sync        [?] Help           │ │
│  │                                                                          │ │
│  └──────────────────────────────────────────────────────────────────────────┘ │
│                                                                               │
├──────────────────────────────────────────────────────────────────────────────┤
│  [q] Quit  [?] Help  [Tab] Navigate  [Enter] Select                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 7.3 Event Handling

```rust
/// TUI event types
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick for animations/updates
    Tick,
    /// Background task completed
    TaskComplete(TaskId, TaskResult),
}

/// Event handler
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self;
    pub async fn next(&mut self) -> Option<Event>;
}

/// App state management
pub struct App {
    state: AppState,
    view: View,
    should_quit: bool,
}

impl App {
    pub fn handle_event(&mut self, event: Event) -> Result<Option<Action>> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(w, h) => self.handle_resize(w, h),
            Event::Tick => self.handle_tick(),
            Event::TaskComplete(id, result) => self.handle_task_complete(id, result),
        }
    }
}
```

---

## 8. External Integrations

### 8.1 Integration Points

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        EXTERNAL INTEGRATIONS                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         PACKAGE MANAGEMENT                           │    │
│  │                                                                      │    │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐           │    │
│  │  │    pacman    │    │  paru / yay  │    │   flatpak    │           │    │
│  │  │              │    │              │    │   (future)   │           │    │
│  │  │ • Query      │    │ • AUR search │    │              │           │    │
│  │  │ • Install    │    │ • AUR build  │    │              │           │    │
│  │  │ • Remove     │    │ • Flags      │    │              │           │    │
│  │  │ • Update     │    │              │    │              │           │    │
│  │  └──────────────┘    └──────────────┘    └──────────────┘           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         VERSION CONTROL                              │    │
│  │                                                                      │    │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐           │    │
│  │  │     git      │    │  git-crypt   │    │     age      │           │    │
│  │  │              │    │              │    │  (alt)       │           │    │
│  │  │ • Clone      │    │ • Encrypt    │    │              │           │    │
│  │  │ • Commit     │    │ • Decrypt    │    │ • Encrypt    │           │    │
│  │  │ • Push/Pull  │    │ • Status     │    │ • Decrypt    │           │    │
│  │  │ • Status     │    │              │    │              │           │    │
│  │  └──────────────┘    └──────────────┘    └──────────────┘           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         SYSTEM SERVICES                              │    │
│  │                                                                      │    │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐           │    │
│  │  │   systemd    │    │  timeshift   │    │   snapper    │           │    │
│  │  │              │    │              │    │              │           │    │
│  │  │ • Enable     │    │ • Create     │    │ • Create     │           │    │
│  │  │ • Disable    │    │ • List       │    │ • List       │           │    │
│  │  │ • Status     │    │ • Restore    │    │ • Rollback   │           │    │
│  │  │ • Timers     │    │ • Delete     │    │ • Delete     │           │    │
│  │  └──────────────┘    └──────────────┘    └──────────────┘           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         EXTERNAL DATA                                │    │
│  │                                                                      │    │
│  │  ┌──────────────┐    ┌──────────────┐                               │    │
│  │  │  Arch News   │    │  AUR API     │                               │    │
│  │  │  (RSS)       │    │              │                               │    │
│  │  │              │    │ • Search     │                               │    │
│  │  │ • Fetch      │    │ • Info       │                               │    │
│  │  │ • Parse      │    │ • Flagged    │                               │    │
│  │  │ • Cache      │    │              │                               │    │
│  │  └──────────────┘    └──────────────┘                               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Integration Interfaces

```rust
/// Pacman integration trait
pub trait PackageManager {
    fn query_installed(&self) -> Result<Vec<InstalledPackage>>;
    fn query_available(&self, name: &str) -> Result<Vec<AvailablePackage>>;
    fn install(&self, packages: &[&str]) -> Result<InstallResult>;
    fn remove(&self, packages: &[&str]) -> Result<RemoveResult>;
    fn update(&self, packages: Option<&[&str]>) -> Result<UpdateResult>;
    fn check_updates(&self) -> Result<Vec<AvailableUpdate>>;
}

/// Snapshot integration trait
pub trait SnapshotManager {
    fn create(&self, description: &str) -> Result<Snapshot>;
    fn list(&self) -> Result<Vec<Snapshot>>;
    fn restore(&self, id: &str) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}

/// Git integration trait
pub trait GitManager {
    fn status(&self) -> Result<GitStatus>;
    fn commit(&self, message: &str) -> Result<()>;
    fn push(&self, remote: &str, branch: &str) -> Result<()>;
    fn pull(&self, remote: &str, branch: &str) -> Result<PullResult>;
    fn diff(&self) -> Result<String>;
}

/// Secrets integration trait
pub trait SecretsManager {
    fn is_unlocked(&self) -> bool;
    fn unlock(&self, key: &str) -> Result<()>;
    fn lock(&self) -> Result<()>;
    fn list_secrets(&self) -> Result<Vec<SecretInfo>>;
}
```

### 8.3 Resilience Patterns

#### Circuit Breaker for External Commands

All external command execution (pacman, git, systemctl, timeshift/snapper) uses a circuit breaker pattern to prevent hangs and enable graceful degradation.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CIRCUIT BREAKER PATTERN                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌──────────────┐                                                          │
│   │    CLOSED    │ ◄─────────────────────────────────────┐                  │
│   │  (Normal)    │                                       │                  │
│   └──────┬───────┘                                       │                  │
│          │                                               │                  │
│          │ failure_count >= 3                    success │                  │
│          │ OR timeout (120s)                             │                  │
│          ▼                                               │                  │
│   ┌──────────────┐         timeout (30s)         ┌──────────────┐          │
│   │     OPEN     │ ─────────────────────────────►│  HALF-OPEN   │          │
│   │  (Failing)   │                               │   (Testing)  │          │
│   └──────────────┘                               └──────┬───────┘          │
│          │                                               │                  │
│          │                                       failure │                  │
│          │                                               │                  │
│          └───────────────────────────────────────────────┘                  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Implementation:**

```rust
/// Circuit breaker for external commands
pub struct CommandCircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    timeout: Duration,
    reset_timeout: Duration,
}

#[derive(Debug, Clone, Copy)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing fast, not executing commands
    HalfOpen,  // Testing if service recovered
}

impl CommandCircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            timeout: Duration::from_secs(120),      // Command timeout
            reset_timeout: Duration::from_secs(30), // Wait before retry
        }
    }

    pub async fn execute<F, T>(&mut self, command: F) -> Result<T, CommandError>
    where
        F: Future<Output = Result<T, CommandError>>,
    {
        match self.state {
            CircuitState::Open => {
                if self.should_attempt_reset() {
                    self.state = CircuitState::HalfOpen;
                } else {
                    return Err(CommandError::CircuitOpen);
                }
            }
            _ => {}
        }

        match tokio::time::timeout(self.timeout, command).await {
            Ok(Ok(result)) => {
                self.record_success();
                Ok(result)
            }
            Ok(Err(e)) => {
                self.record_failure();
                Err(e)
            }
            Err(_) => {
                self.record_failure();
                Err(CommandError::Timeout)
            }
        }
    }
}
```

**Configuration:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `command_timeout` | 120s | Maximum time for single command execution |
| `failure_threshold` | 3 | Failures before circuit opens |
| `reset_timeout` | 30s | Wait time before attempting recovery |
| `half_open_max_calls` | 1 | Calls allowed in half-open state |

#### Graceful Degradation

When external services fail, Iron degrades gracefully:

| Service | Degraded Behavior |
|---------|-------------------|
| pacman | Return cached package info, warn user, disable update commands |
| git | Disable sync commands, local operations continue |
| timeshift/snapper | Warn user, allow operations without snapshots (with confirmation) |
| Arch News RSS | Use cached news, show "news unavailable" warning |
| AUR API | Disable AUR package info, continue with official packages |

---

## 9. File System Layout

### 9.1 Repository Structure

```
~/.config/iron/                      # Default Iron root
├── .git/                            # Git repository
├── .gitattributes                   # git-crypt configuration
│
├── bundles/                         # Desktop environment bundles
│   ├── hyprland/
│   │   ├── bundle.toml              # Bundle manifest
│   │   ├── dotfiles/                # Bundle-specific dotfiles
│   │   │   ├── hypr/
│   │   │   ├── waybar/
│   │   │   └── wofi/
│   │   └── scripts/
│   │       └── setup.sh
│   └── niri/
│       ├── bundle.toml
│       ├── dotfiles/
│       └── scripts/
│
├── profiles/                        # Dotfile profiles
│   ├── developer/
│   │   └── profile.toml
│   ├── minimal/
│   │   └── profile.toml
│   └── gaming/
│       └── profile.toml
│
├── modules/                         # Reusable modules
│   ├── nvim-ide/
│   │   ├── module.toml
│   │   └── config/
│   │       └── nvim/
│   ├── kitty-dev/
│   │   ├── module.toml
│   │   └── config/
│   │       └── kitty/
│   └── fish-config/
│       ├── module.toml
│       └── config/
│           └── fish/
│
├── hosts/                           # Host configurations
│   ├── desktop/
│   │   └── host.toml
│   └── laptop/
│       └── host.toml
│
├── dormant/                         # Inactive bundle configs
│   └── niri/                        # Stored when not active
│       └── .config/
│
├── secrets/                         # Encrypted secrets
│   ├── ssh/
│   ├── gpg/
│   └── tokens/
│
├── scripts/                         # Shared scripts
│   └── lib/
│       └── common.sh
│
└── .iron/                           # Runtime state (gitignored)
    ├── state/
    │   ├── current_host.json
    │   ├── active_bundle.json
    │   ├── active_profile.json
    │   ├── enabled_modules.json
    │   └── maintenance.json
    ├── cache/
    │   ├── arch_news.json
    │   └── aur_info.json
    └── logs/
        └── operations.jsonl
```

### 9.2 Symlink Structure

```
# When Hyprland bundle + Developer profile is active:

~/.config/
├── hypr/           → ~/.config/iron/bundles/hyprland/dotfiles/hypr/
├── waybar/         → ~/.config/iron/bundles/hyprland/dotfiles/waybar/
├── wofi/           → ~/.config/iron/bundles/hyprland/dotfiles/wofi/
├── nvim/           → ~/.config/iron/modules/nvim-ide/config/nvim/
├── kitty/          → ~/.config/iron/modules/kitty-dev/config/kitty/
└── fish/           → ~/.config/iron/modules/fish-config/config/fish/

~/.ssh/
└── *               → ~/.config/iron/secrets/ssh/* (when unlocked)
```

---

## 10. Security Model

### 10.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| Secrets in git history | git-crypt encryption before commit |
| Malicious dotfiles | No code execution by default, hooks require approval |
| Package tampering | Pacman signature verification (system level) |
| Privilege escalation | Minimal sudo usage, documented explicitly |
| State corruption | Transaction support, atomic operations |

### 10.2 Secrets Management

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SECRETS LIFECYCLE                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐                │
│   │  Encrypted  │      │  Unlocked   │      │   Linked    │                │
│   │  (at rest)  │─────►│  (in memory)│─────►│  (active)   │                │
│   └─────────────┘      └─────────────┘      └─────────────┘                │
│         │                    │                    │                         │
│         │ git-crypt          │ iron secrets       │ symlinks                │
│         │ filter             │ unlock             │ created                 │
│         │                    │                    │                         │
│         ▼                    ▼                    ▼                         │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐                │
│   │  secrets/   │      │  /tmp/iron/ │      │  ~/.ssh/    │                │
│   │  (encrypted)│      │  (decrypted)│      │  (symlink)  │                │
│   └─────────────┘      └─────────────┘      └─────────────┘                │
│                                                                              │
│   On lock:                                                                  │
│   • Remove symlinks                                                         │
│   • Clear /tmp/iron/                                                        │
│   • Secrets remain encrypted in repo                                        │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 10.3 Permission Model

```rust
/// Actions requiring elevated privileges
pub enum PrivilegedAction {
    /// Install/remove packages
    PackageManagement,
    /// Enable/disable system services
    SystemdSystem,
    /// Create system-wide symlinks
    SystemConfig,
    /// Restore system snapshot
    SnapshotRestore,
}

impl PrivilegedAction {
    /// Check if action needs sudo
    pub fn needs_sudo(&self) -> bool {
        matches!(self,
            Self::PackageManagement |
            Self::SystemdSystem |
            Self::SystemConfig |
            Self::SnapshotRestore
        )
    }
}
```

---

## 11. Error Handling

### 11.1 Error Types

```rust
/// Top-level error type
#[derive(Debug, thiserror::Error)]
pub enum IronError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error("Package error: {0}")]
    Package(#[from] PackageError),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FsError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Configuration-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("File not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Parse error in {path}: {message}")]
    ParseError { path: PathBuf, message: String },

    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },
}

/// State-specific errors
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("No active host configured")]
    NoActiveHost,

    #[error("No active bundle for host {host}")]
    NoActiveBundle { host: String },

    #[error("Bundle {id} not found")]
    BundleNotFound { id: String },

    #[error("Bundle {id} is already active")]
    BundleAlreadyActive { id: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },
}
```

### 11.2 Error Recovery

```rust
/// Recovery strategy for errors
pub trait Recoverable {
    /// Attempt automatic recovery
    fn auto_recover(&self) -> Option<RecoveryAction>;

    /// Suggest manual recovery steps
    fn manual_recovery(&self) -> Vec<RecoveryStep>;

    /// Can operation be retried?
    fn is_retriable(&self) -> bool;
}

/// Recovery action types
pub enum RecoveryAction {
    /// Rollback to previous state
    Rollback(StateSnapshot),

    /// Retry operation with different parameters
    Retry(RetryConfig),

    /// Skip this item and continue
    Skip,

    /// Abort and cleanup
    Abort,
}
```

---

## 12. Testing Strategy

### 12.1 Test Pyramid

```
                    ┌─────────────────┐
                    │    E2E Tests    │   10%
                    │  (iron binary)  │
                    └────────┬────────┘
                             │
                    ┌────────┴────────┐
                    │ Integration     │   30%
                    │ Tests           │
                    │ (crate combos)  │
                    └────────┬────────┘
                             │
           ┌─────────────────┴─────────────────┐
           │          Unit Tests               │   60%
           │   (individual functions/types)    │
           └───────────────────────────────────┘
```

### 12.2 Test Categories

| Category | Location | Description |
|----------|----------|-------------|
| Unit | `crates/*/src/**/*_test.rs` | Individual function tests |
| Integration | `tests/integration/` | Cross-crate tests |
| E2E | `tests/e2e/` | Full binary tests |
| Fixtures | `tests/fixtures/` | Test data and configs |

### 12.3 Test Utilities

```rust
/// Test fixture builder
pub struct TestFixture {
    root: TempDir,
    bundles: Vec<Bundle>,
    profiles: Vec<Profile>,
    modules: Vec<Module>,
}

impl TestFixture {
    pub fn new() -> Self;
    pub fn with_bundle(self, bundle: Bundle) -> Self;
    pub fn with_profile(self, profile: Profile) -> Self;
    pub fn with_module(self, module: Module) -> Self;
    pub fn build(self) -> Result<TestContext>;
}

/// Mock implementations for testing
pub mod mocks {
    pub struct MockPackageManager;
    pub struct MockSnapshotManager;
    pub struct MockGitManager;
}
```

---

## Appendix A: API Reference

See [API.md](./API.md) for detailed API documentation.

## Appendix B: Configuration Reference

See [CONFIG.md](./CONFIG.md) for configuration file formats.

## Appendix C: Migration Guide

See [MIGRATION.md](./MIGRATION.md) for migrating from jff-arch-config.

---

**Document History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-02-12 | Architecture Session | Initial architecture |
| 1.0.1 | 2025-02-12 | Documentation Update | Finalized after Phase 6 completion |
