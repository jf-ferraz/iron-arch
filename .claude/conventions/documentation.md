# Documentation

Standards for project documentation within the agent framework.

## Hierarchy

Every word must earn its tokens. Documentation has two tiers:

**CLAUDE.md — Pure Index (~200 tokens max)**
- Tabular format only: `| Resource | When to Read |`
- No explanatory prose, no examples, no rationale
- Auto-loaded in every context — keep it minimal
- Points to where detailed information lives
- Content test: "Is this a pointer or an explanation?" Only pointers belong here.

**README.md — Invisible Knowledge**
- Architecture decisions, design rationale, invariants
- Loaded on demand — can be longer
- Content test: "Could a developer learn this by reading the source files?" If yes, don't document it here.
- Target audience: a competent developer encountering this project for the first time

## Project Documentation Structure

```
docs/
├── project-brief.md          # Vision, deliverables, scope (filled by /discover or manually)
├── requirements.md           # Living document — what the system must do
├── architecture.md           # Living document — how the system is structured
├── current.md                # Active state — tasks, issues, priorities
└── iterations/
    └── {descriptor}/         # Per-change context
        ├── overview.md       # Classification, scope, agent chain
        ├── changes.md        # What was modified and why
        └── validation.md     # Review findings and sign-off
```

### Project Brief

`project-brief.md` is the upstream input for requirements. It captures the user's vision before formal analysis:
- **Created by**: `/discover` command (interactive) or manually
- **Consumed by**: analyst (as primary context for requirements)
- **Updated**: rarely — only if the project's vision fundamentally shifts
- **Not a living document** — it's a snapshot of original intent

### Invisible Knowledge

Knowledge NOT deducible from reading the code alone. Captured in `README.md` files **in the same directory as the affected code** (code-adjacent, not in a separate docs folder).

Categories:
- **Architecture decisions**: component relationships, data flow, module boundaries
- **Business rules**: domain constraints that shape implementation
- **Invariants**: properties that must hold but aren't enforced by types/compiler
- **Tradeoffs**: costs and benefits of chosen approaches
- **Performance characteristics**: non-obvious efficiency properties

**Self-contained principle**: Code-adjacent documentation must be self-contained. Do NOT reference external authoritative sources. If knowledge exists elsewhere, summarize it locally. Duplication is acceptable; maintenance burden is the cost of locality.

**The test**: Would a new team member understand this from reading the source files alone? If no, it's invisible knowledge and belongs in a README.md next to the code.

### Living Documents

`requirements.md` and `architecture.md` are updated incrementally:
- **Add** new sections when new features are built
- **Modify** existing sections when behavior changes
- **Never regenerate** the whole document — incremental updates preserve context
- **Version via git** — the document always represents current state

### Current State

`current.md` is the single source of truth for "what's happening now":
```markdown
# Current State

## Active Work
{What's being worked on right now}

## Known Issues
{Bugs, limitations, tech debt items — with severity}

## Recent Changes
{Last 3-5 changes — brief, linked to iteration folders}

## Next Priorities
{What's coming next — helps with context when returning to the project}
```

### Iteration Folders

Named `{type}-{descriptor}/`:
- `bugfix-login-timeout/`
- `enhancement-user-profiles/`
- `refactor-data-layer/`
- `new-payment-api/`

Lightweight enough that a bug fix generates 3 small files. Structured enough that a feature generates useful history.

## Trigger Quality Test

Before creating a new document, ask: "Will an agent or developer need to read this in the future?" If no, don't create it. Documentation that's never read is waste.

## Rules

1. **No documentation in project root.** Everything under `docs/`.
2. **No scattered files.** One `current.md` for active state, not five task files.
3. **No temporal contamination.** Docs describe current state, not change history.
4. **No regeneration.** Incremental updates only on living documents.
5. **Every document has a reader.** If no one will read it, don't write it.
