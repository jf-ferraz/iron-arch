````markdown 
# Technical Specification — Phase 1 Foundation (v0.1 Draft)

> **Purpose:** This document defines the technical foundation for Phase 1 of the project, based on the previously approved project considerations.
>
> **Scope:** Core structure, file/directory layout, data contracts, module contract, command tree, and execution lifecycle.
>
> **Audience:** Project maintainer (you), future contributors, and future-you during refactors.

---

## 1. Technical Goals for Phase 1

Phase 1 should establish a strong core that supports immediate real-world usage during Arch Linux restructuring while preserving a path to future expansion.

### Primary technical goals
1. **Canonical user-owned config layout**
2. **Stable data contracts** (desired state, plan, diff, drift, apply result)
3. **Module-based composition**
4. **Safe plan/diff/apply pipeline**
5. **Interactive CLI UX (not full TUI)**
6. **Clear runtime separation (state/logs/history/backups)**

---

## 2. Repository Structure (Monorepo)

This structure assumes a Rust-first CLI/core (recommended for Phase 1 since you already have Rust CLI progress).

```bash
iron/
├── Cargo.toml                   # workspace
├── Cargo.lock
├── README.md
├── LICENSE
│
├── crates/
│   ├── iron-cli/                # CLI entrypoint + interactive UX
│   ├── iron-core/               # Core orchestration (state→plan→apply)
│   ├── iron-state/              # Config loading/composition (Lua)
│   ├── iron-modules/            # Module contracts + module resolvers
│   ├── iron-scanner/            # Actual system scanning
│   ├── iron-planner/            # Diff/planning engine
│   ├── iron-applier/            # Execution/apply/backups
│   ├── iron-integrations/       # pacman/systemd/git/fs wrappers
│   ├── iron-audit/              # history/logging/operation records
│   ├── iron-output/             # human/json render envelopes
│   └── iron-types/              # shared contracts/types
│
├── contracts/                   # JSON schemas (reference + tests)
│   ├── envelope.schema.json
│   ├── desired-state.schema.json
│   ├── actual-state.schema.json
│   ├── plan.schema.json
│   ├── diff.schema.json
│   ├── drift-report.schema.json
│   └── apply-result.schema.json
│
├── docs/
│   ├── project-considerations-phase1.md
│   ├── technical-spec-phase1.md
│   ├── config-layout.md
│   ├── command-spec-v0.1.md
│   ├── module-spec.md
│   └── safety-model.md
│
├── examples/
│   ├── config-repo/             # sample ~/.config/iron
│   ├── plans/
│   ├── drift/
│   └── outputs/
│
├── scripts/
│   ├── dev.sh
│   ├── test.sh
│   ├── lint.sh
│   └── release.sh
│
└── tests/
    ├── integration/
    ├── fixtures/
    └── golden/
````

---

## 3. User-Controlled Canonical Config Directory (Source of Truth)

The user’s source of truth should live in:

```bash
~/.config/iron/
```

This directory is:

* user-owned
* Git-friendly
* portable across machines
* the only place the user edits to declare desired system state

---

## 4. Canonical Config Layout (Phase 1)

### Proposed layout

```bash
~/.config/iron/
├── iron.lua                     # Entry point (root declaration)
│
├── hosts/
│   ├── desktop.lua              # Host-specific overrides
│   └── laptop.lua
│
├── shared/
│   ├── defaults.lua             # Shared defaults (non-sensitive)
│   ├── packages.lua             # Shared package declarations
│   ├── services.lua             # Shared service declarations
│   └── cleanup.lua              # Shared cleanup policies
│
├── modules/
│   ├── dotfiles/
│   │   ├── fish.lua
│   │   ├── git.lua
│   │   ├── hyprland.lua
│   │   └── nvim.lua
│   │
│   ├── packages/
│   │   ├── dev.lua
│   │   ├── desktop.lua
│   │   └── cli-tools.lua
│   │
│   ├── services/
│   │   ├── network.lua
│   │   ├── bluetooth.lua
│   │   └── ssh.lua
│   │
│   └── cleanup/
│       └── base.lua
│
├── profiles/                    # Optional in v0.1, but layout reserved
│   ├── developer.lua
│   └── minimal.lua
│
├── vars/
│   ├── shared.lua               # Shared variables
│   ├── desktop.lua              # Host-specific variables
│   └── laptop.lua
│
├── files/                       # Version-controlled file sources/templates
│   ├── fish/
│   │   └── config.fish
│   ├── git/
│   │   └── gitconfig
│   ├── hypr/
│   │   └── hyprland.conf
│   └── nvim/
│       └── init.lua
│
└── hooks/                       # Optional scripts (Phase 1: basic support)
    ├── pre-apply/
    └── post-apply/
```

### Design notes

* `modules/` contains declarations (what should happen)
* `files/` contains actual file content/templates (what gets deployed)
* `vars/` contains substitution values
* `hosts/` defines host-specific overrides
* `iron.lua` composes everything

---

## 5. Runtime State Directory (Tool-Owned)

The tool should store operational state under:

```bash
~/.local/state/iron/
```

Optional cache:

```bash
~/.cache/iron/
```

### Proposed runtime layout

```bash
~/.local/state/iron/
├── logs/
│   ├── iron.log
│   └── operations/
│
├── history/
│   ├── 2026-02-22T03-10-01Z_apply/
│   │   ├── manifest.json
│   │   ├── plan.json
│   │   ├── diff.json
│   │   ├── apply-result.json
│   │   ├── stdout.log
│   │   ├── stderr.log
│   │   └── backups/
│   └── ...
│
├── backups/
│   ├── files/
│   └── module/
│
├── locks/
│   └── apply.lock
│
└── sessions/
    └── current-session.json
```

### Rule

* `~/.config/iron` = canonical config (user truth)
* `~/.local/state/iron` = runtime history/logs/backups
* `~/.cache/iron` = ephemeral cache only

---

## 6. Data Contracts (Core Types)

Phase 1 should define stable contracts early. These contracts should exist as:

* Rust types (`iron-types`)
* JSON schemas (`contracts/`)
* example payloads (`examples/`)

---

### 6.1 Envelope (common response contract)

Used for all machine-readable outputs (`--json`) between internal components and CLI rendering.

```json
{
  "ok": true,
  "command": "plan",
  "data": {},
  "error": null,
  "meta": {
    "timestamp": "2026-02-22T03:10:00Z",
    "duration_ms": 123,
    "host": "desktop"
  }
}
```

#### Error form

```json
{
  "ok": false,
  "command": "apply",
  "data": null,
  "error": {
    "code": "CONFIG_VALIDATION_FAILED",
    "message": "Hyprland config validation failed",
    "details": {
      "module": "dotfiles.hyprland",
      "file": "~/.config/hypr/hyprland.conf"
    }
  },
  "meta": {
    "timestamp": "2026-02-22T03:10:00Z",
    "duration_ms": 77,
    "host": "desktop"
  }
}
```

---

### 6.2 DesiredState (resolved canonical state)

This is the result of loading and composing:

* `shared`
* `host`
* `vars`
* enabled modules
* (future) profiles

#### Fields (conceptual)

* `host`: active host name
* `modules_enabled`: list of module IDs
* `packages`: desired package sets (repo/AUR)
* `services`: desired enabled/disabled states
* `files`: managed file specs (source → target)
* `cleanup`: cleanup policies
* `vars`: resolved variables
* `hooks`: declared hooks (optional)
* `metadata`: config version / source paths / generation time

---

### 6.3 ActualState (scanned system state)

Represents what is currently on the machine.

#### Phase 1 scope

* installed packages (repo + AUR where possible)
* enabled/running services
* tracked files (existence/hash/mtime for files managed by modules)
* basic system metadata (hostname, kernel, distro info optional)

#### Fields (conceptual)

* `host_detected`
* `packages_installed`
* `services_state`
* `files_state`
* `scan_warnings`
* `metadata`

---

### 6.4 Plan (what would change)

Generated from `DesiredState` + `ActualState`.

This is the key safety contract.

#### Plan fields

* `summary`
* `actions[]`
* `warnings[]`
* `risks[]`
* `requires_root`
* `backups_required[]`
* `validation_checks[]`

#### Action types (Phase 1)

* `install_package`
* `remove_package`
* `enable_service`
* `disable_service`
* `write_file`
* `backup_file`
* `cleanup_cache`
* `cleanup_orphans`
* `cleanup_logs`
* `run_hook`

Each action should include:

* `id`
* `type`
* `module` (optional but strongly recommended)
* `target`
* `details`
* `risk_level`
* `requires_root`

---

### 6.5 Diff (user-facing change preview)

`Diff` is a presentation-oriented contract derived from `Plan`, especially for configs/files.

#### Fields

* `module` (optional filter)
* `file_diffs[]`
* `package_changes`
* `service_changes`
* `summary`

#### File diff item (conceptual)

* `target_path`
* `change_type` (`create`, `modify`, `delete`, `unchanged`)
* `preview` (optional, truncated)
* `line_changes` (`+`, `-`, context counts)
* `backup_will_be_created` (bool)

---

### 6.6 DriftReport (actual vs declared divergence)

Central contract for intermediate workflow.

#### Drift categories

* `packages_extra` (installed but undeclared)
* `packages_missing` (declared but absent)
* `services_drifted`
* `files_modified_locally`
* `files_missing`
* `files_unmanaged_conflicts` (future)
* `notes/recommendations`

#### Resolution hints

Each drift item should ideally include suggested actions:

* `adopt`
* `revert`
* `ignore` (future support)

---

### 6.7 ApplyResult (execution result)

Records what actually happened.

#### Fields

* `plan_id`
* `started_at`, `finished_at`
* `actions_executed[]`
* `actions_failed[]`
* `backups_created[]`
* `rollback_refs[]` (Phase 1 can be local file-level)
* `summary`
* `warnings`
* `next_steps`

---

## 7. Module Contract (Phase 1 Spec)

Modules are the primary unit of organization and apply.

### Module ID convention

Use dot notation:

* `packages.dev`
* `services.network`
* `dotfiles.fish`
* `dotfiles.hyprland`
* `cleanup.base`

---

### 7.1 Module declaration responsibilities

Each module should declare:

1. **Identity** — name, type, description
2. **Dependencies** — other modules (optional)
3. **Packages** — required packages
4. **Services** — desired service states
5. **Files** — managed files (source/target)
6. **Variables required** — values expected in `vars`
7. **Hooks** (optional) — pre/post apply scripts
8. **Validation hints** (optional Phase 1.1) — syntax checks

---

### 7.2 Phase 1 module categories

* `packages`
* `services`
* `dotfiles`
* `cleanup`

(Future categories: `security`, `host`, `bundle`, `imported`)

---

### 7.3 Dotfiles module behavior (Phase 1)

Phase 1 should keep this simple and reliable:

#### Supported actions

* copy managed file from `~/.config/iron/files/...` to target
* optional template variable substitution
* backup target before overwrite
* diff preview before write
* track hash for drift

#### Not required yet

* complex AST patching
* deep merge of arbitrary config formats
* interactive conflict merge UI

---

## 8. Command Tree (CLI v0.1)

Phase 1 CLI should support both:

* **human-readable interactive mode** (default)
* **machine-readable mode** (`--json`)

---

### 8.1 Top-level commands

```bash
iron
├── status
├── plan
├── diff
├── apply
├── scan
├── module
├── config
├── update
├── clean
├── history
├── rollback
└── doctor   (optional stub in v0.1)
```

---

### 8.2 Command details (v0.1)

#### `iron status`

Quick system overview.

**Outputs**

* active host
* enabled modules count
* pending changes (if detectable)
* drift summary (lightweight)
* updates available (optional if cheap)
* last apply / last backup timestamp

**Flags**

* `--json`
* `--host <name>` (override detection)

---

#### `iron plan`

Generate a plan without applying changes.

**Use cases**

* full plan
* per module
* per host

**Flags**

* `--host <name>`
* `--module <module-id>` (repeatable)
* `--json`
* `--verbose`

---

#### `iron diff`

Show user-facing diffs (files/packages/services).

**Flags**

* `--host <name>`
* `--module <module-id>`
* `--file <path>`
* `--json`

---

#### `iron apply`

Apply planned changes safely.

**Behavior**

* build plan
* show summary + diff
* confirm (unless `--yes`)
* create backups as needed
* execute actions
* record history

**Flags**

* `--host <name>`
* `--module <module-id>` (repeatable)
* `--yes`
* `--dry-run` (alias to `plan`, still useful UX)
* `--json`
* `--verbose`

---

#### `iron scan`

Scan actual system state.

**Flags**

* `--drift` (compare against canonical state)
* `--packages`
* `--services`
* `--files`
* `--json`

---

#### `iron module ...`

Module management namespace.

Subcommands:

* `iron module list`
* `iron module show <module-id>`
* `iron module apply <module-id>` (wrapper over `apply --module`)
* `iron module diff <module-id>`
* `iron module edit <module-id>` (opens source declaration in `$EDITOR`)

Flags:

* `--json` (for list/show)
* `--host <name>` (where relevant)

---

#### `iron config ...`

Canonical config utilities.

Subcommands:

* `iron config path` (prints `~/.config/iron`)
* `iron config edit` (opens root config in `$EDITOR`)
* `iron config validate`
* `iron config init` (optional scaffold)

---

#### `iron update`

Safe update routine (built on plan/apply principles where possible).

Phase 1 behavior:

* check updates
* optionally create lightweight pre-update backup/record
* run package updates (repo + AUR helper if configured)
* record history
* summarize changes

Flags:

* `--yes`
* `--json`
* `--no-aur` (optional)
* `--dry-run` (show available updates only)

---

#### `iron clean`

Cleanup routine.

Phase 1 scope:

* orphan packages
* package cache policy
* logs cleanup (safe subset)
* temp cleanup (safe subset)

Flags:

* `--yes`
* `--dry-run`
* `--json`
* `--orphans`
* `--cache`
* `--logs`
* `--temp`

---

#### `iron history`

Operation history.

Subcommands:

* `list`
* `show <id>`
* `last`

---

#### `iron rollback`

Phase 1 rollback should focus on file/module rollback (not full snapshots yet).

Subcommands:

* `last`
* `operation <history-id>`
* `module <module-id>` (best effort, file-based)
* `file <path>` (optional)

---

## 9. Execution Lifecycle (Plan → Diff → Apply)

This lifecycle should be consistent across all mutating operations.

---

### 9.1 Load & Compose

1. Detect or select host
2. Load canonical config from `~/.config/iron`
3. Resolve variables
4. Compose shared + host + modules
5. Produce `DesiredState`

### 9.2 Scan

6. Scan actual system (`ActualState`) for relevant domains:

   * packages
   * services
   * managed files

### 9.3 Plan

7. Compare desired vs actual
8. Generate `Plan`
9. Generate `Diff` (presentation-oriented)

### 9.4 Validate

10. Validate plan safety (root needed, risk levels)
11. Validate file sources exist
12. (Optional) Validate syntax for known configs if validators exist

### 9.5 Confirm

13. Render summary + diff
14. Prompt for confirmation unless `--yes`

### 9.6 Apply

15. Create lock
16. Create backups for affected files
17. Execute actions in deterministic order
18. Record success/fail per action

### 9.7 Finalize

19. Write `ApplyResult` + artifacts into history directory
20. Release lock
21. Print summary + next steps

---

## 10. Action Ordering (Deterministic Apply Policy)

Phase 1 should use a deterministic order to reduce surprises.

### Recommended order

1. `backup_file`
2. `install_package`
3. `remove_package` (if enabled in plan; can be Phase 1.1)
4. `write_file`
5. `enable_service` / `disable_service`
6. `run_hook`
7. cleanup actions (when applicable)

### Why this order

* backups first
* dependencies installed before configs/services
* configs before service toggles/reloads
* hooks last (unless pre-apply hook explicitly requested)

---

## 11. Safety Model (Phase 1)

### 11.1 Risk levels

Each action and plan should classify risk:

* `read_only`
* `additive`
* `destructive`
* `critical`

### 11.2 Confirmation policy

* `read_only`: no confirmation needed
* `additive`: summary confirmation (unless `--yes`)
* `destructive`: explicit confirmation with details
* `critical`: explicit warning + confirmation (future stronger protections)

### 11.3 Root usage policy

* Read-only commands should not require root
* Root should be requested only when necessary
* CLI should explain why elevated privileges are needed

---

## 12. Drift Detection Model (Phase 1)

Phase 1 drift detection should focus on **declared domains only**, not full system introspection.

### Scope

* Packages declared by modules/shared config
* Services declared by modules/shared config
* Files managed by modules

### Drift outcomes

For each drift item, return:

* category
* target
* declared state
* actual state
* suggested resolution (`adopt` or `revert`)

### Future expansion

* unmanaged package audit
* unmanaged service audit
* config semantic drift
* richer conflict resolution

---

## 13. CLI Interaction Style (Bubble Tea-like, Non-TUI)

Phase 1 CLI should feel interactive and polished without full-screen UI.

### Expected patterns

* Select prompts (module selection)
* Confirm prompts with summaries
* Progress spinners for long operations
* Tables for `status`, `module list`, `drift`
* Human-readable summaries at the end of every command

### Modes

* **Default:** human interactive output
* **`--json`:** machine-readable envelope output
* **`--verbose`:** underlying command detail/logs
* **`--yes`:** non-interactive apply/update/clean

---

## 14. Suggested Implementation Order (Technical Roadmap)

### Phase 1A — Core Contracts + Config Loading

* `iron-types` contracts
* `iron-state` Lua loader/composer
* `iron config validate`
* `iron status` (minimal)

### Phase 1B — Packages + Services

* scanners for packages/services
* planner actions for packages/services
* `plan`, `diff`, `apply` for packages/services
* `update` command (safe wrapper)

### Phase 1C — Dotfiles (simple but reliable)

* file source → target deployment
* backups before overwrite
* file diff previews
* file drift detection
* `module edit`, `module apply`

### Phase 1D — Cleanup + History + Rollback

* cleanup actions (orphans/cache/logs)
* operation history
* file-level rollback
* improved summaries

---

## 15. Open Design Decisions (To Finalize Next)

These can be decided after initial scaffolding but should be tracked explicitly:

1. **Lua schema style**

   * exact syntax and helper API for `iron.lua` and modules

2. **Template variable syntax**

   * `{{var}}` vs `${var}` vs Lua-native interpolation strategy

3. **AUR helper selection**

   * auto-detect `paru`/`yay` vs explicit config

4. **Config validation strategy**

   * which modules get syntax validation in v0.1 (Hyprland, Fish, etc.)

5. **Rollback depth**

   * file-only in v0.1 vs partial module rollback metadata

6. **Host detection**

   * system hostname by default + override in CLI/config

---

## 16. Acceptance Criteria for the Phase 1 Foundation

Phase 1 foundation is considered successful when:

* The user can maintain a canonical config repo in `~/.config/iron`
* `iron plan` and `iron diff` produce reliable previews
* `iron apply` safely applies package/service/file changes with backups
* `iron scan --drift` reports meaningful drift for declared items
* `iron module list/show/apply/edit` supports module-centric workflow
* `iron update` and `iron clean` are usable in real system maintenance
* All commands support consistent human output and `--json` output
* Runtime artifacts are stored cleanly in `~/.local/state/iron`

---

## 17. Summary

This technical specification defines a Phase 1 foundation centered on:

* **canonical config ownership**
* **module-driven organization**
* **stable contracts**
* **safe change orchestration**
* **interactive CLI ergonomics**
* **clear separation between declared state and runtime state**

It is intentionally structured to support immediate real-world testing on an Arch Linux machine while keeping the architecture extensible for future phases (full TUI, advanced rollback, richer config intelligence, remote/multi-host operations, and beyond).

```
::contentReference[oaicite:0]{index=0}
```
