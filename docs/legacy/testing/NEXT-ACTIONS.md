# Iron Testing - Next Actions to 80% Coverage

**Current Status (2026-02-13):**
- Tests: 1342 (+351 from last update)
- Coverage: 56.75% (4191/7385 lines)
- Progress: 71% toward 80% target

### Recently Completed
- ✅ Phase 1.5: Snapshot tool mocks (timeshift/snapper) - `snapshot_fixtures.rs`
- ✅ Phase 1.6: Integrated mocks into existing tests - parsing helpers + 18 tests

---

## ✅ Completed Phases

### Phase 8: Comprehensive Testing Infrastructure
- [x] 8.1 TUI TestBackend Rendering (+50 tests, 52.1% TUI coverage)
- [x] 8.2 Service Layer Mocking (+55 tests with MockFileSystem)
- [x] 8.3 Pacman Parser Tests (+49 tests, 55% pacman coverage)
- [x] 8.4 Concurrent Access Tests (+12 tests)
- [x] 8.5 Property-Based Tests (+19 tests with proptest)
- [x] 8.6 TUI Keyboard Tests (+33 tests)
- [x] 8.7 Resilience Tests (+18 tests)
- [x] 8.8 CLI Argument Parsing (+35 tests)
- [x] 8.9 Service Layer Tests (+43 tests)
- [x] 8.10 Wizard UI Tests (+16 tests)

### Phase 9.1: Low-Hanging Fruit
- [x] 9.1.A: TUI Actions tests (actions.rs coverage improved)
- [x] 9.1.B: TUI Event Loop tests (event.rs coverage improved)

---

## 🎯 Priority Actions (Path from 56.75% → 80%)

### **Priority 1: Add Doctests for Public APIs** (Task #29)

**Status:** IN PROGRESS
**Effort:** ~20 doctests, 2 hours
**Impact:** +3% overall coverage
**Files to Update:**

```rust
// crates/iron-pacman/src/lib.rs
/// Parse pacman update output into package updates.
///
/// # Examples
///
/// ```
/// use iron_pacman::parse_updates_output;
///
/// let output = "hyprland 0.40.0-1 -> 0.41.0-1";
/// let updates = parse_updates_output(output, false);
/// assert_eq!(updates[0].name, "hyprland");
/// assert_eq!(updates[0].old_version, "0.40.0-1");
/// assert_eq!(updates[0].new_version, "0.41.0-1");
/// ```
pub fn parse_updates_output(output: &str, is_aur: bool) -> Vec<PackageUpdate> { ... }
```

**Target Functions:**
- `iron-pacman`: parse_updates_output, parse_package_list, parse_search_output, parse_size
- `iron-git`: parse_git_status, parse_encrypted_files
- `iron-systemd`: parse_service_state, parse_enabled_state, parse_list_units
- `iron-core`: StateManager methods, BundleService, ModuleService

**Command:** `cargo test --doc`

---

### **Priority 2: CLI Output Format Validation** (Task #32)

**Status:** PENDING
**Effort:** ~25 tests, 4 hours
**Impact:** +5% overall coverage
**Location:** `crates/iron-cli/tests/cli_output.rs` (new file)

**Test Categories:**

```rust
// 1. JSON Output Validation
#[test]
fn test_status_json_structure() {
    let output = run_cli(&["status", "--format", "json"]);
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(json["health"].is_object());
    assert!(json["packages"].is_object());
    assert!(json["active_bundle"].is_string());
    assert!(json["active_profile"].is_string());
}

// 2. Table Format Validation
#[test]
fn test_bundle_list_table_output() {
    let output = run_cli(&["bundle", "list"]);

    assert!(output.contains("NAME"));
    assert!(output.contains("STATUS"));
    assert!(output.contains("PACKAGES"));
    assert!(output.contains("hyprland"));
}

// 3. Verbose Output Validation
#[test]
fn test_status_verbose_includes_details() {
    let output = run_cli(&["status", "--verbose"]);

    assert!(output.contains("System Health:"));
    assert!(output.contains("Package Information:"));
    assert!(output.contains("Active Configuration:"));
    assert!(output.contains("Recent Operations:"));
}

// 4. Error Message Validation
#[test]
fn test_invalid_bundle_error_message() {
    let output = run_cli(&["bundle", "switch", "nonexistent"]);

    assert!(output.contains("Error"));
    assert!(output.contains("Bundle 'nonexistent' not found"));
}
```

**Commands to Test:**
- `iron status --format json|table`
- `iron bundle list --all --format json`
- `iron module list --enabled`
- `iron profile show developer --verbose`
- `iron doctor --format json`

---

### **Priority 3: Coverage Gap Hunting** (Task #33)

**Status:** PENDING
**Effort:** ~30 tests, 6 hours
**Impact:** +10-15% overall coverage

**Step 1: Identify Gaps**

```bash
# Generate detailed coverage report with line numbers
cargo tarpaulin --workspace --engine llvm --out Html --output-dir target/coverage

# View uncovered lines per file
xdg-open target/coverage/tarpaulin-report.html

# Focus on files with <70% coverage
cargo tarpaulin --workspace --out Xml | grep -B2 "coverage-rate=\"[0-6]"
```

**Step 2: Targeted Testing Strategy**

Focus on:
1. **Error paths** - Test all error variants in iron-core::error
2. **Branch coverage** - Test all if/else and match arms
3. **Edge cases** - Empty inputs, boundary conditions, unicode
4. **Integration points** - Bundle→Profile→Module activation flow

**Example Gap Analysis:**

```rust
// If coverage shows lines 45-52 uncovered in bundle_service.rs:
// Likely missing: test for bundle conflicts, circular dependencies

#[test]
fn test_bundle_conflict_detection() {
    // Test that switching bundles detects conflicts
}

#[test]
fn test_bundle_with_missing_packages() {
    // Test graceful handling of missing package dependencies
}
```

---

### **Priority 4: Integration Test Expansion** (Optional)

**Effort:** ~15 tests, 3 hours
**Impact:** +2% coverage + confidence boost

```rust
// crates/iron-cli/tests/integration_workflows.rs

#[test]
fn test_full_module_enable_workflow() {
    let env = TestEnv::new();

    // Enable module
    env.run(&["module", "enable", "nvim-ide"]);

    // Verify state updated
    let state: IronState = env.read_state();
    assert!(state.active_modules.contains(&"nvim-ide".to_string()));

    // Verify symlinks created
    assert!(env.config_path().join("nvim/init.lua").exists());

    // Disable module
    env.run(&["module", "disable", "nvim-ide", "--yes"]);

    // Verify cleanup
    assert!(!state.active_modules.contains(&"nvim-ide".to_string()));
    assert!(!env.config_path().join("nvim/init.lua").exists());
}
```

---

## 📊 Coverage Projection

| Task | Tests | Coverage Gain | Cumulative |
|------|-------|---------------|------------|
| **Current** | 991 | - | 56.75% |
| Doctests | +20 | +3% | 59.75% |
| CLI Output Tests | +25 | +5% | 64.75% |
| Coverage Gap Hunting | +30 | +10% | 74.75% |
| Integration Tests | +15 | +2% | 76.75% |
| Final Cleanup | +10 | +3% | **~80%** |
| **Total to 80%** | **+100** | **+23%** | **80%** |

**Realistic Timeline:**
- Week 1: Doctests (2h) + CLI Output (4h) = **64.75%**
- Week 2: Coverage Gap Hunting (6h) = **74.75%**
- Week 3: Integration + Cleanup (6h) = **80%**

---

## 🚀 Execution Commands

### Daily Workflow

```bash
# 1. Start with fresh coverage baseline
cargo clean
cargo tarpaulin --workspace --out Html --output-dir target/coverage

# 2. Work on one priority task
# (Add doctests, CLI tests, or targeted tests)

# 3. Run tests and measure progress
cargo test --workspace
cargo tarpaulin --workspace --out Html --output-dir target/coverage

# 4. Check improvement
echo "Previous: 56.75%, Current: $(cargo tarpaulin --workspace 2>&1 | grep -oP '\d+\.\d+(?=%)' | head -1)%"

# 5. Commit progress
git add .
git commit -m "test: add [task description] (+X tests, +Y% coverage)"
```

### Coverage Analysis Commands

```bash
# Per-crate breakdown
cargo tarpaulin -p iron-core --out Html
cargo tarpaulin -p iron-cli --out Html
cargo tarpaulin -p iron-tui --out Html

# Find specific uncovered lines
cargo tarpaulin --workspace --engine llvm --out Lcov
lcov --list target/coverage/lcov.info | grep -v "100%"

# Focus on low-coverage files
cargo tarpaulin --workspace --out Json | jq '.files[] | select(.coverage < 70) | {name: .name, coverage: .coverage}'
```

---

## ✅ Quality Gate Checklist (v0.1.0 Release)

- [ ] 1000+ tests passing
- [ ] 80%+ overall coverage
- [ ] Zero clippy warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo audit` clean
- [ ] All public APIs have doctests
- [ ] CLI commands produce valid JSON output
- [ ] Bundle switch verified on desktop
- [ ] Recovery workflow tested end-to-end
- [ ] Documentation complete

---

## 🎯 Key Insights

### Why Not More Parsing Tests?

Both `iron-git` (39 tests) and `iron-systemd` (37 tests) **already have comprehensive parsing tests**. The low coverage (29% and 38%) is because:

1. **Parsing functions are well-tested** ✅
2. **DefaultGitManager/DefaultServiceManager require real system calls** ❌
3. **Integration tests need actual git/systemctl** (out of scope for unit coverage)

### Where Coverage Gains Will Come From

1. **Doctests (20 tests)** - Easy wins on public APIs
2. **CLI Output (25 tests)** - Test format validation logic
3. **Error Paths (15 tests)** - Test all error variants
4. **Branch Coverage (15 tests)** - Test all if/else arms
5. **Integration Points (15 tests)** - Test cross-service flows
6. **Edge Cases (10 tests)** - Unicode, empty, boundaries

**Total: 100 tests = +23% coverage = 80% target** 🎯

---

*Last Updated: 2026-02-13*
*Next Review: After Priority 1 (Doctests) complete*
