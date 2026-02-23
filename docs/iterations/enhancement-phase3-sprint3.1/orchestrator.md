# Orchestrator Handoff — Phase 3 Sprint 3.1

## Classification

- **Type**: ENHANCEMENT (structural)
- **Rationale**: Sprint 3.1 introduces new domain models (ActualState, IronEnvelope), new source files (actual_state.rs, envelope.rs, status.rs, plan.rs), changes to existing service API contracts (compute_plan, detect signatures), and new CLI commands. This requires architectural review.
- **Agent Chain**: analyst → architect → developer → tester → reviewer
- **Architect Required**: Yes — new domain contracts, API signature changes cascading through CLI + TUI, XDG directory separation

## Sprint 3.1 Scope — 9 Tasks

| Task | Title | Priority | Effort | Dependencies |
|------|-------|----------|--------|-------------|
| F3-001 | ActualState struct and contract | Critical | M | None |
| F3-002a | scan_actual_state() implementation | Critical | M | F3-001 |
| F3-002b | Refactor compute_plan/detect consumers | Critical | L | F3-002a |
| F3-003a | Response envelope infrastructure | Critical | M | None |
| F3-003b | Migrate existing --json to envelope | Medium | M | F3-003a |
| F3-004 | `iron status` command | Critical | M | F3-002b, F3-003a |
| F3-005 | `iron plan` command | Critical | M | F3-002b, F3-003a |
| F3-006 | XDG state directory separation | Critical | M | None |
| F3-007 | Legacy state migration | Medium | S | F3-006 |

## Task Dependency Ordering

The technical guide specifies this implementation order:

```
Phase 1 (parallel starts):
  F3-001 (ActualState struct)
  F3-003a (Envelope infra)     -- can parallel with F3-001
  F3-006 (XDG state dir)       -- can parallel with F3-001

Phase 2 (after phase 1 completes):
  F3-002a (scan impl)          -- depends on F3-001
  F3-007 (state migration)     -- depends on F3-006

Phase 3 (after F3-002a):
  F3-002b (consumer refactor)  -- depends on F3-002a, cascading change

Phase 4 (after F3-002b and F3-003a):
  F3-003b (envelope migration) -- depends on F3-003a
  F3-004 (iron status)         -- depends on F3-002b, F3-003a
  F3-005 (iron plan)           -- depends on F3-002b, F3-003a
```

## Key Architectural Decisions (from phase3-technical-guide.md)

1. **D5: Single ActualState scan** — One `ActualState::scan()` replaces all ad-hoc system queries in compute_plan() and detect(). Ensures consistency.

2. **D6: iron plan is display-only** — No `--output` or `--plan` serialization in Phase 3. ApplyPlan contains trait references that complicate serialization.

3. **D4: XDG state separation** — Runtime state moves to `$XDG_STATE_HOME/iron/`. Resolution priority: `$IRON_STATE_DIR` > `$XDG_STATE_HOME/iron` > `~/.local/state/iron`.

4. **Envelope pattern** — `IronEnvelope<T>` wraps all `--json` output with `ok`, `command`, `data`, `error`, `meta` fields.

5. **Status performance** — `iron status` targets < 2 second latency. Uses cached state data, NOT full ActualState::scan(). `--full` flag triggers full scan.

## Cascading Changes (F3-002b)

The consumer refactor is the highest-risk task. It changes the signatures of:
- `ApplyService::compute_plan()` — adds `&ActualState` parameter
- `DriftService::detect()` — adds `&ActualState` parameter

Affected files:
- `iron-core/src/services/apply.rs`
- `iron-core/src/services/drift.rs`
- `iron-cli/src/commands/apply.rs`
- `iron-cli/src/commands/diff.rs`
- `iron-cli/src/commands/snapshot.rs`
- `iron-tui/src/ui/apply.rs`
- `iron-tui/src/app/actions.rs`
- All integration and unit tests calling these functions

## New Files to Create

```
iron-core/src/actual_state.rs    -- ActualState struct + scan
iron-core/src/envelope.rs        -- IronEnvelope<T> response wrapper
iron-cli/src/commands/status.rs  -- CLI iron status command
iron-cli/src/commands/plan.rs    -- CLI iron plan command
```

## Crate Dependencies to Verify/Add

- `gethostname` or `hostname` crate for ActualState hostname
- `chrono` for timestamps (likely already present)
- `dirs` for XDG resolution (or manual env var approach)
- `sha2` for file checksums (may already be present)

## Phase 2 Lessons to Apply

- L1: All new struct fields use `#[serde(default)]` and update test helpers in same PR
- L2: All new CLI commands include `--dry-run` for integration tests
- L3: New Output methods wired into commands immediately (no dead_code)
- L8: All new traits include `Send + Sync` bounds from day 1
- L9: List all affected files when changing trait signatures

## Instructions for Analyst

Begin scoping Sprint 3.1 by:

1. Read the existing source files that will be modified to understand current API contracts:
   - `iron-core/src/services/apply.rs` — current compute_plan() signature and system queries
   - `iron-core/src/services/drift.rs` — current detect() signature and system queries
   - `iron-core/src/services/state.rs` — current StateManager and state.json location
   - `iron-cli/src/output.rs` — current json_value() method
   - `iron-cli/src/context.rs` — current path resolution
   - `iron-core/src/lib.rs` — module registration

2. Verify crate dependencies in `iron-core/Cargo.toml` and `iron-cli/Cargo.toml`

3. Identify all test helpers that construct State, Host, Module structs (they need updating)

4. Produce a scoped requirements document with:
   - Exact API signatures for ActualState::scan() and IronEnvelope
   - Complete list of files affected by F3-002b consumer refactor
   - Test strategy per task
   - Risk assessment (F3-002b cascading changes are highest risk)

## Reference Documents

- `/home/fer/dev/projects/iron-arch/docs/phase3-kanban.md` — Full task descriptions and acceptance criteria
- `/home/fer/dev/projects/iron-arch/docs/phase3-technical-guide.md` — Implementation specs with code examples
- `/home/fer/dev/projects/iron-arch/docs/current.md` — Current project state
- `/home/fer/dev/projects/iron-arch/CLAUDE.md` — Build commands, architecture, code style
