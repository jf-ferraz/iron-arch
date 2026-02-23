# Analyst Report -- Sprint 3.2 (Full Declarative Convergence)

**Date:** 2026-02-23
**Type:** ENHANCEMENT (structural)
**Analyst:** Claude Opus 4.6

---

## 1. Scope Verification

### 1.1 Task ID Cross-Reference

The orchestrator lists 8 tasks. Cross-referencing against the kanban (`docs/phase3-kanban.md` lines 312-484) and technical guide (`docs/phase3-technical-guide.md` section 3, lines 498-711):

| Orchestrator Task | Kanban ID | Tech Guide Section | Title | Verified |
|---|---|---|---|---|
| F3-021 | F3-021 (line 320) | 3.0 (line 500) | Managed resource tracking | YES |
| F3-008 | F3-008 (line 347) | 3.1 (line 539) | Template variable rendering in apply | YES |
| F3-009 | F3-009 (line 370) | 3.2 (line 577) | File copy deployment mode (CopyFile) | YES |
| F3-010 | F3-010 (line 396) | 3.3 (line 623) | Package removal (RemovePackages) | YES |
| F3-011 | F3-011 (line 419) | 3.4 (line 657) | Service disable (DisableService) | YES |
| F3-012 | F3-012 (line 441) | 3.4 (line 657) | Symlink/module removal | YES |
| F3-013 | F3-013 (line 464) | 3.5 (line 661) | Risk levels on ApplyAction | YES |
| F3-016 | Not in kanban S3.2 | -- | `iron apply --confirm` UX flow | SEE NOTE |

**Note on F3-016 ("iron apply --confirm UX"):** The kanban F3-016 is `iron history` in Sprint 3.3, not confirmation UX. The confirmation UX is the last part of F3-013's acceptance criteria (line 478: "Confirmation prompt scales: Additive -> simple, Destructive -> detailed, Critical -> typed"). The orchestrator created this as a separate task, which is reasonable given the scope. I will treat the confirmation UX as an extension of F3-013, not a separate kanban task. The sprint scope is effectively **7 kanban tasks + 1 orchestrator-scoped UX task** embedded in F3-013.

### 1.2 Ambiguities Flagged

**AMB-1: Prune flag architecture.** The technical guide (line 649) says "removal actions are shown in the plan but skipped during execution." The orchestrator (question 5, line 159) asks whether prune flags live on `ApplyPlan` or the execute step. This needs an architect decision. My position: prune flags should affect **plan computation**, not execution. `compute_plan()` should accept prune flags and conditionally include removal actions. Rationale: the plan should be the single source of truth for what will happen. An execute step that silently skips planned actions violates the "plan is what you get" contract that `iron plan` establishes.

**AMB-2: Bootstrap strategy for managed_packages.** The technical guide (line 537) says "On first iron apply after upgrade, all installed packages that match the desired state are recorded as managed." What happens if `managed_packages` is empty (first apply on upgraded system) AND there are packages in the desired state that are already installed? Should they be recorded as managed immediately? My position: YES. On the first apply where `managed_packages` is empty, every desired package that is currently installed should be seeded into `managed_packages`. This is a one-time bootstrap. The `managed_packages.is_empty()` condition gates this behavior.

**AMB-3: Template detection timing.** The orchestrator (question 3, line 155) asks whether template detection happens at plan time or desired-state resolution time. My position: plan time (`compute_plan()`). Template detection requires reading file content from disk, which is I/O. `resolve_desired_state()` only reads TOML config files. Adding arbitrary file reads there would break its performance contract (used by `iron status` without `--full`). The `compute_plan()` method already does I/O (via `scan_actual_state`), so reading dotfile sources there is consistent.

**AMB-4: `DotfileMapping.link` field semantics.** The `DotfileMapping.link` field (line 98 of `module.rs`) defaults to `true`. When `link = false`, F3-009 says to use `CopyFile`. But what about template files? A file with `link = true` that contains `{{variables}}` should become `RenderAndCopy` (not a symlink). Priority: template detection overrides the `link` field. The decision tree should be:
1. If file contains `{{...}}` -> `RenderAndCopy` (regardless of `link` field)
2. If `link = false` -> `CopyFile`
3. If `link = true` (default) -> `CreateSymlink`

---

## 2. Codebase Impact Analysis

### 2.1 F3-021: Managed Resource Tracking

**New fields on `IronState`** (`crates/iron-core/src/state.rs`, line 148-181):

Add after `last_scan_report` (line 180):
```rust
#[serde(default)]
pub managed_packages: Vec<String>,

#[serde(default)]
pub managed_services: Vec<String>,

#[serde(default)]
pub managed_dotfiles: Vec<String>,

#[serde(default, skip_serializing_if = "Option::is_none")]
pub last_apply: Option<DateTime<Utc>>,
```

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/state.rs` | Add 4 new fields to `IronState` struct | After line 180 |
| `crates/iron-core/src/services/state.rs` | Add methods: `record_managed_packages()`, `record_managed_service()`, `record_managed_dotfile()`, `unrecord_managed_package()`, `unrecord_managed_service()`, `unrecord_managed_dotfile()`, `managed_packages()`, `managed_services()`, `managed_dotfiles()`, `update_last_apply()` | New methods near line 660 |
| `crates/iron-core/src/services/apply.rs` | In `execute_action()` (line 741), after each successful action, call the appropriate `record_managed_*` method. Update `execute()` (line 475) to set `last_apply` timestamp after completion. | Lines 741-801, 475-503 |
| `crates/iron-core/src/test_helpers.rs` | NOT directly impacted -- `IronState` uses `#[serde(default)]` so empty `state.json` files (`{}`) still deserialize. However, any test that constructs `IronState` directly (none found -- tests use `StateManager::new()`) would need updating. |
| `crates/iron-cli/src/commands/status.rs` | Display managed resource counts and `last_apply` timestamp (addresses SHOULD-4 from Sprint 3.1 review). | Wherever `PackagesStatus` is built |

**Existing infrastructure:**
- `StateManager::with_locked_state()` (line 539) -- use for atomic managed list updates
- `StateManager::persist()` (line 562) -- already handles serialization
- `IronState` already uses `#[serde(default)]` pattern on all optional fields

### 2.2 F3-008: Template Variable Rendering in Apply

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `RenderAndCopy` variant to `ApplyAction` enum (line 244). Modify `compute_plan()` dotfile section (lines 650-708) to read source file content and check `iron_fs::template::has_variables()`. If true, produce `RenderAndCopy` action instead of `CreateSymlink`. Add `execute_action()` match arm for `RenderAndCopy`. Update `display()` (line 264), `summary()` (line 309). | Lines 244-260, 650-708, 741-801, 264-295, 309-338 |
| `crates/iron-core/src/services/apply.rs` | `compute_plan()` needs access to `self.iron_root` (already has it) and `desired.variables` (already in DesiredState) to pass to the RenderAndCopy action. | Line 621 |
| `crates/iron-cli/src/commands/plan.rs` | Update grouped display to handle `RenderAndCopy` actions alongside `CreateSymlink` in the "Dotfiles" section. Update the `filter` patterns (lines 70-74). | Lines 60-84 |
| `crates/iron-tui/src/ui/apply.rs` | Minimal impact -- TUI currently shows only plan count, not individual actions (line 19). Future enhancement. |

**Existing infrastructure that is ALREADY available:**
- `iron_fs::template::has_variables(content: &str) -> bool` -- line 677 of `crates/iron-fs/src/lib.rs`
- `iron_fs::template::render(content: &str, vars: &HashMap<String, String>) -> String` -- line 630
- `iron_fs::template::extract_variables(content: &str) -> Vec<String>` -- line 682
- `DesiredState.variables: HashMap<String, String>` -- line 46 of apply.rs
- `builtin_variables(iron_root)` -- line 212 of apply.rs, already populates `hostname`, `username`, `home`, `config_dir`, `iron_root`

**What is truly new:**
- The `RenderAndCopy` variant on `ApplyAction`
- The template detection logic in `compute_plan()`
- The execute logic for `RenderAndCopy` (read source, render, backup target, write rendered content)

### 2.3 F3-009: File Copy Deployment Mode (CopyFile)

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `CopyFile { source: String, target: String, backup_existing: bool }` variant to `ApplyAction` (line 244). In `compute_plan()` dotfile section, when `!has_templates && !dotfile.link`, produce `CopyFile`. Add `execute_action()` match arm. Update `display()`, `summary()`. | Lines 244-260, 650-708, 741-801, 264-295, 309-338 |
| `crates/iron-cli/src/commands/plan.rs` | Handle `CopyFile` in Dotfiles display section. | Lines 70-132 |

**Existing infrastructure:**
- `DotfileMapping.link: bool` field -- already exists (line 98 of `module.rs`), defaults to `true`, but is currently ignored in `compute_plan()` (line 686 always produces `CreateSymlink`).

### 2.4 F3-010: Package Removal (RemovePackages)

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `RemovePackages { packages: Vec<String> }` variant to `ApplyAction` (line 244). In `compute_plan()`, after the existing package install diff (lines 624-647), add removal diff: `managed AND installed AND NOT desired -> RemovePackages`. Add `execute_action()` match arm calling `self.package_manager.remove()`. Update `display()`, `summary()`. | Lines 244-260, 624-647, 741-801, 264-295, 309-338 |
| `crates/iron-core/src/services/apply.rs` | `compute_plan()` needs access to `managed_packages` from StateManager. Currently the method does not have `&self.state_manager` access within compute_plan (it's a private method on `DefaultApplyService` which has `state_manager`). It already uses `self.state_manager.active_modules()` at line 728, so accessing managed_packages is straightforward. | Line 621 |
| `crates/iron-cli/src/commands/apply.rs` | Add `--prune`, `--prune-packages`, `--prune-services`, `--prune-dotfiles` flags. Pass these through to the plan/execute flow. | Line 11 |
| `crates/iron-cli/src/cli.rs` | Add prune flags to `Commands::Apply` variant. | Lines 159-172 |
| `crates/iron-cli/src/main.rs` | Update dispatch to pass prune flags. |
| `crates/iron-cli/src/commands/plan.rs` | Display `RemovePackages` actions with `-` indicators and "Critical" badge. | Lines 60-84 |

**Existing infrastructure:**
- `PackageManager::remove(packages: &[String], remove_deps: bool) -> IronResult<()>` -- line 122 of `crates/iron-core/src/packages.rs`
- `NoopPackageManager::remove()` -- already implemented (line 192)
- All mock PackageManagers in tests already implement `remove()` (confirmed in `actual_state.rs` line 230)

### 2.5 F3-011: Service Disable (DisableService)

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `DisableService { name: String }` variant. In `compute_plan()` service section (lines 710-724), add: `managed AND enabled AND NOT desired -> DisableService`. Add `execute_action()` match arm calling `self.service_manager.disable_service()`. Update `display()`, `summary()`. | Lines 244-260, 710-724, 741-801, 264-295, 309-338 |

**Existing infrastructure:**
- `SystemService::disable_service(name: &str) -> IronResult<()>` -- line 19 of `system_service.rs`
- `NoopSystemService::disable_service()` -- already implemented (line 43)
- All mock SystemService impls in tests already implement `disable_service()` (confirmed in `actual_state.rs` line 259)

### 2.6 F3-012: Symlink/Module Removal (RemoveSymlink, DeactivateModule)

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `RemoveSymlink { target: String }` and `DeactivateModule { id: String }` variants. In `compute_plan()`, add: managed dotfiles NOT in desired -> `RemoveSymlink`. Active modules NOT in desired -> `DeactivateModule`. Add `execute_action()` match arms. Update `display()`, `summary()`. | Lines 244-260, after 735, 741-801, 264-295, 309-338 |

**Existing infrastructure:**
- `std::fs::remove_file()` for symlink removal
- `StateManager::disable_module()` at line 315 of `state.rs` for deactivation

### 2.7 F3-013: Risk Levels on ApplyAction + Confirmation UX

**Files that MUST change:**

| File | Change | Line(s) |
|------|--------|---------|
| `crates/iron-core/src/services/apply.rs` | Add `RiskLevel` enum (new, distinct from `packages::RiskLevel` which is for update risk). Add `ApplyAction::risk_level() -> RiskLevel` method. Add `ApplyPlan::max_risk() -> RiskLevel` method. | After line 260 |
| `crates/iron-cli/src/commands/apply.rs` | Replace simple y/N prompt (lines 56-63) with risk-scaled confirmation: `Additive` -> simple y/N, `Destructive` -> show detailed list then confirm, `Critical` -> require typed "yes" confirmation. | Lines 50-64 |
| `crates/iron-cli/src/commands/plan.rs` | Add risk badges per action in display: `[+]` for Additive, `[!]` for Destructive, `[!!]` for Critical. | Lines 60-160 |
| `crates/iron-tui/src/ui/apply.rs` | Color-code actions by risk level (green/yellow/red). Minimal change since TUI currently doesn't display individual actions. | Future enhancement area |

**IMPORTANT:** `crates/iron-core/src/packages.rs` already has a `RiskLevel` enum (line 11) for package update risk assessment. The new `RiskLevel` for apply actions is a DIFFERENT concept (Additive/Destructive/Critical vs Low/Medium/High/Critical). The architect must decide naming. My recommendation: name the new one `ApplyRiskLevel` to avoid confusion, OR place the new enum in the `apply` module scope (which is what the tech guide does at line 664).

---

## 3. Existing Infrastructure Audit

### What Already Exists (NO new trait methods needed)

| Capability | Location | Status |
|---|---|---|
| `PackageManager::remove(packages, remove_deps)` | `packages.rs:122` | Trait + NoopPackageManager + all mocks |
| `SystemService::disable_service(name)` | `system_service.rs:19` | Trait + NoopSystemService + all mocks |
| `template::has_variables(content)` | `iron-fs/src/lib.rs:677` | Function, tested |
| `template::render(content, vars)` | `iron-fs/src/lib.rs:630` | Function, tested |
| `template::extract_variables(content)` | `iron-fs/src/lib.rs:682` | Function, tested |
| `DotfileMapping.link: bool` | `module.rs:98` | Field exists, defaults `true`, currently a no-op |
| `StateManager::disable_module(id)` | `state.rs:315` | Method, tested |
| `StateManager::with_locked_state(op)` | `state.rs:539` | Atomic state mutation |
| `DesiredState.variables` | `apply.rs:46` | Already populated with host + builtin vars |
| `builtin_variables(iron_root)` | `apply.rs:212` | `hostname`, `username`, `home`, `config_dir`, `iron_root` |
| `ActualState.installed_packages: HashSet<String>` | `actual_state.rs:24` | O(1) lookup |
| `ActualState.services: Vec<ActualServiceState>` | `actual_state.rs:32` | With `.enabled` field |
| `ActualState.managed_files: Vec<ActualFileState>` | `actual_state.rs:36` | With `.file_type`, `.exists` |
| `IronState` serde(default) pattern | `state.rs:148-181` | New fields auto-default to empty |

### What is Truly New

| New Component | Why It Cannot Be Derived From Existing Code |
|---|---|
| `IronState.managed_packages/services/dotfiles` | New fields; recording logic in execute flow is new |
| `IronState.last_apply` | New field; timestamp recording is new |
| `ApplyAction::RenderAndCopy` | New variant; execution logic (read, render, backup, write) is new |
| `ApplyAction::CopyFile` | New variant; execution logic (backup, copy) is new |
| `ApplyAction::RemovePackages` | New variant; plan computation logic (managed-installed-desired diff) is new |
| `ApplyAction::DisableService` | New variant; plan computation logic is new |
| `ApplyAction::RemoveSymlink` | New variant; plan computation logic is new |
| `ApplyAction::DeactivateModule` | New variant; plan computation logic is new |
| `RiskLevel` (for apply) | New enum; classification logic is new |
| `ApplyPlan::max_risk()` | New method |
| Prune flags on CLI | New CLI flags; conditional plan inclusion is new |
| Risk-scaled confirmation | New UX flow replacing simple y/N |

---

## 4. Dependency Validation

The orchestrator's 3-wave ordering is **correct** with one refinement:

```
Wave 1 (parallel, no dependencies):
  F3-021 (managed resource tracking)  -- prerequisite for F3-010/011/012
  F3-008 (template rendering)          -- independent of F3-021

Wave 2 (depends on Wave 1):
  F3-009 (CopyFile)         -- depends on F3-008 (template detection -> copy vs symlink decision tree)
  F3-010 (RemovePackages)   -- depends on F3-021 (needs managed_packages data)
  F3-011 (DisableService)   -- depends on F3-021 (needs managed_services data)
  F3-012 (RemoveSymlink)    -- depends on F3-021 (needs managed_dotfiles data)

Wave 3 (depends on Wave 2):
  F3-013 (RiskLevel + confirmation UX) -- depends on ALL Wave 2 tasks (must classify ALL action variants)
```

**Validation of dependency logic:**

- F3-010/011/012 MUST follow F3-021: The removal plan computation requires `state.managed_packages/services/dotfiles` to determine what Iron owns. Without this data, the system would either remove nothing (safe but useless) or remove everything not desired (dangerous). CONFIRMED.

- F3-009 MUST follow F3-008: The decision tree in `compute_plan()` for dotfiles is `has_templates? -> RenderAndCopy : (link=false? -> CopyFile : CreateSymlink)`. F3-009 (CopyFile) requires F3-008 (template detection) to be in place so the decision tree is correct. Without F3-008, all non-template `link=false` files would still get `CreateSymlink`. CONFIRMED.

- F3-013 MUST be last: The `risk_level()` method needs to match on ALL action variants including the 6 new ones from Wave 2. If implemented before Wave 2, it would need retroactive updates. CONFIRMED.

**Refinement:** F3-009 could technically be implemented in parallel with F3-010/011/012 since they touch different sections of `compute_plan()`. However, since F3-008 and F3-009 both modify the dotfile section, they should be done sequentially to avoid merge conflicts.

---

## 5. Risk Assessment

### Highest Risk Changes

**RISK-1: `ApplyAction` enum expansion (HIGH)**
- The `ApplyAction` enum is used in 6+ exhaustive match sites: `execute_action()`, `display()`, `summary()`, `plan.rs` display, and any future `matches!()` patterns. Adding 6 new variants requires updating ALL match sites atomically.
- **Mitigation:** Add all variants at once in Wave 2, even if execution logic is stubbed with `todo!()`. This prevents partial compilation failures.

**RISK-2: `compute_plan()` complexity (MEDIUM)**
- The `compute_plan()` function (line 621) is already 116 lines. Adding removal logic for 3 resource types (packages, services, dotfiles) plus template detection will roughly double its size.
- **Mitigation:** Extract helper methods: `plan_package_actions()`, `plan_dotfile_actions()`, `plan_service_actions()`, `plan_module_actions()`. Each handles both install and remove for its resource type.

**RISK-3: Prune flag threading through call chain (MEDIUM)**
- Prune flags must flow from CLI -> `ApplyService::plan()` -> `compute_plan()`. Currently `plan()` accepts only `host_id: &str`. The trait signature may need to change, or prune flags can be passed via a config/options struct.
- **Mitigation:** The architect should decide whether to change the `ApplyService` trait or use a separate mechanism (e.g., `PlanOptions` struct).

**RISK-4: Template file I/O at plan time (LOW)**
- `compute_plan()` will now read dotfile source contents from disk (for `has_variables()` check). This adds filesystem I/O to the planning phase.
- **Mitigation:** File reads are small (config files). Cache the content -- it is also needed for `RenderAndCopy` action construction. If a file is unreadable, fall back to `CreateSymlink` with a warning.

**RISK-5: `packages::RiskLevel` name collision (LOW)**
- A `RiskLevel` enum already exists in `packages.rs` for update risk. The new apply `RiskLevel` has different semantics (ReadOnly/Additive/Destructive/Critical vs Low/Medium/High/Critical).
- **Mitigation:** Scope the new enum inside `apply.rs` module or name it differently.

---

## 6. Implementation Order

### Recommended Order (within waves)

**Wave 1 (parallel):**
1. **F3-021** -- Managed resource tracking
   - Add `IronState` fields + `StateManager` methods
   - Wire recording into `execute_action()` for existing 5 variants
   - Add bootstrap logic
   - Add `last_apply` timestamp (addresses SHOULD-4)
2. **F3-008** -- Template variable rendering
   - Add `RenderAndCopy` variant to `ApplyAction`
   - Add template detection in `compute_plan()` dotfile section
   - Add `RenderAndCopy` execution logic

**Wave 2 (sequential within, after Wave 1):**
3. **F3-009** -- CopyFile action (pairs with F3-008, same code section)
4. **F3-010** -- RemovePackages action
5. **F3-011** -- DisableService action (parallel with F3-010 if separate developer)
6. **F3-012** -- RemoveSymlink + DeactivateModule (parallel with F3-010/011)

**Rationale for F3-010 before F3-011/012:** Package removal is the highest-value removal feature (most common drift type) and exercises the managed tracking infrastructure most thoroughly. F3-011 and F3-012 follow the same pattern.

**Wave 3:**
7. **F3-013** -- RiskLevel enum + classification + confirmation UX

### Prune Flags Integration

Prune flags should be added with F3-010 (first removal task). The flag infrastructure (CLI args, threading to compute_plan) is needed by all three removal tasks, so it is natural to introduce it with the first one.

---

## 7. Test Strategy

### New Tests Needed

| Task | New Unit Tests | Key Test Scenarios |
|------|---------------|-------------------|
| F3-021 | 8+ | Record packages after install, record service after enable, record dotfile after symlink, unrecord after removal, bootstrap (managed empty + desired installed -> seed), persistence across `StateManager` reload, concurrent recording via `with_locked_state`, `last_apply` timestamp update |
| F3-008 | 8+ | Template detection (`has_variables` on dotfile source), `RenderAndCopy` planned for template file, `CreateSymlink` still used for non-template, `RenderAndCopy` execution (renders variables into output), built-in variables available (`hostname`, `username`, `home`), unreadable source falls back to `CreateSymlink`, empty template renders correctly, variables from host override built-ins |
| F3-009 | 4+ | `CopyFile` planned when `link=false` and no templates, `CopyFile` execution creates file copy, backup created when `backup_existing=true`, parent directories created |
| F3-010 | 8+ | Removal candidates computed correctly (managed AND installed AND NOT desired), unmanaged packages never in removal candidates, removal shown in plan without `--prune` (hint message), removal executed with `--prune`, `--prune-packages` enables only package removal, `managed_packages` updated after removal, empty managed_packages triggers bootstrap, AUR packages included in removal candidates |
| F3-011 | 4+ | Service disable candidates computed (managed AND enabled AND NOT desired), disable executed with `--prune-services`, `managed_services` updated after disable, services not in `managed_services` are never disabled |
| F3-012 | 5+ | Symlink removal candidates computed (managed AND exists AND NOT desired), `RemoveSymlink` execution backs up then removes, `DeactivateModule` execution calls `disable_module()`, `managed_dotfiles` updated after removal, `--prune-dotfiles` flag controls execution |
| F3-013 | 6+ | Risk classification for each of the 11 action variants, `max_risk()` returns highest, `ReadOnly` for empty plan, `risk_level()` returns `Additive` for `InstallPackages`, `Critical` for `RemovePackages`, `Destructive` for `RenderAndCopy` |

**Total new tests: ~43+**

### Existing Tests That Will Need Updates

| File | Test(s) | Reason |
|------|---------|--------|
| `crates/iron-core/src/services/apply.rs` | `test_apply_plan_summary` (line 991) | `summary()` method will have new categories for removal/copy actions |
| `crates/iron-core/src/services/apply.rs` | `test_apply_action_display` (line 1024) | New display variants need coverage |
| `crates/iron-cli/src/commands/apply.rs` | `test_plan_summary_format` (line 108) | May need update if summary format changes |
| `crates/iron-cli/src/commands/plan.rs` | `test_plan_groups_by_type` (line 170) | New action types need to be included in the plan fixture |
| `crates/iron-cli/tests/cli_integration.rs` | Apply-related tests | New `--prune` flags need integration test coverage |

### CLI Integration Tests (all must use --dry-run)

```
iron apply --dry-run                        # existing, still works
iron apply --prune --dry-run                # new flag accepted
iron apply --prune-packages --dry-run       # new flag accepted
iron apply --prune-services --dry-run       # new flag accepted
iron apply --prune-dotfiles --dry-run       # new flag accepted
iron plan --dry-run                         # existing, still works
```

---

## 8. Acceptance Criteria

### F3-021: Managed Resource Tracking

- **AC-021-1:** `IronState` has `managed_packages: Vec<String>`, `managed_services: Vec<String>`, `managed_dotfiles: Vec<String>` fields, all with `#[serde(default)]`.
- **AC-021-2:** `IronState` has `last_apply: Option<DateTime<Utc>>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- **AC-021-3:** After successful `InstallPackages` execution, packages are added to `managed_packages` (no duplicates).
- **AC-021-4:** After successful `EnableService` execution, service name is added to `managed_services`.
- **AC-021-5:** After successful `CreateSymlink`/`CopyFile`/`RenderAndCopy` execution, target path is added to `managed_dotfiles`.
- **AC-021-6:** After successful `RemovePackages` execution, packages are removed from `managed_packages`.
- **AC-021-7:** After successful `DisableService` execution, service is removed from `managed_services`.
- **AC-021-8:** After successful `RemoveSymlink` execution, target is removed from `managed_dotfiles`.
- **AC-021-9:** Bootstrap: when `managed_packages` is empty and desired packages exist, all currently-installed desired packages are seeded into `managed_packages`.
- **AC-021-10:** `last_apply` timestamp set after `execute()` completes.
- **AC-021-11:** `iron status` displays managed resource counts and `last_apply` timestamp (addresses SHOULD-4).
- **AC-021-12:** 8+ unit tests covering record, unrecord, bootstrap, persistence.

### F3-008: Template Variable Rendering in Apply

- **AC-008-1:** `ApplyAction::RenderAndCopy { source: String, target: String, variables: HashMap<String, String>, module_id: String }` variant exists.
- **AC-008-2:** During `compute_plan()`, dotfiles whose source file contains `{{...}}` patterns produce `RenderAndCopy` actions.
- **AC-008-3:** Dotfiles without template patterns continue producing `CreateSymlink` actions (no regression).
- **AC-008-4:** `RenderAndCopy` execution: reads source, renders via `iron_fs::template::render()`, backs up existing target, writes rendered content.
- **AC-008-5:** `DesiredState.variables` (host vars + built-ins) are passed to the template engine.
- **AC-008-6:** Built-in variables available: `hostname`, `username`, `home`, `config_dir`, `iron_root` (already in `builtin_variables()`).
- **AC-008-7:** Unreadable source file produces a warning and falls back to `CreateSymlink`.
- **AC-008-8:** `display()` method returns human-readable text for `RenderAndCopy`.
- **AC-008-9:** 8+ unit tests.

### F3-009: File Copy Deployment Mode

- **AC-009-1:** `ApplyAction::CopyFile { source: String, target: String, backup_existing: bool, module_id: String }` variant exists.
- **AC-009-2:** When `DotfileMapping.link = false` and file has no templates, `CopyFile` action produced.
- **AC-009-3:** `CopyFile` execution: backup existing target if `backup_existing`, copy source to target, create parent dirs.
- **AC-009-4:** `display()` and `summary()` updated.
- **AC-009-5:** 4+ unit tests.

### F3-010: Package Removal

- **AC-010-1:** `ApplyAction::RemovePackages { packages: Vec<String> }` variant exists.
- **AC-010-2:** Plan computation: packages in `managed_packages` AND in `actual.installed_packages` AND NOT in `desired.packages` produce `RemovePackages`.
- **AC-010-3:** Packages NOT in `managed_packages` are NEVER included in `RemovePackages` (safety invariant).
- **AC-010-4:** Without `--prune`/`--prune-packages`, removal actions are included in plan output with a hint message but NOT executed.
- **AC-010-5:** With `--prune` or `--prune-packages`, removal actions are executed.
- **AC-010-6:** Execution calls `PackageManager::remove(packages, false)`.
- **AC-010-7:** After execution, `managed_packages` updated (removed packages removed from list).
- **AC-010-8:** `display()` and `summary()` updated.
- **AC-010-9:** 8+ unit tests.

### F3-011: Service Disable

- **AC-011-1:** `ApplyAction::DisableService { name: String }` variant exists.
- **AC-011-2:** Plan: services in `managed_services` AND enabled AND NOT in `desired.services` produce `DisableService`.
- **AC-011-3:** Requires `--prune` or `--prune-services` to execute.
- **AC-011-4:** Execution calls `SystemService::disable_service(name)`.
- **AC-011-5:** After execution, `managed_services` updated.
- **AC-011-6:** 4+ unit tests.

### F3-012: Symlink/Module Removal

- **AC-012-1:** `ApplyAction::RemoveSymlink { target: String }` variant exists.
- **AC-012-2:** `ApplyAction::DeactivateModule { id: String }` variant exists.
- **AC-012-3:** Plan: managed dotfiles (in `managed_dotfiles`) whose module is no longer active produce `RemoveSymlink`.
- **AC-012-4:** Plan: active modules NOT in `desired.modules` produce `DeactivateModule`.
- **AC-012-5:** Requires `--prune` or `--prune-dotfiles` to execute.
- **AC-012-6:** `RemoveSymlink` execution: backup target, remove symlink/file.
- **AC-012-7:** `DeactivateModule` execution: calls `StateManager::disable_module()`.
- **AC-012-8:** After execution, `managed_dotfiles` updated.
- **AC-012-9:** 5+ unit tests.

### F3-013: Risk Levels + Confirmation UX

- **AC-013-1:** `RiskLevel` enum in `apply.rs`: `ReadOnly`, `Additive`, `Destructive`, `Critical` with `PartialOrd` + `Ord`.
- **AC-013-2:** `ApplyAction::risk_level()` returns correct level for all 11 variants:
  - `Additive`: InstallPackages, InstallAurPackages, CreateSymlink, EnableService, ActivateModule
  - `Destructive`: CopyFile(backup=true), RenderAndCopy, RemoveSymlink, DisableService, DeactivateModule
  - `Critical`: RemovePackages
  - `Additive`: CopyFile(backup=false)
- **AC-013-3:** `ApplyPlan::max_risk()` returns highest risk across all actions (or `ReadOnly` for empty plan).
- **AC-013-4:** Plan output (CLI `plan` and `apply --dry-run`) shows risk badge per action: `[+]` Additive, `[!]` Destructive, `[!!]` Critical.
- **AC-013-5:** Confirmation scales with risk:
  - `ReadOnly`: no confirmation
  - `Additive`: simple "Proceed? [y/N]"
  - `Destructive`: show detailed action list, then "Review changes above. Proceed? [y/N]"
  - `Critical`: "Type 'yes' to confirm critical changes: "
- **AC-013-6:** `--json` plan output includes `risk_level` field per action.
- **AC-013-7:** 6+ unit tests.

---

## 9. Architectural Questions for Architect

**AQ-1: Prune flag threading mechanism.**
How should prune flags reach `compute_plan()`? Options:
- (a) Add `PlanOptions` struct passed alongside `host_id` to `plan()` trait method
- (b) Add prune fields to `ApplyService` builder (set at construction time)
- (c) Change `ApplyService::plan()` signature to `plan(&self, host_id: &str, options: &PlanOptions)`

My recommendation: (a) with a trait signature change. The `PlanOptions` struct can also hold the `module` filter, replacing the current `plan_module()` method.

**AQ-2: `managed_packages` data type.**
`Vec<String>` (tech guide) vs `HashSet<String>` (consistent with `ActualState.installed_packages`). The managed lists need:
- Membership testing (is this package managed?) -- `HashSet` is O(1)
- Serialization to JSON -- both work
- Ordered iteration for display -- `Vec` preserves insertion order; `HashSet` does not

My recommendation: Use `Vec<String>` for serialization stability and display ordering, but convert to `HashSet` for membership testing in `compute_plan()`.

**AQ-3: RiskLevel naming.**
`packages::RiskLevel` (Low/Medium/High/Critical) already exists for update risk. The new apply risk level (ReadOnly/Additive/Destructive/Critical) is a different concept. Options:
- (a) Name the new one `ApplyRiskLevel` to distinguish
- (b) Keep it as `RiskLevel` scoped within `apply.rs` module
- (c) Unify into a single `RiskLevel` with all variants

My recommendation: (b) -- keep it scoped in `apply.rs`. Users access it via `apply::RiskLevel`. The `packages::RiskLevel` is only used in the update flow. No confusion in practice.

**AQ-4: Should `compute_plan()` read dotfile source files?**
Template detection requires reading dotfile source content from disk during planning. This is I/O in the planning phase. Is this acceptable?

My position: YES. `compute_plan()` already performs I/O indirectly (via `ActualState::scan()` which reads filesystem, checksums, etc.). Reading a few dozen dotfile source files is negligible compared to pacman queries. The alternative (deferring to execution time) would mean the plan cannot show whether a dotfile will be symlinked or rendered, defeating the purpose of `iron plan`.

**AQ-5: Should the `ApplyService` trait change?**
Currently the trait has `plan(host_id)` and `plan_module(module_id)`. With prune flags, these need to be extended. Options:
- (a) Add `PlanOptions` parameter to `plan()` and `plan_module()`
- (b) Keep trait unchanged, make prune a property of `DefaultApplyService` (set via builder)
- (c) Add separate `plan_with_options()` method

Sprint 3.1 kept trait signatures unchanged (architect decision AQ-2 in that sprint). The same approach could work here: prune flags set on `DefaultApplyService` via builder, internal `compute_plan()` reads from `self`. But this means the trait doesn't express the full API. The architect should decide.

---

## 10. Sprint 3.1 SHOULD Findings Disposition

| Finding | Disposition | Rationale |
|---------|------------|-----------|
| SHOULD-1: 8 unmigrated `output.json()` calls | DEFER | Sprint 3.2 does not touch host.rs, profile.rs, bundle.rs, sync.rs. These files are out of scope. Migrate opportunistically if touching them for other reasons. |
| SHOULD-2: `json_error_envelope` unused | ADDRESS IF POSSIBLE | The new risk-scaled confirmation UX in F3-013 creates new error paths (e.g., user types wrong confirmation). If adding `--json` error handling, wire `json_error_envelope` into apply command error paths. |
| SHOULD-3: Info lines bleed into JSON | DEFER | Systemic issue not caused by this sprint. Would require changing `Output::info()` to route to stderr in JSON mode -- a separate concern. |
| SHOULD-4: Missing "last apply timestamp" in status | ADDRESS in F3-021 | Adding `last_apply` to `IronState` and displaying in `iron status` is natural scope for F3-021 (managed tracking). |

### COULD findings from Sprint 3.1 review:

| Finding | Disposition | Rationale |
|---------|------------|-----------|
| COULD-1: `ActualState::scan_from_desired()` convenience | CONSIDER | Both `apply.rs` and `drift.rs` have nearly identical `scan_actual_state()` helpers (lines 590-614 and 178-202). If Sprint 3.2 adds more consumers, this convenience method becomes more valuable. Recommend implementing if the developer touches both files. |
| COULD-2: `ManagedFileSpec::expected_source` utility | EVALUATE | This field is `#[allow(dead_code)]`. Sprint 3.2's symlink correctness checks (F3-012) could benefit from knowing the expected source when determining if a managed dotfile is still correct. |

---

## 11. Summary

Sprint 3.2 transforms Iron from an additive-only system to one that can fully converge to declared state, including removing resources no longer declared. The sprint adds 6 new `ApplyAction` variants, managed resource tracking in state, template rendering in the apply pipeline, risk-level classification, and risk-scaled confirmation UX.

**Key numbers:**
- 6 new `ApplyAction` variants
- 4 new `IronState` fields
- ~10 new `StateManager` methods
- ~43+ new unit tests
- 7 primary files modified in `iron-core`
- 3 primary files modified in `iron-cli`
- 3 waves of implementation

**Critical path:** F3-021 (managed tracking) is the hard prerequisite. Without it, none of the removal tasks (F3-010/011/012) can be implemented. The wave ordering is validated and correct.
