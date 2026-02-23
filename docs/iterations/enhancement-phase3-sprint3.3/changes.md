# Changes -- Sprint 3.3 Waves 1-2

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/module.rs` | Added `HookBehavior` enum (Always/Once/Ask/Skip), `HookType` enum (PreInstall/PostInstall/PreUninstall/StatusCheck) with Display impl, added `hook_behavior`, `dotfiles_sync`, `dotfiles_sync_target` fields to Module struct, added 5 unit tests | F3-014: Hook lifecycle enums and Module fields |
| `crates/iron-core/src/state.rs` | Added `hooks_executed: HashMap<String, Vec<String>>` to IronState, added `duration_secs: Option<f64>` and `action_count: Option<usize>` to OperationRecord, updated `record_operation()` and 4 test constructors | F3-015: Hook execution tracking in state |
| `crates/iron-core/src/services/state.rs` | Added `record_hook_executed()`, `is_hook_executed()`, `clear_hooks_for_module()` methods to StateManager, removed temporal sprint comment | F3-015: Hook tracking methods |
| `crates/iron-core/src/services/apply.rs` | Added `RunHook` variant to `ApplyAction` enum, updated all 7 match arms (risk_level, is_prunable, display, summary, record_managed_resource, should_execute_prune, execute_action), restructured `compute_plan()` into 5 phase vectors (pre_hooks, install_actions, post_hooks, removal_pre_hooks, removal_actions), added `run_hook()` helper with timeout/env vars, added `HookOutput` struct, added `DEFAULT_HOOK_TIMEOUT` constant, added `force_hooks`, `interactive`, `hook_timeout` fields and builder methods to `DefaultApplyService`, removed 6 temporal sprint comments, added 8 new unit tests | F3-014: Hook execution in apply, F3-015: Once tracking, SHOULD-5: Temporal comments |
| `crates/iron-core/src/services/mod.rs` | Added `DEFAULT_HOOK_TIMEOUT` and `HookOutput` to re-exports | F3-014: Public API |
| `crates/iron-core/src/test_helpers.rs` | Added `pre_uninstall`, `hook_behavior`, `dotfiles_sync`, `dotfiles_sync_target` fields and builder methods (`with_hook_behavior`, `with_pre_install`, `with_post_install`, `with_pre_uninstall`, `with_dotfiles_sync`, `with_dotfiles_sync_target`) to TestModule | F3-014: Test helper compatibility |
| `crates/iron-cli/src/cli.rs` | Added `--force-hooks` flag to Apply command, updated test pattern match | F3-015: CLI flag for hook re-execution |
| `crates/iron-cli/src/commands/apply.rs` | Added `force_hooks` parameter, chained `.with_force_hooks()` and `.with_interactive()` on service builder | F3-015: Wire force_hooks to service |
| `crates/iron-cli/src/main.rs` | Added `force_hooks` to Apply command dispatch | F3-015: Wire CLI to command |
| `crates/iron-cli/src/commands/status.rs` | Added `managed_packages`, `managed_services`, `managed_dotfiles`, `last_apply` to PackagesStatus struct and display, added "Managed Resources" section and "Last Apply" to human-readable output | SHOULD-1: Managed counts in status |
| `crates/iron-cli/tests/cli_integration.rs` | Removed 5 temporal sprint comments from test section headers | SHOULD-5: Temporal comments |
| `crates/iron-core/src/services/module.rs` | Added 3 new Module fields to 4 test constructors | F3-014: Test compatibility |
| `crates/iron-core/src/services/profile.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |
| `crates/iron-core/src/services/scan.rs` | Added 3 new Module fields to 3 test constructors | F3-014: Test compatibility |
| `crates/iron-core/src/validation.rs` | Added 3 new Module fields to 9 test constructors | F3-014: Test compatibility |
| `crates/iron-core/tests/toml_parsing.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |
| `crates/iron-tui/src/lib.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |
| `crates/iron-tui/src/ui/security.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |
| `crates/iron-tui/src/ui/tests.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |
| `crates/iron-tui/src/app/actions.rs` | Added 3 new Module fields to 3 test constructors | F3-014: Test compatibility |
| `crates/iron-tui/src/app/handlers.rs` | Added 3 new Module fields to test constructor | F3-014: Test compatibility |

## Files Created

| File | Purpose |
|------|---------|
| `docs/iterations/enhancement-phase3-sprint3.3/changes.md` | This change log |

## Flagged Items

- The existing `state.json` files that use `"{}"` as empty state content will fail to parse because IronState has required fields without `#[serde(default)]` (e.g., `active_bundles`, `active_profiles`). Tests that need StateManager should NOT write `state.json` beforehand -- let StateManager create a default. This is an existing pattern issue, not introduced by this sprint.

## Notes

- **Hook execution uses `std::process::Command` directly** (per architect decision AQ-014-1), not the `CommandExecutor` circuit breaker. This is intentional -- hooks are user-defined code, not system infrastructure commands.
- **Hook working directory** is the module source directory (`iron_root/modules/<module-id>/`), with `IRON_ROOT` and `IRON_MODULE` environment variables injected.
- **compute_plan() restructured** from a single `actions` vector to 5 phase vectors concatenated in order: pre_hooks, install_actions, post_hooks, removal_pre_hooks, removal_actions. Existing behavior preserved -- the restructure only affects ordering when hooks are present.
- **Ask hooks in non-interactive mode** are silently skipped (not failed). The `interactive` flag is derived from `!yes` in the CLI.
- **SHOULD-3 (cargo fmt)**: Ran `cargo fmt --all` as final step. No manual formatting changes needed.
- **All 2186 workspace tests pass** (Waves 1-2), clippy clean with `-D warnings`, formatting verified.

---

# Changes -- Sprint 3.3 Wave 3

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `crates/iron-core/src/services/apply.rs` | Added `discover_dotfiles()` and `discover_dotfiles_recursive()` functions for recursive file discovery. Modified `resolve_desired_state()` Step 6 to auto-discover dotfiles when `module.dotfiles_sync == true`, merge with explicit dotfiles (explicit wins on target collision), warn via `tracing::warn!` for hyphenated module IDs. Added 6 unit tests for dotfiles_sync. | F3-018: dotfiles_sync auto-mirror |
| `crates/iron-core/src/services/mod.rs` | Added `pub mod history;` and re-exports for `DefaultHistoryService`, `HistoryEntry`, `HistoryService` | F3-016: History service registration |
| `crates/iron-cli/src/cli.rs` | Added `Commands::History` variant with `HistoryAction` subcommand (List/Show/Last), `--limit` flag (default 20). Added `HistoryAction` enum. Added 5 CLI parsing tests. | F3-016: History CLI definition |
| `crates/iron-cli/src/commands/mod.rs` | Added `pub mod history;` | F3-016: History command module |
| `crates/iron-cli/src/main.rs` | Added dispatch for `Commands::History` to `commands::history::execute()` | F3-016: History dispatch |

## Files Created

| File | Purpose |
|------|---------|
| `crates/iron-core/src/services/history.rs` | `HistoryService` trait + `DefaultHistoryService` impl. Reads `IronState.last_operations`, provides `list(limit)`, `show(index)`, `last()` with 1-based indexing (most recent = 1). Includes `HistoryEntry` display model. 5 unit tests. |
| `crates/iron-cli/src/commands/history.rs` | `iron history [list]` (table), `iron history show <id>` (detail), `iron history last` (detail). Supports `--json` envelope output. Relative timestamp display (e.g., "2h ago", "3d ago"). |

## Flagged Items

(None for Wave 3)

## Notes

- **dotfiles_sync discovery** happens in `resolve_desired_state()` at desired-state resolution time (per architect decision AQ-018-1), so all consumers (plan, diff, status, TUI) see auto-discovered dotfiles consistently.
- **Merge semantics**: When `dotfiles_sync = true`, auto-discovered files from `modules/<id>/dotfiles/` are merged with explicit `[[dotfiles]]` declarations. If both target the same path, the explicit entry wins. Auto-discovered entries default to `link: true` (symlink).
- **Hyphenated module ID warning**: Uses `tracing::warn!` (structured logging), not `log::warn!` (log crate not in iron-core dependencies).
- **History indexing**: 1-based, most recent first. `iron history show 1` = most recent operation. The underlying data comes from `IronState.last_operations` (capped at 100 entries).
- **Directory traversal** in `discover_dotfiles_recursive` uses sorted entries for deterministic output across runs.
- **All 2212 workspace tests pass**, clippy clean with `-D warnings`, formatting verified.
