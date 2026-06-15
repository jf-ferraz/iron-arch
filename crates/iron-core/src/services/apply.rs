//! Apply Service — Converge system to declared state
//!
//! F1-003: DesiredState resolution (host.toml → bundle → profile → modules)
//! F1-005: ApplyService compares desired vs actual, produces ApplyPlan
//! F1-006: ApplyPlan struct with action list
//! F1-009: Selective module apply

use crate::actual_state::{ActualState, ManagedFileSpec, ManagedServiceSpec};
use crate::bundle::Bundle;
use crate::module::{DotfileMapping, HookBehavior, HookType, Module};
use crate::packages::PackageManager;
use crate::profile::Profile;
use crate::services::state::StateManager;
use crate::system_service::SystemService;
use crate::validation::expand_home;
use crate::{Host, IronResult, StateError};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// ==========================================================================
// F1-003: DesiredState — the fully resolved target for a host
// ==========================================================================

/// The fully resolved desired state for a host.
/// Computed from: host.toml → bundle → profile → modules → packages/dotfiles/services.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DesiredState {
    /// Declared bundle ID
    pub bundle: Option<String>,
    /// Declared profile ID
    pub profile: Option<String>,
    /// All resolved module IDs (from profile + extra_modules, deduplicated)
    pub modules: Vec<String>,
    /// All packages to install (from bundle + all modules, deduplicated)
    pub packages: Vec<String>,
    /// All AUR packages to install
    pub aur_packages: Vec<String>,
    /// All dotfile mappings (from all modules)
    pub dotfiles: Vec<DotfileMapping>,
    /// All systemd services to enable (from bundle)
    pub services: Vec<String>,
    /// Template variables (host variables merged with built-ins)
    pub variables: HashMap<String, String>,
}

/// Resolve the fully desired state by loading all referenced configs from disk.
///
/// Resolution algorithm:
/// 1. Read host.bundle → load Bundle → collect packages, services
/// 2. Read host.profile → load Profile → resolve module list (+ extends chain)
/// 3. Append host.extra_modules to module list
/// 4. Deduplicate modules, resolve dependencies
/// 5. Check conflicts → error if found
/// 6. For each module: collect packages, aur_packages, dotfiles
/// 7. Merge host.variables with built-in variables
pub fn resolve_desired_state(iron_root: &Path, host: &Host) -> IronResult<DesiredState> {
    let mut state = DesiredState {
        bundle: host.bundle.clone(),
        profile: host.profile.clone(),
        variables: builtin_variables(iron_root),
        ..Default::default()
    };

    // Host variables override built-ins
    state.variables.extend(host.variables.clone());

    let mut all_packages: Vec<String> = Vec::new();
    let mut all_aur: Vec<String> = Vec::new();
    let mut module_ids: Vec<String> = Vec::new();

    // Step 1: Bundle packages + services
    if let Some(ref bundle_id) = host.bundle {
        let bundle_dir = iron_root.join("bundles").join(bundle_id);
        if bundle_dir.join("bundle.toml").exists()
            && let Ok(bundle) = Bundle::load(&bundle_dir)
        {
            all_packages.extend(bundle.packages.clone());
            all_aur.extend(bundle.aur_packages.clone());
            state.services = bundle.services.clone();
        }
    }

    // Step 2: Profile → module list (with extends inheritance)
    if let Some(ref profile_id) = host.profile {
        let profiles_dir = iron_root.join("profiles");
        if let Ok(profile) = load_profile_with_inheritance(&profiles_dir, profile_id) {
            module_ids.extend(profile);
        }
    }

    // Step 3: Extra modules from host
    module_ids.extend(host.extra_modules.clone());

    // Step 4: Deduplicate modules
    let mut seen = HashSet::new();
    module_ids.retain(|id| seen.insert(id.clone()));

    // Step 5: Resolve module dependencies and check conflicts
    let modules_dir = iron_root.join("modules");
    let mut resolved_modules: Vec<String> = Vec::new();
    let mut _all_conflict_ids: HashSet<String> = HashSet::new();

    for module_id in &module_ids {
        resolve_module_deps(
            &modules_dir,
            module_id,
            &mut resolved_modules,
            &mut HashSet::new(),
        );
    }

    // Deduplicate resolved (deps may repeat)
    let mut seen2 = HashSet::new();
    resolved_modules.retain(|id| seen2.insert(id.clone()));

    // Check conflicts
    for module_id in &resolved_modules {
        let module_dir = modules_dir.join(module_id);
        if let Ok(module) = Module::load(&module_dir) {
            for conflict in &module.conflicts {
                if resolved_modules.contains(conflict) {
                    return Err(crate::IronError::Validation(
                        crate::ValidationError::ModuleConflict {
                            module_a: module_id.clone(),
                            module_b: conflict.clone(),
                        },
                    ));
                }
                _all_conflict_ids.insert(conflict.clone());
            }
        }
    }

    // Step 6: Collect packages, dotfiles from each module
    for module_id in &resolved_modules {
        let module_dir = modules_dir.join(module_id);
        if let Ok(module) = Module::load(&module_dir) {
            all_packages.extend(module.packages.clone());
            all_aur.extend(module.aur_packages.clone());

            // Collect explicit dotfiles
            let explicit_dotfiles = module.dotfiles.clone();

            // F3-018: dotfiles_sync auto-discovery
            if module.dotfiles_sync {
                let dotfiles_dir = module_dir.join("dotfiles");
                if dotfiles_dir.is_dir() {
                    let default_target = module
                        .dotfiles_sync_target
                        .clone()
                        .unwrap_or_else(|| format!("~/.config/{}/", module_id));

                    // Warn if module ID has hyphens and using
                    // default target
                    if module.dotfiles_sync_target.is_none() && module_id.contains('-') {
                        tracing::warn!(
                            module_id = %module_id,
                            target = %default_target,
                            "Module uses dotfiles_sync with \
                             default target. Consider setting \
                             dotfiles_sync_target explicitly if \
                             the config directory differs from \
                             the module ID."
                        );
                    }

                    let discovered = discover_dotfiles(&dotfiles_dir, &default_target);

                    // Merge: explicit entries override discovered
                    // for same target
                    let explicit_targets: HashSet<String> =
                        explicit_dotfiles.iter().map(|d| d.target.clone()).collect();

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

    // Deduplicate packages
    let mut pkg_seen = HashSet::new();
    all_packages.retain(|p| pkg_seen.insert(p.clone()));
    let mut aur_seen = HashSet::new();
    all_aur.retain(|p| aur_seen.insert(p.clone()));

    state.modules = resolved_modules;
    state.packages = all_packages;
    state.aur_packages = all_aur;

    Ok(state)
}

/// Load a profile's module list, resolving the `extends` inheritance chain.
fn load_profile_with_inheritance(
    profiles_dir: &Path,
    profile_id: &str,
) -> anyhow::Result<Vec<String>> {
    let mut modules = Vec::new();
    let mut visited = HashSet::new();
    let mut current_id = Some(profile_id.to_string());

    while let Some(id) = current_id {
        if !visited.insert(id.clone()) {
            break; // Cycle detected
        }
        let profile_dir = profiles_dir.join(&id);
        let profile = Profile::load(&profile_dir)?;
        // Parent modules come first (base), then child modules override
        let mut parent_modules = profile.modules.clone();
        parent_modules.extend(modules);
        modules = parent_modules;
        current_id = profile.extends.clone();
    }

    // Deduplicate preserving order
    let mut seen = HashSet::new();
    modules.retain(|m| seen.insert(m.clone()));
    Ok(modules)
}

/// Recursively resolve module dependencies.
fn resolve_module_deps(
    modules_dir: &Path,
    module_id: &str,
    resolved: &mut Vec<String>,
    visiting: &mut HashSet<String>,
) {
    if resolved.contains(&module_id.to_string()) || !visiting.insert(module_id.to_string()) {
        return; // Already resolved or cycle
    }

    let module_dir = modules_dir.join(module_id);
    if let Ok(module) = Module::load(&module_dir) {
        // Resolve dependencies first
        for dep in &module.depends {
            resolve_module_deps(modules_dir, dep, resolved, visiting);
        }
    }

    resolved.push(module_id.to_string());
    visiting.remove(module_id);
}

/// Recursively discover files in a module's dotfiles/ directory
/// and produce DotfileMapping entries.
///
/// Directory structure is preserved:
///   dotfiles/init.lua       -> <target>/init.lua
///   dotfiles/lua/plugins.lua -> <target>/lua/plugins.lua
///
/// Files starting with '.' are included. Directories are traversed,
/// not mapped.
fn discover_dotfiles(dotfiles_dir: &Path, target_base: &str) -> Vec<DotfileMapping> {
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

    // Sort entries for deterministic output
    let mut sorted: Vec<_> = entries.flatten().collect();
    sorted.sort_by_key(|e| e.file_name());

    for entry in sorted {
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
                link: true,
            });
        }
    }
}

/// Built-in template variables available to all hosts.
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

// ==========================================================================
// F1-006: ApplyPlan — action list
// ==========================================================================

/// A plan of actions to converge the system to desired state.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyPlan {
    /// Ordered list of actions to execute
    pub actions: Vec<ApplyAction>,
}

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

    /// Render a template file and deploy as a copy (F3-008)
    RenderAndCopy {
        source: String,
        target: String,
        variables: HashMap<String, String>,
        module_id: String,
    },
    /// Copy a file without template rendering (F3-009)
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
}

/// Risk classification for apply actions.
///
/// Distinct from `packages::RiskLevel` which classifies update risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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

impl ApplyAction {
    /// Risk classification for this action.
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Self::InstallPackages { .. }
            | Self::InstallAurPackages { .. }
            | Self::CreateSymlink { .. }
            | Self::EnableService { .. }
            | Self::ActivateModule { .. } => RiskLevel::Additive,
            Self::CopyFile {
                backup_existing: false,
                ..
            } => RiskLevel::Additive,
            Self::CopyFile {
                backup_existing: true,
                ..
            }
            | Self::RenderAndCopy { .. }
            | Self::RemoveSymlink { .. }
            | Self::DisableService { .. }
            | Self::DeactivateModule { .. } => RiskLevel::Destructive,
            Self::RemovePackages { .. } => RiskLevel::Critical,
            Self::RunHook { .. } => RiskLevel::Destructive,
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
                format!(
                    "[+] Install {} package(s): {}",
                    packages.len(),
                    packages.join(", ")
                )
            }
            Self::InstallAurPackages { packages } => {
                format!(
                    "[+] Install {} AUR package(s): {}",
                    packages.len(),
                    packages.join(", ")
                )
            }
            Self::CreateSymlink {
                source,
                target,
                module_id,
            } => {
                format!("[+] Link {} -> {} ({})", target, source, module_id)
            }
            Self::EnableService { name } => {
                format!("[+] Enable service: {}", name)
            }
            Self::ActivateModule { id } => {
                format!("[+] Activate module: {}", id)
            }
            Self::RenderAndCopy {
                target, module_id, ..
            } => {
                format!("[!] Render template -> {} ({})", target, module_id)
            }
            Self::CopyFile {
                target, module_id, ..
            } => {
                format!("[+] Copy file -> {} ({})", target, module_id)
            }
            Self::RemovePackages { packages } => {
                format!(
                    "[!!] Remove {} package(s): {}",
                    packages.len(),
                    packages.join(", ")
                )
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
            Self::RunHook {
                module_id,
                hook_type,
                command,
                behavior,
            } => {
                let badge = match behavior {
                    HookBehavior::Ask => "[?]",
                    _ => "[!]",
                };
                let cmd_display = if command.len() > 60 {
                    format!("{}...", &command[..57])
                } else {
                    command.clone()
                };
                format!(
                    "{} Run {} hook for {}: {}",
                    badge, hook_type, module_id, cmd_display
                )
            }
        }
    }
}

impl ApplyPlan {
    /// Check if there is nothing to do
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Total number of actions
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Maximum risk level across all actions.
    /// Returns ReadOnly for an empty plan.
    pub fn max_risk(&self) -> RiskLevel {
        self.actions
            .iter()
            .map(|a| a.risk_level())
            .max()
            .unwrap_or(RiskLevel::ReadOnly)
    }

    /// Count of prunable (removal) actions in the plan.
    pub fn prune_count(&self) -> usize {
        self.actions.iter().filter(|a| a.is_prunable()).count()
    }

    /// Count of actions per risk level.
    pub fn risk_summary(&self) -> HashMap<RiskLevel, usize> {
        let mut counts: HashMap<RiskLevel, usize> = HashMap::new();
        for action in &self.actions {
            *counts.entry(action.risk_level()).or_insert(0) += 1;
        }
        counts
    }

    /// Summary string for display
    pub fn summary(&self) -> String {
        let pkgs_install: usize = self
            .actions
            .iter()
            .filter_map(|a| match a {
                ApplyAction::InstallPackages { packages } => Some(packages.len()),
                ApplyAction::InstallAurPackages { packages } => Some(packages.len()),
                _ => None,
            })
            .sum();
        let pkgs_remove: usize = self
            .actions
            .iter()
            .filter_map(|a| match a {
                ApplyAction::RemovePackages { packages } => Some(packages.len()),
                _ => None,
            })
            .sum();
        let links = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::CreateSymlink { .. }))
            .count();
        let copies = self
            .actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    ApplyAction::CopyFile { .. } | ApplyAction::RenderAndCopy { .. }
                )
            })
            .count();
        let removes = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::RemoveSymlink { .. }))
            .count();
        let svcs_enable = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::EnableService { .. }))
            .count();
        let svcs_disable = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::DisableService { .. }))
            .count();
        let mods_activate = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::ActivateModule { .. }))
            .count();
        let mods_deactivate = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::DeactivateModule { .. }))
            .count();

        let mut parts = Vec::new();
        if pkgs_install > 0 {
            parts.push(format!("+{} pkg", pkgs_install));
        }
        if pkgs_remove > 0 {
            parts.push(format!("-{} pkg", pkgs_remove));
        }
        if links > 0 {
            parts.push(format!("+{} link", links));
        }
        if copies > 0 {
            parts.push(format!("+{} copy", copies));
        }
        if removes > 0 {
            parts.push(format!("-{} file", removes));
        }
        if svcs_enable > 0 {
            parts.push(format!("+{} svc", svcs_enable));
        }
        if svcs_disable > 0 {
            parts.push(format!("-{} svc", svcs_disable));
        }
        if mods_activate > 0 {
            parts.push(format!("+{} mod", mods_activate));
        }
        if mods_deactivate > 0 {
            parts.push(format!("-{} mod", mods_deactivate));
        }
        let hooks = self
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::RunHook { .. }))
            .count();
        if hooks > 0 {
            parts.push(format!("{} hook", hooks));
        }

        if parts.is_empty() {
            "No changes".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Result of executing an apply plan.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyResult {
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub duration_secs: f64,
}

// ==========================================================================
// F1-005: ApplyService trait + implementation
// ==========================================================================

/// Service for converging system to desired state.
pub trait ApplyService {
    /// Compute a plan comparing desired (host.toml) vs actual (system)
    fn plan(&self, host_id: &str) -> IronResult<ApplyPlan>;

    /// Compute plan for a single module only (F1-009)
    fn plan_module(&self, module_id: &str) -> IronResult<ApplyPlan>;

    /// Execute an apply plan
    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult>;

    /// F2-015: Validate config before apply. Returns a list of warnings.
    fn validate(&self, host_id: &str) -> IronResult<Vec<ValidationWarning>>;
}

/// A validation warning or error found during pre-apply checks.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationWarning {
    pub severity: ValidationSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Severity of a validation warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ValidationSeverity {
    /// Informational only
    Info,
    /// May cause issues
    Warning,
    /// Will cause apply failure
    Error,
}

/// Policy controlling whether removal actions are executed during apply.
///
/// Removal actions (RemovePackages, DisableService, RemoveSymlink,
/// DeactivateModule) are always included in the plan for visibility.
/// This policy controls whether they are actually executed.
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

/// Default hook timeout in seconds.
pub const DEFAULT_HOOK_TIMEOUT: u64 = 60;

/// Default implementation.
pub struct DefaultApplyService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Arc<dyn PackageManager>,
    service_manager: Arc<dyn SystemService>,
    prune_policy: PrunePolicy,
    /// Re-run hooks even if already executed (overrides Once behavior)
    force_hooks: bool,
    /// Whether the session is interactive (affects Ask behavior)
    interactive: bool,
    /// Timeout for hook execution in seconds
    hook_timeout: u64,
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
            prune_policy: PrunePolicy::default(),
            force_hooks: false,
            interactive: true,
            hook_timeout: DEFAULT_HOOK_TIMEOUT,
        }
    }

    /// Set the prune policy for removal actions.
    pub fn with_prune_policy(mut self, policy: PrunePolicy) -> Self {
        self.prune_policy = policy;
        self
    }

    /// Force re-execution of hooks (overrides Once behavior).
    pub fn with_force_hooks(mut self, force: bool) -> Self {
        self.force_hooks = force;
        self
    }

    /// Set whether the session is interactive (affects Ask hooks).
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Set the hook execution timeout in seconds.
    pub fn with_hook_timeout(mut self, timeout: u64) -> Self {
        self.hook_timeout = timeout;
        self
    }
}

impl ApplyService for DefaultApplyService {
    fn plan(&self, host_id: &str) -> IronResult<ApplyPlan> {
        // Load host
        let host_svc = crate::services::host::DefaultHostService::new(&self.iron_root);
        use crate::services::host::HostService;
        let host = host_svc.load_host(host_id)?;

        // If no bundle and no profile declared, nothing to apply
        if host.bundle.is_none() && host.profile.is_none() && host.extra_modules.is_empty() {
            return Ok(ApplyPlan::default());
        }

        // Resolve desired state
        let desired = resolve_desired_state(&self.iron_root, &host)?;

        // F3-002b: Scan actual state once, pass to compute_plan
        let actual = self.scan_actual_state(&desired)?;

        // F3-021: Bootstrap managed tracking on first use
        self.bootstrap_managed_tracking(&desired, &actual);

        self.compute_plan(&desired, &actual)
    }

    fn plan_module(&self, module_id: &str) -> IronResult<ApplyPlan> {
        let modules_dir = self.iron_root.join("modules").join(module_id);
        let _module = Module::load(&modules_dir).map_err(|_e| StateError::ModuleNotFound {
            id: module_id.to_string(),
        })?;

        // Build a mini desired-state for just this module (+ deps)
        let mut resolved = Vec::new();
        resolve_module_deps(
            &self.iron_root.join("modules"),
            module_id,
            &mut resolved,
            &mut HashSet::new(),
        );

        let mut all_packages = Vec::new();
        let mut all_aur = Vec::new();
        let mut all_dotfiles = Vec::new();

        for mid in &resolved {
            let mdir = self.iron_root.join("modules").join(mid);
            if let Ok(m) = Module::load(&mdir) {
                all_packages.extend(m.packages);
                all_aur.extend(m.aur_packages);
                all_dotfiles.extend(m.dotfiles);
            }
        }

        let desired = DesiredState {
            modules: resolved,
            packages: all_packages,
            aur_packages: all_aur,
            dotfiles: all_dotfiles,
            ..Default::default()
        };

        // F3-002b: Scan actual state once, pass to compute_plan
        let actual = self.scan_actual_state(&desired)?;

        self.compute_plan(&desired, &actual)
    }

    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult> {
        // F2-008: Auto-snapshot before destructive operations
        if !plan.is_empty() {
            let snapshot_svc = crate::services::snapshot_service::DefaultSnapshotService::new(
                &self.iron_root,
                self.state_manager.clone(),
            )
            .with_package_manager(self.package_manager.clone());
            use crate::services::snapshot_service::SnapshotService;
            let _ = snapshot_svc.create_auto("pre-apply");
            let _ = snapshot_svc.prune_auto(crate::services::snapshot_service::DEFAULT_AUTO_KEEP);
        }

        let start = Instant::now();
        let mut result = ApplyResult::default();

        for action in &plan.actions {
            // F3-014: Skip prunable actions if prune policy disallows
            if action.is_prunable() && !self.should_execute_prune(action) {
                continue;
            }

            match self.execute_action(action) {
                Ok(()) => {
                    result.succeeded += 1;
                    // F3-021: Record managed resource after success
                    self.record_managed_resource(action);
                }
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!("{}: {}", action.display(), e));
                }
            }
        }

        // F3-021: Record last_apply timestamp
        let _ = self.state_manager.update_last_apply();

        result.duration_secs = start.elapsed().as_secs_f64();
        Ok(result)
    }

    fn validate(&self, host_id: &str) -> IronResult<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        // Check host exists
        let host_svc = crate::services::host::DefaultHostService::new(&self.iron_root);
        use crate::services::host::HostService;
        let host = match host_svc.load_host(host_id) {
            Ok(h) => h,
            Err(_) => {
                warnings.push(ValidationWarning {
                    severity: ValidationSeverity::Error,
                    message: format!("Host '{}' not found", host_id),
                    suggestion: Some("Run 'iron host list' to see available hosts.".into()),
                });
                return Ok(warnings);
            }
        };

        // Check referenced bundle exists
        if let Some(ref bundle_id) = host.bundle {
            let bundle_dir = self.iron_root.join("bundles").join(bundle_id);
            if !bundle_dir.join("bundle.toml").exists() {
                warnings.push(ValidationWarning {
                    severity: ValidationSeverity::Error,
                    message: format!("Referenced bundle '{}' not found", bundle_id),
                    suggestion: Some("Run 'iron bundle list' to see available bundles.".into()),
                });
            }
        }

        // Check referenced profile exists
        if let Some(ref profile_id) = host.profile {
            let profile_dir = self.iron_root.join("profiles").join(profile_id);
            if !profile_dir.join("profile.toml").exists() {
                warnings.push(ValidationWarning {
                    severity: ValidationSeverity::Error,
                    message: format!("Referenced profile '{}' not found", profile_id),
                    suggestion: Some("Run 'iron profile list' to see available profiles.".into()),
                });
            }
        }

        // Check extra_modules exist
        for mod_id in &host.extra_modules {
            let mod_dir = self.iron_root.join("modules").join(mod_id);
            if !mod_dir.join("module.toml").exists() {
                warnings.push(ValidationWarning {
                    severity: ValidationSeverity::Error,
                    message: format!("Referenced module '{}' not found", mod_id),
                    suggestion: Some("Run 'iron module list' to see available modules.".into()),
                });
            }
        }

        // Check for duplicate packages across modules
        let mut seen_packages: HashMap<String, String> = HashMap::new();
        let modules_dir = self.iron_root.join("modules");

        let all_module_ids: Vec<String> = host.extra_modules.clone();
        for mod_id in &all_module_ids {
            let mdir = modules_dir.join(mod_id);
            if let Ok(m) = Module::load(&mdir) {
                for pkg in &m.packages {
                    if let Some(prev_mod) = seen_packages.get(pkg) {
                        warnings.push(ValidationWarning {
                            severity: ValidationSeverity::Info,
                            message: format!(
                                "Package '{}' declared in both '{}' and '{}'",
                                pkg, prev_mod, mod_id
                            ),
                            suggestion: None,
                        });
                    } else {
                        seen_packages.insert(pkg.clone(), mod_id.clone());
                    }
                }
            }
        }

        Ok(warnings)
    }
}

impl DefaultApplyService {
    /// Build managed file/service specs from desired state and scan the system.
    fn scan_actual_state(&self, desired: &DesiredState) -> IronResult<ActualState> {
        let managed_services: Vec<ManagedServiceSpec> = desired
            .services
            .iter()
            .map(|s| ManagedServiceSpec { name: s.clone() })
            .collect();

        let managed_files: Vec<ManagedFileSpec> = desired
            .dotfiles
            .iter()
            .map(|d| ManagedFileSpec {
                target: expand_home(Path::new(&d.target))
                    .to_string_lossy()
                    .to_string(),
                expected_source: Some(d.source.clone()),
            })
            .collect();

        ActualState::scan(
            self.package_manager.as_ref(),
            self.service_manager.as_ref(),
            &managed_services,
            &managed_files,
        )
    }

    /// Compute the plan by diffing desired against actual system state.
    ///
    /// F3-002b: Accepts a pre-scanned `ActualState` instead of querying
    /// the system inline. The caller (`plan()` / `plan_module()`) is
    /// responsible for scanning once and passing the result here.
    fn compute_plan(&self, desired: &DesiredState, actual: &ActualState) -> IronResult<ApplyPlan> {
        // F3-014: Five phase vectors for correct hook ordering.
        // Final order: pre_hooks -> install_actions -> post_hooks
        //           -> removal_pre_hooks -> removal_actions
        let mut pre_hooks: Vec<ApplyAction> = Vec::new();
        let mut install_actions: Vec<ApplyAction> = Vec::new();
        let mut post_hooks: Vec<ApplyAction> = Vec::new();
        let mut removal_pre_hooks: Vec<ApplyAction> = Vec::new();
        let mut removal_actions: Vec<ApplyAction> = Vec::new();

        // Load all modules once for hook planning
        let modules_dir = self.iron_root.join("modules");
        let mut loaded_modules: HashMap<String, Module> = HashMap::new();
        for module_id in &desired.modules {
            let mdir = modules_dir.join(module_id);
            if let Ok(m) = Module::load(&mdir) {
                loaded_modules.insert(module_id.clone(), m);
            }
        }

        // 1. Package diff — read from ActualState instead of querying
        let missing_packages: Vec<String> = desired
            .packages
            .iter()
            .filter(|p| !actual.installed_packages.contains(*p))
            .cloned()
            .collect();
        if !missing_packages.is_empty() {
            install_actions.push(ApplyAction::InstallPackages {
                packages: missing_packages,
            });
        }

        let missing_aur: Vec<String> = desired
            .aur_packages
            .iter()
            .filter(|p| !actual.installed_packages.contains(*p))
            .cloned()
            .collect();
        if !missing_aur.is_empty() {
            install_actions.push(ApplyAction::InstallAurPackages {
                packages: missing_aur,
            });
        }

        // 2. Dotfile diff — read from ActualState managed_files
        //    F3-008/F3-009: Template detection + CopyFile mode
        for dotfile in &desired.dotfiles {
            let target_expanded = expand_home(Path::new(&dotfile.target))
                .to_string_lossy()
                .to_string();

            // Module that owns this dotfile — resolved up front so we can build
            // the canonical source path the deployed symlink should point at.
            let module_id = desired
                .modules
                .iter()
                .find(|mid| {
                    loaded_modules
                        .get(*mid)
                        .map(|m| m.dotfiles.iter().any(|d| d.target == dotfile.target))
                        .unwrap_or(false)
                })
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());

            // Absolute path the deployed symlink is expected to point at.
            let expected_source = self
                .iron_root
                .join("modules")
                .join(&module_id)
                .join(&dotfile.source);

            // Look up this file in the actual state scan results
            let actual_file = actual
                .managed_files
                .iter()
                .find(|f| f.target == target_expanded);

            let needs_action = match actual_file {
                Some(af) => match af.file_type {
                    // Re-link unless the symlink ALREADY points at the canonical
                    // source. This previously compared only the final path
                    // component, so a symlink into an unrelated directory whose
                    // last component happened to match (e.g. ~/.config/niri ->
                    // .../arch-config/.../niri vs the iron module's config/niri)
                    // was wrongly treated as correct and never re-pointed.
                    crate::actual_state::FileStateType::Symlink => match &af.symlink_target {
                        Some(current) => Path::new(current) != expected_source.as_path(),
                        None => true,
                    },
                    crate::actual_state::FileStateType::Missing => true,
                    _ => dotfile.link,
                },
                None => {
                    let target_path = expand_home(Path::new(&dotfile.target));
                    !target_path.exists() || dotfile.link
                }
            };

            if needs_action {
                // F3-008: Template detection at plan time
                let source_content = std::fs::read_to_string(&expected_source).ok();

                let has_templates = source_content
                    .as_ref()
                    .map(|c| has_template_variables(c))
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
                        backup_existing: actual_file.map(|f| f.exists).unwrap_or(false),
                        module_id,
                    }
                } else {
                    ApplyAction::CreateSymlink {
                        source: dotfile.source.clone(),
                        target: dotfile.target.clone(),
                        module_id,
                    }
                };

                install_actions.push(action);
            }
        }

        // 3. Service diff — read from ActualState services
        for service in &desired.services {
            let is_enabled = actual
                .services
                .iter()
                .find(|s| s.name == *service)
                .map(|s| s.enabled)
                .unwrap_or(false);

            if !is_enabled {
                install_actions.push(ApplyAction::EnableService {
                    name: service.clone(),
                });
            }
        }

        // 4. Module activation diff (state tracking)
        let active_modules: HashSet<String> =
            self.state_manager.active_modules().into_iter().collect();
        for module_id in &desired.modules {
            if !active_modules.contains(module_id) {
                install_actions.push(ApplyAction::ActivateModule {
                    id: module_id.clone(),
                });
            }
        }

        // F3-014: Plan hooks for desired modules
        for module_id in &desired.modules {
            if let Some(module) = loaded_modules.get(module_id) {
                let behavior = module.hook_behavior;

                if let Some(ref cmd) = module.pre_install {
                    pre_hooks.push(ApplyAction::RunHook {
                        module_id: module_id.clone(),
                        hook_type: HookType::PreInstall,
                        command: cmd.clone(),
                        behavior,
                    });
                }
                if let Some(ref cmd) = module.post_install {
                    post_hooks.push(ApplyAction::RunHook {
                        module_id: module_id.clone(),
                        hook_type: HookType::PostInstall,
                        command: cmd.clone(),
                        behavior,
                    });
                }
            }
        }

        // ── F3-011: Package removal diff ──────────────────────
        let managed_pkgs: HashSet<String> =
            self.state_manager.managed_packages().into_iter().collect();
        let desired_pkgs: HashSet<&str> = desired.packages.iter().map(|s| s.as_str()).collect();
        let desired_aur: HashSet<&str> = desired.aur_packages.iter().map(|s| s.as_str()).collect();
        let to_remove: Vec<String> = managed_pkgs
            .iter()
            .filter(|p| actual.installed_packages.contains(*p))
            .filter(|p| !desired_pkgs.contains(p.as_str()) && !desired_aur.contains(p.as_str()))
            .cloned()
            .collect();
        if !to_remove.is_empty() {
            removal_actions.push(ApplyAction::RemovePackages {
                packages: to_remove,
            });
        }

        // ── F3-012: Service disable diff ──────────────────────
        let managed_svcs: HashSet<String> =
            self.state_manager.managed_services().into_iter().collect();
        let desired_svcs: HashSet<&str> = desired.services.iter().map(|s| s.as_str()).collect();
        for svc_name in &managed_svcs {
            let is_enabled = actual
                .services
                .iter()
                .find(|s| s.name == *svc_name)
                .map(|s| s.enabled)
                .unwrap_or(false);
            if is_enabled && !desired_svcs.contains(svc_name.as_str()) {
                removal_actions.push(ApplyAction::DisableService {
                    name: svc_name.clone(),
                });
            }
        }

        // ── F3-013: Dotfile removal diff ──────────────────────
        let managed_dots: HashSet<String> =
            self.state_manager.managed_dotfiles().into_iter().collect();
        let desired_targets: HashSet<String> = desired
            .dotfiles
            .iter()
            .map(|d| {
                expand_home(Path::new(&d.target))
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        for dot_target in &managed_dots {
            if !desired_targets.contains(dot_target) {
                removal_actions.push(ApplyAction::RemoveSymlink {
                    target: dot_target.clone(),
                });
            }
        }

        // ── Module deactivation diff ──────────────────────────
        let desired_mods: HashSet<&str> = desired.modules.iter().map(|s| s.as_str()).collect();
        for active_id in &active_modules {
            if !desired_mods.contains(active_id.as_str()) {
                removal_actions.push(ApplyAction::DeactivateModule {
                    id: active_id.clone(),
                });
            }
        }

        // F3-014: Plan pre_uninstall hooks for modules being removed
        for active_id in &active_modules {
            if !desired_mods.contains(active_id.as_str()) {
                let mdir = modules_dir.join(active_id);
                if let Ok(module) = Module::load(&mdir)
                    && let Some(ref cmd) = module.pre_uninstall
                {
                    removal_pre_hooks.push(ApplyAction::RunHook {
                        module_id: active_id.clone(),
                        hook_type: HookType::PreUninstall,
                        command: cmd.clone(),
                        behavior: module.hook_behavior,
                    });
                }
            }
        }

        // Concatenate in correct phase order
        let actions = [
            pre_hooks,
            install_actions,
            post_hooks,
            removal_pre_hooks,
            removal_actions,
        ]
        .concat();

        Ok(ApplyPlan { actions })
    }

    /// Check whether a prunable action should be executed based on policy.
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
            ApplyAction::EnableService { name } => self.state_manager.record_managed_service(name),
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
            | ApplyAction::DeactivateModule { .. }
            | ApplyAction::RunHook { .. } => Ok(()),
        };
    }

    /// Bootstrap managed tracking on first plan/apply.
    ///
    /// Seeds managed_packages with `desired intersect installed`,
    /// managed_services with `desired intersect enabled`,
    /// and managed_dotfiles with `desired intersect existing`.
    fn bootstrap_managed_tracking(&self, desired: &DesiredState, actual: &ActualState) {
        let state = self.state_manager.state();
        if !state.managed_packages.is_empty()
            || !state.managed_services.is_empty()
            || !state.managed_dotfiles.is_empty()
        {
            return; // Already tracked
        }
        drop(state);

        // Seed packages: desired intersect installed
        let seed_pkgs: Vec<String> = desired
            .packages
            .iter()
            .chain(desired.aur_packages.iter())
            .filter(|p| actual.installed_packages.contains(*p))
            .cloned()
            .collect();
        if !seed_pkgs.is_empty() {
            let _ = self.state_manager.record_managed_packages(&seed_pkgs);
        }

        // Seed services: desired intersect enabled
        for svc in &desired.services {
            let is_enabled = actual
                .services
                .iter()
                .find(|s| s.name == *svc)
                .map(|s| s.enabled)
                .unwrap_or(false);
            if is_enabled {
                let _ = self.state_manager.record_managed_service(svc);
            }
        }

        // Seed dotfiles: desired intersect existing on disk
        for dotfile in &desired.dotfiles {
            let target_expanded = expand_home(Path::new(&dotfile.target))
                .to_string_lossy()
                .to_string();
            let exists = actual
                .managed_files
                .iter()
                .find(|f| f.target == target_expanded)
                .map(|f| f.exists)
                .unwrap_or(false);
            if exists {
                let _ = self.state_manager.record_managed_dotfile(&target_expanded);
            }
        }
    }

    /// Execute a single action.
    fn execute_action(&self, action: &ApplyAction) -> IronResult<()> {
        match action {
            ApplyAction::InstallPackages { packages } => {
                self.package_manager.install(packages)?;
                Ok(())
            }
            ApplyAction::InstallAurPackages { packages } => {
                self.package_manager.install(packages)?;
                Ok(())
            }
            ApplyAction::CreateSymlink {
                source,
                target,
                module_id,
            } => {
                let target_path = expand_home(Path::new(target));

                // Find the actual source path from the module directory
                let source_path = self.iron_root.join("modules").join(module_id).join(source);

                // Create parent directory
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                // Backup existing file if not a symlink
                if target_path.exists() && !target_path.is_symlink() {
                    let backup = target_path.with_extension("iron-backup");
                    std::fs::rename(&target_path, &backup).ok();
                }

                // Remove existing symlink
                if target_path.is_symlink() {
                    std::fs::remove_file(&target_path).ok();
                }

                // Create symlink
                #[cfg(unix)]
                std::os::unix::fs::symlink(&source_path, &target_path).map_err(|e| {
                    crate::FsError::IoError {
                        message: format!(
                            "Failed to symlink {} → {}: {}",
                            source_path.display(),
                            target_path.display(),
                            e
                        ),
                    }
                })?;

                Ok(())
            }
            ApplyAction::EnableService { name } => {
                self.service_manager.enable_service(name)?;
                Ok(())
            }
            ApplyAction::ActivateModule { id } => {
                self.state_manager.enable_module(id)?;
                Ok(())
            }

            ApplyAction::RenderAndCopy {
                source,
                target,
                variables,
                module_id,
            } => {
                let target_path = expand_home(Path::new(target));
                let source_path = self.iron_root.join("modules").join(module_id).join(source);

                let content =
                    std::fs::read_to_string(&source_path).map_err(|e| crate::FsError::IoError {
                        message: format!(
                            "Failed to read template {}: {}",
                            source_path.display(),
                            e
                        ),
                    })?;

                let rendered = render_template(&content, variables);

                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                if target_path.exists() {
                    let backup = target_path.with_extension("iron-backup");
                    std::fs::rename(&target_path, &backup).ok();
                }

                std::fs::write(&target_path, rendered).map_err(|e| crate::FsError::IoError {
                    message: format!(
                        "Failed to write rendered file {}: {}",
                        target_path.display(),
                        e
                    ),
                })?;

                Ok(())
            }
            ApplyAction::CopyFile {
                source,
                target,
                module_id,
                ..
            } => {
                let target_path = expand_home(Path::new(target));
                let source_path = self.iron_root.join("modules").join(module_id).join(source);

                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                if target_path.exists() {
                    let backup = target_path.with_extension("iron-backup");
                    std::fs::rename(&target_path, &backup).ok();
                }

                std::fs::copy(&source_path, &target_path).map_err(|e| crate::FsError::IoError {
                    message: format!(
                        "Failed to copy {} -> {}: {}",
                        source_path.display(),
                        target_path.display(),
                        e
                    ),
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
            ApplyAction::RunHook {
                module_id,
                hook_type,
                command,
                behavior,
            } => {
                // Skip hooks based on behavior policy
                match behavior {
                    HookBehavior::Skip => return Ok(()),
                    HookBehavior::Ask if !self.interactive => {
                        // Non-interactive: skip with implicit warning
                        return Ok(());
                    }
                    HookBehavior::Once if !self.force_hooks => {
                        // Check if already executed
                        let ht_str = hook_type.to_string();
                        if self.state_manager.is_hook_executed(module_id, &ht_str) {
                            return Ok(());
                        }
                    }
                    _ => {}
                }

                let result = run_hook(
                    &self.iron_root,
                    module_id,
                    hook_type,
                    command,
                    self.hook_timeout,
                );

                match result {
                    Ok(_output) => {
                        // Record hook execution for Once tracking
                        let ht_str = hook_type.to_string();
                        let _ = self.state_manager.record_hook_executed(module_id, &ht_str);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}

// ==========================================================================
// F3-014: Hook execution helper
// ==========================================================================

/// Output from a successfully executed hook.
#[derive(Debug, Clone)]
pub struct HookOutput {
    /// Standard output from the hook process
    pub stdout: String,
    /// Standard error from the hook process
    pub stderr: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
}

/// Execute a module hook as a shell command.
///
/// Working directory is set to the module source directory.
/// Environment variables `IRON_ROOT` and `IRON_MODULE` are injected.
/// Enforces a timeout via `wait_with_output` + thread.
fn run_hook(
    iron_root: &Path,
    module_id: &str,
    hook_type: &HookType,
    command: &str,
    timeout_secs: u64,
) -> IronResult<HookOutput> {
    use std::process::Command;
    use std::time::Duration;

    let module_dir = iron_root.join("modules").join(module_id);

    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(&module_dir)
        .env("IRON_ROOT", iron_root.as_os_str())
        .env("IRON_MODULE", module_id)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| crate::IronError::OperationFailed {
            message: format!("spawn {} hook for {}: {}", hook_type, module_id, e),
        })?;

    // Timeout enforcement via thread + channel
    let timeout = Duration::from_secs(timeout_secs);
    let (tx, rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            let _ = handle.join();
            let code = output.status.code().unwrap_or(-1);
            if code != 0 {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::IronError::OperationFailed {
                    message: format!(
                        "{} hook for {}: exit code {}: {}",
                        hook_type,
                        module_id,
                        code,
                        stderr.trim()
                    ),
                });
            }
            Ok(HookOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: code,
            })
        }
        Ok(Err(e)) => {
            let _ = handle.join();
            Err(crate::IronError::OperationFailed {
                message: format!("{} hook for {}: {}", hook_type, module_id, e),
            })
        }
        Err(_timeout) => {
            // Thread is still running with the child process;
            // we cannot kill it portably here, but we report the
            // timeout. The thread will eventually clean up.
            Err(crate::IronError::OperationFailed {
                message: format!(
                    "{} hook for {}: timed out after {}s",
                    hook_type, module_id, timeout_secs
                ),
            })
        }
    }
}

// ==========================================================================
// F3-008: Template helpers (inline to avoid iron-core -> iron-fs dep)
// ==========================================================================

/// Check if content contains any `{{...}}` template variables.
fn has_template_variables(content: &str) -> bool {
    content.contains("{{")
}

/// Render template content by substituting `{{variable}}` placeholders.
///
/// Whitespace inside braces is trimmed. Unknown variables are left
/// unchanged.
fn render_template(content: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut var_name = String::new();
            let mut found_close = false;

            while let Some(inner) = chars.next() {
                if inner == '}' {
                    if chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        found_close = true;
                        break;
                    }
                    var_name.push(inner);
                } else {
                    var_name.push(inner);
                }
            }

            if found_close {
                let key = var_name.trim();
                if let Some(val) = vars.get(key) {
                    result.push_str(val);
                } else {
                    result.push_str("{{");
                    result.push_str(&var_name);
                    result.push_str("}}");
                }
            } else {
                result.push_str("{{");
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a test iron root with host, bundle, profile, modules
    fn create_test_root() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Host
        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::write(
            root.join("hosts/test.toml"),
            r#"
id = "test"
name = "Test Host"
installed_bundles = []
bundle = "test-bundle"
profile = "test-profile"
extra_modules = ["extra-mod"]

[variables]
terminal = "kitty"

[hardware]
"#,
        )
        .unwrap();

        // Bundle
        fs::create_dir_all(root.join("bundles/test-bundle")).unwrap();
        fs::write(
            root.join("bundles/test-bundle/bundle.toml"),
            r#"
id = "test-bundle"
name = "Test Bundle"
bundle_type = "WaylandCompositor"
packages = ["pkg-a", "pkg-b"]
aur_packages = []
profiles = []
conflicts = []
services = ["svc-a.service"]
"#,
        )
        .unwrap();

        // Profile
        fs::create_dir_all(root.join("profiles/test-profile")).unwrap();
        fs::write(
            root.join("profiles/test-profile/profile.toml"),
            r#"
id = "test-profile"
name = "Test Profile"
modules = ["mod-a", "mod-b"]
"#,
        )
        .unwrap();

        // Modules
        for (id, pkgs) in &[
            ("mod-a", r#"["neovim"]"#),
            ("mod-b", r#"["fish"]"#),
            ("extra-mod", r#"["ripgrep"]"#),
        ] {
            fs::create_dir_all(root.join(format!("modules/{}", id))).unwrap();
            fs::write(
                root.join(format!("modules/{}/module.toml", id)),
                format!(
                    r#"
id = "{id}"
name = "{id}"
kind = "AppConfig"
packages = {pkgs}
aur_packages = []
conflicts = []
depends = []

[[dotfiles]]
source = "config"
target = "~/.config/{id}"
link = true
"#
                ),
            )
            .unwrap();
        }

        // State file
        fs::write(root.join("state.json"), "{}").unwrap();

        tmp
    }

    #[test]
    fn test_resolve_desired_state_basic() {
        let tmp = create_test_root();
        let root = tmp.path();

        let host: Host =
            toml::from_str(&fs::read_to_string(root.join("hosts/test.toml")).unwrap()).unwrap();

        let desired = resolve_desired_state(root, &host).unwrap();

        assert_eq!(desired.bundle, Some("test-bundle".to_string()));
        assert_eq!(desired.profile, Some("test-profile".to_string()));
        // Modules: mod-a, mod-b (from profile) + extra-mod
        assert!(desired.modules.contains(&"mod-a".to_string()));
        assert!(desired.modules.contains(&"mod-b".to_string()));
        assert!(desired.modules.contains(&"extra-mod".to_string()));
        // Packages: from bundle (pkg-a, pkg-b) + modules (neovim, fish, ripgrep)
        assert!(desired.packages.contains(&"pkg-a".to_string()));
        assert!(desired.packages.contains(&"neovim".to_string()));
        assert!(desired.packages.contains(&"fish".to_string()));
        assert!(desired.packages.contains(&"ripgrep".to_string()));
        // Services
        assert!(desired.services.contains(&"svc-a.service".to_string()));
        // Variables
        assert_eq!(
            desired.variables.get("terminal"),
            Some(&"kitty".to_string())
        );
        // Dotfiles
        assert!(!desired.dotfiles.is_empty());
    }

    #[test]
    fn test_resolve_no_bundle_no_profile() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("modules")).unwrap();
        fs::create_dir_all(tmp.path().join("bundles")).unwrap();
        fs::create_dir_all(tmp.path().join("profiles")).unwrap();

        let host = Host {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: None,
            hardware: crate::HardwareSpec::default(),
            install_params: None,
            installed_bundles: vec![],
            active_bundle: None,
            bundle: None,
            profile: None,
            extra_modules: vec![],
            variables: HashMap::new(),
        };

        let desired = resolve_desired_state(tmp.path(), &host).unwrap();
        assert!(desired.modules.is_empty());
        assert!(desired.packages.is_empty());
        assert!(desired.dotfiles.is_empty());
    }

    #[test]
    fn test_resolve_deduplicates_packages() {
        let tmp = create_test_root();
        let root = tmp.path();

        // Make mod-a also have "fish" so it collides with mod-b
        fs::write(
            root.join("modules/mod-a/module.toml"),
            r#"
id = "mod-a"
name = "mod-a"
kind = "AppConfig"
packages = ["fish", "neovim"]
aur_packages = []
conflicts = []
depends = []
"#,
        )
        .unwrap();

        let host: Host =
            toml::from_str(&fs::read_to_string(root.join("hosts/test.toml")).unwrap()).unwrap();
        let desired = resolve_desired_state(root, &host).unwrap();

        let fish_count = desired.packages.iter().filter(|p| *p == "fish").count();
        assert_eq!(fish_count, 1, "fish should be deduplicated");
    }

    #[test]
    fn test_apply_plan_summary() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into(), "b".into()],
                },
                ApplyAction::CreateSymlink {
                    source: "s".into(),
                    target: "t".into(),
                    module_id: "m".into(),
                },
                ApplyAction::EnableService { name: "svc".into() },
                ApplyAction::ActivateModule { id: "mod".into() },
            ],
        };

        assert_eq!(plan.action_count(), 4);
        assert!(!plan.is_empty());
        let summary = plan.summary();
        assert!(summary.contains("+2 pkg"));
        assert!(summary.contains("+1 link"));
        assert!(summary.contains("+1 svc"));
        assert!(summary.contains("+1 mod"));
    }

    #[test]
    fn test_apply_plan_empty() {
        let plan = ApplyPlan::default();
        assert!(plan.is_empty());
        assert_eq!(plan.action_count(), 0);
    }

    #[test]
    fn test_apply_action_display() {
        let action = ApplyAction::InstallPackages {
            packages: vec!["neovim".into()],
        };
        let display = action.display();
        assert!(display.contains("neovim"));
        assert!(display.contains("[+]"));
    }

    #[test]
    fn test_builtin_variables_has_iron_root() {
        let tmp = TempDir::new().unwrap();
        let vars = builtin_variables(tmp.path());
        assert!(vars.contains_key("iron_root"));
    }

    #[test]
    fn test_host_variables_override_builtins() {
        let tmp = TempDir::new().unwrap();
        let mut host_vars = HashMap::new();
        host_vars.insert("iron_root".to_string(), "/custom/path".to_string());

        let mut vars = builtin_variables(tmp.path());
        vars.extend(host_vars);

        assert_eq!(vars.get("iron_root"), Some(&"/custom/path".to_string()));
    }

    #[test]
    fn test_profile_inheritance() {
        let tmp = TempDir::new().unwrap();
        let profiles_dir = tmp.path();

        // Base profile
        fs::create_dir_all(profiles_dir.join("base")).unwrap();
        fs::write(
            profiles_dir.join("base/profile.toml"),
            r#"
id = "base"
name = "Base"
modules = ["core-a", "core-b"]
"#,
        )
        .unwrap();

        // Child profile extends base
        fs::create_dir_all(profiles_dir.join("child")).unwrap();
        fs::write(
            profiles_dir.join("child/profile.toml"),
            r#"
id = "child"
name = "Child"
modules = ["extra-c"]
extends = "base"
"#,
        )
        .unwrap();

        let modules = load_profile_with_inheritance(profiles_dir, "child").unwrap();
        // Should have base modules first, then child
        assert!(modules.contains(&"core-a".to_string()));
        assert!(modules.contains(&"core-b".to_string()));
        assert!(modules.contains(&"extra-c".to_string()));
    }

    // ==========================================================================
    // ApplyAction variant tests
    // ==========================================================================

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(
            ApplyAction::InstallPackages {
                packages: vec!["a".into()],
            }
            .risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::InstallAurPackages {
                packages: vec!["a".into()],
            }
            .risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::CreateSymlink {
                source: "s".into(),
                target: "t".into(),
                module_id: "m".into(),
            }
            .risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::EnableService { name: "svc".into() }.risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::ActivateModule { id: "m".into() }.risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::CopyFile {
                source: "s".into(),
                target: "t".into(),
                backup_existing: false,
                module_id: "m".into(),
            }
            .risk_level(),
            RiskLevel::Additive
        );
        assert_eq!(
            ApplyAction::CopyFile {
                source: "s".into(),
                target: "t".into(),
                backup_existing: true,
                module_id: "m".into(),
            }
            .risk_level(),
            RiskLevel::Destructive
        );
        assert_eq!(
            ApplyAction::RenderAndCopy {
                source: "s".into(),
                target: "t".into(),
                variables: HashMap::new(),
                module_id: "m".into(),
            }
            .risk_level(),
            RiskLevel::Destructive
        );
        assert_eq!(
            ApplyAction::RemoveSymlink { target: "t".into() }.risk_level(),
            RiskLevel::Destructive
        );
        assert_eq!(
            ApplyAction::DisableService { name: "svc".into() }.risk_level(),
            RiskLevel::Destructive
        );
        assert_eq!(
            ApplyAction::DeactivateModule { id: "m".into() }.risk_level(),
            RiskLevel::Destructive
        );
        assert_eq!(
            ApplyAction::RemovePackages {
                packages: vec!["a".into()],
            }
            .risk_level(),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_is_prunable() {
        assert!(!ApplyAction::InstallPackages { packages: vec![] }.is_prunable());
        assert!(
            !ApplyAction::CreateSymlink {
                source: "s".into(),
                target: "t".into(),
                module_id: "m".into(),
            }
            .is_prunable()
        );
        assert!(
            ApplyAction::RemovePackages {
                packages: vec!["a".into()],
            }
            .is_prunable()
        );
        assert!(ApplyAction::DisableService { name: "svc".into() }.is_prunable());
        assert!(ApplyAction::RemoveSymlink { target: "t".into() }.is_prunable());
        assert!(ApplyAction::DeactivateModule { id: "m".into() }.is_prunable());
    }

    #[test]
    fn test_max_risk_empty_plan() {
        let plan = ApplyPlan::default();
        assert_eq!(plan.max_risk(), RiskLevel::ReadOnly);
    }

    #[test]
    fn test_max_risk_additive_only() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::ActivateModule { id: "m".into() },
            ],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Additive);
    }

    #[test]
    fn test_max_risk_critical_dominates() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into()],
                },
            ],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Critical);
    }

    #[test]
    fn test_prune_count() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into()],
                },
                ApplyAction::DisableService { name: "svc".into() },
                ApplyAction::RemoveSymlink { target: "t".into() },
            ],
        };
        assert_eq!(plan.prune_count(), 3);
    }

    #[test]
    fn test_summary_with_new_variants() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into(), "c".into()],
                },
                ApplyAction::RenderAndCopy {
                    source: "s".into(),
                    target: "t".into(),
                    variables: HashMap::new(),
                    module_id: "m".into(),
                },
                ApplyAction::DisableService { name: "svc".into() },
                ApplyAction::DeactivateModule { id: "m".into() },
            ],
        };
        let summary = plan.summary();
        assert!(summary.contains("+1 pkg"));
        assert!(summary.contains("-2 pkg"));
        assert!(summary.contains("+1 copy"));
        assert!(summary.contains("-1 svc"));
        assert!(summary.contains("-1 mod"));
    }

    #[test]
    fn test_display_new_variants() {
        let render = ApplyAction::RenderAndCopy {
            source: "s".into(),
            target: "~/.config/test".into(),
            variables: HashMap::new(),
            module_id: "test-mod".into(),
        };
        assert!(render.display().contains("[!]"));
        assert!(render.display().contains("Render template"));
        assert!(render.display().contains("test-mod"));

        let copy = ApplyAction::CopyFile {
            source: "s".into(),
            target: "~/.config/test".into(),
            backup_existing: false,
            module_id: "test-mod".into(),
        };
        assert!(copy.display().contains("[+]"));
        assert!(copy.display().contains("Copy file"));

        let rm_pkg = ApplyAction::RemovePackages {
            packages: vec!["pkg".into()],
        };
        assert!(rm_pkg.display().contains("[!!]"));
        assert!(rm_pkg.display().contains("Remove"));

        let disable = ApplyAction::DisableService { name: "svc".into() };
        assert!(disable.display().contains("[!]"));
        assert!(disable.display().contains("Disable"));

        let rm_link = ApplyAction::RemoveSymlink {
            target: "/path".into(),
        };
        assert!(rm_link.display().contains("[!]"));
        assert!(rm_link.display().contains("Remove symlink"));

        let deactivate = ApplyAction::DeactivateModule { id: "m".into() };
        assert!(deactivate.display().contains("[!]"));
        assert!(deactivate.display().contains("Deactivate"));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::ReadOnly < RiskLevel::Additive);
        assert!(RiskLevel::Additive < RiskLevel::Destructive);
        assert!(RiskLevel::Destructive < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(format!("{}", RiskLevel::ReadOnly), "read-only");
        assert_eq!(format!("{}", RiskLevel::Additive), "additive");
        assert_eq!(format!("{}", RiskLevel::Destructive), "destructive");
        assert_eq!(format!("{}", RiskLevel::Critical), "critical");
    }

    #[test]
    fn test_risk_summary() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::ActivateModule { id: "m".into() },
                ApplyAction::RenderAndCopy {
                    source: "s".into(),
                    target: "t".into(),
                    variables: HashMap::new(),
                    module_id: "m".into(),
                },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into()],
                },
            ],
        };
        let summary = plan.risk_summary();
        assert_eq!(summary.get(&RiskLevel::Additive), Some(&2));
        assert_eq!(summary.get(&RiskLevel::Destructive), Some(&1));
        assert_eq!(summary.get(&RiskLevel::Critical), Some(&1));
        assert_eq!(summary.get(&RiskLevel::ReadOnly), None);
    }

    #[test]
    fn test_risk_summary_empty_plan() {
        let plan = ApplyPlan::default();
        let summary = plan.risk_summary();
        assert!(summary.is_empty());
    }

    // ==========================================================================
    // Removal diff + PrunePolicy tests
    // ==========================================================================

    #[test]
    fn test_prune_policy_defaults_to_none() {
        let policy = PrunePolicy::default();
        assert!(!policy.packages);
        assert!(!policy.services);
        assert!(!policy.dotfiles);
        assert!(!policy.any_enabled());
    }

    #[test]
    fn test_prune_policy_none() {
        let policy = PrunePolicy::none();
        assert!(!policy.any_enabled());
    }

    #[test]
    fn test_prune_policy_all() {
        let policy = PrunePolicy::all();
        assert!(policy.packages);
        assert!(policy.services);
        assert!(policy.dotfiles);
        assert!(policy.any_enabled());
    }

    #[test]
    fn test_prune_policy_selective() {
        let policy = PrunePolicy {
            packages: true,
            services: false,
            dotfiles: false,
        };
        assert!(policy.any_enabled());
        assert!(policy.packages);
        assert!(!policy.services);
        assert!(!policy.dotfiles);
    }

    #[test]
    fn test_compute_plan_removal_packages() {
        // Setup: create a minimal test environment
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Host with no packages desired
        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::write(
            root.join("hosts/test.toml"),
            r#"
id = "test"
name = "Test Host"
installed_bundles = []
[hardware]
"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();

        // Record "old-pkg" as managed
        state_mgr
            .record_managed_packages(&["old-pkg".to_string()])
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // Desired state: no packages
        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("old-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Should have RemovePackages action for old-pkg
        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        assert!(
            remove_action.is_some(),
            "Should emit RemovePackages for managed pkg not in desired"
        );
        if let Some(ApplyAction::RemovePackages { packages }) = remove_action {
            assert!(packages.contains(&"old-pkg".to_string()));
        }
    }

    #[test]
    fn test_compute_plan_no_removal_for_unmanaged() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();

        // Do NOT record "unmanaged-pkg" as managed
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("unmanaged-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Should NOT have RemovePackages for unmanaged pkg
        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        assert!(
            remove_action.is_none(),
            "Should NOT emit RemovePackages for unmanaged pkg"
        );
    }

    #[test]
    fn test_compute_plan_disable_service() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr.record_managed_service("old.service").unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![crate::actual_state::ActualServiceState {
                name: "old.service".to_string(),
                enabled: true,
                running: false,
            }],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let disable_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::DisableService { .. }));
        assert!(
            disable_action.is_some(),
            "Should emit DisableService for managed svc not in desired"
        );
        if let Some(ApplyAction::DisableService { name }) = disable_action {
            assert_eq!(name, "old.service");
        }
    }

    #[test]
    fn test_compute_plan_no_disable_unmanaged_service() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        // Do NOT record "random.service" as managed

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![crate::actual_state::ActualServiceState {
                name: "random.service".to_string(),
                enabled: true,
                running: true,
            }],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let disable_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::DisableService { .. }));
        assert!(
            disable_action.is_none(),
            "Should NOT disable unmanaged service"
        );
    }

    #[test]
    fn test_compute_plan_remove_dotfile() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_dotfile("/home/user/.config/old/config")
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // Desired has no dotfiles
        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemoveSymlink { .. }));
        assert!(
            remove_action.is_some(),
            "Should emit RemoveSymlink for managed dotfile not in desired"
        );
        if let Some(ApplyAction::RemoveSymlink { target }) = remove_action {
            assert_eq!(target, "/home/user/.config/old/config");
        }
    }

    #[test]
    fn test_compute_plan_no_remove_current_dotfile() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules/test-mod")).unwrap();
        fs::write(
            root.join("modules/test-mod/module.toml"),
            r#"
id = "test-mod"
name = "test"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []

[[dotfiles]]
source = "config"
target = "/home/user/.config/test/config"
link = true
"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_dotfile("/home/user/.config/test/config")
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // Desired still has this dotfile
        let desired = DesiredState {
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "/home/user/.config/test/config".to_string(),
                link: true,
            }],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemoveSymlink { .. }));
        assert!(
            remove_action.is_none(),
            "Should NOT remove a dotfile still in desired state"
        );
    }

    #[test]
    fn test_compute_plan_relinks_symlink_pointing_elsewhere() {
        // Regression: a dotfile target that is ALREADY a symlink but points at
        // the wrong source must be re-linked. Previously the planner compared
        // only the final path component, so an old symlink like
        // ~/.config/test/config -> /elsewhere/config was treated as correct
        // (both end in "config") and never re-pointed at the iron module.
        use crate::actual_state::{ActualFileState, FileStateType};

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules/test-mod/config")).unwrap();
        fs::write(
            root.join("modules/test-mod/module.toml"),
            r#"
id = "test-mod"
name = "test"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []

[[dotfiles]]
source = "config"
target = "/home/user/.config/test/config"
link = true
"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();

        use crate::packages::NoopPackageManager;
        use crate::system_service::NoopSystemService;
        let service = DefaultApplyService::new(
            root,
            state_mgr,
            Arc::new(NoopPackageManager),
            Arc::new(NoopSystemService),
        );

        let desired = DesiredState {
            modules: vec!["test-mod".to_string()],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "/home/user/.config/test/config".to_string(),
                link: true,
            }],
            ..Default::default()
        };

        let make_actual = |symlink_to: &str| ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![ActualFileState {
                target: "/home/user/.config/test/config".to_string(),
                exists: true,
                symlink_target: Some(symlink_to.to_string()),
                checksum: None,
                file_type: FileStateType::Symlink,
            }],
            scanned_at: chrono::Utc::now(),
        };

        // (a) points at an unrelated dir whose last component matches -> re-link
        let plan = service
            .compute_plan(&desired, &make_actual("/some/other/place/config"))
            .unwrap();
        assert!(
            plan.actions.iter().any(|a| matches!(
                a,
                ApplyAction::CreateSymlink { target, .. }
                    if target == "/home/user/.config/test/config"
            )),
            "symlink pointing elsewhere must be re-linked, got: {:?}",
            plan.actions
        );

        // (b) already points at the canonical iron source -> no re-link (idempotent)
        let correct = root
            .join("modules/test-mod/config")
            .to_string_lossy()
            .to_string();
        let plan = service
            .compute_plan(&desired, &make_actual(&correct))
            .unwrap();
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, ApplyAction::CreateSymlink { .. })),
            "correctly-linked dotfile must not be re-linked, got: {:?}",
            plan.actions
        );
    }

    #[test]
    fn test_compute_plan_deactivate_module() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        // Activate "old-module" in state
        state_mgr.enable_module("old-module").unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // Desired modules: none
        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let deactivate_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::DeactivateModule { .. }));
        assert!(
            deactivate_action.is_some(),
            "Should emit DeactivateModule for active module not in desired"
        );
        if let Some(ApplyAction::DeactivateModule { id }) = deactivate_action {
            assert_eq!(id, "old-module");
        }
    }

    #[test]
    fn test_should_execute_prune_policy_packages() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr)
            .with_prune_policy(PrunePolicy {
                packages: true,
                services: false,
                dotfiles: false,
            });

        // RemovePackages should be allowed
        assert!(service.should_execute_prune(&ApplyAction::RemovePackages {
            packages: vec!["a".into()],
        }));
        // DisableService should NOT be allowed
        assert!(!service.should_execute_prune(&ApplyAction::DisableService { name: "svc".into() }));
        // RemoveSymlink should NOT be allowed
        assert!(!service.should_execute_prune(&ApplyAction::RemoveSymlink { target: "t".into() }));
    }

    #[test]
    fn test_should_execute_prune_policy_all() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr)
            .with_prune_policy(PrunePolicy::all());

        assert!(service.should_execute_prune(&ApplyAction::RemovePackages {
            packages: vec!["a".into()],
        }));
        assert!(service.should_execute_prune(&ApplyAction::DisableService { name: "svc".into() }));
        assert!(service.should_execute_prune(&ApplyAction::RemoveSymlink { target: "t".into() }));
        assert!(service.should_execute_prune(&ApplyAction::DeactivateModule { id: "m".into() }));
    }

    #[test]
    fn test_should_execute_prune_policy_none() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);
        // Default prune policy = none

        assert!(!service.should_execute_prune(&ApplyAction::RemovePackages {
            packages: vec!["a".into()],
        }));
        assert!(!service.should_execute_prune(&ApplyAction::DisableService { name: "svc".into() }));
        // Non-prunable actions always execute
        assert!(service.should_execute_prune(&ApplyAction::InstallPackages {
            packages: vec!["a".into()],
        }));
    }

    #[test]
    fn test_bootstrap_managed_tracking_seeds() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr.clone(), pkg_mgr, svc_mgr);

        let desired = DesiredState {
            packages: vec!["pkg-a".into(), "pkg-b".into()],
            services: vec!["svc-a.service".into()],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("pkg-a".to_string());
                // pkg-b is NOT installed, so should NOT be seeded
                s
            },
            aur_packages: HashSet::new(),
            services: vec![crate::actual_state::ActualServiceState {
                name: "svc-a.service".to_string(),
                enabled: true,
                running: false,
            }],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        service.bootstrap_managed_tracking(&desired, &actual);

        let managed = state_mgr.managed_packages();
        assert!(managed.contains(&"pkg-a".to_string()));
        assert!(
            !managed.contains(&"pkg-b".to_string()),
            "pkg-b not installed, should not be seeded"
        );

        let managed_svcs = state_mgr.managed_services();
        assert!(managed_svcs.contains(&"svc-a.service".to_string()));
    }

    #[test]
    fn test_bootstrap_skips_when_already_tracked() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        // Pre-populate with one managed package
        state_mgr
            .record_managed_packages(&["existing-pkg".to_string()])
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr.clone(), pkg_mgr, svc_mgr);

        let desired = DesiredState {
            packages: vec!["new-pkg".into()],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("new-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        service.bootstrap_managed_tracking(&desired, &actual);

        let managed = state_mgr.managed_packages();
        // Should still only have existing-pkg, not new-pkg
        assert!(managed.contains(&"existing-pkg".to_string()));
        assert!(
            !managed.contains(&"new-pkg".to_string()),
            "Bootstrap should not run when managed lists are non-empty"
        );
    }

    // ==========================================================================
    // Additional coverage tests
    // ==========================================================================

    // ── Template helpers ─────────────────────────────────────

    #[test]
    fn test_has_template_variables_basic() {
        assert!(has_template_variables("Hello {{name}}!"));
        assert!(has_template_variables("{{a}} and {{b}}"));
    }

    #[test]
    fn test_has_template_variables_false_for_no_templates() {
        assert!(!has_template_variables("No templates here"));
        assert!(!has_template_variables("Just {one brace}"));
        assert!(!has_template_variables(""));
    }

    #[test]
    fn test_has_template_variables_literal_double_brace() {
        // A file with literal {{ that is not a variable (e.g. Jinja/Nunjucks comment)
        // should still be detected as having templates — the detection is intentionally broad
        assert!(has_template_variables("{{ this is not closed"));
        assert!(has_template_variables("{{}}")); // empty variable
    }

    #[test]
    fn test_render_template_basic_substitution() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        let result = render_template("Hello {{name}}!", &vars);
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_render_template_whitespace_trimming() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        let result = render_template("Hello {{ name }}!", &vars);
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_render_template_unknown_variable_preserved() {
        let vars = HashMap::new();
        let result = render_template("Hello {{unknown}}!", &vars);
        assert_eq!(result, "Hello {{unknown}}!");
    }

    #[test]
    fn test_render_template_empty_content() {
        let vars = HashMap::new();
        let result = render_template("", &vars);
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_template_no_variables_passthrough() {
        let vars = HashMap::new();
        let result = render_template("plain text with no vars", &vars);
        assert_eq!(result, "plain text with no vars");
    }

    #[test]
    fn test_render_template_multiple_same_variable() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), "42".to_string());
        let result = render_template("{{x}} and {{x}}", &vars);
        assert_eq!(result, "42 and 42");
    }

    #[test]
    fn test_render_template_unclosed_brace() {
        let vars = HashMap::new();
        let result = render_template("{{unclosed", &vars);
        assert_eq!(result, "{{unclosed");
    }

    // ── PrunePolicy edge cases ───────────────────────────────

    #[test]
    fn test_prune_policy_services_only() {
        let policy = PrunePolicy {
            packages: false,
            services: true,
            dotfiles: false,
        };
        assert!(policy.any_enabled());
        assert!(!policy.packages);
        assert!(policy.services);
        assert!(!policy.dotfiles);
    }

    #[test]
    fn test_prune_policy_dotfiles_only() {
        let policy = PrunePolicy {
            packages: false,
            services: false,
            dotfiles: true,
        };
        assert!(policy.any_enabled());
    }

    // ── Summary edge cases ───────────────────────────────────

    #[test]
    fn test_summary_empty_plan() {
        let plan = ApplyPlan::default();
        assert_eq!(plan.summary(), "No changes");
    }

    #[test]
    fn test_summary_only_removals() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::RemovePackages {
                    packages: vec!["old".into()],
                },
                ApplyAction::RemoveSymlink {
                    target: "/old/file".into(),
                },
                ApplyAction::DisableService {
                    name: "old.service".into(),
                },
                ApplyAction::DeactivateModule {
                    id: "old-mod".into(),
                },
            ],
        };
        let summary = plan.summary();
        assert!(summary.contains("-1 pkg"), "summary: {}", summary);
        assert!(summary.contains("-1 file"), "summary: {}", summary);
        assert!(summary.contains("-1 svc"), "summary: {}", summary);
        assert!(summary.contains("-1 mod"), "summary: {}", summary);
        // Should NOT contain any + prefixed items
        assert!(!summary.contains("+"), "summary: {}", summary);
    }

    #[test]
    fn test_summary_copy_and_render() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::CopyFile {
                    source: "s".into(),
                    target: "t".into(),
                    backup_existing: false,
                    module_id: "m".into(),
                },
                ApplyAction::RenderAndCopy {
                    source: "s2".into(),
                    target: "t2".into(),
                    variables: HashMap::new(),
                    module_id: "m".into(),
                },
            ],
        };
        let summary = plan.summary();
        assert!(summary.contains("+2 copy"), "summary: {}", summary);
    }

    // ── RiskLevel additional edge cases ──────────────────────

    #[test]
    fn test_max_risk_destructive_without_critical() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RenderAndCopy {
                    source: "s".into(),
                    target: "t".into(),
                    variables: HashMap::new(),
                    module_id: "m".into(),
                },
            ],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Destructive);
    }

    #[test]
    fn test_risk_summary_all_same_level() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::EnableService { name: "svc".into() },
                ApplyAction::ActivateModule { id: "m".into() },
            ],
        };
        let summary = plan.risk_summary();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary.get(&RiskLevel::Additive), Some(&3));
    }

    #[test]
    fn test_risk_summary_mixed_all_four_levels_impossible() {
        // ReadOnly cannot appear as an action risk level (no action returns ReadOnly).
        // Verify this invariant: no action has ReadOnly risk.
        let all_actions = vec![
            ApplyAction::InstallPackages {
                packages: vec!["a".into()],
            },
            ApplyAction::InstallAurPackages {
                packages: vec!["b".into()],
            },
            ApplyAction::CreateSymlink {
                source: "s".into(),
                target: "t".into(),
                module_id: "m".into(),
            },
            ApplyAction::EnableService { name: "svc".into() },
            ApplyAction::ActivateModule { id: "m".into() },
            ApplyAction::CopyFile {
                source: "s".into(),
                target: "t".into(),
                backup_existing: false,
                module_id: "m".into(),
            },
            ApplyAction::CopyFile {
                source: "s".into(),
                target: "t".into(),
                backup_existing: true,
                module_id: "m".into(),
            },
            ApplyAction::RenderAndCopy {
                source: "s".into(),
                target: "t".into(),
                variables: HashMap::new(),
                module_id: "m".into(),
            },
            ApplyAction::RemoveSymlink { target: "t".into() },
            ApplyAction::DisableService { name: "svc".into() },
            ApplyAction::DeactivateModule { id: "m".into() },
            ApplyAction::RemovePackages {
                packages: vec!["a".into()],
            },
        ];
        for action in &all_actions {
            assert_ne!(
                action.risk_level(),
                RiskLevel::ReadOnly,
                "No action should have ReadOnly risk: {:?}",
                action
            );
        }
    }

    // ── Removal diff edge cases ──────────────────────────────

    #[test]
    fn test_compute_plan_empty_managed_lists_no_removals() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // Empty desired state, empty managed lists, but system has packages
        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("system-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![crate::actual_state::ActualServiceState {
                name: "system.service".to_string(),
                enabled: true,
                running: true,
            }],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // No removals should be emitted because managed lists are empty
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, ApplyAction::RemovePackages { .. })),
            "No RemovePackages with empty managed_packages"
        );
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, ApplyAction::DisableService { .. })),
            "No DisableService with empty managed_services"
        );
    }

    #[test]
    fn test_compute_plan_managed_but_already_uninstalled() {
        // Package is in managed_packages but no longer installed on system
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_packages(&["gone-pkg".to_string()])
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(), // gone-pkg is NOT installed
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Should NOT emit RemovePackages because the package is not installed
        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        assert!(
            remove_action.is_none(),
            "Should not remove managed pkg that is already uninstalled"
        );
    }

    #[test]
    fn test_compute_plan_aur_packages_not_removed() {
        // A package appears in managed_packages AND desired.aur_packages -> no removal
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_packages(&["aur-pkg".to_string()])
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        // aur-pkg is in desired.aur_packages (NOT desired.packages)
        let desired = DesiredState {
            aur_packages: vec!["aur-pkg".to_string()],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("aur-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        assert!(
            remove_action.is_none(),
            "AUR packages in desired should not be flagged for removal"
        );
    }

    #[test]
    fn test_compute_plan_service_not_disabled_when_already_stopped() {
        // Service is managed but not enabled on system -> no DisableService
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr.record_managed_service("stopped.service").unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![crate::actual_state::ActualServiceState {
                name: "stopped.service".to_string(),
                enabled: false, // already disabled
                running: false,
            }],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let disable_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::DisableService { .. }));
        assert!(
            disable_action.is_none(),
            "Should not disable a service that is already not enabled"
        );
    }

    // ── Bootstrap edge cases ─────────────────────────────────

    #[test]
    fn test_bootstrap_includes_aur_packages() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr.clone(), pkg_mgr, svc_mgr);

        let desired = DesiredState {
            packages: vec!["pacman-pkg".into()],
            aur_packages: vec!["aur-pkg".into()],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("pacman-pkg".to_string());
                s.insert("aur-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        service.bootstrap_managed_tracking(&desired, &actual);

        let managed = state_mgr.managed_packages();
        assert!(
            managed.contains(&"pacman-pkg".to_string()),
            "pacman pkg should be seeded"
        );
        assert!(
            managed.contains(&"aur-pkg".to_string()),
            "AUR pkg should be seeded"
        );
    }

    #[test]
    fn test_bootstrap_skips_when_services_exist() {
        // If managed_services is non-empty but managed_packages is empty,
        // bootstrap should NOT run (guard checks all three lists)
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_service("existing.service")
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr.clone(), pkg_mgr, svc_mgr);

        let desired = DesiredState {
            packages: vec!["new-pkg".into()],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("new-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        service.bootstrap_managed_tracking(&desired, &actual);

        let managed = state_mgr.managed_packages();
        assert!(
            managed.is_empty(),
            "Bootstrap should not run when managed_services is non-empty"
        );
    }

    #[test]
    fn test_bootstrap_seeds_dotfiles() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr.clone(), pkg_mgr, svc_mgr);

        let desired = DesiredState {
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "/home/user/.config/nvim".to_string(),
                link: true,
            }],
            ..Default::default()
        };
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![crate::actual_state::ActualFileState {
                target: "/home/user/.config/nvim".to_string(),
                exists: true,
                file_type: crate::actual_state::FileStateType::Symlink,
                symlink_target: Some("modules/nvim/config".to_string()),
                checksum: None,
            }],
            scanned_at: chrono::Utc::now(),
        };

        service.bootstrap_managed_tracking(&desired, &actual);

        let managed_dots = state_mgr.managed_dotfiles();
        assert!(
            managed_dots.contains(&"/home/user/.config/nvim".to_string()),
            "Existing dotfile should be seeded during bootstrap"
        );
    }

    // ── Prune gating with selective policy ───────────────────

    #[test]
    fn test_should_execute_prune_deactivate_uses_dotfiles_policy() {
        // DeactivateModule uses the dotfiles policy (architect decision)
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        // Only dotfiles pruning enabled
        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr)
            .with_prune_policy(PrunePolicy {
                packages: false,
                services: false,
                dotfiles: true,
            });

        assert!(service.should_execute_prune(&ApplyAction::DeactivateModule { id: "m".into() }));
        assert!(service.should_execute_prune(&ApplyAction::RemoveSymlink { target: "t".into() }));
        // packages and services should not execute
        assert!(!service.should_execute_prune(&ApplyAction::RemovePackages {
            packages: vec!["a".into()],
        }));
        assert!(!service.should_execute_prune(&ApplyAction::DisableService { name: "svc".into() }));
    }

    // ── is_prunable comprehensive ────────────────────────────

    #[test]
    fn test_is_prunable_comprehensive_non_prunable() {
        // Verify all non-prunable variants
        let non_prunable = vec![
            ApplyAction::InstallAurPackages {
                packages: vec!["a".into()],
            },
            ApplyAction::EnableService { name: "svc".into() },
            ApplyAction::ActivateModule { id: "m".into() },
            ApplyAction::CopyFile {
                source: "s".into(),
                target: "t".into(),
                backup_existing: false,
                module_id: "m".into(),
            },
            ApplyAction::RenderAndCopy {
                source: "s".into(),
                target: "t".into(),
                variables: HashMap::new(),
                module_id: "m".into(),
            },
        ];
        for action in &non_prunable {
            assert!(!action.is_prunable(), "{:?} should not be prunable", action);
        }
    }

    // ── Multiple removal packages in single action ───────────

    #[test]
    fn test_compute_plan_multiple_packages_in_single_removal() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();
        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        state_mgr
            .record_managed_packages(&[
                "old-a".to_string(),
                "old-b".to_string(),
                "old-c".to_string(),
            ])
            .unwrap();

        use crate::packages::NoopPackageManager;
        let pkg_mgr = Arc::new(NoopPackageManager);
        use crate::system_service::NoopSystemService;
        let svc_mgr = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let desired = DesiredState::default();
        let actual = ActualState {
            hostname: "test".into(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("old-a".to_string());
                s.insert("old-b".to_string());
                s.insert("old-c".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let plan = service.compute_plan(&desired, &actual).unwrap();

        let remove_action = plan
            .actions
            .iter()
            .find(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        assert!(remove_action.is_some());
        if let Some(ApplyAction::RemovePackages { packages }) = remove_action {
            assert_eq!(
                packages.len(),
                3,
                "All 3 managed packages should be in a single removal action"
            );
        }
    }

    // ==========================================================================
    // F3-014: Hook lifecycle tests
    // ==========================================================================

    #[test]
    fn test_run_hook_risk_level() {
        let action = ApplyAction::RunHook {
            module_id: "test".into(),
            hook_type: HookType::PreInstall,
            command: "echo test".into(),
            behavior: HookBehavior::Always,
        };
        assert_eq!(action.risk_level(), RiskLevel::Destructive);
    }

    #[test]
    fn test_run_hook_not_prunable() {
        let action = ApplyAction::RunHook {
            module_id: "test".into(),
            hook_type: HookType::PostInstall,
            command: "echo test".into(),
            behavior: HookBehavior::Always,
        };
        assert!(!action.is_prunable());
    }

    #[test]
    fn test_run_hook_display_always() {
        let action = ApplyAction::RunHook {
            module_id: "nvim".into(),
            hook_type: HookType::PostInstall,
            command: "nvim --headless +Lazy! sync +qa".into(),
            behavior: HookBehavior::Always,
        };
        let display = action.display();
        assert!(display.contains("[!]"));
        assert!(display.contains("post_install"));
        assert!(display.contains("nvim"));
    }

    #[test]
    fn test_run_hook_display_ask() {
        let action = ApplyAction::RunHook {
            module_id: "security".into(),
            hook_type: HookType::PreInstall,
            command: "gpasswd -a $USER video".into(),
            behavior: HookBehavior::Ask,
        };
        let display = action.display();
        assert!(display.contains("[?]"));
        assert!(display.contains("pre_install"));
    }

    #[test]
    fn test_run_hook_display_truncates_long_command() {
        let long_cmd = "a".repeat(100);
        let action = ApplyAction::RunHook {
            module_id: "m".into(),
            hook_type: HookType::PostInstall,
            command: long_cmd,
            behavior: HookBehavior::Always,
        };
        let display = action.display();
        assert!(display.contains("..."));
        // Display should be reasonable length
        assert!(display.len() < 200);
    }

    #[test]
    fn test_summary_includes_hooks() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RunHook {
                    module_id: "m".into(),
                    hook_type: HookType::PreInstall,
                    command: "echo pre".into(),
                    behavior: HookBehavior::Always,
                },
                ApplyAction::RunHook {
                    module_id: "m".into(),
                    hook_type: HookType::PostInstall,
                    command: "echo post".into(),
                    behavior: HookBehavior::Always,
                },
            ],
        };
        let summary = plan.summary();
        assert!(summary.contains("+1 pkg"));
        assert!(summary.contains("2 hook"));
    }

    #[test]
    fn test_hook_ordering_in_plan() {
        // Verify hooks are placed in correct phase order
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create a module with both pre and post hooks
        fs::create_dir_all(root.join("modules/hooked")).unwrap();
        fs::write(
            root.join("modules/hooked/module.toml"),
            r#"
id = "hooked"
name = "Hooked Module"
kind = "AppConfig"
packages = ["pkg-x"]
aur_packages = []
conflicts = []
depends = []
pre_install = "echo pre"
post_install = "echo post"

[[dotfiles]]
source = "config"
target = "~/.config/hooked"
link = true
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();

        let desired = DesiredState {
            modules: vec!["hooked".to_string()],
            packages: vec!["pkg-x".to_string()],
            ..Default::default()
        };

        use crate::packages::NoopPackageManager;
        use crate::system_service::NoopSystemService;

        let actual = crate::actual_state::ActualState {
            hostname: "test".to_string(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        let pkg_mgr: Arc<dyn PackageManager> = Arc::new(NoopPackageManager);
        let svc_mgr: Arc<dyn SystemService> = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Find indices
        let pre_hook_idx = plan.actions.iter().position(|a| {
            matches!(
                a,
                ApplyAction::RunHook {
                    hook_type: HookType::PreInstall,
                    ..
                }
            )
        });
        let install_idx = plan
            .actions
            .iter()
            .position(|a| matches!(a, ApplyAction::InstallPackages { .. }));
        let post_hook_idx = plan.actions.iter().position(|a| {
            matches!(
                a,
                ApplyAction::RunHook {
                    hook_type: HookType::PostInstall,
                    ..
                }
            )
        });

        assert!(pre_hook_idx.is_some(), "Pre-install hook should be in plan");
        assert!(install_idx.is_some(), "Install action should be in plan");
        assert!(
            post_hook_idx.is_some(),
            "Post-install hook should be in plan"
        );

        // Ordering: pre_hook < install < post_hook
        assert!(
            pre_hook_idx.unwrap() < install_idx.unwrap(),
            "Pre-install hook should come before install"
        );
        assert!(
            install_idx.unwrap() < post_hook_idx.unwrap(),
            "Install should come before post-install hook"
        );
    }

    #[test]
    fn test_default_apply_service_builder_methods() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        use crate::packages::NoopPackageManager;
        use crate::system_service::NoopSystemService;

        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        let pkg_mgr: Arc<dyn PackageManager> = Arc::new(NoopPackageManager);
        let svc_mgr: Arc<dyn SystemService> = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr)
            .with_force_hooks(true)
            .with_interactive(false)
            .with_hook_timeout(30)
            .with_prune_policy(PrunePolicy::all());

        assert!(service.force_hooks);
        assert!(!service.interactive);
        assert_eq!(service.hook_timeout, 30);
        assert!(service.prune_policy.packages);
    }

    // ==========================================================
    // F3-018: dotfiles_sync tests
    // ==========================================================

    #[test]
    fn test_discover_dotfiles_basic() {
        let tmp = TempDir::new().unwrap();
        let dotfiles_dir = tmp.path().join("dotfiles");
        fs::create_dir_all(&dotfiles_dir).unwrap();
        fs::write(dotfiles_dir.join("init.lua"), "-- config").unwrap();
        fs::write(dotfiles_dir.join("settings.json"), "{}").unwrap();

        let mappings = discover_dotfiles(&dotfiles_dir, "~/.config/nvim/");

        assert_eq!(mappings.len(), 2);
        let sources: Vec<&str> = mappings.iter().map(|m| m.source.as_str()).collect();
        assert!(sources.contains(&"dotfiles/init.lua"));
        assert!(sources.contains(&"dotfiles/settings.json"));

        let targets: Vec<&str> = mappings.iter().map(|m| m.target.as_str()).collect();
        assert!(targets.contains(&"~/.config/nvim/init.lua"));
        assert!(targets.contains(&"~/.config/nvim/settings.json"));

        // All default to symlink
        assert!(mappings.iter().all(|m| m.link));
    }

    #[test]
    fn test_discover_dotfiles_nested_structure() {
        let tmp = TempDir::new().unwrap();
        let dotfiles_dir = tmp.path().join("dotfiles");
        fs::create_dir_all(dotfiles_dir.join("lua/plugins")).unwrap();
        fs::write(dotfiles_dir.join("init.lua"), "-- root").unwrap();
        fs::write(dotfiles_dir.join("lua/plugins/treesitter.lua"), "-- ts").unwrap();

        let mappings = discover_dotfiles(&dotfiles_dir, "~/.config/nvim");

        assert_eq!(mappings.len(), 2);
        let targets: Vec<&str> = mappings.iter().map(|m| m.target.as_str()).collect();
        assert!(targets.contains(&"~/.config/nvim/init.lua"));
        assert!(targets.contains(&"~/.config/nvim/lua/plugins/treesitter.lua"));
    }

    #[test]
    fn test_discover_dotfiles_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let dotfiles_dir = tmp.path().join("dotfiles");
        fs::create_dir_all(&dotfiles_dir).unwrap();

        let mappings = discover_dotfiles(&dotfiles_dir, "~/.config/test/");
        assert!(mappings.is_empty());
    }

    #[test]
    fn test_dotfiles_sync_merge_explicit_wins() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create a module with dotfiles_sync and explicit dotfiles
        let module_dir = root.join("modules/nvim");
        fs::create_dir_all(module_dir.join("dotfiles")).unwrap();
        fs::write(module_dir.join("dotfiles/init.lua"), "-- auto").unwrap();
        fs::write(module_dir.join("dotfiles/options.lua"), "-- auto").unwrap();

        // The explicit dotfile targets init.lua with a custom source
        fs::write(
            module_dir.join("module.toml"),
            r#"
id = "nvim"
name = "Neovim"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []
dotfiles_sync = true

[[dotfiles]]
source = "custom/init.lua"
target = "~/.config/nvim/init.lua"
link = false
"#,
        )
        .unwrap();

        // Create minimal host as TOML string
        let host: Host = toml::from_str(
            r#"
id = "test"
name = "Test"
installed_bundles = []
extra_modules = ["nvim"]

[variables]

[hardware]
"#,
        )
        .unwrap();

        let desired = resolve_desired_state(root, &host).unwrap();

        // Should have 2 dotfile entries: explicit init.lua +
        // auto-discovered options.lua
        let init_targets: Vec<&DotfileMapping> = desired
            .dotfiles
            .iter()
            .filter(|d| d.target == "~/.config/nvim/init.lua")
            .collect();
        // Explicit wins -- only the explicit entry for init.lua
        assert_eq!(init_targets.len(), 1);
        assert_eq!(init_targets[0].source, "custom/init.lua");
        assert!(!init_targets[0].link); // explicit says copy

        // options.lua auto-discovered
        let options: Vec<&DotfileMapping> = desired
            .dotfiles
            .iter()
            .filter(|d| d.target == "~/.config/nvim/options.lua")
            .collect();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].source, "dotfiles/options.lua");
        assert!(options[0].link); // auto default
    }

    #[test]
    fn test_dotfiles_sync_custom_target() {
        let tmp = TempDir::new().unwrap();
        let dotfiles_dir = tmp.path().join("dotfiles");
        fs::create_dir_all(&dotfiles_dir).unwrap();
        fs::write(dotfiles_dir.join("config.yml"), "test").unwrap();

        let mappings = discover_dotfiles(&dotfiles_dir, "~/.local/share/app");

        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].target, "~/.local/share/app/config.yml");
    }

    #[test]
    fn test_skip_behavior_hooks_in_plan_but_skip_at_execution() {
        // Per architect decision: ALL hooks are included in the plan for visibility.
        // Skip/Ask/Once filtering happens at execution time, not plan time.
        // A module with hook_behavior = Skip still has RunHook in the plan,
        // but they carry HookBehavior::Skip so execute_action skips them.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("modules/skipped")).unwrap();
        fs::write(
            root.join("modules/skipped/module.toml"),
            r#"
id = "skipped"
name = "Skipped Hooks Module"
kind = "AppConfig"
packages = ["pkg-skip"]
aur_packages = []
dotfiles = []
conflicts = []
depends = []
pre_install = "echo pre"
post_install = "echo post"
hook_behavior = "skip"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();

        let desired = DesiredState {
            modules: vec!["skipped".to_string()],
            packages: vec!["pkg-skip".to_string()],
            ..Default::default()
        };

        use crate::packages::NoopPackageManager;
        use crate::system_service::NoopSystemService;

        let actual = crate::actual_state::ActualState {
            hostname: "test".to_string(),
            installed_packages: HashSet::new(),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        let pkg_mgr: Arc<dyn PackageManager> = Arc::new(NoopPackageManager);
        let svc_mgr: Arc<dyn SystemService> = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Hooks ARE in the plan (for dry-run visibility)
        let hooks: Vec<&ApplyAction> = plan
            .actions
            .iter()
            .filter(|a| matches!(a, ApplyAction::RunHook { .. }))
            .collect();
        assert_eq!(
            hooks.len(),
            2,
            "Skip hooks should still appear in plan for visibility"
        );

        // Verify they carry the Skip behavior
        for hook in hooks {
            if let ApplyAction::RunHook { behavior, .. } = hook {
                assert_eq!(*behavior, HookBehavior::Skip);
            }
        }

        // Packages should still be in the plan
        assert!(
            plan.actions
                .iter()
                .any(|a| matches!(a, ApplyAction::InstallPackages { .. }))
        );
    }

    #[test]
    fn test_pre_uninstall_hook_ordering_before_removal() {
        // When a module is deactivated and has pre_uninstall,
        // the pre_uninstall hook should appear before removal actions
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("modules/removable")).unwrap();
        fs::write(
            root.join("modules/removable/module.toml"),
            r#"
id = "removable"
name = "Removable Module"
kind = "AppConfig"
packages = ["old-pkg"]
aur_packages = []
dotfiles = []
conflicts = []
depends = []
pre_uninstall = "echo cleanup"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("hosts")).unwrap();
        fs::create_dir_all(root.join("bundles")).unwrap();
        fs::create_dir_all(root.join("profiles")).unwrap();

        let state_mgr = StateManager::new(root.to_path_buf()).unwrap();
        // Simulate that "removable" is currently active
        state_mgr.enable_module("removable").unwrap();
        state_mgr
            .record_managed_packages(&["old-pkg".to_string()])
            .unwrap();

        use crate::packages::NoopPackageManager;
        use crate::system_service::NoopSystemService;

        // Desired state has NO modules -- "removable" is being removed
        let desired = DesiredState::default();

        let actual = crate::actual_state::ActualState {
            hostname: "test".to_string(),
            installed_packages: {
                let mut s = HashSet::new();
                s.insert("old-pkg".to_string());
                s
            },
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: chrono::Utc::now(),
        };

        let pkg_mgr: Arc<dyn PackageManager> = Arc::new(NoopPackageManager);
        let svc_mgr: Arc<dyn SystemService> = Arc::new(NoopSystemService);

        let service = DefaultApplyService::new(root, state_mgr, pkg_mgr, svc_mgr);

        let plan = service.compute_plan(&desired, &actual).unwrap();

        // Find indices of pre_uninstall hook and removal actions
        let pre_uninstall_idx = plan.actions.iter().position(|a| {
            matches!(
                a,
                ApplyAction::RunHook {
                    hook_type: HookType::PreUninstall,
                    ..
                }
            )
        });
        let removal_idx = plan
            .actions
            .iter()
            .position(|a| matches!(a, ApplyAction::RemovePackages { .. }));
        let deactivate_idx = plan
            .actions
            .iter()
            .position(|a| matches!(a, ApplyAction::DeactivateModule { .. }));

        assert!(
            pre_uninstall_idx.is_some(),
            "Pre-uninstall hook should be in plan"
        );

        // pre_uninstall should come before both RemovePackages and DeactivateModule
        if let Some(removal) = removal_idx {
            assert!(
                pre_uninstall_idx.unwrap() < removal,
                "Pre-uninstall hook should come before package removal"
            );
        }
        if let Some(deactivate) = deactivate_idx {
            assert!(
                pre_uninstall_idx.unwrap() < deactivate,
                "Pre-uninstall hook should come before module deactivation"
            );
        }
    }

    #[test]
    fn test_hook_run_hook_not_prunable_comprehensive() {
        // RunHook should not be prunable regardless of hook type
        for hook_type in [
            HookType::PreInstall,
            HookType::PostInstall,
            HookType::PreUninstall,
            HookType::StatusCheck,
        ] {
            let action = ApplyAction::RunHook {
                module_id: "m".into(),
                hook_type,
                command: "echo test".into(),
                behavior: HookBehavior::Always,
            };
            assert!(
                !action.is_prunable(),
                "RunHook {:?} should not be prunable",
                hook_type
            );
        }
    }

    #[test]
    fn test_dotfiles_sync_custom_target_in_resolve() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Module with dotfiles_sync and custom target
        let module_dir = root.join("modules/myapp");
        fs::create_dir_all(module_dir.join("dotfiles")).unwrap();
        fs::write(module_dir.join("dotfiles/config.yml"), "key: value").unwrap();

        fs::write(
            module_dir.join("module.toml"),
            r#"
id = "myapp"
name = "My App"
kind = "AppConfig"
packages = []
aur_packages = []
dotfiles = []
conflicts = []
depends = []
dotfiles_sync = true
dotfiles_sync_target = "~/.local/share/myapp"
"#,
        )
        .unwrap();

        let host: Host = toml::from_str(
            r#"
id = "test"
name = "Test"
installed_bundles = []
extra_modules = ["myapp"]

[variables]

[hardware]
"#,
        )
        .unwrap();

        let desired = resolve_desired_state(root, &host).unwrap();

        // Should use custom target, not default ~/.config/myapp/
        let targets: Vec<&str> = desired.dotfiles.iter().map(|d| d.target.as_str()).collect();
        assert!(
            targets.contains(&"~/.local/share/myapp/config.yml"),
            "Custom target should override default. Got targets: {:?}",
            targets
        );
    }

    #[test]
    fn test_dotfiles_sync_hidden_files_included() {
        // Files starting with . should be discovered
        let tmp = TempDir::new().unwrap();
        let dotfiles_dir = tmp.path().join("dotfiles");
        fs::create_dir_all(&dotfiles_dir).unwrap();
        fs::write(dotfiles_dir.join(".gitignore"), "*.bak").unwrap();
        fs::write(dotfiles_dir.join("visible.conf"), "cfg").unwrap();

        let mappings = discover_dotfiles(&dotfiles_dir, "~/.config/test/");

        assert_eq!(mappings.len(), 2);
        let sources: Vec<&str> = mappings.iter().map(|m| m.source.as_str()).collect();
        assert!(
            sources.contains(&"dotfiles/.gitignore"),
            "Hidden files should be included"
        );
        assert!(sources.contains(&"dotfiles/visible.conf"));
    }

    #[test]
    fn test_dotfiles_sync_false_default() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Module without dotfiles_sync (defaults to false)
        let module_dir = root.join("modules/test-mod");
        fs::create_dir_all(module_dir.join("dotfiles")).unwrap();
        fs::write(module_dir.join("dotfiles/file.txt"), "content").unwrap();
        fs::write(
            module_dir.join("module.toml"),
            r#"
id = "test-mod"
name = "Test Module"
kind = "AppConfig"
packages = []
aur_packages = []
conflicts = []
depends = []
"#,
        )
        .unwrap();

        let host: Host = toml::from_str(
            r#"
id = "test"
name = "Test"
installed_bundles = []
extra_modules = ["test-mod"]

[variables]

[hardware]
"#,
        )
        .unwrap();

        let desired = resolve_desired_state(root, &host).unwrap();

        // No auto-discovered dotfiles (dotfiles_sync defaults false)
        assert!(desired.dotfiles.is_empty());
    }
}
