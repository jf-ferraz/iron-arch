# Planning

Load this skill when facing complex, multi-step work that needs structured decomposition before implementation.

## Three-Document Model

Maintain three living documents during complex work:

**PLAN.md** — The map. What needs to happen and in what order.
```markdown
## Goal
{One sentence — what "done" looks like}

## Tasks
1. [ ] {task} — depends on: {nothing|task N}
2. [ ] {task} — depends on: {task 1}
3. [ ] {task} — depends on: {nothing}

## Decision Log
| Decision | Reasoning Chain |
|----------|----------------|
| {what you decided} | {premise → implication → conclusion} |

Each rationale must contain at least 2 reasoning steps. Single-step rationales are insufficient.
- INSUFFICIENT: "Polling over webhooks | Webhooks are unreliable"
- SUFFICIENT: "Polling over webhooks | 30% webhook delivery failure in testing → unreliable delivery requires fallback anyway → simpler to poll as primary"

Include both architectural AND implementation-level micro-decisions.

## Rejected Alternatives
| Alternative | Why Rejected |
|-------------|-------------|
| {approach not taken} | {concrete reason: performance, complexity, constraint mismatch} |

## Invisible Knowledge
{Knowledge NOT deducible from reading the code alone}
- Architecture decisions: why components relate this way
- Business rules: domain constraints shaping implementation
- Invariants: properties that must hold but aren't enforced by types
- Tradeoffs: known compromises and their costs

## Risks
| Risk | Impact | Mitigation |
|------|--------|-----------|
| {what could go wrong} | {consequence} | {prevention or contingency} |
```

**WIP.md** — The status. What's happening right now.
```markdown
## Current Task
{Task N from PLAN.md}

## Status
🔴 RED — Writing failing test
🟢 GREEN — Making test pass
🔵 REFACTOR — Improving structure
⏸️ WAITING — Awaiting approval/decision

## Progress
- [x] {completed sub-step}
- [ ] {next sub-step}

## Blockers
{Anything preventing progress}

## Next Action
{Specific next thing to do}
```

**LEARNINGS.md** — The memory. What was discovered during work.
```markdown
## Discoveries
- {unexpected finding — behavior, constraint, pattern}

## Corrections
- {something assumed that turned out wrong}

## Conventions Detected
- {patterns found in codebase that should be followed}
```

## Lifecycle

1. **Start**: Create PLAN.md with task breakdown
2. **During work**: Keep WIP.md current. Add to LEARNINGS.md as things are discovered.
3. **Task completion**: Check off task in PLAN.md, update WIP.md to next task
4. **Feature completion**:
   - Merge gotchas and patterns from LEARNINGS.md into project CLAUDE.md
   - Merge architectural decisions into `docs/decisions/` as ADRs
   - Merge invisible knowledge into relevant README.md files (code-adjacent)
   - Delete PLAN.md, WIP.md, LEARNINGS.md

The knowledge lives on in permanent locations. Planning docs are ephemeral.

## Task Decomposition

Break work into tasks that are:
- **Independent where possible** — minimize sequential dependencies
- **Verifiable** — each task has a clear "done" state
- **Small** — completable in a single focused session
- **Ordered by dependency** — prerequisites before dependents
- **Ordered by risk** — uncertain/risky tasks first (fail fast)

## Known-Good Increment

Every completed task should leave the codebase in a working state. If a task can't be completed without breaking something, it's too big — decompose further.

A known-good increment means:
- All existing tests pass
- No partial implementations exposed (feature flags if needed)
- Code compiles/lints/type-checks
- Could be committed and deployed without rollback

## When Not to Plan

Skip formal planning for:
- Single-file changes with clear scope
- Bug fixes with obvious root cause
- Documentation updates
- Dependency updates

The overhead of planning should be proportional to the uncertainty of the work.
