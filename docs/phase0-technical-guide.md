# Phase 0 — Technical Implementation Guide

> **Phase:** 0 — Foundation Fixes
> **Sprints:** 0.1 (UX Quick Wins) + 0.2 (Tech Debt Closure)
> **Audience:** Implementing engineers
> **Prerequisites:** Read `docs/product-review-and-roadmap.md`, `CLAUDE.md`
>
> This document provides exact file locations, code patterns, data flows, and implementation guidance for every Phase 0 task. Each section includes the codebase analysis that informed the task, the product requirement it satisfies, and the precise changes needed.

---

## Table of Contents

1. [Architecture Context](#1-architecture-context)
2. [Sprint 0.1: F0-001 — TUI as Default](#2-f0-001)
3. [Sprint 0.1: F0-002 — Dashboard Sync + Disk](#3-f0-002)
4. [Sprint 0.1: F0-003 — Disk Space in Doctor](#4-f0-003)
5. [Sprint 0.1: F0-004 — Getting Started Hints](#5-f0-004)
6. [Sprint 0.1: F0-005 — CLI Summary Lines](#6-f0-005)
7. [Sprint 0.1: F0-006 — Explain Flag](#7-f0-006)
8. [Sprint 0.2: F0-007 — SyncService CommandExecutor Migration](#8-f0-007)
9. [Sprint 0.2: F0-008 — CleanupService iron_pacman Migration](#9-f0-008)
10. [Sprint 0.2: F0-009 — Async Sync in TUI](#10-f0-009)
11. [Sprint 0.2: F0-010 — Pre-Push Secrets Lock](#11-f0-010)
12. [Sprint 0.2: F0-011 — ModuleCreator Dotfile Step](#12-f0-011)
13. [Sprint 0.2: F0-012 — Full Recovery Import](#13-f0-012)
14. [Testing Strategy](#14-testing-strategy)
15. [Product Requirement Cross-Reference](#15-product-cross-reference)

---

## 1. Architecture Context

### Crate Dependency Flow (relevant to Phase 0)

```
iron-cli (binary)
  ├── main.rs           ← F0-001: None arm change
  ├── cli.rs            ← F0-006: --explain flag
  ├── output.rs         ← F0-005, F0-006: summary + explain methods
  ├── context.rs        ← F0-006: expose explain flag
  └── commands/
      ├── clean.rs      ← F0-005: wire summary
      ├── update.rs     ← F0-005: wire summary
      ├── module.rs     ← F0-005: wire summary
      ├── bundle.rs     ← F0-005: wire summary
      └── sync.rs       ← F0-005: wire summary

iron-tui (library)
  ├── app/
  │   ├── mod.rs        ← F0-002, F0-004, F0-009: new App fields
  │   ├── actions.rs    ← F0-009: async sync spawn
  │   └── handlers.rs   ← F0-009, F0-011: key handling
  └── ui/
      ├── dashboard.rs  ← F0-002, F0-004: new panels
      └── module_creator.rs ← F0-011: dotfile step

iron-core (library)
  └── services/
      ├── sync.rs       ← F0-007, F0-010: executor migration + secrets lock
      ├── clean.rs      ← F0-008: PackageManager + executor migration
      ├── doctor.rs     ← F0-003: disk space check
      ├── recovery.rs   ← F0-012: full import flow
      └── mod.rs        ← F0-008: re-export new trait methods

iron-pacman (library)
  └── lib.rs            ← F0-008: new PackageManager trait methods
```

### Key Patterns to Follow

**1. Circuit Breaker / CommandExecutor (for F0-007, F0-008):**
```rust
// Located: iron-core/src/resilience/command_executor.rs
pub trait CommandExecutor: Send + Sync {
    fn execute(&self, cmd: &str, args: &[&str]) -> Result<String, CommandError>;
    fn execute_full(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, CommandError>;
}

// Usage pattern (SyncService already does this for some calls):
if let Some(ref executor) = self.executor {
    let output = executor.execute_full("git", &full_args)?;
    // ...
}
```

**2. Trait + Default Implementation (for F0-008 new methods):**
```rust
// Pattern from iron-core/src/packages.rs
pub trait PackageManager: Send + Sync {
    fn install(&self, packages: &[String]) -> IronResult<()>;
    fn remove(&self, packages: &[String]) -> IronResult<()>;
    fn is_installed(&self, package: &str) -> bool;
    // Add new methods here with default impls for backward compat
}
```

**3. Builder pattern for DI (established in CleanupService, SyncService):**
```rust
// Example: CleanupService already has this
impl DefaultCleanupService {
    pub fn new() -> Self { ... }
    pub fn with_package_manager(mut self, pm: Arc<dyn PackageManager>) -> Self { ... }
    // Add: pub fn with_executor(mut self, ex: Arc<dyn CommandExecutor>) -> Self { ... }
}
```

---

## 2. F0-001 — Make `iron` (no args) Launch TUI {#2-f0-001}

### Product Requirement
- **Newcomer doc §6.2:** TUI is the #1 interface preference
- **Mid-level doc §13.1:** "Simple things should be simple"
- **Project brief:** "TUI dashboard with guided wizards" is a key deliverable
- **Current behavior:** `iron` → welcome text; `iron go` → TUI
- **Desired behavior:** `iron` → TUI; `iron -f json` → JSON welcome (machine consumers)

### Codebase Analysis

**File:** `crates/iron-cli/src/main.rs`, lines 89-117

The `None` arm currently prints a welcome message. The `Some(Commands::Go)` arm (lines 77-83) has the exact TUI launch code we need.

### Implementation

**Change 1:** In `main.rs`, modify the `None` arm:

```rust
// crates/iron-cli/src/main.rs — None arm (line 89)
None => {
    if matches!(cli.format, cli::OutputFormat::Json) {
        // JSON mode: structured output for machine consumers (preserve existing)
        let welcome = serde_json::json!({
            "name": "iron",
            "description": "Less is More - Turning your Arch into Iron",
            "version": env!("CARGO_PKG_VERSION"),
            "hint": "Run 'iron --help' for CLI commands"
        });
        println!("{}", serde_json::to_string_pretty(&welcome).unwrap_or_default());
        Ok(())
    } else {
        // Default: launch TUI (same as `iron go`)
        let root = std::path::PathBuf::from(&cli.root);
        let package_manager =
            std::sync::Arc::new(iron_pacman::DefaultPackageManager::default());
        let service_manager =
            std::sync::Arc::new(iron_systemd::SystemdServiceAdapter::user());
        iron_tui::run_with_config(root, package_manager, service_manager)
    }
}
```

**Change 2:** Keep `Commands::Go` for backward compatibility (no change needed — it already works).

**Change 3:** Update the JSON hint text to remove "iron go" reference.

### Testing
- Existing CLI integration tests should still pass (they invoke specific subcommands)
- No new unit test needed (this is a routing change)
- Manual test: run `iron` → verify TUI launches
- Manual test: run `iron -f json` → verify JSON output

### Risk Mitigation
- If TUI launch fails (e.g., not a TTY), the error propagates naturally via `iron_tui::run_with_config` which returns `Result<()>`
- CI/headless environments that run `iron` will get an error about terminal — this is correct behavior (same as `iron go` today)

---

## 3. F0-002 — Dashboard Sync Status + Disk Space {#3-f0-002}

### Product Requirement
- **Newcomer doc §5:** "System health dashboard — disk usage, last backup, pending updates"
- **Mid-level doc §13.2:** Dashboard Layer 1 includes "System: ✓ Clean, Packages: 847"

### Codebase Analysis

**Dashboard layout** (`crates/iron-tui/src/ui/dashboard.rs`, lines 35-62):
- Left column: System Health (10 rows), Maintenance (6 rows), Quick Actions (min 8)
- Right column: Active Configuration (10 rows), Recent Operations (8 rows), Alerts (min 5)

The **Maintenance panel** (`render_quick_stats`, lines 127-170) currently shows only Last Update and Last Cleanup. There is room for 2 more lines.

**Sync info** is already cached in `App::sync_info: Option<SyncInfo>` (populated in `actions.rs::init()`).

**Disk space** — not currently collected. Need to add via `statvfs` syscall (Rust `nix` crate) or by parsing `df` output. Since we want to avoid new dependencies, use `std::fs::metadata` + platform-specific approach.

### Implementation

**Change 1:** Add disk usage fields to `App`:

```rust
// crates/iron-tui/src/app/mod.rs — new fields
/// Root partition disk usage (used_bytes, total_bytes)
pub disk_usage: Option<(u64, u64)>,
```

**Change 2:** Collect disk usage in `App::init()`:

```rust
// crates/iron-tui/src/app/actions.rs — inside init()
// Load disk usage
self.disk_usage = Self::get_disk_usage();

// ... (new static method)
fn get_disk_usage() -> Option<(u64, u64)> {
    // Use libc::statvfs on unix
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        let path = CString::new("/").ok()?;
        let mut stat = MaybeUninit::<libc::statvfs>::uninit();
        let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
        if result == 0 {
            let stat = unsafe { stat.assume_init() };
            let total = stat.f_blocks as u64 * stat.f_frsize as u64;
            let free = stat.f_bavail as u64 * stat.f_frsize as u64;
            let used = total - free;
            Some((used, total))
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    { None }
}
```

> **Note:** `libc` is already a transitive dependency via `crossterm` and other crates. Add `libc = "0.2"` to `iron-tui/Cargo.toml` if not already present.

**Change 3:** Update `render_quick_stats` in `dashboard.rs` to add sync + disk lines:

```rust
// After the Last Cleanup line, add:
// Sync status line
let sync_str = match &app.sync_info {
    Some(info) => match info.status {
        SyncStatus::UpToDate => "✓ Up to date".to_string(),
        SyncStatus::Ahead => format!("↑ {} ahead", info.commits_ahead),
        SyncStatus::Behind => format!("↓ {} behind", info.commits_behind),
        SyncStatus::Diverged => format!("⚠ Diverged ({}↑ {}↓)", info.commits_ahead, info.commits_behind),
        SyncStatus::Dirty => "~ Uncommitted changes".to_string(),
        SyncStatus::NotARepo => "— Not a repo".to_string(),
    },
    None => "— Unknown".to_string(),
};

// Disk usage line
let disk_str = match app.disk_usage {
    Some((used, total)) => {
        let used_gb = used as f64 / 1_073_741_824.0;
        let total_gb = total as f64 / 1_073_741_824.0;
        let pct = (used as f64 / total as f64 * 100.0) as u64;
        format!("{:.0}% ({:.1}G / {:.1}G)", pct, used_gb, total_gb)
    },
    None => "— Unknown".to_string(),
};
```

**Panel height adjustment:** The Maintenance panel is currently `Constraint::Length(6)`. Increase to `Constraint::Length(8)` to fit 4 rows (update, clean, sync, disk).

### Testing
- Existing `test_render_dashboard_no_panic` covers render without data
- Add test: `App` with `disk_usage = Some((50_000_000_000, 256_000_000_000))` renders without panic
- Add test: `App` with `sync_info = Some(SyncInfo { status: SyncStatus::Ahead, commits_ahead: 3, ... })` renders without panic

---

## 4. F0-003 — Disk Space Check in Doctor {#4-f0-003}

### Product Requirement
- **Newcomer doc §2.8:** "Health check — disk space, failed services"
- **Requirements FR-10.1-10.8:** Doctor validates system health
- **Mid-level doc §15.1:** "Disk full — detect before starting operations"

### Codebase Analysis

**File:** `crates/iron-core/src/services/doctor.rs`

The `DefaultDoctorService` has individual check methods (`check_state_file`, `check_directories`, `check_host`, etc.) called from `check_all()`. Each returns a `HealthCheck` struct.

`check_all()` aggregates all checks (approximately line 350+). We add a new check method and call it.

### Implementation

**New method:**

```rust
// crates/iron-core/src/services/doctor.rs
/// Check N: Root partition disk space
fn check_disk_space(&self) -> HealthCheck {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        let path = CString::new("/").unwrap();
        let mut stat = MaybeUninit::<libc::statvfs>::uninit();
        let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
        if result == 0 {
            let stat = unsafe { stat.assume_init() };
            let free_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
            let total_bytes = stat.f_blocks as u64 * stat.f_frsize as u64;
            let free_gb = free_bytes as f64 / 1_073_741_824.0;
            let pct_used = ((total_bytes - free_bytes) as f64 / total_bytes as f64 * 100.0) as u64;

            if free_gb < 1.0 {
                return HealthCheck {
                    name: "disk_space".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Root: {:.1} GB free ({pct_used}% used) — critically low!", free_gb),
                    details: vec!["Run 'iron clean' to free space".to_string()],
                };
            }
            if free_gb < 5.0 {
                return HealthCheck {
                    name: "disk_space".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("Root: {:.1} GB free ({pct_used}% used) — consider cleanup", free_gb),
                    details: vec!["Run 'iron clean' to free space".to_string()],
                };
            }
            return HealthCheck {
                name: "disk_space".to_string(),
                status: CheckStatus::Pass,
                message: format!("Root: {:.1} GB free ({pct_used}% used)", free_gb),
                details: vec![],
            };
        }
    }
    // Fallback if statvfs fails or non-unix
    HealthCheck {
        name: "disk_space".to_string(),
        status: CheckStatus::Warn,
        message: "Unable to check disk space".to_string(),
        details: vec![],
    }
}
```

**Wire into `check_all()`:** Add `self.check_disk_space()` to the checks vector.

**Dependency:** Add `libc = "0.2"` to `iron-core/Cargo.toml` (if not present already).

### Testing
- Unit test with mock: since `statvfs` is a syscall, the test verifies the non-unix fallback path and the formatting logic
- Integration test: `iron doctor` output includes "disk_space" check

---

## 5. F0-004 — Getting Started Hints {#5-f0-004}

### Product Requirement
- **Newcomer doc §6.3:** "Discoverability — explore without reading a manual"
- **Newcomer doc §7.1:** Day-to-day workflow starts with `iron status`
- **Mid-level doc §13.2:** Progressive disclosure

### Codebase Analysis

**Recent Operations panel** (`render_recent_ops`, `dashboard.rs` lines 370-395):
- Renders last 5 operations from `app.recent_operations`
- When empty: "No operations recorded yet"

**Operation count source:** `app.recent_operations` is populated from `StateManager::recent_audit(5)` in `actions.rs::init()`. The `state.json` file stores `last_operations: Vec<OperationRecord>`.

### Implementation

**Change 1:** Replace `render_recent_ops` with a conditional renderer:

```rust
fn render_recent_ops_or_getting_started(frame: &mut Frame, area: Rect, app: &App) {
    let op_count = app.recent_operations.len();

    if op_count < 3 {
        render_getting_started(frame, area, op_count);
    } else {
        render_recent_ops(frame, area, app);
    }
}
```

**New function `render_getting_started`:**

```rust
fn render_getting_started(frame: &mut Frame, area: Rect, completed: usize) {
    let block = simple_block("Getting Started");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let steps = [
        ("[s]", "Scan your system", "Discover existing configs & packages"),
        ("[u]", "Check for updates", "Safe system update with risk scoring"),
        ("[b]", "Explore bundles", "Choose a desktop environment"),
        ("[m]", "Browse modules", "Enable app configurations"),
    ];

    let mut content = vec![Line::from("")];
    for (i, (key, action, desc)) in steps.iter().enumerate() {
        let done = i < completed;
        let icon = if done { "✓" } else { "→" };
        let icon_color = if done { theme::GREEN } else { theme::MAUVE };
        content.push(Line::from(vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(icon_color)),
            Span::styled(format!("{} ", key), Style::default().fg(theme::MAUVE).bold()),
            Span::styled(*action, Style::default().fg(theme::TEXT)),
            Span::styled(format!("  {}", desc), Style::default().fg(theme::SUBTEXT)),
        ]));
    }

    frame.render_widget(Paragraph::new(content), inner);
}
```

**Change 2:** In `render_dashboard()`, replace `render_recent_ops(frame, right_layout[1], app)` with `render_recent_ops_or_getting_started(frame, right_layout[1], app)`.

### Testing
- Test: `App` with empty `recent_operations` → `render_getting_started` called (no panic)
- Test: `App` with 5 operations → `render_recent_ops` called (existing behavior)

---

## 6. F0-005 — CLI Operation Summary Lines {#6-f0-005}

### Product Requirement
- **Newcomer doc §6.1:** "Summary views — after any operation, give me a summary"
- **Mid-level doc §13.3:** "Summary: 0 packages installed, 1 config updated, 1 script executed"

### Codebase Analysis

**File:** `crates/iron-cli/src/output.rs`

Current `Output` has: `success()`, `error()`, `warning()`, `info()`, `header()`, `subheader()`, `kv()`, `list_item()`, `list_item_status()`, `separator()`, `table_row()`, `json()`, `raw()`.

Missing: a `summary()` method for post-operation summaries.

### Implementation

**New method in `Output`:**

```rust
// crates/iron-cli/src/output.rs

/// Print an operation summary block
///
/// Items: slice of (label, count) pairs. E.g., [("packages installed", 3), ("configs linked", 2)]
/// Error count determines color: 0 = green, >0 for "errors" or "failed" = red
pub fn summary(&self, items: &[(&str, usize)]) {
    if self.quiet {
        return;
    }
    match self.format {
        OutputFormat::Text => {
            let parts: Vec<String> = items
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|(label, count)| format!("{} {}", count, label))
                .collect();

            if parts.is_empty() {
                return;
            }

            let summary_text = parts.join(" · ");
            let has_errors = items.iter().any(|(label, count)| {
                *count > 0 && (label.contains("error") || label.contains("fail"))
            });

            let color = if has_errors { "\x1b[31m" } else { "\x1b[32m" };
            let no_color_prefix = if has_errors { "[!]" } else { "[=]" };

            if self.no_color {
                println!("\n  {} Summary: {}\n", no_color_prefix, summary_text);
            } else {
                println!("\n  {}▸ Summary:\x1b[0m {}\n", color, summary_text);
            }
        }
        OutputFormat::Json => {
            let map: std::collections::HashMap<&str, usize> =
                items.iter().cloned().collect();
            if let Ok(json) = serde_json::to_string(&map) {
                println!(r#"{{"summary":{}}}"#, json);
            }
        }
        OutputFormat::Minimal => {}
    }
}
```

**Wire into commands:**

Example for `clean.rs` (already has summary data):

```rust
// At the end of commands/clean.rs::execute(), before Ok(())
output.summary(&[
    ("items cleaned", summary.total_items),
    ("succeeded", summary.successful),
    ("failed", summary.failed),
]);
```

Example for `module.rs` enable:

```rust
output.summary(&[
    ("packages checked", module.packages.len()),
    ("configs linked", module.dotfiles.len()),
    ("errors", 0),
]);
```

### Testing
- Unit test: `Output::summary` with items produces expected format
- Unit test: Empty items produces no output
- Unit test: JSON mode produces valid JSON

---

## 7. F0-006 — `--explain` Flag {#7-f0-006}

### Product Requirement
- **Newcomer doc §3 #10:** "Transparent — I can see what it does"
- **Newcomer doc §11.1:** "Show underlying commands"
- **Mid-level doc §14.1:** Progressive disclosure

### Codebase Analysis

**Global flags** in `crates/iron-cli/src/cli.rs` (lines 21-41):
- `--root`, `--format`, `--verbose`, `--quiet`, `--no-color` — all global

**Context** in `crates/iron-cli/src/context.rs`:
- `AppContext` holds `root`, `state`, `output`
- `Output` holds format settings

### Implementation

**Change 1:** Add flag to `Cli`:

```rust
// crates/iron-cli/src/cli.rs — Cli struct
/// Show underlying commands being executed
#[arg(long, global = true)]
pub explain: bool,
```

**Change 2:** Add to `Output`:

```rust
// crates/iron-cli/src/output.rs — Output struct
explain: bool,

// New method
/// Print an explain line showing the command being executed
pub fn explain(&self, cmd: &str) {
    if !self.explain || self.quiet {
        return;
    }
    match self.format {
        OutputFormat::Text => {
            if self.no_color {
                println!("  -> Running: {}", cmd);
            } else {
                println!("  \x1b[36m→\x1b[0m \x1b[90m{}\x1b[0m", cmd);
            }
        }
        OutputFormat::Json => {
            println!(r#"{{"command":"{}"}}"#, cmd);
        }
        OutputFormat::Minimal => {}
    }
}
```

**Change 3:** Update `Output::new()` and `AppContext::new()` to accept and pass through the `explain` flag.

**Change 4:** Wire into commands. Example in `update.rs` before pacman call:

```rust
output.explain("sudo pacman -Syu --noconfirm");
```

### Testing
- Unit test: `Output` with `explain=true` outputs command
- Unit test: `Output` with `explain=false` outputs nothing

---

## 8. F0-007 — SyncService CommandExecutor Migration (A-001) {#8-f0-007}

### Product Requirement
- **Requirements FR-5.9:** "All external commands time out after 120s"
- **Newcomer doc §3 #9:** "No silent failures"
- **Architecture:** Circuit breaker pattern for all external commands

### Codebase Analysis

**File:** `crates/iron-core/src/services/sync.rs`

**Current state:**
- `DefaultSyncService::new()` creates service with `executor: None`
- `DefaultSyncService::with_resilience()` creates with `RealCommandExecutor`
- `fn git()` has two branches: executor path (lines 140-155) and raw `Command` fallback (lines 156-185)
- 18 total `Command::new("git")` — 1 in `fn git()` fallback, rest in `#[cfg(test)]` helpers

**Call sites creating SyncService:**
1. `crates/iron-cli/src/context.rs` line 81: `DefaultSyncService::with_resilience()` ✅ (already uses executor)
2. `crates/iron-tui/src/app/actions.rs` line 51: `DefaultSyncService::with_resilience()` ✅ (already uses executor)

**Finding:** Both production call sites already use `with_resilience()`. The raw `Command` fallback in `fn git()` is dead code in production but exists as a safety net. The task is to:
1. Make the executor non-optional (always present)
2. Remove the fallback branch
3. Update `new()` to use the default executor

### Implementation

**Change 1:** Make `executor` non-optional:

```rust
// crates/iron-core/src/services/sync.rs
pub struct DefaultSyncService {
    repo_root: PathBuf,
    state_manager: StateManager,
    // Changed: no longer Option
    executor: Arc<dyn CommandExecutor>,
    secrets_service: Option<Arc<dyn crate::services::secrets::SecretsService>>,
}

impl DefaultSyncService {
    /// Create a new sync service with default resilient executor
    pub fn new(repo_root: &Path, state_manager: StateManager) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            state_manager,
            executor: Arc::new(RealCommandExecutor::with_defaults()),
            secrets_service: None,
        }
    }

    // with_resilience() becomes an alias for new() or is kept for clarity
    pub fn with_resilience(repo_root: &Path, state_manager: StateManager) -> Self {
        Self::new(repo_root, state_manager)
    }

    // with_executor() still useful for testing with mocks
    pub fn with_executor(
        repo_root: &Path,
        state_manager: StateManager,
        executor: Arc<dyn CommandExecutor>,
    ) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            state_manager,
            executor,
            secrets_service: None,
        }
    }
}
```

**Change 2:** Simplify `fn git()`:

```rust
fn git(&self, args: &[&str]) -> IronResult<String> {
    let mut full_args = vec!["-C", self.repo_root.to_str().unwrap_or(".")];
    full_args.extend(args);

    let output = self.executor
        .execute_full("git", &full_args)
        .map_err(|e| GitError::CommandFailed {
            message: format!("{}", e),
        })?;

    if output.success() {
        Ok(output.stdout.trim().to_string())
    } else {
        Err(GitError::CommandFailed {
            message: output.stderr.trim().to_string(),
        }.into())
    }
}
```

**Change 3:** Update test helpers — `init_git_repo` and test setup functions use raw `Command::new("git")` which is fine for test-only code. No change needed.

**Change 4:** Update `context.rs` — `sync_service()` can now use `DefaultSyncService::new()` instead of `with_resilience()` (they're equivalent). Keep `with_resilience()` for readability.

### Testing
- All 20+ existing SyncService tests must pass
- New test: `DefaultSyncService::new()` creates service with working executor
- New test: `with_executor()` + `MockCommandExecutor` verifies timeout behavior

### Risk
- The `Clone` derive on `DefaultSyncService` must still work. `Arc<dyn CommandExecutor>` is `Clone`. ✅

---

## 9. F0-008 — CleanupService iron_pacman Migration (F-005) {#9-f0-008}

### Product Requirement
- **Architecture:** All external commands through circuit breaker
- **FR-5.9:** 120s timeout on external commands

### Codebase Analysis

**File:** `crates/iron-core/src/services/clean.rs`

**6 raw `Command::new` calls:**

| Line | Command | Purpose | Migration Target |
|------|---------|---------|-----------------|
| 385 | `Command::new("pacman").args(["-Qtdq"])` | List orphan packages | `PackageManager::query_orphans()` (new) |
| 421 | `Command::new("journalctl").args(["--disk-usage"])` | Journal size check | `CommandExecutor::execute("journalctl", ...)` |
| 668 | `Command::new("sudo").args(["pacman", "-Rns", ...])` | Remove orphans | `PackageManager::remove_orphans()` (new) |
| 706 | `Command::new("pacman").args(["-Qtdq"])` | Re-check orphans | `PackageManager::query_orphans()` |
| 758 | `Command::new("sudo").args(["paccache", "-rk3"])` | Clean pkg cache | `CommandExecutor::execute("sudo", ["paccache", "-rk3"])` |
| 804 | `Command::new("sudo").args(["journalctl", "--vacuum-size=100M"])` | Vacuum journal | `CommandExecutor::execute("sudo", ...)` |

### Implementation

**Step 1:** Add new methods to `PackageManager` trait:

```rust
// crates/iron-core/src/packages.rs — PackageManager trait
/// Query orphan packages (installed as deps, no longer required)
fn query_orphans(&self) -> IronResult<Vec<String>> {
    Ok(vec![]) // default: no orphans
}

/// Remove orphan packages
fn remove_orphans(&self, packages: &[String]) -> IronResult<()> {
    Ok(()) // default: no-op
}
```

**Step 2:** Implement in `iron-pacman`:

```rust
// crates/iron-pacman/src/lib.rs (or relevant file)
fn query_orphans(&self) -> IronResult<Vec<String>> {
    let output = Command::new("pacman")
        .args(["-Qtdq"])
        .output()
        .map_err(|e| PackageError::PacmanError { message: e.to_string() })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect())
    } else {
        Ok(vec![]) // No orphans = empty output with exit code 1
    }
}

fn remove_orphans(&self, packages: &[String]) -> IronResult<()> {
    if packages.is_empty() {
        return Ok(());
    }
    let mut args = vec!["-Rns", "--noconfirm"];
    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    args.extend(pkg_refs);

    let output = Command::new("sudo")
        .arg("pacman")
        .args(&args)
        .output()
        .map_err(|e| PackageError::PacmanError { message: e.to_string() })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(PackageError::RemoveFailed {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        }.into())
    }
}
```

**Step 3:** Add `CommandExecutor` to `DefaultCleanupService`:

```rust
// crates/iron-core/src/services/clean.rs
pub struct DefaultCleanupService {
    home_dir: PathBuf,
    package_manager: Option<Arc<dyn crate::PackageManager>>,
    executor: Option<Arc<dyn CommandExecutor>>,   // NEW
}

pub fn with_executor(mut self, executor: Arc<dyn CommandExecutor>) -> Self {
    self.executor = Some(executor);
    self
}
```

**Step 4:** Replace raw `Command::new` calls in clean.rs with `self.package_manager` and `self.executor` calls.

**Step 5:** Wire in CLI `clean.rs`:

```rust
// crates/iron-cli/src/commands/clean.rs
let service = DefaultCleanupService::new()
    .with_package_manager(Arc::new(iron_pacman::DefaultPackageManager::new()))
    .with_executor(Arc::new(iron_core::RealCommandExecutor::with_defaults()));
```

### Testing
- New unit tests for `query_orphans()` and `remove_orphans()` in iron-pacman
- Mock tests in clean.rs: `MockCommandExecutor` returns expected journalctl/paccache output
- All existing cleanup tests pass

---

## 10. F0-009 — Async Sync in TUI (D-009) {#10-f0-009}

### Product Requirement
- **Mid-level doc §18.2:** "UI never freezes"
- **Newcomer doc §4 #2:** "Progress indicators for long operations"

### Codebase Analysis

**Already scaffolded in `App`:**

```rust
// crates/iron-tui/src/app/mod.rs
pub sync_in_progress: bool,
pub sync_result_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
```

**Current sync calls** in `actions.rs` (search for `sync_push`, `sync_pull`):
- They call `self.sync_service.push()` / `.pull()` synchronously on the main thread.

### Implementation

**New methods in `App`:**

```rust
// crates/iron-tui/src/app/actions.rs

/// Start async push operation
pub fn sync_push_async(&mut self) {
    if self.sync_in_progress {
        self.set_warning("Sync already in progress");
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    self.sync_in_progress = true;
    self.sync_result_rx = Some(rx);

    // Clone what the thread needs
    let config_dir = self.config_dir.clone();
    let state_manager = self.state_manager.clone();

    std::thread::spawn(move || {
        let result = if let Some(sm) = state_manager {
            let svc = iron_core::services::sync::DefaultSyncService::new(&config_dir, sm);
            match svc.push() {
                Ok(()) => Ok("Push complete".to_string()),
                Err(e) => Err(format!("{}", e)),
            }
        } else {
            Err("State manager not initialized".to_string())
        };
        let _ = tx.send(result);
    });

    self.set_status("Syncing...");
}

/// Poll for async sync result (call in event loop tick)
pub fn poll_sync_result(&mut self) {
    if let Some(ref rx) = self.sync_result_rx {
        if let Ok(result) = rx.try_recv() {
            self.sync_in_progress = false;
            self.sync_result_rx = None;
            match result {
                Ok(msg) => self.set_status(msg),
                Err(msg) => self.set_error(msg),
            }
            // Refresh sync info after operation
            self.refresh_sync_info();
        }
    }
}
```

**Wire into event loop:** The TUI event loop (in `iron-tui/src/lib.rs` or `terminal.rs`) should call `app.poll_sync_result()` on each tick.

**Wire into handlers:** Replace synchronous push/pull calls in `handlers.rs` with `sync_push_async()` / `sync_pull_async()`.

### Testing
- Test: `sync_push_async` sets `sync_in_progress = true`
- Test: `poll_sync_result` with received `Ok(msg)` clears `sync_in_progress`
- Test: Double-press while `sync_in_progress` shows warning

---

## 11. F0-010 — Pre-Push Secrets Lock (A-010) {#11-f0-010}

### Product Requirement
- **Mid-level doc §7.3:** "Secrets are NEVER in the config repo"
- **Requirements FR-8.1:** "Encrypt secrets at rest"

### Codebase Analysis

**File:** `crates/iron-core/src/services/sync.rs`

The `push()` method (approximately lines 400-440) already has the `secrets_service` field wired. The `SecretsService` trait has `lock()` and `status()` methods.

### Implementation

**In `DefaultSyncService::push()`**, add before the `git push` call:

```rust
// Before pushing, auto-lock secrets if unlocked
if let Some(ref secrets) = self.secrets_service {
    if let Ok(status) = secrets.status() {
        if matches!(status, crate::services::secrets::SecretsStatus::Unlocked) {
            tracing::info!("Auto-locking secrets before push");
            if let Err(e) = secrets.lock() {
                return Err(GitError::CommandFailed {
                    message: format!("Failed to lock secrets before push: {}. Aborting push to prevent leaking secrets.", e),
                }.into());
            }
        }
    }
}
```

### Testing
- Test with `MockSecretsService` that reports Unlocked → verify `lock()` called before push
- Test with `MockSecretsService` that reports Locked → verify no lock call, push proceeds
- Test with no secrets service → verify push proceeds normally

---

## 12. F0-011 — ModuleCreator Dotfile Step (D-012) {#12-f0-011}

### Product Requirement
- **Mid-level doc §4.4:** "Help me turn my config into a reusable module"
- **Requirements FR-4.1:** "Module is defined by a single module.toml containing packages, dotfiles, and hooks"

### Codebase Analysis

**UI exists:** `crates/iron-tui/src/ui/module_creator.rs` has `render_step_dotfiles()` (step 2 of 3).

**App state exists:**
```rust
pub module_creator_dotfiles: Vec<(String, String)>,
pub module_creator_dotfile_field: usize, // 0=source, 1=target
```

**Handler code:** `handle_module_creator_key()` in `handlers.rs` — need to verify step 2 handles Tab (switch field), Enter (add pair / advance), Backspace (delete last), Esc (back to step 1).

**Module save code:** In `actions.rs`, the `create_module_from_wizard()` method builds a `Module` struct and calls `module.save()`. Verify that `module_creator_dotfiles` entries are mapped into `module.dotfiles`.

### Implementation

**Verify/fix handler for step 2:** Ensure the key handler for `module_creator_step == 1` handles:

```
Tab → toggle module_creator_dotfile_field between 0 and 1
Enter (both fields non-empty) → push (source, target) to module_creator_dotfiles, clear inputs
Enter (both fields empty) → advance to step 2 (preview)
Backspace (source field empty) → pop last dotfile entry
Esc → go back to step 0
```

**Verify/fix module creation:** In the `create_module_from_wizard()` method, ensure:

```rust
dotfiles: self.module_creator_dotfiles
    .iter()
    .map(|(source, target)| DotfileMapping {
        source: source.clone(),
        target: target.clone(),
        link: true,
    })
    .collect(),
```

### Testing
- Test: create module with 2 dotfile pairs, call `save()`, read back `module.toml`, verify `[[dotfiles]]` entries present
- Test: step 2 key handling — Tab toggles field, Enter adds pair

---

## 13. F0-012 — Full Recovery Import (C-009) {#13-f0-012}

### Product Requirement
- **Requirements FR-6.3:** "iron recover runs a 4-step flow: Install → Bundle → Profile → Verify"
- **Newcomer doc §2.5:** "Easy restoration — ideally to a different machine"
- **Mid-level doc §8.4:** "Import a state on a new machine and reproduce my setup"

### Codebase Analysis

**File:** `crates/iron-core/src/services/recovery.rs`

**Current `import()` (lines 218-290):**
1. ✅ Set current host
2. ✅ Set active bundle
3. ✅ Set active profile
4. ✅ Enable modules (in state)
5. ✅ Install packages (C-009 partial — best-effort)
6. ✅ Enable services (C-009 partial — best-effort)
7. ❌ **Missing:** Re-link dotfiles for active modules
8. ❌ **Missing:** Run post-install hooks
9. ❌ **Missing:** Verification step

### Implementation

**Add Step 7 after service enablement:**

```rust
// Step 7: Re-link dotfiles for active modules
let modules_dir = self.iron_root.join("modules");
for module_id in &export.active_modules {
    let module_dir = modules_dir.join(module_id);
    let module_toml = module_dir.join("module.toml");
    if !module_toml.exists() {
        self.state_manager.record_operation(
            "import_dotfiles",
            OperationStatus::Failed,
            Some(format!("Module '{}' not found on disk, skipping dotfiles", module_id)),
        )?;
        continue;
    }

    match crate::module::Module::load(&module_dir) {
        Ok(module) => {
            for dotfile in &module.dotfiles {
                let source = module_dir.join(&dotfile.source);
                let target = crate::validation::expand_home(std::path::Path::new(&dotfile.target));

                if !source.exists() {
                    continue;
                }

                // Create parent directories
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                // Backup existing file
                if target.exists() && !target.is_symlink() {
                    let backup = target.with_extension("iron-backup");
                    let _ = fs::rename(&target, &backup);
                }

                // Remove existing symlink
                if target.is_symlink() {
                    let _ = fs::remove_file(&target);
                }

                // Create symlink
                #[cfg(unix)]
                if let Err(e) = std::os::unix::fs::symlink(&source, &target) {
                    self.state_manager.record_operation(
                        "import_dotfiles",
                        OperationStatus::Failed,
                        Some(format!("Failed to link {} -> {}: {}", source.display(), target.display(), e)),
                    )?;
                }
            }

            // Run post-install hook if present
            if let Some(ref hook) = module.post_install {
                let hook_path = module_dir.join("hooks").join(hook);
                if hook_path.exists() {
                    let status = std::process::Command::new("bash")
                        .arg(&hook_path)
                        .current_dir(&module_dir)
                        .env("IRON_MODULE_ID", &module.id)
                        .env("IRON_MODULE_DIR", &module_dir)
                        .status();

                    if let Ok(s) = status {
                        if !s.success() {
                            self.state_manager.record_operation(
                                "import_hook",
                                OperationStatus::Failed,
                                Some(format!("Post-install hook failed for '{}'", module_id)),
                            )?;
                        }
                    }
                }
            }
        }
        Err(e) => {
            self.state_manager.record_operation(
                "import_dotfiles",
                OperationStatus::Failed,
                Some(format!("Failed to load module '{}': {}", module_id, e)),
            )?;
        }
    }
}

// Step 8: Verification
let verification = self.verify_installation(export);
if !verification.passed {
    self.state_manager.record_operation(
        "import_verify",
        OperationStatus::Partial,
        Some(verification.summary.clone()),
    )?;
}

self.state_manager
    .record_operation("import_recovery", OperationStatus::Success, None)?;
```

### Testing
- Test with temp dir: create 2 module dirs with dotfile sources, run import, verify symlinks created
- Test: missing module dir → logged as failure, import continues
- Test: existing file at target → backed up before overwrite
- Test: verification runs and reports missing packages

---

## 14. Testing Strategy

### Test Categories

| Category | Where | What |
|----------|-------|------|
| **Unit tests** | `#[cfg(test)]` modules in each file | Individual function behavior |
| **Render tests** | `iron-tui/src/ui/tests.rs` and per-view `#[cfg(test)]` | TUI views render without panic |
| **Integration tests** | `iron-cli/tests/` | CLI commands produce expected output |
| **Mock-based** | Using `MockCommandExecutor`, `NoopPackageManager` | Service behavior without real system tools |

### Test Count Target

| Sprint | New Tests | Total (est.) |
|--------|-----------|-------------|
| 0.1 | ≥ 10 | ~1,713 |
| 0.2 | ≥ 20 | ~1,733 |

### What NOT to Test in Phase 0
- Real `pacman` operations (require sudo, not CI-safe)
- Real `git push`/`pull` to remotes
- Real `statvfs` results (platform-dependent)

Use mocks and `#[cfg(test)]` overrides for all system-dependent behavior.

---

## 15. Product Requirement Cross-Reference

| Task | Product Review Section | Newcomer Constraint | Mid-Level Constraint | FR |
|------|----------------------|---------------------|---------------------|----|
| F0-001 | §4.4 (iron go vs iron) | §6.2 (TUI preference) | §13.1 (simple is simple) | FR-9.7 |
| F0-002 | §6.2 (Dashboard missing info) | §5 (health dashboard) | §13.2 (Layer 1 dashboard) | FR-9.1 |
| F0-003 | §6.2 (Dashboard missing info) | §2.8 (health check) | §15.1 (disk full) | FR-10.4 |
| F0-004 | §6.4 (Onboarding flow) | §6.3 (discoverability) | §13.2 (progressive disclosure) | FR-9.7 |
| F0-005 | §4.8 (CLI output) | §6.1 (summary views) | §13.3 (structured output) | — |
| F0-006 | §4.8 (CLI output) | §3 #10 (transparent) | §11.1 (show commands) | — |
| F0-007 | §2 (Tech debt A-001) | §3 #9 (no silent failures) | — | FR-5.9 |
| F0-008 | §2 (Tech debt F-005) | — | — | FR-5.9 |
| F0-009 | §2 (Tech debt D-009) | §4 #2 (progress) | §18.2 (UI never freezes) | NFR-1 |
| F0-010 | §2 (Tech debt A-010) | — | §7.3 (secrets protection) | FR-8.1 |
| F0-011 | §2 (Tech debt D-012) | — | §4.4 (module creation) | FR-4.1 |
| F0-012 | §2 (Tech debt C-009) | §2.5 (restoration) | §8.4 (import state) | FR-6.3 |

---

*This document is the technical source of truth for Phase 0 implementation. Each task section contains the exact files, lines, patterns, and code structures needed to implement the change. Cross-reference the kanban board (`docs/phase0-kanban.md`) for task status tracking.*
