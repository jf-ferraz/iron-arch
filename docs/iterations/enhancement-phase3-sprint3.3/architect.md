# Architect Report -- Sprint 3.3 (Execution Lifecycle Completion)

**Date:** 2026-02-23
**Type:** ENHANCEMENT (structural)
**Sprint:** 3.3 -- Execution Lifecycle Completion
**Tasks:** F3-014, F3-015, F3-016, F3-018, F3-017 (STRETCH) + 3 carryovers

---

## 1. Architectural Decisions

### Decision AQ-014-1: Hook Execution Mechanism

- **Choice**: Use `std::process::Command` directly, NOT the `CommandExecutor` trait. Wrap execution in a dedicated `run_hook()` helper with a configurable timeout (default 60 seconds, enforced via thread + channel pattern).
- **Rationale**: The `CommandExecutor` trait (in `crates/iron-core/src/resilience/command_executor.rs`) is designed for known system commands (pacman, git, systemctl) with circuit breaker semantics -- after N failures, it refuses to execute further calls to that command. This makes sense for infrastructure commands where repeated failures indicate systemic problems. Hooks are user-defined, arbitrary, and module-specific. A circuit breaker on "hook execution" would mean that if one module's hook fails 3 times, ALL module hooks get blocked. The failure domain is wrong. Additionally, `CommandExecutor` has retry logic with exponential backoff -- hooks should not retry automatically because they may not be idempotent (e.g., `gpasswd -a $USER video` is idempotent, but `curl ... | sh` is not). The CLAUDE.md rule "no raw `Command::new()` in production paths" was established for system commands where the circuit breaker adds genuine value. Hooks are a different category -- they are user code, not system infrastructure.
- **Rejected**: Using `CommandExecutor` with circuit breaker disabled (`use_circuit_breaker: false`) and `max_retries: 0`. This would work technically but abuses the abstraction -- stripping out both defining features of `CommandExecutor` makes it a complicated wrapper around `Command::new()` with no added value. It also pulls in the entire resilience dependency chain for no benefit.
- **Rejected**: Adding a new `HookExecutor` trait. Unnecessary abstraction for a straightforward operation. A simple function with timeout is sufficient. If hook execution needs to become more sophisticated (sandbox, resource limits), the function can be upgraded to a trait later.
- **Consequences**: A private `run_hook()` function is added to `DefaultApplyService` (or as a standalone function in `apply.rs`). It uses `Command::new("sh").arg("-c").arg(command)` with a timeout enforced via `child.wait_timeout()` or thread+channel pattern. Stderr and stdout are captured for logging. The function returns `Result<HookOutput, HookError>` where `HookError` carries the exit code, stderr, and whether it timed out.

### Decision AQ-014-2: Hook Working Directory

- **Choice**: Option A -- the module source directory (`iron_root/modules/<module-id>/`).
- **Rationale**: Hooks are defined per-module in `module.toml`. They typically reference module-local resources: setup scripts (`./scripts/setup.sh`), config files to validate, or relative paths. The module directory is the natural "project root" for a hook. The user's home directory is accessible via `$HOME` and the iron root via `$IRON_ROOT` (passed as an environment variable to the hook process).
- **Rejected**: Option B (user home). Too generic -- hooks would need absolute paths to reference any module-local files. The user's home is always available via `$HOME` inside the hook.
- **Rejected**: Option C (iron root). Better than home, but still forces hooks to include `modules/<id>/` prefixes for module-local file references.
- **Consequences**: The `run_hook()` function sets `current_dir` to `iron_root.join("modules").join(module_id)`. Two environment variables are injected into the hook process: `IRON_ROOT` (the iron config root) and `IRON_MODULE` (the module ID). These give hooks access to the broader context without coupling them to a specific directory layout.

### Decision AQ-014-3: Hook Ordering in compute_plan()

- **Choice**: Option A -- build separate vectors, concatenate in correct order at the end. The current `compute_plan()` method builds a single `actions` vector by pushing linearly. For hooks, we split into phases:
  1. `pre_hooks: Vec<ApplyAction>` -- pre_install hooks
  2. `install_actions: Vec<ApplyAction>` -- packages, dotfiles, services, module activation
  3. `post_hooks: Vec<ApplyAction>` -- post_install hooks
  4. `removal_pre_hooks: Vec<ApplyAction>` -- pre_uninstall hooks
  5. `removal_actions: Vec<ApplyAction>` -- removals, deactivations

  Final concatenation: `pre_hooks + install_actions + post_hooks + removal_pre_hooks + removal_actions`.

- **Rationale**: This is the simplest approach that guarantees correct ordering. The current linear push approach cannot handle "insert before all package installs" without index arithmetic. Separate vectors make the intent explicit and are trivially debuggable. The concatenation order is visible in one line of code, not spread across conditional logic.
- **Rejected**: Option B (sort by priority field). Adds complexity -- every action needs a priority, the sort must be stable to preserve relative order within a phase, and the phase semantics become implicit rather than explicit.
- **Rejected**: Option C (index-based insertion). Fragile -- indices shift as actions are added. Would require tracking "where did packages start" and "where did services end" as counters. Error-prone and hard to maintain.
- **Consequences**: The `compute_plan()` method is restructured from a single `let mut actions = Vec::new()` to five named vectors. The existing action-building logic moves into the `install_actions` vector. Hook planning inserts into `pre_hooks` and `post_hooks`. The final `Ok(ApplyPlan { actions: [pre_hooks, install_actions, post_hooks, removal_pre_hooks, removal_actions].concat() })` makes ordering explicit. Removal hooks (`pre_uninstall`) are placed before removal actions so the module's cleanup hook runs while the module's packages are still installed.

### Decision AQ-014-4: Ask Behavior in Non-Interactive Contexts

- **Choice**: Option A -- skip with warning. When `HookBehavior::Ask` is set and the context is non-interactive (`--yes` flag, TUI, piped stdin), the hook is skipped and a warning is logged: `"Skipping hook for <module_id> (ask behavior, non-interactive mode)"`.
- **Rationale**: The `Ask` behavior exists specifically to give users a choice. Auto-approving defeats the purpose -- if the user wanted auto-approval, they would use `Always`. Skipping is the safe default: it preserves user agency by ensuring hooks with `Ask` only run when a human explicitly confirms. Failing the entire plan would be too disruptive for a non-critical hook.
- **Rejected**: Option B (auto-approve). Violates the semantic contract of `Ask`. Users set `Ask` because the hook has side effects they want to review (e.g., adding user to groups, modifying system files).
- **Rejected**: Option C (fail the plan). Too aggressive. A hook behavior configuration should not prevent apply from running.
- **Consequences**: The `execute_action()` method for `RunHook` checks an `interactive: bool` flag (passed through the service or inferred from context). In `compute_plan()`, `Ask` hooks are always included in the plan (for visibility in `iron plan` output). The skip decision happens at execution time. Plan display shows `Ask` hooks with a `[?]` badge.

### Decision AQ-016-1: History Data Model

- **Choice**: Option C -- use the existing `IronState.last_operations: Vec<OperationRecord>` field. Enhance `OperationRecord` with optional `duration_secs: Option<f64>` and `action_count: Option<usize>` fields (both `#[serde(default)]`).
- **Rationale**: `OperationRecord` already has `operation`, `timestamp`, `status`, and `details` -- the core data for history display. The audit log (`Vec<AuditEntry>`) is low-level and records individual state mutations (enable_module, set_current_host), not user-facing operations. Aggregating audit entries by timestamp proximity is fragile and adds complexity. `last_operations` records operations at the right granularity -- it just needs minor enhancement for richer display.
- **Rejected**: Option A (aggregate audit entries). The audit log records state mutations, not user commands. Grouping by timestamp proximity is heuristic and unreliable (what if two operations happen within the same second?).
- **Rejected**: Option B (new `OperationHistory` model). Unnecessary -- `OperationRecord` already serves this purpose. Adding a parallel model would mean two places to record operations.
- **Consequences**: `OperationRecord` gains two optional fields. The `HistoryService` reads `last_operations` from `IronState`. Operation numbering is by index (most recent = #1, descending). The `iron history show <id>` command uses 1-based indexing into the reversed list. The `details` field on `OperationRecord` is used for the detailed view (action breakdown). Apply and update operations should populate `details` with a summary of actions performed.

---

## 2. Struct and Enum Definitions

### 2.1 HookBehavior Enum

File: `crates/iron-core/src/module.rs`

```rust
/// Controls when and how module hooks are executed.
///
/// Module hooks (pre_install, post_install, pre_uninstall, status_check)
/// are shell commands. This enum controls the execution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HookBehavior {
    /// Run every time the module is applied
    #[default]
    Always,
    /// Run only the first time (tracked in state.json, re-run with --force-hooks)
    Once,
    /// Prompt the user before running (skipped in non-interactive mode)
    Ask,
    /// Never run
    Skip,
}
```

### 2.2 HookType Enum

File: `crates/iron-core/src/module.rs`

```rust
/// Type of module lifecycle hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Runs before packages are installed and dotfiles deployed
    PreInstall,
    /// Runs after all packages, dotfiles, and services are configured
    PostInstall,
    /// Runs before module removal (packages still available)
    PreUninstall,
    /// Informational check of module health (not part of apply flow)
    StatusCheck,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreInstall => write!(f, "pre_install"),
            Self::PostInstall => write!(f, "post_install"),
            Self::PreUninstall => write!(f, "pre_uninstall"),
            Self::StatusCheck => write!(f, "status_check"),
        }
    }
}
```

### 2.3 HookSpec (Internal Planning Struct)

File: `crates/iron-core/src/services/apply.rs`

```rust
/// Internal representation of a hook to be planned.
/// Not serialized -- used only during compute_plan().
struct HookSpec {
    module_id: String,
    hook_type: HookType,
    command: String,
    behavior: HookBehavior,
}
```

### 2.4 Module Struct Changes

File: `crates/iron-core/src/module.rs`

Add to `Module` struct after `security_points`:

```rust
    /// Hook execution policy for this module
    #[serde(default)]
    pub hook_behavior: HookBehavior,

    /// Auto-mirror all files in module's dotfiles/ directory
    #[serde(default)]
    pub dotfiles_sync: bool,

    /// Override default target directory for dotfiles_sync
    /// Default is ~/.config/<module-id>/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dotfiles_sync_target: Option<String>,
```

### 2.5 RunHook Action Variant

File: `crates/iron-core/src/services/apply.rs`

Add to `ApplyAction` enum:

```rust
    /// Execute a module lifecycle hook (shell command)
    RunHook {
        /// Module that owns this hook
        module_id: String,
        /// Type of hook (pre_install, post_install, etc.)
        hook_type: HookType,
        /// Shell command to execute
        command: String,
        /// Execution behavior policy
        behavior: HookBehavior,
    },
```

### 2.6 Enhanced OperationRecord

File: `crates/iron-core/src/state.rs`

Add to `OperationRecord`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub status: OperationStatus,
    pub details: Option<String>,

    /// Duration of the operation in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,

    /// Number of actions in the operation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_count: Option<usize>,
}
```

### 2.7 HistoryEntry (Display Model)

File: `crates/iron-core/src/services/history.rs`

```rust
use crate::state::{OperationRecord, OperationStatus};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// A history entry for display purposes.
/// Wraps OperationRecord with a 1-based display index.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    /// 1-based index (most recent = 1)
    pub index: usize,
    /// Operation name (e.g., "apply", "update", "snapshot restore")
    pub operation: String,
    /// When it happened
    pub timestamp: DateTime<Utc>,
    /// How long it took
    pub duration_secs: Option<f64>,
    /// Number of actions
    pub action_count: Option<usize>,
    /// Outcome
    pub status: OperationStatus,
    /// Detailed breakdown (action list, errors)
    pub details: Option<String>,
}
```

### 2.8 IronState.hooks_executed Field

File: `crates/iron-core/src/state.rs`

Add to `IronState` after `last_apply`:

```rust
    /// Tracks which Once hooks have been executed per module.
    /// Key: module_id, Value: list of hook types that have run (e.g., ["post_install"])
    #[serde(default)]
    pub hooks_executed: HashMap<String, Vec<String>>,
```

---

## 3. Action Ordering Strategy

### Current compute_plan() Structure

```
actions.push(InstallPackages)      // line 914
actions.push(InstallAurPackages)   // line 926
for dotfile { actions.push(...) }  // lines 933-1033
for service { actions.push(EnableService) }  // lines 1036-1049
for module { actions.push(ActivateModule) }  // lines 1054-1060
actions.push(RemovePackages)       // lines 1062-1085
for svc { actions.push(DisableService) }     // lines 1087-1108
for dot { actions.push(RemoveSymlink) }      // lines 1110-1131
for mod { actions.push(DeactivateModule) }   // lines 1133-1142
```

### New Structure

```rust
fn compute_plan(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<ApplyPlan> {
    let mut pre_hooks: Vec<ApplyAction> = Vec::new();
    let mut install_actions: Vec<ApplyAction> = Vec::new();
    let mut post_hooks: Vec<ApplyAction> = Vec::new();
    let mut pre_uninstall_hooks: Vec<ApplyAction> = Vec::new();
    let mut removal_actions: Vec<ApplyAction> = Vec::new();

    // Load modules once for hook data
    let modules_dir = self.iron_root.join("modules");
    let loaded_modules: HashMap<String, Module> = desired.modules.iter()
        .filter_map(|mid| {
            Module::load(&modules_dir.join(mid)).ok().map(|m| (mid.clone(), m))
        })
        .collect();

    // ---- Phase 1: Collect pre_install hooks ----
    for module_id in &desired.modules {
        if let Some(module) = loaded_modules.get(module_id) {
            if module.hook_behavior == HookBehavior::Skip {
                continue;
            }
            if let Some(ref cmd) = module.pre_install {
                // Check Once tracking (skip if already executed and not --force-hooks)
                if module.hook_behavior == HookBehavior::Once
                    && self.is_hook_executed(module_id, &HookType::PreInstall)
                    && !self.force_hooks
                {
                    continue;
                }
                pre_hooks.push(ApplyAction::RunHook {
                    module_id: module_id.clone(),
                    hook_type: HookType::PreInstall,
                    command: cmd.clone(),
                    behavior: module.hook_behavior,
                });
            }
        }
    }

    // ---- Phase 2: Existing install/configure logic ----
    // (Package diff, dotfile diff, service diff, module activation)
    // All push into install_actions instead of actions

    // ---- Phase 3: Collect post_install hooks ----
    for module_id in &desired.modules {
        if let Some(module) = loaded_modules.get(module_id) {
            if module.hook_behavior == HookBehavior::Skip {
                continue;
            }
            if let Some(ref cmd) = module.post_install {
                if module.hook_behavior == HookBehavior::Once
                    && self.is_hook_executed(module_id, &HookType::PostInstall)
                    && !self.force_hooks
                {
                    continue;
                }
                post_hooks.push(ApplyAction::RunHook {
                    module_id: module_id.clone(),
                    hook_type: HookType::PostInstall,
                    command: cmd.clone(),
                    behavior: module.hook_behavior,
                });
            }
        }
    }

    // ---- Phase 4: Collect pre_uninstall hooks for deactivating modules ----
    let desired_mods: HashSet<&str> = desired.modules.iter().map(|s| s.as_str()).collect();
    let active_modules: HashSet<String> = self.state_manager.active_modules().into_iter().collect();
    for active_id in &active_modules {
        if !desired_mods.contains(active_id.as_str()) {
            // Module is being deactivated -- run pre_uninstall if defined
            if let Ok(module) = Module::load(&modules_dir.join(active_id)) {
                if module.hook_behavior != HookBehavior::Skip {
                    if let Some(ref cmd) = module.pre_uninstall {
                        pre_uninstall_hooks.push(ApplyAction::RunHook {
                            module_id: active_id.clone(),
                            hook_type: HookType::PreUninstall,
                            command: cmd.clone(),
                            behavior: module.hook_behavior,
                        });
                    }
                }
            }
        }
    }

    // ---- Phase 5: Existing removal logic ----
    // (RemovePackages, DisableService, RemoveSymlink, DeactivateModule)
    // All push into removal_actions

    // ---- Assemble final action list ----
    let mut actions = Vec::with_capacity(
        pre_hooks.len() + install_actions.len() + post_hooks.len()
        + pre_uninstall_hooks.len() + removal_actions.len()
    );
    actions.extend(pre_hooks);
    actions.extend(install_actions);
    actions.extend(post_hooks);
    actions.extend(pre_uninstall_hooks);
    actions.extend(removal_actions);

    Ok(ApplyPlan { actions })
}
```

### Execution Order Summary

```
1. pre_install hooks      (prepare environment before changes)
2. InstallPackages        (pacman -S)
3. InstallAurPackages     (paru/yay)
4. CreateSymlink/CopyFile/RenderAndCopy (dotfiles)
5. EnableService          (systemctl enable)
6. ActivateModule         (state tracking)
7. post_install hooks     (run after module is fully set up)
8. pre_uninstall hooks    (cleanup before removal)
9. RemovePackages         (requires --prune)
10. DisableService        (requires --prune)
11. RemoveSymlink         (requires --prune)
12. DeactivateModule      (requires --prune)
```

---

## 4. API Contracts

### 4.1 Hook Execution Function

File: `crates/iron-core/src/services/apply.rs`

```rust
use std::process::Command;
use std::time::Duration;

/// Output from a hook execution.
#[derive(Debug)]
struct HookOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Error from hook execution.
#[derive(Debug)]
enum HookError {
    /// Hook process exited with non-zero code
    Failed { exit_code: i32, stderr: String },
    /// Hook exceeded timeout
    TimedOut { timeout_secs: u64 },
    /// Could not start the hook process
    SpawnFailed { reason: String },
}

/// Execute a shell command as a module hook.
///
/// - Working directory: iron_root/modules/<module_id>/
/// - Timeout: 60 seconds (default)
/// - Environment: inherits parent env + IRON_ROOT + IRON_MODULE
/// - Shell: /bin/sh -c "<command>"
fn run_hook(
    iron_root: &Path,
    module_id: &str,
    command: &str,
    timeout: Duration,
) -> Result<HookOutput, HookError> {
    let working_dir = iron_root.join("modules").join(module_id);

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(&working_dir)
        .env("IRON_ROOT", iron_root)
        .env("IRON_MODULE", module_id)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| HookError::SpawnFailed {
            reason: e.to_string(),
        })?;

    // Timeout enforcement via wait_timeout (requires wait-timeout crate)
    // or thread+channel pattern matching existing resilience patterns
    // Implementation detail for developer.

    todo!("Developer implements timeout wait + output capture")
}
```

### 4.2 DefaultApplyService Changes

```rust
pub struct DefaultApplyService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Arc<dyn PackageManager>,
    service_manager: Arc<dyn SystemService>,
    prune_policy: PrunePolicy,
    force_hooks: bool,       // NEW: --force-hooks flag
    interactive: bool,       // NEW: whether Ask hooks can prompt
    hook_timeout: Duration,  // NEW: timeout for hook execution (default 60s)
}

impl DefaultApplyService {
    /// Set force-hooks mode (re-runs Once hooks).
    pub fn with_force_hooks(mut self, force: bool) -> Self {
        self.force_hooks = force;
        self
    }

    /// Set interactive mode (controls Ask hook behavior).
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Check if a hook has been executed (for Once tracking).
    fn is_hook_executed(&self, module_id: &str, hook_type: &HookType) -> bool {
        let state = self.state_manager.state();
        state.hooks_executed
            .get(module_id)
            .map(|hooks| hooks.contains(&hook_type.to_string()))
            .unwrap_or(false)
    }
}
```

### 4.3 RunHook Match Arms

```rust
// risk_level()
Self::RunHook { .. } => RiskLevel::Destructive,

// is_prunable()
// RunHook is NOT prunable -- hooks are part of the apply flow
Self::RunHook { .. } => false,  // (already handled by not matching in is_prunable)

// display()
Self::RunHook { module_id, hook_type, command, behavior } => {
    let badge = match behavior {
        HookBehavior::Ask => "[?]",
        _ => "[!]",
    };
    format!("{} Run {} hook for {}: {}", badge, hook_type, module_id,
        if command.len() > 60 { &command[..57].to_string() + "..." } else { command.clone() })
}

// record_managed_resource()
Self::RunHook { .. } => Ok(()),  // Hooks don't create managed resources

// summary() -- add hook count
let hooks = self.actions.iter()
    .filter(|a| matches!(a, ApplyAction::RunHook { .. }))
    .count();
if hooks > 0 { parts.push(format!("{} hook", hooks)); }

// execute_action()
ApplyAction::RunHook { module_id, hook_type, command, behavior } => {
    // Check Ask behavior
    if *behavior == HookBehavior::Ask && !self.interactive {
        log::warn!("Skipping {} hook for {} (ask behavior, non-interactive mode)",
            hook_type, module_id);
        return Ok(());
    }

    // Execute the hook
    match run_hook(&self.iron_root, module_id, command, self.hook_timeout) {
        Ok(output) => {
            if !output.stdout.is_empty() {
                log::info!("[hook:{}:{}] {}", module_id, hook_type, output.stdout.trim());
            }
            // Record Once hooks
            if *behavior == HookBehavior::Once {
                self.state_manager.record_hook_executed(module_id, &hook_type.to_string())?;
            }
            Ok(())
        }
        Err(e) => {
            // Hook failure is non-fatal: log error, continue
            log::error!("Hook {} for {} failed: {:?}", hook_type, module_id, e);
            Ok(())  // Non-fatal -- return Ok to continue apply
        }
    }
}

// should_execute_prune() -- RunHook is not prunable, no change needed
```

### 4.4 HistoryService

File: `crates/iron-core/src/services/history.rs` (NEW)

```rust
use crate::services::state::StateManager;
use crate::state::OperationRecord;
use crate::IronResult;
use serde::Serialize;

/// Display model for history entries.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub index: usize,
    pub operation: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_secs: Option<f64>,
    pub action_count: Option<usize>,
    pub status: crate::state::OperationStatus,
    pub details: Option<String>,
}

/// Service for querying operation history.
pub trait HistoryService {
    /// List recent operations, most recent first.
    fn list(&self, limit: usize) -> IronResult<Vec<HistoryEntry>>;

    /// Get a specific operation by 1-based index.
    fn show(&self, index: usize) -> IronResult<Option<HistoryEntry>>;

    /// Get the most recent operation.
    fn last(&self) -> IronResult<Option<HistoryEntry>>;
}

/// Default implementation reading from StateManager.
pub struct DefaultHistoryService {
    state_manager: StateManager,
}

impl DefaultHistoryService {
    pub fn new(state_manager: StateManager) -> Self {
        Self { state_manager }
    }

    /// Convert OperationRecord to HistoryEntry with 1-based indexing.
    fn to_entry(record: &OperationRecord, index: usize) -> HistoryEntry {
        HistoryEntry {
            index,
            operation: record.operation.clone(),
            timestamp: record.timestamp,
            duration_secs: record.duration_secs,
            action_count: record.action_count,
            status: record.status.clone(),
            details: record.details.clone(),
        }
    }
}

impl HistoryService for DefaultHistoryService {
    fn list(&self, limit: usize) -> IronResult<Vec<HistoryEntry>> {
        let state = self.state_manager.state();
        let ops = &state.last_operations;
        // Reverse chronological (most recent first)
        let entries: Vec<HistoryEntry> = ops.iter()
            .rev()
            .take(limit)
            .enumerate()
            .map(|(i, r)| Self::to_entry(r, i + 1))
            .collect();
        Ok(entries)
    }

    fn show(&self, index: usize) -> IronResult<Option<HistoryEntry>> {
        let state = self.state_manager.state();
        let ops = &state.last_operations;
        if index == 0 || index > ops.len() {
            return Ok(None);
        }
        // index 1 = most recent = last element
        let record_index = ops.len() - index;
        Ok(Some(Self::to_entry(&ops[record_index], index)))
    }

    fn last(&self) -> IronResult<Option<HistoryEntry>> {
        self.show(1)
    }
}
```

### 4.5 StateManager Hook Tracking Methods

File: `crates/iron-core/src/services/state.rs`

```rust
impl StateManager {
    /// Record that a hook has been executed for a module.
    pub fn record_hook_executed(&self, module_id: &str, hook_type: &str) -> IronResult<()> {
        let module_id = module_id.to_string();
        let hook_type = hook_type.to_string();
        self.with_locked_state(|state| {
            let hooks = state.hooks_executed.entry(module_id).or_default();
            if !hooks.contains(&hook_type) {
                hooks.push(hook_type);
            }
        })
    }

    /// Check if a hook has been executed for a module.
    pub fn is_hook_executed(&self, module_id: &str, hook_type: &str) -> bool {
        let state = self.state();
        state.hooks_executed
            .get(module_id)
            .map(|hooks| hooks.iter().any(|h| h == hook_type))
            .unwrap_or(false)
    }

    /// Clear all hook tracking for a module (called on module disable).
    pub fn clear_hooks_for_module(&self, module_id: &str) -> IronResult<()> {
        let module_id = module_id.to_string();
        self.with_locked_state(|state| {
            state.hooks_executed.remove(&module_id);
        })
    }
}
```

### 4.6 CLI History Command

File: `crates/iron-cli/src/commands/history.rs` (NEW)

```rust
/// Execute `iron history` command.
///
/// Subcommands:
///   iron history              -- list recent operations (default: 20)
///   iron history list         -- same as above
///   iron history show <id>    -- show details for operation #id
///   iron history last         -- show most recent operation
///
/// Flags:
///   --limit <n>  -- max entries to show (default 20)
///   --json       -- output in envelope format
pub fn execute(ctx: &CliContext, action: &HistoryAction) -> IronResult<()>;
```

File: `crates/iron-cli/src/cli.rs`

```rust
/// View operation history
History {
    #[command(subcommand)]
    action: Option<HistoryAction>,

    /// Maximum number of entries to show
    #[arg(short, long, default_value = "20")]
    limit: usize,
},

#[derive(Debug, Subcommand)]
pub enum HistoryAction {
    /// List recent operations
    List,
    /// Show details for a specific operation
    Show {
        /// Operation number (1 = most recent)
        id: usize,
    },
    /// Show the most recent operation
    Last,
}
```

---

## 5. dotfiles_sync Design

### 5.1 Discovery Location

Discovery happens in `resolve_desired_state()` (file: `crates/iron-core/src/services/apply.rs`, lines 137-145), immediately after the existing module dotfile collection loop.

### 5.2 Discovery Algorithm

```rust
// In resolve_desired_state(), inside the "Step 6" loop:
for module_id in &resolved_modules {
    let module_dir = modules_dir.join(module_id);
    if let Ok(module) = Module::load(&module_dir) {
        all_packages.extend(module.packages.clone());
        all_aur.extend(module.aur_packages.clone());

        // Existing: collect explicit dotfiles
        let explicit_dotfiles = module.dotfiles.clone();

        // NEW: dotfiles_sync auto-discovery
        if module.dotfiles_sync {
            let dotfiles_dir = module_dir.join("dotfiles");
            if dotfiles_dir.is_dir() {
                let default_target = module.dotfiles_sync_target
                    .clone()
                    .unwrap_or_else(|| format!("~/.config/{}/", module_id));

                // Warn if module ID has hyphens and using default target
                if module.dotfiles_sync_target.is_none() && module_id.contains('-') {
                    log::warn!(
                        "Module '{}' uses dotfiles_sync with default target '{}'. \
                         Consider setting dotfiles_sync_target explicitly if the \
                         config directory differs from the module ID.",
                        module_id, default_target
                    );
                }

                let discovered = discover_dotfiles(
                    &dotfiles_dir,
                    &default_target,
                    module_id,
                );

                // Merge: explicit entries override discovered for same target
                let explicit_targets: HashSet<String> = explicit_dotfiles
                    .iter()
                    .map(|d| d.target.clone())
                    .collect();

                for mapping in discovered {
                    if !explicit_targets.contains(&mapping.target) {
                        state.dotfiles.push(mapping);
                    }
                }
            }
        }

        // Add explicit dotfiles (always)
        state.dotfiles.extend(explicit_dotfiles);
    }
}
```

### 5.3 Discovery Function

```rust
/// Recursively discover files in a module's dotfiles/ directory
/// and produce DotfileMapping entries.
///
/// Directory structure is preserved:
///   dotfiles/init.lua       -> <target>/init.lua
///   dotfiles/lua/plugins.lua -> <target>/lua/plugins.lua
///
/// Files starting with '.' are included. Directories are traversed, not mapped.
fn discover_dotfiles(
    dotfiles_dir: &Path,
    target_base: &str,
    module_id: &str,
) -> Vec<DotfileMapping> {
    let mut mappings = Vec::new();
    discover_dotfiles_recursive(dotfiles_dir, dotfiles_dir, target_base, &mut mappings);
    mappings
}

fn discover_dotfiles_recursive(
    base_dir: &Path,
    current_dir: &Path,
    target_base: &str,
    mappings: &mut Vec<DotfileMapping>,
) {
    let Ok(entries) = std::fs::read_dir(current_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_dotfiles_recursive(base_dir, &path, target_base, mappings);
        } else if path.is_file() {
            // Compute relative path from dotfiles/ dir
            let relative = path.strip_prefix(base_dir).unwrap_or(&path);
            let source = format!("dotfiles/{}", relative.to_string_lossy());

            // Normalize target_base: ensure trailing /
            let target_base_normalized = if target_base.ends_with('/') {
                target_base.to_string()
            } else {
                format!("{}/", target_base)
            };
            let target = format!("{}{}", target_base_normalized, relative.to_string_lossy());

            mappings.push(DotfileMapping {
                source,
                target,
                link: true, // Default to symlink; template detection happens in compute_plan()
            });
        }
    }
}
```

### 5.4 Template Detection on Auto-Discovered Files

Auto-discovered files go through the same template detection logic in `compute_plan()` as explicit dotfiles. The existing decision tree (AMB-4 from Sprint 3.2) applies: if a discovered file contains `{{variable}}` patterns, it becomes `RenderAndCopy` instead of `CreateSymlink`. No special handling needed -- the existing `compute_plan()` code handles this uniformly.

---

## 6. Security Considerations

### 6.1 Hook Command Execution Safety

**Timeout**: All hooks have a 60-second default timeout. This prevents indefinite hangs from hooks that wait for user input, network requests, or deadlocked processes. The timeout is configurable per-service instance but not per-module (to keep the model simple). If a module needs a longer timeout, the hook should use its own timeout mechanism internally.

**No Sandboxing**: Hooks run as the same user as Iron. They have full access to the filesystem, network, and user environment. This is by design -- hooks like `systemctl --user enable ...` or `gpasswd -a $USER video` need system access. Sandboxing would break most practical hooks. The `requires_root` field on Module is informational only (for display warnings).

**User Consent Model**:
- `Always`: Runs without prompt. Used for safe, idempotent hooks.
- `Once`: Runs once, then tracked. Used for one-time setup (adding user to groups).
- `Ask`: Requires explicit user confirmation in interactive mode. Skipped silently in non-interactive mode. Used for hooks with significant side effects.
- `Skip`: Never runs. Used to disable hooks without removing them from config.

**Visibility**: `iron plan` and `iron apply --dry-run` always show hooks that would run, including their full command strings. Users can review what will execute before confirming.

**Exit Code Handling**: Hook failure is non-fatal by default. A failed hook logs an error with its stderr output but does not abort the apply. This prevents a single module's broken hook from blocking the entire system convergence. The apply result status is set to `Partial` if any hook fails.

**No Shell Injection Risk**: Hook commands come from module.toml files, which are either authored by the user or obtained from a trusted source (Git repository). They are not constructed from user input at runtime. The `sh -c` invocation treats the entire command string as a single shell expression, which is the intended behavior.

### 6.2 dotfiles_sync Safety

**No Binary Detection**: The discovery function does not attempt to detect or exclude binary files. All files in `dotfiles/` are included. This is intentional -- some config files are binary (e.g., compiled themes, font caches). The symlink deployment mode is safe for binary files. Template detection only activates on files containing `{{...}}` patterns, which binary files almost never contain.

**No Recursive Symlink Following**: The discovery function uses `std::fs::read_dir` which does not follow symlinks into directories. Symlinked files within `dotfiles/` are included as regular entries.

---

## 7. Updated Module Fields and Test Helpers

### TestModule Updates

File: `crates/iron-core/src/test_helpers.rs`

```rust
pub struct TestModule {
    // ...existing fields...
    pub hook_behavior: HookBehavior,   // NEW
    pub dotfiles_sync: bool,           // NEW
    pub dotfiles_sync_target: Option<String>,  // NEW
}

impl TestModule {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            // ...existing defaults...
            hook_behavior: HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
        }
    }

    pub fn with_hook_behavior(mut self, behavior: HookBehavior) -> Self {
        self.hook_behavior = behavior;
        self
    }

    pub fn with_dotfiles_sync(mut self, sync: bool) -> Self {
        self.dotfiles_sync = sync;
        self
    }

    pub fn with_dotfiles_sync_target(mut self, target: impl Into<String>) -> Self {
        self.dotfiles_sync_target = Some(target.into());
        self
    }

    pub fn to_module(&self) -> Module {
        Module {
            // ...existing fields...
            hook_behavior: self.hook_behavior,
            dotfiles_sync: self.dotfiles_sync,
            dotfiles_sync_target: self.dotfiles_sync_target.clone(),
        }
    }
}
```

### create_test_module() in module.rs

Add the three new fields:

```rust
fn create_test_module() -> Module {
    Module {
        // ...existing fields...
        hook_behavior: HookBehavior::default(),
        dotfiles_sync: false,
        dotfiles_sync_target: None,
    }
}
```

Also update the `test_target_paths_empty` test which constructs a Module directly.

---

## 8. Migration Path

### Backward Compatibility

All new fields use `#[serde(default)]`:
- `Module.hook_behavior` defaults to `HookBehavior::Always`
- `Module.dotfiles_sync` defaults to `false`
- `Module.dotfiles_sync_target` defaults to `None`
- `IronState.hooks_executed` defaults to `HashMap::new()`
- `OperationRecord.duration_secs` defaults to `None`
- `OperationRecord.action_count` defaults to `None`

Existing module.toml files work without changes. Existing state.json files deserialize correctly. No migration step is needed.

### No Breaking Changes

- `ApplyService` trait is unchanged.
- `compute_plan()` is private -- restructuring the action assembly is internal.
- `resolve_desired_state()` is a free function. Adding dotfiles_sync discovery extends its output but does not change its signature.
- The `HistoryService` is entirely new (no existing code modified).

---

## 9. Boundaries

- **In scope**: Hook execution mechanism, hook ordering, hook tracking, history service, dotfiles_sync auto-discovery, iron status managed counts, temporal comment cleanup, formatting.
- **Out of scope**: `iron config` namespace (F3-017 STRETCH -- defer to Phase 4 per analyst recommendation), hook sandboxing, per-hook timeout configuration, history data export, TUI history view.
- **Extension points**:
  - `HookBehavior` enum can be extended with new variants (e.g., `Notify` for hooks that run but alert the user).
  - `run_hook()` can be upgraded to a `HookExecutor` trait if different execution strategies are needed (e.g., container isolation).
  - `HistoryService` trait allows alternative implementations (e.g., reading from a persistent database instead of state.json).
  - `dotfiles_sync` discovery can be extended with glob exclusion patterns via a future `dotfiles_sync_exclude` field.

---

## 10. Files Summary

### New Files

| File | Responsibility |
|------|---------------|
| `crates/iron-core/src/services/history.rs` | `HistoryService` trait + `DefaultHistoryService` + `HistoryEntry` |
| `crates/iron-cli/src/commands/history.rs` | `iron history list/show/last` CLI command |

### Modified Files

| File | Changes |
|------|---------|
| `crates/iron-core/src/module.rs` | Add `HookBehavior` enum, `HookType` enum, `hook_behavior`/`dotfiles_sync`/`dotfiles_sync_target` fields to `Module` |
| `crates/iron-core/src/state.rs` | Add `hooks_executed` to `IronState`, add `duration_secs`/`action_count` to `OperationRecord` |
| `crates/iron-core/src/services/apply.rs` | Add `RunHook` variant, restructure `compute_plan()` into phased vectors, add `run_hook()`, add `force_hooks`/`interactive`/`hook_timeout` to `DefaultApplyService`, add dotfiles_sync discovery to `resolve_desired_state()` |
| `crates/iron-core/src/services/state.rs` | Add `record_hook_executed()`, `is_hook_executed()`, `clear_hooks_for_module()` methods |
| `crates/iron-core/src/services/mod.rs` | Add `pub mod history;` and re-exports |
| `crates/iron-core/src/test_helpers.rs` | Add `hook_behavior`, `dotfiles_sync`, `dotfiles_sync_target` to `TestModule` |
| `crates/iron-cli/src/cli.rs` | Add `Commands::History` variant with `HistoryAction` subcommand, add `--force-hooks` to Apply |
| `crates/iron-cli/src/commands/mod.rs` | Add `pub mod history;` |
| `crates/iron-cli/src/main.rs` | Wire `Commands::History` dispatch, pass `force_hooks` through Apply |
| `crates/iron-cli/src/commands/apply.rs` | Pass `force_hooks` and `interactive` to `DefaultApplyService` |
| `crates/iron-cli/src/commands/status.rs` | Wire managed counts + `last_apply` into display (SHOULD-1) |
| `crates/iron-tui/src/ui/apply.rs` | Render `RunHook` actions in plan display (uses existing `display()` method) |
