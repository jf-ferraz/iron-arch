# Quality Review

Load this skill for deep code quality analysis. Organized by cognitive mode — each section is a question you ask while reviewing.

## How to Use

Work through each question in order. For each, scan the code changes and note findings classified by severity:
- **MUST**: Knowledge preservation or production reliability issue — blocking
- **SHOULD**: Project conformance issue — important, not blocking
- **COULD**: Structural improvement opportunity — suggestion only

Skip questions that don't apply to the changes under review (e.g., skip "Are boundaries clean?" for a single-file change).

---

## 1. Do Names and Types Express Intent?

**Principle**: A reader should understand what code does from names alone, without reading implementation.

**Detect**:
- Variables named `data`, `result`, `temp`, `val`, `item`, `obj`, `x`, `tmp`
- Functions named `process`, `handle`, `do`, `manage`, `run` without qualifying context
- Boolean names that don't read as questions: `flag`, `status`, `check` vs `isActive`, `hasPermission`, `canDelete`
- Types that don't describe domain concepts: `StringMap`, `DataObject`, `GenericProcessor`
- Abbreviated names where the full word fits: `usr`, `msg`, `btn`, `cfg`

**Threshold**: MUST if the name is actively misleading (says one thing, does another). SHOULD if the name is merely vague.

---

## 2. Is This Well-Structured?

**Principle**: Each unit of code has one reason to change. Smaller, focused units are easier to understand, test, and modify.

**Detect**:
- Functions longer than 50 lines → likely doing multiple things
- Classes/modules with more than 15 public methods → god object
- Functions with more than 4 parameters → likely missing a concept (extract parameter object)
- Nested conditionals deeper than 3 levels → extract early returns or helper functions
- Functions that return different types based on input → unclear contract

**Threshold**: MUST if a function handles unrelated responsibilities (e.g., validation + persistence + notification in one function). SHOULD if merely long but coherent.

---

## 3. Is This Idiomatic?

**Principle**: Code should use the language's and project's established patterns. Idiomatic code is predictable.

**Detect**:
- Manual iteration where built-in methods exist (e.g., manual loop vs `map`/`filter`/`reduce`)
- Reimplemented standard library functionality
- Error handling that fights the language's pattern (exceptions in Go, Results in Rust, etc.)
- Concurrency patterns that ignore the language's preferred model
- String concatenation where template literals/interpolation is available

**Threshold**: SHOULD for non-idiomatic patterns that work correctly. MUST only if the non-idiomatic approach introduces a bug risk (e.g., manual iteration with off-by-one potential).

---

## 4. Is This DRY and Consistent?

**Principle**: DRY means knowledge, not code. Same business rule in two places is duplication. Similar-looking code serving different purposes is not.

**Detect**:
- Same validation logic in multiple places
- Same calculation or business rule duplicated
- Configuration values hardcoded in multiple files
- Inconsistent patterns: one module uses callbacks, another uses promises, a third uses async/await for the same kind of operation

**Threshold**: MUST if duplicated knowledge means a business rule change requires finding all copies. SHOULD for inconsistent patterns. COULD for cosmetic inconsistency.

---

## 5. Is This Documented and Tested?

**Principle**: Public API boundaries need documentation. All behavior needs tests. Code that explains itself doesn't need comments.

**Detect**:
- Public functions without documentation (parameters, return values, error conditions)
- Comments that describe WHAT rather than WHY: `// increment counter` vs `// rate limiter requires monotonic counter`
- Temporal contamination in comments: "We changed X because..." → should be "X works because..."
- Code paths without test coverage, especially error paths and edge cases
- Tests that test implementation details (mock internals, test private methods)

**Threshold**: MUST for untested error handling in production paths. SHOULD for missing docs on public APIs. COULD for missing comments on non-obvious code.

---

## 6. Are Cross-File Patterns Clean?

**Principle**: Some quality issues only emerge at the codebase level — invisible in single-file review.

**Detect**:
- A single operation requires reading 5+ files with no documentation or orchestrator explaining the flow
- Same transformation or pattern implemented in 3+ files → missing abstraction trying to emerge
- Exported functions with 0 callers anywhere in the codebase → dead API surface
- Implicit contracts between files (file A assumes file B ran first, with no enforcement)
- Feature flags that are always true or always false → dead code

**Threshold**: MUST for implicit contracts that cause runtime failures. SHOULD for 3+ duplicated patterns where extraction is feasible. COULD for dead exports that haven't caused confusion yet.

Skip this question for single-file changes or small fixes. Activate for ENHANCEMENT, REFACTOR, and codebase-wide reviews.

---

## Review Report Format

```markdown
## Quality Review

### MUST
- [{file}:{line}] {observation} — {recommendation}

### SHOULD
- [{file}:{line}] {observation} — {recommendation}

### COULD
- [{file}:{line}] {observation} — {suggestion}

### Clean
{Areas reviewed with no findings — confirms they were checked, not skipped}
```

Always include the "Clean" section. It proves you reviewed those areas rather than skipping them.
