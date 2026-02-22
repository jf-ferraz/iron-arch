# Debugging

Load this skill when facing hard-to-diagnose problems: intermittent failures, unclear root causes, complex system interactions.

## Systematic Debugging Protocol

### Step 1: Reproduce

Before analyzing, establish reliable reproduction:
- **Minimum viable reproduction**: Strip away everything unnecessary until you have the shortest path to the bug
- **Consistent trigger**: If intermittent, identify the conditions that increase frequency
- **Environment isolation**: Confirm the bug isn't environment-specific (or confirm it is)

If you can't reproduce it, you can't verify the fix. Do not proceed to root cause analysis without reproduction.

### Step 2: Gather Evidence

Minimum evidence threshold before forming a hypothesis:
- **10+ observations** — log statements, debug output, variable inspections
- **3+ test inputs** — different data that triggers the bug, different data that doesn't
- **Isolated reproduction** — narrowed down to smallest triggering scenario

Use **open verification questions** (70% accuracy) instead of closed questions (17% accuracy):
- "What value does X hold at this point?" — not "Is X null?"
- "What happens when the input is empty?" — not "Does it handle empty input?"
- "Which code path executes when condition Z is true?" — not "Does it take the right path?"

Open questions force investigation. Closed questions invite assumption confirmation.

### Step 3: Form Hypothesis

Based on evidence, form a specific, falsifiable hypothesis:
- "The error occurs because X is null when function Y accesses it, because the async call on line Z hasn't completed"
- NOT "something is probably wrong with the async handling"

A good hypothesis:
- Explains ALL observations (if it doesn't explain one, it's incomplete)
- Can be disproven with a specific test
- Identifies the mechanism, not just the location

### Step 4: Verify

Prove the hypothesis through targeted tests:
- Add a check at the suspected point — does it confirm the hypothesis?
- Modify the suspected cause — does the bug disappear?
- Test edge cases around the hypothesis — does it explain related behaviors?

If verification fails, return to Step 2 with new evidence. Do not patch without verification.

### Step 5: Fix

Only after reproduction + evidence + verified hypothesis:
- Fix the root cause, not the symptom
- The fix should make your reproduction test pass
- Existing tests should still pass
- Add a regression test that would catch this bug if it recurs

### Clean Exit Invariant

Before declaring debugging complete:
1. Remove ALL debug statements (log statements, temporary variables, debug flags)
2. Remove ALL temporary test files or harnesses
3. Verify the fix with the reproduction test
4. Verify no debug artifacts remain: `grep -r "DEBUG\|TEMP\|TODO.*debug\|console.log.*debug" --include="*.{js,ts,py,go,rs,java,cs}"`

The codebase must be cleaner after debugging than before. Debug artifacts in production code are a defect.

## Anti-Patterns

| Anti-Pattern | Problem | Instead |
|-------------|---------|---------|
| Shotgun debugging | Change random things hoping something works | Systematic evidence gathering |
| Confirmation bias | Only looking for evidence that confirms your theory | Test the opposite of your hypothesis |
| Fix-and-pray | Apply a fix without understanding the cause | Verify hypothesis before fixing |
| Scope creep | "While I'm here, let me also fix..." | Fix only the investigated bug. Log others. |
| Incomplete cleanup | Leave debug logging/tooling in place | Clean exit invariant |

## When to Escalate

If after 3 hypothesis-verify cycles you haven't found the root cause:
- Document what you've tried, what you've eliminated, and what evidence remains unexplained
- The findings so far narrow the search space for the next attempt
- Switch context — fresh perspective often sees what repeated investigation misses
