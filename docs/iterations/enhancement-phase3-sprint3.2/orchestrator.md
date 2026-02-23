# Orchestrator Handoff -- Sprint 3.2 (Full Declarative Convergence)

**Date:** 2026-02-23
**Type:** ENHANCEMENT (structural)
**Agent Chain:** analyst -> architect -> developer -> tester -> reviewer

---

## 1. Classification and Rationale

This is an **ENHANCEMENT with structural changes** requiring the full chain including architect:

- **New ApplyAction variants**: 6 new enum variants (CopyFile, RenderAndCopy, RemovePackages, DisableService, RemoveSymlink, DeactivateModule) -- changes the core data model
- **New RiskLevel enum**: New type with PartialOrd semantics affecting plan output, confirmation policy, and TUI rendering
- **State model extension**: 3 new fields on IronState (managed_packages, managed_services, managed_dotfiles) -- persistent data model change
- **New CLI flags**: --prune, --prune-packages, --prune-services, --prune-dotfiles on `iron apply`
- **Template-to-apply pipeline wiring**: Connects iron-fs template engine into the apply flow, changing dotfile deployment behavior

These are not additive-within-existing-structure changes. They modify the core ApplyAction enum, ApplyPlan behavior, State persistence model, CLI argument structure, and TUI rendering -- warranting architect involvement.

---

## 2. Sprint 3.2 Scope (8 tasks)

| Task | Title | Priority | Effort | Dependencies |
|------|-------|----------|--------|--------------|
| F3-021 | Managed resource tracking | Critical | M | None (MUST be first) |
| F3-008 | Template variable rendering in apply | Critical | L | None (parallel with F3-021) |
| F3-009 | File copy deployment mode (CopyFile) | Medium | M | F3-008 |
| F3-010 | Package removal (RemovePackages) | Critical | L | F3-021 |
| F3-011 | Service disable (DisableService) | Medium | M | F3-021 |
| F3-012 | Symlink/module removal | Medium | M | F3-021 |
| F3-013 | Risk levels on ApplyAction | Medium | S | F3-008, F3-009, F3-010, F3-011, F3-012 |
| F3-016 | `iron apply --confirm` UX flow | Medium | M | F3-013 |

**Note on task numbering:** The user's request lists "F3-014: Granular prune flags" and "F3-015: RiskLevel enum" and "F3-016: iron apply --confirm UX flow." However, the kanban assigns these differently. The kanban's Sprint 3.2 tasks are F3-021, F3-008, F3-009, F3-010, F3-011, F3-012, F3-013. The prune flags are part of F3-010/F3-011/F3-012 (each removal task includes its --prune-* flag). Risk levels are F3-013. The confirmation UX is implicitly part of F3-013's acceptance criteria ("Confirmation prompt scales: Additive -> simple, Destructive -> detailed, Critical -> typed"). The analyst should treat the 8 kanban tasks as the canonical scope.

---

## 3. Task Dependency Ordering

```
Wave 1 (parallel):
  F3-021 (managed resource tracking) -- prerequisite for all removal tasks
  F3-008 (template rendering in apply) -- independent, can parallel

Wave 2 (after Wave 1):
  F3-009 (CopyFile action) -- depends on F3-008 (template detection -> copy vs symlink)
  F3-010 (RemovePackages) -- depends on F3-021
  F3-011 (DisableService) -- depends on F3-021, parallel with F3-010
  F3-012 (RemoveSymlink/DeactivateModule) -- depends on F3-021, parallel with F3-010

Wave 3 (after Wave 2):
  F3-013 (RiskLevel + confirmation UX) -- classifies ALL action variants including new ones
```

---

## 4. Key Existing Infrastructure

The analyst and architect should be aware of what already exists:

### Already implemented (Sprint 3.1):
- `ActualState` struct with `scan()` -- `crates/iron-core/src/actual_state.rs`
- `IronEnvelope<T>` response wrapper -- `crates/iron-core/src/envelope.rs`
- XDG state directory separation -- `StateManager::state_dir()` in `crates/iron-core/src/services/state.rs`
- `iron status` and `iron plan` CLI commands
- All existing CLI --json commands migrated to envelope format

### Already exists in crate APIs (no new trait methods needed):
- `PackageManager::remove(packages, remove_deps)` -- already in trait (`crates/iron-core/src/packages.rs` line 122)
- `SystemService::disable_service(name)` -- already in trait (`crates/iron-core/src/system_service.rs` line 19)
- `iron_fs::template::has_variables(content)` -- already exists (`crates/iron-fs/src/lib.rs` line 677)
- `iron_fs::template::render(content, vars)` -- already exists (`crates/iron-fs/src/lib.rs` line 630)
- `iron_fs::backup::create(path)` -- already exists (`crates/iron-fs/src/lib.rs` line 197)
- `iron_fs::symlink::remove(target, restore_backup)` -- already exists (`crates/iron-fs/src/lib.rs` line 142)
- `DotfileMapping.link: bool` field -- already on the struct but is a no-op in current apply logic

### Current ApplyAction enum (5 variants):
```rust
pub enum ApplyAction {
    InstallPackages { packages: Vec<String> },
    InstallAurPackages { packages: Vec<String> },
    CreateSymlink { source: String, target: String, module_id: String },
    EnableService { name: String },
    ActivateModule { id: String },
}
```

### Current IronState struct (crates/iron-core/src/state.rs line 150):
- `current_host`, `active_bundles`, `active_profiles`, `active_modules`
- `last_operations`, `maintenance`, `update_progress`
- `news_acknowledgment`, `last_scan_report`
- **Does NOT yet have**: `managed_packages`, `managed_services`, `managed_dotfiles`

### Current apply execution (crates/iron-core/src/services/apply.rs):
- `compute_plan()` accepts `&ActualState` (refactored in Sprint 3.1)
- `execute()` loops over actions with match on 5 variants
- `scan_actual_state()` private helper builds specs from DesiredState

---

## 5. SHOULD Findings from Sprint 3.1 Review to Address

The Sprint 3.1 reviewer identified 4 SHOULD findings. The analyst should evaluate which can be addressed in this sprint:

### SHOULD-1: 8 unmigrated `output.json()` calls
**Files:** host.rs (3), profile.rs (2), bundle.rs (2), sync.rs (1)
**Recommendation:** Migrate if touching these files; otherwise defer.

### SHOULD-2: `json_error_envelope` method is unused
**File:** output.rs line 247
**Recommendation:** Wire into error paths of any new or modified commands in this sprint.

### SHOULD-3: Info lines bleed into JSON output
**Recommendation:** If adding new info() calls in JSON-emitting commands, route to stderr. Otherwise defer -- this is a systemic issue.

### SHOULD-4: Missing "last apply timestamp" in status
**Recommendation:** Address naturally with F3-021 -- once managed tracking is in state, record last_apply timestamp too. The status command already exists from Sprint 3.1.

### Reviewer recommendations for Sprint 3.2:
1. Consider `ActualState::scan_from_desired()` convenience method (COULD-1) to reduce duplication
2. Evaluate utility of `ManagedFileSpec::expected_source` field (COULD-2) for symlink correctness checks

---

## 6. Affected Files (anticipated cascade)

### Core modifications:
- `crates/iron-core/src/services/apply.rs` -- ApplyAction new variants, compute_plan changes, execute changes, managed tracking
- `crates/iron-core/src/services/state.rs` -- managed_packages/services/dotfiles fields on IronState
- `crates/iron-core/src/state.rs` -- IronState struct definition (new fields)
- `crates/iron-core/src/test_helpers.rs` -- update all builders/helpers for new State fields

### CLI modifications:
- `crates/iron-cli/src/commands/apply.rs` -- --prune flags, confirmation UX, risk display
- `crates/iron-cli/src/commands/plan.rs` -- risk badges in plan output
- `crates/iron-cli/src/cli.rs` -- new flags on Apply command

### TUI modifications:
- `crates/iron-tui/src/ui/apply.rs` -- render new action types, risk color-coding
- `crates/iron-tui/src/app/actions.rs` -- if apply action dispatch needs updating

### Infrastructure:
- `crates/iron-fs/src/lib.rs` -- possibly new copy-with-backup helper (or reuse existing)

---

## 7. Key Architectural Questions for Architect

1. **Managed tracking granularity**: Should `managed_packages` be a `Vec<String>` (simple) or `HashSet<String>` (O(1) lookup, matches ActualState's `installed_packages`)? The technical guide says `Vec<String>` but Sprint 3.1 used `HashSet` for `installed_packages`.

2. **Bootstrap strategy**: On first apply after upgrade, how to populate `managed_packages`? The tech guide says "mark all installed+desired packages as managed." Should this include AUR packages? What about packages installed by bundle vs module?

3. **Template detection timing**: Should template detection happen during `compute_plan()` (reads file content at plan time) or earlier during `resolve_desired_state()`? Reading file content at plan time means the plan phase does I/O beyond TOML parsing.

4. **RiskLevel on existing variants**: The tech guide gives specific risk levels for each action. Should `CreateSymlink` with an existing target be `Destructive` (overwrites) vs `Additive` (new symlink)?

5. **Prune flag architecture**: Should prune flags live on `ApplyPlan` (computed during planning, affecting which actions are generated) or on the execute step (all removal actions are always planned, but only executed when prune flags allow)?

6. **Confirmation UX**: The tech guide describes three confirmation tiers (simple/detailed/typed). Should the confirmation logic live in the CLI command handler (output.rs helpers) or in a shared service?

---

## 8. Instructions for Analyst

Begin by reading:
1. This orchestrator document (you are here)
2. `docs/phase3-kanban.md` -- Sprint 3.2 section (line 312-484) for acceptance criteria
3. `docs/phase3-technical-guide.md` -- Section 3 (line 498-711) for implementation specs
4. `docs/iterations/enhancement-phase3-sprint3.1/review.md` -- SHOULD findings to carry forward
5. Current source files:
   - `crates/iron-core/src/services/apply.rs` -- current ApplyAction, compute_plan, execute
   - `crates/iron-core/src/state.rs` -- current IronState struct (line 150)
   - `crates/iron-core/src/services/state.rs` -- StateManager
   - `crates/iron-fs/src/lib.rs` -- template module (line 622)
   - `crates/iron-core/src/packages.rs` -- PackageManager trait (remove method exists)
   - `crates/iron-core/src/system_service.rs` -- SystemService trait (disable_service exists)

Your deliverable: a requirements document at `docs/iterations/enhancement-phase3-sprint3.2/analyst.md` with:
- Numbered acceptance criteria for each of the 8 tasks
- Dependency ordering (Wave 1/2/3)
- Answers or analyst-perspective positions on the 6 architectural questions above
- Which Sprint 3.1 SHOULD findings to address in this sprint
- Test strategy requirements (unit test counts, integration test expectations)
- Explicit list of files expected to be modified per task
- Edge cases and safety constraints (especially for removal actions)

---

## 9. Conventions Reminder

- All new struct fields: `#[serde(default)]` for backward compat
- All test helpers updated when struct fields change
- CLI integration tests: always use `--dry-run`
- No raw `Command::new()` -- use CommandExecutor/trait abstractions
- Rust 2024 edition (let-chains, if-let chains allowed)
- Line width: 100 chars
- New TUI View variants: update ALL 7 exhaustive match sites
