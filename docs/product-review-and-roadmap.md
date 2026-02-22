# Iron-Arch: Product & Engineering Review — Roadmap to Maximal User Success

> **Author Role:** Product Manager & Senior Software Engineer (Rust, UX, Arch Linux)
> **Date:** February 22, 2026
> **Inputs:** Newcomer expectations brainstorm, Mid-level user expectations brainstorm, full codebase audit (~64,500 LOC, 1,703 tests, 7 crates)
>
> **Purpose:** Map every user expectation against the current implementation, identify what works, what doesn't, what's missing, and produce a prioritized development roadmap to reach maximal user success across both audience segments.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Codebase Health Assessment](#2-codebase-health-assessment)
3. [What Works Well — Strengths](#3-what-works-well--strengths)
4. [What Doesn't Work — Critical Gaps](#4-what-doesnt-work--critical-gaps)
5. [What's Missing — Unmet User Expectations](#5-whats-missing--unmet-user-expectations)
6. [UX & Interface Deep Dive](#6-ux--interface-deep-dive)
7. [Architecture Simplification Opportunities](#7-architecture-simplification-opportunities)
8. [User Constraint Coverage Matrix](#8-user-constraint-coverage-matrix)
9. [Roadmap: Development Phases](#9-roadmap-development-phases)
10. [Phase 0: Foundation Fixes (Sprint 1-2)](#10-phase-0-foundation-fixes)
11. [Phase 1: Core Experience (Sprint 3-5)](#11-phase-1-core-experience)
12. [Phase 2: Power User Features (Sprint 6-8)](#12-phase-2-power-user-features)
13. [Phase 3: Ecosystem & Community (Sprint 9-10)](#13-phase-3-ecosystem--community)
14. [Simplification Strategy](#14-simplification-strategy)
15. [Risk Register](#15-risk-register)
16. [Success Metrics](#16-success-metrics)
17. [Final Verdict](#17-final-verdict)

---

## 1. Executive Summary

### The Bottom Line

Iron has **exceptional engineering bones** — a well-structured Rust workspace, clean separation of concerns, 1,703 tests, trait-based abstractions, and a thoughtful domain model (Host → Bundle → Profile → Module). The architecture is sound.

However, there is a **critical gap between the engineering quality and the user experience quality.** The tool has been built *inside-out* (architecture → features → UI) rather than *outside-in* (user workflow → UI → features → architecture). This manifests as:

1. **The tool is feature-complete on paper but not workflow-complete in practice.** All the pieces exist (scan, update, clean, sync, backup) but they don't compose into the fluid workflows users described in the brainstorm documents.
2. **The CLI is functional but not delightful.** Output is adequate but doesn't match the structured, tree-style, color-coded output both user personas expect.
3. **The TUI is impressive in scope (27 views!) but potentially overwhelming.** There's no clear entry-point simplification for newcomers vs. power users.
4. **Configuration management is declared but not truly operational.** Modules define packages and dotfiles, but the actual apply→verify→rollback cycle has gaps (no `iron apply`, no `iron diff` for config changes, drift detection is partial).
5. **The "iron" command invoked with no arguments shows a welcome message instead of the TUI.** The TUI requires `iron go` — an unnecessary friction point.

### Opportunity Score: 8.5/10

The codebase is 80% of the way to being extraordinary. The remaining 20% is the difference between "a tool that works" and "a tool users love." That 20% is almost entirely about **workflow polish, UX refinement, and closing the apply→verify→rollback loop.**

---

## 2. Codebase Health Assessment

### Architecture: A+ 

| Aspect | Rating | Evidence |
|--------|--------|----------|
| **Crate separation** | ★★★★★ | 7 crates with clean layering: core has zero infra deps |
| **Trait abstractions** | ★★★★★ | `PackageManager`, `SnapshotManager`, `FileSystem`, `CommandExecutor`, `SecretsBackend` — all testable |
| **Error handling** | ★★★★☆ | Comprehensive `IronError` hierarchy with `thiserror`. Some raw `Command::new` still bypasses circuit breaker |
| **Testing** | ★★★★☆ | 1,703 tests, mock-based, good coverage. Integration tests require sudo (expected but noted) |
| **Resilience** | ★★★★☆ | Circuit breaker pattern, 120s timeouts, graceful degradation. A-001 (SyncService raw commands) still open |
| **State management** | ★★★★☆ | Three-layer model (config/runtime/dormant) is well-designed. State file is JSON — less human-friendly but machine-friendly |
| **Domain model** | ★★★★★ | Host → Bundle → Profile → Module hierarchy is elegant and maps to real user mental models |
| **Code quality** | ★★★★☆ | Consistent style, 100-char line width, minimal clippy warnings. Some functions have too many arguments |

### Known Tech Debt (from `current.md`)

| Item | Impact | Status |
|------|--------|--------|
| A-001: SyncService raw `Command::new("git")` | Bypasses resilience layer | **Open** |
| F-005: CleanupService raw `Command::new` | Bypasses iron_pacman | **Open** |
| C-009: Recovery import is state-only | Can't restore packages/services/dotfiles | **Open** |
| D-012: ModuleCreator missing dotfile step | Wizard can't create complete modules | **Open** |
| A-009: SyncService per-action instances | Memory/performance overhead | **Open** |
| A-010: Secrets not locked before push | Security risk | **Open** |
| D-009: Push/pull blocks TUI thread | UI freezes on sync | **Open** |

**Verdict:** The tech debt is manageable and well-documented. None of it is structural — all items are targeted fixes.

---

## 3. What Works Well — Strengths

### 3.1 🟢 Declarative Configuration Model

The TOML-based configuration is exactly what both user personas asked for:

```toml
# modules/nvim-ide/module.toml — Self-contained, readable, composable
id = "nvim-ide"
name = "Neovim IDE"
packages = ["neovim", "ripgrep", "fd", "lazygit", "nodejs", "npm"]
conflicts = ["vim-minimal"]

[[dotfiles]]
source = "config/nvim"
target = "~/.config/nvim"
link = true
```

**This is a home run.** The module format captures packages, dotfiles, hooks, conflicts, and dependencies in a single readable file. This directly satisfies:
- ✅ Newcomer: "Define system in files, not remember 50 commands"
- ✅ Mid-level: "Module-level isolation, self-contained with files, packages, dependencies"

### 3.2 🟢 Bundle System for Desktop Environments

The ability to switch entire desktop environments (Hyprland, Niri, KDE, GNOME, Cosmic) with dormant state management is a genuinely differentiating feature. The state machine (NotInstalled → Active → Dormant → Active) with snapshot-before-switch is well-engineered.

**Satisfies:**
- ✅ Newcomer: "Choose my DE and have the tool set it up"
- ✅ Mid-level: "Try a different window manager without losing my current setup"

### 3.3 🟢 Safe Update Workflow

The update system is the most polished feature:
- Risk scoring (LOW/MEDIUM/HIGH/CRITICAL)
- Pre-flight checks before any update
- Arch News integration
- Typed confirmation for CRITICAL updates
- Partial update recovery (FR-5.10) with resume capability
- Real-time pacman output parsing

**This directly addresses the #1 user anxiety** — "Will this update break my system?"

### 3.4 🟢 System Scan

The `ScanService` discovers existing configs, finds package overlaps, identifies conflicts, and generates recommendations. It scans well-known XDG config paths and home dotfiles.

**Satisfies:**
- ✅ Newcomer: "I want to know what's on my system"
- ✅ Mid-level: "Auto-detect installed configs and generate module definitions"

### 3.5 🟢 Comprehensive Cleanup

9 cleanup categories from safe (package cache, orphans, journal) to aggressive (browser cache, dev cache). Preview before execute. Space estimation.

### 3.6 🟢 TUI Theme System

The theme uses terminal ANSI colors that respect the user's colorscheme. This is the correct approach — it adapts to Catppuccin, Dracula, Gruvbox, etc. automatically. The icon set is clean and consistent.

### 3.7 🟢 Recovery & Backup

State export to JSON, install script generation, backup/restore workflow. The recovery export captures host, bundle, profile, modules, packages, AUR packages, and services.

### 3.8 🟢 Secrets Management

git-crypt integration with init/unlock/lock/link workflow. Secrets are encrypted at rest and separated from the main config.

---

## 4. What Doesn't Work — Critical Gaps

### 4.1 🔴 No `iron apply` Command

**This is the single biggest gap in the entire tool.**

Both user personas described `iron apply` as their most critical workflow:
- Newcomer: *"iron apply — Apply my declared config to the system"*
- Mid-level: *"iron apply --module hyprland — Apply just this module"*

The tool has:
- `iron module enable <id>` — enables one module (links dotfiles, runs hooks)
- `iron bundle install <id>` — installs a bundle
- `iron profile select <id>` — activates a profile

But there is **no single command that says "make my system match my declared state."** This is the `terraform apply` equivalent — the most important command in any declarative system — and it's missing.

**What `iron apply` should do:**
1. Read the host definition
2. Ensure the declared bundle is installed and active
3. Ensure the declared profile is selected
4. Ensure all declared modules are enabled
5. Install any missing packages
6. Create any missing symlinks
7. Enable any declared services
8. Report what changed

**Impact:** Without this, the tool is a collection of individual operations, not a declarative system manager. Users must manually orchestrate the correct sequence of `bundle install`, `profile select`, `module enable` — which defeats the purpose.

### 4.2 🔴 No `iron diff` for Configuration State

Users expect to see what would change before applying:
- *"iron diff — See what would change before applying"*
- *"Diff view for config changes — before applying, show me what's changing"*

The tool has `iron update --dry-run` for package updates, but there's no equivalent for configuration state. No way to see:
- "Module X would create 3 symlinks"
- "Profile Y would enable modules A, B, C (currently disabled)"
- "Your actual system has diverged from declared state in these ways"

**Impact:** Users can't preview configuration changes, which violates the #1 deal breaker: "Dry-run / preview mode before any operation."

### 4.3 🔴 Drift Detection is Partial

The mid-level user called drift detection "arguably the MOST IMPORTANT feature." The codebase has:
- ✅ `check_divergence()` in the TUI — checks if module dotfile symlinks are broken
- ✅ `ScanService` — discovers existing configs and overlaps

But it's missing:
- ❌ **Package drift** — Packages installed manually (not in any module/bundle) vs. packages declared but not installed
- ❌ **Service drift** — Services enabled/disabled manually vs. declared state
- ❌ **Config file content drift** — A dotfile symlink exists but the source has been modified outside the tool
- ❌ **Adopt or correct** — No workflow for incorporating drift into canonical state OR reverting to declared state

The `diverged_modules` field in the TUI only tracks broken symlinks, not full semantic drift.

### 4.4 🔴 `iron go` vs. Bare `iron`

Running `iron` with no arguments prints a welcome message. The TUI requires `iron go`. This is counterintuitive:

**Expected behavior (based on both user personas):**
- `iron` → Launch TUI dashboard (the primary interface)
- `iron --help` → CLI help

**Current behavior:**
- `iron` → Welcome text with "Run `iron --help` for CLI commands, `iron go` for TUI"
- `iron go` → Launch TUI

**Fix:** Make `iron` launch the TUI by default. The CLI is the secondary interface; the TUI is the primary one per the project's own requirements (FR-9).

### 4.5 🔴 No Template/Variable System

The mid-level user identified this as a deal breaker: *"{{hostname}}, {{monitor_primary}}, {{terminal}} — I need configs that adapt to the machine they're on."*

Currently, module dotfiles are symlinked as-is. There's no template rendering:
- `hyprland.conf` can't reference `{{primary_monitor}}` to auto-adapt between desktop (DP-1) and laptop (eDP-1)
- No host-specific variables that modules can reference
- No way to share a config file between machines with minor hardware-specific differences

This is the core enabler for cross-machine config management. Without it, users must maintain separate module configs per host, defeating the purpose of multi-host management.

### 4.6 🟡 Host Configuration is Thin

The `hosts/*.toml` files store hardware info (CPU, GPU, RAM, monitors) but:
- ❌ No `bundle`, `profile`, or `modules` fields — the host doesn't declare what should be active on it
- ❌ No host-specific variables
- ❌ No host-specific package overrides
- The relationship between host and bundle/profile exists only in `state.json` (runtime), not in the declarative config

**Expected:**
```toml
# hosts/desktop.toml
id = "desktop"
name = "Desktop Workstation"
bundle = "hyprland"
profile = "developer"
extra_modules = ["gaming", "vm-tools"]

[hardware]
cpu = "AMD Ryzen 9"
# ...

[variables]
primary_monitor = "DP-1"
terminal = "kitty"
```

**Actual:** Host files contain only hardware info and `installed_bundles = []`.

### 4.7 🟡 No Rollback Per-Module

The mid-level user expected: *"Granular rollback. I don't want to rollback my entire system because one module's config was bad."*

Current state:
- Module enable creates `.iron-backup` files before overwriting existing configs ✅
- Module disable restores from `.iron-backup` ✅
- But there's no timestamped history — only one backup level deep
- No `iron rollback --module X` command
- No snapshot history per module

### 4.8 🟡 CLI Output is Functional but Not Structured

The `Output` struct in `output.rs` provides basic formatting (✓, ✗, ⚠, ℹ with colors), but the output doesn't match the structured tree-style output the mid-level user described:

**What users expect:**
```
  ● Applying module: hyprland
    ├── Checking packages...
    │   ✓ hyprland (already installed)
    ├── Applying configs...
    │   ~ ~/.config/hypr/hyprland.conf (3 lines changed)
    └── Done ✓
```

**What they get:**
```
✓ Module 'hyprland' enabled
```

Single-line success messages are informative but not educational or transparent.

---

## 5. What's Missing — Unmet User Expectations

### Priority 1: Missing Core Features

| Feature | Newcomer Need | Mid-Level Need | Difficulty | Impact |
|---------|:------------:|:--------------:|:----------:|:------:|
| `iron apply` command | 🔴 Critical | 🔴 Critical | Medium | 🔴 Highest |
| `iron diff` command | 🔴 Critical | 🔴 Critical | Medium | 🔴 Highest |
| Template variables in dotfiles | 🟡 Desired | 🔴 Critical | Medium | 🔴 High |
| Host declares bundle/profile/modules | 🟡 Desired | 🔴 Critical | Low | 🔴 High |
| Full drift detection (pkgs + services + configs) | 🟢 Nice | 🔴 Critical | High | 🔴 High |
| TUI as default (no `iron go`) | 🔴 Critical | 🟡 Desired | Trivial | 🟡 Medium |

### Priority 2: Missing Workflow Features

| Feature | Newcomer Need | Mid-Level Need | Difficulty | Impact |
|---------|:------------:|:--------------:|:----------:|:------:|
| Module rollback with history | 🟡 Desired | 🔴 Critical | Medium | 🟡 High |
| Selective import from external dotfile repos | 🟢 Nice | 🔴 Critical | High | 🟡 High |
| Config validation before apply | 🔴 Critical | 🔴 Critical | Medium | 🟡 High |
| Structured CLI output (tree-style) | 🟡 Desired | 🟡 Desired | Low | 🟡 Medium |
| Progressive security levels display | 🟡 Desired | 🟡 Desired | Low | 🟡 Medium |
| Explain mode (show underlying commands) | 🔴 Critical | 🟢 Nice | Low | 🟡 Medium |

### Priority 3: Missing Experience Features

| Feature | Newcomer Need | Mid-Level Need | Difficulty | Impact |
|---------|:------------:|:--------------:|:----------:|:------:|
| Module/config snapshot timeline | 🟡 Desired | 🟡 Desired | Medium | 🟢 Medium |
| Host comparison / diff between hosts | 🟢 Nice | 🟡 Desired | Medium | 🟢 Medium |
| Interactive diff/merge for conflicts | 🟢 Nice | 🟡 Desired | High | 🟢 Medium |
| Tab completions (fish/zsh/bash) | 🟡 Desired | 🟡 Desired | Done ✅ | 🟢 Done |
| Scheduled operations (cron/timer) | 🟢 Nice | 🟢 Nice | Medium | 🟢 Low |

---

## 6. UX & Interface Deep Dive

### 6.1 TUI: Impressive Scope, Needs Focus

**27 views** is a lot. The TUI has: Dashboard, SetupWizard, Bundles, BundleDetail, Profiles, ProfileDetail, Modules, ModuleDetail, UpdatePreview, Sync, Settings, SystemMaintenance, CleanSystem, CleanupPreview, CleanupResults, SecurityModules, ConfigManager, OperationLog, Doctor, Secrets, Recovery, ProfileBuilder, ModuleCreator, SystemScan, HostSelection.

**Problem:** No clear information hierarchy. A newcomer opening the TUI sees the Dashboard with 6 panels — System Health, Maintenance, Quick Actions, Active Configuration, Recent Operations, Alerts. This is good for a returning user but overwhelming for a first-time visitor.

**Recommendation:**
- **First-launch experience** works well (SetupWizard detects when no state exists) ✅
- **Add a "Getting Started" panel on Dashboard** that appears when < 3 operations have been performed. Guide users to: "Try [u]pdate to check for updates" → "Run [s]can to discover your system" → "Visit [m]odules to enable configurations"
- **Consolidate views:** SecurityModules, ConfigManager, and Settings could be tabs within a single Settings view. 27 views → ~20 views.

### 6.2 TUI Dashboard: Good but Missing Key Info

The dashboard shows:
- ✅ System Health (individual checks)
- ✅ Maintenance timestamps (last update, last clean)
- ✅ Quick Actions (keyboard shortcuts)
- ✅ Active Configuration (bundle/profile/modules)
- ✅ Recent Operations
- ✅ Alerts

**Missing from dashboard:**
- ❌ **Drift indicator** — "2 modules diverged from declared state" with a shortcut to view details
- ❌ **Pending updates count** — Shows in the update view but not on the dashboard summary
- ❌ **Disk space** — A fundamental health metric
- ❌ **Sync status** — "3 commits ahead, 0 behind" or "Last sync: 2 days ago"

### 6.3 CLI: Needs a Personality Upgrade

The CLI output module (`output.rs`, 334 lines) provides:
- ✅ Color with `--no-color` fallback
- ✅ JSON output mode
- ✅ Quiet and verbose modes
- ✅ Icons (✓, ✗, ⚠, ℹ)

**What it needs:**
- Tree-style hierarchical output for multi-step operations
- Summary blocks after operations ("3 packages installed, 2 configs linked, 0 errors")
- `--explain` mode that shows underlying commands being executed
- Table formatting for list outputs (modules, packages, hosts)
- Progress spinners/bars for long operations

### 6.4 Onboarding Flow Analysis

**Current first-run:**
1. User runs `iron` → sees welcome text
2. Must know to run `iron go` for TUI or `iron init` for CLI
3. TUI wizard: Welcome → Host Setup → Bundle Selection → Profile Selection → Confirmation → Complete

**Issues:**
- Step 2 is an unnecessary knowledge barrier
- The wizard doesn't run `iron scan` to discover existing configs
- After wizard, the user is on the Dashboard but hasn't applied anything — no packages installed, no configs linked
- No guidance on "what to do next"

**Recommended first-run:**
1. `iron` → Auto-detects first run → Launches TUI wizard
2. Wizard: Welcome → Auto-scan system → Host Setup → Bundle Selection → Profile Selection → Preview changes (diff) → Apply with progress → Complete with next-steps guide
3. Dashboard shows "Getting Started" prompts until user has performed 3+ operations

### 6.5 Error Messages: Adequate but Not Exemplary

The error hierarchy (`error.rs`, 1,086 lines) is comprehensive. Error messages are descriptive:
- `"No active host configured. Run 'iron init' first."` ✅
- `"Module 'nvim-ide' not found"` ✅
- `"Conflict: module X conflicts with module Y"` ✅

**What could be better:**
- No `Did you mean...?` suggestions for typos
- No `Learn more:` links to documentation
- No recovery suggestions beyond "Run 'iron init'"
- Errors don't show the operation context ("While enabling module 'nvim-ide'...")

---

## 7. Architecture Simplification Opportunities

### 7.1 Eliminate the `iron go` Indirection

**Change:** Make `None` (no subcommand) launch the TUI instead of printing a welcome message.

```rust
// main.rs — change the None arm
None => {
    // Launch TUI as default (the primary interface)
    let root = std::path::PathBuf::from(&cli.root);
    let package_manager = Arc::new(iron_pacman::DefaultPackageManager::default());
    let service_manager = Arc::new(iron_systemd::SystemdServiceAdapter::user());
    iron_tui::run_with_config(root, package_manager, service_manager)
}
```

Keep `iron go` as an alias for backward compatibility. This is a 5-line change with massive UX impact.

### 7.2 Unify the Apply Workflow

Currently, applying a system state requires:
1. `iron bundle install hyprland`
2. `iron profile select developer`
3. `iron module enable nvim-ide`
4. `iron module enable kitty-dev`
5. ... for each module

This should be:
1. Define everything in `hosts/desktop.toml`:
   ```toml
   bundle = "hyprland"
   profile = "developer"
   extra_modules = ["gaming"]
   ```
2. `iron apply` — Makes it all happen.
3. `iron apply --dry-run` — Shows what would change.

### 7.3 Simplify Host Definition

Add bundle/profile/modules to host TOML:

```toml
# hosts/desktop.toml
id = "desktop"
name = "Desktop Workstation"

# Declared system state
bundle = "hyprland"
profile = "developer"
extra_modules = ["gaming", "vm-tools"]

# Template variables for this host
[variables]
primary_monitor = "DP-1"
secondary_monitor = "DP-2"
terminal = "kitty"
browser = "firefox"

[hardware]
cpu = "AMD Ryzen 9"
gpu = "AMD RX 9060 XT"
ram_mb = 31191
chassis = "Desktop"

[[hardware.monitors]]
output = "DP-1"
resolution = "2560x1440"
```

This makes the host file the **single source of truth** for that machine, which is what both user personas demanded.

### 7.4 Template Engine for Dotfiles

Add a lightweight template system. When a dotfile has a `.tmpl` extension, render it with host variables before symlinking:

```
# modules/hyprland/config/hyprland.conf.tmpl
monitor = {{primary_monitor}}, 2560x1440@60, 0x0, 1
monitor = {{secondary_monitor}}, 1920x1080@60, 2560x0, 1

$terminal = {{terminal}}
$browser = {{browser}}
```

Use a simple `{{variable}}` syntax — no need for a full template engine. A regex replace with the host's `[variables]` table is sufficient for v1.

### 7.5 Consolidate Command Surface

Current: 14 top-level commands × subcommands = ~40+ command combinations.

Proposal — Keep all existing commands but add these workflow commands that compose them:

| Command | What It Does | Composes |
|---------|-------------|----------|
| `iron apply` | Converge system to declared state | bundle + profile + modules + packages + services |
| `iron diff` | Show difference between declared and actual state | scan + package check + symlink check |
| `iron snapshot` | Create a named restore point | state export + config backup |
| `iron rollback [name]` | Restore to a snapshot | state import + config restore |

These four commands cover 80% of daily workflows. The existing granular commands remain for advanced use.

---

## 8. User Constraint Coverage Matrix

### Newcomer Deal Breakers

| # | Constraint | Status | Gap |
|---|-----------|--------|-----|
| 1 | No destructive ops without confirmation | ✅ **Met** | Risk-differentiated confirms in TUI + CLI |
| 2 | Dry-run / preview mode | 🟡 **Partial** | `--dry-run` for updates only. No dry-run for apply/module/bundle |
| 3 | Clear, human-readable output | 🟡 **Partial** | Good icons/colors but flat output, not structured |
| 4 | Rollback / undo capability | 🟡 **Partial** | Module backup exists but only 1 level deep. No named snapshots |
| 5 | Declarative system definition | 🟡 **Partial** | Modules are declarative. Hosts don't declare desired state |
| 6 | Works offline | ✅ **Met** | Explicitly offline-first design |
| 7 | Doesn't fight the system | ✅ **Met** | Uses pacman, systemd, git, standard files |
| 8 | Idempotent operations | ✅ **Met** | Module enable is idempotent. Bundle switch is safe |
| 9 | No silent failures | ✅ **Met** | Comprehensive error hierarchy, operation logging |
| 10 | Transparent (shows commands) | 🟡 **Partial** | No `--explain` mode. Logs exist but hidden in JSONL |

### Mid-Level Deal Breakers

| # | Constraint | Status | Gap |
|---|-----------|--------|-----|
| 1 | Layered, composable configuration | 🟡 **Partial** | Bundle → Profile → Module works. No host-level composition |
| 2 | Git-native workflow | ✅ **Met** | Git-backed state, sync push/pull, conflict detection |
| 3 | Granular module system | ✅ **Met** | Self-contained modules with packages, dotfiles, hooks |
| 4 | Drift detection | 🟡 **Partial** | Symlink divergence only. No package/service drift |
| 5 | Diff before apply | ❌ **Not Met** | No `iron diff` command |
| 6 | Non-destructive by default | ✅ **Met** | Backup before overwrite, confirms before destructive ops |
| 7 | Template/variable system | ❌ **Not Met** | Dotfiles are static, no interpolation |
| 8 | Selective operations | ✅ **Met** | Per-module enable/disable, per-category cleanup |
| 9 | Clear dependency tracking | ✅ **Met** | Modules declare `depends` and `conflicts` |
| 10 | Escape hatch | ✅ **Met** | Standard files in standard locations. No proprietary runtime |

---

## 9. Roadmap: Development Phases

### Overview

```
Phase 0: Foundation Fixes           [2 sprints]   — Close critical gaps, fix UX friction
Phase 1: Core Experience            [3 sprints]   — Apply/diff/drift, template engine, host-as-truth
Phase 2: Power User Features        [3 sprints]   — Dotfile import, rollback timeline, explain mode
Phase 3: Ecosystem & Community      [2 sprints]   — Module registry, config sharing, community bundles
```

**Total:** ~10 sprints (~20 weeks at 2-week sprints)

Each phase is independently shippable. Users get value after Phase 0. Each subsequent phase adds multiplicative value.

---

## 10. Phase 0: Foundation Fixes (Sprint 1-2)

> **Goal:** Remove friction, close the highest-impact gaps with minimal code changes, resolve all open tech debt.

### Sprint 0.1: UX Quick Wins

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F0-001** | Make `iron` (no args) launch TUI. Keep `iron go` as alias. | XS | 🔴 High |
| **F0-002** | Add pending update count + sync status to TUI dashboard | S | 🟡 Medium |
| **F0-003** | Add disk space check to Doctor and Dashboard | S | 🟡 Medium |
| **F0-004** | Add "Getting Started" hints on Dashboard for new users (< 3 operations) | M | 🟡 Medium |
| **F0-005** | Add summary line after every CLI operation ("3 packages installed, 2 configs linked") | S | 🟡 Medium |
| **F0-006** | Add `--explain` flag to CLI that prints underlying commands before executing | M | 🟡 Medium |

### Sprint 0.2: Tech Debt Closure

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F0-007** | Close A-001: Migrate SyncService to CommandExecutor (16 raw Command calls) | L | 🔴 High |
| **F0-008** | Close F-005: Migrate CleanupService to iron_pacman | M | 🟡 Medium |
| **F0-009** | Close D-009: Make sync push/pull async with TUI progress indicator | M | 🟡 Medium |
| **F0-010** | Close A-010: Auto-lock secrets before sync push | S | 🟡 Medium |
| **F0-011** | Close D-012: Add dotfile mapping step to ModuleCreator wizard | M | 🟡 Medium |
| **F0-012** | Close C-009: Recovery import installs packages + enables services + links dotfiles | L | 🔴 High |

---

## 11. Phase 1: Core Experience (Sprint 3-5)

> **Goal:** Implement the declarative apply→diff→verify→rollback loop that both user personas describe as their core workflow.

### Sprint 1.1: Host as Source of Truth

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F1-001** | Extend `Host` struct with `bundle`, `profile`, `extra_modules`, `[variables]` fields | M | 🔴 Critical |
| **F1-002** | Backward-compatible TOML parsing (new fields optional, migration from state.json) | M | 🟡 Medium |
| **F1-003** | `HostService::desired_state()` — returns desired bundle/profile/modules from host.toml | M | 🔴 Critical |
| **F1-004** | TUI HostSelection and wizard write bundle/profile into host.toml on completion | M | 🟡 Medium |

### Sprint 1.2: The Apply Command

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F1-005** | New `ApplyService` in iron-core: compares desired state (host.toml) vs actual state (system) | L | 🔴 Critical |
| **F1-006** | `ApplyPlan` struct: list of actions (install bundle, select profile, enable modules, install packages, create symlinks, enable services) | M | 🔴 Critical |
| **F1-007** | `iron apply` CLI command: compute plan → display → confirm → execute with progress | L | 🔴 Critical |
| **F1-008** | `iron apply --dry-run` — display plan without executing | S | 🔴 Critical |
| **F1-009** | `iron apply --module <id>` — apply only one module | M | 🟡 Medium |
| **F1-010** | TUI Apply view: visual plan with progress bars, accessible from Dashboard [a] key | L | 🟡 Medium |

### Sprint 1.3: Diff & Drift Detection

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F1-011** | `DriftService` in iron-core: compares declared state vs actual state | L | 🔴 Critical |
| **F1-012** | Package drift: declared packages vs `pacman -Qqe` output | M | 🔴 High |
| **F1-013** | Service drift: declared services vs `systemctl list-unit-files --state=enabled` | M | 🟡 Medium |
| **F1-014** | Config drift: symlink targets correct + source file unchanged (checksum) | M | 🔴 High |
| **F1-015** | `iron diff` CLI command: displays full drift report | M | 🔴 Critical |
| **F1-016** | `iron diff --adopt` — incorporates discovered drift into canonical state | M | 🟡 Medium |
| **F1-017** | `iron diff --correct` — reverts system to match declared state | M | 🟡 Medium |
| **F1-018** | TUI drift indicator on Dashboard + Drift detail view | M | 🟡 Medium |

### Sprint 1.4: Template Engine (optional — can be Sprint 2.x)

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F1-019** | Simple `{{variable}}` template engine in iron-fs | M | 🔴 High |
| **F1-020** | `.tmpl` files in modules rendered with host `[variables]` before symlinking | M | 🔴 High |
| **F1-021** | Built-in variables: `{{hostname}}`, `{{username}}`, `{{home}}`, `{{config_dir}}` | S | 🟡 Medium |
| **F1-022** | TUI variable editor in Host settings | M | 🟢 Nice |

---

## 12. Phase 2: Power User Features (Sprint 6-8)

> **Goal:** Enable fearless experimentation, multi-machine workflows, and progressive security.

### Sprint 2.1: Snapshot & Rollback System

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F2-001** | `SnapshotService`: creates named snapshots (config state + file checksums) | L | 🔴 High |
| **F2-002** | `iron snapshot create <name>` — saves current state with label | M | 🔴 High |
| **F2-003** | `iron snapshot list` — shows all snapshots with timestamps and labels | S | 🟡 Medium |
| **F2-004** | `iron snapshot restore <name>` — restores to a named snapshot | L | 🔴 High |
| **F2-005** | `iron rollback` — restores last auto-snapshot (created before apply/update) | M | 🔴 High |
| **F2-006** | Per-module rollback: `iron rollback --module <id>` | M | 🟡 Medium |
| **F2-007** | TUI Snapshot timeline view: visual list of snapshots with restore action | M | 🟡 Medium |
| **F2-008** | Auto-snapshot before `iron apply`, `iron update`, `iron bundle switch` | S | 🔴 High |

### Sprint 2.2: Enhanced CLI Output

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F2-009** | Tree-style output renderer: `├──`, `└──`, `│` formatting | M | 🟡 Medium |
| **F2-010** | Operation summary blocks after every command | S | 🟡 Medium |
| **F2-011** | Table output for list commands (modules, packages, hosts) | M | 🟡 Medium |
| **F2-012** | Progress spinner for long operations (using indicatif or similar) | M | 🟡 Medium |
| **F2-013** | `--explain` mode shows actual commands: "Running: sudo pacman -S neovim" | S | 🟡 Medium |
| **F2-014** | Error messages include context, suggestions, and recovery hints | M | 🟡 Medium |

### Sprint 2.3: Config Validation & Security Levels

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F2-015** | Pre-apply config validation: check TOML syntax, required fields, path validity | M | 🟡 Medium |
| **F2-016** | Security level calculator: Basic/Standard/Advanced/Paranoid based on enabled security modules | M | 🟡 Medium |
| **F2-017** | `iron security status` — shows current level + recommendations to level up | S | 🟡 Medium |
| **F2-018** | Dashboard security indicator with level badge | S | 🟡 Medium |
| **F2-019** | Security modules (`modules/ufw`, `modules/fail2ban`, etc.) tagged with security levels | S | 🟢 Nice |

---

## 13. Phase 3: Ecosystem & Community (Sprint 9-10)

> **Goal:** Enable dotfile sharing, community modules, and advanced multi-host workflows.

### Sprint 3.1: Dotfile Import & Sharing

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F3-001** | `iron import <git-url>` — clone and parse a dotfile repo into importable modules | L | 🟡 Medium |
| **F3-002** | `iron import <url> --module <name>` — selective module import | M | 🟡 Medium |
| **F3-003** | `iron export --module <id>` — export a module as a standalone shareable package | M | 🟢 Nice |
| **F3-004** | Conflict preview before import: what files conflict, what packages are needed | M | 🟡 Medium |
| **F3-005** | `iron module create-from-scan` — auto-generate module from discovered config (from ScanService) | M | 🟡 Medium |

### Sprint 3.2: Advanced Multi-Host & Comparison

| Task | Description | Effort | Impact |
|------|-------------|--------|--------|
| **F3-006** | `iron host compare <host1> <host2>` — show differences between two host configs | M | 🟢 Nice |
| **F3-007** | `iron host provision <host-id>` — full provisioning from host definition | L | 🟡 Medium |
| **F3-008** | Remote apply via SSH (stretch goal): `iron apply --host notebook --remote` | XL | 🟢 Nice |
| **F3-009** | Config staging: test on one host before rolling to another | M | 🟢 Nice |
| **F3-010** | TUI host comparison side-by-side view | M | 🟢 Nice |

---

## 14. Simplification Strategy

### 14.1 Commands Users Should Know (80/20 Rule)

Reduce the "must learn" surface to 8 commands for newcomers and 12 for mid-level:

**Newcomer (8 commands):**
```
iron                    # Launch TUI (covers everything visually)
iron status             # Quick health check
iron apply              # Make system match declared state
iron update             # Safe system update
iron clean              # System cleanup
iron diff               # What's different from declared state?
iron snapshot create    # Save a restore point
iron rollback           # Go back to last good state
```

**Mid-level (adds 4 more):**
```
iron apply --module X   # Apply just one module
iron diff --adopt       # Incorporate changes into declared state
iron sync push/pull     # Git sync between machines
iron import <url>       # Import external dotfiles
```

Everything else is accessible via the TUI or `iron <command> --help`.

### 14.2 Configuration Simplification

**Current:** Users must understand 4 directory types (hosts/, bundles/, profiles/, modules/) + state.json + dormant/.

**Simplified mental model:**
1. **Host file** — "This is my machine and what I want on it" (single source of truth)
2. **Modules** — "These are my application configs" (the library)
3. **Profiles** — "These are curated module groups" (pre-built combinations)
4. **Bundles** — "These are desktop environments" (DE-specific packages + configs)

Users start with (1), everything else is optional composition.

### 14.3 TUI Simplification

**Reduce cognitive load on first encounter:**
- Dashboard panels: 6 → 4 (merge Maintenance into Quick Stats, merge Alerts into System Health)
- Navigation depth: Maximum 3 levels from Dashboard to any action
- Every view has a visible hint bar at the bottom with available actions
- Consistent [Esc] to go back, [?] for help, everywhere

---

## 15. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|:-----------:|:------:|------------|
| Template engine introduces complexity | Medium | Medium | Keep it dead-simple: `{{var}}` only, no conditionals/loops. Users who need more can use external tools. |
| `iron apply` is too slow for large systems | Low | High | Preview first (dry-run by default), then execute. Cache package queries. Parallel symlink creation. |
| Breaking changes to host.toml format | Medium | High | Make all new fields optional. Provide `iron migrate` command. Validate backward compatibility in tests. |
| Drift detection false positives | Medium | Medium | Conservative defaults: only flag clear divergence (missing packages, broken symlinks). Content drift requires opt-in `--deep` flag. |
| Feature creep delays Phase 1 | Medium | High | Phase 0 ships first. Each phase is independently valuable. Cut scope to ship on time. |
| TUI becomes too complex to maintain | Low | High | Consolidate views where possible. Shared components via `widgets/`. Strong separation between data (App) and rendering (ui/). |

---

## 16. Success Metrics

### Newcomer Success

| Metric | Target | How to Measure |
|--------|--------|----------------|
| First-run to working system | ≤ 10 minutes | Time from `iron` to all declared packages installed + configs linked |
| Learning curve | ≤ 5 minutes | Time to complete: status → scan → apply (guided by TUI) |
| Recovery time | ≤ 5 minutes | Time from "something broke" to `iron rollback` completing |
| "Do I understand what happened?" | Yes | Post-operation summary includes what changed + what to do next |

### Mid-Level Success

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Config change cycle | < 60 seconds | edit → diff → apply → verify for a single module |
| Cross-machine sync | < 5 minutes | Change on desktop → push → pull on laptop → apply |
| Drift detection | < 10 seconds | `iron diff` returns complete drift report |
| Module creation | < 5 minutes | From existing config to shareable module via wizard or `create-from-scan` |
| Experiment cycle | < 2 minutes | snapshot → try → rollback (or adopt) |

### Technical Metrics

| Metric | Target | Current |
|--------|--------|---------|
| `iron status` latency | < 1s | ~1s ✅ |
| `iron diff` latency | < 2s | N/A (not implemented) |
| `iron apply --dry-run` latency | < 3s | N/A (not implemented) |
| TUI startup | < 500ms | < 100ms ✅ |
| Binary size | < 10MB | ~2.9MB ✅ |
| Test count (target) | > 2,000 | 1,703 |
| Test coverage | > 70% | ~64% |

---

## 17. Final Verdict

### What Iron IS

Iron is a **remarkably well-engineered** system management tool with:
- A clean, layered Rust architecture that's a joy to work in
- A thoughtful domain model that maps to how users think about their systems
- The best safe-update workflow I've seen in any Arch tool
- A TUI that's already more capable than most competing tools' CLIs
- Strong foundations in testing, error handling, and resilience

### What Iron NEEDS to Become

Iron needs to close the loop from **"a collection of features"** to **"a cohesive workflow."** Specifically:

1. **`iron apply`** — The single command that makes the declarative promise real
2. **`iron diff`** — The preview that builds trust before action
3. **Host as source of truth** — The file that says "this is what my machine should be"
4. **Template variables** — The mechanism that makes one config work across machines
5. **Named snapshots** — The safety net that enables fearless experimentation
6. **Structured output** — The feedback that teaches users what happened and why

These six features transform Iron from "a good tool" into "the definitive Arch management platform."

### The Path Forward

```
TODAY                          PHASE 0              PHASE 1              PHASE 2+
─────                          ───────              ───────              ──────
Feature-complete               Friction-free        Workflow-complete    Community-ready
but workflow-fragmented        entry point          apply/diff/drift     import/share/scale
                               
iron go → TUI          →      iron → TUI           iron apply    →      iron import <url>
iron module enable X   →      Tech debt closed      iron diff            iron export --module
iron bundle install Y  →      Dashboard polished    Host as truth        Named snapshots
Manual orchestration   →      Summary output        Templates            Rollback timeline
                                                    Full drift detection Security levels
```

**Iron is 80% of the way to extraordinary. The remaining 20% is the most important 20%.**

Let's build it.

---

*This document serves as the canonical product roadmap for Iron-Arch development. Each phase should be broken into sprint-level implementation plans before development begins. All task IDs (F0-xxx, F1-xxx, etc.) are stable references for tracking.*
