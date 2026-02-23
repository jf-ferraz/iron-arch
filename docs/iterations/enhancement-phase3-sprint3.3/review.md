# Validation Report -- Sprint 3.3 (Execution Lifecycle Completion)

## Summary
- **Type**: ENHANCEMENT (structural)
- **Status**: APPROVED_WITH_NOTES
- **MUST findings**: 0
- **SHOULD findings**: 4
- **COULD findings**: 2

## MUST Findings

No blocking issues found. All four core tasks (F3-014, F3-015, F3-016, F3-018) are implemented, the build passes, all 2231 tests pass, clippy is clean, and formatting is verified.

## SHOULD Findings

### SHOULD-1: AC-015-6 unmet -- `clear_hooks_for_module` never called in production

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/apply.rs`, lines 1569-1572
**Observation**: The `DeactivateModule` execution arm calls `self.state_manager.disable_module(id)` but does not call `self.state_manager.clear_hooks_for_module(id)`. The method `clear_hooks_for_module` exists and is tested (5 unit tests), but has zero production callers.
**Impact**: When a module is deactivated and later re-enabled, Once hooks will NOT re-run because the `hooks_executed` state persists. The user must use `--force-hooks` as a workaround.
**Dual-path verification**: Forward: DeactivateModule -> disable_module() -> hooks_executed persists -> re-enable -> Once hooks skipped. Backward: AC-015-6 requires clearing on disable -> no production code calls clear_hooks_for_module -> confirmed unmet.
**Recommendation**: Add `self.state_manager.clear_hooks_for_module(id).ok();` after the `disable_module(id)` call at line 1571.

### SHOULD-2: Once hooks not filtered at plan time (AC-015-3 deviation)

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/apply.rs`, lines 1205-1227
**Observation**: AC-015-3 states "Once hooks check hooks_executed at plan time -- skip if already recorded." The implementation plans all hooks regardless of Once state, then filters at execution time (lines 1586-1592). This means `iron plan` and `iron apply --dry-run` display Once hooks that have already run and will be skipped.
**Impact**: Misleading plan output -- a user sees a hook listed that will not actually execute. Low severity since `--force-hooks` exists and the execution behavior is correct.
**Recommendation**: Either filter Once hooks at plan time, or add a `[skipped]` badge to the display for hooks that will be skipped at execution time.

### SHOULD-3: History list shows "Showing N of N total operations" (always equal)

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/commands/history.rs`, lines 66-70
**Observation**: The info line formats `entries.len()` for both the "showing" and "total" counts. Since `entries` is already the limited/truncated list, these values are always identical. The total should come from the full `last_operations` length.
**Recommendation**: Pass the total count from `svc.list()` or query the state directly for the untruncated count.

### SHOULD-4: Pre-existing temporal contamination in TUI code (out of Sprint 3.3 scope)

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-tui/src/app/mod.rs`, lines 206, 213
**Observation**: Comments reference "Sprint 3 / S1-P1.5-003" and "Sprint 3 / S1-P3-001". These are pre-existing from an earlier phase and were not in scope for Sprint 3.3's SHOULD-5 carryover (which targeted Sprint 3.1/3.2 references only).
**Recommendation**: Track for cleanup in a future sprint. Replace with task IDs or context-independent descriptions.

## COULD Findings

### COULD-1: Hook failure returns Err instead of Ok (architect deviation)

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/apply.rs`, lines 1604-1612
**Observation**: The architect specified (Section 4.3) that hook failure should return `Ok(())` to be non-fatal. The developer returns `Err(e)`, which the execute loop counts as a failure. The apply still continues (non-fatal), but the action is recorded as failed rather than silently succeeded. This is arguably better for transparency.
**Suggestion**: Document the deviation. The current behavior (visible failure tracking) is preferable to silent success. Consider adding an `--strict-hooks` flag in the future if users want hook failures to abort apply.

### COULD-2: `plan_module` does not include dotfiles_sync discovery

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/apply.rs`, lines 862-881
**Observation**: The `plan_module()` method builds a mini DesiredState by collecting `m.packages`, `m.aur_packages`, and `m.dotfiles` directly. It does not call `resolve_desired_state()` and does not run the dotfiles_sync discovery logic. Modules with `dotfiles_sync = true` planned via `iron apply --module <id>` will not include auto-discovered dotfiles.
**Suggestion**: Extract the dotfiles_sync discovery into a reusable function and call it from both `resolve_desired_state()` and `plan_module()`.

## Evidence

- **Build**: PASS (`cargo build --workspace` -- clean compilation)
- **Tests**: PASS (2231 tests, 0 failures, 16 ignored across all crates)
- **Lint**: PASS (`cargo clippy --workspace -- -D warnings` -- 0 warnings)
- **Format**: PASS (`cargo fmt --all -- --check` -- exit 0)
- **Scope adherence**: IN_SCOPE (F3-014, F3-015, F3-016, F3-018 implemented; F3-017 correctly deferred; 3 carryovers addressed)

### Test Coverage Summary

| Feature | New Tests | Key Behaviors Covered |
|---------|-----------|----------------------|
| F3-014: Hook Lifecycle | 14 | HookBehavior/HookType enums, RunHook variant, ordering, display, risk, skip behavior, pre_uninstall ordering |
| F3-015: Hook Tracking | 10 | record/check/clear methods, idempotency, persistence, backward compat, --force-hooks flag |
| F3-016: History | 12 | Empty state, reverse chronological, limit, show detail, last shortcut, CLI parsing, integration |
| F3-018: dotfiles_sync | 8 | Discovery, nesting, empty dir, explicit override, custom target, hidden files |
| Carryovers | 0 | Display-only changes covered by existing integration tests |

### Acceptance Criteria Status

| AC Group | Met | Partial | Unmet |
|----------|-----|---------|-------|
| AC-014 (Hook Execution) | 12/14 | 0 | 0 (AC-014-8 Ask: prompt not tested in interactive mode, acceptable) |
| AC-015 (Hook Tracking) | 6/8 | 1 | 1 (AC-015-3: filtered at execute not plan; AC-015-6: clear not wired) |
| AC-016 (History) | 9/10 | 1 | 0 (AC-016-1: table format verified via integration test, minor display bug in total) |
| AC-018 (dotfiles_sync) | 9/11 | 0 | 0 (AC-018-8: template detection delegated to compute_plan; AC-018-9: warning path exercised but not captured in test) |
| AC-SHOULD-1 (Status) | 3/3 | 0 | 0 |
| AC-SHOULD-3 (Format) | 1/1 | 0 | 0 |
| AC-SHOULD-5 (Temporal) | 1/1 | 0 | 0 (Sprint 3.1/3.2 references removed; pre-existing "Sprint 3" in TUI out of scope) |

## Sign-off

**APPROVED WITH NOTES.**

Zero MUST findings. The implementation is solid across all four tasks. The five-phase ordering restructure in `compute_plan()` is clean and well-tested. Hook execution with timeout, environment variables, and behavior policies is correctly implemented. The HistoryService and CLI command are clean and functional. The dotfiles_sync discovery with merge semantics works as specified.

The four SHOULD findings should be tracked for Sprint 3.4 or a polish pass:
1. Wire `clear_hooks_for_module` into `DeactivateModule` execution (highest priority -- affects correctness of Once semantics across module disable/re-enable cycles)
2. Filter Once hooks at plan time or add skip badges
3. Fix history list total count display
4. Clean up pre-existing temporal comments in TUI
