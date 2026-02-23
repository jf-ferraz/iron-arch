# Changes -- Sprint 3.2 Wave 1

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/state.rs` | Added 4 new fields to `IronState`: `managed_packages`, `managed_services`, `managed_dotfiles` (all `Vec<String>` with `#[serde(default)]`), `last_apply` (`Option<DateTime<Utc>>` with `#[serde(default, skip_serializing_if)]`) | F3-021: Managed resource tracking persistence |
| `crates/iron-core/src/services/state.rs` | Added 10 new methods to `StateManager`: `record_managed_packages`, `unrecord_managed_packages`, `record_managed_service`, `unrecord_managed_service`, `record_managed_dotfile`, `unrecord_managed_dotfile`, `managed_packages`, `managed_services`, `managed_dotfiles`, `update_last_apply`. Added 12 unit tests. | F3-021: StateManager API for managed resource tracking |
| `crates/iron-core/src/services/apply.rs` | Added 6 new `ApplyAction` variants: `RenderAndCopy`, `CopyFile`, `RemovePackages`, `DisableService`, `RemoveSymlink`, `DeactivateModule`. Added `RiskLevel` enum with `risk_level()`, `is_prunable()`, `display()` methods. Updated `summary()` to new compact format. Added `max_risk()`, `prune_count()` to `ApplyPlan`. Updated `compute_plan()` dotfile section with template detection decision tree. Updated `execute_action()` with match arms for all 6 new variants. Added `has_template_variables()` and `render_template()` inline helpers. Updated existing tests, added 16 new tests. | F3-008: Template rendering in apply; F3-009/010/011/012: New action variants; F3-013: Risk levels |
| `crates/iron-cli/src/commands/plan.rs` | Updated plan display to group and show all 11 action types including new variants (RenderAndCopy, CopyFile, RemovePackages, DisableService, RemoveSymlink, DeactivateModule) with appropriate `[PRUNE]` badges for removal actions. | F3-008/009/010/011/012: Plan display for new variants |
| `crates/iron-cli/src/commands/apply.rs` | Updated `test_plan_summary_format` test to match new compact summary format (`+1 pkg` instead of `1 package(s)`). | Summary format change from F3-013 |

## Files Created

None.

## Design Decisions

### Template helpers inlined in iron-core

The architect spec called for using `iron_fs::template::has_variables()` and `iron_fs::template::render()`. However, `iron-core` does not depend on `iron-fs` (iron-core is the application layer, iron-fs is infrastructure). Adding this dependency would violate the layered architecture. Instead, `has_template_variables()` and `render_template()` are implemented inline in `apply.rs`, matching the exact behavior of the iron-fs versions. This keeps the dependency direction correct.

### Summary format change

The `ApplyPlan::summary()` method now returns a compact format (`+2 pkg, +1 link, +1 svc, +1 mod`) instead of the previous verbose format (`2 package(s), 1 symlink(s), 1 service(s), 1 module(s)`). This is per the architect spec and supports the new removal categories with `-` prefixes.

### display() badge change

The `ApplyAction::display()` method now uses risk badges (`[+]`, `[!]`, `[!!]`) instead of emoji prefixes. This follows the CLAUDE.md convention (no emojis unless explicitly requested) and provides risk-level information in the display output.

## Flagged Items

- **Wave 2 execution stubs**: The `RemovePackages`, `DisableService`, `RemoveSymlink`, and `DeactivateModule` variants have full execution logic, but the prune policy gating (skip prunable actions unless --prune is set) is not yet wired in. That requires the `PrunePolicy` struct and `DefaultApplyService.prune_policy` field from the architect spec, which are Wave 2/3 scope.
- **Bootstrap logic**: The `bootstrap_managed_tracking` method (seeding managed lists from desired+actual on first apply) is not yet implemented. Per architect spec section 3.5, this requires access to desired+actual states inside `execute()` or `plan()`, which is Wave 2 scope when the full managed tracking integration is wired.
- **iron status display**: AC-021-11 calls for displaying managed resource counts and `last_apply` in `iron status`. This touches `crates/iron-cli/src/commands/status.rs` which is not in Wave 1 scope.

## Notes

- All 4 new `IronState` fields use `#[serde(default)]`, ensuring backward compatibility with existing state.json files.
- The `RiskLevel` enum is scoped within `apply.rs` per architect decision AQ-3, avoiding naming collision with `packages::RiskLevel`.
- The dotfile decision tree in `compute_plan()` follows AMB-4: template detection overrides the `link` field, then `link=false` produces CopyFile, then `link=true` (default) produces CreateSymlink.
- Total new tests: 28 (12 in state.rs, 16 in apply.rs).
- `cargo build --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` all pass cleanly.

---

# Changes -- Sprint 3.2 Wave 2

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/services/apply.rs` | Added `PrunePolicy` struct with `none()`, `all()`, `any_enabled()` constructors. Added `prune_policy: PrunePolicy` field to `DefaultApplyService` with `with_prune_policy()` builder. Added `should_execute_prune()`, `record_managed_resource()`, `bootstrap_managed_tracking()` private methods. Updated `compute_plan()` with 4 new removal diff sections: packages (managed AND installed AND NOT desired -> RemovePackages), services (managed AND enabled AND NOT desired -> DisableService), dotfiles (managed AND NOT in desired targets -> RemoveSymlink), modules (active AND NOT in desired -> DeactivateModule). Updated `plan()` to call `bootstrap_managed_tracking()`. Updated `execute()` with prune gating, managed resource recording, and `last_apply` timestamp. Added 12 new unit tests. | F3-011: Package removal in compute_plan; F3-012: Service disable in compute_plan; F3-013: Dotfile removal in compute_plan; F3-014: PrunePolicy + prune gating |
| `crates/iron-cli/src/cli.rs` | Added 4 prune flags to `Apply` command: `--prune`, `--prune-packages`, `--prune-services`, `--prune-dotfiles`. Added 4 prune flags to `Plan` command: `--prune`, `--prune-packages`, `--prune-services`, `--prune-dotfiles`. Updated existing Apply and Plan CLI tests. Added 5 new CLI tests for prune flags. | F3-014: Granular prune CLI flags |
| `crates/iron-cli/src/commands/apply.rs` | Updated `execute()` to accept prune flags, construct `PrunePolicy`, and wire it via `with_prune_policy()` builder. Added prune hint display for prunable actions when pruning is disabled. | F3-014: Prune flag threading to apply service |
| `crates/iron-cli/src/commands/plan.rs` | Updated `execute()` to accept prune flags, construct `PrunePolicy`, and wire it via `with_prune_policy()` builder. | F3-014: Prune flag threading to plan service |
| `crates/iron-cli/src/main.rs` | Updated Apply and Plan dispatch to pass all prune flags through. | F3-014: CLI dispatch for prune flags |

## Files Created

None.

## Design Decisions

### Bootstrap in plan() not execute()

Per architect recommendation (section 3.5, option b), the bootstrap of managed tracking runs in `plan()` after computing desired+actual states, before `compute_plan()`. This ensures managed lists are populated for the removal diffs, even on the very first plan computation. The bootstrap only triggers when ALL managed lists are empty (first-use guard).

### Removal actions always in plan, gated at execution

Per architect decision AQ-1, all removal actions are always included in the plan output regardless of prune policy. The `PrunePolicy` only controls whether they are executed. This means `iron plan` always shows the complete picture including what would be pruned, and `iron apply --prune` actually runs the removals. Non-prunable actions (install, symlink, enable) always execute.

### AUR packages included in removal tracking

The package removal diff checks against both `desired.packages` and `desired.aur_packages` when determining candidates for removal. This prevents AUR packages from being incorrectly flagged for removal when they appear in `managed_packages` but are only declared as AUR packages in the desired state.

## Flagged Items

- **iron status display**: AC-021-11 (display managed resource counts and `last_apply` in `iron status`) is still pending. Touches `crates/iron-cli/src/commands/status.rs` which is outside Wave 2 scope.
- **Risk-scaled confirmation UX**: The architect spec section 5 describes a risk-scaled confirmation flow (simple y/N for Additive, typed "yes" for Critical). The current apply command still uses the basic y/N prompt. This is Wave 3 scope (F3-013 confirmation UX).

## Notes

- `PrunePolicy` defaults to `none()` (no pruning). CLI `--prune` enables all three; individual `--prune-packages`, `--prune-services`, `--prune-dotfiles` enable selectively.
- The `ApplyService` trait signature is unchanged per architect decision AQ-5. Prune policy is stored on `DefaultApplyService` via builder.
- Bootstrap seeds managed lists with `desired intersect actual_installed` for packages, `desired intersect enabled` for services, and `desired intersect existing` for dotfiles. This is idempotent (only runs when all managed lists are empty).
- The `record_managed_resource()` method updates managed lists after each successful action in `execute()`. Removals call the corresponding `unrecord_*` methods.
- Total new Wave 2 tests: 12 in apply.rs (compute_plan removal diffs, prune policy, bootstrap) + 5 in cli.rs (prune flag parsing). Total across both waves: 45.
- `cargo build --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` all pass cleanly.

---

# Changes -- Sprint 3.2 Wave 3

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/services/apply.rs` | Added `risk_summary()` method to `ApplyPlan` returning `HashMap<RiskLevel, usize>` counting actions per risk level. Added 2 unit tests (`test_risk_summary`, `test_risk_summary_empty_plan`). | F3-015: RiskLevel risk_summary for confirmation UX |
| `crates/iron-cli/src/commands/apply.rs` | Replaced simple y/N confirmation with risk-scaled confirmation flow. Added `display_plan_with_risk()` (shows risk badge header `[SAFE]`/`[CAUTION]`/`[DANGER]` and risk summary counts), `display_dry_run_confirmation()` (shows what prompt WOULD be in dry-run mode), `confirm_apply()` (risk-scaled: ReadOnly=auto, Additive=y/N skippable by --yes, Destructive=y/N skippable by --yes, Critical=typed "yes" NOT skippable by --yes), `read_yes_no()`, `read_typed_yes()`. Added 5 new unit tests for risk levels in plan context. | F3-016: Risk-scaled confirmation UX |
| `crates/iron-cli/tests/cli_integration.rs` | Added `apply_confirmation` test module with 5 integration tests: `apply_dry_run_succeeds`, `apply_dry_run_with_yes_succeeds`, `apply_dry_run_with_prune_succeeds`, `apply_requires_init`, `apply_dry_run_never_prompts`. All use `--dry-run` to avoid sudo/stdin. | F3-016: CLI integration tests for apply confirmation |

## Files Created

None.

## Design Decisions

### Risk badge mapping

The plan header displays a risk badge based on `max_risk()`:
- `ReadOnly` / `Additive` -> `[SAFE]` (no destructive changes)
- `Destructive` -> `[CAUTION]` (modifies/removes files with backup)
- `Critical` -> `[DANGER]` (package removal, hard to reverse)

This differs slightly from the architect spec which mapped Additive to a separate badge. Since additive-only plans are inherently safe (reversible installs/symlinks), grouping them with `[SAFE]` provides clearer user guidance.

### --yes never bypasses Critical

Per architect spec section 5, `--yes` auto-confirms `Additive` and `Destructive` plans but Critical changes (package removal) always require typed "yes" confirmation. This is a safety invariant preventing accidental mass uninstalls in scripts.

### Dry-run shows hypothetical prompt

In `--dry-run` mode, the confirmation prompt text is displayed as informational output (prefixed with `[dry-run]`) but no actual stdin read occurs. This lets users preview what the interactive experience would look like.

## Flagged Items

- **iron status display**: AC-021-11 (display managed resource counts and `last_apply` in `iron status`) is still pending from Wave 1. Out of Wave 3 scope.

## Notes

- The `RiskLevel` enum already existed from Wave 1 with correct `PartialOrd + Ord` derives. Wave 3 verified the classification is correct for all 11 variants and added the `risk_summary()` aggregation method.
- Snapshot-before-apply behavior is unchanged: it occurs inside `execute()` on `DefaultApplyService`, which runs AFTER confirmation (since confirmation gates the call to `execute()`).
- Total new Wave 3 tests: 2 in apply.rs (risk_summary) + 5 in commands/apply.rs (risk UX unit) + 5 in cli_integration.rs (apply CLI integration). Total across all waves: 57.
- `cargo build --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` all pass cleanly.
