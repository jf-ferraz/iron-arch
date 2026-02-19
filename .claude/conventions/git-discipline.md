# Git Discipline

Standards for commit practices, branch management, and change tracking.

## Known-Good Increment

Every commit leaves the codebase in a working state:
- All tests pass
- Code compiles / lints / type-checks
- No partial implementations exposed (use feature flags if necessary)
- Could be deployed without immediate rollback

**Never commit a broken state.** If you're in the middle of a change and need to switch context, either finish the current task or stash your changes.

## Commit Practices

### When to Commit
- After completing a logical unit of work
- Before starting a refactor (clean rollback point)
- Before context switches (switching to a different task or agent)
- At natural stopping points during long implementations
- After every passing test cycle during TDD

### Commit Messages
```
{type}: {concise description}

{optional body — what and why, not how}
```

**Types**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

Examples:
```
feat: add user authentication endpoint
fix: handle null response from payment gateway
refactor: extract validation logic into shared module
test: add regression tests for order calculation
docs: update API documentation for v2 endpoints
```

### What NOT to Commit
- Debug logging or temporary test files (clean exit invariant)
- Commented-out code (delete it — git has history)
- Generated files that should be in .gitignore
- Secrets, credentials, or environment-specific configuration

## Branch Naming

Align branches with iteration descriptors when using the agent workflow:
```
feature/{descriptor}     # new features and enhancements
bugfix/{descriptor}      # bug fixes
refactor/{descriptor}    # refactoring work
```

Examples:
```
feature/user-auth
bugfix/login-500-error
refactor/data-layer
```

## During Agent Workflow

- **Orchestrator**: Commits the iteration overview after creating it
- **Developer**: Commits after completing each logical unit of implementation
- **Tester**: Commits after test suite additions
- **Reviewer**: Does not commit — only reviews

If the workflow is interrupted (context limit, error, user pause):
1. Commit current state with message: `wip: {what was in progress}`
2. Update `docs/current.md` with interruption context
3. The workflow can resume from the last commit
