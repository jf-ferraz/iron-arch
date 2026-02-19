# Iron Implementation Plan

> **Document Status**: IN PROGRESS
> **Version**: 2.0.0
> **Created**: 2025-02-12
> **Last Updated**: 2026-02-13
> **Estimated Duration**: 16-20 weeks
> **Current Status**: Phase 8 In Progress - Production Hardening (1342 tests, 56.75% coverage)

---

## Executive Summary

This implementation plan provides a structured approach to building the Iron configuration management platform. The plan is organized into 7 phases with clear dependencies, validation checkpoints, and deliverables.

### Implementation Philosophy

1. **Foundation First** - Build robust core before features
2. **Test-Driven** - Write tests alongside implementation
3. **Incremental Value** - Each phase delivers usable functionality
4. **Dependency Order** - Infrastructure before application layer

---

## Implementation Timeline

```
Week  1-2   │████████│ Phase 1: Foundation ✅
Week  3-4   │████████│ Phase 2: Infrastructure ✅
Week  5-6   │████████│ Phase 3: Core Services ✅
Week  7-8   │████████│ Phase 4: CLI Implementation ✅
Week  9-11  │████████████│ Phase 5: TUI Implementation ✅
Week 12-13  │████████│ Phase 6: Integration & Flows ✅
Week 14-16  │████████████│ Phase 7: Polish & Release ✅
Week 17-20  │████████████████│ Phase 8: Production Hardening 🚧 ← CURRENT
Week 21-22  │████████│ Phase 9: v1.0.0 Release
```

---

## Phase 1: Foundation (Week 1-2)

### Objective
Establish domain models, error types, and workspace configuration.

### Tasks

#### 1.1 Workspace Setup
| Task | Description | Estimated |
|------|-------------|-----------|
| 1.1.1 | Configure Cargo workspace with all 7 crates | 2h |
| 1.1.2 | Set up shared dependencies in workspace Cargo.toml | 1h |
| 1.1.3 | Configure rustfmt.toml and clippy.toml | 30m |
| 1.1.4 | Set up CI/CD with GitHub Actions | 2h |
| 1.1.5 | Configure code coverage with tarpaulin | 1h |

**Dependencies**: None
**Deliverable**: Building workspace with CI pipeline

#### 1.2 Domain Models (iron-core)
| Task | Description | Estimated |
|------|-------------|-----------|
| 1.2.1 | Define `Host` struct with hardware catalog | 3h |
| 1.2.2 | Define `Bundle` struct with state machine | 3h |
| 1.2.3 | Define `Profile` struct with inheritance | 2h |
| 1.2.4 | Define `Module` struct with dotfile mappings | 2h |
| 1.2.5 | Define `DotfileMapping` value object | 1h |
| 1.2.6 | Implement enums: ChassisType, BundleType, ModuleKind | 2h |
| 1.2.7 | Implement state enums: BundleState, ModuleState | 2h |
| 1.2.8 | Write unit tests for all domain models | 4h |

**Dependencies**: 1.1
**Deliverable**: Tested domain model library

#### 1.3 Error System
| Task | Description | Estimated |
|------|-------------|-----------|
| 1.3.1 | Define `IronError` top-level error enum | 2h |
| 1.3.2 | Define `ConfigError` with parse/validation errors | 1h |
| 1.3.3 | Define `StateError` with state-specific errors | 1h |
| 1.3.4 | Define `PackageError`, `GitError`, `FsError` | 2h |
| 1.3.5 | Implement `Recoverable` trait for errors | 2h |
| 1.3.6 | Write error conversion tests | 2h |

**Dependencies**: 1.2
**Deliverable**: Comprehensive error handling system

#### 1.4 Validation Layer
| Task | Description | Estimated |
|------|-------------|-----------|
| 1.4.1 | Create `validation` module in iron-core | 1h |
| 1.4.2 | Implement ID validation (alphanumeric + hyphen) | 1h |
| 1.4.3 | Implement path validation (no escape) | 1h |
| 1.4.4 | Implement conflict detection for modules | 2h |
| 1.4.5 | Implement dependency resolution for modules | 3h |
| 1.4.6 | Write validation unit tests | 2h |

**Dependencies**: 1.2, 1.3
**Deliverable**: Input validation library

### Phase 1 Checkpoint ✅ COMPLETE

```
✓ Workspace builds with `cargo build --workspace`
✓ All domain models have serde Serialize/Deserialize
✓ Error types implement std::error::Error
✓ Unit test coverage ≥ 80% for domain models (62 tests)
✓ CI pipeline passes all checks
```

---

## Phase 2: Infrastructure (Week 3-4)

### Objective
Build infrastructure crates for external system integration.

### Tasks

#### 2.1 Filesystem Operations (iron-fs)
| Task | Description | Estimated |
|------|-------------|-----------|
| 2.1.1 | Create iron-fs crate structure | 1h |
| 2.1.2 | Implement TOML parser for config files | 3h |
| 2.1.3 | Implement symlink manager (create/remove/status) | 4h |
| 2.1.4 | Implement backup manager (copy with timestamp) | 2h |
| 2.1.5 | Implement atomic file operations | 2h |
| 2.1.6 | Implement directory traversal utilities | 2h |
| 2.1.7 | Add path expansion (~, env vars) | 1h |
| 2.1.8 | Write integration tests with temp dirs | 4h |

**Dependencies**: 1.2
**Deliverable**: Filesystem abstraction layer

#### 2.2 Package Management (iron-pacman)
| Task | Description | Estimated |
|------|-------------|-----------|
| 2.2.1 | Create iron-pacman crate structure | 1h |
| 2.2.2 | Define `PackageManager` trait | 2h |
| 2.2.3 | Implement pacman command wrapper | 4h |
| 2.2.4 | Implement AUR helper detection (paru/yay) | 2h |
| 2.2.5 | Implement `check_updates()` using checkupdates | 2h |
| 2.2.6 | Implement package query (installed/available) | 2h |
| 2.2.7 | Implement risk assessment algorithm | 4h |
| 2.2.8 | Implement Arch News RSS parser | 3h |
| 2.2.9 | Write mock tests for package operations | 3h |

**Dependencies**: 1.3
**Deliverable**: Package management abstraction

#### 2.3 Git Operations (iron-git)
| Task | Description | Estimated |
|------|-------------|-----------|
| 2.3.1 | Create iron-git crate structure | 1h |
| 2.3.2 | Define `GitManager` trait | 1h |
| 2.3.3 | Implement git command wrapper | 3h |
| 2.3.4 | Implement status/diff operations | 2h |
| 2.3.5 | Implement commit/push/pull operations | 3h |
| 2.3.6 | Implement git-crypt status/unlock/lock | 3h |
| 2.3.7 | Write integration tests with test repo | 3h |

**Dependencies**: 1.3
**Deliverable**: Git operations abstraction

#### 2.4 Systemd Integration (iron-systemd)
| Task | Description | Estimated |
|------|-------------|-----------|
| 2.4.1 | Create iron-systemd crate structure | 1h |
| 2.4.2 | Define `ServiceManager` trait | 1h |
| 2.4.3 | Implement systemctl command wrapper | 3h |
| 2.4.4 | Implement service enable/disable/status | 2h |
| 2.4.5 | Implement user vs system service handling | 2h |
| 2.4.6 | Write mock tests for service operations | 2h |

**Dependencies**: 1.3
**Deliverable**: Systemd abstraction layer

#### 2.5 Snapshot Integration
| Task | Description | Estimated |
|------|-------------|-----------|
| 2.5.1 | Define `SnapshotManager` trait in iron-core | 1h |
| 2.5.2 | Implement timeshift backend | 3h |
| 2.5.3 | Implement snapper backend | 3h |
| 2.5.4 | Implement auto-detection of available backend | 1h |
| 2.5.5 | Write snapshot tests (mocked) | 2h |

**Dependencies**: 1.3
**Deliverable**: Snapshot management abstraction

### Phase 2 Checkpoint ✅ COMPLETE

```
✓ iron-fs can parse all TOML config formats
✓ iron-fs symlink operations work correctly
✓ iron-pacman can query installed packages
✓ iron-pacman risk assessment returns correct levels
✓ iron-git can detect repository status
✓ All infrastructure crates have tests (fs:3, pacman:12, git:9, systemd:3)
```

---

## Phase 3: Core Services (Week 5-6)

### Objective
Implement application services and state management.

### Tasks

#### 3.1 State Management
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.1.1 | Define state file JSON schemas | 2h |
| 3.1.2 | Implement `StateManager` struct | 4h |
| 3.1.3 | Implement state loading from disk | 2h |
| 3.1.4 | Implement state persistence | 2h |
| 3.1.5 | Implement transaction support | 4h |
| 3.1.6 | Implement operations audit log | 2h |
| 3.1.7 | Write state management tests | 3h |

**Dependencies**: 2.1
**Deliverable**: Robust state management system

#### 3.2 Host Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.2.1 | Define `HostService` trait | 1h |
| 3.2.2 | Implement hardware detection (CPU/GPU/RAM) | 3h |
| 3.2.3 | Implement monitor detection (Wayland outputs) | 2h |
| 3.2.4 | Implement chassis type detection | 1h |
| 3.2.5 | Implement host matching by hostname | 1h |
| 3.2.6 | Implement host TOML persistence | 2h |
| 3.2.7 | Write host service tests | 2h |

**Dependencies**: 3.1, 2.1
**Deliverable**: Host management service

#### 3.3 Bundle Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.3.1 | Define `BundleService` trait | 1h |
| 3.3.2 | Implement bundle discovery from filesystem | 2h |
| 3.3.3 | Implement bundle installation | 3h |
| 3.3.4 | Implement bundle activation (link dotfiles) | 3h |
| 3.3.5 | Implement bundle deactivation (move to dormant) | 3h |
| 3.3.6 | Implement bundle switch workflow | 4h |
| 3.3.7 | Implement conflict detection | 2h |
| 3.3.8 | Write bundle service tests | 4h |

**Dependencies**: 3.1, 2.1, 2.2
**Deliverable**: Bundle management service

#### 3.4 Profile Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.4.1 | Define `ProfileService` trait | 1h |
| 3.4.2 | Implement profile discovery | 2h |
| 3.4.3 | Implement profile inheritance resolution | 3h |
| 3.4.4 | Implement profile selection | 2h |
| 3.4.5 | Implement effective modules calculation | 2h |
| 3.4.6 | Implement profile creation | 2h |
| 3.4.7 | Write profile service tests | 3h |

**Dependencies**: 3.1, 3.5
**Deliverable**: Profile management service

#### 3.5 Module Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.5.1 | Define `ModuleService` trait | 1h |
| 3.5.2 | Implement module discovery | 2h |
| 3.5.3 | Implement module enable (link dotfiles) | 3h |
| 3.5.4 | Implement module disable (unlink) | 2h |
| 3.5.5 | Implement module conflict checking | 2h |
| 3.5.6 | Implement pre/post hook execution | 3h |
| 3.5.7 | Write module service tests | 3h |

**Dependencies**: 3.1, 2.1
**Deliverable**: Module management service

#### 3.6 Update Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.6.1 | Define `UpdateService` trait | 1h |
| 3.6.2 | Implement update checking | 2h |
| 3.6.3 | Implement risk assessment orchestration | 3h |
| 3.6.4 | Implement Arch News integration | 2h |
| 3.6.5 | Implement update execution with snapshot | 4h |
| 3.6.6 | Implement pacnew detection | 2h |
| 3.6.7 | Write update service tests | 3h |

**Dependencies**: 3.1, 2.2, 2.5
**Deliverable**: Safe update service

#### 3.7 Sync Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.7.1 | Define `SyncService` trait | 1h |
| 3.7.2 | Implement sync status detection | 2h |
| 3.7.3 | Implement push workflow | 2h |
| 3.7.4 | Implement pull workflow | 3h |
| 3.7.5 | Implement conflict detection | 2h |
| 3.7.6 | Write sync service tests | 2h |

**Dependencies**: 3.1, 2.3
**Deliverable**: Git sync service

#### 3.8 Secrets Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.8.1 | Define `SecretsService` trait | 1h |
| 3.8.2 | Implement secrets status detection | 2h |
| 3.8.3 | Implement unlock workflow | 2h |
| 3.8.4 | Implement lock workflow | 1h |
| 3.8.5 | Implement secrets linking | 2h |
| 3.8.6 | Write secrets service tests | 2h |

**Dependencies**: 3.1, 2.3
**Deliverable**: Secrets management service

#### 3.9 Recovery Service
| Task | Description | Estimated |
|------|-------------|-----------|
| 3.9.1 | Define `RecoveryService` trait | 1h |
| 3.9.2 | Implement install script generation | 4h |
| 3.9.3 | Implement state export | 2h |
| 3.9.4 | Implement installation verification | 3h |
| 3.9.5 | Write recovery service tests | 2h |

**Dependencies**: 3.2, 3.3, 3.4
**Deliverable**: Recovery workflow service

### Phase 3 Checkpoint ✅ COMPLETE

```
✓ StateManager handles transactions correctly
✓ BundleService can switch between bundles
✓ ProfileService resolves inheritance correctly
✓ ModuleService detects and prevents conflicts
✓ UpdateService calculates risk scores correctly
✓ All 8 services implemented with tests (62 core tests)
✓ Services integrate with infrastructure crates
```

---

## Phase 4: CLI Implementation (Week 7-8)

### Objective
Build complete CLI interface with all commands.

### Tasks

#### 4.1 CLI Framework
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.1.1 | Create iron-cli crate structure | 1h |
| 4.1.2 | Set up clap with derive macros | 2h |
| 4.1.3 | Define top-level command enum | 2h |
| 4.1.4 | Implement global flags (--json, --quiet, --verbose) | 2h |
| 4.1.5 | Implement output formatting trait | 2h |
| 4.1.6 | Set up colored terminal output | 1h |

**Dependencies**: 3.x
**Deliverable**: CLI framework scaffold

#### 4.2 Core Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.2.1 | Implement `iron init` command | 2h |
| 4.2.2 | Implement `iron status` command | 2h |
| 4.2.3 | Implement `iron doctor` command | 3h |
| 4.2.4 | Implement `iron clean` command | 2h |
| 4.2.5 | Implement `iron go` (launch TUI) | 1h |
| 4.2.6 | Write CLI command tests | 3h |

**Dependencies**: 4.1
**Deliverable**: Core CLI commands

#### 4.3 Bundle Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.3.1 | Implement `iron bundle list` | 1h |
| 4.3.2 | Implement `iron bundle status` | 1h |
| 4.3.3 | Implement `iron bundle install` | 2h |
| 4.3.4 | Implement `iron bundle switch` | 2h |
| 4.3.5 | Implement `iron bundle remove` | 2h |
| 4.3.6 | Write bundle command tests | 2h |

**Dependencies**: 4.1, 3.3
**Deliverable**: Bundle CLI commands

#### 4.4 Profile Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.4.1 | Implement `iron profile list` | 1h |
| 4.4.2 | Implement `iron profile show` | 1h |
| 4.4.3 | Implement `iron profile select` | 2h |
| 4.4.4 | Implement `iron profile create` | 2h |
| 4.4.5 | Implement `iron profile edit` | 2h |
| 4.4.6 | Write profile command tests | 2h |

**Dependencies**: 4.1, 3.4
**Deliverable**: Profile CLI commands

#### 4.5 Module Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.5.1 | Implement `iron module list` | 1h |
| 4.5.2 | Implement `iron module show` | 1h |
| 4.5.3 | Implement `iron module enable` | 2h |
| 4.5.4 | Implement `iron module disable` | 1h |
| 4.5.5 | Write module command tests | 2h |

**Dependencies**: 4.1, 3.5
**Deliverable**: Module CLI commands

#### 4.6 Host Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.6.1 | Implement `iron host list` | 1h |
| 4.6.2 | Implement `iron host current` | 1h |
| 4.6.3 | Implement `iron host catalog` | 2h |
| 4.6.4 | Implement `iron host select` | 1h |
| 4.6.5 | Implement `iron host snapshot` | 2h |
| 4.6.6 | Write host command tests | 2h |

**Dependencies**: 4.1, 3.2
**Deliverable**: Host CLI commands

#### 4.7 Update & Sync Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.7.1 | Implement `iron update` with preview | 3h |
| 4.7.2 | Implement `iron sync status` | 1h |
| 4.7.3 | Implement `iron sync push` | 2h |
| 4.7.4 | Implement `iron sync pull` | 2h |
| 4.7.5 | Write update/sync command tests | 2h |

**Dependencies**: 4.1, 3.6, 3.7
**Deliverable**: Update and sync CLI commands

#### 4.8 Secrets & Recovery Commands
| Task | Description | Estimated |
|------|-------------|-----------|
| 4.8.1 | Implement `iron secrets status` | 1h |
| 4.8.2 | Implement `iron secrets unlock` | 2h |
| 4.8.3 | Implement `iron secrets lock` | 1h |
| 4.8.4 | Implement `iron secrets link` | 2h |
| 4.8.5 | Implement `iron recover` command | 3h |
| 4.8.6 | Write secrets/recovery command tests | 2h |

**Dependencies**: 4.1, 3.8, 3.9
**Deliverable**: Secrets and recovery CLI commands

### Phase 4 Checkpoint ✅ COMPLETE

```
✓ All CLI commands return correct exit codes
✓ JSON output mode works for all commands
✓ Verbose mode shows detailed progress
✓ Error messages are user-friendly
✓ CLI help text is comprehensive
✓ CLI integration tests pass (54 tests)
```

---

## Phase 5: TUI Implementation (Week 9-11)

### Objective
Build full TUI dashboard with wizards and interactive features.

### Tasks

#### 5.1 TUI Framework
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.1.1 | Create iron-tui crate structure | 1h |
| 5.1.2 | Set up ratatui with crossterm backend | 2h |
| 5.1.3 | Implement App struct with event loop | 3h |
| 5.1.4 | Implement View trait and view registry | 2h |
| 5.1.5 | Implement EventHandler for keyboard/resize | 3h |
| 5.1.6 | Implement background task manager | 3h |
| 5.1.7 | Set up terminal initialization/cleanup | 1h |

**Dependencies**: 4.x
**Deliverable**: TUI framework scaffold

#### 5.2 Common Widgets
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.2.1 | Implement header widget (title, host, bundle) | 2h |
| 5.2.2 | Implement footer widget (key bindings) | 1h |
| 5.2.3 | Implement status badge widget (OK/Warning/Error) | 2h |
| 5.2.4 | Implement risk score widget (colored badge) | 2h |
| 5.2.5 | Implement scrollable list widget | 3h |
| 5.2.6 | Implement confirmation dialog widget | 2h |
| 5.2.7 | Implement progress bar widget | 1h |
| 5.2.8 | Implement text input widget | 2h |

**Dependencies**: 5.1
**Deliverable**: Reusable widget library

#### 5.3 Dashboard View
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.3.1 | Implement dashboard layout | 3h |
| 5.3.2 | Implement SystemHealthWidget | 2h |
| 5.3.3 | Implement ActiveConfigWidget | 2h |
| 5.3.4 | Implement MaintenanceWidget | 2h |
| 5.3.5 | Implement AlertsWidget | 2h |
| 5.3.6 | Implement QuickActionsWidget | 2h |
| 5.3.7 | Wire dashboard navigation | 2h |

**Dependencies**: 5.2
**Deliverable**: Dashboard home view

#### 5.4 Bundle Views
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.4.1 | Implement BundleListView | 3h |
| 5.4.2 | Implement BundleDetailView | 2h |
| 5.4.3 | Implement BundleSwitchConfirmDialog | 2h |
| 5.4.4 | Implement bundle switch progress view | 2h |
| 5.4.5 | Wire bundle view navigation | 1h |

**Dependencies**: 5.2, 3.3
**Deliverable**: Bundle management views

#### 5.5 Profile Views
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.5.1 | Implement ProfileListView | 3h |
| 5.5.2 | Implement ProfileDetailView | 2h |
| 5.5.3 | Implement ProfileBuilderWizard | 6h |
| 5.5.4 | Wire profile view navigation | 1h |

**Dependencies**: 5.2, 3.4
**Deliverable**: Profile management views

#### 5.6 Module Views
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.6.1 | Implement ModuleListView | 3h |
| 5.6.2 | Implement ModuleDetailView | 2h |
| 5.6.3 | Implement ModuleToggleWidget | 2h |
| 5.6.4 | Wire module view navigation | 1h |

**Dependencies**: 5.2, 3.5
**Deliverable**: Module management views

#### 5.7 Update View
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.7.1 | Implement UpdatePreviewView layout | 3h |
| 5.7.2 | Implement RiskScoreWidget (large) | 2h |
| 5.7.3 | Implement PackageListWidget | 2h |
| 5.7.4 | Implement NewsAlertWidget | 2h |
| 5.7.5 | Implement ApprovalDialog | 2h |
| 5.7.6 | Implement update progress view | 2h |
| 5.7.7 | Wire update view workflow | 2h |

**Dependencies**: 5.2, 3.6
**Deliverable**: Safe update TUI flow

#### 5.8 Wizards
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.8.1 | Implement SetupWizard framework | 3h |
| 5.8.2 | Implement WelcomeStep | 2h |
| 5.8.3 | Implement HardwareDetectionStep | 3h |
| 5.8.4 | Implement BundleSelectionStep | 3h |
| 5.8.5 | Implement ProfileSelectionStep | 3h |
| 5.8.6 | Implement CompletionStep | 2h |
| 5.8.7 | Implement RecoveryWizard | 4h |

**Dependencies**: 5.2, 3.2, 3.3, 3.4
**Deliverable**: Guided setup wizards

#### 5.9 Settings View
| Task | Description | Estimated |
|------|-------------|-----------|
| 5.9.1 | Implement HostSettingsView | 2h |
| 5.9.2 | Implement SyncSettingsView | 2h |
| 5.9.3 | Implement PreferencesView | 2h |

**Dependencies**: 5.2
**Deliverable**: Settings management views

### Phase 5 Checkpoint ✅ COMPLETE

```
✓ TUI launches without errors
✓ Dashboard displays correct system state
✓ All views are accessible via navigation
✓ Setup wizard completes successfully (6-step wizard)
✓ Update preview shows correct risk score
✓ Keyboard navigation works throughout
✓ TUI handles terminal resize gracefully (22 tests)
```

---

## Phase 6: Integration & Flows (Week 12-13)

### Objective
Complete end-to-end workflows and integration testing.

### Tasks

#### 6.1 End-to-End Workflows
| Task | Description | Estimated |
|------|-------------|-----------|
| 6.1.1 | Implement first-time setup flow (CLI + TUI) | 4h |
| 6.1.2 | Implement bundle switch flow | 3h |
| 6.1.3 | Implement profile change flow | 2h |
| 6.1.4 | Implement safe update flow | 4h |
| 6.1.5 | Implement recovery flow | 4h |
| 6.1.6 | Implement multi-machine sync flow | 3h |

**Dependencies**: 4.x, 5.x
**Deliverable**: Complete user workflows

#### 6.2 Integration Tests
| Task | Description | Estimated |
|------|-------------|-----------|
| 6.2.1 | Create test fixtures (sample configs) | 4h |
| 6.2.2 | Write bundle switch integration tests | 4h |
| 6.2.3 | Write profile selection integration tests | 3h |
| 6.2.4 | Write module enable/disable integration tests | 3h |
| 6.2.5 | Write update flow integration tests (mocked) | 4h |
| 6.2.6 | Write sync flow integration tests (mocked) | 3h |

**Dependencies**: 6.1
**Deliverable**: Integration test suite

#### 6.3 E2E Tests
| Task | Description | Estimated |
|------|-------------|-----------|
| 6.3.1 | Set up E2E test framework | 3h |
| 6.3.2 | Write E2E tests for CLI commands | 4h |
| 6.3.3 | Write E2E tests for TUI flows | 4h |
| 6.3.4 | Create CI pipeline for E2E tests | 2h |

**Dependencies**: 6.2
**Deliverable**: E2E test suite

#### 6.4 Performance Optimization
| Task | Description | Estimated |
|------|-------------|-----------|
| 6.4.1 | Profile TUI render performance | 2h |
| 6.4.2 | Optimize state loading | 2h |
| 6.4.3 | Implement config caching | 2h |
| 6.4.4 | Verify < 100ms TUI response time | 2h |

**Dependencies**: 6.1
**Deliverable**: Performance-optimized application

### Phase 6 Checkpoint ✅ COMPLETE

```
✓ All user stories from requirements pass
✓ Integration test coverage ≥ 70%
✓ E2E tests pass on CI
✓ TUI response time < 100ms (<1ms actual)
✓ No critical bugs in issue tracker
```

---

## Phase 7: Polish & Release (Week 14-16)

### Objective
Documentation, packaging, and release preparation.

### Tasks

#### 7.1 Documentation
| Task | Description | Estimated |
|------|-------------|-----------|
| 7.1.1 | Write README.md with quick start | 3h |
| 7.1.2 | Write INSTALL.md with detailed setup | 3h |
| 7.1.3 | Write USER-GUIDE.md | 6h |
| 7.1.4 | Generate CLI help documentation | 2h |
| 7.1.5 | Create example configs for bundles | 4h |
| 7.1.6 | Create example configs for profiles | 3h |
| 7.1.7 | Create example configs for modules | 3h |
| 7.1.8 | Write CONTRIBUTING.md | 2h |

**Dependencies**: 6.x
**Deliverable**: Complete documentation

#### 7.2 Packaging
| Task | Description | Estimated |
|------|-------------|-----------|
| 7.2.1 | Create PKGBUILD for Arch Linux | 3h |
| 7.2.2 | Create AUR package | 2h |
| 7.2.3 | Set up release binaries with cross | 4h |
| 7.2.4 | Create installation script | 2h |
| 7.2.5 | Test installation on fresh Arch | 3h |

**Dependencies**: 6.x
**Deliverable**: Distributable packages

#### 7.3 Final Testing
| Task | Description | Estimated |
|------|-------------|-----------|
| 7.3.1 | Full test suite run | 2h |
| 7.3.2 | Manual testing on real system | 4h |
| 7.3.3 | Bug fixes from testing | 8h |
| 7.3.4 | Security review | 4h |
| 7.3.5 | Performance validation | 2h |

**Dependencies**: 7.1, 7.2
**Deliverable**: Release-ready application

#### 7.4 Release
| Task | Description | Estimated |
|------|-------------|-----------|
| 7.4.1 | Update version numbers | 1h |
| 7.4.2 | Write CHANGELOG.md | 2h |
| 7.4.3 | Create GitHub release | 1h |
| 7.4.4 | Submit to AUR | 1h |
| 7.4.5 | Announce release | 1h |

**Dependencies**: 7.3
**Deliverable**: v1.0.0 release

### Phase 7 Checkpoint ✅ COMPLETE

```
✓ Documentation is complete and accurate
✓ PKGBUILD installs correctly
✓ All tests pass (165 tests → 1072 tests)
✓ Security review complete
✓ v0.1.0 release prepared
□ AUR package published (manual step)
```

---

## Phase 8: Production Hardening (Week 17-20) 🚧 IN PROGRESS

### Objective
Implement resilience patterns, health diagnostics, and achieve 80% test coverage for production readiness.

### Tasks

#### 8.1 Circuit Breaker Pattern (FR-5.9, NFR-8)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.1.1 | Create `CommandCircuitBreaker` struct in iron-core | 3h | P0 |
| 8.1.2 | Implement state machine (Closed/Open/HalfOpen) | 2h | P0 |
| 8.1.3 | Add 120s timeout for external commands | 2h | P0 |
| 8.1.4 | Integrate circuit breaker with iron-pacman | 3h | P0 |
| 8.1.5 | Integrate circuit breaker with iron-git | 2h | P0 |
| 8.1.6 | Integrate circuit breaker with iron-systemd | 2h | P0 |
| 8.1.7 | Write circuit breaker unit tests | 3h | P0 |
| 8.1.8 | Write resilience integration tests | 2h | P1 |

**Dependencies**: Phase 7
**Deliverable**: Fault-tolerant external command execution
**Requirement Coverage**: FR-5.9, NFR-8

**Implementation Reference** (from ARCHITECTURE.md v1.1.0):
```rust
pub struct CommandCircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    timeout: Duration,           // 120s default
    reset_timeout: Duration,     // Time in Open before HalfOpen
}

#[derive(Debug, Clone, Copy)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing fast, not executing commands
    HalfOpen,  // Testing if service recovered
}
```

#### 8.2 Partial Update Recovery (FR-5.10)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.2.1 | Add `UpdateProgress` struct to track package-level progress | 2h | P0 |
| 8.2.2 | Implement checkpoint file persistence during updates | 2h | P0 |
| 8.2.3 | Implement resume logic in UpdateService | 3h | P0 |
| 8.2.4 | Add `--resume` flag to `iron update` CLI | 1h | P0 |
| 8.2.5 | Write partial update recovery tests | 3h | P1 |

**Dependencies**: 8.1
**Deliverable**: Resilient update workflow that survives interruption
**Requirement Coverage**: FR-5.10

#### 8.3 Enhanced Health Diagnostics (FR-10.1-10.8)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.3.1 | Implement state file validation (FR-10.1) | 2h | P0 |
| 8.3.2 | Implement symlink integrity check (FR-10.2) | 2h | P0 |
| 8.3.3 | Implement package installation check (FR-10.3) | 2h | P0 |
| 8.3.4 | Implement snapshot backend check (FR-10.4) | 2h | P0 |
| 8.3.5 | Implement config directory check (FR-10.5) | 1h | P0 |
| 8.3.6 | Implement git repository check (FR-10.6) | 1h | P1 |
| 8.3.7 | Implement secrets status check (FR-10.7) | 1h | P1 |
| 8.3.8 | Implement structured JSON health report (FR-10.8) | 2h | P0 |
| 8.3.9 | Update `iron doctor` CLI with new checks | 2h | P0 |
| 8.3.10 | Write health check unit tests | 3h | P1 |

**Dependencies**: Phase 7
**Deliverable**: Comprehensive `iron doctor` with pass/warn/fail status per check
**Requirement Coverage**: FR-10.1-10.8

**Expected Output Format**:
```json
{
  "checks": [
    {"name": "state_file", "status": "pass", "message": "state.json valid"},
    {"name": "symlinks", "status": "warn", "message": "2 broken symlinks found"},
    {"name": "packages", "status": "pass", "message": "127 packages verified"},
    {"name": "snapshot", "status": "pass", "message": "timeshift available"},
    {"name": "directories", "status": "pass", "message": "all directories exist"},
    {"name": "git", "status": "warn", "message": "uncommitted changes"},
    {"name": "secrets", "status": "pass", "message": "git-crypt unlocked"}
  ],
  "overall": "warn",
  "timestamp": "2026-02-13T14:30:00Z"
}
```

#### 8.4 Structured Logging (NFR-9, NFR-10)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.4.1 | Implement JSON structured logging format | 2h | P1 |
| 8.4.2 | Add log rotation (10MB/5 files) | 2h | P1 |
| 8.4.3 | Integrate with tracing crate | 2h | P1 |
| 8.4.4 | Add component-level logging | 2h | P1 |
| 8.4.5 | Write logging tests | 1h | P2 |

**Dependencies**: Phase 7
**Deliverable**: Production-grade logging infrastructure
**Requirement Coverage**: NFR-9, NFR-10

#### 8.5 Graceful Degradation (NFR-11)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.5.1 | Implement fallback for unavailable secrets backend | 2h | P1 |
| 8.5.2 | Implement fallback for unavailable git remote | 2h | P1 |
| 8.5.3 | Implement fallback for unavailable snapshot backend | 2h | P1 |
| 8.5.4 | Add graceful degradation status to `iron status` | 2h | P1 |
| 8.5.5 | Write degradation scenario tests | 3h | P1 |

**Dependencies**: 8.1
**Deliverable**: System remains usable when optional components fail
**Requirement Coverage**: NFR-11

**Graceful Degradation Matrix**:
| Service | Degraded Behavior |
|---------|-------------------|
| Secrets (git-crypt/age) | Warn user, skip secret operations |
| Sync (git remote) | Work offline, queue sync operations |
| Snapshots (timeshift/snapper) | Warn user, proceed with confirmation |
| AUR Helper (paru/yay) | Fall back to official repos only |

#### 8.6 Acceptance Test Suite (AT-1 through AT-6)
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.6.1 | Create `tests/acceptance/` directory structure | 1h | P0 |
| 8.6.2 | Implement AT-1: First-Time Setup (6 scenarios) | 4h | P0 |
| 8.6.3 | Implement AT-2: Bundle Management (5 scenarios) | 3h | P0 |
| 8.6.4 | Implement AT-3: Profile Management (4 scenarios) | 3h | P0 |
| 8.6.5 | Implement AT-4: Module Operations (4 scenarios) | 3h | P1 |
| 8.6.6 | Implement AT-5: Update Workflow (4 scenarios) | 3h | P0 |
| 8.6.7 | Implement AT-5.5: E2E Bundle Switch (BLOCKING) | 4h | P0 |
| 8.6.8 | Implement AT-6: Recovery Workflow (3 scenarios) | 3h | P1 |

**Dependencies**: 8.1-8.5
**Deliverable**: Gherkin-style acceptance test coverage
**Requirement Coverage**: US-1 through US-6

#### 8.7 Coverage Push to 80% Target
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 8.7.1 | Add doctests for public APIs (+20 tests) | 2h | P1 |
| 8.7.2 | CLI output format validation (+29 tests) ✅ | - | DONE |
| 8.7.7 | Snapshot tool mocks (Phase 1.5) ✅ | - | DONE |
| 8.7.8 | Integrate snapshot mocks into tests (Phase 1.6) ✅ | - | DONE |
| 8.7.3 | Coverage gap hunting (+30 tests) | 6h | P1 |
| 8.7.4 | Integration test expansion (+15 tests) | 3h | P2 |
| 8.7.5 | Error path testing (+15 tests) | 3h | P1 |
| 8.7.6 | Edge case testing (+10 tests) | 2h | P2 |

**Dependencies**: 8.1-8.6
**Deliverable**: 80% test coverage (currently 56.75%)
**Current Progress**: 1072 tests, 56.75% → target 1172+ tests, 80%

### Phase 8 Checkpoint

```
✓ Circuit breaker pattern implemented and tested (8.1) ✅
✓ Partial update recovery working (8.2) ✅
✓ `iron doctor` returns structured JSON with all FR-10.x checks (8.3) ✅
□ Structured JSON logging with rotation (8.4)
□ Graceful degradation for optional services (8.5)
□ Acceptance tests AT-1 through AT-6 passing (8.6)
□ E2E bundle switch verified on real system (BLOCKING)
□ Test coverage ≥ 80% (currently 56.75%) (8.7)
✓ 1337 tests passing
```

---

## Phase 9: v1.0.0 Release (Week 21-22)

### Objective
Final validation, documentation polish, and production release.

### Tasks

#### 9.1 Final Validation
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 9.1.1 | Full test suite regression | 2h | P0 |
| 9.1.2 | Manual E2E testing on fresh Arch install | 4h | P0 |
| 9.1.3 | Security audit with cargo-audit | 2h | P0 |
| 9.1.4 | Performance profiling (< 100ms TUI response) | 2h | P1 |
| 9.1.5 | Memory leak detection | 2h | P1 |

#### 9.2 Documentation Final Pass
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 9.2.1 | Update USER-GUIDE with new features | 3h | P0 |
| 9.2.2 | Update API documentation | 2h | P0 |
| 9.2.3 | Create troubleshooting guide | 2h | P1 |
| 9.2.4 | Update CHANGELOG for v1.0.0 | 1h | P0 |

#### 9.3 Release
| Task | Description | Estimated | Priority |
|------|-------------|-----------|----------|
| 9.3.1 | Tag v1.0.0 release | 1h | P0 |
| 9.3.2 | Build release binaries | 2h | P0 |
| 9.3.3 | Update AUR PKGBUILD | 1h | P0 |
| 9.3.4 | Publish to AUR | 1h | P0 |
| 9.3.5 | Create GitHub release with notes | 1h | P0 |

### Phase 9 Checkpoint

```
□ All Phase 8 checkpoints complete
□ Zero critical/high severity issues
□ Documentation reviewed and accurate
□ v1.0.0 tagged and released
□ AUR package published
```

---

## Dependency Graph

```
Phase 1: Foundation ✅
    ├── 1.1 Workspace Setup
    │   └── 1.2 Domain Models
    │       └── 1.3 Error System
    │           └── 1.4 Validation Layer
    │
Phase 2: Infrastructure ✅
    ├── 2.1 Filesystem (depends: 1.2)
    ├── 2.2 Package Mgmt (depends: 1.3)
    ├── 2.3 Git (depends: 1.3)
    ├── 2.4 Systemd (depends: 1.3)
    └── 2.5 Snapshots (depends: 1.3)
    │
Phase 3: Core Services ✅
    ├── 3.1 State Management (depends: 2.1)
    │   ├── 3.2 Host Service (depends: 3.1, 2.1)
    │   ├── 3.3 Bundle Service (depends: 3.1, 2.1, 2.2)
    │   ├── 3.5 Module Service (depends: 3.1, 2.1)
    │   │   └── 3.4 Profile Service (depends: 3.1, 3.5)
    │   ├── 3.6 Update Service (depends: 3.1, 2.2, 2.5)
    │   ├── 3.7 Sync Service (depends: 3.1, 2.3)
    │   └── 3.8 Secrets Service (depends: 3.1, 2.3)
    │       └── 3.9 Recovery Service (depends: 3.2, 3.3, 3.4)
    │
Phase 4: CLI ✅
    └── 4.1-4.8 All commands (depends: Phase 3)
    │
Phase 5: TUI ✅
    └── 5.1-5.9 All views (depends: Phase 4)
    │
Phase 6: Integration ✅
    └── 6.1-6.4 Workflows & tests (depends: Phase 5)
    │
Phase 7: Release ✅
    └── 7.1-7.4 Docs & packaging (depends: Phase 6)
    │
Phase 8: Production Hardening 🚧 (Current)
    ├── 8.1 Circuit Breaker (depends: Phase 7)
    │   └── 8.2 Partial Update Recovery (depends: 8.1)
    ├── 8.3 Health Diagnostics (depends: Phase 7)
    ├── 8.4 Structured Logging (depends: Phase 7)
    ├── 8.5 Graceful Degradation (depends: 8.1)
    ├── 8.6 Acceptance Tests (depends: 8.1-8.5)
    └── 8.7 Coverage Target (depends: 8.1-8.6)
    │
Phase 9: v1.0.0 Release
    └── 9.1-9.3 Final validation & release (depends: Phase 8)
```

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Pacman wrapper complexity | Medium | High | Start early, mock heavily |
| TUI performance issues | Low | Medium | Profile early, optimize layout |
| Snapshot integration varies | Medium | Medium | Support multiple backends |
| Git-crypt edge cases | Medium | Low | Comprehensive testing |
| Cross-machine sync conflicts | Medium | Medium | Clear conflict UX |

---

## Quality Gates

### Definition of Done (per task)

- [ ] Code compiles without warnings
- [ ] Unit tests pass
- [ ] Code reviewed (if team)
- [ ] Documentation updated
- [ ] No clippy warnings

### Phase Completion Criteria

- [ ] All tasks in phase complete
- [ ] Checkpoint items verified
- [ ] Integration tests pass
- [ ] No critical bugs
- [ ] Technical debt documented

---

## Resource Requirements

### Development Environment

- Arch Linux (primary development)
- Rust 1.75+ toolchain
- Git with git-crypt
- timeshift or snapper installed
- Test machine with multiple DEs

### CI/CD

- GitHub Actions (Linux runners)
- Code coverage with tarpaulin
- Release builds with cross-rs

---

## Next Steps

**Immediate Actions (Phase 8.1 - Circuit Breaker):**

1. **Implement Circuit Breaker**: Run `/sc:implement "Circuit breaker pattern for external commands"`
   - Create `CommandCircuitBreaker` struct in `iron-core/src/resilience.rs`
   - Implement state machine with 120s timeout
   - Integrate with iron-pacman, iron-git, iron-systemd

2. **Implement Health Diagnostics**: Run `/sc:implement "Enhanced iron doctor health checks"`
   - Add FR-10.1-10.8 checks
   - Return structured JSON with pass/warn/fail status

3. **Acceptance Tests**: Run `/sc:test --acceptance`
   - Create `tests/acceptance/` directory
   - Implement AT-1 through AT-6 scenarios

4. **Coverage Push**: Continue from NEXT-ACTIONS.md
   - Doctests for public APIs
   - Coverage gap hunting
   - Target: 80% (currently 56.75%)

**Execution Order (Priority P0 first):**
```
8.1.1-8.1.7 → 8.2.1-8.2.4 → 8.3.1-8.3.9 → 8.6.1-8.6.7 → 8.7.1-8.7.6
```

**Track Progress**: Use TodoWrite for multi-step task tracking

---

**Document History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-02-12 | Workflow Session | Initial implementation plan |
| 1.1.0 | 2025-02-12 | Documentation Update | Updated Phase 1-6 as complete (165 tests) |
| 1.2.0 | 2025-02-12 | Documentation Update | Updated Phase 7 as complete |
| 2.0.0 | 2026-02-13 | /sc:workflow | Added Phase 8 (Production Hardening) and Phase 9 (v1.0.0 Release) based on expert panel recommendations. Added circuit breaker, health diagnostics, graceful degradation, and acceptance tests. Current: 1072 tests, 56.75% coverage |
