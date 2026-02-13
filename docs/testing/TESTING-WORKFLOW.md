# Iron Project Testing Workflow Framework

> Comprehensive testing strategy for production readiness at v0.1.0

## Overview

This document defines a structured, incremental testing workflow for the Iron configuration management system. Tests are categorized by safety level with rollback strategies for each phase.

**Current State (Updated 2025-02-13):**
- **354 tests passing** across 7 crates (+115% from baseline)
- **34.51% code coverage** (+1.83% from baseline)
- 14 CLI command groups implemented
- 8 TUI views with rendering tests
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
| iron-core | 64 | 46.8% | `cargo test -p iron-core` | CRITICAL |
| iron-cli | 85 | 0%* | `cargo test -p iron-cli` | HIGH |
| iron-tui | 113 | 52.1% | `cargo test -p iron-tui` | HIGH |
| iron-systemd | 37 | 37.6% | `cargo test -p iron-systemd` | MEDIUM |
| iron-git | 34 | 29.3% | `cargo test -p iron-git` | MEDIUM |
| iron-fs | 12 | 46.2% | `cargo test -p iron-fs` | MEDIUM |
| iron-pacman | 9 | 21.4% | `cargo test -p iron-pacman` | MEDIUM |

*Note: CLI coverage is 0% due to subprocess spawning limitation in tarpaulin. CLI integration tests (24 tests) validate command behavior.

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
- [ ] All 165+ tests passing: `cargo test --workspace`
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
| Unit tests | **304** | 400+ | 🟢 |
| Test coverage | **34.51%** | 80%+ | 🟡 |
| CLI commands tested | 0/14 | 14/14 | ⬜ |
| TUI views tested | 2/8 | 8/8 | 🟡 |
| Error scenarios tested | 5 | 10+ | 🟡 |
| Recovery scenarios tested | 2 | 3+ | 🟡 |
| Bundle switch verified | No | Yes | ⬜ |
| Documentation complete | Partial | Yes | 🟡 |

### Coverage by Crate (Updated 2025-02-12)

| Crate | Lines Covered | Total Lines | Coverage |
|-------|--------------|-------------|----------|
| iron-core | 763 | 1630 | 46.8% |
| iron-fs | 128 | 277 | 46.2% |
| iron-systemd | 50 | 133 | 37.6% |
| iron-git | 44 | 150 | 29.3% |
| iron-tui | 171 | 624 | 27.4% |
| iron-pacman | 60 | 281 | 21.4% |
| iron-cli | 796 | 798 | 99.7%* |

*CLI coverage reflects integration tests, not command handlers (subprocess limitation).

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

#### 8.2 iron-core Service Layer Mocking

**Target Coverage Increase:** +10-15%

Services like `BundleService`, `ModuleService`, and `UpdateService` interact with the filesystem. Add mock filesystem traits for isolated testing.

**Implementation Pattern:**

```rust
// crates/iron-core/src/services/test_helpers.rs

use std::path::PathBuf;
use std::collections::HashMap;

pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn symlink(&self, src: &Path, dst: &Path) -> io::Result<()>;
}

pub struct MockFileSystem {
    files: HashMap<PathBuf, String>,
    directories: Vec<PathBuf>,
    symlinks: HashMap<PathBuf, PathBuf>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: Vec::new(),
            symlinks: HashMap::new(),
        }
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, content: &str) -> Self {
        self.files.insert(path.into(), content.to_string());
        self
    }
}
```

**Files to Create/Modify:**
- `crates/iron-core/src/services/test_helpers.rs` (new)
- `crates/iron-core/src/services/bundle.rs` (add mock tests)
- `crates/iron-core/src/services/module.rs` (add mock tests)

**Estimated Tests:** 40-60 new tests

#### 8.3 iron-pacman Parser Tests

**Target Coverage Increase:** +8-10%

The pacman crate has parsing logic for package queries that can be tested with mock outputs.

**Implementation Pattern:**

```rust
// Add to crates/iron-pacman/src/lib.rs

pub fn parse_pacman_query(output: &str) -> Vec<PackageInfo> {
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(PackageInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_pacman_query_multiple_packages() {
        let output = "hyprland 0.40.0-1\nwaybar 0.10.0-1\nwofi 1.4-1";
        let packages = parse_pacman_query(output);
        assert_eq!(packages.len(), 3);
        assert_eq!(packages[0].name, "hyprland");
        assert_eq!(packages[0].version, "0.40.0-1");
    }

    #[test]
    fn test_parse_pacman_query_empty_output() {
        let packages = parse_pacman_query("");
        assert!(packages.is_empty());
    }
}
```

**Estimated Tests:** 15-25 new tests

---

### Priority 2: Resilience & Edge Cases

#### 8.4 Concurrent Access Tests

**Location:** `crates/iron-core/src/services/state.rs`

```rust
#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_state_reads() {
        let state = Arc::new(State::default());
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    state.enabled_modules().len()
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().is_ok());
        }
    }

    #[test]
    fn test_file_locking_prevents_corruption() {
        // Test with file-based locking
    }
}
```

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
| Week 1 | TUI TestBackend rendering | +35 | 40% |
| Week 1 | iron-pacman parsers | +20 | 43% |
| Week 2 | iron-core service mocks | +50 | 55% |
| Week 2 | Property-based testing | +15 | 58% |
| Week 3 | CLI integration expansion | +30 | 65% |
| Week 3 | Concurrent access tests | +10 | 67% |
| Week 4 | Doctests | +20 | 72% |
| Week 4 | E2E automation | +15 | 75% |
| Week 5 | Coverage gap hunting | +25 | 80% |

**Total Estimated:** +220 tests → **524 total tests** at **80% coverage**

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

*Document Version: 1.2.0*
*Last Updated: 2025-02-13*
*Author: Iron Development Team*

---

## Changelog

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
