# Iron Project - Requirements Specification v1.0

> **Document Status**: APPROVED
> **Created**: 2025-02-12
> **Last Updated**: 2025-02-12
> **Author**: Requirements Discovery Session

---

## Executive Summary

**Iron** is a declarative Arch Linux configuration management platform that transforms system administration from a complex, error-prone process into an elegant, safe, and user-friendly experience.

### Vision Statement

> "Less is More - Turning your Arch into Iron"

Iron empowers users of all experience levels to:
- Manage system configurations with confidence
- Update safely with proactive breaking-change detection
- Reproduce their exact system on any machine
- Switch between desktop environments seamlessly

### Target Users

| Priority | User Type | Description |
|----------|-----------|-------------|
| PRIMARY | Developer/Power User | Wants Arch benefits without maintenance burden |
| SECONDARY | Linux Migrant | Coming from Ubuntu/Fedora, ready for Arch with guardrails |
| TERTIARY | Complete Newcomer | Not primary target, but UX must be accessible to them |

---

## System Architecture

### Hierarchy Model

```
HOST (Hardware + System)
  └── BUNDLE (Desktop Environment)
        └── PROFILE (Dotfile Collection)
              └── MODULE (Individual Component)
```

### Relationships

| Entity | Contains | Cardinality | Notes |
|--------|----------|-------------|-------|
| Host | Bundles | 1:N | Only 1 active at a time |
| Bundle | Profiles | 1:N | User selects active |
| Profile | Modules | 1:N | Modules can be shared |
| Module | Dotfiles + Packages | 1:N | Atomic unit |

### Bundle State Management

```
INSTALLED: Packages present on system
ACTIVE: Configs linked to ~/.config (only ONE)
DORMANT: Configs stored in iron/dormant/ (unlinked)
```

---

## Functional Requirements

### FR-1: Host Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-1.1 | Catalog host hardware | HIGH | CPU, GPU, RAM, monitors stored in host.toml |
| FR-1.2 | Store Arch install parameters | HIGH | Partition scheme, bootloader, drivers recorded |
| FR-1.3 | Support multiple hosts | HIGH | Single repo manages N machines |
| FR-1.4 | Auto-detect current host | MEDIUM | Match by hostname or hardware fingerprint |
| FR-1.5 | Alert if no snapshot exists | HIGH | TUI shows warning badge on dashboard |

### FR-2: Bundle Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-2.1 | Bundle = DE + core packages | HIGH | Hyprland bundle includes compositor, bar, launcher |
| FR-2.2 | Single active bundle | HIGH | Attempting second activation prompts switch |
| FR-2.3 | Multiple installed bundles | HIGH | Packages for N bundles coexist |
| FR-2.4 | Safe bundle switch | HIGH | Snapshot created before switch, rollback available |
| FR-2.5 | Conflict detection | HIGH | Warn if bundle A conflicts with bundle B packages |
| FR-2.6 | Dormant config storage | HIGH | Inactive bundle configs stored in iron/dormant/ |

### FR-3: Profile Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-3.1 | Profile = module collection | HIGH | Profile references N modules by ID |
| FR-3.2 | Switch profiles in bundle | HIGH | Change from "minimal" to "developer" without reinstall |
| FR-3.3 | Shared modules | HIGH | Module "nvim-config" usable in multiple profiles |
| FR-3.4 | Dotfile linking | HIGH | Profile activation creates symlinks to ~/.config |
| FR-3.5 | Smart merge | MEDIUM | Overlapping targets prompt user choice |
| FR-3.6 | TUI Profile Builder | HIGH | User creates profile by selecting modules visually |
| FR-3.7 | Mixed module sourcing | HIGH | Use waybar from Profile A, kitty from Profile B |

### FR-4: Module Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-4.1 | Module = packages + dotfiles + hooks | HIGH | Single module.toml defines all three |
| FR-4.2 | Independent toggle | HIGH | Enable/disable module without affecting others |
| FR-4.3 | Conflict detection | HIGH | Two modules targeting same path warned |
| FR-4.4 | Pre/post hooks | HIGH | Scripts run at install/uninstall |
| FR-4.5 | Module versioning | MEDIUM | Track module version for updates |

### FR-5: Update & Safety

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-5.1 | Arch News integration | HIGH | Fetch and display relevant news before update |
| FR-5.2 | AUR flagged packages | HIGH | Warn if installed AUR packages are flagged |
| FR-5.3 | Dependency conflict detection | HIGH | Predict conflicts before pacman runs |
| FR-5.4 | Risk score calculation | HIGH | Display LOW/MEDIUM/HIGH based on changes |
| FR-5.5 | Approval workflow | HIGH | MEDIUM/HIGH risk requires explicit confirmation |
| FR-5.6 | Auto-snapshot | HIGH | Timeshift/snapper snapshot before any update |
| FR-5.7 | Pacnew handling | MEDIUM | Detect, diff, and merge .pacnew files |
| FR-5.8 | Update preview | HIGH | Show what will change before proceeding |

### FR-6: Recovery

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-6.1 | Arch install script generation | HIGH | Generate install.sh from host config |
| FR-6.2 | Complete state export | HIGH | All configs, packages, services to git |
| FR-6.3 | 4-step recovery flow | HIGH | Install → Bundle → Profile → Verify |
| FR-6.4 | Post-install verification | HIGH | Script checks drivers, services, permissions |
| FR-6.5 | Recovery time target | HIGH | Full system restore < 30 minutes |

### FR-7: Git Sync

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-7.1 | Push to remote | HIGH | `iron sync push` commits and pushes |
| FR-7.2 | Pull from remote | HIGH | `iron sync pull` fetches and applies |
| FR-7.3 | Local change detection | HIGH | Warn if uncommitted changes before pull |
| FR-7.4 | Conflict resolution | MEDIUM | Interactive merge for config conflicts |
| FR-7.5 | Multi-machine sync | HIGH | Same repo works on desktop + laptop |

### FR-8: Secrets Management

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-8.1 | Encrypted storage | HIGH | Secrets encrypted with git-crypt or age |
| FR-8.2 | SSH key management | HIGH | Store and link SSH keys |
| FR-8.3 | GPG key management | HIGH | Store and link GPG keys |
| FR-8.4 | Token management | HIGH | Store API tokens securely |
| FR-8.5 | Unlock workflow | HIGH | `iron secrets unlock` decrypts on clone |
| FR-8.6 | Link workflow | HIGH | `iron secrets link` symlinks to proper locations |

### FR-9: TUI Experience

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-9.1 | Dashboard home | HIGH | System health, active bundle/profile, alerts |
| FR-9.2 | First-time wizard | HIGH | Guided setup for new installations |
| FR-9.3 | Bundle/Profile wizard | HIGH | Visual selection with descriptions |
| FR-9.4 | Profile Builder | HIGH | Create custom profile by selecting modules |
| FR-9.5 | Visual diff | HIGH | Show changes before applying |
| FR-9.6 | Keyboard navigation | HIGH | Arrow keys, vim bindings optional |
| FR-9.7 | Newcomer accessible | HIGH | 5-minute learning curve |
| FR-9.8 | Pre-update screen | HIGH | Risk score + changes + approval buttons |

---

## Non-Functional Requirements

| ID | Requirement | Target | Measurement |
|----|-------------|--------|-------------|
| NFR-1 | TUI response time | < 100ms | Time from keypress to screen update |
| NFR-2 | Risk calculation | < 5s | Time to compute update risk score |
| NFR-3 | Snapshot creation | < 30s | Timeshift/snapper snapshot duration |
| NFR-4 | Single binary | Yes | No runtime dependencies except system tools |
| NFR-5 | Offline capability | Full | All features except git sync work offline |
| NFR-6 | Learning curve | 5 min | Time for newcomer to perform basic tasks |
| NFR-7 | Recovery time | < 30 min | Full system restore from scratch |

---

## User Stories

### US-1: First-Time Setup
```gherkin
AS A developer new to Arch
I WANT a guided setup wizard
SO THAT I can configure my system without reading wikis

GIVEN I have a fresh Arch installation
WHEN I run `iron` for the first time
THEN I see a welcome wizard that guides me through:
  - Detecting my hardware
  - Choosing a bundle (Hyprland/Niri/KDE)
  - Selecting a profile (Minimal/Developer/Gaming)
  - Applying configurations
AND my system is ready to use in under 10 minutes
```

### US-2: Safe Updates
```gherkin
AS A user afraid of breaking updates
I WANT to see risk scores before updating
SO THAT I can make informed decisions

GIVEN there are system updates available
WHEN I run `iron update`
THEN I see a pre-update screen showing:
  - Risk score (LOW/MEDIUM/HIGH)
  - Arch News alerts if relevant
  - List of packages to update
  - Any potential conflicts
AND I must explicitly approve before proceeding
AND a snapshot is created automatically
```

### US-3: Multi-Machine Sync
```gherkin
AS A user with multiple machines
I WANT my configs synced via git
SO THAT all my machines stay consistent

GIVEN I have Iron configured on my desktop
WHEN I run `iron sync push` on desktop
AND run `iron sync pull` on laptop
THEN my laptop has the same configurations
AND host-specific settings are preserved
```

### US-4: Disaster Recovery
```gherkin
AS A user whose PC just died
I WANT to restore my exact setup
SO THAT I can continue working within 30 minutes

GIVEN I have my Iron git repo
WHEN I install fresh Arch and clone my repo
THEN `iron recover` guides me through:
  - Step 1: Core system installation
  - Step 2: Bundle installation
  - Step 3: Profile selection
  - Step 4: Post-install verification
AND my system is identical to before
```

### US-5: Environment Switch
```gherkin
AS A user who wants to try Hyprland
I WANT to switch from Niri safely
SO THAT I can always go back if I don't like it

GIVEN I have Niri as my active bundle
WHEN I run `iron bundle switch hyprland`
THEN Iron:
  - Creates a snapshot
  - Stores Niri configs in dormant/
  - Links Hyprland configs
  - Confirms successful switch
AND I can switch back with `iron bundle switch niri`
```

### US-6: Custom Profile Creation
```gherkin
AS A power user
I WANT to create my own profile
SO THAT I have exactly the configs I want

GIVEN I am in the Iron TUI
WHEN I select "Create New Profile"
THEN I can:
  - Name my profile
  - Browse available modules
  - Select modules to include
  - Preview the result
  - Save and activate
AND my custom profile is usable immediately
```

---

## Technical Constraints

### Platform
- **Target OS**: Arch Linux and derivatives
- **Language**: Rust (core) + Bash (operations)
- **TUI Framework**: Ratatui
- **Config Format**: TOML

### Dependencies
- **Required**: pacman, systemd, stow, git
- **Optional**: paru/yay (AUR), timeshift/snapper (backups), git-crypt/age (secrets)

### Compatibility
- **Display Servers**: Wayland (primary), X11 (secondary)
- **Architectures**: x86_64 (primary), aarch64 (secondary)

---

## Glossary

| Term | Definition |
|------|------------|
| **Bundle** | Desktop environment + core packages (e.g., Hyprland bundle) |
| **Profile** | Collection of modules representing a dotfile set |
| **Module** | Atomic unit containing packages + dotfiles + hooks |
| **Host** | Physical or virtual machine with unique hardware |
| **Dormant** | Bundle state where configs are stored but not linked |
| **Active** | Bundle/profile state where configs are symlinked |

---

## Appendix A: Migration from jff-arch-config

The following components will be migrated:

| Source | Destination | Action |
|--------|-------------|--------|
| rust/crates/core-domain | iron/crates/iron-core | Refactor |
| rust/crates/app-cli | iron/crates/iron-cli | Refactor |
| rust/crates/app-tui | iron/crates/iron-tui | Redesign |
| modules/ | iron/modules/ | Restructure |
| hosts/ | iron/hosts/ | Extend |
| scripts/ | iron/scripts/ | Migrate |

---

## Approval

This requirements specification has been reviewed and approved through the interactive brainstorming session.

**Next Steps:**
1. `/sc:design` - Create technical architecture
2. `/sc:workflow` - Generate implementation plan
