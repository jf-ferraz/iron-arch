# Mind Agent Framework

| Resource | When to Read |
|----------|-------------|
| `agents/orchestrator.md` | Starting any workflow — classifies requests, routes to agent chains, resumes interrupted workflows |
| `agents/analyst.md` | Requirements analysis, context gathering, scope definition |
| `agents/architect.md` | System design, structural decisions (NEW_PROJECT or structural changes only) |
| `agents/developer.md` | Implementation, code writing, incremental changes |
| `agents/tester.md` | Test strategy, test implementation, coverage verification |
| `agents/reviewer.md` | Code review, quality validation, final sign-off |
| `agents/discovery.md` | Interactive project exploration — transforms vague ideas into structured briefs |
| `skills/` | Deep-dive reference — load on demand when agent needs detailed guidance |
| `conventions/` | Universal rules — code quality, documentation, git, severity classification |
| `commands/discover.md` | Entry point: `/discover "idea"` — explore before building |
| `commands/workflow.md` | Entry point: `/workflow "description"` — build with full agent pipeline |

## Request Types → Agent Chains

| Type | Trigger | Chain |
|------|---------|-------|
| `NEW_PROJECT` | No existing codebase | analyst → architect → developer → tester → reviewer |
| `BUG_FIX` | Fix, bug, error, broken, regression | analyst → developer → tester → reviewer |
| `ENHANCEMENT` | Add, extend, improve, integrate | analyst → [architect]* → developer → tester → reviewer |
| `REFACTOR` | Refactor, clean, restructure, optimize | analyst → developer → reviewer |

*Architect activates only when structural changes are needed.

## Quality Gates

- **After planning** (analyst + architect): Requirements are testable, scoped, and bounded before proceeding
- **After development** (developer + tester): Tests pass, implementation addresses all requirements
- **Max 2 retry loops** — then proceed with documented concerns
