# Validation Report -- Sprint 3.2 (Full Declarative Convergence)

**Date:** 2026-02-23
**Tester:** Claude Opus 4.6
**Sprint:** 3.2

---

## 1. Baseline

All pre-existing tests passed before any changes:

| Crate | Tests | Ignored | Status |
|-------|-------|---------|--------|
| iron-core (lib) | 1025 | 4 | PASS |
| iron-core (scan_integration) | 7 | 0 | PASS |
| iron-core (toml_parsing) | 24 | 0 | PASS |
| iron-cli (lib) | 117 | 0 | PASS |
| iron-cli (integration) | 77 | 0 | PASS |
| iron-tui | 100 | 0 | PASS |
| iron-fs | 453 | 0 | PASS |
| iron-pacman | 95 | 0 | PASS |
| iron-systemd | 69 | 0 | PASS |
| iron-git | 31 | 0 | PASS |

Build: `cargo build --workspace` -- PASS
Clippy: `cargo clippy --workspace -- -D warnings` -- PASS
Format: `cargo fmt --all -- --check` -- pre-existing formatting issues in developer code (not introduced by tester)

---

## 2. Test Summary

### Added

| Test | Type | File | Verifies |
|------|------|------|----------|
| `test_has_template_variables_basic` | unit | apply.rs | F3-008: template detection for standard {{var}} |
| `test_has_template_variables_false_for_no_templates` | unit | apply.rs | F3-008: non-template files return false |
| `test_has_template_variables_literal_double_brace` | unit | apply.rs | F3-008: broad detection of {{ even without proper closing |
| `test_render_template_basic_substitution` | unit | apply.rs | F3-008: AC-008-4 variable rendering |
| `test_render_template_whitespace_trimming` | unit | apply.rs | F3-008: {{ name }} trims whitespace |
| `test_render_template_unknown_variable_preserved` | unit | apply.rs | F3-008: AC-008-7 unknown vars left unchanged |
| `test_render_template_empty_content` | unit | apply.rs | F3-008: empty file renders to empty |
| `test_render_template_no_variables_passthrough` | unit | apply.rs | F3-008: plain text passes through unmodified |
| `test_render_template_multiple_same_variable` | unit | apply.rs | F3-008: same variable used twice |
| `test_render_template_unclosed_brace` | unit | apply.rs | F3-008: unclosed {{ is preserved |
| `test_prune_policy_services_only` | unit | apply.rs | F3-014: selective policy with only services |
| `test_prune_policy_dotfiles_only` | unit | apply.rs | F3-014: selective policy with only dotfiles |
| `test_summary_empty_plan` | unit | apply.rs | F3-015: "No changes" for empty plan |
| `test_summary_only_removals` | unit | apply.rs | F3-015: summary with only removal actions |
| `test_summary_copy_and_render` | unit | apply.rs | F3-015: copy+render counted together |
| `test_max_risk_destructive_without_critical` | unit | apply.rs | F3-015: Destructive max when no Critical present |
| `test_risk_summary_all_same_level` | unit | apply.rs | F3-015: risk_summary with homogeneous actions |
| `test_risk_summary_mixed_all_four_levels_impossible` | unit | apply.rs | F3-015: ReadOnly never appears as action risk |
| `test_compute_plan_empty_managed_lists_no_removals` | unit | apply.rs | F3-010/011: no removals when managed lists empty |
| `test_compute_plan_managed_but_already_uninstalled` | unit | apply.rs | F3-010: managed pkg already removed from system |
| `test_compute_plan_aur_packages_not_removed` | unit | apply.rs | F3-010: AUR pkgs in desired.aur_packages excluded from removal |
| `test_compute_plan_service_not_disabled_when_already_stopped` | unit | apply.rs | F3-012: managed svc already disabled on system |
| `test_bootstrap_includes_aur_packages` | unit | apply.rs | F3-021: AC-021-9 bootstrap seeds AUR packages |
| `test_bootstrap_skips_when_services_exist` | unit | apply.rs | F3-021: bootstrap guard checks all three lists |
| `test_bootstrap_seeds_dotfiles` | unit | apply.rs | F3-021: bootstrap seeds existing dotfiles |
| `test_should_execute_prune_deactivate_uses_dotfiles_policy` | unit | apply.rs | F3-014: DeactivateModule gated by dotfiles policy |
| `test_is_prunable_comprehensive_non_prunable` | unit | apply.rs | F3-014: all non-prunable variants verified |
| `test_compute_plan_multiple_packages_in_single_removal` | unit | apply.rs | F3-010: multiple pkgs in one RemovePackages action |
| `test_unrecord_nonexistent_package_is_noop` | unit | state.rs | F3-021: unrecord missing package does not error |
| `test_unrecord_nonexistent_service_is_noop` | unit | state.rs | F3-021: unrecord missing service does not error |
| `test_unrecord_nonexistent_dotfile_is_noop` | unit | state.rs | F3-021: unrecord missing dotfile does not error |
| `test_record_empty_packages_is_noop` | unit | state.rs | F3-021: recording empty list is harmless |
| `test_unrecord_empty_packages_is_noop` | unit | state.rs | F3-021: unrecording empty list preserves existing |
| `test_managed_dotfile_deduplicates` | unit | state.rs | F3-021: dotfile deduplication |
| `test_last_apply_timestamp_updates` | unit | state.rs | F3-021: AC-021-10 timestamp monotonically increases |
| `test_managed_resources_persist_after_multiple_operations` | unit | state.rs | F3-021: persistence after record+unrecord cycle |
| `apply_dry_run_prune_packages_flag` | integration | cli_integration.rs | F3-014: --prune-packages CLI flag accepted |
| `apply_dry_run_prune_services_flag` | integration | cli_integration.rs | F3-014: --prune-services CLI flag accepted |
| `apply_dry_run_prune_dotfiles_flag` | integration | cli_integration.rs | F3-014: --prune-dotfiles CLI flag accepted |
| `apply_dry_run_all_prune_flags_combined` | integration | cli_integration.rs | F3-014: all granular prune flags together |
| `plan_dry_run_with_prune_succeeds` | integration | cli_integration.rs | F3-014: plan --prune CLI flag accepted |
| `plan_dry_run_with_prune_packages_succeeds` | integration | cli_integration.rs | F3-014: plan --prune-packages CLI flag accepted |

### Post-change Counts

| Crate | Before | After | Delta |
|-------|--------|-------|-------|
| iron-core (lib) | 1025 | 1061 | +36 |
| iron-cli (integration) | 77 | 83 | +6 |
| **Total** | **1102** | **1144** | **+42** |

---

## 3. Coverage by Task

### F3-021: Managed Resource Tracking
- **AC-021-1 through AC-021-2**: Verified by `test_managed_backward_compat_empty_state` (existing) + new persistence tests
- **AC-021-3 through AC-021-8**: Verified by existing `test_record_managed_*` and `test_unrecord_managed_*` tests in state.rs, plus new edge case tests (`_nonexistent_is_noop`, `_empty_is_noop`, `_deduplicates`)
- **AC-021-9**: Bootstrap verified by existing `test_bootstrap_managed_tracking_seeds` + new `test_bootstrap_includes_aur_packages`, `test_bootstrap_skips_when_services_exist`, `test_bootstrap_seeds_dotfiles`
- **AC-021-10**: Verified by existing `test_update_last_apply` + new `test_last_apply_timestamp_updates`
- **AC-021-11**: NOT TESTED -- `iron status` display of managed counts is flagged as out-of-scope in all three waves
- **AC-021-12**: 12 existing + 8 new = 20 unit tests (exceeds requirement of 8+)

### F3-008: Template Variable Rendering
- **AC-008-1 through AC-008-3**: Verified by existing compute_plan tests (template detection decision tree in `compute_plan`)
- **AC-008-4 through AC-008-6**: Template rendering verified by new `test_render_template_*` tests (7 tests)
- **AC-008-7**: Unreadable source fallback not explicitly tested at unit level (requires filesystem mocking); covered architecturally by the `unwrap_or(false)` in `has_template_variables` detection
- **AC-008-8**: Display verified by existing `test_display_new_variants`
- **AC-008-9**: 3 existing + 10 new = 13 tests (exceeds requirement of 8+)

### F3-009: File Copy Deployment Mode
- **AC-009-1 through AC-009-4**: Verified by existing `test_display_new_variants`, `test_summary_with_new_variants` + new `test_summary_copy_and_render`
- **AC-009-5**: 4+ tests covered across existing and new

### F3-010: Package Removal
- **AC-010-1**: Verified by existing `test_compute_plan_removal_packages`
- **AC-010-2**: Verified by same test
- **AC-010-3**: Verified by existing `test_compute_plan_no_removal_for_unmanaged` + new `test_compute_plan_empty_managed_lists_no_removals`
- **AC-010-4 through AC-010-5**: Verified by existing prune policy tests
- **AC-010-6**: Verified at integration level
- **AC-010-7**: Verified by managed tracking recording tests
- **AC-010-8**: Verified by display/summary tests
- **AC-010-9**: 8 existing + 4 new = 12 tests (exceeds requirement of 8+)

### F3-011: Service Disable
- **AC-011-1 through AC-011-5**: Verified by existing `test_compute_plan_disable_service`, `test_compute_plan_no_disable_unmanaged_service` + new `test_compute_plan_service_not_disabled_when_already_stopped`
- **AC-011-6**: 4+ tests covered

### F3-012: Symlink/Module Removal
- **AC-012-1 through AC-012-8**: Verified by existing `test_compute_plan_remove_dotfile`, `test_compute_plan_no_remove_current_dotfile`, `test_compute_plan_deactivate_module`
- **AC-012-9**: 5+ tests covered

### F3-013/F3-016: Risk Levels + Confirmation UX
- **AC-013-1**: Verified by existing `test_risk_level_ordering`, `test_risk_level_display` + new invariant test
- **AC-013-2**: Verified by existing `test_risk_level_classification` (all 11 variants)
- **AC-013-3**: Verified by existing `test_max_risk_*` tests + new `test_max_risk_destructive_without_critical`
- **AC-013-4**: Verified at CLI integration level
- **AC-013-5**: Verified by existing CLI integration tests (`apply_dry_run_succeeds`, `apply_dry_run_with_yes_succeeds`)
- **AC-013-7**: 6 existing + 4 new = 10 tests (exceeds requirement of 6+)

### F3-014: PrunePolicy
- Verified by existing `test_prune_policy_*` (4 tests) + new `test_prune_policy_services_only`, `test_prune_policy_dotfiles_only`
- Prune gating verified by existing `test_should_execute_prune_policy_*` (3 tests) + new `test_should_execute_prune_deactivate_uses_dotfiles_policy`
- CLI prune flags verified by 6 new CLI integration tests

---

## 4. Observations

### Items Not Tested (Out of Scope)
1. **AC-021-11** (`iron status` display of managed resource counts) -- flagged as pending in all three wave change documents
2. **AC-008-7** (unreadable source file fallback to CreateSymlink) -- would require mocking `std::fs::read_to_string` at the apply service level; the code path is covered by the `.ok()` + `unwrap_or(false)` pattern but not exercised in tests
3. **AC-013-6** (`--json` plan output includes `risk_level` field per action) -- plan JSON output was not modified to include per-action risk_level; this appears to be a gap in the implementation, not just in tests

### Formatting
Pre-existing `cargo fmt` check failures exist in `crates/iron-cli/src/cli.rs` from developer changes. These are not introduced by tester changes.

---

## 5. Final Verification

```
cargo build --workspace          PASS
cargo test --workspace           PASS (1144+ tests, 0 failures)
cargo clippy --workspace         PASS (0 warnings-as-errors)
```

All 42 new tests pass. All pre-existing tests continue to pass.
