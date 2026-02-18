# Coverage Improvement Workflow

> Systematic plan to achieve 75-80% test coverage from current 58.78%

Generated: 2026-02-13 | Target: 80% | Current: 58.78% (5125/8719 lines)

## Executive Summary

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| Coverage | 58.78% | 80% | +21.22% |
| Lines Covered | 5,125 | ~6,975 | ~1,850 |
| Test Count | 1,200 | ~1,600 | ~400 |

**Projected Outcome**: 75-80% coverage achievable with Phases 1-4

---

## Phase 1: Mock Command Executor Infrastructure

**Impact**: HIGH (+8-12% coverage)
**Effort**: 2-3 days
**Dependencies**: None

### Objective
Create comprehensive MockCommandExecutor to simulate system command responses without actual execution.

### Tasks

#### 1.1 Create MockCommandExecutor Trait Implementation
- **File**: `crates/iron-core/src/resilience/mock_executor.rs`
- **Description**: Implement CommandExecutor trait with configurable responses
- **Acceptance Criteria**:
  - [ ] Supports success/failure configuration per command
  - [ ] Supports output stubbing for specific command patterns
  - [ ] Supports timeout simulation
  - [ ] Supports circuit breaker triggering

```rust
// Example structure
pub struct MockCommandExecutor {
    responses: HashMap<String, MockResponse>,
    failure_mode: Option<FailureMode>,
    call_count: AtomicUsize,
}

pub enum MockResponse {
    Success { stdout: String, stderr: String },
    Failure { exit_code: i32, stderr: String },
    Timeout,
    CircuitOpen,
}
```

#### 1.2 Add Pacman Mock Responses
- **File**: `crates/iron-pacman/src/test_fixtures.rs`
- **Description**: Pre-built mock responses for pacman operations
- **Commands to mock**:
  - [ ] `pacman -Qi <pkg>` - Package info
  - [ ] `pacman -Qe` - Explicit packages list
  - [ ] `pacman -Qu` - Available updates
  - [ ] `pacman -Ss <query>` - Search results
  - [ ] `pacman -Sy` - Sync database
  - [ ] `paccache -d` - Cache cleanup

#### 1.3 Add Git Mock Responses
- **File**: `crates/iron-git/src/test_fixtures.rs`
- **Description**: Pre-built mock responses for git operations
- **Commands to mock**:
  - [ ] `git status --porcelain -b` - Repository status
  - [ ] `git diff` - Uncommitted changes
  - [ ] `git log --oneline -n 10` - Recent commits
  - [ ] `git push/pull` - Remote operations
  - [ ] `git-crypt status -e` - Encrypted files

#### 1.4 Add Systemd Mock Responses
- **File**: `crates/iron-systemd/src/test_fixtures.rs`
- **Description**: Pre-built mock responses for systemctl operations
- **Commands to mock**:
  - [ ] `systemctl status <service>` - Service status
  - [ ] `systemctl is-enabled <service>` - Enable state
  - [ ] `systemctl list-units --type=service` - Service list

#### 1.5 Add Snapshot Tool Mock Responses
- **File**: `crates/iron-core/src/snapshot_fixtures.rs`
- **Description**: Mock responses for timeshift/snapper
- **Commands to mock**:
  - [ ] `timeshift --list` - Snapshot list
  - [ ] `timeshift --create` - Create snapshot
  - [ ] `snapper list` - Snapper snapshots
  - [ ] `snapper create` - Create snapshot

#### 1.6 Integrate Mocks into Existing Tests
- **Files**: All `**/tests.rs` in affected crates
- **Description**: Wire mock executor into service constructors
- **Acceptance Criteria**:
  - [ ] iron-pacman tests use mock executor
  - [ ] iron-git tests use mock executor
  - [ ] iron-systemd tests use mock executor
  - [ ] iron-core update/snapshot tests use mock executor

### Validation Checkpoint
```bash
cargo test --workspace
cargo tarpaulin --workspace | grep "coverage"
# Expected: 66-70% coverage
```

---

## Phase 2: TUI Event System Testing

**Impact**: MEDIUM (+3-5% coverage)
**Effort**: 1-2 days
**Dependencies**: None (parallel with Phase 1)

### Objective
Expand TestBackend-based testing to cover event handling and terminal management.

### Tasks

#### 2.1 Create Mock Event Stream
- **File**: `crates/iron-tui/src/event/mock.rs`
- **Description**: Generate synthetic events for testing
- **Features**:
  - [ ] Key event generation (arrows, enter, escape, chars)
  - [ ] Resize event simulation
  - [ ] Tick event timing
  - [ ] Signal simulation (SIGINT, SIGTERM)

```rust
pub struct MockEventStream {
    events: VecDeque<Event>,
}

impl MockEventStream {
    pub fn with_events(events: Vec<Event>) -> Self;
    pub fn next_event(&mut self) -> Option<Event>;
}
```

#### 2.2 Test Event Handler Coverage
- **File**: `crates/iron-tui/src/event.rs` (add tests section)
- **Description**: Test all event handler code paths
- **Test cases**:
  - [ ] Key press handling for each key type
  - [ ] Event polling with timeout
  - [ ] Event queue overflow handling
  - [ ] Cross-term event conversion

#### 2.3 Test Terminal State Management
- **File**: `crates/iron-tui/src/terminal.rs` (add tests section)
- **Description**: Test terminal initialization and restoration
- **Test cases**:
  - [ ] Terminal creation with TestBackend
  - [ ] Terminal size detection
  - [ ] Alternate screen handling (mocked)
  - [ ] Raw mode handling (mocked)

#### 2.4 Test Application Lifecycle
- **File**: `crates/iron-tui/src/lib.rs` (add tests section)
- **Description**: Test application startup and shutdown
- **Test cases**:
  - [ ] App initialization with mock terminal
  - [ ] Panic hook registration verification
  - [ ] Graceful shutdown sequence

### Validation Checkpoint
```bash
cargo test -p iron-tui
cargo tarpaulin -p iron-tui | grep "coverage"
# Expected: 85-90% TUI coverage
```

---

## Phase 3: Service Layer Mock Integration

**Impact**: MEDIUM (+5-7% coverage)
**Effort**: 2-3 days
**Dependencies**: Phase 1 (MockCommandExecutor)

### Objective
Create comprehensive test harnesses for iron-core services using mock dependencies.

### Tasks

#### 3.1 Create PackageManager Mock
- **File**: `crates/iron-core/src/packages/mock.rs`
- **Description**: Mock implementation of PackageManager trait
- **Features**:
  - [ ] Configurable package database
  - [ ] Simulated install/remove operations
  - [ ] Update checking with preset results
  - [ ] AUR helper simulation

#### 3.2 Create GitManager Mock
- **File**: `crates/iron-core/src/services/sync/mock.rs`
- **Description**: Mock implementation for SyncService
- **Features**:
  - [ ] Configurable repository state
  - [ ] Simulated push/pull with conflicts
  - [ ] Dirty state detection

#### 3.3 Enhance BundleService Tests
- **File**: `crates/iron-core/src/services/bundle.rs`
- **Description**: Add tests using mock package manager
- **Test cases**:
  - [ ] Bundle installation with package dependencies
  - [ ] Bundle switching with cleanup
  - [ ] Bundle validation failures
  - [ ] Partial installation recovery

#### 3.4 Enhance HostService Tests
- **File**: `crates/iron-core/src/services/host.rs`
- **Description**: Add hardware detection mocking
- **Test cases**:
  - [ ] Hardware spec detection paths
  - [ ] Chassis type identification
  - [ ] Host initialization workflow

#### 3.5 Enhance SyncService Tests
- **File**: `crates/iron-core/src/services/sync.rs`
- **Description**: Add git operation simulation
- **Test cases**:
  - [ ] Push with conflicts
  - [ ] Pull with merge required
  - [ ] Status with uncommitted changes
  - [ ] Remote tracking branch handling

#### 3.6 Enhance RecoveryService Tests
- **File**: `crates/iron-core/src/services/recovery.rs`
- **Description**: Test export/import workflows
- **Test cases**:
  - [ ] Full configuration export
  - [ ] Selective module export
  - [ ] Import with conflict resolution
  - [ ] Backup verification

### Validation Checkpoint
```bash
cargo test -p iron-core
cargo tarpaulin -p iron-core | grep "coverage"
# Expected: 75-80% iron-core coverage
```

---

## Phase 4: CLI Integration Testing

**Impact**: LOW-MEDIUM (+2-4% coverage)
**Effort**: 1-2 days
**Dependencies**: Phases 1-3

### Objective
Create CLI integration tests that verify command behavior without system modification.

### Tasks

#### 4.1 Create CLI Test Harness
- **File**: `crates/iron-cli/tests/cli_harness.rs`
- **Description**: Infrastructure for capturing CLI output
- **Features**:
  - [ ] Stdout/stderr capture
  - [ ] Exit code verification
  - [ ] JSON output parsing
  - [ ] Environment variable mocking

```rust
pub struct CliTestHarness {
    root: TempDir,
    env_vars: HashMap<String, String>,
}

impl CliTestHarness {
    pub fn run(&self, args: &[&str]) -> CliOutput;
}

pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
```

#### 4.2 Test Help and Version Output
- **File**: `crates/iron-cli/tests/cli_basic.rs`
- **Description**: Verify basic CLI functionality
- **Test cases**:
  - [ ] `iron --help` output format
  - [ ] `iron --version` output
  - [ ] All subcommand `--help` outputs
  - [ ] Invalid argument error messages

#### 4.3 Test Status Command Output
- **File**: `crates/iron-cli/tests/cli_status.rs`
- **Description**: Test status command formatting
- **Test cases**:
  - [ ] Text output format
  - [ ] JSON output format (`--format json`)
  - [ ] Verbose output (`-v`)
  - [ ] Error state display

#### 4.4 Test Doctor Command Output
- **File**: `crates/iron-cli/tests/cli_doctor.rs`
- **Description**: Test diagnostic output
- **Test cases**:
  - [ ] All check categories displayed
  - [ ] JSON report structure
  - [ ] Exit codes for pass/warn/fail
  - [ ] Verbose diagnostic details

#### 4.5 Test Error Message Formatting
- **File**: `crates/iron-cli/tests/cli_errors.rs`
- **Description**: Verify error presentation
- **Test cases**:
  - [ ] Configuration errors
  - [ ] Missing file errors
  - [ ] Permission errors (simulated)
  - [ ] Network errors (simulated)

### Validation Checkpoint
```bash
cargo test -p iron-cli
cargo tarpaulin -p iron-cli | grep "coverage"
# Expected: 70-75% CLI coverage
```

---

## Phase 5: CI/CD Integration

**Impact**: INFRASTRUCTURE
**Effort**: 0.5 days
**Dependencies**: Phases 1-4

### Objective
Integrate coverage tracking into continuous integration pipeline.

### Tasks

#### 5.1 Add Coverage to GitHub Actions
- **File**: `.github/workflows/ci.yml`
- **Description**: Run coverage on every PR
- **Configuration**:
  - [ ] Run tarpaulin on PR
  - [ ] Upload coverage report as artifact
  - [ ] Comment coverage delta on PR
  - [ ] Fail if coverage drops below threshold

#### 5.2 Add Coverage Badge
- **File**: `README.md`
- **Description**: Display current coverage status
- **Badge**: Coverage percentage from CI

#### 5.3 Create Coverage Threshold
- **File**: `tarpaulin.toml` or CI config
- **Description**: Enforce minimum coverage
- **Threshold**: 70% minimum, 75% target

### Validation
```bash
# Verify CI workflow
act pull_request
```

---

## Execution Timeline

```
Week 1:
├── Day 1-2: Phase 1.1-1.3 (MockCommandExecutor + Pacman/Git mocks)
├── Day 3: Phase 1.4-1.5 (Systemd/Snapshot mocks)
├── Day 4-5: Phase 1.6 + Phase 2.1-2.2 (Integration + Event mocks)

Week 2:
├── Day 1-2: Phase 2.3-2.4 + Phase 3.1-3.2 (Terminal tests + Service mocks)
├── Day 3-4: Phase 3.3-3.6 (Service test enhancement)
├── Day 5: Phase 4.1-4.5 (CLI integration tests)

Week 3:
├── Day 1: Phase 5 (CI/CD integration)
├── Day 2: Final validation and documentation
```

---

## Success Metrics

| Phase | Coverage Before | Coverage After | Tests Added |
|-------|-----------------|----------------|-------------|
| 1 | 58.78% | 66-70% | ~150 |
| 2 | 66-70% | 69-73% | ~50 |
| 3 | 69-73% | 74-78% | ~150 |
| 4 | 74-78% | 76-80% | ~50 |
| **Total** | **58.78%** | **75-80%** | **~400** |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Mock complexity | Medium | Medium | Start with simple cases, iterate |
| Test flakiness | Low | High | Use deterministic mocks, avoid timing |
| Diminishing returns | High | Low | Accept 75% as success threshold |
| Platform differences | Medium | Medium | Test on Linux CI, document limitations |

---

## Post-Workflow Actions

After completing this workflow:

1. Run `/sc:test --coverage` to verify final metrics
2. Run `/sc:git` to commit all changes
3. Update `PROJECT_INDEX.md` with new test counts
4. Create PR with coverage improvement summary

---

*Generated by /sc:workflow - Implementation planning only, no code execution*
