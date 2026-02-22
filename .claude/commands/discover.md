---
name: discover
description: Interactive project discovery. Explores vision, constraints, and deliverables before formal requirements.
---

# /discover

Explore and define a project idea before building. The discovery agent asks targeted questions and produces a structured project brief.

## Usage

```
/discover "your project idea"
```

## Examples

```
/discover "I want to build an inventory management system for small warehouses"
/discover "I need a tool that monitors our microservices and alerts on failures"
/discover "A mobile-friendly dashboard for tracking sales metrics"
```

## What Happens

1. The **discovery** agent reads your description and identifies knowledge gaps
2. It asks 5-8 targeted questions in small batches (2-3 at a time)
3. You answer conversationally — no formal format needed
4. It synthesizes your answers into `docs/project-brief.md`
5. You review and confirm the brief
6. When ready, run `/workflow` to start building

## When to Use

- **Before `/workflow`** — when you have an idea but haven't defined specifics
- **New projects** — to extract vision, users, deliverables, and boundaries
- **Major features** — to clarify scope before formal requirements analysis

## When to Skip

- You already have clear, detailed requirements
- The task is a bug fix, refactor, or small enhancement
- You've already written `docs/project-brief.md` manually

## Output

The brief captures:

| Section | Purpose |
|---------|---------|
| Vision | What it does and why — prevents scope drift |
| Target Users | Who uses it and their goals — shapes priorities |
| Problem Statement | What problem it solves — prevents solution-first thinking |
| Key Deliverables | Tangible outputs — makes "done" concrete |
| Success Metrics | How we'll know it worked — prevents gold-plating |
| Scope (In/Out) | Boundaries — prevents creep |
| Constraints | Hard limits — shapes trade-offs |
| Open Questions | Unknowns — tracked for later resolution |

## After Discovery

```
/workflow "Build the system described in the project brief"
```

The analyst will read `docs/project-brief.md` automatically and produce formal requirements from it.
