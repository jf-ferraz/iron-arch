# Validation Report — Phase 2: Power User Features

## Summary
- **Type**: ENHANCEMENT (19 tasks across 3 sprints)
- **Status**: APPROVED_WITH_NOTES
- **MUST findings**: 0
- **SHOULD findings**: 6
- **COULD findings**: 4

## Evidence
- Tests: PASS (2,033 tests; 0 failed; 4 ignored)
- Lint: PASS (clippy --workspace -- -D warnings: clean)
- Formatting: PASS (cargo fmt --all -- --check: clean)
- Scope adherence: IN_SCOPE (all 19 F2-xxx tasks addressed)

---

## MUST Findings

No blocking issues found.

All 19 acceptance criteria sets are met at the code level. Tests pass, no regressions, no raw `Command::new()` in new service code, no `unwrap()` in production paths, all new struct fields use `#[serde(default)]`, all `View::Snapshots` exhaustive match arms are updated (render, header, footer, help overlay, cycle_forward, cycle_backward, test_view_names).

---

## SHOULD Findings

### SHOULD-1: Auto-snapshot in `apply.rs` lacks package_manager — empty package lists

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/apply.rs`, lines 471-474
**Observation**: The auto-snapshot created before apply constructs `DefaultSnapshotService::new()` without `.with_package_manager()`. The `capture_state()` method at `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/snapshot_service.rs` lines 174-179 returns `unwrap_or_default()` when no package manager is available, meaning `explicit_packages` will always be an empty Vec in auto-snapshots created during apply.
**Impact**: Snapshot restore cannot accurately diff package state when restoring from auto-snapshots created during apply. The CLI snapshot_service factory at `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/context.rs` line 160 correctly calls `.with_package_manager()`, but the core service does not receive it.
**Recommendation**: Pass the `self.package_manager` from `DefaultApplyService` into the snapshot service construction. Same issue exists at `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/commands/update.rs` line 302 — but that one uses `ctx.snapshot_service()` which does include the package manager, so only the apply.rs path is affected.

### SHOULD-2: Snapshot restore does not converge system state — only updates metadata

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/commands/snapshot.rs`, lines 184-206
**Observation**: The `execute_restore()` function modifies `StateManager` (enable_module, disable_module, set_active_bundle, set_active_profile) but does not install/remove packages, link/unlink dotfiles, or enable/disable systemd services. The technical guide at `/home/fer/dev/projects/iron-arch/docs/phase2-technical-guide.md` section 2.1.4 explicitly specifies: "Execute plan via ApplyService" as step 6 of the restore flow.
**Impact**: After `iron snapshot restore`, the system metadata will reflect the snapshot but the actual system state (installed packages, symlinks, services) will not match. The user would need to manually run `iron apply` afterward.
**Recommendation**: After updating state metadata, invoke `ApplyService::plan()` and `ApplyService::execute()` (or at minimum, advise the user to run `iron apply` in the success message). This is the most significant functional gap in Phase 2.

### SHOULD-3: Security CLI command uses hardcoded ANSI escape codes, ignoring `--no-color`

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/commands/security.rs`, lines 27-38
**Observation**: The `execute()` function directly embeds `\x1b[31m`, `\x1b[33m`, `\x1b[32m`, `\x1b[36m`, and `\x1b[0m` escape codes via `output.raw()`. It does not check `output.no_color` or use the existing `Output` formatting methods that handle this.
**Impact**: Running `iron security --no-color` or piping output will include raw ANSI codes.
**Recommendation**: Use conditional formatting similar to other Output methods, or add a `level_badge()` helper to Output that respects `no_color`.

### SHOULD-4: Duplicated `truncate()` function across two crates

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/commands/snapshot.rs` line 369, `/home/fer/dev/projects/iron-arch/crates/iron-tui/src/ui/snapshot.rs` line 124
**Observation**: Identical `truncate(s: &str, max: usize) -> String` function exists in both CLI and TUI crates. Both use the same `&s[..max.saturating_sub(3)]` byte-slicing approach.
**Impact**: Code duplication. Also, byte-slicing can panic on multi-byte UTF-8 characters (though snapshot names are likely ASCII, the function is generic).
**Recommendation**: Extract to a shared utility (e.g., in iron-core or a shared utils module). Consider using `.chars().take(n)` for UTF-8 safety.

### SHOULD-5: SecurityService recommendation sorting parses points from formatted strings

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/security.rs`, lines 163-173
**Observation**: The `recommendations` Vec is sorted by extracting numeric points from the human-readable recommendation strings via string splitting (`s.split('+').nth(1).and_then(|p| p.split_whitespace().next())`). This is fragile — any change to the recommendation format will silently break sorting.
**Recommendation**: Sort `available_modules` by points before generating recommendation strings, rather than parsing points back out of formatted strings.

### SHOULD-6: `SnapshotRecord` does not implement `Default`

**File**: `/home/fer/dev/projects/iron-arch/crates/iron-core/src/services/snapshot_service.rs`, lines 29-71
**Observation**: The `SnapshotRecord` struct has `#[serde(default)]` on optional fields but does not derive `Default`. Test code at `/home/fer/dev/projects/iron-arch/crates/iron-tui/src/ui/snapshot.rs` lines 158-183 constructs records with all fields explicitly. The technical guide noted "SnapshotInfo struct uses builder pattern from day 1" as a Phase 1 lesson.
**Recommendation**: Add `#[derive(Default)]` and consider a builder for test construction to reduce boilerplate.

---

## COULD Findings

### COULD-1: `snapshot_service.rs` `load_all()` reads every file on each operation

The `get()`, `delete()`, and `prune_auto()` methods all call `load_all()` which reads the entire `.snapshots/` directory. For typical usage (under 50 snapshots) this is fine, but the technical guide mentioned an `index.json` lightweight index file (section 2.1.3) that was not implemented. Could become a concern with many snapshots.

### COULD-2: Progress spinner not integrated into any command flows

`/home/fer/dev/projects/iron-arch/crates/iron-cli/src/progress.rs` defines `ProgressReporter` and has tests, but it is not imported or used by any command module. The acceptance criteria mention it being used for sync and apply operations, but integration appears deferred.

### COULD-3: `Output::table()`, `Output::tree_*()`, `Output::summary_block()` marked `#[allow(dead_code)]`

These methods at `/home/fer/dev/projects/iron-arch/crates/iron-cli/src/output.rs` lines 331, 342, 355, 372, 442, 521, 548 are implemented and tested in the Output struct but flagged as dead code (not yet called from command modules). The snapshot list command at lines 95-111 builds its own table formatting inline rather than using `output.table()`.

### COULD-4: `SnapshotService` trait is not `dyn`-compatible due to lack of `Send + Sync`

The `SnapshotService` trait does not have `Send + Sync` bounds. While currently used as concrete types, the existing project pattern uses `Arc<dyn PackageManager>` etc. Adding these bounds now would prevent a breaking change if the TUI needs to call snapshot operations from background threads (as it does for sync operations).

---

## Sprint-by-Sprint Analysis

### Sprint 2.1: Snapshot and Rollback (F2-001 through F2-008)

**Strengths**: Clean trait design following the established service pattern. JSON file storage is simple and appropriate. `#[serde(default)]` on all optional fields ensures backward compatibility. Prune logic correctly preserves manual snapshots. Auto-snapshot wired into both apply and update paths. 13 unit tests cover CRUD, prune, serialization, and backward compatibility.

**Concerns**: Restore flow is metadata-only (SHOULD-2). Auto-snapshot in apply.rs lacks package data (SHOULD-1). No index file as spec'd (COULD-1).

### Sprint 2.2: Enhanced CLI Output (F2-009 through F2-014)

**Strengths**: Tree, table, and summary_block methods are well-implemented with proper handling of Text/Json/Minimal output modes and no-color support. ProgressReporter wraps indicatif cleanly with spinner, bar, abandon modes. Error suggestion system covers 10 error types as specified.

**Concerns**: Output methods are defined but mostly unused (COULD-3). ProgressReporter not integrated into commands (COULD-2). The `explain_cmd` method's doc comment references F2-013 appropriately.

### Sprint 2.3: Config Validation and Security (F2-015 through F2-019)

**Strengths**: SecurityLevel enum with `from_score()` is clean. Threshold tests cover all boundaries. Well-known security modules fallback ensures recommendations appear even without module directories. `security_points` field on Module uses `#[serde(default)]` and is reflected in test helpers. Validate command properly delegates to ApplyService.

**Concerns**: Security CLI ignores no-color (SHOULD-3). Recommendation sorting is fragile (SHOULD-5).

---

## Phase 3 Readiness Assessment

Phase 3 targets multi-machine features. Based on the current implementation:

1. **SnapshotRecord.host_id**: Already present and populated from state. Multi-machine snapshot filtering will work.

2. **StateManager cloning**: Services receive `StateManager` by value (clone). For multi-host scenarios, this pattern should hold since each host would have its own state file.

3. **SecurityService**: Currently scans a single `modules/` directory. Multi-machine would require parameterizing the modules path per host.

4. **SnapshotService storage**: `.snapshots/` is a single flat directory. Multi-host would benefit from per-host subdirectories or filtering by `host_id` in `load_all()`.

5. **No blocking architectural issues** for Phase 3. The trait-based DI and service layer patterns are consistent and extensible.

---

## Sign-off

**APPROVED_WITH_NOTES**

Zero MUST findings. The implementation addresses all 19 Phase 2 tasks, tests pass comprehensively (2,033 total), and the code follows established project patterns (trait-based services, builder injection, serde defaults, exhaustive View matches).

The most significant non-blocking concern is **SHOULD-2** (snapshot restore only updates metadata, not system state). This means `iron snapshot restore` does not deliver the full safety-net value promised by the Phase 2 goal of "fearless experimentation." Users must run `iron apply` after restore to converge actual system state. This should be addressed before Phase 3, either by integrating ApplyService into the restore flow or by adding a clear post-restore message instructing users to run `iron apply`.

SHOULD-1 (empty package lists in auto-snapshots from apply) compounds this issue and should also be prioritized.

The remaining SHOULD items (SHOULD-3 through SHOULD-6) are code quality concerns that do not affect correctness when used as designed.
