# Phase 2 — Technical Implementation Guide

> **Phase:** 2 — Power User Features
> **Status:** 🟡 PLANNING
> **Depends On:** Phase 1 ✅ Complete (2026-02-22)
> **Companion:** [`phase2-kanban.md`](phase2-kanban.md)

---

## 1. Architecture Overview

Phase 2 adds three capabilities on top of the Phase 1 foundation:

```
Phase 2 Layer Cake:
┌─────────────────────────────────────────────┐
│  Sprint 2.3: Validation + Security Levels   │  ← Pre-apply safety + security posture
├─────────────────────────────────────────────┤
│  Sprint 2.2: Enhanced CLI Output            │  ← UX polish (trees, tables, spinners)
├─────────────────────────────────────────────┤
│  Sprint 2.1: Snapshot & Rollback            │  ← Safety net for experimentation
├─────────────────────────────────────────────┤
│  Phase 1: Apply + Diff + Templates          │  ← Declarative convergence (DONE)
├─────────────────────────────────────────────┤
│  Phase 0: Foundation Fixes                  │  ← UX basics + tech debt (DONE)
└─────────────────────────────────────────────┘
```

### New Files Created in Phase 2

```
iron-core/src/services/snapshot_service.rs   ← SnapshotService, SnapshotRecord
iron-core/src/services/security.rs           ← SecurityService, SecurityLevel
iron-cli/src/commands/snapshot.rs            ← CLI snapshot/rollback commands
iron-cli/src/commands/security.rs            ← CLI security status
iron-cli/src/progress.rs                     ← Progress spinner/bar
iron-tui/src/ui/snapshot.rs                  ← TUI snapshot timeline view
```

### Modified Files

```
iron-core/src/services/mod.rs               ← Register snapshot_service, security
iron-core/src/services/apply.rs             ← Auto-snapshot before execute
iron-core/src/services/update.rs            ← Auto-snapshot before update
iron-core/src/module.rs                     ← security_points field
iron-core/src/error.rs                      ← suggestion() method
iron-cli/src/cli.rs                         ← Snapshot, Security commands
iron-cli/src/commands/mod.rs                ← Register new command modules
iron-cli/src/main.rs                        ← Wire new commands
iron-cli/src/output.rs                      ← Tree, table, summary_block methods
iron-cli/Cargo.toml                         ← indicatif dependency
iron-tui/src/app/mod.rs                     ← View::Snapshots, security_level, snapshot_list fields
iron-tui/src/app/handlers.rs                ← Snapshot keybindings, cycle_view arms
iron-tui/src/ui/mod.rs                      ← Register snapshot view
iron-tui/src/ui/dashboard.rs                ← Security level indicator
iron-tui/src/widgets/mod.rs                 ← View::Snapshots in ALL 4 match arms
```

---

## 2. Sprint 2.1: Snapshot & Rollback

### 2.1.1 SnapshotRecord Model (F2-001)

```rust
// iron-core/src/services/snapshot_service.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A snapshot captures system state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    /// Unique ID (UUID)
    pub id: String,

    /// Human-readable name ("pre-kde", "backup-2026-02-22")
    pub name: String,

    /// When the snapshot was created
    pub timestamp: DateTime<Utc>,

    /// Host this snapshot belongs to
    #[serde(default)]
    pub host_id: Option<String>,

    /// Active bundle at snapshot time
    #[serde(default)]
    pub active_bundle: Option<String>,

    /// Active profile at snapshot time
    #[serde(default)]
    pub active_profile: Option<String>,

    /// All active module IDs at snapshot time
    #[serde(default)]
    pub active_modules: Vec<String>,

    /// Installed packages (from pacman -Qe)
    #[serde(default)]
    pub explicit_packages: Vec<String>,

    /// Dotfile checksums: target_path → sha256
    #[serde(default)]
    pub dotfile_checksums: HashMap<String, String>,

    /// Whether this was auto-created (vs user-created)
    #[serde(default)]
    pub auto: bool,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}
```

### 2.1.2 SnapshotService Trait (F2-001)

```rust
pub trait SnapshotService {
    /// Create a named snapshot of current state
    fn create(&self, name: &str, description: Option<&str>) -> IronResult<SnapshotRecord>;

    /// Create auto-snapshot (named with timestamp, auto=true)
    fn create_auto(&self, prefix: &str) -> IronResult<SnapshotRecord>;

    /// List all snapshots, newest first
    fn list(&self) -> IronResult<Vec<SnapshotRecord>>;

    /// Get a specific snapshot by name or ID
    fn get(&self, name_or_id: &str) -> IronResult<SnapshotRecord>;

    /// Delete a snapshot
    fn delete(&self, name_or_id: &str) -> IronResult<()>;

    /// Prune old auto-snapshots, keeping at most `keep` recent ones
    fn prune_auto(&self, keep: usize) -> IronResult<usize>;
}
```

### 2.1.3 Storage Layout

```
$IRON_ROOT/.snapshots/
├── index.json              ← Vec<SnapshotRecord> (lightweight index)
├── snap-abc123.json        ← Full snapshot data
├── snap-def456.json
└── ...
```

The index file contains metadata only (fast listing). Each snapshot file contains the full record including package lists and checksums.

### 2.1.4 Restore Flow (F2-004)

```
iron snapshot restore "pre-kde"
  │
  ├── 1. Load snapshot record
  ├── 2. Auto-snapshot current state ("pre-restore-{timestamp}")
  ├── 3. Build DesiredState from snapshot data
  │      (modules, packages, dotfile targets)
  ├── 4. Compute ApplyPlan (diff snapshot vs current)
  ├── 5. Show plan + require confirmation
  └── 6. Execute plan via ApplyService
```

### 2.1.5 Auto-Snapshot Integration (F2-008)

```rust
// In iron-core/src/services/apply.rs

impl ApplyService for DefaultApplyService {
    fn execute(&self, plan: &ApplyPlan) -> IronResult<ApplyResult> {
        // F2-008: Auto-snapshot before destructive operations
        if !plan.is_empty() {
            if let Ok(snapshot_svc) = self.snapshot_service() {
                let _ = snapshot_svc.create_auto("pre-apply");
                let _ = snapshot_svc.prune_auto(10);
            }
        }

        // ... existing execute logic ...
    }
}
```

### 2.1.6 TUI View (F2-007)

Add to `iron-tui/src/app/mod.rs`:
```rust
pub enum View {
    // ...existing variants...
    /// Snapshot timeline view
    Snapshots,
}

pub struct App {
    // ...existing fields...
    /// F2-007: Cached snapshot list for TUI display
    pub snapshot_list: Vec<iron_core::services::snapshot_service::SnapshotRecord>,
}
```

**CRITICAL REMINDER:** Adding a View variant requires updating ALL exhaustive matches:
1. `render()` in `ui/mod.rs`
2. `render_header()` in `widgets/mod.rs` (view name + icon)
3. `render_footer()` in `widgets/mod.rs` (keybindings)
4. `get_view_keybindings()` in `widgets/mod.rs` (help overlay)
5. `cycle_view_forward()` in `handlers.rs`
6. `cycle_view_backward()` in `handlers.rs`
7. `test_view_names()` in `widgets/mod.rs` (test)

---

## 3. Sprint 2.2: Enhanced CLI Output

### 3.1 Tree Renderer (F2-009)

```rust
// In iron-cli/src/output.rs

impl Output {
    /// Render a tree root item
    pub fn tree_root(&self, label: &str) { ... }

    /// Render a tree branch (not last child)
    pub fn tree_branch(&self, label: &str, depth: usize) { ... }

    /// Render the last child in a tree level
    pub fn tree_last(&self, label: &str, depth: usize) { ... }

    /// Render a tree continuation line (for multi-line items)
    pub fn tree_indent(&self, depth: usize) { ... }
}
```

Example output:
```
📦 Apply Plan
├── Install packages (3)
│   ├── neovim
│   ├── fish
│   └── ripgrep
├── Create symlinks (2)
│   ├── ~/.config/nvim → modules/nvim-ide/config
│   └── ~/.config/fish → modules/fish/config
└── Enable services (1)
    └── pipewire.service
```

### 3.2 Table Output (F2-011)

```rust
impl Output {
    /// Render a table with headers and rows
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) { ... }
}
```

Uses column-width calculation based on content. Right-aligns numeric columns.

### 3.3 Progress Spinner (F2-012)

Add `indicatif = "0.17"` to `iron-cli/Cargo.toml`.

```rust
// iron-cli/src/progress.rs

pub struct ProgressReporter {
    spinner: Option<ProgressBar>,
    quiet: bool,
}

impl ProgressReporter {
    pub fn spinner(msg: &str) -> Self { ... }
    pub fn bar(total: u64, msg: &str) -> Self { ... }
    pub fn tick(&self, msg: &str) { ... }
    pub fn finish(&self, msg: &str) { ... }
}
```

---

## 4. Sprint 2.3: Config Validation & Security Levels

### 4.1 Pre-Apply Validation (F2-015)

Add to `ApplyService`:
```rust
pub trait ApplyService {
    // ...existing methods...

    /// F2-015: Validate config before apply
    fn validate(&self, host_id: &str) -> IronResult<Vec<ValidationWarning>>;
}
```

Validation checks:
- All referenced bundles exist in `bundles/`
- All referenced profiles exist in `profiles/`
- All referenced modules exist in `modules/`
- No circular module dependencies
- All dotfile source paths exist
- TOML syntax is valid for all referenced configs
- No duplicate package declarations

### 4.2 Security Level (F2-016)

```rust
// iron-core/src/services/security.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SecurityLevel {
    Basic,      // 0-20 points
    Standard,   // 21-50 points
    Advanced,   // 51-80 points
    Paranoid,   // 81+ points
}

pub struct SecurityReport {
    pub level: SecurityLevel,
    pub score: u32,
    pub max_score: u32,
    pub enabled_modules: Vec<SecurityModuleInfo>,
    pub recommendations: Vec<String>,
}

pub trait SecurityService {
    fn calculate(&self) -> IronResult<SecurityReport>;
}
```

Default point values:
| Module | Points |
|--------|--------|
| ufw / firewalld | 10 |
| fail2ban | 10 |
| ssh-hardening | 10 |
| apparmor / sandboxing | 15 |
| audit (auditd) | 10 |
| intrusion-detection | 15 |
| kernel-hardening | 15 |
| password-policy | 5 |
| dns-security | 10 |

### 4.3 Module Security Points (F2-019)

```rust
// In iron-core/src/module.rs
pub struct Module {
    // ...existing fields...

    /// F2-019: Security hardening points this module contributes
    #[serde(default)]
    pub security_points: u32,
}
```

---

## 5. Testing Strategy

### Unit Tests Per Feature

| Feature | Test Count | Key Tests |
|---------|-----------|-----------|
| F2-001 | 8+ | create/list/restore/delete roundtrip, auto flag, prune |
| F2-002-005 | 6+ | CLI parsing tests for all snapshot subcommands |
| F2-007 | 4+ | TUI render no-panic tests, snapshot list display |
| F2-008 | 3+ | auto-snapshot creation before apply/update |
| F2-009-011 | 6+ | tree/table formatting, edge cases (empty, long strings) |
| F2-012 | 2+ | spinner creation, non-TTY fallback |
| F2-015 | 5+ | validation catches: missing bundle, missing module, circular deps |
| F2-016 | 4+ | level thresholds: Basic/Standard/Advanced/Paranoid |

### Integration Tests

- `iron snapshot create test-snap --dry-run` → exits 0
- `iron snapshot list` → exits 0 (empty list)
- `iron security status` → exits 0
- `iron validate` → exits 0 on valid config
- All commands with `--json` flag produce valid JSON

### Anti-Patterns to Avoid

1. **Never** create real btrfs/timeshift snapshots in tests
2. **Never** run `pacman -S` in tests (always `--dry-run` or mocked)
3. **Never** hardcode snapshot counts in assertions (use `>= N`)
4. **Always** add `--dry-run` flags to all new commands
5. **Always** update doctor check count test if adding checks
6. **Always** update ALL View match arms when adding TUI views

---

## 6. Implementation Order

Recommended order within each sprint:

### Sprint 2.1 (Snapshot):
1. F2-001 (SnapshotRecord + SnapshotService) ← foundation
2. F2-002 (CLI create) ← first user-facing feature
3. F2-003 (CLI list) ← verification
4. F2-008 (auto-snapshot) ← safety integration
5. F2-004 (CLI restore) ← core value
6. F2-005 (rollback) ← shortcut for restore
7. F2-006 (per-module rollback) ← refinement
8. F2-007 (TUI view) ← last, needs stable API

### Sprint 2.2 (CLI Output):
1. F2-009 (tree renderer) ← used by all subsequent
2. F2-010 (summary blocks) ← standardization
3. F2-011 (table output) ← list commands
4. F2-012 (progress spinner) ← independent
5. F2-013 (explain enhancement) ← uses tree renderer
6. F2-014 (error suggestions) ← independent

### Sprint 2.3 (Validation + Security):
1. F2-019 (module security_points field) ← struct change first
2. F2-016 (security level calculator) ← core logic
3. F2-017 (CLI security status) ← user-facing
4. F2-018 (TUI dashboard indicator) ← visual
5. F2-015 (pre-apply validation) ← integrates with apply

---

## 7. Rollback Safety Guarantees

The snapshot system provides the following safety properties:

1. **Every destructive operation auto-snapshots first** (apply, update, bundle switch)
2. **Snapshots are lightweight** (metadata + checksums, not full file copies)
3. **Restore uses the existing ApplyService** (same convergence logic, same dry-run support)
4. **Auto-snapshots are pruned** (max 10 by default, configurable)
5. **Manual snapshots are never auto-pruned** (user must explicitly delete)
6. **Snapshot restore creates its own snapshot first** (so you can undo an undo)
