# Code Quality

Universal standards applied across all agents and all technologies.

## Priority Hierarchy

When sources conflict, higher tier wins. Cite backing when auditing.

| Tier | Source | Action |
|------|--------|--------|
| 1 | User instruction | Apply as stated |
| 2 | Project docs (CLAUDE.md, README) | Apply |
| 3 | This convention | Apply |
| 4 | Assumption (no backing) | **Confirm with user** |

## Naming

Names communicate intent. A reader should understand what code does from names alone.

- Functions: verb + noun describing the action and target. `validateUser`, `calculateTax`, `fetchOrders`
- Booleans: read as yes/no questions. `isActive`, `hasPermission`, `canDelete`
- Collections: plural nouns. `users`, `orderItems`, `pendingRequests`
- Constants: describe the concept, not the value. `MAX_RETRY_ATTEMPTS` not `THREE`
- Avoid: `data`, `result`, `temp`, `info`, `item`, `obj`, `val`, `tmp`, `x`

**Misleading names are bugs.** A function named `validateUser` that also saves to the database has a misleading name. The name must match what the code does.
- **Threshold**: MUST if the name is actively misleading. SHOULD if merely vague.

## Structure

- **Function length**: Target ≤ 30 lines. Investigate at > 50 lines. A function over 50 lines is likely doing multiple things.
- **Parameters**: Target ≤ 3. Investigate at > 4. Group related parameters into a named object/struct.
- **Nesting depth**: Target ≤ 2 levels. Investigate at > 3. Use early returns, guard clauses, or extracted helper functions.
- **Public surface**: Target ≤ 10 methods per class/module. Investigate at > 15. Split by responsibility.
- **God objects**: >15 public methods OR >10 dependencies OR mixed concerns (networking + UI + data) — regardless of size.

These are guidelines, not absolute rules. A 60-line function that does one coherent thing (like a state machine) may be fine.
- **Threshold**: MUST if unrelated responsibilities are merged. SHOULD if merely long but coherent.

## Complexity

- **Prefer composition over inheritance**: Small, combinable units over deep hierarchies
- **Prefer explicit over clever**: Readable code over compact code
- **Prefer immutability**: Const/readonly by default. Mutability is the exception, documented with rationale.
- **Prefer standard library**: Use built-in solutions before external dependencies. Use well-maintained dependencies before rolling your own.

## Error Handling

- Handle errors at the appropriate level — where you have enough context to handle them meaningfully
- Don't swallow errors silently. At minimum: log with context.
- Distinguish recoverable from unrecoverable errors
- Input validation at boundaries (API endpoints, user input, external data). Trust internal data.
- Consistent strategy per project — don't mix exceptions and error codes for the same concern

## DRY = Knowledge, Not Code

Duplication is about **knowledge**, not about code that looks similar.

- **Deduplicate**: Same business rule in multiple places — if one changes, all must update
- **Don't deduplicate**: Similar-looking code serving different purposes that will evolve independently
- **The test**: "If this changes for one reason, must it change in the other place too?" If yes → duplicated knowledge. If no → coincidence.

Abstract by **meaning** (semantic), not by **code shape** (structural). If the best name for your abstraction is `doThingWrapper`, it's structural — don't extract it.

## File Organization

- Extend existing files before creating new ones
- Create new files only when: clear module boundary, >300-500 lines, or distinct responsibility
- Dead code (no callers, impossible branches, unused imports) should be removed
- Feature flags always true/false (never toggled) are dead code

## Temporal Contamination Rule

All comments and documentation must be written from the perspective of a first-time reader. No change narratives. See `conventions/temporal.md` for the full detection heuristic.

| Contaminated | Clean |
|-------------|-------|
| "We changed the handler because the old one didn't support pagination" | "The handler supports pagination via cursor-based traversal" |
| "This was added in v2 to fix the auth bug" | "Validates JWT tokens before processing requests" |
| "Previously this used callbacks, now uses async/await" | "Uses async/await for asynchronous operations" |

Comments describe the current state and WHY, never the history of how it got there. History lives in git.
