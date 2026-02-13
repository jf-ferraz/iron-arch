# Iron Project Testing Workflow Framework

> Comprehensive testing strategy for production readiness at v0.1.0

## Overview

This document defines a structured, incremental testing workflow for the Iron configuration management system. Tests are categorized by safety level with rollback strategies for each phase.

**Current State (Updated 2026-02-13):**
- **1180 tests passing** across 7 crates (+620% from baseline)
- **54.23% code coverage** (measured via tarpaulin, +8.97% from 45.26%)
- 14 CLI command groups implemented with **129 CLI tests** (39 unit + 61 integration + 29 output validation)
- 8 TUI views with rendering tests (**210 TUI tests**)
- MockFileSystem trait for isolated service testing
- Public parsing APIs for iron-pacman
- Comprehensive concurrent access tests for state management
- **19 property-based tests** with proptest (packages + state)
- **33 TUI keyboard handler tests**
- **18 resilience tests** for error handling and edge cases
- **39 CLI argument parsing tests** (includes FR-5.10 flags)
- **35 TUI actions tests**
- **20 TUI event tests**
- **59 update service tests** (includes FR-5.10 recovery tests)
- **30 snapshot manager tests**
- **25 IronState tests**
- **43 service layer tests** (bundle, host, module, profile)
- **FR-5.10 Partial Update Recovery** - 26 new recovery tests
- Desktop host: AMD Ryzen 9 7950X, RTX 4080, 64GB RAM
- Active bundle: hyprland
- Dormant bundle: niri

---

## Test Safety Labels

| Label | Meaning | Pre-requisite |
|-------|---------|---------------|
| `[SAFE]` | Read-only, no state changes | None |
| `[DRY-RUN]` | Preview mode, no changes applied | None |
| `[STATE-MOD]` | Modifies Iron state file | Backup `state.json` |
| `[SYS-MOD]` | Modifies system packages/configs | Create system snapshot |
| `[NETWORK]` | Requires network connectivity | Verify connection |

---

## Phase 1: Foundation Verification

**Duration:** Day 1
**Risk Level:** `[SAFE]`

### 1.1 Baseline Tests

```bash
# Run all existing tests
cargo test --workspace

# Verify zero linting warnings
cargo clippy --workspace -- -D warnings

# Build release binary
cargo build --release

# Smoke tests
./target/release/iron --version
./target/release/iron --help
```

### 1.2 Success Criteria

- [x] All 354 tests pass
- [ ] Zero clippy warnings
- [ ] Release binary builds successfully
- [ ] `--version` displays correct version
- [ ] `--help` displays all command groups

---

## Phase 2: Unit Test Deep-Dive

**Duration:** Days 2-3
**Risk Level:** `[SAFE]`

### 2.1 Per-Crate Test Execution

| Crate | Tests | Coverage | Command | Priority |
|-------|-------|----------|---------|----------|
| iron-core | 328 | ~55% | `cargo test -p iron-core` | CRITICAL |
| iron-cli | 96 | ~15%* | `cargo test -p iron-cli` | HIGH |
| iron-tui | 210 | ~65% | `cargo test -p iron-tui` | HIGH |
| iron-git | 53 | 29.3% | `cargo test -p iron-git` | MEDIUM |
| iron-systemd | 37 | 37.6% | `cargo test -p iron-systemd` | MEDIUM |
| iron-pacman | 34 | ~40% | `cargo test -p iron-pacman` | MEDIUM |
| iron-fs | 12 | 46.2% | `cargo test -p iron-fs` | MEDIUM |

*Note: CLI coverage is low due to subprocess spawning limitation in tarpaulin. CLI integration tests (61 tests) + argument parsing tests (35 tests) validate command behavior.

### 2.2 Coverage Measurement

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html --output-dir target/coverage

# View report
xdg-open target/coverage/tarpaulin-report.html
```

### 2.3 Coverage Targets

| Crate | Minimum Coverage |
|-------|------------------|
| iron-core | 85% |
| iron-cli | 80% |
| iron-tui | 70% |
| iron-fs | 80% |
| iron-pacman | 75% |
| Overall | 80% |

---

## Phase 3: CLI Command Test Matrix

**Duration:** Days 4-5
**Risk Level:** Varies by command

### 3.1 Safe Commands `[SAFE]`

Run freely without backup.

#### Status & Information
```bash
iron status
iron status --verbose
iron status --format json
iron doctor
iron doctor --fix  # Reports issues, prompts before fixing
```

#### List Operations
```bash
# Bundles
iron bundle list
iron bundle list --all
iron bundle status hyprland
iron bundle status niri

# Profiles
iron profile list
iron profile show developer
iron profile show minimal

# Modules
iron module list
iron module list --enabled
iron module show nvim-ide
iron module show kitty-dev

# Hosts
iron host list
iron host current
iron host catalog
```

#### Sync & Secrets Status
```bash
iron sync status
iron secrets status
```

#### Recovery (Read-Only)
```bash
iron recover --export > /tmp/iron-state-backup.json
iron recover --script > /tmp/iron-install-script.sh
```

### 3.2 Dry-Run Commands `[DRY-RUN]`

Safe preview mode.

```bash
# Preview update operations
iron update --dry-run
iron update --dry-run --verbose
iron update --dry-run --format json | jq .

# Preview clean operations
iron clean --dry-run
iron clean --symlinks --dry-run
iron clean --orphans --dry-run
```

### 3.3 State-Modifying Commands `[STATE-MOD]`

**Pre-requisite:** Backup state file

```bash
# Backup before testing
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak
```

#### Module Operations
```bash
# Enable module
iron module enable nvim-ide
iron module list --enabled  # Verify

# Disable module
iron module disable nvim-ide --yes
iron module list --enabled  # Verify

# Re-enable
iron module enable nvim-ide
```

#### Profile Operations
```bash
# Switch profiles
iron profile select developer
iron status  # Verify active profile

iron profile select minimal
iron status  # Verify

# Restore
iron profile select developer
```

#### Rollback
```bash
# If anything goes wrong
cp ~/.config/iron/state.json.bak ~/.config/iron/state.json
```

### 3.4 System-Modifying Commands `[SYS-MOD]`

**Pre-requisite:** Create system snapshot

```bash
# Create snapshot before testing
sudo timeshift --create --comments "pre-iron-test-$(date +%Y%m%d-%H%M)"
# OR with snapper
sudo snapper create -d "pre-iron-test"
```

#### Bundle Operations (CAREFUL)
```bash
# Verify current state
iron bundle status hyprland  # Should be active
iron bundle status niri      # Should be dormant

# Switch to niri (CAREFUL - will change compositor)
iron bundle switch niri --yes

# Verify switch
iron bundle status niri      # Should now be active
iron bundle status hyprland  # Should now be dormant

# Switch back to hyprland
iron bundle switch hyprland --yes

# Verify restoration
iron bundle status hyprland  # Should be active again
```

#### Clean Operations (CAREFUL)
```bash
# Remove broken symlinks only
iron clean --symlinks

# Preview orphans before removal
iron clean --orphans --dry-run
iron clean --orphans  # Prompts for confirmation

# Clear package cache
iron clean --cache
```

#### Update Operations
```bash
# Preview first
iron update --dry-run

# Execute (will modify system packages)
iron update --yes
```

---

## Phase 4: TUI Feature Test Matrix

**Duration:** Day 6
**Risk Level:** `[SAFE]` to `[STATE-MOD]`

### 4.1 Launch TUI

```bash
# Standard launch
iron tui

# With specific view
iron tui --view dashboard
iron tui --view bundles
```

### 4.2 View Navigation Tests

| View | Hotkey | Verification Steps |
|------|--------|-------------------|
| Dashboard | `d` | Health status visible, package counts displayed |
| Bundles | `b` | List bundles, hyprland marked active, niri marked dormant |
| Profiles | `p` | List profiles, current selection highlighted |
| Modules | `m` | List all modules, enabled status shown |
| Updates | `u` | Risk assessment visible, package list displayed |
| Settings | `s` | Configuration options accessible |
| Logs | `l` | Recent operations displayed |
| Help | `?` | Keybindings reference shown |

### 4.3 Keyboard Interaction Tests

| Key | Expected Behavior | View Context |
|-----|-------------------|--------------|
| `Tab` | Cycle to next view | Global |
| `Shift+Tab` | Cycle to previous view | Global |
| `j` / `↓` | Move selection down | Lists |
| `k` / `↑` | Move selection up | Lists |
| `Enter` | Select/confirm action | Lists, dialogs |
| `Esc` | Cancel/go back | Dialogs, detail views |
| `?` | Show help overlay | Global |
| `q` | Show quit confirmation | Global |
| `r` | Refresh current view | Global |
| `e` | Toggle module enable | Modules view |
| `a` | Activate bundle | Bundles view |
| `Space` | Toggle selection | Multi-select lists |

### 4.4 Setup Wizard Flow Test

Test first-run experience in isolated environment.

```bash
# Create isolated test directory
mkdir -p /tmp/iron-wizard-test

# Run with custom root (triggers wizard)
iron --root /tmp/iron-wizard-test tui

# Expected flow:
# 1. Welcome screen
# 2. Host detection/selection
# 3. Bundle selection (hyprland/niri)
# 4. Profile selection (developer/minimal)
# 5. Confirmation
# 6. Dashboard with active config

# Cleanup
rm -rf /tmp/iron-wizard-test
```

### 4.5 TUI Responsiveness Tests

```bash
# Test different terminal sizes
# Minimum: 80x24
resize -s 24 80 && iron tui

# Standard: 120x40
resize -s 40 120 && iron tui

# Large: 200x60
resize -s 60 200 && iron tui
```

---

## Phase 5: Desktop-Specific Tests

**Duration:** Days 7-8
**Risk Level:** `[SAFE]` to `[SYS-MOD]`

### 5.1 Host Detection Verification `[SAFE]`

```bash
# Verify host detection
iron host current
# Expected: desktop (Desktop Workstation)

# Verify hardware catalog
iron host catalog
# Expected output should include:
# - CPU: AMD Ryzen 9 7950X
# - GPU: NVIDIA RTX 4080
# - RAM: 65536 MB
# - Monitors: DP-1 (2560x1440@165Hz), HDMI-1 (1920x1080@60Hz)
```

### 5.2 Bundle State Verification `[SAFE]`

```bash
# Check hyprland bundle
iron bundle status hyprland
# Expected: Active, all packages installed

# Check niri bundle
iron bundle status niri
# Expected: Dormant, packages may not be installed
```

### 5.3 Real Package Verification `[SAFE]`

```bash
# Verify hyprland packages are installed
pacman -Q hyprland waybar wofi hyprpaper hypridle hyprlock

# Verify wayland packages
pacman -Q wayland wlroots xdg-desktop-portal-hyprland

# Verify NVIDIA drivers
pacman -Q nvidia nvidia-utils nvidia-settings

# Verify user services
systemctl --user status pipewire pipewire-pulse wireplumber

# Check AUR packages (if applicable)
paru -Q hyprshot 2>/dev/null || yay -Q hyprshot 2>/dev/null || echo "hyprshot not installed"
```

### 5.4 Bundle Switch Test `[SYS-MOD]`

**CRITICAL TEST - Create snapshot first**

```bash
# Pre-test snapshot
sudo timeshift --create --comments "pre-bundle-switch-test"

# Save current session info
echo "Current compositor: $XDG_CURRENT_DESKTOP"
```

#### Switch to Niri

```bash
# Perform switch
iron bundle switch niri --yes

# Post-switch verification
iron bundle status niri      # Should be: Active
iron bundle status hyprland  # Should be: Dormant

# Verify niri packages installed
pacman -Q niri

# Verify niri config linked
ls -la ~/.config/niri/
```

#### Switch Back to Hyprland

```bash
# Perform switch
iron bundle switch hyprland --yes

# Post-switch verification
iron bundle status hyprland  # Should be: Active
iron bundle status niri      # Should be: Dormant

# Verify hyprland config linked
ls -la ~/.config/hypr/
```

### 5.5 Module Symlink Verification `[STATE-MOD]`

```bash
# Test nvim-ide module
iron module enable nvim-ide

# Verify symlinks created
ls -la ~/.config/nvim/
# Should point to iron modules directory

# Test disable
iron module disable nvim-ide --yes

# Verify symlinks removed
ls -la ~/.config/nvim/
# Should be empty or show local config

# Re-enable for normal use
iron module enable nvim-ide
```

### 5.6 Profile Module Cascade `[STATE-MOD]`

```bash
# Developer profile enables multiple modules
iron profile select developer
iron module list --enabled
# Expected: nvim-ide, kitty-dev, and other dev modules

# Minimal profile has fewer modules
iron profile select minimal
iron module list --enabled
# Expected: Only essential modules

# Restore developer profile
iron profile select developer
```

---

## Phase 6: Resilience Tests

**Duration:** Day 9
**Risk Level:** `[STATE-MOD]`

### 6.1 Error Handling Scenarios

#### Corrupted State File
```bash
# Backup first
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak

# Corrupt the file
echo "invalid json content" > ~/.config/iron/state.json

# Test error handling
iron status
# Expected: Clear error message, suggestion to run recovery

# Restore
cp ~/.config/iron/state.json.bak ~/.config/iron/state.json
```

#### Missing Config Directory
```bash
# Backup and remove
mv ~/.config/iron/bundles ~/.config/iron/bundles.bak

# Test error handling
iron bundle list
# Expected: "Bundles directory not found" error

# Restore
mv ~/.config/iron/bundles.bak ~/.config/iron/bundles
```

#### Invalid Bundle ID
```bash
iron bundle install nonexistent-bundle
# Expected: "Bundle 'nonexistent-bundle' not found" error

iron bundle switch nonexistent
# Expected: "Bundle 'nonexistent' not found" error
```

#### Invalid Module ID
```bash
iron module enable fake-module
# Expected: "Module 'fake-module' not found" error

iron module show does-not-exist
# Expected: "Module 'does-not-exist' not found" error
```

#### Permission Denied
```bash
# Test state file permission
chmod 000 ~/.config/iron/state.json
iron status
# Expected: Permission denied error

# Restore
chmod 644 ~/.config/iron/state.json
```

#### Network Offline (for sync operations)
```bash
# Temporarily disable network
nmcli networking off

# Test sync
iron sync status
# Expected: Graceful handling, "Network unavailable" warning

# Restore network
nmcli networking on
```

### 6.2 Concurrent Access Test

```bash
# Terminal 1: Enable module
iron module enable nvim-ide &
PID1=$!

# Terminal 2: Disable different module (simultaneously)
iron module disable kitty-dev &
PID2=$!

# Wait for both
wait $PID1 $PID2

# Verify state integrity
iron status
iron module list --enabled

# State should be consistent (one enabled, one disabled)
```

### 6.3 Recovery Workflow Test

```bash
# Step 1: Export current state
iron recover --export > /tmp/iron-recovery-test.json

# Step 2: Generate install script
iron recover --script > /tmp/iron-recovery-test.sh
chmod +x /tmp/iron-recovery-test.sh

# Step 3: Validate script syntax
bash -n /tmp/iron-recovery-test.sh
echo "Script syntax: $([[ $? -eq 0 ]] && echo 'VALID' || echo 'INVALID')"

# Step 4: Backup current state
cp ~/.config/iron/state.json ~/.config/iron/state-pre-import.json

# Step 5: Modify state (simulate corruption)
iron module disable nvim-ide --yes

# Step 6: Import from backup
iron recover --import /tmp/iron-recovery-test.json

# Step 7: Verify restoration
diff ~/.config/iron/state.json /tmp/iron-recovery-test.json
echo "State restored: $([[ $? -eq 0 ]] && echo 'SUCCESS' || echo 'MISMATCH')"

# Cleanup
rm /tmp/iron-recovery-test.json /tmp/iron-recovery-test.sh
```

### 6.4 Large State Stress Test

```bash
# Generate large state with many modules enabled
for i in $(seq 1 50); do
    iron module enable "test-module-$i" 2>/dev/null || true
done

# Test performance
time iron status
time iron module list

# Cleanup (disable test modules)
for i in $(seq 1 50); do
    iron module disable "test-module-$i" --yes 2>/dev/null || true
done
```

---

## Phase 7: Acceptance Tests

**Duration:** Day 10
**Risk Level:** All levels

### 7.1 End-to-End User Scenarios

#### Scenario 1: New User Setup
```bash
# Create clean environment
rm -rf /tmp/iron-e2e-test
mkdir -p /tmp/iron-e2e-test

# Initialize
iron --root /tmp/iron-e2e-test init

# Setup wizard (if TUI)
iron --root /tmp/iron-e2e-test tui

# Verify
iron --root /tmp/iron-e2e-test status
# Success: Dashboard shows active config

rm -rf /tmp/iron-e2e-test
```

#### Scenario 2: Safe Update Workflow
```bash
# Preview changes
iron update --dry-run

# Review output (should show package changes)
iron update --dry-run --format json | jq '.packages | length'

# Execute update (if preview looks good)
iron update --yes

# Verify
iron status
# Success: No pending updates, all healthy
```

#### Scenario 3: Module Management Cycle
```bash
# Start state
iron module list --enabled > /tmp/initial-modules.txt

# Enable nvim-ide
iron module enable nvim-ide

# Verify symlinks exist
ls ~/.config/nvim/init.lua

# Use neovim briefly
nvim --headless -c 'echo "test"' -c 'qa'

# Disable
iron module disable nvim-ide --yes

# Verify symlinks removed
[[ ! -L ~/.config/nvim/init.lua ]] && echo "Symlink removed: SUCCESS"

# Restore initial state
iron module enable nvim-ide
```

#### Scenario 4: Bundle Switch Cycle
```bash
# Pre-test snapshot
sudo timeshift --create --comments "e2e-bundle-test"

# Record initial state
INITIAL_BUNDLE=$(iron bundle list --format json | jq -r '.active')

# Switch to alternate
iron bundle switch niri --yes
sleep 2

# Verify switch
NEW_BUNDLE=$(iron bundle list --format json | jq -r '.active')
[[ "$NEW_BUNDLE" == "niri" ]] && echo "Switch to niri: SUCCESS"

# Switch back
iron bundle switch hyprland --yes
sleep 2

# Verify restoration
FINAL_BUNDLE=$(iron bundle list --format json | jq -r '.active')
[[ "$FINAL_BUNDLE" == "$INITIAL_BUNDLE" ]] && echo "Restore: SUCCESS"
```

#### Scenario 5: Disaster Recovery
```bash
# Export state
iron recover --export > /tmp/disaster-test.json

# Simulate disaster (corrupt state)
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak
echo "corrupted" > ~/.config/iron/state.json

# Verify iron detects corruption
iron status 2>&1 | grep -i error && echo "Corruption detected: SUCCESS"

# Recover
cp ~/.config/iron/state.json.bak ~/.config/iron/state.json
# OR
iron recover --import /tmp/disaster-test.json

# Verify recovery
iron status && echo "Recovery: SUCCESS"

rm /tmp/disaster-test.json
```

### 7.2 Production Readiness Checklist

#### Code Quality
- [ ] All 1072+ tests passing: `cargo test --workspace`
- [ ] Zero clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Formatted correctly: `cargo fmt --check`
- [ ] No unsafe code (or justified): `grep -r "unsafe" crates/`

#### CLI Completeness
- [ ] `iron status` - Shows system health
- [ ] `iron doctor` - Diagnoses issues
- [ ] `iron bundle list/status/switch` - Bundle management
- [ ] `iron profile list/show/select` - Profile management
- [ ] `iron module list/show/enable/disable` - Module management
- [ ] `iron host list/current/catalog` - Host information
- [ ] `iron update --dry-run/--yes` - Package updates
- [ ] `iron clean --symlinks/--orphans/--cache` - Cleanup
- [ ] `iron sync status` - Git sync status
- [ ] `iron secrets status` - Secrets management
- [ ] `iron recover --export/--import/--script` - Recovery

#### TUI Completeness
- [ ] Dashboard view renders correctly
- [ ] All 8 views accessible via hotkeys
- [ ] Keyboard navigation works (j/k, arrows, Tab)
- [ ] Action confirmations work (Enter, Esc)
- [ ] Help overlay displays (?)
- [ ] Quit confirmation works (q)
- [ ] Setup wizard completes successfully

#### Desktop Integration
- [ ] Host detection correct (AMD Ryzen 9 7950X)
- [ ] Hyprland bundle active and functional
- [ ] Niri bundle switches cleanly
- [ ] Module symlinks create/remove correctly
- [ ] Profile selection cascades to modules
- [ ] Hardware catalog accurate

#### Resilience
- [ ] Corrupted state handled gracefully
- [ ] Missing directories reported clearly
- [ ] Invalid IDs return helpful errors
- [ ] Permission errors caught
- [ ] Network offline handled
- [ ] Concurrent access safe
- [ ] Recovery workflow functional

#### Documentation
- [ ] README.md complete
- [ ] CLI help text accurate (`--help` on all commands)
- [ ] User guide covers all features
- [ ] Architecture docs up-to-date

---

## Rollback Strategies

### State Rollback (Quick)
```bash
# Restore from backup
cp ~/.config/iron/state.json.bak ~/.config/iron/state.json

# Or import from export
iron recover --import /path/to/backup.json
```

### Bundle Rollback
```bash
# Switch back to previous bundle
iron bundle switch <previous-bundle> --yes

# e.g., after failed niri switch
iron bundle switch hyprland --yes
```

### System Rollback (Nuclear)
```bash
# Using timeshift
sudo timeshift --restore

# Or specific snapshot
sudo timeshift --list
sudo timeshift --restore --snapshot '2024-XX-XX_XX-XX-XX'

# Using snapper
sudo snapper list
sudo snapper rollback <number>
```

### Package Rollback
```bash
# Downgrade specific package
sudo downgrade <package-name>

# View pacman cache
ls /var/cache/pacman/pkg/ | grep <package>

# Install specific version
sudo pacman -U /var/cache/pacman/pkg/<package-version>.pkg.tar.zst
```

---

## Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Unit tests | **672** | 700+ | 🟢 |
| Test coverage | **52.11%** | 80%+ | 🟡 |
| CLI commands tested | 14/14 | 14/14 | 🟢 |
| TUI views tested | 8/8 | 8/8 | 🟢 |
| Error scenarios tested | 18 | 10+ | 🟢 |
| Recovery scenarios tested | 5 | 3+ | 🟢 |
| Bundle switch verified | No | Yes | ⬜ |
| Documentation complete | Partial | Yes | 🟡 |

### Coverage by Crate (Updated 2025-02-13)

| Crate | Lines Covered | Total Lines | Coverage | Status |
|-------|--------------|-------------|----------|--------|
| iron-tui (wizard.rs) | 246 | 246 | **100%** | 🟢 |
| iron-tui (ui/) | 495 | 676 | **73%** | 🟢 |
| iron-tui (handlers) | 73 | 133 | **55%** | 🟡 |
| iron-core | ~1200 | 2300 | **52%** | 🟡 |
| iron-fs | 128 | 277 | **46%** | 🟡 |
| iron-pacman | 130 | 333 | **39%** | 🟡 |
| iron-systemd | 50 | 133 | **38%** | 🟡 |
| iron-git | 44 | 150 | **29%** | 🟡 |
| iron-cli | ~150 | ~800 | **~15%*** | 🟡 |

*CLI coverage is low due to subprocess spawning limitation in tarpaulin. Argument parsing and integration tests validate behavior.

**Overall: 52.11% coverage (3848/7385 lines)**

---

## Phase 8: Next Enhancement Implementations

### Priority 1: High-Impact Coverage Gaps

#### 8.1 TUI Rendering Tests with TestBackend [COMPLETED]

**Status:** DONE - 50 new tests added (+50 tests, iron-tui: 63 → 113)
**Coverage Increase:** +24.7% (iron-tui: 27.4% → 52.1%)

The `ui/` directory now has comprehensive TestBackend rendering tests covering all 8 TUI views: Dashboard, Bundles, BundleDetail, Modules, ModuleDetail, Profiles, ProfileDetail, Settings, Update Preview, and Sync.

**Implementation Pattern:**

```rust
// crates/iron-tui/src/ui/tests.rs

use ratatui::{backend::TestBackend, Terminal};
use super::*;

fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).unwrap()
}

#[test]
fn test_dashboard_renders_health_status() {
    let mut terminal = create_test_terminal(80, 24);
    let app = App::default();

    terminal.draw(|f| {
        render_dashboard(f, f.area(), &app);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    // Assert expected content rendered
    assert!(buffer.content().iter().any(|c| c.symbol() == "●"));
}

#[test]
fn test_bundles_view_shows_active_indicator() {
    let mut terminal = create_test_terminal(80, 24);
    let mut app = App::default();
    app.bundles = vec![
        BundleInfo { id: "hyprland".into(), active: true, .. },
        BundleInfo { id: "niri".into(), active: false, .. },
    ];

    terminal.draw(|f| {
        render_bundles(f, f.area(), &app);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    // Verify "Active" indicator appears
    let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Active"));
}
```

**Files Created:**
- `crates/iron-tui/src/ui/tests.rs` - 50 comprehensive rendering tests
- `crates/iron-tui/src/ui/mod.rs` - added `#[cfg(test)] mod tests;`

**Tests Added:** 50 tests (exceeds 30-40 estimate)

#### 8.2 iron-core Service Layer Mocking [COMPLETED]

**Status:** DONE - MockFileSystem trait and test helpers implemented (+31 tests)
**Coverage Increase:** +5.2% (iron-core: 46.8% → ~52%)

Implemented comprehensive filesystem abstraction with `FileSystem` trait for isolated testing of services that interact with the filesystem.

**Files Created:**
- `crates/iron-core/src/fs_trait.rs` - FileSystem trait with 14 methods, RealFileSystem, MockFileSystem
- `crates/iron-core/src/test_helpers.rs` - Test builders (TestBundle, TestModule, TestProfile), MockFsBuilder, preset configurations

**Key Features:**
- Thread-safe MockFileSystem using `Arc<RwLock<HashMap>>`
- Error simulation with `set_error()` / `clear_error()`
- Symlink support with automatic link following
- Builder patterns for creating test fixtures
- Preset configurations: `hyprland_bundle()`, `niri_bundle()`, `nvim_ide_module()`, etc.
- `complete_test_env()` for full mock environment setup

**Implementation Highlights:**

```rust
// crates/iron-core/src/fs_trait.rs
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> FsResult<String>;
    fn write(&self, path: &Path, contents: &str) -> FsResult<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn is_symlink(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> FsResult<()>;
    fn symlink(&self, src: &Path, dst: &Path) -> FsResult<()>;
    // ... 14 total methods
}

// crates/iron-core/src/test_helpers.rs
let fs = MockFsBuilder::new("/iron")
    .add_bundle(hyprland_bundle())
    .add_module(nvim_ide_module())
    .add_profile(developer_profile())
    .build();
```

**Tests Added:** 31 new tests (28 fs_trait + 14 test_helpers = 42, net +31 from baseline)

#### 8.3 iron-pacman Parser Tests [COMPLETED]

**Status:** DONE - 49 new tests added (44 unit tests + 5 doc tests)
**Coverage Increase:** +33.6% (iron-pacman: 21.4% → ~55%)

Comprehensive parser tests for all pacman output parsing logic with public APIs.

**Files Modified:**
- `crates/iron-pacman/src/lib.rs` - Added public parsing functions with doc tests

**Public APIs Added:**
- `parse_updates_output()` - Parse `pacman -Qu` / `checkupdates` output
- `parse_package_list()` - Parse `pacman -Q` output
- `parse_search_output()` - Parse `pacman -Ss` output
- `parse_package_info()` - Parse `pacman -Qi` output
- `parse_size()` - Parse size strings (B, KiB, MiB, GiB)
- `SearchResult` struct for search results

**Test Categories:**
- Update output parsing (10 tests)
- Package list parsing (5 tests)
- Search output parsing (5 tests)
- Package info parsing (4 tests)
- Size parsing (6 tests)
- RSS news parsing (7 tests)
- AUR helper tests (5 tests)
- Edge cases and error conditions (7 tests)

**Example:**
```rust
use iron_pacman::{parse_updates_output, parse_search_output};

let output = "hyprland 0.40.0-1 -> 0.41.0-1";
let updates = parse_updates_output(output, false);
assert_eq!(updates[0].name, "hyprland");

let search = "extra/hyprland 0.40.0-1
    A tiling Wayland compositor";
let results = parse_search_output(search);
assert!(results[0].description.contains("Wayland"));
```

**Tests Added:** 49 new tests (exceeds 15-25 estimate)

---

### Priority 2: Resilience & Edge Cases

#### 8.4 Concurrent Access Tests [COMPLETED]

**Status:** DONE - 12 new concurrent tests added (+12 tests, iron-core: 119 → 131)
**Coverage Increase:** +2% (iron-core: ~52% → ~54%)

**Location:** `crates/iron-core/src/services/state.rs`

The state management module now has comprehensive concurrent access tests covering:

- **test_concurrent_reads_no_blocking** - Verifies 10 concurrent readers complete quickly
- **test_concurrent_writes_no_data_loss** - 5 threads × 20 modules = 100 modules all persisted
- **test_concurrent_enable_disable_same_module** - Race condition on same module handled safely
- **test_stress_test_many_threads** - 20 threads with mixed operations
- **test_file_locking_prevents_corruption** - All locked operations succeed atomically
- **test_sequential_transaction_commit_and_rollback** - Transaction commit/rollback semantics
- **test_mixed_read_write_operations** - 5 readers + 5 writers run concurrently
- **test_concurrent_host_and_bundle_operations** - State consistency with multiple operation types
- **test_audit_log_concurrent_access** - Audit entries preserved under concurrent load
- **test_state_reload_consistency** - Multiple managers see each other's changes
- **test_transaction_auto_rollback_on_drop** - RAII pattern for transaction cleanup
- **test_concurrent_transactions_both_commit** - Multiple concurrent commits succeed

**Key findings:**
- File locking via `fs2::FileExt` prevents write corruption
- `with_locked_state()` provides atomic read-modify-write operations
- Transaction rollback restores full snapshot (not partial) - designed for single-user sessions
- State persisted to disk is consistent even under high concurrency

#### 8.5 Property-Based Testing with Proptest

**Add to Cargo.toml:**
```toml
[dev-dependencies]
proptest = "1.4"
```

**Example:**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_git_status_never_panics(s in ".*") {
        let _ = parse_git_status(&s);
    }

    #[test]
    fn parse_service_state_roundtrip(state in prop_oneof![
        Just(ServiceState::Running),
        Just(ServiceState::Stopped),
        Just(ServiceState::Failed),
    ]) {
        let output = format!("ActiveState={:?}", state);
        let parsed = parse_service_state(&output);
        // Verify invariants
    }
}
```

---

### Priority 3: Integration & E2E Tests

#### 8.6 CLI Integration Test Expansion

**Location:** `crates/iron-cli/tests/cli_integration.rs`

Add tests for remaining command groups:
- `iron doctor` with various health states
- `iron update --dry-run` output validation
- `iron clean` operations
- `iron sync` with mock git repository
- `iron secrets` operations

**Example:**
```rust
#[test]
fn test_doctor_reports_missing_packages() {
    let dir = TempDir::new().unwrap();
    // Setup bundle with missing package
    // Run doctor command
    // Assert warning reported
}

#[test]
fn test_update_dry_run_shows_package_changes() {
    let dir = TempDir::new().unwrap();
    // Setup state with outdated packages
    // Run update --dry-run
    // Assert output shows pending updates
}
```

#### 8.7 TUI E2E Tests with Playwright

**Setup:** Install `playwright-rust` or use terminal automation.

**Example:**
```rust
// tests/tui_e2e.rs

#[tokio::test]
async fn test_tui_navigation_flow() {
    let mut child = Command::new("./target/release/iron")
        .arg("tui")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start TUI");

    // Send keystrokes
    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(b"d").unwrap(); // Dashboard
    stdin.write_all(b"b").unwrap(); // Bundles
    stdin.write_all(b"q").unwrap(); // Quit
    stdin.write_all(b"y").unwrap(); // Confirm

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
}
```

---

### Priority 4: Documentation & Quality Gates

#### 8.8 Documentation Tests (doctests)

Add `///` documentation with examples that compile:

```rust
/// Parse git status output into structured data.
///
/// # Examples
///
/// ```
/// use iron_git::parse_git_status;
///
/// let output = "## main\n M src/lib.rs\n?? newfile.txt";
/// let status = parse_git_status(output);
/// assert_eq!(status.branch, Some("main".to_string()));
/// assert_eq!(status.modified.len(), 1);
/// assert_eq!(status.untracked.len(), 1);
/// ```
pub fn parse_git_status(output: &str) -> GitStatus { ... }
```

#### 8.9 CI/CD Quality Gates

**GitHub Actions Workflow:**
```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: cargo test --workspace

      - name: Check formatting
        run: cargo fmt --check

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

      - name: Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --workspace --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: cobertura.xml
          fail_ci_if_error: true
          threshold: 70%
```

---

### Implementation Roadmap

| Phase | Enhancement | Estimated Tests | Target Coverage |
|-------|-------------|-----------------|-----------------|
| Week | Task | Est. Tests | Act. Tests | Coverage |
|------|------|------------|------------|----------|
| Week 1 | TUI TestBackend rendering | +35 | **+50** ✅ | 52.1% |
| Week 1 | iron-pacman parsers | +20 | **+49** ✅ | ~55% |
| Week 2 | iron-core service mocks | +50 | **+55** ✅ | ~52% |
| Week 2 | Concurrent access tests | +10 | **+12** ✅ | ~54% |
| Week 2 | Property-based testing | +15 | **+19** ✅ | ~54% |
| Week 2 | TUI keyboard handlers | +25 | **+33** ✅ | ~54% |
| Week 2 | Resilience tests | +10 | **+18** ✅ | ~54% |
| Week 3 | Wizard UI rendering | +15 | **+16** ✅ | ~52% |
| Week 3 | Service layer tests | +40 | **+43** ✅ | ~52% |
| Week 3 | CLI argument parsing | +30 | **+35** ✅ | ~52% |
| Week 4 | CLI integration expansion | +20 | - | - |
| Week 4 | Doctests | +20 | - | - |
| Week 5 | TUI E2E automation | +15 | - | - |
| Week 5 | Coverage gap hunting | +25 | - | - |

**Progress:** +330 tests completed (Phases 8.1-8.8) → **672 total tests** at **52.11% coverage**
**Remaining:** +80 tests estimated → **750+ total tests** at **~65% coverage**

---

## Phase 9: Next Enhancement Recommendations

Based on the current coverage analysis, here are the prioritized next steps to reach 80% coverage:

### 9.1 Low-Hanging Fruit (High Impact, Low Effort)

#### A. TUI Actions Module Tests (0% → 60%)
**Location:** `crates/iron-tui/src/app/actions.rs` (6/153 lines = 3.92%)
**Effort:** ~20 tests, 2 hours
**Impact:** +10% TUI coverage

```rust
// Test pattern for actions.rs
#[test]
fn test_action_enable_module_updates_state() {
    let mut app = App::default();
    app.handle_action(Action::EnableModule("nvim-ide".to_string()));
    assert!(app.modules.iter().any(|m| m.id == "nvim-ide" && m.enabled));
}
```

**Key actions to test:**
- `EnableModule` / `DisableModule`
- `SelectBundle` / `SwitchBundle`
- `SelectProfile`
- `RefreshData`
- `RunUpdate`

#### B. TUI Event Loop Tests (0% → 50%)
**Location:** `crates/iron-tui/src/event.rs` (0/21 lines = 0%)
**Effort:** ~10 tests, 1 hour
**Impact:** +5% TUI coverage

Test event handling without blocking:
```rust
#[test]
fn test_event_poll_timeout() {
    let events = EventHandler::new(std::time::Duration::from_millis(100));
    let start = std::time::Instant::now();
    let _ = events.next_event(); // Should return after timeout
    assert!(start.elapsed() >= Duration::from_millis(100));
}
```

### 9.2 Medium Effort (Significant Coverage Gains)

#### C. iron-git Parsing Tests (29% → 60%)
**Location:** `crates/iron-git/src/lib.rs` (44/150 lines)
**Effort:** ~15 tests, 2 hours
**Impact:** +31% iron-git coverage

Test all git output parsing:
- `parse_status_output()` - branch, staged, modified, untracked
- `parse_log_output()` - commit history
- `parse_diff_output()` - file changes

#### D. iron-systemd Service Tests (38% → 70%)
**Location:** `crates/iron-systemd/src/lib.rs` (50/133 lines)
**Effort:** ~15 tests, 2 hours
**Impact:** +32% iron-systemd coverage

Test systemd output parsing:
- `parse_service_status()` - active, inactive, failed states
- `parse_unit_list()` - list-units output
- `parse_journal_output()` - journalctl parsing

### 9.3 Integration Testing Phase

#### E. CLI Command Output Tests
**Location:** `crates/iron-cli/tests/cli_integration.rs`
**Effort:** ~25 tests, 4 hours
**Impact:** +20% CLI integration coverage

Test actual command outputs:
```rust
#[test]
fn test_status_command_json_output() {
    let dir = setup_test_env();
    let output = Command::new("iron")
        .args(["--root", dir.path().to_str().unwrap(), "status", "--format", "json"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["health"].is_object());
}
```

#### F. TUI E2E Flow Tests (Optional)
**Location:** `crates/iron-tui/tests/tui_e2e.rs`
**Effort:** ~10 tests, 3 hours
**Impact:** Complete TUI flow validation

### 9.4 Prioritized Action Plan

| Priority | Task | Est. Tests | Est. Coverage | Hours |
|----------|------|------------|---------------|-------|
| 1 | TUI Actions tests | +20 | +10% (→62%) | 2h |
| 2 | iron-git parsing | +15 | +5% (→67%) | 2h |
| 3 | iron-systemd parsing | +15 | +3% (→70%) | 2h |
| 4 | TUI Event tests | +10 | +2% (→72%) | 1h |
| 5 | CLI output tests | +25 | +5% (→77%) | 4h |
| 6 | Doctests (public APIs) | +20 | +3% (→80%) | 2h |

**Total to 80%:** ~105 additional tests, ~13 hours of effort

### 9.5 Quality Gates Before v0.1.0 Release

- [ ] All 750+ tests pass: `cargo test --workspace`
- [ ] 80%+ coverage: `cargo tarpaulin --workspace`
- [ ] Zero clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Formatted: `cargo fmt --check`
- [ ] No security issues: `cargo audit`
- [ ] Documentation complete with doctests
- [ ] Bundle switch cycle verified on real desktop
- [ ] Recovery workflow tested end-to-end
- [ ] **Acceptance tests pass for US-1 through US-6** (NEW)
- [ ] **E2E bundle switch cycle validated** (BLOCKING)

---

## Phase 10: Acceptance Test Suite (Expert Panel Recommendation)

### 10.1 Overview

Based on the Expert Panel review (Wiegers, Crispin), user stories US-1 through US-6 require executable acceptance tests. This phase creates a formal acceptance test suite using Gherkin-style scenarios.

**Location:** `tests/acceptance/`
**Framework:** Custom Rust test harness with Gherkin-style assertions

### 10.2 Acceptance Test Specifications

#### AT-1: First-Time Setup (US-1)

```rust
// tests/acceptance/first_time_setup.rs

#[test]
fn at_1_first_time_setup_wizard() {
    // GIVEN I have a fresh Iron installation (empty root directory)
    let test_root = TempDir::new().unwrap();

    // WHEN I run `iron init`
    let output = Command::new("iron")
        .args(["--root", test_root.path().to_str().unwrap(), "init"])
        .output()
        .unwrap();

    // THEN I see a welcome message
    assert!(String::from_utf8_lossy(&output.stdout).contains("Welcome"));

    // AND host.toml is created with detected hardware
    assert!(test_root.path().join("hosts").exists());

    // AND state.json is initialized
    assert!(test_root.path().join("state.json").exists());
}

#[test]
fn at_1_hardware_detection() {
    // GIVEN I run init on a machine with known hardware
    // WHEN I run `iron host catalog`
    // THEN CPU, GPU, RAM are detected and stored
}

#[test]
fn at_1_bundle_selection() {
    // GIVEN I have initialized Iron
    // WHEN I select a bundle (hyprland)
    // THEN the bundle is set as active in state.json
}

#[test]
fn at_1_profile_selection() {
    // GIVEN I have an active bundle
    // WHEN I select a profile (developer)
    // THEN the profile is set as active
    // AND modules from the profile are enabled
}

#[test]
fn at_1_completion_under_10_minutes() {
    // GIVEN a fresh installation
    // WHEN I complete the full setup flow
    // THEN total elapsed time < 10 minutes
}
```

#### AT-2: Safe Updates (US-2)

```rust
// tests/acceptance/safe_updates.rs

#[test]
fn at_2_risk_score_display() {
    // GIVEN there are system updates available
    // WHEN I run `iron update --dry-run`
    // THEN I see a risk score (LOW/MEDIUM/HIGH/CRITICAL)
}

#[test]
fn at_2_risk_thresholds() {
    // GIVEN updates with 2 minor package changes
    // WHEN risk is calculated
    // THEN score is LOW

    // GIVEN updates with 5 package changes including config updates
    // WHEN risk is calculated
    // THEN score is MEDIUM

    // GIVEN updates with kernel/bootloader changes
    // WHEN risk is calculated
    // THEN score is CRITICAL
}

#[test]
fn at_2_approval_required_for_medium_risk() {
    // GIVEN MEDIUM risk updates
    // WHEN I run `iron update`
    // THEN explicit confirmation is required before proceeding
}

#[test]
fn at_2_auto_snapshot_before_update() {
    // GIVEN I approve an update
    // WHEN the update executes
    // THEN a snapshot is created BEFORE any packages are installed
}

#[test]
fn at_2_arch_news_display() {
    // GIVEN there are relevant Arch News items
    // WHEN I run `iron update --dry-run`
    // THEN news items are displayed in the preview
}
```

#### AT-3: Multi-Machine Sync (US-3)

```rust
// tests/acceptance/multi_machine_sync.rs

#[test]
fn at_3_sync_push() {
    // GIVEN I have local configuration changes
    // WHEN I run `iron sync push`
    // THEN changes are committed and pushed to remote
}

#[test]
fn at_3_sync_pull() {
    // GIVEN there are remote configuration changes
    // WHEN I run `iron sync pull`
    // THEN changes are fetched and applied locally
}

#[test]
fn at_3_host_specific_preservation() {
    // GIVEN I have host-specific settings on laptop
    // WHEN I sync from desktop
    // THEN laptop-specific settings are preserved
    // AND shared settings are updated
}
```

#### AT-4: Disaster Recovery (US-4)

```rust
// tests/acceptance/disaster_recovery.rs

#[test]
fn at_4_state_export() {
    // GIVEN I have a configured Iron installation
    // WHEN I run `iron recover --export`
    // THEN a complete state backup is created
}

#[test]
fn at_4_install_script_generation() {
    // GIVEN I have a host configuration
    // WHEN I run `iron recover --script`
    // THEN a valid bash install script is generated
}

#[test]
fn at_4_recovery_flow_steps() {
    // GIVEN I have exported state and a fresh Arch installation
    // WHEN I run `iron recover --import <backup.json>`
    // THEN the 4-step recovery flow executes:
    //   1. Core system installation
    //   2. Bundle installation
    //   3. Profile selection
    //   4. Post-install verification
}

#[test]
fn at_4_recovery_under_30_minutes() {
    // GIVEN a backed-up Iron configuration
    // WHEN I complete full recovery
    // THEN total elapsed time < 30 minutes
}
```

#### AT-5: Environment Switch (US-5)

```rust
// tests/acceptance/environment_switch.rs

#[test]
fn at_5_bundle_switch_creates_snapshot() {
    // GIVEN I have Niri as active bundle
    // WHEN I run `iron bundle switch hyprland`
    // THEN a snapshot is created BEFORE the switch
}

#[test]
fn at_5_dormant_storage() {
    // GIVEN I switch from Niri to Hyprland
    // WHEN the switch completes
    // THEN Niri configs are stored in dormant/
}

#[test]
fn at_5_config_linking() {
    // GIVEN I switch to Hyprland
    // WHEN the switch completes
    // THEN Hyprland configs are linked to ~/.config/
}

#[test]
fn at_5_switch_back() {
    // GIVEN I switched from Niri to Hyprland
    // WHEN I run `iron bundle switch niri`
    // THEN Niri configs are restored from dormant/
    // AND Hyprland configs are moved to dormant/
}

#[test]
fn at_5_e2e_bundle_switch_cycle() {
    // FULL E2E TEST - BLOCKING FOR v0.1.0 RELEASE
    // GIVEN I have Hyprland as active bundle
    // WHEN I:
    //   1. Create snapshot
    //   2. Switch to Niri
    //   3. Verify Niri is active
    //   4. Switch back to Hyprland
    //   5. Verify Hyprland is active
    // THEN all steps succeed
    // AND symlinks are correct at each stage
    // AND state.json reflects correct state
}
```

#### AT-5.10: Partial Update Recovery (FR-5.10) [COMPLETED]

```rust
// tests/acceptance/partial_update_recovery.rs

#[test]
fn at_5_10_1_detect_interrupted_update() {
    // GIVEN an update was started with 10 packages
    // AND 5 packages were completed before interruption
    // AND the phase was set to "Interrupted"
    // WHEN I run "iron update"
    // THEN I see "Previous update was interrupted"
    // AND I see "5/10 packages completed (50.0%)"
    // AND I am prompted to Resume, Clear, or Abort
}

#[test]
fn at_5_10_2_resume_interrupted_update() {
    // GIVEN an interrupted update with 5/10 packages completed
    // WHEN I run "iron update --resume"
    // THEN only 5 remaining packages are installed
    // AND progress state is cleared after success
    // AND the update completes successfully
}

#[test]
fn at_5_10_3_clear_stale_progress() {
    // GIVEN stale progress state from a previous session
    // WHEN I run "iron update --clear-progress"
    // THEN progress state is removed
    // AND "iron update" starts fresh
}

#[test]
fn at_5_10_4_progress_survives_crash() {
    // GIVEN an update is in progress
    // WHEN the process is killed (SIGKILL)
    // AND I restart "iron update"
    // THEN I see the interrupted update prompt
    // AND progress reflects last persisted state
}

#[test]
fn at_5_10_5_status_flag_shows_progress() {
    // GIVEN an update is in progress or was interrupted
    // WHEN I run "iron update --status"
    // THEN I see session ID, started time, phase
    // AND I see completed/remaining package counts
    // AND I see completion percentage
}

#[test]
fn at_5_10_6_resume_flag_continues_update() {
    // GIVEN an interrupted update exists
    // WHEN I run "iron update --resume"
    // THEN the update resumes without prompting
    // AND only remaining packages are installed
}

#[test]
fn at_5_10_7_clear_progress_flag_resets_state() {
    // GIVEN progress state exists (interrupted or stale)
    // WHEN I run "iron update --clear-progress"
    // THEN progress is removed from state.json
    // AND success message is displayed
}
```

**Status:** COMPLETED - All 7 FR-5.10 acceptance tests implemented and verified.

---

#### AT-6: Custom Profile Creation (US-6)

```rust
// tests/acceptance/custom_profile.rs

#[test]
fn at_6_profile_create() {
    // GIVEN I am creating a new profile
    // WHEN I run `iron profile create my-profile`
    // THEN a new profile.toml is created
}

#[test]
fn at_6_module_selection() {
    // GIVEN I have a new profile
    // WHEN I add modules to it
    // THEN modules are recorded in profile.toml
}

#[test]
fn at_6_profile_activation() {
    // GIVEN I have created a custom profile
    // WHEN I run `iron profile select my-profile`
    // THEN the profile becomes active
    // AND modules from the profile are enabled
}
```

### 10.3 E2E Bundle Switch Gate

**This test is BLOCKING for v0.1.0 release:**

```rust
// tests/acceptance/e2e_bundle_switch.rs

/// BLOCKING: This test must pass before v0.1.0 release
/// Validates the complete bundle switch cycle on a real desktop environment
#[test]
#[ignore] // Run manually with: cargo test --ignored
fn e2e_bundle_switch_full_cycle() {
    // Pre-requisites:
    // - Desktop host with hyprland active
    // - niri bundle available
    // - Timeshift/snapper configured

    let initial_bundle = get_active_bundle();
    assert_eq!(initial_bundle, "hyprland");

    // Step 1: Create pre-test snapshot
    let snapshot_id = create_snapshot("pre-e2e-bundle-test");
    assert!(snapshot_id.is_some());

    // Step 2: Switch to niri
    let switch_result = run_command(&["bundle", "switch", "niri", "--yes"]);
    assert!(switch_result.success);

    // Step 3: Verify niri is active
    let active_bundle = get_active_bundle();
    assert_eq!(active_bundle, "niri");

    // Step 4: Verify hyprland configs in dormant
    assert!(Path::new("dormant/hyprland").exists());

    // Step 5: Verify niri symlinks in ~/.config
    assert!(is_symlink("~/.config/niri"));

    // Step 6: Switch back to hyprland
    let switch_back_result = run_command(&["bundle", "switch", "hyprland", "--yes"]);
    assert!(switch_back_result.success);

    // Step 7: Verify hyprland is active again
    let final_bundle = get_active_bundle();
    assert_eq!(final_bundle, "hyprland");

    // Step 8: Verify niri configs in dormant
    assert!(Path::new("dormant/niri").exists());

    // Step 9: Verify hyprland symlinks restored
    assert!(is_symlink("~/.config/hypr"));

    println!("✅ E2E Bundle Switch Cycle: PASSED");
}
```

### 10.4 Acceptance Test Checklist

| Test ID | User Story | Description | Status |
|---------|------------|-------------|--------|
| AT-1.1 | US-1 | First-time setup wizard | ⬜ |
| AT-1.2 | US-1 | Hardware detection | ⬜ |
| AT-1.3 | US-1 | Bundle selection | ⬜ |
| AT-1.4 | US-1 | Profile selection | ⬜ |
| AT-1.5 | US-1 | Completion under 10 minutes | ⬜ |
| AT-2.1 | US-2 | Risk score display | ⬜ |
| AT-2.2 | US-2 | Risk thresholds (LOW/MEDIUM/HIGH/CRITICAL) | ⬜ |
| AT-2.3 | US-2 | Approval required for MEDIUM+ risk | ⬜ |
| AT-2.4 | US-2 | Auto-snapshot before update | ⬜ |
| AT-2.5 | US-2 | Arch News display | ⬜ |
| AT-3.1 | US-3 | Sync push | ⬜ |
| AT-3.2 | US-3 | Sync pull | ⬜ |
| AT-3.3 | US-3 | Host-specific preservation | ⬜ |
| AT-4.1 | US-4 | State export | ⬜ |
| AT-4.2 | US-4 | Install script generation | ⬜ |
| AT-4.3 | US-4 | Recovery flow steps | ⬜ |
| AT-4.4 | US-4 | Recovery under 30 minutes | ⬜ |
| AT-5.1 | US-5 | Bundle switch creates snapshot | ⬜ |
| AT-5.2 | US-5 | Dormant storage | ⬜ |
| AT-5.3 | US-5 | Config linking | ⬜ |
| AT-5.4 | US-5 | Switch back | ⬜ |
| **AT-5.5** | **US-5** | **E2E bundle switch cycle (BLOCKING)** | ⬜ |
| AT-5.10.1 | FR-5.10 | Detect interrupted update | ✅ |
| AT-5.10.2 | FR-5.10 | Resume interrupted update | ✅ |
| AT-5.10.3 | FR-5.10 | Clear stale progress | ✅ |
| AT-5.10.4 | FR-5.10 | Progress survives crash | ✅ |
| AT-5.10.5 | FR-5.10 | CLI --status flag | ✅ |
| AT-5.10.6 | FR-5.10 | CLI --resume flag | ✅ |
| AT-5.10.7 | FR-5.10 | CLI --clear-progress flag | ✅ |
| AT-6.1 | US-6 | Profile create | ⬜ |
| AT-6.2 | US-6 | Module selection | ⬜ |
| AT-6.3 | US-6 | Profile activation | ⬜ |

**Total Acceptance Tests:** 30 (23 original + 7 FR-5.10)
**Blocking for v0.1.0:** AT-5.5 (E2E bundle switch cycle)
**FR-5.10 Complete:** 7/7 acceptance tests passing

---

## Test Execution Log

Use this section to track test execution:

```
Date: ____-__-__
Phase: ___
Tester: __________

Tests Run:
- [ ] Test 1: ___ (PASS/FAIL)
- [ ] Test 2: ___ (PASS/FAIL)

Issues Found:
1. _______________
2. _______________

Notes:
_______________
```

---

## Appendix A: Critical File Paths

| File | Purpose |
|------|---------|
| `~/.config/iron/state.json` | Application state |
| `~/.config/iron/config.toml` | User configuration |
| `crates/iron-cli/tests/` | CLI integration tests |
| `crates/iron-core/src/services/state.rs` | State management |
| `crates/iron-core/src/snapshot.rs` | Snapshot integration |
| `hosts/desktop/host.toml` | Desktop host config |
| `bundles/hyprland/bundle.toml` | Hyprland bundle |
| `bundles/niri/bundle.toml` | Niri bundle |
| `profiles/developer/profile.toml` | Developer profile |
| `modules/nvim-ide/module.toml` | Neovim module |

---

## Appendix B: Quick Reference Commands

```bash
# Full test suite
cargo test --workspace

# Linting
cargo clippy --workspace -- -D warnings

# Coverage
cargo tarpaulin --workspace --out Html

# Build release
cargo build --release

# Run CLI
./target/release/iron <command>

# Create snapshot
sudo timeshift --create --comments "description"

# Backup state
cp ~/.config/iron/state.json ~/.config/iron/state.json.bak

# Export state
iron recover --export > backup.json
```

---

*Document Version: 1.4.0*
*Last Updated: 2026-02-13*
*Author: Iron Development Team*

---

## Changelog

### v1.4.0 (2026-02-13)

**Expert Panel Review: Phase 10 Acceptance Tests**

Based on expert panel review (Wiegers, Fowler, Nygard, Crispin), added:

- **Phase 10: Acceptance Test Suite**
  - Added 23 acceptance test specifications for US-1 through US-6
  - Created test templates for all user stories in Gherkin style
  - AT-5.5 (E2E bundle switch cycle) marked as **BLOCKING for v0.1.0**

- **Updated Quality Gates**
  - Added: "Acceptance tests pass for US-1 through US-6"
  - Added: "E2E bundle switch cycle validated (BLOCKING)"

- **Expert Panel Findings Addressed**
  - Requirements: FR-5.4.1 risk thresholds, FR-5.9 command timeout, FR-10 health checks
  - Architecture: Bundle transitional states, circuit breaker pattern
  - Testing: Formal acceptance test specifications

**Related Documentation Updates:**
- `REQUIREMENTS-SPEC-v1.0.md` → v1.1.0 (risk thresholds, health checks, timeouts)
- `ARCHITECTURE.md` → v1.1.0 (transitional states, circuit breaker pattern)

### v1.3.0 (2025-02-13)

**Major Milestone: 672 Tests at 52.11% Coverage**

- **Completed Phase 8.8:** CLI Argument Parsing Tests
  - Added 35 CLI argument parsing tests to `crates/iron-cli/src/cli.rs`
  - Tests cover all 14 command groups with various flag combinations
  - Validates: status, doctor, go, init, update, bundle, profile, module, host, sync, secrets, clean, recover
  - Tests global flags: -v, -q, --no-color, --format, --root
  - iron-cli unit tests: 0 → 35 tests

- **Completed Phase 8.9:** Service Layer Tests
  - Added 11 bundle service tests (activate, deactivate, switch, conflicts)
  - Added 12 host service tests (hardware detection, save/load, overwrite)
  - Added 11 module service tests (enable, disable, effective modules)
  - Added 9 profile service tests (inheritance chain, circular detection, for_bundle)
  - Fixed ChassisType to derive PartialEq, Eq for comparison
  - Created `create_module_without_dotfiles` helper to avoid symlink issues
  - iron-core tests: 207 → 249 (+42 tests)

- **Completed Phase 8.10:** Wizard UI Rendering Tests
  - Added 16 wizard step rendering tests using TestBackend
  - Tests cover: Welcome, HostSetup, BundleSelection, ProfileSelection, Confirmation
  - Validates step progress indicators and navigation

**Test Metrics:**
- Total tests: 579 → 672 (+93 tests, +16%)
- Overall coverage: 45.26% → 52.11% (+6.85%)
- wizard.rs coverage: 0% → 100% 🎉

### v1.2.6 (2025-02-13)

**Phase 8.7: Resilience Tests**

- Added 18 resilience tests for error handling and edge cases:
  - Corrupted state file handling (invalid JSON, partial JSON, binary garbage)
  - Missing directory/file recovery
  - Invalid state value handling (null fields, extra fields)
  - Recovery from corrupted and deleted state
  - Edge cases (large module lists, unicode, special characters)
  - Audit log resilience (corrupted/missing audit log)

**Test Metrics:**
- Total tests: 561 → 579 (+18 tests)
- iron-core tests: 189 → 207 (+18 tests)

### v1.2.5 (2025-02-13)

**Phase 8.6: TUI Keyboard Handler Tests**

- Added 33 TUI keyboard interaction tests:
  - Global shortcuts (Ctrl+C, Ctrl+Q, q, ?)
  - View navigation (d, b, p, m, u, s, Tab, BackTab, Esc)
  - List navigation (j/k, arrows, Home, End)
  - Detail view navigation (Enter to open details)
  - Confirm dialog handling (y/n, Enter, Esc)
  - Bounds checking for list navigation

**Test Metrics:**
- Total tests: 528 → 561 (+33 tests)
- iron-tui tests: 113 → 146 (+33 tests)

### v1.2.4 (2025-02-13)

**Phase 8.5: Property-Based Tests with Proptest**

- Added proptest dependency to iron-core
- Created 10 property-based tests for packages.rs:
  - Risk level ordering (transitivity, reflexivity)
  - assess_risk monotonicity and invariants
  - Serialization roundtrip for PackageUpdate
  - Kernel update, flagged package, and large update detection
- Created 9 property-based tests for state.rs:
  - State serialization roundtrip preservation
  - Enable/disable idempotency
  - Double enable/disable safety
  - Active modules count accuracy
  - Host and bundle persistence across reload
  - Transaction commit/rollback behavior

**Test Metrics:**
- Total tests: 446 → 528 (+82 tests including domain model tests)
- iron-core tests: 131 → 189 (+58 tests)
- Property-based tests: 0 → 19

### v1.2.3 (2025-02-13)
- Updated test counts: 434 → 446 (+171% from baseline)
- **Completed Phase 8.4:** Concurrent access tests for state management
  - Added 12 new concurrent access tests to `crates/iron-core/src/services/state.rs`
  - Tests cover: concurrent reads, writes, stress testing, file locking, transactions
  - Verified state consistency under high concurrency (20 threads, 100+ operations)
  - Documented transaction rollback behavior (full snapshot restoration)
  - iron-core tests: 119 → 131 (+12 tests)
  - iron-core coverage: ~52% → ~54% (+2%)
- State tests now total 23 (was 11 before Phase 8.4)

### v1.2.2 (2025-02-13)
- Updated test counts: 385 → 434 (+164% from baseline)
- **Completed Phase 8.3:** iron-pacman parser tests with mock output
  - Added 44 new unit tests + 5 doc tests to `crates/iron-pacman/src/lib.rs`
  - Created public parsing APIs: `parse_updates_output()`, `parse_package_list()`, `parse_search_output()`, `parse_package_info()`, `parse_size()`
  - Added `SearchResult` struct for search result representation
  - iron-pacman tests: 9 → 58 (+49 tests)
  - iron-pacman coverage: 21.4% → ~55% (+33.6%)
- All parsing functions now have doc tests with examples

### v1.2.1 (2025-02-13)
- Updated test counts: 354 → 385 (+134% from baseline)
- **Completed Phase 8.2:** iron-core service layer mocking with MockFileSystem trait
  - Added `crates/iron-core/src/fs_trait.rs` with FileSystem trait (14 methods)
  - Added `crates/iron-core/src/test_helpers.rs` with test builders and presets
  - 28 new MockFileSystem tests (fs_trait)
  - 14 new test helper tests (test_helpers)
  - iron-core tests: 64 → 119 (+55 tests including TOML parsing)
  - iron-core coverage: ~46.8% → ~52% (+5.2%)
- Fixed borrow checker issue in symlink-following logic
- Added FsError variants: AlreadyExists, IoError
- Added exports: FileSystem, FsResult, MockFileSystem, RealFileSystem

### v1.2.0 (2025-02-13)
- Updated test counts: 304 → 354 (+115% from baseline)
- **Completed Phase 8.1:** TUI TestBackend rendering tests
  - Added 50 new TUI rendering tests in `crates/iron-tui/src/ui/tests.rs`
  - iron-tui coverage: 27.4% → 52.1% (+24.7%)
  - Covers all 8 TUI views: Dashboard, Bundles, Modules, Profiles, Settings, Update Preview, Sync
  - Test helpers for Bundle, Module, Profile, PackageUpdate creation
- iron-tui total tests: 63 → 113

### v1.1.0 (2025-02-12)
- Updated test counts: 165 → 304 (+84%)
- Added coverage metrics: 34.51% overall
- Added per-crate coverage breakdown
- Added Phase 8: Next Enhancement Implementations
  - TUI TestBackend rendering tests
  - iron-core service mocking strategy
  - iron-pacman parser tests
  - Property-based testing with proptest
  - CLI integration expansion
  - E2E automation with Playwright
  - CI/CD quality gates
- Added implementation roadmap to 80% coverage
- Updated success metrics with current progress

### v1.0.0 (2025-02-12)
- Initial testing workflow framework
- Defined safety labels and rollback strategies
- Created 7-phase testing approach
- Documented all CLI and TUI test scenarios
