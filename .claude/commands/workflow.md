---
name: workflow
description: Entry point for the agent framework. Classifies the request and dispatches to the correct agent chain.
---

# /workflow

Start the agent workflow by describing what you need.

> **Have a vague idea?** Use `/discover "idea"` first to explore and produce a `docs/project-brief.md`. Then return here with a clear description.

## Usage

```
/workflow "description of what you need"
```

## Examples

```
/workflow "Create a REST API for user management with authentication"
/workflow "Fix the 500 error on the /api/users endpoint"
/workflow "Add WebSocket support for real-time notifications"
/workflow "Refactor the data access layer to use repository pattern"
```

## What Happens

1. The **orchestrator** reads your description and scans the workspace
2. It classifies the request as NEW_PROJECT, BUG_FIX, ENHANCEMENT, or REFACTOR
3. It selects the minimal agent chain needed (3-5 agents depending on type)
4. It creates an iteration folder in `docs/iterations/` for tracking
5. Each agent in the chain runs sequentially, reading the previous agent's output
6. Quality gates check deliverables between phases
7. The **reviewer** provides final sign-off with an evidence-based validation report

## Influencing Classification

The orchestrator detects request type from your description. To be explicit:

- Start with **"fix:"** or **"bug:"** for bug fixes
- Start with **"add:"** or **"feature:"** for enhancements
- Start with **"refactor:"** for refactors
- Start with **"create:"** or **"new:"** for new projects

## What Each Agent Does

| Agent | Role | Invoked For |
|-------|------|-------------|
| Analyst | Scopes requirements, analyzes bugs, defines deltas | All types |
| Architect | Designs structure, component boundaries, data models | NEW_PROJECT, structural ENHANCEMENT |
| Developer | Implements code following specs and codebase patterns | All types |
| Tester | Test strategy + implementation, coverage verification | NEW_PROJECT, BUG_FIX, ENHANCEMENT |
| Reviewer | Evidence-based review, quality gate, final sign-off | All types |
