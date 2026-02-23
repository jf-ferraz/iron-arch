# Phase 2 Gap Analysis: Original Vision vs. Current Implementation

> **Date:** 2026-02-22
> **Purpose:** Deep-dive comparison of the founder's original vision documents (pre-phase3-concerns.md, pre-phase3-reasoning.md, pre-phase3-technicals.md) against the actual codebase at Phase 2 completion. Identifies gaps, divergences, architectural concerns, and opportunities before proceeding to Phase 3.
> **Scope:** Data contracts, config layout, user interaction, safety model, execution lifecycle, module system, and overall architecture.

---

## Executive Summary

The Iron-Arch project has achieved an impressive amount of functionality (~69,000 LOC, 2,033 tests, 7 crates, 28 TUI views) but has **diverged significantly from the original technical specification** in several foundational areas. While the divergences are not necessarily wrong — many represent pragmatic decisions that accelerated development — they have created gaps that **must be addressed before Phase 3** to prevent architectural debt from compounding.

### Verdict: 5 Critical Gaps, 8 Significant Gaps, 6 Observations

| Severity | Count | Nature |
|----------|-------|--------|
| **CRITICAL** | 5 | Foundational misalignments that affect the product's core identity |
| **SIGNIFICANT** | 8 | Important gaps that limit capability or create tech debt |
| **OBSERVATION** | 6 | Divergences that are acceptable but should be documented |

---

## 1. CRITICAL GAPS

### GAP-C1: No Runtime Template Variable Substitution

**Original Vision** (pre-phase3-technicals.md, Section 7.3):
> "optional template variable substitution" for dotfile management
> `vars/` directory with shared and host-specific variables
> Variables resolved during composition of DesiredState

**What Was Built:**
- `Host.variables: HashMap<String, String>` exists (F1-001)
- `DesiredState.variables` is populated during resolution (apply.rs:135-145)
- A template engine exists in iron-fs: `{{variable}}` substitution with whitespace trimming
- **BUT**: The template engine is never invoked during the apply flow. `CreateSymlink` actions create raw symlinks to source files — they do NOT process templates.

**Impact:**
- Dotfiles cannot be customized per-host (the #1 use case for variables)
- The `host.variables` field and `DesiredState.variables` are dead data in the apply pipeline
- A user declaring `variables = { terminal = "kitty" }` in host.toml has no way to use those values in their configs
- This undermines Pillar A ("Canonical State Management") — the source of truth cannot express per-machine customization

**What's Needed:**
- During `CreateSymlink` action execution, if the source file contains `{{variable}}` patterns, render through the template engine before deploying
- Decision: Deploy rendered copy vs. symlink to rendered output
- The current symlink-only model won't work for templated files — need a "render + copy" action type or a "render → staging dir → symlink" flow

---

### GAP-C2: No Formal `ActualState` Contract

**Original Vision** (pre-phase3-technicals.md, Section 6.3):
> ActualState: `host_detected`, `packages_installed`, `services_state`, `files_state`, `scan_warnings`, `metadata`
> "Represents what is currently on the machine"

**What Was Built:**
- There is NO `ActualState` struct anywhere in the codebase
- Instead, actual state is queried ad-hoc inside `compute_plan()` (apply.rs:583-683):
  - `package_manager.query_installed()` returns `Vec<InstalledPackage>`
  - Symlink checks are done inline with `std::fs::read_link()`
  - Service checks done inline with `service_manager.is_enabled()`
- Similarly, `DriftService::detect()` (drift.rs:142-296) re-queries the same data independently

**Impact:**
- **No single snapshot of system reality** — each service queries independently, potentially getting inconsistent views
- `compute_plan()` and `detect()` duplicate system scanning logic
- No way to serialize/compare actual states over time (a key requirement for drift trending)
- Phase 3 multi-machine features need a portable `ActualState` to compare across hosts
- The original spec called for `ActualState` as a first-class contract alongside `DesiredState` — this is fundamental to the declarative model

**What's Needed:**
- Define `ActualState` struct with: installed_packages, enabled_services, managed_file_states (exists/missing/wrong-target/modified)
- Single `scan_actual_state()` function that queries everything once
- Both `compute_plan()` and `detect()` consume `ActualState` rather than querying independently
- Serializable for history/comparison

---

### GAP-C3: No Response Envelope for Machine-Readable Output

**Original Vision** (pre-phase3-technicals.md, Section 6.1):
> Envelope contract for all `--json` output:
> ```json
> { "ok": true, "command": "plan", "data": {}, "error": null, "meta": { "timestamp", "duration_ms", "host" } }
> ```

**What Was Built:**
- Each command emits ad-hoc JSON via `Output::json_value()` and `Output::raw()`
- No consistent envelope wrapping
- Error output varies: sometimes `{"status":"error","message":"..."}`, sometimes raw text to stderr
- No `meta` field (timestamp, duration, host) on any JSON output
- No `command` field identifying which command produced the output

**Impact:**
- External tools/scripts cannot reliably parse Iron output
- No way to distinguish between command outputs programmatically
- No execution metadata for logging/monitoring
- This directly violates Principle 6 ("Consistency is a feature") from the concerns document
- Breaks the "machine-readable mode" contract from the technical spec

**What's Needed:**
- `IronEnvelope<T>` generic wrapper: `{ ok, command, data: T, error, meta: { timestamp, duration_ms, host } }`
- All `--format json` outputs wrapped in envelope
- Standardized error envelope with error code, message, suggestion, details
- Applied uniformly across all commands

---

### GAP-C4: Config Format Divergence (TOML vs. Lua)

**Original Vision** (pre-phase3-technicals.md, Section 4):
> All configuration in Lua: `iron.lua`, `hosts/desktop.lua`, `modules/dotfiles/fish.lua`
> Lua as the composition language — programmable config with `require()`, conditionals, loops

**What Was Built:**
- All configuration in TOML: `host.toml`, `bundle.toml`, `profile.toml`, `module.toml`
- No Lua integration at all
- TOML is parsed via serde with static struct deserialization

**Assessment:**
This is a **deliberate divergence** that was likely the right call for Phase 1 — TOML is simpler, more predictable, and works well with serde. However, TOML has fundamental limitations that become problems as the project scales:

- **No composition logic**: Cannot conditionally include modules based on host properties
- **No programmatic defaults**: Cannot compute values (e.g., `packages = base_packages + extra_packages`)
- **No conditional includes**: Cannot say "if gpu == 'nvidia' then include nvidia-module"
- **No inheritance beyond `extends`**: Profile inheritance is ad-hoc, not a language feature

**Impact:**
- The "single entry point composes everything" vision (iron.lua) was never realized
- Host-specific customization is limited to explicit field overrides
- The `vars/` directory concept from the spec has no equivalent
- Multi-machine configurations will become verbose without composition logic

**What's Needed (Decision Required):**
- **Option A**: Stay with TOML, add a lightweight expression layer (e.g., `iron.toml` with `[include]` sections, conditional modules via `when = "host.gpu == 'nvidia'"`)
- **Option B**: Migrate to a scriptable format (Lua via mlua, Starlark, Nickel, or CUE)
- **Option C**: Keep TOML for declarations, add a separate `iron.lua` composition layer that generates the TOML
- This is the most impactful architectural decision for Phase 3

---

### GAP-C5: No `iron plan` Command (Plan ≠ Apply --dry-run)

**Original Vision** (pre-phase3-technicals.md, Section 8.2):
> `iron plan` — Generate a plan without applying changes
> Supports: `--host`, `--module` (repeatable), `--json`, `--verbose`
> Separate from `apply` — the plan is a first-class artifact

**What Was Built:**
- `iron apply --dry-run` shows what would change, but there is no standalone `iron plan` command
- The plan is computed inside `ApplyService::plan()` but never serialized or stored
- No way to: generate a plan, save it, review it, then apply it later
- No way to share a plan for review before execution

**Impact:**
- The Plan → Diff → Apply lifecycle (Section 9) from the spec is collapsed into a single command
- Plans are ephemeral — computed and either executed or discarded
- No plan history, no plan diffing, no plan approval workflow
- This limits collaboration scenarios (e.g., "here's what would change — approve before I apply")

**What's Needed:**
- `iron plan` command that outputs a serializable `ApplyPlan` (JSON or custom format)
- `iron apply --plan <file>` to execute a previously saved plan
- Plan storage in operation history
- This reinforces Pillar C ("Safe Execution Workflow") from the concerns document

---

## 2. SIGNIFICANT GAPS

### GAP-S1: No `iron status` Command

**Original Vision** (pre-phase3-technicals.md, Section 8.2):
> `iron status` — Quick system overview: active host, enabled modules, pending changes, drift summary, updates available, last apply timestamp

**What Was Built:**
- No `iron status` command exists
- The TUI Dashboard provides some of this information visually
- But there's no CLI equivalent for scripting or quick terminal checks

**Impact:**
- Users must launch the TUI (`iron go`) for a system overview
- No scriptable status check (critical for cron jobs, CI, monitoring)
- Violates the "interactive CLI" focus from the concerns document

---

### GAP-S2: No `iron config` Namespace

**Original Vision** (pre-phase3-technicals.md, Section 8.2):
> `iron config path` — prints config directory
> `iron config edit` — opens root config in $EDITOR
> `iron config validate` — validates all config
> `iron config init` — scaffold new config repo

**What Was Built:**
- `iron init` exists (partial equivalent of `config init`)
- `iron validate` exists (equivalent of `config validate`)
- No `iron config path` or `iron config edit`
- No config namespace grouping

**Impact:**
- Configuration management commands are scattered across top-level
- No unified "config" experience for users exploring the tool

---

### GAP-S3: No Operation History (`iron history`)

**Original Vision** (pre-phase3-technicals.md, Section 8.2):
> `iron history list` / `show <id>` / `last`
> Operation history with plan, diff, apply-result, stdout/stderr, backups per operation

**What Was Built:**
- `StateManager` tracks `last_operations: Vec<OperationRecord>` (last 100)
- `AuditLog` records operations to `audit.log` (JSONL)
- TUI has an `OperationLog` view
- BUT: No CLI `iron history` command
- No per-operation artifacts (plan.json, diff.json, backups/)
- No history directory structure as specified in Section 5

**Impact:**
- Users cannot review past operations from the CLI
- No forensic capability after a bad apply
- No operation-level rollback ("undo the last apply")

---

### GAP-S4: Runtime State Location Divergence

**Original Vision** (pre-phase3-technicals.md, Section 5):
> - `~/.config/iron/` = canonical config (user truth, Git-tracked)
> - `~/.local/state/iron/` = runtime history/logs/backups
> - `~/.cache/iron/` = ephemeral cache only

**What Was Built:**
- `state.json` lives inside `~/.config/iron/` (the Git-tracked repo)
- `audit.log` lives inside `~/.config/iron/`
- `.snapshots/` lives inside `~/.config/iron/`
- `.state.lock` lives inside `~/.config/iron/`
- No `~/.local/state/iron/` separation
- No `~/.cache/iron/` usage

**Impact:**
- Runtime state (mutable, machine-specific) is mixed with canonical config (declarative, portable)
- `state.json` will cause merge conflicts in multi-machine Git sync
- `.snapshots/` directory grows unbounded in the Git repo
- Violates the clean separation principle from Section 6 of the reasoning document
- `.gitignore` must carefully exclude runtime files — fragile

**What's Needed:**
- Move `state.json`, `audit.log`, `.state.lock` to `~/.local/state/iron/`
- Move `.snapshots/` to `~/.local/state/iron/snapshots/`
- Keep `~/.config/iron/` purely declarative (hosts/, bundles/, profiles/, modules/, files/)
- This is the single most important structural fix for multi-machine support in Phase 3

---

### GAP-S5: No File Content Deployment (Only Symlinks)

**Original Vision** (pre-phase3-technicals.md, Section 7.3):
> "copy managed file from ~/.config/iron/files/... to target"
> "optional template variable substitution"
> "backup target before overwrite"
> "diff preview before write"

**What Was Built:**
- `DotfileMapping.link: bool` field exists — when `true`, creates symlink; when `false`, should copy
- But the apply pipeline ONLY creates symlinks (`CreateSymlink` action)
- No `CopyFile` or `RenderTemplate` action type exists in `ApplyAction` enum
- No file content diff preview (only symlink exists/broken/wrong-target checks)

**Impact:**
- Cannot deploy files that need to be modified after deployment (e.g., compiled configs)
- Cannot use template variables in dotfiles (related to GAP-C1)
- Cannot generate derivative configs (e.g., render a template, then copy the output)
- The `link: false` option on DotfileMapping is a no-op

---

### GAP-S6: Missing Safety Risk Levels on Actions

**Original Vision** (pre-phase3-technicals.md, Section 11.1):
> Risk levels: `read_only`, `additive`, `destructive`, `critical`
> Each action has a risk_level field
> Confirmation policy scales with risk

**What Was Built:**
- TUI has `ConfirmStyle` (Simple/EnhancedWarning/TypedConfirmation) and `RiskLevel` for updates
- BUT: `ApplyAction` has no `risk_level` field
- All apply actions get the same confirmation prompt
- No distinction between "install a package" (additive) and "remove a symlink to replace it" (destructive)

**Impact:**
- Users cannot assess risk before confirming
- No way to filter plan by risk level
- `--dry-run` output doesn't indicate which actions are dangerous

---

### GAP-S7: No Per-Module Hooks Execution in Apply

**Original Vision** (pre-phase3-technicals.md, Section 7.1):
> "Hooks (optional) — pre/post apply scripts"
> Module declares: `pre_install`, `post_install`, `pre_uninstall`, `status_check`

**What Was Built:**
- `Module` struct has: `pre_install`, `post_install`, `pre_uninstall`, `status_check` (all `Option<String>`)
- BUT: The apply pipeline ignores hooks entirely
- No `RunHook` action type in `ApplyAction` enum
- Hooks are defined but never executed

**Impact:**
- Modules cannot run setup scripts (e.g., `nvim --headless +PlugInstall`)
- Post-install configuration steps must be done manually
- The `status_check` hook for module health is unused

---

### GAP-S8: No Package Removal in Apply

**Original Vision** (pre-phase3-technicals.md, Section 6.4):
> Action types include: `remove_package`, `disable_service`

**What Was Built:**
- `ApplyAction` only has `InstallPackages`, `InstallAurPackages`, `CreateSymlink`, `EnableService`, `ActivateModule`
- No `RemovePackage`, `RemoveSymlink`, `DisableService`, `DeactivateModule` actions
- Apply is additive-only — it can add but not remove

**Impact:**
- Switching from one bundle to another leaves orphaned packages
- Disabling a module doesn't uninstall its packages
- No convergence toward "exactly the desired state" — only convergence toward "at least the desired state"
- Drift detection (`iron diff`) can identify extras but cannot correct them via apply

---

## 3. OBSERVATIONS (Acceptable Divergences)

### OBS-1: TUI Was Built Despite CLI-First Vision

**Original Vision** (pre-phase3-concerns.md, Section 4):
> "For Phase 1, I want to focus only on an interactive CLI. I am not avoiding a richer interface forever. I am sequencing the work."

**Reality:** A full 28-view Ratatui TUI was built alongside the CLI. This appears to be a scope expansion that happened during development. The TUI is well-built and provides significant value, but it consumed development effort that could have gone toward the CLI-first gaps identified above.

**Assessment:** Not a problem per se — the TUI is a genuine asset. But the CLI experience has gaps (no `iron status`, no `iron plan`, no `iron history`, limited JSON output) that should have been prioritized first per the original vision.

---

### OBS-2: TOML Instead of Lua (Covered in GAP-C4)

This is an acceptable divergence for Phase 1-2. TOML is simpler and faster to implement. The question is whether Phase 3 needs composition logic that TOML cannot express.

---

### OBS-3: Bundle/Profile Model vs. Original Module Categories

**Original Vision** (pre-phase3-technicals.md, Section 7.2):
> Module categories: `packages`, `services`, `dotfiles`, `cleanup`

**Reality:** Modules are classified by `ModuleKind` (AppConfig, Shell, DesktopComponent, Theme, SystemUtil, DevTools, SecurityHardening) and organized in a flat namespace. The Bundle/Profile/Module hierarchy is more sophisticated than the original spec's flat category model.

**Assessment:** The implemented model is arguably better than the original spec. The hierarchical Host → Bundle → Profile → Module model provides more structure and supports the desktop environment use case well.

---

### OBS-4: Git-Crypt for Secrets vs. Original "Later Phase"

**Original Vision** (pre-phase3-concerns.md, Section 15):
> "Full secrets management workflow" listed as postponed

**Reality:** A full secrets management system exists (iron-git with git-crypt integration, SecretsBackend trait, lock/unlock/link/add-key/export-key commands, TUI view).

**Assessment:** This was built ahead of schedule and appears well-implemented. No issue.

---

### OBS-5: Circuit Breaker Pattern Not in Original Spec

The resilience/circuit breaker infrastructure was not in the original technical spec but was added during hardening sprints. This is a valuable addition that protects against system command failures.

---

### OBS-6: Crate Organization Divergence

**Original Vision:** 10 crates (iron-cli, iron-core, iron-state, iron-modules, iron-scanner, iron-planner, iron-applier, iron-integrations, iron-audit, iron-output, iron-types)

**Reality:** 7 crates (iron-cli, iron-core, iron-tui, iron-fs, iron-pacman, iron-git, iron-systemd)

**Assessment:** The actual organization is more practical. The original spec over-segmented the core (separate planner, applier, scanner crates). The current model puts all business logic in iron-core with infrastructure in dedicated crates. This is cleaner.

---

## 4. CROSS-CUTTING THEMES

### Theme 1: The "Declarative" Identity Is Incomplete

The original vision emphasizes "declarative" as the core identity. Currently, Iron is **partially declarative**:

| Aspect | Declarative? | Issue |
|--------|-------------|-------|
| Package installation | Yes | Declares packages, installs missing |
| Package removal | **No** | Cannot remove undeclared packages |
| Service enable | Yes | Declares services, enables missing |
| Service disable | **No** | Cannot disable undeclared services |
| Dotfile linking | Partial | Creates symlinks but no template rendering |
| Module activation | Yes | Tracks in state |
| Module deactivation | **No** | No cleanup on disable |

True declarative means "the system should look exactly like this" — Iron currently only does "the system should have at least this." The gap is the additive-only apply (GAP-S8).

### Theme 2: The Execution Lifecycle Is Collapsed

The original 7-step lifecycle (Load → Scan → Plan → Validate → Confirm → Apply → Finalize) is partially implemented:

| Step | Status | Notes |
|------|--------|-------|
| 1. Load & Compose | **Done** | `resolve_desired_state()` |
| 2. Scan | **Partial** | No `ActualState` contract (GAP-C2) |
| 3. Plan | **Done** | `compute_plan()` returns `ApplyPlan` |
| 4. Validate | **Done** | `iron validate` + pre-apply validation |
| 5. Confirm | **Done** | Interactive prompt with `-y` override |
| 6. Apply | **Partial** | Additive only, no hooks (GAP-S7, S8) |
| 7. Finalize | **Partial** | No history artifacts, no plan storage (GAP-S3) |

### Theme 3: Multi-Machine Readiness

Phase 3 includes multi-machine features. Current blockers:

1. **state.json in Git repo** (GAP-S4) — Will cause merge conflicts
2. **No ActualState serialization** (GAP-C2) — Cannot compare states across hosts
3. **No template rendering** (GAP-C1) — Cannot customize dotfiles per-host
4. **No envelope format** (GAP-C3) — Cannot reliably pipe JSON between hosts
5. **Additive-only apply** (GAP-S8) — Cannot converge hosts to identical states

### Theme 4: The User Interaction Model

The original vision emphasized "guided, pleasant CLI" with "Bubble Tea-like" interactivity. What exists:

| Pattern | Status | Notes |
|---------|--------|-------|
| Clear prompts | **Done** | Confirmation before destructive actions |
| Progress indicators | **Done** | indicatif spinners/bars |
| Structured output | **Done** | Tree, table, summary blocks |
| Diff/preview before changes | **Partial** | `--dry-run` exists but no file content diffs |
| Readable summaries | **Done** | Summary blocks on most commands |
| Select prompts | **Missing** | No interactive selection (e.g., "pick a module") |
| `--json` consistency | **Missing** | No envelope (GAP-C3) |
| Status at a glance | **Missing** | No `iron status` (GAP-S1) |

---

## 5. PRIORITIZED RECOMMENDATIONS

### Must-Fix Before Phase 3 (Foundation)

| Priority | Gap | Effort | Why Now |
|----------|-----|--------|---------|
| 1 | **GAP-S4**: Separate runtime state from config dir | Medium | Blocks multi-machine Git sync |
| 2 | **GAP-C2**: Define `ActualState` contract | Medium | Blocks clean plan/drift, needed for multi-host comparison |
| 3 | **GAP-C1**: Template variable rendering in apply | Medium | Core value prop — per-host customization |
| 4 | **GAP-S8**: Add removal actions to apply | Large | True declarative convergence |
| 5 | **GAP-C3**: Response envelope for `--json` | Medium | Scriptability, consistency |

### Should-Fix in Phase 3 (Core Experience)

| Priority | Gap | Effort | Why |
|----------|-----|--------|-----|
| 6 | **GAP-C5**: `iron plan` command | Small | Plan as first-class artifact |
| 7 | **GAP-S1**: `iron status` command | Small | CLI system overview |
| 8 | **GAP-S3**: `iron history` command | Medium | Operation forensics |
| 9 | **GAP-S5**: File copy deployment | Small | Non-symlink dotfile support |
| 10 | **GAP-S7**: Module hooks execution | Medium | Post-install automation |

### Can Defer (Nice-to-Have)

| Priority | Gap | Effort | Why Deferrable |
|----------|-----|--------|----------------|
| 11 | **GAP-S6**: Risk levels on actions | Small | UX improvement, not blocking |
| 12 | **GAP-S2**: `iron config` namespace | Small | Organizational, not blocking |
| 13 | **GAP-C4**: Config format decision | Large | TOML works for now; evaluate if Phase 3 truly needs composition logic |

---

## 6. ARCHITECTURAL ASSESSMENT

### What's Strong

1. **Service layer pattern** — Trait-based DI with builder injection is clean, testable, and extensible
2. **Circuit breaker resilience** — Production-quality protection for external commands
3. **State management** — Atomic writes, file locking, transactions, audit logging
4. **Domain model** — Host → Bundle → Profile → Module hierarchy is well-designed
5. **Test coverage** — 2,033 tests with comprehensive mocking infrastructure
6. **Error system** — Thiserror + suggestions + recovery actions is thoughtful
7. **TUI** — 28 views with risk-differentiated confirmations is impressive

### What Needs Strengthening

1. **Declarative convergence** — Must go from additive-only to full convergence (add AND remove)
2. **Data contracts** — `ActualState` and response `Envelope` are missing first-class types
3. **State/config separation** — Runtime data must move out of the Git-tracked config dir
4. **Template rendering** — Variables exist but aren't used — this is dead infrastructure
5. **CLI completeness** — `iron status`, `iron plan`, `iron history` are important missing commands
6. **Execution lifecycle** — Hooks, file copies, and removal actions complete the apply pipeline

### Overall Verdict

The project has a **strong foundation** with excellent patterns (traits, DI, circuit breakers, atomic state, comprehensive testing). The gaps are mostly about **completeness**, not quality. The code that exists is well-structured — the issue is that some critical pieces of the original vision were deferred or missed during the rapid Phase 1-2 development.

The most important pre-Phase 3 work is:
1. **Separate runtime state** (make multi-machine Git sync possible)
2. **Add ActualState contract** (make the scan→plan pipeline clean)
3. **Enable template rendering** (make per-host customization real)
4. **Add removal actions** (make Iron truly declarative)
5. **Add response envelope** (make Iron scriptable and consistent)

These five items transform Iron from "a system that adds what's missing" to "a system that converges to exactly what's declared" — which is the original vision's core identity.
