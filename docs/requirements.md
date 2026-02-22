# Requirements

## Overview

Iron is a declarative Arch Linux configuration management platform. It provides a single binary
with a TUI and CLI interface that lets users manage host hardware profiles, desktop environment
bundles, dotfile profiles, and individual application modules. The system prioritizes safety
(risk-scored updates, automatic snapshots, rollback) and accessibility (5-minute learning curve,
guided wizards) over raw flexibility.

## Functional Requirements

### FR-1: Host Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1.1 | Catalog host hardware (CPU, GPU, RAM, monitors) in `host.toml` | HIGH |
| FR-1.2 | Store Arch install parameters (partition scheme, bootloader, drivers) | HIGH |
| FR-1.3 | Support multiple hosts in a single repo | HIGH |
| FR-1.4 | Auto-detect current host by hostname or hardware fingerprint | MEDIUM |
| FR-1.5 | Display warning badge in TUI when no snapshot exists for current host | HIGH |

### FR-2: Bundle Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-2.1 | Bundle defines a desktop environment with core packages, services, and dotfiles | HIGH |
| FR-2.2 | Only one bundle may be active at a time; attempting a second activation prompts a switch | HIGH |
| FR-2.3 | Multiple bundles may be installed (packages coexist) with only one active | HIGH |
| FR-2.4 | Bundle switch creates a snapshot before proceeding and offers rollback on failure | HIGH |
| FR-2.5 | Detect and warn if bundle A conflicts with bundle B packages | HIGH |
| FR-2.6 | Store inactive bundle configs in `dormant/` (unlinked but versioned) | HIGH |

### FR-3: Profile Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-3.1 | Profile is a named collection of module references | HIGH |
| FR-3.2 | Switch profiles within a bundle without reinstalling packages | HIGH |
| FR-3.3 | Modules may be shared across multiple profiles | HIGH |
| FR-3.4 | Profile activation creates symlinks to `~/.config` via stow | HIGH |
| FR-3.5 | Overlapping symlink targets prompt user choice (smart merge) | MEDIUM |
| FR-3.6 | TUI profile builder lets users create profiles by selecting modules visually | HIGH |
| FR-3.7 | Profiles may borrow individual modules from other profiles | HIGH |

### FR-4: Module Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-4.1 | Module is defined by a single `module.toml` containing packages, dotfiles, and hooks | HIGH |
| FR-4.2 | Enable or disable individual modules without affecting others | HIGH |
| FR-4.3 | Warn when two modules target the same config path | HIGH |
| FR-4.4 | Execute pre/post hooks at install and uninstall time | HIGH |
| FR-4.5 | Track module version for update awareness | MEDIUM |

### FR-5: Update & Safety

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-5.1 | Fetch and display Arch News items relevant to pending updates | HIGH |
| FR-5.2 | Warn if any installed AUR packages are flagged out-of-date | HIGH |
| FR-5.3 | Predict dependency conflicts before pacman executes | HIGH |
| FR-5.4 | Calculate and display risk score: LOW / MEDIUM / HIGH / CRITICAL | HIGH |
| FR-5.4.1 | Risk thresholds: LOW = 0–2 minor changes; MEDIUM = 3–5 changes or config updates; HIGH = 6–10 changes or driver updates; CRITICAL = kernel/bootloader/glibc changes | HIGH |
| FR-5.5 | MEDIUM/HIGH risk requires explicit confirmation; CRITICAL requires typed confirmation | HIGH |
| FR-5.6 | Create a timeshift or snapper snapshot automatically before any update | HIGH |
| FR-5.7 | Detect, diff, and merge `.pacnew` files with options: keep-new, keep-old, interactive | MEDIUM |
| FR-5.8 | Show a preview of all package changes before proceeding | HIGH |
| FR-5.9 | All external commands (pacman, git, systemctl) time out after 120s with `RetryableError` | HIGH |
| FR-5.10 | Track update progress; if interrupted, resume from last successful package on next run | HIGH |

### FR-6: Recovery

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-6.1 | Generate `install.sh` from host config for bare-metal reinstall | HIGH |
| FR-6.2 | Export all configs, packages, and services to git | HIGH |
| FR-6.3 | `iron recover` runs a 4-step flow: Install → Bundle → Profile → Verify | HIGH |
| FR-6.4 | Post-install verification checks drivers, services, and permissions | HIGH |
| FR-6.5 | Full system restore from a cloned repo completes in < 30 minutes | HIGH |

### FR-7: Git Sync

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-7.1 | `iron sync push` commits all config state and pushes to remote | HIGH |
| FR-7.2 | `iron sync pull` fetches from remote and applies config changes | HIGH |
| FR-7.3 | Warn if uncommitted local changes exist before pulling | HIGH |
| FR-7.4 | Provide interactive merge for config conflicts on pull | MEDIUM |
| FR-7.5 | Same repo operates across multiple machines with host-specific isolation | HIGH |

### FR-8: Secrets Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-8.1 | Encrypt secrets at rest using git-crypt or age | HIGH |
| FR-8.2 | Store and symlink SSH keys | HIGH |
| FR-8.3 | Store and symlink GPG keys | HIGH |
| FR-8.4 | Store API tokens securely | HIGH |
| FR-8.5 | `iron secrets unlock` decrypts secrets after a fresh clone | HIGH |
| FR-8.6 | `iron secrets link` symlinks decrypted secrets to their target locations | HIGH |

### FR-9: TUI Experience

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-9.1 | Dashboard home shows system health, active bundle/profile, and alerts | HIGH |
| FR-9.2 | First-run wizard guides new installations step by step | HIGH |
| FR-9.3 | Bundle/profile wizard provides visual selection with descriptions | HIGH |
| FR-9.4 | Profile builder lets users compose a profile by selecting modules | HIGH |
| FR-9.5 | Visual diff shows all changes before applying | HIGH |
| FR-9.6 | Arrow-key and vim-style keyboard navigation | HIGH |
| FR-9.7 | A newcomer can complete basic tasks in ≤ 5 minutes | HIGH |
| FR-9.8 | Pre-update screen shows risk score, change list, Arch News, and approval controls | HIGH |

### FR-10: Health Check & Diagnostics

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-10.1 | `iron doctor` validates `state.json` is parseable and schema-compliant | HIGH |
| FR-10.2 | Verify all active bundle/profile symlinks point to valid targets | HIGH |
| FR-10.3 | Verify all required packages from the active bundle/profile are installed | HIGH |
| FR-10.4 | Verify timeshift or snapper is available and configured | HIGH |
| FR-10.5 | Verify `bundles/`, `profiles/`, `modules/`, `hosts/` directories exist | HIGH |
| FR-10.6 | Verify `.git` exists and warn of uncommitted changes | MEDIUM |
| FR-10.7 | Report git-crypt/age status and warn if secrets are locked | MEDIUM |
| FR-10.8 | Output a structured JSON report with pass/warn/fail status per check | HIGH |

---

## Non-Functional Requirements

| ID | Requirement | Target | Measurement |
|----|-------------|--------|-------------|
| NFR-1 | TUI response time | < 100ms | Time from keypress to screen update |
| NFR-2 | Risk calculation | < 5s | Time to compute update risk score |
| NFR-3 | Snapshot creation | < 30s | Timeshift/snapper snapshot duration |
| NFR-4 | Single binary | Yes | No runtime dependencies except system tools |
| NFR-5 | Offline capability | Full | All features except git sync work offline |
| NFR-6 | Learning curve | 5 min | Time for newcomer to complete basic tasks |
| NFR-7 | Recovery time | < 30 min | Full system restore from scratch |
| NFR-8 | Command timeout | 120s | External command execution timeout via circuit breaker |
| NFR-9 | Structured logging | JSON | Logs include timestamp, level, component, and message |
| NFR-10 | Log rotation | 10MB / 5 files | Rotate when file exceeds 10MB; keep 5 files |
| NFR-11 | Graceful degradation | Required | System remains usable when optional components fail (secrets, sync) |

---

## Constraints

### Platform
- **Target OS**: Arch Linux and derivatives
- **Language**: Rust (core) + Bash (install scripts and hooks)
- **TUI framework**: Ratatui + crossterm
- **Config format**: TOML

### Required Tools
- `pacman`, `systemd`, `stow`, `git`

### Optional Tools
- `paru` or `yay` (AUR support)
- `timeshift` or `snapper` (snapshot/backup support)
- `git-crypt` or `age` (secrets encryption)

### Compatibility
- **Display servers**: Wayland (primary), X11 (secondary)
- **Architectures**: x86_64 (primary), aarch64 (secondary)

---

## Acceptance Criteria

| FR Group | Key Acceptance Criteria |
|----------|------------------------|
| FR-1: Host | CPU, GPU, RAM, monitors stored in `host.toml`; multiple hosts managed from a single repo |
| FR-2: Bundle | Only one bundle active; dormant configs stored in `dormant/`; snapshot created before switch |
| FR-3: Profile | Profile activation creates symlinks to `~/.config`; modules shared across profiles |
| FR-4: Module | Single `module.toml` defines packages + dotfiles + hooks; independent enable/disable |
| FR-5: Update | Risk score displayed before any update; CRITICAL requires typed confirmation; 120s timeout |
| FR-6: Recovery | `iron recover` completes full restore in < 30 minutes from a cloned repo |
| FR-7: Sync | `iron sync push/pull` works across multiple machines; uncommitted changes warned |
| FR-8: Secrets | Secrets encrypted at rest; `unlock` and `link` commands functional after fresh clone |
| FR-9: TUI | Dashboard visible in < 100ms; newcomer completes basic task in ≤ 5 minutes |
| FR-10: Doctor | JSON health report with pass/warn/fail per check; all symlinks and packages verified |
