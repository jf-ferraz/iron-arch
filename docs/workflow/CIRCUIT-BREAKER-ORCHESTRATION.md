# Circuit Breaker Pattern - Task Orchestration

> **Generated**: 2026-02-13
> **Strategy**: Deep (Hierarchical Decomposition with Dependency Analysis)
> **Epic**: Task #4
> **Stories**: Tasks #5, #6, #7, #8

---

## Task Hierarchy

```
EPIC #4: Circuit Breaker Pattern (FR-5.9, NFR-8)
│
├── Story #5: Core Circuit Breaker State Machine [READY]
│   ├── 1.1: Define CircuitState enum
│   ├── 1.2: Implement CommandCircuitBreaker struct
│   ├── 1.3: Implement state transition logic
│   └── 1.4: Add configuration
│
├── Story #6: Command Execution Layer [blocked by #5]
│   ├── 2.1: Create CommandExecutor trait
│   ├── 2.2: Implement timeout wrapper (120s)
│   ├── 2.3: Integrate circuit breaker
│   └── 2.4: Add retry with backoff
│
├── Story #7: Infrastructure Integration [blocked by #6]
│   ├── 3.1: Integrate iron-pacman ──┐
│   ├── 3.2: Integrate iron-git ────┼── PARALLEL
│   ├── 3.3: Integrate iron-systemd ┘
│   └── 3.4: Add graceful degradation
│
└── Story #8: Testing & Validation [blocked by #7]
    ├── 4.1: Unit tests (state machine) ──┐
    ├── 4.2: Unit tests (timeout) ────────┼── PARALLEL
    ├── 4.3: Integration tests
    └── 4.4: E2E resilience validation
```

---

## Execution Strategy

### Phase A: Foundation (Story #5) - 5 hours
**Parallel Opportunities**: Task 1.1 + 1.4 can run concurrently
**Delegation**: `/sc:implement "Circuit breaker state machine"`

```
┌─────────────┐     ┌─────────────┐
│ Task 1.1    │     │ Task 1.4    │
│ CircuitState│ ──▶ │ Config      │ (parallel)
└──────┬──────┘     └──────┬──────┘
       │                   │
       ▼                   │
┌─────────────┐            │
│ Task 1.2    │◀───────────┘
│ Struct      │
└──────┬──────┘
       ▼
┌─────────────┐
│ Task 1.3    │
│ Transitions │
└─────────────┘
```

### Phase B: Execution Layer (Story #6) - 5 hours
**Sequential**: Strict dependency chain
**Delegation**: `/sc:implement "Command executor with timeout"`

```
Task 2.1 → Task 2.2 → Task 2.3 → Task 2.4
(trait)    (timeout)  (integrate) (retry)
```

### Phase C: Integration (Story #7) - 5 hours
**Parallel Opportunities**: Tasks 3.1, 3.2, 3.3 are independent
**Delegation**: `/sc:implement "Circuit breaker infrastructure integration"`

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ Task 3.1    │  │ Task 3.2    │  │ Task 3.3    │
│ iron-pacman │  │ iron-git    │  │ iron-systemd│
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        ▼
                ┌─────────────┐
                │ Task 3.4    │
                │ Degradation │
                └─────────────┘
```

### Phase D: Testing (Story #8) - 4 hours
**Parallel Opportunities**: Tasks 4.1, 4.2 can run concurrently
**Delegation**: `/sc:test "Circuit breaker comprehensive testing"`

```
┌─────────────┐  ┌─────────────┐
│ Task 4.1    │  │ Task 4.2    │ (parallel)
│ State tests │  │ Timeout tests│
└──────┬──────┘  └──────┬──────┘
       │                │
       └────────────────┘
                │
                ▼
        ┌─────────────┐
        │ Task 4.3    │
        │ Integration │
        └──────┬──────┘
               ▼
        ┌─────────────┐
        │ Task 4.4    │
        │ E2E Valid.  │
        └─────────────┘
```

---

## Critical Path Analysis

**Critical Path**: 1.1 → 1.2 → 1.3 → 2.1 → 2.2 → 2.3 → 3.1 → 4.3
**Duration**: ~12 hours (with parallel optimization)
**Without Parallelization**: ~19 hours

**Parallelization Savings**: ~7 hours (37% reduction)

---

## File Structure

```
crates/iron-core/
├── src/
│   ├── lib.rs                    # Add: pub mod resilience;
│   └── resilience/
│       ├── mod.rs                # Module exports
│       ├── circuit_breaker.rs    # Story #5: State machine
│       ├── command_executor.rs   # Story #6: Execution layer
│       └── tests.rs              # Story #8: Unit tests
│
crates/iron-core/tests/
└── circuit_breaker_integration.rs # Story #8: Integration tests

crates/iron-pacman/src/lib.rs     # Story #7: Integration point
crates/iron-git/src/lib.rs        # Story #7: Integration point
crates/iron-systemd/src/lib.rs    # Story #7: Integration point
```

---

## Delegation Commands

Execute in order:

```bash
# Phase A: Foundation (Story #5)
/sc:implement "CircuitState enum and CommandCircuitBreaker struct with state transitions"

# Phase B: Execution Layer (Story #6)
/sc:implement "CommandExecutor trait with 120s timeout and retry logic"

# Phase C: Integration (Story #7) - Can spawn 3 parallel agents
/sc:implement "Circuit breaker integration with iron-pacman"
/sc:implement "Circuit breaker integration with iron-git"
/sc:implement "Circuit breaker integration with iron-systemd"

# Phase D: Testing (Story #8)
/sc:test "Circuit breaker unit and integration tests"
```

---

## Acceptance Criteria

### Story #5 (Foundation)
- [ ] `CircuitState` enum with `Closed`, `Open`, `HalfOpen` variants
- [ ] `CommandCircuitBreaker` struct with configurable thresholds
- [ ] State transitions follow circuit breaker pattern
- [ ] Thread-safe implementation (Arc<Mutex<>>)

### Story #6 (Execution)
- [ ] `CommandExecutor` trait defined
- [ ] 120s timeout for all commands (configurable)
- [ ] `RetryableError` for timeout failures
- [ ] Exponential backoff (1s, 2s, 4s cap)

### Story #7 (Integration)
- [ ] iron-pacman uses circuit breaker for pacman commands
- [ ] iron-git uses circuit breaker for git commands
- [ ] iron-systemd uses circuit breaker for systemctl commands
- [ ] Each service has independent circuit state

### Story #8 (Testing)
- [ ] State transition tests (≥10 test cases)
- [ ] Timeout behavior tests (≥5 test cases)
- [ ] Concurrent access tests (≥3 test cases)
- [ ] Integration tests for failure scenarios (≥5 test cases)

---

## Next Action

**Start with**: Story #5 (Task #5) - Core Circuit Breaker State Machine

```bash
/sc:implement "Circuit breaker state machine in iron-core/src/resilience/"
```

This will create:
1. `CircuitState` enum
2. `CommandCircuitBreaker` struct
3. State transition logic
4. Configuration with defaults

---

*Generated by /sc:spawn --strategy deep*
