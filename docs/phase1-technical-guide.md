# Phase 1 — Technical Implementation Guide

> **Phase:** 1 — Core Experience
> **Sprints:** 1.1 (Host as Truth) + 1.2 (Apply Command) + 1.3 (Diff & Drift) + 1.4 (Template Engine — Stretch)
> **Audience:** Implementing engineers
> **Prerequisites:** Read `docs/product-review-and-roadmap.md`, `docs/phase1-kanban.md`, `CLAUDE.md`
>
> This document provides exact file locations, struct definitions, function signatures, and implementation patterns for every Phase 1 task. Each section includes the current codebase state, the target change, and testing guidance.

---

## Table of Contents

1. [Architecture Context](#1-architecture-context)
2. [Sprint 1.1: F1-001 — Extend Host Struct](#2-f1-001)
3. [Sprint 1.1: F1-002 — Backward-Compatible Parsing](#3-f1-002)
4. [Sprint 1.1: F1-003 — DesiredState Resolver](#4-f1-003)
5. [Sprint 1.1: F1-004 — TUI Wizard Host Write](#5-f1-004)
6. [Sprint 1.2: F1-005 — ApplyService](#6-f1-005)
7. [Sprint 1.2: F1-006 — ApplyPlan Struct](#7-f1-006)
8. [Sprint 1.2: F1-007 — CLI Apply Command](#8-f1-007)
9. [Sprint 1.2: F1-008 — Apply Dry-Run](#9-f1-008)
10. [Sprint 1.2: F1-009 — Selective Module Apply](#10-f1-009)
11. [Sprint 1.2: F1-010 — TUI Apply View](#11-f1-010)
12. [Sprint 1.3: F1-011 — DriftService](#12-f1-011)
13. [Sprint 1.3: F1-012 — Package Drift](#13-f1-012)
14. [Sprint 1.3: F1-013 — Service Drift](#14-f1-013)
15. [Sprint 1.3: F1-014 — Config Drift](#15-f1-014)
16. [Sprint 1.3: F1-015 — CLI Diff Command](#16-f1-015)
17. [Sprint 1.3: F1-016 — Diff Adopt](#17-f1-016)
18. [Sprint 1.3: F1-017 — Diff Correct](#18-f1-017)
19. [Sprint 1.3: F1-018 — TUI Drift Indicator](#19-f1-018)
20. [Sprint 1.4: F1-019 — Template Engine](#20-f1-019)
21. [Sprint 1.4: F1-020 — Template Rendering in Apply](#21-f1-020)
22. [Sprint 1.4: F1-021 — Built-in Variables](#22-f1-021)
23. [Sprint 1.4: F1-022 — TUI Variable Editor](#23-f1-022)
24. [Testing Strategy](#24-testing-strategy)
25. [Product Requirement Cross-Reference](#25-product-cross-reference)

---

## 1. Architecture Context

### New Files Introduced in Phase 1

```
iron-core/src/services/
  ├── apply.rs          ← NEW (F1-005, F1-006, F1-009)
  ├── drift.rs          ← NEW (F1-011, F1-012, F1-013, F1-014)
  └── mod.rs            ← MODIFIED (register new services)

iron-core/src/
  ├── host.rs           ← MODIFIED (F1-001: new fields)
  └── system_service.rs ← MODIFIED (F1-013: is_enabled)

iron-cli/src/
  ├── cli.rs            ← MODIFIED (F1-007, F1-015: new commands)
  ├── main.rs           ← MODIFIED (F1-007, F1-015: wire commands)
  ├── context.rs        ← MODIFIED (F1-007, F1-015: service factories)
  └── commands/
      ├── apply.rs      ← NEW (F1-007, F1-008, F1-009)
      └── diff.rs       ← NEW (F1-015, F1-016, F1-017)

iron-tui/src/
  ├── app/
  │   ├── mod.rs        ← MODIFIED (F1-010, F1-018: new state fields)
  │   ├── actions.rs    ← MODIFIED (F1-010, F1-018: apply/drift actions)
  │   └── handlers.rs   ← MODIFIED (F1-010, F1-018: keybindings)
  └── ui/
      ├── apply.rs      ← NEW (F1-010)
      ├── variables.rs  ← NEW (F1-022)
      ├── dashboard.rs  ← MODIFIED (F1-018: drift indicator)
      └── mod.rs        ← MODIFIED (register new views)

iron-fs/src/
  └── lib.rs            ← MODIFIED (F1-019: template module, F1-014: checksum)

iron-systemd/src/
  └── lib.rs            ← MODIFIED (F1-013: is_enabled)
```

### Key Design Principle: Desired vs Actual

Phase 1 introduces a fundamental architectural concept:

```
┌──────────────────────┐         ┌──────────────────────┐
│   DESIRED STATE      │         │   ACTUAL STATE       │
│   (host.toml)        │         │   (system)           │
│                      │   diff  │                      │
│  bundle: hyprland    │ ──────▶ │  pacman -Qqe         │
│  profile: developer  │         │  systemctl --enabled  │
│  extra_modules: []   │         │  readlink ~/.config/* │
│  [variables]         │         │  stat / checksum      │
│    terminal = kitty  │         │                      │
└──────────┬───────────┘         └──────────┬───────────┘
           │                                │
           │         ┌───────────┐          │
           └────────▶│ ApplyPlan │◀─────────┘
                     │ (actions) │
                     └─────┬─────┘
                           │
                    ┌──────▼──────┐
                    │   execute   │
                    │ converge    │
                    └─────────────┘
```

- **DesiredState** = resolved from host.toml → bundle → profile → modules → packages/dotfiles/services
- **ActualState** = queried from system (pacman, systemctl, readlink, checksums)
- **ApplyPlan** = diff between desired and actual = list of actions to converge
- **DriftReport** = same diff but presented as a report (no execution)

---

## 2. F1-001: Extend Host Struct with Desired-State Fields

### Product Requirement
- Newcomer #5: "Declarative system definition"
- Mid-level #1: "Layered, composable configuration"
- Roadmap §7.3: "Host file as single source of truth"

### Current State

**File:** `crates/iron-core/src/host.rs` (lines 7-28)

```rust
pub struct Host {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub hardware: HardwareSpec,
    pub install_params: Option<InstallParams>,
    pub installed_bundles: Vec<String>,
    pub active_bundle: Option<String>,
}
```

**Current `hosts/desktop.toml`:**
```toml
id = "desktop"
name = "Desktop Workstation"
installed_bundles = []

[hardware]
cpu = "AMD Ryzen 7 9800X3D 8-Core Processor"
gpu = "Advanced Micro Devices, Inc. [AMD/ATI] Navi 44 [Radeon RX 9060 XT] (rev c0)"
ram_mb = 31191
chassis = "Desktop"
```

### Implementation

Add four new fields to `Host` struct:

```rust
pub struct Host {
    // ...existing fields...

    /// F1-001: Declared bundle for this host (desired state)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,

    /// F1-001: Declared profile for this host (desired state)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// F1-001: Extra modules beyond what the profile includes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_modules: Vec<String>,

    /// F1-001: Template variables for this host
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,
}
```

**Target `hosts/desktop.toml` after user configures:**
```toml
id = "desktop"
name = "Desktop Workstation"
installed_bundles = []
bundle = "hyprland"
profile = "developer"
extra_modules = ["gaming"]

[variables]
primary_monitor = "DP-1"
secondary_monitor = "DP-2"
terminal = "kitty"
browser = "firefox"

[hardware]
# ...unchanged...
```

### Test Updates Required (Lesson L1)

**File:** `crates/iron-core/src/host.rs` — `create_test_host()` (line ~150)

Add `bundle: None, profile: None, extra_modules: vec![], variables: HashMap::new()` to the constructor. Also update `test_host_minimal()`.

**File:** `crates/iron-core/src/services/host.rs` — any test that constructs `Host` directly.

**New test:**
```rust
#[test]
fn test_host_roundtrip_with_desired_state() {
    let host = Host {
        // ...
        bundle: Some("hyprland".to_string()),
        profile: Some("developer".to_string()),
        extra_modules: vec!["gaming".to_string()],
        variables: HashMap::from([
            ("terminal".to_string(), "kitty".to_string()),
        ]),
    };
    let serialized = toml::to_string_pretty(&host).unwrap();
    let deserialized: Host = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.bundle, host.bundle);
    assert_eq!(deserialized.variables.get("terminal"), Some(&"kitty".to_string()));
}

#[test]
fn test_host_backward_compat_no_new_fields() {
    let toml = r#"
        id = "legacy"
        name = "Legacy Host"
        installed_bundles = []
        [hardware]
    "#;
    let host: Host = toml::from_str(toml).unwrap();
    assert!(host.bundle.is_none());
    assert!(host.profile.is_none());
    assert!(host.extra_modules.is_empty());
    assert!(host.variables.is_empty());
}
```

---

## 3. F1-002: Backward-Compatible TOML Parsing & Migration

### Product Requirement
- Newcomer #8: "Idempotent operations"

### Current State

Active bundle/profile are tracked in `state.json` via `IronState`:

**File:** `crates/iron-core/src/state.rs` (lines ~166-170)
```rust
pub struct IronState {
    pub current_host: Option<String>,
    pub active_bundles: HashMap<String, String>,   // host_id → bundle_id
    pub active_profiles: HashMap<String, String>,  // host_id → profile_id
    pub active_modules: Vec<String>,
    // ...
}
```

### Implementation

**Add to `HostService` trait** (`crates/iron-core/src/services/host.rs`):

```rust
/// F1-002: Migrate active bundle/profile from state.json into host.toml
fn migrate_state_to_host(&self, host_id: &str, state: &StateManager) -> IronResult<()>;
```

**Implementation logic:**
1. Load host from TOML
2. If `host.bundle.is_none()`, read `state.active_bundles[host_id]` and set it
3. If `host.profile.is_none()`, read `state.active_profiles[host_id]` and set it
4. Save host TOML (preserving all other fields)
5. Idempotent: if fields already set, skip

**Doctor integration:** Add a new health check in `doctor.rs`:
```rust
fn check_host_desired_state(&self) -> HealthCheck {
    // Warn if current host has no bundle/profile declared in host.toml
}
```

### Testing

- Test migration from state.json → host.toml
- Test idempotency (second migration is no-op)
- Test preserves hardware fields
- Test with host that already has bundle/profile (no overwrite)

---

## 4. F1-003: DesiredState Resolver

### Product Requirement
- Mid-level #1: "Layered, composable configuration"
- Core to ApplyService (F1-005) and DriftService (F1-011)

### Current State

No concept of "desired state" exists. `HostService` only reads host hardware + bundles. Module/profile resolution is scattered across TUI wizard code.

### Implementation

**New struct** in `crates/iron-core/src/services/host.rs`:

```rust
/// The fully resolved desired state for a host.
/// Computed from: host.toml → bundle → profile → modules → packages/dotfiles/services.
#[derive(Debug, Clone, Default)]
pub struct DesiredState {
    /// Declared bundle
    pub bundle: Option<String>,
    /// Declared profile
    pub profile: Option<String>,
    /// All resolved module IDs (from profile + extra_modules, deduplicated)
    pub modules: Vec<String>,
    /// All packages to install (from bundle + all modules)
    pub packages: Vec<String>,
    /// All AUR packages to install
    pub aur_packages: Vec<String>,
    /// All dotfile mappings (from all modules)
    pub dotfiles: Vec<DotfileMapping>,
    /// All systemd services to enable (from bundle + modules)
    pub services: Vec<String>,
    /// Template variables (host [variables] merged with built-ins)
    pub variables: HashMap<String, String>,
}
```

**Add to `HostService` trait:**

```rust
/// F1-003: Resolve the fully desired state for a host
fn desired_state(&self, host_id: &str) -> IronResult<DesiredState>;
```

**Resolution algorithm:**

```
1. Load Host from hosts/<id>.toml
2. If host.bundle → Load Bundle → collect bundle.packages, bundle.services
3. If host.profile → Load Profile → resolve profile.modules (+ extends chain)
4. Append host.extra_modules to module list
5. Deduplicate modules, resolve dependencies (Module.depends)
6. Check conflicts (Module.conflicts) → error if conflict found
7. For each module:
   - Collect module.packages, module.aur_packages
   - Collect module.dotfiles
   - If module has services field → collect
8. Merge host.variables with built-in variables
9. Return DesiredState
```

**Dependencies needed:** `BundleService::load()`, `ProfileService` (for profile loading and inheritance resolution), `ModuleService::load()`.

The `DefaultHostService` implementation needs access to these services. Options:
- **Option A:** Pass services as parameters to `desired_state()`
- **Option B:** Extend `DefaultHostService` with service references (builder pattern)

**Recommendation:** Option A — keep `desired_state()` pure by accepting the root path and resolving inline. The `DesiredState` resolver can be a standalone function:

```rust
/// Resolve desired state by loading all referenced configs from disk
pub fn resolve_desired_state(
    iron_root: &Path,
    host: &Host,
) -> IronResult<DesiredState> {
    // ...implementation...
}
```

This keeps it testable without mocking services — just create temp directories with TOML files.

### Testing

Create tempdir with:
- `hosts/test.toml` (with bundle + profile + extra_modules + variables)
- `bundles/test-bundle/bundle.toml`
- `profiles/test-profile/profile.toml` (with modules list)
- `modules/mod-a/module.toml`, `modules/mod-b/module.toml`

Verify resolved DesiredState has correct packages, dotfiles, services from all sources.

Test edge cases:
- No bundle declared
- Profile with `extends` inheritance
- Module dependency chain (A depends on B depends on C)
- Module conflict detection

---

## 5. F1-004: TUI Wizard Writes to Host TOML

### Current State

**File:** `crates/iron-tui/src/app/actions.rs` — wizard completion writes to state.json:
- `self.state_manager.set_active_bundle(host_id, bundle_id)`
- `self.state_manager.set_active_profile(host_id, profile_id)`

### Implementation

After the existing state.json writes, add host.toml write:

```rust
// After state.json updates, also persist to host.toml (F1-004)
if let Some(ref host_svc) = self.host_service {
    if let Ok(mut host) = host_svc.load_host(host_id) {
        host.bundle = Some(bundle_id.to_string());
        host.profile = Some(profile_id.to_string());
        if let Err(e) = host_svc.save_host(&host) {
            self.set_warning(format!("Host TOML write failed: {}", e));
        }
    }
}
```

The TUI `App` already holds a reference to host data. We need to ensure `HostService` (or equivalent) is available.

---

## 6. F1-005: ApplyService — Compare Desired vs Actual

### Product Requirement
- Newcomer #5: "Declarative system definition"
- Roadmap §7.2: "Unify the Apply Workflow"

### Implementation

**New file:** `crates/iron-core/src/services/apply.rs`

```rust
use crate::packages::PackageManager;
use crate::services::host::DesiredState;
use crate::services::state::StateManager;
use crate::system_service::SystemService;
use crate::{DotfileMapping, IronResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Service for converging system to desired state
pub trait ApplyService {
    /// Compute an apply plan (what needs to change)
    fn plan(&self, host_id: &str) -> IronResult<ApplyPlan>;

    /// Compute plan for a single module only
    fn plan_module(&self, module_id: &str) -> IronResult<ApplyPlan>;

    /// Execute an apply plan
    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult>;
}

pub struct DefaultApplyService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Arc<dyn PackageManager>,
    service_manager: Arc<dyn SystemService>,
}
```

**Plan computation** (in `plan()` method):

```rust
fn plan(&self, host_id: &str) -> IronResult<ApplyPlan> {
    // 1. Resolve desired state
    let desired = resolve_desired_state(&self.iron_root, &host)?;

    // 2. Query actual state
    let installed_packages: HashSet<String> = self.package_manager
        .query_installed()?
        .into_iter()
        .map(|p| p.name)
        .collect();

    // 3. Compute diffs
    let mut actions = Vec::new();

    // Packages to install
    let missing_packages: Vec<String> = desired.packages.iter()
        .filter(|p| !installed_packages.contains(*p))
        .cloned()
        .collect();
    if !missing_packages.is_empty() {
        actions.push(ApplyAction::InstallPackages { packages: missing_packages });
    }

    // Symlinks to create
    for dotfile in &desired.dotfiles {
        let target = crate::validation::expand_home(Path::new(&dotfile.target));
        if !target.exists() || !target.is_symlink() {
            actions.push(ApplyAction::CreateSymlink {
                source: dotfile.source.clone(),
                target: dotfile.target.clone(),
            });
        }
    }

    // Services to enable
    for service in &desired.services {
        if !self.service_manager.is_enabled(service)? {
            actions.push(ApplyAction::EnableService { name: service.clone() });
        }
    }

    // Modules to activate in state
    let active_modules: HashSet<String> = self.state_manager
        .active_modules()
        .into_iter()
        .collect();
    for module_id in &desired.modules {
        if !active_modules.contains(module_id) {
            actions.push(ApplyAction::EnableModule { id: module_id.clone() });
        }
    }

    Ok(ApplyPlan { actions })
}
```

### Register in services/mod.rs

Add to `crates/iron-core/src/services/mod.rs`:
```rust
pub mod apply;
pub use apply::{ApplyAction, ApplyPlan, ApplyResult, ApplyService, DefaultApplyService};
```

---

## 7. F1-006: ApplyPlan Struct

### Implementation

In `crates/iron-core/src/services/apply.rs`:

```rust
/// A plan of actions to converge the system
#[derive(Debug, Clone, Serialize)]
pub struct ApplyPlan {
    pub actions: Vec<ApplyAction>,
}

/// Individual action in an apply plan
#[derive(Debug, Clone, Serialize)]
pub enum ApplyAction {
    InstallPackages { packages: Vec<String> },
    CreateSymlink { source: String, target: String },
    EnableService { name: String },
    EnableModule { id: String },
    ActivateBundle { id: String },
    ActivateProfile { id: String },
}

impl ApplyPlan {
    pub fn is_empty(&self) -> bool { self.actions.is_empty() }

    pub fn action_count(&self) -> usize { self.actions.len() }

    pub fn summary(&self) -> String {
        let pkgs = self.actions.iter().filter_map(|a| match a {
            ApplyAction::InstallPackages { packages } => Some(packages.len()),
            _ => None,
        }).sum::<usize>();
        let links = self.actions.iter().filter(|a| matches!(a, ApplyAction::CreateSymlink { .. })).count();
        let svcs = self.actions.iter().filter(|a| matches!(a, ApplyAction::EnableService { .. })).count();
        let mods = self.actions.iter().filter(|a| matches!(a, ApplyAction::EnableModule { .. })).count();
        format!("{} packages, {} symlinks, {} services, {} modules", pkgs, links, svcs, mods)
    }
}

/// Result of executing an apply plan
#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub duration: Duration,
}
```

---

## 8. F1-007: CLI Apply Command

### Implementation

**File:** `crates/iron-cli/src/cli.rs` — add variant:

```rust
/// Apply declared system state
Apply {
    /// Preview changes without executing
    #[arg(long)]
    dry_run: bool,

    /// Apply a single module only
    #[arg(long)]
    module: Option<String>,

    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
},
```

**New file:** `crates/iron-cli/src/commands/apply.rs`

```rust
pub fn execute(ctx: &AppContext, dry_run: bool, module: Option<String>, yes: bool) -> Result<()> {
    require_init(ctx)?;
    let output = &ctx.output;
    let service = ctx.apply_service();

    output.header("Iron Apply");

    // Compute plan
    let plan = if let Some(ref mod_id) = module {
        output.info(&format!("Computing plan for module '{}'...", mod_id));
        service.plan_module(mod_id)?
    } else {
        output.info("Computing system apply plan...");
        service.plan(&ctx.current_host().unwrap_or_default())?
    };

    if plan.is_empty() {
        output.success("System is already in desired state — nothing to do.");
        return Ok(());
    }

    // Display plan
    output.subheader("Plan");
    for action in &plan.actions {
        output.info(&format!("  {}", action.display()));
    }
    output.info(&format!("\nSummary: {}", plan.summary()));

    if dry_run {
        output.success("[DRY RUN] No changes made.");
        return Ok(());
    }

    // Confirm
    if !yes {
        // ... confirmation prompt ...
    }

    // Execute
    let result = service.execute(&plan)?;
    output.summary(&[
        ("succeeded", result.succeeded),
        ("failed", result.failed),
    ]);

    Ok(())
}
```

**Wire in `main.rs`:**
```rust
Some(Commands::Apply { dry_run, module, yes }) =>
    commands::apply::execute(&ctx, dry_run, module.as_deref(), yes),
```

**Wire in `context.rs`:**
```rust
pub fn apply_service(&self) -> impl ApplyService {
    DefaultApplyService::new(
        &self.root,
        self.state.clone(),
        Arc::new(iron_pacman::DefaultPackageManager::new()),
        Arc::new(iron_systemd::SystemdServiceAdapter::user()),
    )
}
```

### Integration Test (Lesson L2, L3)

```rust
#[test]
fn apply_dry_run_succeeds() {
    let dir = create_initialized_iron_dir();
    iron()
        .arg("--root").arg(dir.path())
        .arg("apply").arg("--dry-run")
        .assert()
        .success();
}
```

---

## 9. F1-008: Apply Dry-Run

Handled by the `--dry-run` flag in F1-007. The `plan()` is always computed; `--dry-run` skips `execute()`.

---

## 10. F1-009: Selective Module Apply

### Implementation

In `DefaultApplyService`:

```rust
fn plan_module(&self, module_id: &str) -> IronResult<ApplyPlan> {
    let module_dir = self.iron_root.join("modules").join(module_id);
    let module = Module::load(&module_dir)
        .map_err(|e| StateError::InvalidState { message: format!("Module '{}' not found: {}", module_id, e) })?;

    // Resolve dependencies
    let mut all_modules = vec![module_id.to_string()];
    // ... resolve module.depends transitively ...

    // Build plan from just these modules' packages/dotfiles/services
    // ... (same diff logic as plan() but scoped to these modules)
}
```

---

## 11. F1-010: TUI Apply View

### Implementation

**New file:** `crates/iron-tui/src/ui/apply.rs`

Pattern follows existing views. Key elements:
- View enum: add `View::Apply`
- Keybinding: `[a]` from Dashboard
- State fields in `App`: `apply_plan: Option<ApplyPlan>`, `apply_in_progress: bool`, `apply_result_rx: Option<Receiver<...>>`
- Background execution using `std::thread::spawn` + `mpsc::channel` (F0-009 pattern from `sync_push`)
- Plan display: scrollable list of actions with type icons
- Progress: poll result on tick (same as `poll_sync_result`)

### Layout

```
┌─ Apply System State ─────────────────────────────┐
│                                                    │
│  Plan (3 packages, 5 symlinks, 1 service)         │
│                                                    │
│  📦 Install packages: neovim, ripgrep, fd          │
│  🔗 Link ~/.config/nvim → modules/nvim/config     │
│  🔗 Link ~/.config/kitty → modules/kitty/config   │
│  🔗 Link ~/.config/starship.toml → ...            │
│  🔗 Link ~/.config/fish → modules/fish/config     │
│  🔗 Link ~/.tmux.conf → modules/tmux/config       │
│  ⚙ Enable bluetooth.service                       │
│                                                    │
│  ─────────────────────────────────────             │
│  [Enter] Apply  [Esc] Cancel  [d] Dry-run         │
└────────────────────────────────────────────────────┘
```

---

## 12. F1-011: DriftService

### Product Requirement
- Mid-level #4: "Drift detection"

### Implementation

**New file:** `crates/iron-core/src/services/drift.rs`

```rust
pub trait DriftService {
    fn detect(&self, host_id: &str) -> IronResult<DriftReport>;
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DriftReport {
    pub package_drift: Vec<PackageDrift>,
    pub service_drift: Vec<ServiceDrift>,
    pub config_drift: Vec<ConfigDrift>,
    pub summary: DriftSummary,
}

#[derive(Debug, Clone, Serialize)]
pub enum PackageDrift {
    Missing { name: String },
    Extra { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum ServiceDrift {
    NotEnabled { name: String },
    ExtraEnabled { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum ConfigDrift {
    MissingSymlink { source: String, target: String },
    BrokenSymlink { target: String },
    WrongTarget { target: String, expected: String, actual: String },
    ContentModified { path: String, expected_checksum: String, actual_checksum: String },
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DriftSummary {
    pub total_drifts: usize,
    pub packages_missing: usize,
    pub packages_extra: usize,
    pub configs_drifted: usize,
    pub services_drifted: usize,
}

impl DriftReport {
    pub fn is_clean(&self) -> bool {
        self.package_drift.is_empty()
            && self.service_drift.is_empty()
            && self.config_drift.is_empty()
    }
}
```

`DefaultDriftService` follows same builder pattern as `ApplyService`.

---

## 13. F1-012: Package Drift Detection

### Implementation

```rust
fn detect_package_drift(&self, desired: &DesiredState) -> IronResult<Vec<PackageDrift>> {
    let installed: HashSet<String> = self.package_manager
        .query_installed()?
        .into_iter()
        .map(|p| p.name)
        .collect();

    let desired_pkgs: HashSet<String> = desired.packages.iter().cloned().collect();

    let mut drift = Vec::new();

    // Missing: desired but not installed
    for pkg in &desired_pkgs {
        if !installed.contains(pkg) {
            drift.push(PackageDrift::Missing { name: pkg.clone() });
        }
    }

    // Extra: installed by Iron but no longer desired
    // Only flag packages that Iron previously installed (tracked in state.json)
    let iron_installed = self.state_manager.iron_installed_packages();
    for pkg in &iron_installed {
        if !desired_pkgs.contains(pkg) && installed.contains(pkg) {
            drift.push(PackageDrift::Extra { name: pkg.clone() });
        }
    }

    Ok(drift)
}
```

**State tracking addition:** When `ApplyService::execute()` installs packages, record them in `state.json` as `iron_installed_packages: Vec<String>`. This avoids false positives for user-installed packages.

---

## 14. F1-013: Service Drift Detection

### SystemService Trait Extension

**File:** `crates/iron-core/src/system_service.rs`

```rust
pub trait SystemService: Send + Sync {
    // ...existing methods...

    /// F1-013: Check if a service is enabled
    fn is_enabled(&self, name: &str) -> IronResult<bool> {
        let _ = name;
        Ok(false) // default impl for NoopSystemService compat
    }
}
```

**File:** `crates/iron-systemd/src/lib.rs`

```rust
fn is_enabled(&self, name: &str) -> IronResult<bool> {
    let output = Command::new("systemctl")
        .args(["is-enabled", name])
        .output()
        .map_err(|e| /* ... */)?;
    Ok(output.status.success())
}
```

---

## 15. F1-014: Config Drift Detection (Symlink + Checksum)

### New Checksum Utility

**File:** `crates/iron-fs/src/lib.rs` — new `checksum` module:

```rust
pub mod checksum {
    use std::path::Path;
    use std::io::Read;
    use sha2::{Sha256, Digest};

    /// Compute SHA-256 checksum of a file
    pub fn sha256(path: &Path) -> std::io::Result<String> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 { break; }
            hasher.update(&buffer[..n]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
}
```

**Dependency:** Add `sha2 = "0.10"` to `iron-fs/Cargo.toml`.

### Checksum Storage

Add to `IronState` in `crates/iron-core/src/state.rs`:

```rust
/// F1-014: Checksums of managed dotfile sources (path → sha256)
#[serde(default, skip_serializing_if = "HashMap::is_empty")]
pub dotfile_checksums: HashMap<String, String>,
```

Checksums are stored when `ApplyService::execute()` creates symlinks, and verified during drift detection.

---

## 16. F1-015: CLI Diff Command

### Implementation

**File:** `crates/iron-cli/src/cli.rs`:
```rust
/// Show differences between declared and actual state
Diff {
    /// Incorporate discovered drift into canonical state
    #[arg(long)]
    adopt: bool,

    /// Revert system to match declared state
    #[arg(long)]
    correct: bool,

    /// Preview corrections without executing
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation
    #[arg(short, long)]
    yes: bool,
},
```

**New file:** `crates/iron-cli/src/commands/diff.rs`

Output format:
```
─── Iron Diff ───

📦 Packages
  ✗ missing: neovim (declared in nvim-ide)
  ✗ missing: ripgrep (declared in nvim-ide)
  ⚠ extra: vim (installed by Iron, no longer declared)

🔗 Configs
  ✗ missing: ~/.config/nvim → modules/nvim-ide/config/nvim
  ⚠ modified: ~/.config/kitty/kitty.conf (content changed)

⚙ Services
  ✗ not enabled: bluetooth.service

▸ Summary: 5 drifts (2 packages · 2 configs · 1 service)
```

---

## 17-18. F1-016/F1-017: Diff Adopt & Correct

### Adopt (`--adopt`)
- Extra packages → acknowledged in state.json `acknowledged_packages`
- Modified configs → checksum updated in state.json
- Confirmation prompt before writing

### Correct (`--correct`)
- Compute `ApplyPlan` from drift report
- Delegate to `ApplyService::execute()`
- Supports `--dry-run` for preview

---

## 19. F1-018: TUI Drift Indicator

### Implementation

Add to Dashboard System Status panel (`crates/iron-tui/src/ui/dashboard.rs`):

```rust
// After health status line
let drift_count = app.drift_count.unwrap_or(0);
let (drift_str, drift_color) = if drift_count == 0 {
    ("Drift: 0 ✓".to_string(), theme::GREEN)
} else {
    (format!("Drift: {} ⚠", drift_count), theme::YELLOW)
};
```

Add `drift_count: Option<usize>` and `drift_report: Option<DriftReport>` to `App`.

Background computation on startup using same `std::thread::spawn` + `mpsc::channel` pattern.

---

## 20. F1-019: Template Engine

### Product Requirement
- Mid-level #7: "Template/variable system"

### Implementation

**File:** `crates/iron-fs/src/lib.rs` — new `template` module:

```rust
pub mod template {
    use std::collections::HashMap;
    use regex::Regex;

    /// Render template content by substituting {{variable}} placeholders
    pub fn render(content: &str, vars: &HashMap<String, String>) -> String {
        let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
        re.replace_all(content, |caps: &regex::Captures| {
            let key = &caps[1];
            match vars.get(key) {
                Some(val) => val.clone(),
                None => {
                    tracing::warn!("Unknown template variable: {{{{{}}}}}", key);
                    caps[0].to_string() // leave unchanged
                }
            }
        }).to_string()
    }

    /// Check if content contains template variables
    pub fn has_variables(content: &str) -> bool {
        content.contains("{{")
    }

    /// Extract variable names from template content
    pub fn extract_variables(content: &str) -> Vec<String> {
        let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
        re.captures_iter(content)
            .map(|c| c[1].to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
}
```

**Dependency:** Add `regex = "1"` to `iron-fs/Cargo.toml`.

### Testing

```rust
#[test]
fn test_render_basic() {
    let vars = HashMap::from([("name".into(), "kitty".into())]);
    assert_eq!(template::render("terminal = {{name}}", &vars), "terminal = kitty");
}

#[test]
fn test_render_whitespace() {
    let vars = HashMap::from([("x".into(), "1".into())]);
    assert_eq!(template::render("{{ x }}", &vars), "1");
}

#[test]
fn test_render_unknown_preserved() {
    let vars = HashMap::new();
    assert_eq!(template::render("{{unknown}}", &vars), "{{unknown}}");
}

#[test]
fn test_extract_variables() {
    let vars = template::extract_variables("a={{x}} b={{y}} c={{x}}");
    assert!(vars.contains(&"x".to_string()));
    assert!(vars.contains(&"y".to_string()));
    assert_eq!(vars.len(), 2); // deduplicated
}
```

---

## 21. F1-020: Template Rendering in Apply

### Implementation

In `ApplyService::execute()`, when creating symlinks:

```rust
for action in &plan.actions {
    match action {
        ApplyAction::CreateSymlink { source, target } => {
            let source_path = module_dir.join(source);
            let target_path = expand_home(Path::new(target));

            if source.ends_with(".tmpl") {
                // Render template → write to rendered dir → symlink to rendered
                let content = fs::read_to_string(&source_path)?;
                let rendered = iron_fs::template::render(&content, &desired.variables);

                let rendered_dir = self.iron_root.join("rendered").join(module_id);
                fs::create_dir_all(&rendered_dir)?;

                // Strip .tmpl extension for output filename
                let filename = source_path.file_stem().unwrap();
                let rendered_path = rendered_dir.join(filename);
                fs::write(&rendered_path, &rendered)?;

                // Symlink to rendered file
                create_symlink(&rendered_path, &target_path)?;
            } else {
                // Direct symlink to source
                create_symlink(&source_path, &target_path)?;
            }
        }
        // ... other actions
    }
}
```

---

## 22. F1-021: Built-in Variables

### Implementation

```rust
pub fn builtin_variables(iron_root: &Path) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        vars.insert("hostname".to_string(), hostname.trim().to_string());
    }
    if let Ok(user) = std::env::var("USER") {
        vars.insert("username".to_string(), user);
    }
    if let Some(home) = dirs::home_dir() {
        vars.insert("home".to_string(), home.display().to_string());
    }
    if let Some(config) = dirs::config_dir() {
        vars.insert("config_dir".to_string(), config.display().to_string());
    }
    vars.insert("iron_root".to_string(), iron_root.display().to_string());

    vars
}
```

Merged in `resolve_desired_state()`:
```rust
let mut variables = builtin_variables(iron_root);
// Host variables override built-ins
variables.extend(host.variables.clone());
```

---

## 23. F1-022: TUI Variable Editor

New view for editing `[variables]` in host.toml. Follow ModuleCreator pattern:
- Key-value list with add/edit/delete
- Tab between key and value fields
- Enter to save, Esc to cancel
- Built-in variables shown as read-only

---

## 24. Testing Strategy

### Unit Tests (iron-core)

| Area | Approach | Count Target |
|------|----------|-------------|
| Host struct parsing | TOML roundtrip, backward compat, migration | ~10 |
| DesiredState resolver | Tempdir with TOML files, dependency chains | ~15 |
| ApplyService plan computation | Mock PackageManager/SystemService | ~15 |
| ApplyPlan display/summary | Struct construction tests | ~5 |
| DriftService detection | Mock services with known state | ~15 |
| Package/service/config drift | Each drift type with edge cases | ~10 |
| Template engine | Substitution, edge cases, extraction | ~10 |

### Integration Tests (iron-cli)

| Test | Key Requirement |
|------|----------------|
| `iron apply --dry-run` | Must not call sudo (L2) |
| `iron apply --module X --dry-run` | Selective apply |
| `iron diff` | Drift report output |
| `iron diff -f json` | JSON output |

### TUI Tests (iron-tui)

Existing pattern: construct `App` with mock data, verify view rendering doesn't panic.

---

## 25. Product Requirement Cross-Reference

| Task | Newcomer Constraint | Mid-Level Constraint | Roadmap Section |
|------|--------------------|--------------------|-----------------|
| F1-001 | #5 Declarative | #1 Composable | §7.3 |
| F1-002 | #8 Idempotent | — | §7.3 |
| F1-003 | #5 Declarative | #1 Composable | §7.2, §7.3 |
| F1-004 | #1 TUI primary | — | §7.3 |
| F1-005 | #5 Declarative | #5 Diff before apply | §7.2 |
| F1-006 | #2 Dry-run | — | §7.2 |
| F1-007 | #1 Confirm, #3 Clear output | — | §7.5, §14.1 |
| F1-008 | #2 Dry-run | #5 Diff before apply | §7.5 |
| F1-009 | — | #8 Selective | §7.5 |
| F1-010 | #1 TUI primary | — | §7.2 |
| F1-011 | — | #4 Drift detection | §7.5 |
| F1-012 | — | #4 Drift detection | §7.5 |
| F1-013 | — | #4 Drift detection | §7.5 |
| F1-014 | — | #4 Drift detection | §7.5 |
| F1-015 | — | #5 Diff before apply | §7.5 |
| F1-016 | — | #4 Drift detection | §7.5 |
| F1-017 | — | #4 Drift detection | §7.5 |
| F1-018 | #1 TUI primary | #4 Drift detection | §7.5 |
| F1-019 | — | #7 Templates | §7.4 |
| F1-020 | — | #7 Templates | §7.4 |
| F1-021 | — | #7 Templates | §7.4 |
| F1-022 | #1 TUI primary | #7 Templates | §7.4 |
