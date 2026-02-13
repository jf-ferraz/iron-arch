# Iron Project Testing Workflow Framework

> Comprehensive testing strategy for production readiness at v0.1.0

## Overview

This document defines a structured, incremental testing workflow for the Iron configuration management system. Tests are categorized by safety level with rollback strategies for each phase.

**Current State:**
- 165 tests passing across 7 crates
- 14 CLI command groups implemented
- 8 TUI views functional
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

- [ ] All 165 tests pass
- [ ] Zero clippy warnings
- [ ] Release binary builds successfully
- [ ] `--version` displays correct version
- [ ] `--help` displays all command groups

---

## Phase 2: Unit Test Deep-Dive

**Duration:** Days 2-3
**Risk Level:** `[SAFE]`

### 2.1 Per-Crate Test Execution

| Crate | Expected Tests | Command | Priority |
|-------|----------------|---------|----------|
| iron-core | ~62 | `cargo test -p iron-core` | CRITICAL |
| iron-cli | ~54 | `cargo test -p iron-cli` | HIGH |
| iron-tui | ~22 | `cargo test -p iron-tui` | HIGH |
| iron-fs | ~12 | `cargo test -p iron-fs` | MEDIUM |
| iron-pacman | ~9 | `cargo test -p iron-pacman` | MEDIUM |
| iron-git | ~3 | `cargo test -p iron-git` | MEDIUM |
| iron-systemd | ~3 | `cargo test -p iron-systemd` | MEDIUM |

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
| Unit tests | 165 | 200+ | 🟡 |
| Test coverage | TBD | 80%+ | ⬜ |
| CLI commands tested | 0/14 | 14/14 | ⬜ |
| TUI views tested | 0/8 | 8/8 | ⬜ |
| Error scenarios tested | 0 | 10+ | ⬜ |
| Recovery scenarios tested | 0 | 3+ | ⬜ |
| Bundle switch verified | No | Yes | ⬜ |
| Documentation complete | No | Yes | ⬜ |

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

*Document Version: 1.0.0*
*Last Updated: 2025-02-12*
*Author: Iron Development Team*
