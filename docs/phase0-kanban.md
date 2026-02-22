# Phase 0 — Sprint Kanban Board

> **Phase:** 0 — Foundation Fixes
> **Sprints:** 0.1 (UX Quick Wins) + 0.2 (Tech Debt Closure)
> **Estimated Duration:** 2 sprints (~4 weeks)
> **Branch Convention:** `phase0/F0-XXX-short-description`
> **Commit Convention:** `F0-XXX: short description`
> **Status:** ✅ ALL 12 TASKS IMPLEMENTED (2026-02-22)

---

## Sprint 0.1 — UX Quick Wins

**Sprint Goal:** Remove friction from the primary user journey — make Iron feel like a product, not a prototype. Every change is low-risk, high-visibility.

---

### 🟡 TODO

*(Empty — all tasks moved to DONE)*

---

### 🔵 IN PROGRESS

*(Empty)*

---

### ✅ DONE

#### F0-001: Make `iron` (no args) launch TUI by default ✅
- **Files changed:** `crates/iron-cli/src/main.rs`
- **What was done:** Changed `None` arm to launch TUI. JSON mode still outputs structured welcome. `iron go` preserved as alias.
- **Acceptance Criteria:** [x] TUI launches on bare `iron` [x] JSON mode preserved [x] `iron go` backward compat [x] `--help` unchanged

---

#### F0-002: Add pending updates + sync status to TUI dashboard ✅
- **Files changed:** `crates/iron-tui/src/ui/dashboard.rs`, `crates/iron-tui/Cargo.toml`
- **What was done:** Added sync status line (Up to date / Ahead / Behind / Diverged / Dirty / Not a repo) and disk space line (percentage + GB used/total) to the Maintenance panel. Increased panel height from 6→8 rows. Color-coded: green <70%, yellow 70-85%, red >85%. Uses `libc::statvfs` for disk space.
- **Acceptance Criteria:** [x] Sync status displayed [x] Disk space displayed [x] Color-coded [x] Graceful when unavailable

---

#### F0-003: Add disk space check to Doctor and Dashboard ✅
- **Files changed:** `crates/iron-core/src/services/doctor.rs`, `crates/iron-core/Cargo.toml`
- **What was done:** Added `check_disk_space()` health check using `libc::statvfs`. Pass >5GB, Warn 1-5GB, Fail <1GB. Wired into `check_all()` so it flows to Dashboard health panel and `iron doctor` output. Added `libc = "0.2"` to iron-core.
- **Acceptance Criteria:** [x] New check in doctor [x] Thresholds correct [x] Shows in dashboard and CLI [x] Human-readable values

---

#### F0-004: Add "Getting Started" hints on Dashboard for new users ✅
- **Files changed:** `crates/iron-tui/src/ui/dashboard.rs`
- **What was done:** Added `render_getting_started()` panel that replaces "Recent Operations" when user has <3 operations. Shows 4 progressive hints with checkmarks for completed steps: Scan → Updates → Bundles → Modules. Switches to normal Recent Operations after 3+ operations.
- **Acceptance Criteria:** [x] Shows when <3 ops [x] Actionable hints with keybindings [x] Switches to recent ops at 3+ [x] Fresh install shows it

---

#### F0-005: Add operation summary line after every CLI command ✅
- **Files changed:** `crates/iron-cli/src/output.rs`, `crates/iron-cli/src/commands/clean.rs`, `crates/iron-cli/src/commands/update.rs`, `crates/iron-cli/src/commands/module.rs`
- **What was done:** Added `Output::summary(&[(&str, usize)])` method. Renders `"▸ Summary: 3 items cleaned · 2 categories succeeded"`. Green when no errors, red when errors present. JSON mode outputs structured summary. Wired into `clean`, `update`, `module enable`, `module disable`.
- **Acceptance Criteria:** [x] summary() method [x] Color-coded [x] Wired into key commands [x] JSON support [x] Quiet mode suppresses

---

#### F0-006: Add `--explain` flag to CLI ✅
- **Files changed:** `crates/iron-cli/src/cli.rs`, `crates/iron-cli/src/output.rs`, `crates/iron-cli/src/context.rs`, `crates/iron-cli/src/main.rs`
- **What was done:** Added `--explain` global flag to `Cli` struct. Added `explain` field to `Output` with `with_explain()` builder. Added `Output::explain_cmd()` that prints `"→ command"` in dim cyan. Wired through `AppContext::new()`.
- **Acceptance Criteria:** [x] Global flag added [x] explain_cmd() method [x] Context wired [x] No output when not set [x] JSON mode support

---
---

## Sprint 0.2 — Tech Debt Closure

**Sprint Goal:** Eliminate all open tech debt from the hardening backlog. Every `Command::new` call that bypasses the resilience layer gets migrated. Recovery import becomes a real full-restore flow. The codebase is clean for Phase 1.

---

### 🟡 TODO

*(Empty — all tasks moved to DONE)*

---

### 🔵 IN PROGRESS

*(Empty)*

---

### ✅ DONE

#### F0-007: Migrate SyncService to CommandExecutor (A-001) ✅
- **Files changed:** `crates/iron-core/src/services/sync.rs`
- **What was done:** Made `executor` field non-optional (`Arc<dyn CommandExecutor>` instead of `Option`). `new()` now creates with `RealCommandExecutor::with_defaults()` by default. `with_resilience()` is now an alias for `new()`. Removed the raw `Command::new("git")` fallback branch from `fn git()` — all git operations now go through the executor with 120s timeout and circuit breaker. Test helpers keep raw `Command` (test-only).
- **Acceptance Criteria:** [x] executor non-optional [x] new() uses resilient executor [x] Fallback branch removed [x] with_resilience() alias preserved

---

#### F0-008: Migrate CleanupService to iron_pacman (F-005) ✅
- **Files changed:** `crates/iron-core/src/services/clean.rs`, `crates/iron-cli/src/commands/clean.rs`
- **What was done:** Added `executor: Option<Arc<dyn CommandExecutor>>` field and `with_executor()` builder to `DefaultCleanupService`. Replaced raw `Command::new("journalctl")` in `preview_systemd_journal()` and raw `Command::new("sudo")` in `execute_systemd_journal()` with executor-delegated paths (raw Command remains as fallback when no executor injected). CLI `clean.rs` now wires `RealCommandExecutor::with_defaults()`. PackageManager paths already delegated via `get_orphans()`/`clean_cache()`.
- **Acceptance Criteria:** [x] Executor field added [x] journalctl uses executor [x] CLI wires executor [x] Existing PM delegation preserved

---

#### F0-009: Make sync push/pull async with TUI progress (D-009) ✅
- **Files changed:** *(Already implemented — verified during audit)*
- **What was found:** `sync_push()` and `sync_pull()` already spawn background threads with `std::sync::mpsc::channel()`. `poll_sync_result()` is called every tick from `App::tick()`. `sync_in_progress` flag prevents double-press. Result displayed as status/error message.
- **Status:** Was already done. No code changes needed. Verified and confirmed.

---

#### F0-010: Auto-lock secrets before sync push (A-010) ✅
- **Files changed:** `crates/iron-core/src/services/sync.rs`
- **What was done:** Strengthened the existing A-010 code in `push()`. Previously: `let _ = secrets.lock()` (silently ignored failures). Now: logs "Auto-locking secrets before push", and if `lock()` fails, aborts the push with a clear error message explaining why. Prevents leaking decrypted secrets to remote.
- **Acceptance Criteria:** [x] Lock called when unlocked [x] Push aborted on lock failure [x] Clear error message [x] No-op when already locked [x] No-op when no secrets service

---

#### F0-011: Complete ModuleCreator dotfile mapping step (D-012) ✅
- **Files changed:** `crates/iron-tui/src/app/mod.rs`, `crates/iron-tui/src/app/handlers.rs`, `crates/iron-tui/src/ui/module_creator.rs`
- **What was done:** Added `module_creator_dotfile_source` and `module_creator_dotfile_target` temporary input buffers to `App`. Fixed step 1 key handler: Char input writes to active field, Tab toggles source/target, Enter with both fields filled adds the pair and clears inputs, Enter with empty fields advances to preview, Backspace deletes from active field or pops last entry. Updated `render_step_dotfiles()` to show actual typed values with cursor indicator. Create step already included dotfiles in saved TOML (verified).
- **Acceptance Criteria:** [x] Character input works [x] Tab toggles [x] Enter adds or advances [x] Backspace deletes [x] UI shows typed values [x] Created module includes [[dotfiles]]

---

#### F0-012: Full recovery import — packages + services + dotfiles (C-009) ✅
- **Files changed:** `crates/iron-core/src/services/recovery.rs`
- **What was done:** Added Step 7 (relink dotfiles for active modules: loads