# Refactoring

Load this skill when improving code structure without changing behavior. Provides priority classification, safe techniques, and guardrails.

## Golden Rule

**Commit before refactoring. Always.** If the refactor goes wrong, you need a clean rollback point. Never start a refactor with uncommitted changes.

## Priority Classification

Not all code smells are equal. Classify before acting:

**Critical** — Fix now. These cause bugs or block development.
- Wrong abstractions (interface lies about what the code does)
- Hidden coupling (changing A breaks B with no visible connection)
- Race conditions or shared mutable state

**High** — Fix soon. These slow down development.
- Knowledge duplication (same business rule in multiple places — if one changes, others must too)
- God objects (>15 public methods) or god functions (>50 lines)
- Misleading names (name says one thing, code does another)

**Nice** — Fix when convenient. These are improvement opportunities.
- Style inconsistencies within a file
- Overly verbose code that could be simplified
- Missing type safety that hasn't caused issues yet

**Skip** — Don't touch. The cost of refactoring exceeds the benefit.
- Working code that's ugly but well-tested and rarely changed
- Generated code or vendored dependencies
- Cosmetic issues in code that's about to be replaced

## DRY = Knowledge, Not Code

Duplication is about **knowledge**, not about code that looks similar.

**Deduplicate this** (same knowledge):
```
# Two functions that both calculate tax with the same business rule
# If the tax rate changes, BOTH need updating → duplicated knowledge
```

**Don't deduplicate this** (coincidentally similar):
```
# A validation function and a formatting function that happen to have 
# similar structure but serve different purposes
# They'll evolve independently → NOT duplicated knowledge
```

The test: "If this changes for one reason, must it change in the other place too?" If yes, it's duplicated knowledge. If no, it's coincidence.

## Semantic Over Structural Abstraction

Abstract by **meaning**, not by **code shape**.

**Structural** (fragile): "These three functions all have a try-catch with logging, let me extract a wrapper" → Breaks when one function needs different error handling.

**Semantic** (stable): "These three functions all validate user input, let me create a validation pipeline" → Survives because the abstraction matches the concept.

The test: Can you name the abstraction with a domain term? If the best name is "doThingWrapper" or "processHelper", the abstraction is structural, not semantic.

## Safe Refactoring Steps

### Extract Function
1. Identify the code block to extract
2. Determine inputs (parameters) and outputs (return value)
3. Create the function with a name that describes WHAT, not HOW
4. Replace the original code with a call to the new function
5. Run tests — behavior is identical

### Rename
1. Use IDE rename (not find-replace) to catch all references
2. If renaming across module boundaries, check imports
3. Run tests — behavior is identical

### Simplify Conditional
1. Identify the complex conditional
2. Extract into named boolean or function: `isEligibleForDiscount()` over `(user.age > 65 || user.isPremium) && !user.isDelinquent`
3. Run tests — behavior is identical

### Extract Type/Interface
1. Identify repeated data shapes
2. Create a named type that describes the concept
3. Replace inline shapes with the named type
4. Run tests — behavior is identical

### Inline
1. Identify unnecessary indirection (function that just calls another function, variable used once)
2. Replace the reference with the actual content
3. Run tests — behavior is identical

## Verification

After every refactoring step:
1. Run the full test suite — zero failures or the refactor introduced a bug
2. Check type-checker output — zero new errors
3. Check linter output — zero new warnings (some may disappear, none should appear)
4. Review the diff — does it change only structure, never behavior?

If any test fails after a refactoring step, the step was wrong. Revert and re-approach.

## When Not to Refactor

- **Code you don't understand yet**: Read and understand first, then refactor
- **Code with no tests**: Add tests first (to lock behavior), then refactor
- **During a feature implementation**: Finish the feature, commit, THEN refactor as a separate commit
- **Code that's about to be replaced**: Don't polish what you're deleting
