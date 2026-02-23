# Validation Report -- Sprint 3.2 (Full Declarative Convergence)

## Summary
- **Type**: ENHANCEMENT (structural)
- **Status**: APPROVED_WITH_NOTES
- **MUST findings**: 0
- **SHOULD findings**: 5
- **COULD findings**: 3

---

## MUST Findings

No blocking issues found.

All 8 tasks (F3-021, F3-008, F3-009, F3-010, F3-011, F3-012, F3-013/RiskLevel, F3-016/Confirmation UX) are implemented with correct logic, appropriate test coverage, and no regressions.

---

## SHOULD Findings

### SHOULD-1: AC-021-11 not implemented (iron status managed resource display)

**File**: `crates/iron-cli/src/commands/status.rs` (not modified)
**Observation**: The analyst specified AC-021-11: "iron status displays managed resource counts and last_apply timestamp (addresses SHOULD-4 from Sprint 3.1)." All three wave change documents explicitly flag this as out-of-scope. The `IronState` fields exist and are populated, but `iron status` does not display them.
**Recommendation**: Implement in the next sprint or as a standalone patch. The data is available via `state.managed_packages.len()` etc. This is a display-only change.

### SHOULD-2: AC-013-6 not implemented (JSON plan output with risk_level field)

**File**: `crates/iron-cli/src/commands/plan.rs` line 64-67
**Observation**: The analyst specified AC-013-6: "--json plan output includes risk_level field per action." The JSON envelope output serializes the `ApplyPlan` directly, but `ApplyAction` does not have a `risk_level` field in its `Serialize` impl -- `risk_level()` is a method, not a serialized field. JSON consumers cannot see the risk classification.
**Recommendation**: Add `#[serde(serialize_with = "...")]` or a wrapper struct that includes `risk_level` in the JSON output. Alternatively, add a `risk_level` computed field to the `Serialize` impl using a custom serializer.

### SHOULD-3: Formatting issues in new code

**Files**: `crates/iron-core/src/services/state.rs` (lines ~2033, 2043, 2063)
**Observation**: `cargo fmt --all -- --check` reports formatting differences in the new test code added by the developer/tester. These are minor whitespace/line-wrapping issues (assert macro arguments on single vs multiple lines).
**Recommendation**: Run `cargo fmt --all` to fix. Non-blocking.

### SHOULD-4: has_template_variables is overly broad

**File**: `crates/iron-core/src/services/apply.rs` line 1444-1446
**Observation**: The template detection function `has_template_variables()` uses `content.contains("{{")` which will match any file containing `{{` regardless of whether it is actually a template variable (e.g., JSON files with `{{"key": "value"}}`, Jinja2 comments `{# ... #}` near `{{`, or literal `{{` in documentation). The iron-fs version (`iron_fs::template::has_variables`) may have the same behavior, but the inline version duplicates logic without referencing the canonical implementation.
**Recommendation**: This is acceptable for now since most dotfiles are config files where `{{` is uncommon outside template usage. Document the false-positive risk. Consider checking for `{{...}}` with a closing `}}` pattern in a future iteration.

### SHOULD-5: Temporal contamination in comments

**File**: `crates/iron-core/src/services/apply.rs` line 261
**Observation**: The comment `// -- Sprint 3.2: New variants --` is temporal contamination. A first-time reader should not need to know which sprint introduced which variants.
**File**: `crates/iron-core/src/services/apply.rs` line 1325
**Observation**: The comment `// -- Sprint 3.2 new variants --` same issue.
**File**: `crates/iron-core/src/services/apply.rs` line 287
**Observation**: The doc comment `/// Risk classification for apply actions (Sprint 3.2, F3-013).` contains sprint reference.
**Recommendation**: Remove sprint references from code comments. The git history preserves this information. Comments should describe what the code does, not when it was added.

---

## COULD Findings

### COULD-1: Duplicate backup/mkdir pattern across execute_action arms

**File**: `crates/iron-core/src/services/apply.rs` lines 1280-1294, 1333-1360, 1374-1395, 1418-1428
**Observation**: Four `execute_action` match arms (CreateSymlink, RenderAndCopy, CopyFile, RemoveSymlink) repeat the same pattern: create parent dirs, backup existing file, then operate. This could be extracted into a `prepare_target(target_path) -> IronResult<()>` helper.
**Suggestion**: Extract a shared `prepare_target_path()` method to reduce duplication across the four file-operation match arms.

### COULD-2: Module lookup in compute_plan is O(n*m) for dotfiles

**File**: `crates/iron-core/src/services/apply.rs` lines 972-987
**Observation**: For each dotfile that needs action, `compute_plan()` iterates over all modules and loads each module from disk to find which module owns the dotfile. With many modules and dotfiles, this is O(dotfiles * modules * disk_reads). A pre-built `HashMap<target, module_id>` would reduce this to O(1) lookup.
**Suggestion**: Build a `dotfile_target_to_module: HashMap<String, String>` at the start of `compute_plan()` by iterating modules once.

### COULD-3: PrunePolicy on plan command has no effect on display

**File**: `crates/iron-cli/src/commands/plan.rs` lines 31-41
**Observation**: The `plan` command accepts prune flags and sets them on the service, but since `compute_plan()` always includes removal actions regardless of prune policy (per architect decision AQ-1), the prune flags on `iron plan` have no visible effect on output. The prune policy only gates execution in `execute()`. The flags are accepted but functionally inert for the plan command.
**Suggestion**: Either document this behavior explicitly in the `--prune` flag help text for `iron plan`, or remove the prune flags from the Plan command since they have no effect there. Currently they may mislead users into thinking `iron plan --prune` shows different results than `iron plan`.

---

## Per-Task Assessment

### F3-021: Managed Resource Tracking -- PASS

- `IronState` has `managed_packages`, `managed_services`, `managed_dotfiles` (Vec<String>, #[serde(default)]) and `last_apply` (Option<DateTime<Utc>>, #[serde(default, skip_serializing_if)]): verified at `crates/iron-core/src/state.rs` lines 182-199.
- 10 StateManager methods implemented: verified at `crates/iron-core/src/services/state.rs`.
- `record_managed_resource()` and `unrecord_*` wired into `execute()`: verified at `crates/iron-core/src/services/apply.rs` lines 764-777, 1167-1196.
- Bootstrap logic seeds from desired intersect actual: verified at lines 1203-1262.
- `last_apply` timestamp set after execute: verified at line 781.
- Backward compatibility test exists: verified at state.rs line 1921.
- 20+ unit tests (exceeds requirement of 8+).

### F3-008: Template Variable Rendering -- PASS

- `RenderAndCopy` variant with `source`, `target`, `variables`, `module_id`: verified at lines 264-269.
- Template detection in `compute_plan()` using `has_template_variables()`: verified at lines 998-1001.
- Decision tree (template -> RenderAndCopy, !link -> CopyFile, else -> CreateSymlink): verified at lines 1007-1029.
- Execute logic reads source, renders, backs up, writes: verified at lines 1327-1373.
- `display()` returns `"[!] Render template -> {target} ({module_id})"`: verified at lines 382-389.
- 13+ tests (exceeds requirement of 8+).

### F3-009: File Copy Deployment Mode -- PASS

- `CopyFile` variant with correct fields: verified at lines 271-276.
- Produced when `link=false` and no templates: verified at lines 1014-1023.
- Execute logic copies file with backup: verified at lines 1374-1409.
- `display()` and `summary()` updated: verified.

### F3-010: Package Removal (RemovePackages) -- PASS

- `RemovePackages` variant exists: verified at line 278.
- Plan computation: `managed AND installed AND NOT (desired OR desired_aur)`: verified at lines 1062-1085. Correctly checks both `desired_pkgs` and `desired_aur` to prevent false positives on AUR packages.
- Safety invariant: unmanaged packages never included (gated by `managed_pkgs` set): verified.
- Execute calls `package_manager.remove(packages, false)`: verified at lines 1410-1413.
- 12+ tests.

### F3-011: Service Disable (DisableService) -- PASS

- `DisableService` variant exists: verified at line 279.
- Plan: `managed AND enabled AND NOT desired`: verified at lines 1087-1108.
- Execute calls `service_manager.disable_service(name)`: verified at lines 1414-1417.
- 4+ tests.

### F3-012: Symlink/Module Removal -- PASS

- `RemoveSymlink` and `DeactivateModule` variants exist: verified at lines 280-284.
- Plan: managed dotfiles not in desired targets produce RemoveSymlink: verified at lines 1110-1131.
- Plan: active modules not in desired produce DeactivateModule: verified at lines 1133-1142.
- Execute: RemoveSymlink backs up and removes: verified at lines 1418-1430.
- Execute: DeactivateModule calls `disable_module()`: verified at lines 1431-1434.
- 5+ tests.

### F3-013/F3-015: RiskLevel -- PASS

- `RiskLevel` enum with ReadOnly/Additive/Destructive/Critical, PartialOrd+Ord: verified at lines 290-303.
- `risk_level()` correctly classifies all 11 variants: verified at lines 316-338. CopyFile(backup=false) is Additive, CopyFile(backup=true) is Destructive, RemovePackages is Critical.
- `max_risk()` returns highest or ReadOnly for empty: verified at lines 428-434.
- `risk_summary()` implemented: verified at lines 442-448.
- Plan output shows risk badges ([+], [!], [!!]) via `display()`: verified at lines 352-412.
- 10+ tests.

### F3-014: Prune Flags -- PASS

- `PrunePolicy` struct with `packages`, `services`, `dotfiles` booleans: verified at lines 603-637.
- `with_prune_policy()` builder on `DefaultApplyService`: verified at lines 664-668.
- `should_execute_prune()` gates removal actions: verified at lines 1148-1164.
- CLI flags `--prune`, `--prune-packages`, `--prune-services`, `--prune-dotfiles` on Apply: verified at `cli.rs` lines 173-188.
- Same flags on Plan: verified at `cli.rs` lines 245-259.
- `main.rs` dispatches all flags: verified at lines 67-113.
- `apply.rs` constructs PrunePolicy from flags: verified at lines 27-35.
- 6 integration tests for prune flags.

### F3-016: Confirmation UX -- PASS

- `display_plan_with_risk()` shows risk badge header: verified at `apply.rs` lines 137-176.
- `confirm_apply()` scales with risk: ReadOnly=auto, Additive=y/N (--yes skips), Destructive=y/N (--yes skips), Critical=typed "yes" (--yes does NOT bypass): verified at lines 224-261.
- `display_dry_run_confirmation()` shows hypothetical prompt: verified at lines 179-221.
- Prune hint displayed when prunable actions exist without --prune: verified at lines 74-84.

---

## Evidence

- **Build**: `cargo build --workspace` -- PASS
- **Tests**: `cargo test --workspace` -- PASS (1057 iron-core + 117 iron-cli lib + 83 iron-cli integration + 100 iron-tui + 453 iron-fs + others = all passing, 0 failures)
- **Clippy**: `cargo clippy --workspace -- -D warnings` -- PASS (0 warnings)
- **Format**: `cargo fmt --all -- --check` -- minor formatting diffs in new test code (SHOULD-3)
- **Scope adherence**: IN_SCOPE (AC-021-11 and AC-013-6 are noted gaps but non-blocking)

---

## Sign-off

**APPROVED WITH NOTES.**

Zero MUST findings. The implementation is correct, well-tested (42+ new tests), and architecturally sound. The 5 SHOULD findings are all non-blocking:

1. `iron status` managed display (AC-021-11) -- deferred, data exists, display missing
2. JSON risk_level field (AC-013-6) -- gap in JSON API, not blocking CLI usage
3. Formatting -- trivial `cargo fmt` fix
4. Template detection broadness -- acceptable for config file use case
5. Temporal contamination in comments -- cosmetic

The sprint successfully transforms Iron from additive-only to full declarative convergence with appropriate safety guards (managed tracking prevents removal of untracked resources, prune flags are opt-in, critical changes require typed confirmation).
