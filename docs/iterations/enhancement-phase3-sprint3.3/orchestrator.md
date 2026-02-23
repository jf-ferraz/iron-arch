# Orchestrator Handoff -- Sprint 3.3 (Execution Lifecycle Completion)

**Date:** 2026-02-23
**Classification:** ENHANCEMENT (structural -- adds hook lifecycle, history service, config namespace, dotfiles_sync)
**Agent Chain:** analyst -> architect -> developer -> tester -> reviewer

---

## Classification Rationale

This is an ENHANCEMENT with structural changes requiring the architect agent:
- **New domain models**: `HookBehavior` enum, `HookType` enum, `HistoryService`
- **New module fields**: `hook_behavior`, `dotfiles_sync`, `dotfiles_sync_target`
- **New state fields**: `hooks_executed` HashMap
- **New CLI commands**: `iron history` (and stretch: `iron config` namespace)
- **New ApplyAction variant**: `RunHook`
- **Apply pipeline changes**: Hook insertion into execution order, dotfile auto-discovery

---

## Sprint 3.3 Scope (5 Tasks)

| Task | Priority | Effort | Description |
|------|----------|--------|-------------|
| **F3-014** | Critical | L | Hook execution in apply lifecycle -- HookBehavior enum, RunHook action, execution order |
| **F3-015** | Medium | S | Hook execution tracking -- `hooks_executed` state field, Once behavior, `--force-hooks` |
| **F3-016** | Medium | M | `iron history` command -- operation history from audit log |
| **F3-017** | Nice-to-have | S | `iron config` namespace (STRETCH -- can slip to Phase 4) |
| **F3-018** | Medium | M | `dotfiles_sync` auto-mirror -- automatic directory mirroring for modules |

### Task Dependency Ordering

```
F3-014 -> F3-015  (hook execution must exist before tracking)
F3-016             (independent -- reads existing audit log)
F3-018             (independent -- extends compute_plan dotfile logic)
F3-017             (STRETCH, independent -- CLI namespace grouping)
```

**Recommended implementation order:**
1. F3-014 (Hook execution) -- largest task, new ApplyAction variant, apply pipeline changes
2. F3-015 (Hook tracking) -- depends on F3-014, small addition to state
3. F3-016 (iron history) -- independent, new CLI command + service
4. F3-018 (dotfiles_sync) -- independent, extends module + compute_plan
5. F3-017 (iron config) -- STRETCH, implement only if time permits

---

## SHOULD Findings from Prior Sprints to Address

### From Sprint 3.1 Review

| Finding | Description | Action for Sprint 3.3 |
|---------|-------------|----------------------|
| SHOULD-1 | 8 unmigrated `output.json()` calls (host, profile, bundle, sync) | Out of scope -- not related to Sprint 3.3 tasks |
| SHOULD-2 | `json_error_envelope` method unused | Out of scope -- error path migration is separate work |
| SHOULD-3 | Info lines bleed into JSON output | Out of scope -- pre-existing pattern issue |
| SHOULD-4 | Missing "last apply timestamp" in status | **IN SCOPE**: `last_apply` field now exists (F3-021). Wire into `iron status` display as part of Sprint 3.3 polish. |

### From Sprint 3.2 Review

| Finding | Description | Action for Sprint 3.3 |
|---------|-------------|----------------------|
| SHOULD-1 | AC-021-11: `iron status` does not display managed resource counts or `last_apply` | **IN SCOPE**: Wire existing `state.managed_packages.len()`, `managed_services.len()`, `managed_dotfiles.len()`, and `last_apply` into `iron status` output. |
| SHOULD-2 | AC-013-6: JSON plan output missing `risk_level` field per action | Out of scope -- requires serialization strategy change |
| SHOULD-3 | Formatting issues in new test code | **IN SCOPE**: Run `cargo fmt --all` as part of development |
| SHOULD-4 | `has_template_variables` overly broad detection | Out of scope -- acceptable for config files |
| SHOULD-5 | Temporal contamination in comments ("Sprint 3.2" references) | **IN SCOPE**: Remove sprint references from code comments during development |

**Summary of carryover work:**
1. Wire managed resource counts + `last_apply` timestamp into `iron status` display
2. Run `cargo fmt --all` to fix formatting
3. Remove temporal contamination comments from apply.rs

---

## Instructions for Analyst

The analyst should:

1. **Read the technical specifications** for all 5 tasks in `docs/phase3-kanban.md` (Sprint 3.3 section, starting at line 486) and `docs/phase3-technical-guide.md` (Section 4, starting at line 716).

2. **Produce requirements** for each task with testable acceptance criteria. Key areas to specify:
   - F3-014: Hook execution order relative to other actions. Error handling policy (fatal vs non-fatal). How `Ask` behavior works in CLI vs TUI. How hooks interact with `--dry-run`.
   - F3-015: `hooks_executed` HashMap structure. When entries are cleared/reset. How `--force-hooks` interacts with `--dry-run`.
   - F3-016: JSONL parsing from audit.log. Relative time formatting. What constitutes an "operation" (apply, update, snapshot restore?). Error state display.
   - F3-017: Whether existing `iron validate` becomes an alias or is deprecated. Editor launch behavior.
   - F3-018: File discovery algorithm (recursive walk). How nested directories are handled. Interaction with explicit `[[dotfiles]]` entries. Template detection on discovered files.

3. **Include carryover acceptance criteria** for the 3 SHOULD findings marked "IN SCOPE" above.

4. **Flag the STRETCH task** (F3-017) clearly so the architect and developer can deprioritize it.

5. **Consider TUI impact** for each task:
   - F3-014: Apply view must render `RunHook` actions. `Ask` behavior in TUI context.
   - F3-016: No TUI view needed (CLI-only for now).
   - F3-018: No new TUI views, but Apply view must show auto-discovered dotfile actions.

6. **Reference existing code patterns:**
   - Hook fields already exist on `Module` (`pre_install`, `post_install`, `pre_uninstall`, `status_check`) but are never executed
   - `ApplyAction` enum in `crates/iron-core/src/services/apply.rs` -- add `RunHook` variant
   - `IronState` in `crates/iron-core/src/state.rs` -- add `hooks_executed` field
   - `CommandExecutor` trait for hook execution (not raw `Command::new`)
   - Audit log JSONL format in state directory (XDG path from F3-006)

---

## Iteration Folder

All agent outputs go to: `docs/iterations/enhancement-phase3-sprint3.3/`

| Agent | Output File |
|-------|-------------|
| Analyst | `requirements.md` |
| Architect | `architecture.md` |
| Developer | `changes.md` |
| Tester | `test-report.md` |
| Reviewer | `review.md` |
