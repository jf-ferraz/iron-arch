# Severity Classification

How to classify issues, findings, and quality concerns across the framework.

## Three Levels

### MUST — Blocking

Knowledge preservation and production reliability issues. Work cannot proceed with unresolved MUST items.

**Examples:**
- Implementation doesn't match requirements
- Tests fail or are missing for critical paths
- Data can be lost, corrupted, or unsafely exposed
- Error handling is absent in production code paths
- Security boundaries are violated (auth bypass, injection vectors)
- Existing behavior regresses without intentional change

**Verification rule:** Before declaring a MUST violation, verify through dual-path reasoning:
1. Forward: "The code does X → this leads to problem Y"
2. Backward: "Problem Y requires condition Z → the code has condition Z"

Both paths must confirm. If only one confirms, downgrade to SHOULD with noted uncertainty.

### SHOULD — Important

Project conformance issues. Important for consistency and maintainability but don't block delivery.

**Examples:**
- New code doesn't follow established project patterns
- API contracts differ from architect's specification
- Test coverage is insufficient (but critical paths are covered)
- Names are vague but not misleading
- Documentation is missing for public APIs

### COULD — Suggestion

Structural improvement opportunities. Nice to have, low priority.

**Examples:**
- Code could be simplified
- Performance could be optimized
- Better abstractions are available
- Minor inconsistencies between files
- Cosmetic improvements

## Intent Markers

Code authors can mark intentional deviations to prevent false-positive review findings. Reviewers respect these markers.

| Marker | Meaning | Reviewer Action |
|--------|---------|----------------|
| `:PERF:` | Performance-motivated pattern (may look overcomplicated) | Don't flag complexity |
| `:UNSAFE:` | Known unsafe operation with documented justification | Don't flag as security issue |
| `:SCHEMA:` | Schema-driven design choice (may look over-engineered) | Don't suggest simplification |
| `:TEMP:` | Temporary solution — known tech debt | Note for tracking, don't block review |

### Usage in Code
```
// :PERF: Using manual loop instead of map() — measured 3x faster for arrays > 10k elements
for (let i = 0; i < items.length; i++) { ... }

# :UNSAFE: Raw SQL required for recursive CTE — ORM doesn't support this query pattern
cursor.execute(raw_sql)

// :TEMP: Hardcoded timeout — will be configurable in #234
const TIMEOUT = 5000;
```

Intent markers require a justification on the same line or the line immediately following. A marker without justification is not valid — reviewer should flag it as a SHOULD finding.

## Decision Flowchart

When classifying a finding:

```
Is someone's data at risk?                    → MUST
Does the code do what the spec says?     No → MUST
Do existing tests still pass?            No → MUST
Is there an auth/security gap?           Yes→ MUST
Does new code follow project patterns?   No → SHOULD  
Is there missing test coverage?          Yes→ SHOULD
Is the code correct but inelegant?       Yes→ COULD
Would you explain this to a colleague?   No → Not a finding
```
