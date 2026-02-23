# Phase 3 — Technical Implementation Guide

> **Phase:** 3 — Declarative Convergence & Multi-Machine Readiness
> **Status:** PLANNING
> **Depends On:** Phase 2 ✅ Complete (2026-02-22)
> **Companion:** [`phase3-kanban.md`](phase3-kanban.md)
> **Inputs:** [`phase2-gap-analysis.md`](phase2-gap-analysis.md), benchmark analysis (dcli/arch-config), [`pre-phase3-technicals.md`](pre-phase3-technicals.md)
>
> **Note on Phase Numbering:** The original `product-review-and-roadmap.md` defined Phase 3 as "Ecosystem & Community." That scope has been deferred to Phase 4. This Phase 3 addresses foundational gaps identified in the gap analysis and benchmark review. See Decision D7 in the kanban.

---

## 1. Architecture Overview

Phase 3 closes the foundational gaps identified in the gap analysis and incorporates proven patterns from the dcli benchmark ecosystem. The phase transforms Iron from "a system that adds what's missing" to "a system that converges to exactly what's declared."

```
Phase 3 Layer Cake:
┌─────────────────────────────────────────────────┐
│  Sprint 3.4: Multi-Machine Readiness            │  ← Host auto-detect, comparison, portable state
├─────────────────────────────────────────────────┤
│  Sprint 3.3: Execution Lifecycle Completion     │  ← Hooks, history, config namespace, dotfiles_sync
├─────────────────────────────────────────────────┤
│  Sprint 3.2: Full Declarative Convergence       │  ← Managed tracking, template render, removal, risk
├─────────────────────────────────────────────────┤
│  Sprint 3.1: Foundation Contracts               │  ← ActualState, state separation, envelope, status, plan
├─────────────────────────────────────────────────┤
│  Phase 2: Power User Features (DONE)            │  ← Snapshots, CLI output, security
├─────────────────────────────────────────────────┤
│  Phase 1: Apply + Diff + Templates (DONE)       │  ← Declarative convergence (additive)
├─────────────────────────────────────────────────┤
│  Phase 0: Foundation Fixes (DONE)               │  ← UX basics + tech debt
└─────────────────────────────────────────────────┘
```

### Phase 3 Goals

1. **Define `ActualState` as a first-class contract** — single system snapshot, consumed by both plan and drift
2. **Separate runtime state from config directory** — enable clean multi-machine Git sync
3. **Complete the declarative pipeline** — template rendering, file copy, package/service removal, hooks
4. **Add missing CLI commands** — `iron status`, `iron plan`, `iron history`, `iron config`
5. **Standardize machine-readable output** — response envelope for all `--json` commands
6. **Prepare for multi-machine workflows** — hostname auto-detection, host comparison, portable state

### Gap Coverage Matrix

| Gap ID | Description | Sprint | Task(s) |
|--------|-------------|--------|---------|
| **C1** | No template variable rendering in apply | 3.2 | F3-008 |
| **C2** | No formal ActualState contract | 3.1 | F3-001, F3-002a, F3-002b |
| **C3** | No response envelope for --json | 3.1 | F3-003a, F3-003b |
| **C4** | Config format (TOML vs Lua) — deferred | — | Decision D1: stay TOML |
| **C5** | No `iron plan` command | 3.1 | F3-005 |
| **S1** | No `iron status` command | 3.1 | F3-004 |
| **S2** | No `iron config` namespace | 3.3 | F3-017 (STRETCH) |
| **S3** | No `iron history` command | 3.3 | F3-016 |
| **S4** | Runtime state in Git repo | 3.1 | F3-006, F3-007 |
| **S5** | No file copy deployment | 3.2 | F3-009 |
| **S6** | No risk levels on actions | 3.2 | F3-013 |
| **S7** | Module hooks never executed | 3.3 | F3-014, F3-015 |
| **S8** | Apply is additive-only | 3.2 | F3-010, F3-011, F3-012 |
| **BM-1** | dotfiles_sync pattern (from dcli) | 3.3 | F3-018 |
| **BM-2** | Managed resource tracking (from dcli) | 3.2 | F3-021 |
| **BM-3** | Hostname auto-detection (from dcli) | 3.4 | F3-019 |
| **BM-4** | Host comparison (from dcli) | 3.4 | F3-020 (STRETCH) |

### New Files Created in Phase 3

```
iron-core/src/actual_state.rs              ← ActualState struct + scan_actual_state()
iron-core/src/envelope.rs                  ← IronEnvelope<T> response wrapper
iron-core/src/services/history.rs          ← HistoryService for operation history
iron-cli/src/commands/status.rs            ← CLI iron status command
iron-cli/src/commands/plan.rs              ← CLI iron plan command
iron-cli/src/commands/history.rs           ← CLI iron history command
iron-cli/src/commands/config.rs            ← CLI iron config namespace (STRETCH)
```

### Modified Files

```
iron-core/src/services/apply.rs            ← ActualState consumption, template rendering, file copy,
                                              removal actions, hooks, risk levels, managed tracking
iron-core/src/services/drift.rs            ← Consume ActualState instead of ad-hoc queries
iron-core/src/services/state.rs            ← XDG state directory, managed_*, hooks_executed fields
iron-core/src/services/mod.rs              ← Register new services
iron-core/src/module.rs                    ← dotfiles_sync, hook_behavior fields
iron-core/src/host.rs                      ← hostname field for auto-detection
iron-cli/src/cli.rs                        ← New commands registration
iron-cli/src/commands/mod.rs               ← New command modules
iron-cli/src/commands/apply.rs             ← --prune flags, ActualState scan, risk confirmation
iron-cli/src/commands/diff.rs              ← ActualState scan
iron-cli/src/commands/snapshot.rs          ← ActualState scan for restore
iron-cli/src/commands/security.rs          ← Envelope migration
iron-cli/src/commands/scan.rs              ← --save flag, envelope migration
iron-cli/src/main.rs                       ← Wire new commands
iron-cli/src/output.rs                     ← json_envelope(), json_error_envelope()
iron-cli/src/context.rs                    ← XDG path resolution
iron-fs/src/lib.rs                         ← has_variables(), file copy with backup
iron-tui/src/ui/apply.rs                   ← New action types rendering, risk badges
iron-tui/src/app/actions.rs                ← ActualState scan before plan/drift
```

---

## 2. Sprint 3.1: Foundation Contracts

### 2.1 ActualState Contract (F3-001)

```rust
// iron-core/src/actual_state.rs

use serde::{Deserialize, Serialize};

/// Represents the current state of the system as queried from real sources.
/// This is the counterpart to DesiredState — the plan is their diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualState {
    /// Hostname of the machine
    pub hostname: String,

    /// Explicitly installed packages (pacman -Qqe)
    pub installed_packages: Vec<String>,

    /// AUR packages (pacman -Qqm)
    #[serde(default)]
    pub aur_packages: Vec<String>,

    /// Enabled systemd services with their current state
    #[serde(default)]
    pub services: Vec<ActualServiceState>,

    /// State of managed dotfiles/config files
    #[serde(default)]
    pub managed_files: Vec<ActualFileState>,

    /// When this state was captured
    pub scanned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualServiceState {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualFileState {
    /// The target path (e.g., ~/.config/nvim/init.lua)
    pub target: String,
    /// Whether the file/symlink exists
    pub exists: bool,
    /// If symlink, where it points
    #[serde(default)]
    pub symlink_target: Option<String>,
    /// SHA256 of the file content (if exists and is a regular file)
    #[serde(default)]
    pub checksum: Option<String>,
    /// File type: Symlink, Regular, Missing
    pub file_type: FileStateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStateType {
    Symlink,
    Regular,
    Missing,
    Directory,
}
```

### 2.2 scan_actual_state() Implementation (F3-002a)

```rust
// iron-core/src/actual_state.rs

impl ActualState {
    /// Scan the system once and capture all relevant state.
    /// Both ApplyService::compute_plan() and DriftService::detect()
    /// should call this instead of querying independently.
    pub fn scan(
        package_manager: &dyn PackageManager,
        service_manager: &dyn SystemService,
        managed_files: &[ManagedFileSpec],
    ) -> IronResult<Self> {
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        let installed_packages = package_manager.query_installed()?
            .iter()
            .map(|p| p.name.clone())
            .collect();

        let aur_packages = package_manager.query_aur_installed()
            .unwrap_or_default()
            .iter()
            .map(|p| p.name.clone())
            .collect();

        let services = Self::scan_services(service_manager, managed_files)?;
        let files = Self::scan_files(managed_files)?;

        Ok(Self {
            hostname,
            installed_packages,
            aur_packages,
            services,
            managed_files: files,
            scanned_at: chrono::Utc::now(),
        })
    }
}
```

### 2.3 Consumer Refactor (F3-002b)

**Key architectural change:** Both `ApplyService::compute_plan()` and `DriftService::detect()` must be refactored to accept `&ActualState` instead of querying the system independently.

**Before (current):**
```rust
// apply.rs — queries system internally
fn compute_plan(&self, desired: &DesiredState, state: &StateManager) -> IronResult<ApplyPlan> {
    let installed = self.package_manager.query_installed()?;  // ad-hoc query
    let is_enabled = self.service_manager.is_enabled(&svc)?;  // ad-hoc query
    // ...
}
```

**After (refactored):**
```rust
// apply.rs — receives pre-scanned state
fn compute_plan(&self, desired: &DesiredState, actual: &ActualState, state: &StateManager) -> IronResult<ApplyPlan> {
    // Use actual.installed_packages instead of querying
    // Use actual.services instead of querying
    // Use actual.managed_files instead of querying
}
```

**Cascade files that need updating:**
1. `iron-core/src/services/apply.rs` — `compute_plan()` signature
2. `iron-core/src/services/drift.rs` — `detect()` signature
3. `iron-cli/src/commands/apply.rs` — scan ActualState, pass to compute_plan
4. `iron-cli/src/commands/diff.rs` — scan ActualState, pass to detect
5. `iron-cli/src/commands/snapshot.rs` — restore flow calls compute_plan
6. `iron-tui/src/ui/apply.rs` — TUI apply view
7. `iron-tui/src/app/actions.rs` — TUI action dispatchers
8. All integration and unit tests that call these functions

### 2.4 Response Envelope (F3-003a)

```rust
// iron-core/src/envelope.rs

use serde::Serialize;
use chrono::{DateTime, Utc};

/// Standard response envelope for all --json output.
/// Matches the original technical spec (Section 6.1).
#[derive(Debug, Serialize)]
pub struct IronEnvelope<T: Serialize> {
    pub ok: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<EnvelopeError>,
    pub meta: EnvelopeMeta,
}

#[derive(Debug, Serialize)]
pub struct EnvelopeError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct EnvelopeMeta {
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub host: Option<String>,
    pub version: String,
}

impl<T: Serialize> IronEnvelope<T> {
    pub fn success(command: &str, data: T, duration_ms: u64) -> Self { ... }
}

impl IronEnvelope<()> {
    pub fn error(command: &str, err: &IronError, duration_ms: u64) -> Self { ... }
}
```

**Integration pattern (F3-003a adds these methods):**

```rust
// iron-cli/src/output.rs
impl Output {
    /// Wrap data in a standard envelope for --json output.
    pub fn json_envelope<T: Serialize>(
        &self,
        command: &str,
        data: T,
        start_time: std::time::Instant,
    ) {
        let envelope = IronEnvelope::success(
            command,
            data,
            start_time.elapsed().as_millis() as u64,
        );
        self.json_value(&envelope);
    }

    /// Wrap an error in a standard envelope for --json output.
    pub fn json_error_envelope(
        &self,
        command: &str,
        err: &IronError,
        start_time: std::time::Instant,
    ) {
        let envelope = IronEnvelope::<()>::error(
            command,
            err,
            start_time.elapsed().as_millis() as u64,
        );
        self.json_value(&envelope);
    }
}
```

**Envelope migration (F3-003b):** All existing `--json` outputs in apply, diff, snapshot, security, scan, module, and validate commands must be migrated to use `json_envelope()` instead of raw `json_value()`. Each command file needs a `let start_time = std::time::Instant::now();` at the top and `output.json_envelope("command_name", data, start_time)` at output points.

### 2.5 `iron status` Command (F3-004)

```
$ iron status

Iron — System Status
────────────────────
Host:       desktop (auto-detected)
Bundle:     hyprland (active)
Profile:    developer
Modules:    14 enabled, 2 disabled

Packages:   142 managed
Services:   8 managed
Dotfiles:   23 managed

Security:   Standard (45/100 pts)
Last apply: 2h ago
Last sync:  1d ago (3 commits ahead)

Drift:      ⚠ 4 items — run `iron diff` for details
```

**Performance design:** `iron status` must respond in < 2 seconds. It uses:
- `StateManager` data for host/bundle/profile/modules (instant — file read)
- `State.managed_packages.len()` for package count (instant — already in state)
- `SecurityService::calculate()` for security level (fast — reads module files)
- `SyncService` for sync status (fast — git status)
- Existing divergence check for drift indicator (existing TUI logic)

It does **NOT** call `ActualState::scan()` by default. The `--full` flag triggers a full scan for accurate "missing/extra" counts. Without `--full`, it shows managed counts from state and a drift indicator from cached divergence data.

### 2.6 `iron plan` Command (F3-005)

```rust
// iron-cli/src/commands/plan.rs

/// `iron plan` — Generate and display an apply plan without executing.
/// Read-only: no confirmation prompt, no side effects.
///
/// Usage:
///   iron plan                    # Full system plan
///   iron plan --module nvim-ide  # Plan for one module
///   iron plan --json             # Machine-readable plan in envelope
pub fn execute(ctx: &CliContext, args: &PlanArgs) -> IronResult<()> {
    let actual = ActualState::scan(...)?;
    let desired = resolve_desired_state(...)?;
    let plan = apply_service.compute_plan(&desired, &actual, &state)?;

    output.plan_summary(&plan);  // tree-style output with risk badges
    Ok(())
}
```

**Difference from `iron apply --dry-run`:**
- `iron plan` is purely read-only — never prompts for confirmation
- `iron apply --dry-run` shows the plan AND asks "would you like to proceed?"

**Scope boundary:** Plan serialization (`--output plan.json`) and plan replay (`iron apply --plan <file>`) are deferred to Phase 4. The `ApplyPlan` contains trait references that require a serialization strategy and staleness detection.

### 2.7 State Directory Separation (F3-006, F3-007)

**Current layout (problematic):**
```
~/.config/iron/           ← Git-tracked (user config)
├── hosts/
├── bundles/
├── modules/
├── profiles/
├── state.json            ← PROBLEM: machine-specific runtime data in Git
├── audit.log             ← PROBLEM: machine-specific log in Git
├── .state.lock           ← PROBLEM: lock file in Git
└── .snapshots/           ← PROBLEM: machine-specific snapshots in Git
```

**New layout:**
```
~/.config/iron/           ← Git-tracked (user declarations ONLY)
├── hosts/
├── bundles/
├── modules/
├── profiles/
└── files/                ← Template source files

~/.local/state/iron/      ← Machine-specific runtime (NOT Git-tracked)
├── state.json
├── audit.log
├── .state.lock
├── snapshots/
│   ├── snap-abc123.json
│   └── ...
├── history/              ← Future: operation history artifacts
└── scans/                ← Saved ActualState snapshots (F3-022)
```

**State directory resolution (F3-006):**

```rust
impl StateManager {
    /// Resolve the state directory path.
    /// Priority: $IRON_STATE_DIR > $XDG_STATE_HOME/iron > ~/.local/state/iron
    pub fn state_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("IRON_STATE_DIR") {
            return PathBuf::from(dir);
        }
        if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            return PathBuf::from(xdg).join("iron");
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local/state/iron")
    }
}
```

**Dependency:** Check if `dirs` crate is already in the workspace. If not, add `dirs = "5"` to `iron-core/Cargo.toml`. Alternatively, use manual `$XDG_STATE_HOME` resolution as shown above (avoids new dependency).

**Legacy migration (F3-007) — copy-then-delete strategy:**

```rust
impl StateManager {
    /// On startup, check if state.json exists in the config dir (legacy).
    /// If so, copy to new state dir, verify, then delete originals.
    pub fn migrate_if_needed(config_root: &Path) -> IronResult<()> {
        let legacy_state = config_root.join("state.json");
        let new_state_dir = Self::state_dir();
        let new_state = new_state_dir.join("state.json");

        if legacy_state.exists() && !new_state.exists() {
            std::fs::create_dir_all(&new_state_dir)?;

            // Copy (not move) for safety
            std::fs::copy(&legacy_state, &new_state)?;

            // Verify copy succeeded
            if new_state.exists() {
                std::fs::remove_file(&legacy_state)?;

                // Leave breadcrumb
                std::fs::write(
                    config_root.join("MIGRATED.txt"),
                    format!("State migrated to {} on {}", new_state_dir.display(), chrono::Utc::now()),
                )?;
            }

            // Also migrate audit.log, .state.lock, .snapshots/
            Self::migrate_file(config_root, &new_state_dir, "audit.log")?;
            Self::migrate_file(config_root, &new_state_dir, ".state.lock")?;
            Self::migrate_dir(config_root, &new_state_dir, ".snapshots", "snapshots")?;
        }
        Ok(())
    }
}
```

**Failure recovery:** If migration fails (copy succeeds but delete fails, or copy itself fails), the original files remain intact. The next startup will detect the legacy files again and retry. No data loss scenario.

---

## 3. Sprint 3.2: Full Declarative Convergence

### 3.0 Managed Resource Tracking (F3-021) — MUST BE FIRST

**This task MUST be implemented before F3-010/011/012.** The removal tasks consume `State.managed_packages/services/dotfiles` data that this task creates.

Add to `state.json`:
```rust
pub struct State {
    // ...existing fields...

    /// Packages installed by Iron (via apply/module enable).
    /// Only packages in this set are candidates for removal.
    #[serde(default)]
    pub managed_packages: Vec<String>,

    /// Services enabled by Iron.
    #[serde(default)]
    pub managed_services: Vec<String>,

    /// Dotfile target paths created by Iron.
    #[serde(default)]
    pub managed_dotfiles: Vec<String>,
}
```

**Recording logic (integrated into existing apply execution):**
```rust
// After executing InstallPackages action successfully:
fn record_managed_packages(state: &mut StateManager, packages: &[String]) {
    for pkg in packages {
        if !state.managed_packages.contains(pkg) {
            state.managed_packages.push(pkg.clone());
        }
    }
    state.save()?;
}
```

**Bootstrap:** For existing installations, `managed_packages` starts empty. On first `iron apply` after upgrade, all installed packages that match the desired state are recorded as managed. This avoids destructive assumptions about pre-existing packages.

### 3.1 Template Variable Rendering in Apply (F3-008)

The template engine already exists in `iron-fs` (`{{variable}}` substitution). The gap is that it's never invoked during the apply flow. This task wires it in.

**Decision: Rendered copy vs. symlink**

When a dotfile contains `{{variable}}` patterns:
1. Render the template with host variables
2. Deploy the rendered file as a **copy** (not a symlink — symlinks to rendered files would break on re-render)

When a dotfile does NOT contain template patterns:
- Continue using symlinks (existing behavior)

```rust
// In ApplyService::compute_plan()
fn plan_dotfile_action(
    mapping: &DotfileMapping,
    variables: &HashMap<String, String>,
    actual: &ActualFileState,
) -> ApplyAction {
    let source_content = std::fs::read_to_string(&mapping.source)?;
    let has_templates = iron_fs::template::has_variables(&source_content);

    if has_templates {
        ApplyAction::RenderAndCopy {
            source: mapping.source.clone(),
            target: mapping.target.clone(),
            variables: variables.clone(),
            backup_existing: true,
        }
    } else if mapping.link {
        ApplyAction::CreateSymlink { ... }
    } else {
        ApplyAction::CopyFile { ... }
    }
}
```

### 3.2 File Copy Deployment Mode (F3-009)

Add `CopyFile` and `RenderAndCopy` action types to `ApplyAction`:

```rust
pub enum ApplyAction {
    // Existing
    InstallPackages { packages: Vec<String> },
    InstallAurPackages { packages: Vec<String> },
    CreateSymlink { source: PathBuf, target: PathBuf },
    EnableService { name: String },
    ActivateModule { id: String },

    // New in Phase 3
    CopyFile {
        source: PathBuf,
        target: PathBuf,
        backup_existing: bool,
    },
    RenderAndCopy {
        source: PathBuf,
        target: PathBuf,
        variables: HashMap<String, String>,
        backup_existing: bool,
    },
    RemovePackages {
        packages: Vec<String>,
    },
    RemoveSymlink {
        target: PathBuf,
    },
    DisableService {
        name: String,
    },
    DeactivateModule {
        id: String,
    },
    RunHook {
        module_id: String,
        hook_type: HookType,
        command: String,
        behavior: HookBehavior,
    },
}
```

### 3.3 Package Removal in Apply (F3-010)

**Depends on:** F3-021 (managed resource tracking)

```rust
// In compute_plan(), after determining desired packages:
let desired_set: HashSet<&str> = desired.packages.iter().map(|s| s.as_str()).collect();
let managed_set: HashSet<&str> = state.managed_packages.iter().map(|s| s.as_str()).collect();
let installed_set: HashSet<&str> = actual.installed_packages.iter().map(|s| s.as_str()).collect();

// Packages to install: desired but not installed
let to_install: Vec<_> = desired_set.difference(&installed_set).collect();

// Packages to remove: managed by Iron, installed, but no longer desired
let to_remove: Vec<_> = managed_set
    .intersection(&installed_set)
    .filter(|p| !desired_set.contains(*p))
    .collect();

if !to_remove.is_empty() {
    plan.actions.push(ApplyAction::RemovePackages {
        packages: to_remove.into_iter().map(|s| s.to_string()).collect(),
    });
}
```

**Safety:** Removal is opt-in. Without a prune flag, removal actions are shown in the plan but skipped during execution, with a hint: "Run `iron apply --prune` to remove 3 packages no longer in your declared state."

**Prune flags (Decision D2a):**
- `--prune` — prune all resource types (packages, services, dotfiles)
- `--prune-packages` — prune packages only
- `--prune-services` — prune services only
- `--prune-dotfiles` — prune dotfiles/symlinks only

### 3.4 Service Disable & Symlink Removal (F3-011, F3-012)

Follow the same pattern as package removal. Both depend on F3-021 for `managed_services` and `managed_dotfiles` data.

### 3.5 Risk Levels on Actions (F3-013)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    ReadOnly,     // No system changes
    Additive,     // Adds to system, easily reversible
    Destructive,  // Modifies existing state
    Critical,     // Potentially dangerous
}

impl ApplyAction {
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Self::InstallPackages { .. } => RiskLevel::Additive,
            Self::InstallAurPackages { .. } => RiskLevel::Additive,
            Self::CreateSymlink { .. } => RiskLevel::Additive,
            Self::EnableService { .. } => RiskLevel::Additive,
            Self::ActivateModule { .. } => RiskLevel::Additive,
            Self::CopyFile { backup_existing: true, .. } => RiskLevel::Destructive,
            Self::CopyFile { backup_existing: false, .. } => RiskLevel::Additive,
            Self::RenderAndCopy { .. } => RiskLevel::Destructive,
            Self::RemovePackages { .. } => RiskLevel::Critical,
            Self::RemoveSymlink { .. } => RiskLevel::Destructive,
            Self::DisableService { .. } => RiskLevel::Destructive,
            Self::DeactivateModule { .. } => RiskLevel::Destructive,
            Self::RunHook { .. } => RiskLevel::Destructive,
        }
    }
}

impl ApplyPlan {
    pub fn max_risk(&self) -> RiskLevel {
        self.actions.iter()
            .map(|a| a.risk_level())
            .max()
            .unwrap_or(RiskLevel::ReadOnly)
    }
}
```

**Confirmation policy (CLI):**
```rust
match plan.max_risk() {
    RiskLevel::ReadOnly => { /* no confirmation */ },
    RiskLevel::Additive => { output.confirm("Proceed?") },
    RiskLevel::Destructive => { output.confirm_with_details("Review changes above. Proceed?") },
    RiskLevel::Critical => { output.typed_confirm("Type 'yes' to confirm critical changes:") },
}
```

**TUI Impact:** Apply view should color-code actions: green for Additive, yellow for Destructive, red for Critical. Dashboard drift indicator should show max risk level.

---

## 4. Sprint 3.3: Execution Lifecycle Completion

### 4.1 Hook Execution in Apply (F3-014)

Wire the existing `pre_install`, `post_install`, `pre_uninstall`, `status_check` fields on `Module` into the apply lifecycle.

**Hook behavior model** (inspired by dcli):

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum HookBehavior {
    #[default]
    Always,  // Run every time the module is applied
    Once,    // Run only the first time (track in state.json)
    Ask,     // Prompt the user before running
    Skip,    // Never run
}
```

**Execution order** (follows dcli pattern + original spec Section 10):
1. Backup files
2. Install packages
3. Remove packages (if `--prune`)
4. Create symlinks / Copy files / Render templates
5. Enable services / Disable services
6. Run post-install hooks
7. Record results

**Hook tracking (F3-015):** Add `hooks_executed: HashMap<String, Vec<String>>` to `State` with `#[serde(default)]`. The `--force-hooks` flag re-runs `Once` hooks.

### 4.2 `iron history` Command (F3-016)

```
$ iron history
Operation History
─────────────────
#  Time          Command    Duration  Actions  Status
1  2h ago        apply      4.2s      12       ✓ Success
2  1d ago        update     45.1s     28       ✓ Success
3  2d ago        apply      2.1s      5        ✗ Partial (2 failed)

$ iron history show 3
Operation #3 — apply (2d ago)
─────────────────────────────
Status: Partial (3/5 succeeded, 2 failed)

Actions:
  ✓ Install packages: neovim, ripgrep
  ✓ Create symlink: ~/.config/nvim → modules/nvim-ide/config
  ✗ Create symlink: ~/.config/fish (permission denied)

Errors:
  • ~/.config/fish: Permission denied — try `sudo chown $USER ~/.config/fish`
```

**Implementation:** Uses existing `StateManager.last_operations` and `AuditLog` data. The `HistoryService` reads from `~/.local/state/iron/audit.log` (JSONL format, respecting F3-006 XDG path).

### 4.3 `dotfiles_sync` Mode (F3-018)

```toml
# modules/nvim-ide/module.toml
id = "nvim-ide"
name = "Neovim IDE"
packages = ["neovim", "ripgrep"]
dotfiles_sync = true
dotfiles_sync_target = "~/.config/nvim"  # Override default (which would be ~/.config/nvim-ide/)
```

**Important:** The default target `~/.config/<module-id>/` may not match XDG convention. Module IDs like `nvim-ide` map to `~/.config/nvim-ide/` but the real XDG dir is `~/.config/nvim/`. Users should set `dotfiles_sync_target` explicitly when the ID differs from the config dir. A warning is logged when using the default target and the module ID contains hyphens.

---

## 5. Sprint 3.4: Multi-Machine Readiness

### 5.1 Hostname Auto-Detection (F3-019)

```rust
impl HostService {
    pub fn detect_host(&self, config_root: &Path) -> IronResult<Option<Host>> {
        let system_hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        let hosts = self.list_hosts(config_root)?;
        for host in &hosts {
            // Priority: hostname field exact match > id field match
            if host.hostname.as_deref() == Some(&system_hostname)
                || host.id == system_hostname
            {
                return Ok(Some(host.clone()));
            }
        }
        Ok(None)
    }
}
```

**Integration:** On startup, if no `--host` flag and no host in state, attempt auto-detection. If exactly one match, use it. If none or multiple, fall back to interactive selection.

### 5.2 Host Comparison (F3-020 — STRETCH)

Load both host definitions, resolve their `DesiredState`, diff the results. Shows bundle/profile/module/package/variable differences.

### 5.3 ActualState Serialization (F3-022 — STRETCH)

`iron scan --save` saves to `~/.local/state/iron/scans/<timestamp>.json`. The `ActualState` struct is already `Serialize + Deserialize`.

---

## 6. Testing Strategy

### Unit Tests Per Feature

| Feature | Test Count | Key Tests |
|---------|-----------|-----------|
| F3-001 | 6+ | ActualState construction, serialization roundtrip, deserialization with missing fields |
| F3-002a | 8+ | Scan with mocked PackageManager/SystemService, file state detection, hostname |
| F3-002b | 6+ | compute_plan accepts ActualState, detect accepts ActualState, no ad-hoc queries |
| F3-003a | 6+ | Envelope success/error, serialization, timestamp, duration, version |
| F3-003b | 4+ | At least 3 commands verified with envelope output |
| F3-004 | 4+ | Status output modes (text/json/full), empty state, no host |
| F3-005 | 4+ | Plan output, module filter, envelope format |
| F3-006/007 | 8+ | State dir resolution (env > XDG > default), migration (copy+delete), failure recovery, no-op |
| F3-021 | 6+ | Record packages/services/dotfiles, unrecord, bootstrap, persistence |
| F3-008 | 8+ | Template detection, rendering, render+copy, built-in variables |
| F3-009 | 4+ | Copy action planning, execution with/without backup |
| F3-010 | 6+ | Removal planning, managed tracking, safety (no remove unmanaged), prune flags |
| F3-011/012 | 4+ | Service disable plan, symlink removal plan, managed tracking |
| F3-013 | 4+ | Risk classification for each action type, max_risk, PartialOrd |
| F3-014/015 | 6+ | Hook planning (always/once/ask/skip), hook tracking, force-hooks |
| F3-016 | 4+ | History list/show, JSONL parsing, empty history |
| F3-018 | 5+ | dotfiles_sync discovery, explicit override, nested dirs, custom target, warning |
| F3-019 | 4+ | Hostname detection, id match, no match, multiple match fallback |

### Integration Tests

All CLI integration tests must use `--dry-run` (established convention).

```bash
iron status --dry-run                   # exits 0
iron plan --dry-run                     # exits 0
iron plan --json --dry-run              # valid envelope JSON
iron history list                       # exits 0 (read-only, no --dry-run needed)
iron config path                        # exits 0, prints path
iron config validate --dry-run          # exits 0
```

### Anti-Patterns to Avoid

1. **Never** query pacman in tests (always mock via `PackageManager` trait)
2. **Never** write to `~/.local/state/` in tests (use temp dirs via `$IRON_STATE_DIR`)
3. **Never** remove real packages in tests (mock `RemovePackages` execution)
4. **Always** use `--dry-run` for CLI integration tests
5. **Always** add `#[serde(default)]` on all new struct fields
6. **Always** update ALL test helpers when adding struct fields
7. **Always** update ALL View match arms if adding TUI views (7 locations)
8. **Always** list all affected files when changing trait signatures (lesson from F3-002b)

---

## 7. Implementation Order

### Sprint 3.1 (Foundation Contracts — 9 tasks):
1. **F3-001** (ActualState struct) ← foundation, everything depends on this
2. **F3-002a** (scan_actual_state) ← implementation of the scan
3. **F3-003a** (Response envelope infra) ← can parallel with F3-002a
4. **F3-006** (State dir resolution) ← can parallel with above
5. **F3-007** (Legacy state migration) ← depends on F3-006
6. **F3-002b** (Refactor consumers) ← depends on F3-002a, cascading change
7. **F3-003b** (Migrate existing --json) ← depends on F3-003a, incremental
8. **F3-004** (iron status) ← depends on F3-002b, F3-003a
9. **F3-005** (iron plan) ← depends on F3-002b, F3-003a

### Sprint 3.2 (Declarative Convergence — 8 tasks):
1. **F3-021** (Managed resource tracking) ← **MUST be first** — prerequisite for F3-010/011/012
2. **F3-008** (Template rendering in apply) ← can parallel with F3-021
3. **F3-009** (File copy deployment) ← pairs with F3-008
4. **F3-010** (Package removal) ← depends on F3-021
5. **F3-011** (Service disable) ← depends on F3-021, can parallel with F3-010
6. **F3-012** (Symlink removal) ← depends on F3-021, can parallel with F3-010
7. **F3-013** (Risk levels) ← classifies all new action types, done last

### Sprint 3.3 (Lifecycle Completion — 5 tasks):
1. **F3-014** (Hook execution) ← new action type + lifecycle integration
2. **F3-015** (Hook tracking) ← state persistence for Once behavior
3. **F3-016** (iron history) ← CLI command, reads existing audit data
4. **F3-018** (dotfiles_sync) ← module field + plan computation
5. **F3-017** (iron config namespace) ← STRETCH, mostly delegates

### Sprint 3.4 (Multi-Machine — 3 tasks):
1. **F3-019** (Hostname auto-detection) ← Host struct change + detect logic
2. **F3-020** (Host comparison) ← STRETCH, depends on DesiredState resolver
3. **F3-022** (ActualState serialization) ← STRETCH, CLI integration for scan export

---

## 8. Dependency Graph

```
F3-001 → F3-002a → F3-002b ──→ F3-004, F3-005
                                      ↑
F3-003a → F3-003b ────────────────────┘
F3-006 → F3-007

F3-021 ──→ F3-010  (managed tracking MUST precede removal)
       ├─→ F3-011
       └─→ F3-012
F3-008 → F3-009  (template → copy)
F3-013 (depends on F3-008, F3-009, F3-010, F3-011, F3-012)

F3-014 → F3-015  (hooks → tracking)
F3-016, F3-018  (independent)
F3-017 (STRETCH, independent)

F3-019 (independent)
F3-020 (STRETCH, independent)
F3-022 (STRETCH, depends on F3-001)
```

---

## 9. Decision Log

### D1: Stay with TOML, Defer Lua/Scripting (GAP-C4)

**Decision:** Do not adopt Lua scripting in Phase 3. TOML remains the config format.

**Rationale:** The benchmark analysis shows dcli uses Lua extensively, but Iron-Arch's TOML + template variables covers 90% of practical use cases. Lua adds significant complexity (vendored runtime, security sandbox, debugging). If conditional module inclusion becomes necessary, add a `[conditions]` table to host.toml rather than a full scripting engine.

**Revisit when:** Users need conditional logic that can't be expressed with host variables and `extra_modules`.

### D2: Removal is Opt-In (`--prune`)

**Decision:** Package/service/symlink removal requires explicit `--prune` flag.

**Rationale:** dcli's `auto_prune` removes packages not in any module. This is powerful but dangerous for users who install packages manually. Iron should show removable items in the plan but only execute removal with `--prune`. This balances convergence with safety.

### D2a: Granular Prune Flags

**Decision:** In addition to `--prune` (all types), support `--prune-packages`, `--prune-services`, `--prune-dotfiles` for targeted pruning.

**Rationale:** Users may want to prune orphaned packages but keep manually-enabled services. All-or-nothing pruning is too coarse for real workflows.

### D3: Hooks are Shell Commands, Not Scripts

**Decision:** Module hooks are shell command strings, not embedded Lua/script files.

**Rationale:** Keeps hooks simple and debuggable. dcli's hook system uses shell commands with behavior modifiers (always/once/ask/skip). This is sufficient for post-install actions like `systemctl enable`, `gpasswd -a`, `nvim +PlugInstall`.

### D4: State Separation Uses XDG

**Decision:** Runtime state moves to `$XDG_STATE_HOME/iron/` (typically `~/.local/state/iron/`).

**Rationale:** Follows XDG Base Directory Specification. dcli and arch-config both have this separation problem — learning from their experience.

### D5: ActualState is a Single Scan

**Decision:** One `ActualState::scan()` call replaces all ad-hoc system queries in compute_plan and detect.

**Rationale:** Current code queries pacman, systemctl, and filesystem independently in both `compute_plan()` and `detect()`. This can produce inconsistent views. A single scan ensures consistency and enables serialization for history/comparison.

### D6: `iron plan` is Display-Only (No Serialization)

**Decision:** `iron plan` does not support `--output` or `--plan` in Phase 3.

**Rationale:** Plan serialization requires: (a) making `ApplyPlan` serializable (currently contains trait references), (b) staleness detection (system may change between plan and apply), (c) security review of replaying saved plans. This complexity is Phase 4 scope.

### D7: Original Roadmap Phase 3 Deferred to Phase 4

**Decision:** The `product-review-and-roadmap.md` Phase 3 scope (Ecosystem & Community: import/export/registry/remote-apply) is deferred to Phase 4.

**Rationale:** Foundational gaps (ActualState, state separation, removal, hooks, template rendering) must be resolved first. Ecosystem features built on an incomplete foundation would be fragile and hard to maintain. Phase 3 task IDs (F3-XXX) refer to this plan, not the original roadmap. The roadmap document should be updated to reflect this change.
