# Architect Report -- Sprint 3.2 (Full Declarative Convergence)

**Date:** 2026-02-23
**Type:** ENHANCEMENT (structural)
**Sprint:** 3.2 -- Full Declarative Convergence
**Tasks:** F3-021, F3-008, F3-009, F3-010, F3-011, F3-012, F3-013

---

## 1. Architectural Decisions

### Decision AQ-1: Prune Flag Threading Mechanism

- **Choice**: (a) Add a `PrunePolicy` struct as a parameter to the private `compute_plan()` method. The `ApplyService` trait remains unchanged. The public `plan()` method always passes `PrunePolicy::default()` (no pruning). A new builder method `with_prune_policy()` on `DefaultApplyService` stores the policy, which `plan()` reads from `self`.
- **Rationale**: Sprint 3.1 established the precedent (AQ-2) of keeping `ApplyService` trait signatures unchanged and containing changes within `DefaultApplyService`. The prune policy is a CLI concern -- the trait represents the abstract capability, not the CLI's invocation options. Setting prune policy via builder keeps the trait clean while letting CLI callers configure behavior at construction time. The `compute_plan()` private method receives the policy from `self.prune_policy` and conditionally includes removal actions. This approach means the plan always contains ALL actions (including removals), and the `PrunePolicy` is attached to each removal action to indicate whether it should be executed. This follows the analyst's position (AMB-1): the plan is the single source of truth for what will happen.
- **Rejected**: (a-variant) Adding `PlanOptions` to the trait signature. This would cascade changes through every consumer (CLI, TUI, snapshot restore, tests using trait objects). No external caller needs to pass prune options -- it is an implementation detail of how `DefaultApplyService` computes plans.
- **Rejected**: (c) Separate `plan_with_options()` method on the trait. Unnecessary API surface. The trait should not know about prune flags.
- **Consequences**: `DefaultApplyService` gains a `prune_policy: PrunePolicy` field (default: no pruning). CLI sets it via `DefaultApplyService::new(...).with_prune_policy(policy)`. The `compute_plan()` method always computes removal candidates, but tags removal actions with `prunable: true` so the executor and display can distinguish them. Removal actions are always shown in plan output (with a hint if pruning is disabled), and only executed when prune policy allows.

**Refinement on AMB-1**: After further consideration, the approach differs slightly from the analyst's recommendation. Rather than conditionally including removal actions in the plan, ALL removal actions are always included. Each removal action carries a `prunable` boolean. The `execute_action()` method checks the prune policy before executing removal actions. The plan display shows removal actions with `[PRUNE]` badge and a hint "use --prune to execute". This preserves the plan as a complete picture of what diverges from desired state, while making pruning opt-in at execution time. The CLI `iron plan` command always shows all actions including prunable ones.

### Decision AQ-2: managed_packages Data Type

- **Choice**: Use `Vec<String>` for serialization in `IronState`. Convert to `HashSet<String>` at the call site within `compute_plan()` for O(1) membership testing.
- **Rationale**: The analyst's recommendation is correct. `Vec<String>` provides stable JSON serialization order and insertion-order preservation for display. The managed lists are small (tens to low hundreds of entries) so the Vec-to-HashSet conversion in `compute_plan()` is negligible. The `IronState` struct is a persistence model -- optimizing for serialization stability is more important than in-memory access patterns. The `ActualState.installed_packages` uses `HashSet` because it is an in-memory query result, never persisted by Iron. Different concerns, different containers.
- **Rejected**: `HashSet<String>` on `IronState`. JSON serialization of HashSet produces non-deterministic ordering. This causes spurious diffs when state.json is inspected manually or compared across runs. For a configuration management tool, deterministic state files matter.
- **Rejected**: `HashMap<String, ManagedMeta>` with install timestamp and source module. Premature complexity. The source module can be reconstructed from the desired state at any time. Timestamps add value only for debugging, which is not a Sprint 3.2 requirement. If needed later, the migration is straightforward (add metadata fields with `#[serde(default)]`).
- **Consequences**: Three new `Vec<String>` fields on `IronState`. Helper methods on `StateManager` do linear contains-checks (acceptable for list sizes under 1000). `compute_plan()` converts to `HashSet` once per invocation.

### Decision AQ-3: RiskLevel Naming

- **Choice**: (b) Scope the new enum as `RiskLevel` within the `apply` module. External code references it as `apply::RiskLevel` or imports it explicitly.
- **Rationale**: The existing `packages::RiskLevel` (Low/Medium/High/Critical) is used exclusively in the update flow (`check_updates`, `UpdatePreview`). The new apply `RiskLevel` (ReadOnly/Additive/Destructive/Critical) is used exclusively in the apply/plan flow. These are different domains with different semantics. Module-level scoping is the idiomatic Rust approach -- the same pattern used by `std::io::Error` vs `std::fmt::Error`. There is no place in the codebase where both enums appear in the same scope.
- **Rejected**: (a) `ApplyRiskLevel`. Verbose and redundant when module-qualified. `apply::ApplyRiskLevel` stutters.
- **Rejected**: (c) Unify into one enum. The variants have different semantics (Low/Medium/High vs ReadOnly/Additive/Destructive). Forcing them into one enum would create unused variants in each context and obscure the distinct meanings.
- **Consequences**: `apply::RiskLevel` enum added to `apply.rs`. No naming collision with `packages::RiskLevel`. The `plan` CLI command imports `apply::RiskLevel` directly.

### Decision AQ-4: Dotfile I/O at Plan Time

- **Choice**: YES, `compute_plan()` reads dotfile source files from disk during planning for template detection.
- **Rationale**: The analyst's position is correct. `compute_plan()` already performs I/O via `ActualState::scan()` (which queries pacman, systemctl, and checksums files). Reading dotfile source files (~1-10KB config files) is negligible compared to pacman queries (~200ms). The alternative -- deferring template detection to execution time -- would mean `iron plan` cannot show whether a dotfile will be symlinked or template-rendered, defeating the purpose of plan preview. Template detection is a planning concern, not an execution concern.
- **Rejected**: Template detection during `resolve_desired_state()`. That function reads only TOML config files and is used by `iron status` without `--full`. Adding arbitrary filesystem I/O there would break the performance contract for the fast path.
- **Consequences**: `compute_plan()` calls `std::fs::read_to_string()` for each dotfile source. If a file is unreadable, the action falls back to `CreateSymlink` with a warning logged. The read content is reused for `RenderAndCopy` action construction (no double-read).

### Decision AQ-5: ApplyService Trait Changes

- **Choice**: Keep the `ApplyService` trait unchanged. Prune policy is set on `DefaultApplyService` via a builder method (`with_prune_policy`), stored as `self.prune_policy`, and read by the private `compute_plan()`.
- **Rationale**: Consistent with Sprint 3.1 AQ-2 which established the pattern of keeping trait signatures stable while evolving private implementation methods. The trait represents the abstract contract for apply operations. Prune flags are a CLI-specific configuration knob, not part of the core contract. All callers (TUI, snapshot restore, tests) that use the trait interface continue to work without changes.
- **Rejected**: Adding `PrunePolicy` parameter to `plan()` trait method. Would cascade to all trait implementors and all callers. No benefit -- the TUI and snapshot restore never pass prune flags.
- **Consequences**: `DefaultApplyService` struct gains `prune_policy: PrunePolicy` field. CLI constructs the service with appropriate prune policy based on command-line flags. The `execute()` method checks `self.prune_policy` before executing removal actions. Mock implementations used in tests are unaffected.

### Decision AMB-2: Bootstrap Strategy for managed_packages

- **Choice**: On the first `execute()` invocation where `managed_packages` is empty AND the plan contains `InstallPackages` actions, bootstrap by seeding `managed_packages` with all desired packages that are already installed in `ActualState`. This happens once, inside `execute()`, before processing actions.
- **Rationale**: The analyst's position is correct. Without bootstrap, the first apply after upgrade would never have any managed packages to compare against for removal. The `managed_packages.is_empty()` guard ensures this only happens once. Bootstrapping during `execute()` (not `plan()`) ensures it only triggers when the user actually commits to apply, not during read-only `iron plan`.
- **Consequences**: `execute()` gains a bootstrap block at the top. After bootstrapping, subsequent applies track packages incrementally. AUR packages are included in the bootstrap (they are in both `desired.aur_packages` and `actual.installed_packages`). Bundle-sourced packages are also included.

### Decision AMB-3: Template Detection Timing (same as AQ-4)

Covered by AQ-4 above. Template detection happens at plan time in `compute_plan()`.

### Decision AMB-4: Template Variables Override DotfileMapping.link

- **Choice**: The decision tree for dotfile action selection is:
  1. If source file contains `{{...}}` patterns -> `RenderAndCopy` (regardless of `link` field)
  2. If `link = false` (explicitly set) -> `CopyFile`
  3. If `link = true` (default) -> `CreateSymlink`
- **Rationale**: Template files must be rendered and deployed as copies. A symlink to a template source would show the raw `{{variable}}` patterns, not the rendered content. Template detection takes priority because it is a content-driven decision that cannot be overridden by the declarative `link` field -- rendering is always required. The `link = false` setting provides explicit copy-without-rendering for binary files or files the user does not want symlinked.
- **Consequences**: The `link` field is effectively a 3-way switch: `true` (symlink), `false` (copy), or "overridden by template detection" (render+copy). The TOML config does not need changes -- `link = true` on a template file silently becomes render+copy.

---

## 2. Struct Definitions

### 2.1 PrunePolicy

File: `crates/iron-core/src/services/apply.rs`

```rust
/// Policy controlling whether removal actions are executed during apply.
///
/// Removal actions (RemovePackages, DisableService, RemoveSymlink, DeactivateModule)
/// are always included in the plan for visibility. This policy controls whether
/// they are actually executed.
#[derive(Debug, Clone, Default)]
pub struct PrunePolicy {
    /// Prune packages no longer in desired state
    pub packages: bool,
    /// Prune services no longer in desired state
    pub services: bool,
    /// Prune dotfiles/symlinks no longer in desired state
    pub dotfiles: bool,
}

impl PrunePolicy {
    /// No pruning (default). Removal actions are shown but not executed.
    pub fn none() -> Self {
        Self::default()
    }

    /// Prune all resource types.
    pub fn all() -> Self {
        Self {
            packages: true,
            services: true,
            dotfiles: true,
        }
    }

    /// Check if any pruning is enabled.
    pub fn any_enabled(&self) -> bool {
        self.packages || self.services || self.dotfiles
    }
}
```

### 2.2 RiskLevel

File: `crates/iron-core/src/services/apply.rs`

```rust
/// Risk classification for apply actions.
///
/// Used to scale confirmation UX: higher risk requires more explicit confirmation.
/// Distinct from `packages::RiskLevel` which classifies update risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    /// No system changes (empty plan)
    ReadOnly,
    /// Adds to system, easily reversible (install, symlink, enable)
    Additive,
    /// Modifies existing state, backup created (copy, render, remove symlink)
    Destructive,
    /// Potentially dangerous, hard to reverse (remove packages)
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadOnly => write!(f, "read-only"),
            Self::Additive => write!(f, "additive"),
            Self::Destructive => write!(f, "destructive"),
            Self::Critical => write!(f, "critical"),
        }
    }
}
```

### 2.3 New IronState Fields

File: `crates/iron-core/src/state.rs`

Add after `last_scan_report` (line 180):

```rust
/// Global Iron state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IronState {
    // ...existing fields...

    /// Packages installed by Iron (via apply/module enable).
    /// Only packages in this list are candidates for removal by --prune.
    #[serde(default)]
    pub managed_packages: Vec<String>,

    /// Services enabled by Iron (via apply).
    /// Only services in this list are candidates for disabling by --prune.
    #[serde(default)]
    pub managed_services: Vec<String>,

    /// Dotfile target paths created by Iron (via apply).
    /// Only dotfiles in this list are candidates for removal by --prune.
    #[serde(default)]
    pub managed_dotfiles: Vec<String>,

    /// Timestamp of last successful apply execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_apply: Option<DateTime<Utc>>,
}
```

### 2.4 Complete ApplyAction Enum (New)

File: `crates/iron-core/src/services/apply.rs`

```rust
/// Individual action in an apply plan.
#[derive(Debug, Clone, Serialize)]
pub enum ApplyAction {
    /// Install missing packages via pacman
    InstallPackages { packages: Vec<String> },

    /// Install missing AUR packages
    InstallAurPackages { packages: Vec<String> },

    /// Create a dotfile symlink
    CreateSymlink {
        source: String,
        target: String,
        module_id: String,
    },

    /// Enable a systemd service
    EnableService { name: String },

    /// Record a module as active in state
    ActivateModule { id: String },

    // ── New in Sprint 3.2 ──────────────────────────────────

    /// Render a template file and deploy as a copy
    RenderAndCopy {
        source: String,
        target: String,
        variables: HashMap<String, String>,
        module_id: String,
    },

    /// Copy a file (no template rendering)
    CopyFile {
        source: String,
        target: String,
        backup_existing: bool,
        module_id: String,
    },

    /// Remove packages no longer in desired state (requires --prune)
    RemovePackages { packages: Vec<String> },

    /// Disable a service no longer in desired state (requires --prune)
    DisableService { name: String },

    /// Remove a symlink/file no longer in desired state (requires --prune)
    RemoveSymlink { target: String },

    /// Deactivate a module no longer in desired state (requires --prune)
    DeactivateModule { id: String },
}
```

### 2.5 ApplyAction Methods

```rust
impl ApplyAction {
    /// Risk classification for this action.
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Self::InstallPackages { .. } => RiskLevel::Additive,
            Self::InstallAurPackages { .. } => RiskLevel::Additive,
            Self::CreateSymlink { .. } => RiskLevel::Additive,
            Self::EnableService { .. } => RiskLevel::Additive,
            Self::ActivateModule { .. } => RiskLevel::Additive,
            Self::CopyFile { backup_existing: false, .. } => RiskLevel::Additive,
            Self::CopyFile { backup_existing: true, .. } => RiskLevel::Destructive,
            Self::RenderAndCopy { .. } => RiskLevel::Destructive,
            Self::RemoveSymlink { .. } => RiskLevel::Destructive,
            Self::DisableService { .. } => RiskLevel::Destructive,
            Self::DeactivateModule { .. } => RiskLevel::Destructive,
            Self::RemovePackages { .. } => RiskLevel::Critical,
        }
    }

    /// Whether this action requires --prune to execute.
    pub fn is_prunable(&self) -> bool {
        matches!(
            self,
            Self::RemovePackages { .. }
                | Self::DisableService { .. }
                | Self::RemoveSymlink { .. }
                | Self::DeactivateModule { .. }
        )
    }

    /// Human-readable display for plan output
    pub fn display(&self) -> String {
        match self {
            Self::InstallPackages { packages } => {
                format!("[+] Install {} package(s): {}",
                    packages.len(), packages.join(", "))
            }
            Self::InstallAurPackages { packages } => {
                format!("[+] Install {} AUR package(s): {}",
                    packages.len(), packages.join(", "))
            }
            Self::CreateSymlink { source, target, module_id } => {
                format!("[+] Link {} -> {} ({})", target, source, module_id)
            }
            Self::EnableService { name } => {
                format!("[+] Enable service: {}", name)
            }
            Self::ActivateModule { id } => {
                format!("[+] Activate module: {}", id)
            }
            Self::RenderAndCopy { target, module_id, .. } => {
                format!("[!] Render template -> {} ({})", target, module_id)
            }
            Self::CopyFile { target, module_id, .. } => {
                format!("[+] Copy file -> {} ({})", target, module_id)
            }
            Self::RemovePackages { packages } => {
                format!("[!!] Remove {} package(s): {}",
                    packages.len(), packages.join(", "))
            }
            Self::DisableService { name } => {
                format!("[!] Disable service: {}", name)
            }
            Self::RemoveSymlink { target } => {
                format!("[!] Remove symlink: {}", target)
            }
            Self::DeactivateModule { id } => {
                format!("[!] Deactivate module: {}", id)
            }
        }
    }
}
```

### 2.6 ApplyPlan Extensions

```rust
impl ApplyPlan {
    /// Maximum risk level across all actions.
    /// Returns ReadOnly for an empty plan.
    pub fn max_risk(&self) -> RiskLevel {
        self.actions.iter()
            .map(|a| a.risk_level())
            .max()
            .unwrap_or(RiskLevel::ReadOnly)
    }

    /// Count of prunable (removal) actions in the plan.
    pub fn prune_count(&self) -> usize {
        self.actions.iter().filter(|a| a.is_prunable()).count()
    }

    /// Summary string for display (updated for new variants)
    pub fn summary(&self) -> String {
        let pkgs_install: usize = self.actions.iter().filter_map(|a| match a {
            ApplyAction::InstallPackages { packages } => Some(packages.len()),
            ApplyAction::InstallAurPackages { packages } => Some(packages.len()),
            _ => None,
        }).sum();
        let pkgs_remove: usize = self.actions.iter().filter_map(|a| match a {
            ApplyAction::RemovePackages { packages } => Some(packages.len()),
            _ => None,
        }).sum();
        let links = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::CreateSymlink { .. }))
            .count();
        let copies = self.actions.iter()
            .filter(|a| matches!(a,
                ApplyAction::CopyFile { .. } | ApplyAction::RenderAndCopy { .. }))
            .count();
        let removes = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::RemoveSymlink { .. }))
            .count();
        let svcs_enable = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::EnableService { .. }))
            .count();
        let svcs_disable = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::DisableService { .. }))
            .count();
        let mods_activate = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::ActivateModule { .. }))
            .count();
        let mods_deactivate = self.actions.iter()
            .filter(|a| matches!(a, ApplyAction::DeactivateModule { .. }))
            .count();

        let mut parts = Vec::new();
        if pkgs_install > 0 { parts.push(format!("+{} pkg", pkgs_install)); }
        if pkgs_remove > 0 { parts.push(format!("-{} pkg", pkgs_remove)); }
        if links > 0 { parts.push(format!("+{} link", links)); }
        if copies > 0 { parts.push(format!("+{} copy", copies)); }
        if removes > 0 { parts.push(format!("-{} file", removes)); }
        if svcs_enable > 0 { parts.push(format!("+{} svc", svcs_enable)); }
        if svcs_disable > 0 { parts.push(format!("-{} svc", svcs_disable)); }
        if mods_activate > 0 { parts.push(format!("+{} mod", mods_activate)); }
        if mods_deactivate > 0 { parts.push(format!("-{} mod", mods_deactivate)); }

        if parts.is_empty() {
            "No changes".to_string()
        } else {
            parts.join(", ")
        }
    }
}
```

---

## 3. API Contracts

### 3.1 DefaultApplyService Constructor Change

```rust
pub struct DefaultApplyService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Arc<dyn PackageManager>,
    service_manager: Arc<dyn SystemService>,
    prune_policy: PrunePolicy,  // NEW
}

impl DefaultApplyService {
    pub fn new(
        iron_root: &Path,
        state_manager: StateManager,
        package_manager: Arc<dyn PackageManager>,
        service_manager: Arc<dyn SystemService>,
    ) -> Self {
        Self {
            iron_root: iron_root.to_path_buf(),
            state_manager,
            package_manager,
            service_manager,
            prune_policy: PrunePolicy::default(),  // No pruning by default
        }
    }

    /// Set the prune policy for removal actions.
    pub fn with_prune_policy(mut self, policy: PrunePolicy) -> Self {
        self.prune_policy = policy;
        self
    }
}
```

### 3.2 Modified compute_plan() Signature

The private method signature is unchanged from Sprint 3.1. It now reads `self.prune_policy` and `self.state_manager` internally:

```rust
impl DefaultApplyService {
    /// Compute the plan by diffing desired against actual system state.
    ///
    /// Always includes removal actions for visibility.
    /// The `self.prune_policy` determines whether removal actions
    /// will be executed (checked in `execute_action()`).
    fn compute_plan(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> IronResult<ApplyPlan> {
        let mut actions = Vec::new();

        // 1. Package install diff (unchanged)
        // 2. Dotfile diff (now with template detection + CopyFile)
        // 3. Service diff (unchanged)
        // 4. Module activation diff (unchanged)

        // 5. NEW: Package removal diff
        let managed_pkgs: HashSet<String> =
            self.state_manager.state().managed_packages.iter().cloned().collect();
        let desired_pkgs: HashSet<&str> =
            desired.packages.iter().map(|s| s.as_str()).collect();
        let to_remove: Vec<String> = managed_pkgs.iter()
            .filter(|p| actual.installed_packages.contains(*p))
            .filter(|p| !desired_pkgs.contains(p.as_str()))
            .cloned()
            .collect();
        if !to_remove.is_empty() {
            actions.push(ApplyAction::RemovePackages { packages: to_remove });
        }

        // 6. NEW: Service disable diff
        // 7. NEW: Symlink removal diff
        // 8. NEW: Module deactivation diff

        Ok(ApplyPlan { actions })
    }
}
```

### 3.3 Modified execute() with Managed Tracking + Prune Gating

```rust
impl ApplyService for DefaultApplyService {
    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult> {
        // Auto-snapshot (existing)
        // ...

        // F3-021: Bootstrap managed_packages on first apply
        {
            let state = self.state_manager.state();
            if state.managed_packages.is_empty() {
                drop(state);
                // Bootstrap will be done after first successful action
                // or as a pre-step in execute
                self.bootstrap_managed_tracking(plan)?;
            }
        }

        let start = Instant::now();
        let mut result = ApplyResult::default();

        for action in &plan.actions {
            // Skip prunable actions if prune policy does not allow
            if action.is_prunable() && !self.should_execute_prune(action) {
                continue;
            }

            match self.execute_action(action) {
                Ok(()) => {
                    result.succeeded += 1;
                    // F3-021: Record managed resource
                    self.record_managed_resource(action);
                }
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!("{}: {}", action.display(), e));
                }
            }
        }

        // F3-021: Record last_apply timestamp
        self.state_manager.with_locked_state(|state| {
            state.last_apply = Some(Utc::now());
        })?;

        result.duration_secs = start.elapsed().as_secs_f64();
        Ok(result)
    }
}
```

### 3.4 New StateManager Methods

```rust
impl StateManager {
    /// Record packages as managed by Iron. Deduplicates.
    pub fn record_managed_packages(&self, packages: &[String]) -> IronResult<()> {
        self.with_locked_state(|state| {
            for pkg in packages {
                if !state.managed_packages.contains(pkg) {
                    state.managed_packages.push(pkg.clone());
                }
            }
        })
    }

    /// Remove packages from managed tracking.
    pub fn unrecord_managed_packages(&self, packages: &[String]) -> IronResult<()> {
        self.with_locked_state(|state| {
            state.managed_packages.retain(|p| !packages.contains(p));
        })
    }

    /// Record a service as managed by Iron.
    pub fn record_managed_service(&self, name: &str) -> IronResult<()> {
        let name_owned = name.to_string();
        self.with_locked_state(|state| {
            if !state.managed_services.contains(&name_owned) {
                state.managed_services.push(name_owned);
            }
        })
    }

    /// Remove a service from managed tracking.
    pub fn unrecord_managed_service(&self, name: &str) -> IronResult<()> {
        let name_owned = name.to_string();
        self.with_locked_state(|state| {
            state.managed_services.retain(|s| s != &name_owned);
        })
    }

    /// Record a dotfile target as managed by Iron.
    pub fn record_managed_dotfile(&self, target: &str) -> IronResult<()> {
        let target_owned = target.to_string();
        self.with_locked_state(|state| {
            if !state.managed_dotfiles.contains(&target_owned) {
                state.managed_dotfiles.push(target_owned);
            }
        })
    }

    /// Remove a dotfile target from managed tracking.
    pub fn unrecord_managed_dotfile(&self, target: &str) -> IronResult<()> {
        let target_owned = target.to_string();
        self.with_locked_state(|state| {
            state.managed_dotfiles.retain(|d| d != &target_owned);
        })
    }

    /// Get current managed packages list.
    pub fn managed_packages(&self) -> Vec<String> {
        self.state().managed_packages.clone()
    }

    /// Get current managed services list.
    pub fn managed_services(&self) -> Vec<String> {
        self.state().managed_services.clone()
    }

    /// Get current managed dotfiles list.
    pub fn managed_dotfiles(&self) -> Vec<String> {
        self.state().managed_dotfiles.clone()
    }

    /// Update last_apply timestamp to now.
    pub fn update_last_apply(&self) -> IronResult<()> {
        self.with_locked_state(|state| {
            state.last_apply = Some(Utc::now());
        })
    }
}
```

### 3.5 Private Helper Methods on DefaultApplyService

```rust
impl DefaultApplyService {
    /// Check whether a prunable action should be executed based on prune policy.
    fn should_execute_prune(&self, action: &ApplyAction) -> bool {
        match action {
            ApplyAction::RemovePackages { .. } => self.prune_policy.packages,
            ApplyAction::DisableService { .. } => self.prune_policy.services,
            ApplyAction::RemoveSymlink { .. } => self.prune_policy.dotfiles,
            ApplyAction::DeactivateModule { .. } => self.prune_policy.dotfiles,
            _ => true, // Not prunable, always execute
        }
    }

    /// Record a successfully executed action in managed tracking.
    fn record_managed_resource(&self, action: &ApplyAction) {
        let _ = match action {
            ApplyAction::InstallPackages { packages } => {
                self.state_manager.record_managed_packages(packages)
            }
            ApplyAction::InstallAurPackages { packages } => {
                self.state_manager.record_managed_packages(packages)
            }
            ApplyAction::CreateSymlink { target, .. }
            | ApplyAction::CopyFile { target, .. }
            | ApplyAction::RenderAndCopy { target, .. } => {
                self.state_manager.record_managed_dotfile(target)
            }
            ApplyAction::EnableService { name } => {
                self.state_manager.record_managed_service(name)
            }
            ApplyAction::RemovePackages { packages } => {
                self.state_manager.unrecord_managed_packages(packages)
            }
            ApplyAction::DisableService { name } => {
                self.state_manager.unrecord_managed_service(name)
            }
            ApplyAction::RemoveSymlink { target } => {
                self.state_manager.unrecord_managed_dotfile(target)
            }
            ApplyAction::ActivateModule { .. }
            | ApplyAction::DeactivateModule { .. } => Ok(()),
        };
    }

    /// Bootstrap managed_packages on first apply.
    /// Seeds with all desired packages that are already installed.
    fn bootstrap_managed_tracking(&self, plan: &ApplyPlan) -> IronResult<()> {
        // Only bootstrap if managed lists are all empty
        let state = self.state_manager.state();
        if !state.managed_packages.is_empty()
            || !state.managed_services.is_empty()
            || !state.managed_dotfiles.is_empty()
        {
            return Ok(());
        }
        drop(state);

        // Resolve what's desired and installed
        // The plan already has this info implicitly.
        // For packages: desired = all packages in the plan's install actions
        //   + packages that are already installed (not in install actions).
        // We need the desired state, which we can reconstruct from the
        // current host. However, to avoid re-resolving, we accept that
        // bootstrap will be incremental: each successful action in this
        // execute() call records its resources. After the first apply
        // completes, managed_packages will contain everything Iron installed.
        //
        // For pre-existing packages: we scan the actual state and desired
        // state to find desired packages that are already installed.
        // This requires access to the desired state and actual state,
        // which are not available here. The bootstrap must happen
        // in execute() or plan() where these are available.
        //
        // Decision: defer full bootstrap to a dedicated method called
        // from plan() -> compute_plan() path, or from execute() with
        // access to both desired and actual states.
        Ok(())
    }
}
```

**Note on bootstrap implementation**: The `bootstrap_managed_tracking` method above is a skeleton. The developer should implement bootstrap at the top of `execute()` with access to desired and actual states. The concrete approach:

1. Before the action loop in `execute()`, check if `managed_packages.is_empty()`.
2. If empty, resolve the desired state (already available from `plan()`) and actual state.
3. Seed: `managed_packages = desired.packages intersect actual.installed_packages`
4. Seed: `managed_services = desired.services intersect enabled_services`
5. Seed: `managed_dotfiles = desired.dotfile_targets intersect existing_files`
6. Persist once.

Since `execute()` currently does not receive the desired/actual states, the developer has two options:
- (a) Store the desired state as a field on `ApplyPlan` (lightweight, `DesiredState` is already `Clone`).
- (b) Pass desired/actual as parameters to a bootstrap helper called from `plan()` after computing.

**Recommended**: Option (b). The `plan()` method has both `desired` and `actual`. After computing the plan, if managed lists are empty, call `self.bootstrap_managed(desired, actual)` which seeds the managed lists. This happens before `execute()`, so when `execute()` runs, managed lists are already populated and the incremental recording logic works correctly.

---

## 4. Dotfile Decision Tree in compute_plan()

```rust
// In compute_plan(), for each dotfile in desired.dotfiles:
for dotfile in &desired.dotfiles {
    let target_expanded = expand_home(Path::new(&dotfile.target))
        .to_string_lossy().to_string();

    // Check actual state for this dotfile
    let actual_file = actual.managed_files.iter()
        .find(|f| f.target == target_expanded);

    let needs_action = /* existing logic to determine if action needed */;

    if needs_action {
        // Find owning module
        let module_id = /* existing module lookup */;

        // Read source file content for template detection
        let source_path = self.iron_root.join("modules")
            .join(&module_id).join(&dotfile.source);
        let source_content = std::fs::read_to_string(&source_path).ok();

        let has_templates = source_content.as_ref()
            .map(|c| iron_fs::template::has_variables(c))
            .unwrap_or(false);

        let action = if has_templates {
            ApplyAction::RenderAndCopy {
                source: dotfile.source.clone(),
                target: dotfile.target.clone(),
                variables: desired.variables.clone(),
                module_id,
            }
        } else if !dotfile.link {
            ApplyAction::CopyFile {
                source: dotfile.source.clone(),
                target: dotfile.target.clone(),
                backup_existing: actual_file
                    .map(|f| f.exists)
                    .unwrap_or(false),
                module_id,
            }
        } else {
            ApplyAction::CreateSymlink {
                source: dotfile.source.clone(),
                target: dotfile.target.clone(),
                module_id,
            }
        };

        actions.push(action);
    }
}
```

---

## 5. Confirmation Flow (CLI)

File: `crates/iron-cli/src/commands/apply.rs`

The confirmation logic replaces the current simple y/N prompt (lines 56-63). It scales with the plan's maximum risk level:

```rust
// After displaying the plan, before executing:

let max_risk = plan.max_risk();
let prune_count = plan.prune_count();

// Show prune hint if there are prunable actions and pruning is not enabled
if prune_count > 0 && !has_prune_flags {
    output.info(&format!(
        "  {} removal action(s) shown but will not execute. \
         Use --prune to include.",
        prune_count
    ));
}

if dry_run {
    output.success("[DRY RUN] No changes made.");
    return Ok(());
}

// Risk-scaled confirmation
let confirmed = match max_risk {
    RiskLevel::ReadOnly => true,  // Nothing to do, but shouldn't reach here
    RiskLevel::Additive => {
        if yes { true } else {
            output.info("Proceed? [y/N]");
            read_yes_no()?
        }
    }
    RiskLevel::Destructive => {
        if yes { true } else {
            output.info("Review changes above. Proceed? [y/N]");
            read_yes_no()?
        }
    }
    RiskLevel::Critical => {
        if yes {
            // --yes flag does NOT bypass critical confirmation
            output.warning("Critical changes detected. --yes flag does not bypass this.");
        }
        output.warning("Type 'yes' to confirm critical changes:");
        read_typed_yes()?
    }
};

if !confirmed {
    output.info("Cancelled.");
    return Ok(());
}
```

**Key decision**: The `--yes` flag does NOT bypass `Critical` confirmation. Critical actions (package removal) always require typed "yes" confirmation. This is a safety invariant that prevents accidental mass uninstalls in scripts. The `--yes` flag skips Additive and Destructive confirmations only.

**CLI flag additions** in `crates/iron-cli/src/cli.rs`:

```rust
/// Converge system to declared state
Apply {
    /// Dry run (show plan, do not execute)
    #[arg(long)]
    dry_run: bool,

    /// Apply only this module
    #[arg(short, long)]
    module: Option<String>,

    /// Skip confirmation (except for critical changes)
    #[arg(short, long)]
    yes: bool,

    /// Prune all resource types (remove packages/services/dotfiles
    /// no longer in desired state)
    #[arg(long)]
    prune: bool,

    /// Prune only packages
    #[arg(long)]
    prune_packages: bool,

    /// Prune only services
    #[arg(long)]
    prune_services: bool,

    /// Prune only dotfiles/symlinks
    #[arg(long)]
    prune_dotfiles: bool,
},
```

**PrunePolicy construction from CLI flags**:

```rust
let prune_policy = if prune {
    PrunePolicy::all()
} else {
    PrunePolicy {
        packages: prune_packages,
        services: prune_services,
        dotfiles: prune_dotfiles,
    }
};
```

---

## 6. Migration Notes

### 6.1 Backward Compatibility of New IronState Fields

All four new fields use `#[serde(default)]`:

- `managed_packages: Vec<String>` -- defaults to `Vec::new()`
- `managed_services: Vec<String>` -- defaults to `Vec::new()`
- `managed_dotfiles: Vec<String>` -- defaults to `Vec::new()`
- `last_apply: Option<DateTime<Utc>>` -- defaults to `None`

Existing `state.json` files (including `{}`) will deserialize correctly. The empty managed lists trigger the bootstrap behavior on first apply. No manual migration is needed.

### 6.2 ApplyAction Enum Expansion Strategy

Adding 6 new variants to `ApplyAction` requires updating ALL exhaustive match sites atomically:

| Match Site | File | What to Add |
|---|---|---|
| `display()` | `apply.rs` | Display strings for 6 new variants |
| `risk_level()` | `apply.rs` | Risk classification for 6 new variants |
| `is_prunable()` | `apply.rs` | Return `true` for 4 removal variants |
| `summary()` | `apply.rs` | Count categories for new variants |
| `execute_action()` | `apply.rs` | Execution logic for 6 new variants |
| `record_managed_resource()` | `apply.rs` | Managed tracking for 6 new variants |
| CLI plan display | `plan.rs` | Group and display new variants |

**Recommendation**: Add all 6 variants in a single commit with stub execution logic (`todo!()`) for Wave 2 variants. Then implement execution logic per-task. This prevents partial compilation failures when only some variants exist.

### 6.3 summary() Breaking Change

The current `summary()` method returns format `"2 package(s), 1 symlink(s), 1 service(s), 1 module(s)"`. The new format is more compact: `"+2 pkg, +1 link, +1 svc, +1 mod"`. This is a display-only change that affects:

- `test_apply_plan_summary` in `apply.rs` -- test assertions must be updated
- `test_plan_summary_format` in `commands/apply.rs` -- test assertions must be updated
- TUI apply view (if it parses the summary string -- it does not, it displays it verbatim)

The developer should update these tests when implementing the summary changes.

### 6.4 display() Emoji Removal

The current `display()` method uses emoji prefixes. The new `display()` uses risk badges (`[+]`, `[!]`, `[!!]`). This is intentional -- risk badges are more informative and work reliably in all terminals. The CLAUDE.md convention says "Only use emojis if the user explicitly requests it." Tests referencing emoji in display output must be updated.

---

## 7. Execution Order Integration Points

### 7.1 execute_action() New Match Arms

```rust
fn execute_action(&self, action: &ApplyAction) -> IronResult<()> {
    match action {
        // ...existing 5 variants...

        ApplyAction::RenderAndCopy { source, target, variables, module_id } => {
            let target_path = expand_home(Path::new(target));
            let source_path = self.iron_root.join("modules")
                .join(module_id).join(source);

            // Read source
            let content = std::fs::read_to_string(&source_path)
                .map_err(|e| crate::FsError::IoError {
                    message: format!("Failed to read template {}: {}", source_path.display(), e),
                })?;

            // Render
            let rendered = iron_fs::template::render(&content, variables);

            // Create parent dirs
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            // Backup existing
            if target_path.exists() {
                let backup = target_path.with_extension("iron-backup");
                std::fs::rename(&target_path, &backup).ok();
            }

            // Write rendered content
            std::fs::write(&target_path, rendered)
                .map_err(|e| crate::FsError::IoError {
                    message: format!("Failed to write rendered file {}: {}",
                        target_path.display(), e),
                })?;

            Ok(())
        }

        ApplyAction::CopyFile { source, target, module_id, .. } => {
            let target_path = expand_home(Path::new(target));
            let source_path = self.iron_root.join("modules")
                .join(module_id).join(source);

            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            if target_path.exists() {
                let backup = target_path.with_extension("iron-backup");
                std::fs::rename(&target_path, &backup).ok();
            }

            std::fs::copy(&source_path, &target_path)
                .map_err(|e| crate::FsError::IoError {
                    message: format!("Failed to copy {} -> {}: {}",
                        source_path.display(), target_path.display(), e),
                })?;

            Ok(())
        }

        ApplyAction::RemovePackages { packages } => {
            self.package_manager.remove(packages, false)?;
            Ok(())
        }

        ApplyAction::DisableService { name } => {
            self.service_manager.disable_service(name)?;
            Ok(())
        }

        ApplyAction::RemoveSymlink { target } => {
            let target_path = expand_home(Path::new(target));
            if target_path.exists() || target_path.is_symlink() {
                let backup = target_path.with_extension("iron-backup");
                if target_path.is_file() || target_path.is_symlink() {
                    std::fs::rename(&target_path, &backup).ok();
                }
            }
            Ok(())
        }

        ApplyAction::DeactivateModule { id } => {
            self.state_manager.disable_module(id)?;
            Ok(())
        }
    }
}
```

---

## 8. Boundaries

- **In scope**: Managed resource tracking, template rendering in apply, CopyFile deployment, package/service/dotfile removal, risk levels, confirmation UX, prune flags.
- **Out of scope**: Hook execution (Sprint 3.3), `iron history` (Sprint 3.3), `dotfiles_sync` (Sprint 3.3), hostname auto-detection (Sprint 3.4), plan serialization (Phase 4).
- **Extension points**:
  - `PrunePolicy` can be extended with per-module or per-package overrides without changing the struct interface.
  - `RiskLevel` enum is `PartialOrd + Ord`, supporting future risk-based sorting in plan display.
  - `managed_packages/services/dotfiles` are `Vec<String>` -- can be upgraded to `Vec<ManagedMeta>` with `#[serde(default)]` in future sprints without breaking existing state files.
  - The `RunHook` variant (Sprint 3.3) is intentionally NOT included in this sprint's `ApplyAction` enum. It will be added in Sprint 3.3 with its own `HookType` and `HookBehavior` enums.
