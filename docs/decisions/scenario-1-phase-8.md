# Scenario 1 — Phase 8: Git Sync (Save & Replicate Configuration)

## Implementation Guideline (Deep Dive)

> **Scope**: Task S1-P8-001 from `docs/TODO-scenario1.md` + Git Sync (user-workflow Phase 8)
> **Phase**: SyncService push/pull/status, TUI Sync view, conflict resolution, iron-git layer
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-core, iron-tui, iron-cli, iron-git

---

## Table of Contents

1. [Phase 8 Architecture Overview](#1-phase-8-architecture-overview)
2. [SyncService (iron-core) — Deep Dive](#2-syncservice-iron-core--deep-dive)
3. [iron-git Layer — Deep Dive](#3-iron-git-layer--deep-dive)
4. [TUI Sync View — Deep Dive](#4-tui-sync-view--deep-dive)
5. [CLI Sync Command — Deep Dive](#5-cli-sync-command--deep-dive)
6. [Task S1-P8-001 — Sync Conflict Resolution UI](#6-task-s1-p8-001)
7. [Discovered Issues — Outside Original Phase 8 Scope](#7-discovered-issues)
8. [Integration Map](#8-integration-map)
9. [Test Coverage Analysis](#9-test-coverage-analysis)

---

## 1. Phase 8 Architecture Overview

### What Phase 8 Covers

Phase 8 is the **Git Sync** subsystem — saving, pushing, pulling, and replicating an
Iron configuration repository across machines. It covers:

1. **Status checking** — branch, ahead/behind counts, dirty files
2. **Push workflow** — stage all + commit + push to remote
3. **Pull workflow** — fetch + rebase from remote, with optional stash
4. **Conflict handling** — detecting diverged state and unmerged files (currently [STUB])
5. **Secrets integration** — git-crypt for encrypted secrets (separate `SecretsManager`)

The Sync view is accessible via `y` hotkey from any view.

### Key Architectural Finding: Two Independent Git Layers

**This is the most significant discovery in Phase 8 analysis.** Iron has two completely
independent git abstraction layers that do not share any code:

| Layer | Location | Git Invocation | Resilience | Used By |
|-------|----------|----------------|------------|---------|
| `SyncService` | `iron-core/services/sync.rs` | Raw `std::process::Command` | None | TUI Sync view, CLI `iron sync` |
| `GitManager` | `iron-git/src/lib.rs` | `CommandExecutor` (circuit breaker, 120s timeout) | Full | Nothing in Sync path |

`SyncService` does **NOT** use `iron-git` at all. It shells out directly via
`Command::new("git")`. Meanwhile `iron-git::DefaultGitManager` provides the same
operations (status, push, pull, commit) with circuit-breaker resilience through
`CommandExecutor` — but nothing in the sync workflow uses it.

The TUI does not depend on `iron-git` in its `Cargo.toml`. It uses
`iron_core::services::sync::DefaultSyncService` exclusively.

### Key Components

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| SyncStatus enum | `services/sync.rs` L14 | 14 | 6 states: UpToDate, Ahead, Behind, Diverged, Dirty, NotARepo |
| SyncInfo struct | `services/sync.rs` L31 | 16 | Status + branch + ahead/behind + dirty + last_sync |
| SyncService trait | `services/sync.rs` L51 | 26 | 8 methods: status, pull, push, sync, commit, check_conflicts, stash, stash_pop |
| DefaultSyncService | `services/sync.rs` L79 | 240 | Full impl using raw `git` commands |
| GitStatus struct | `iron-git/lib.rs` L37 | 24 | is_clean, modified, untracked, staged, branch, ahead, behind |
| PullResult struct | `iron-git/lib.rs` L63 | 12 | success, updated_files, has_conflicts, conflict_files |
| GitManager trait | `iron-git/lib.rs` L77 | 12 | 6 methods: status, diff, commit, push, pull, has_changes |
| SecretsManager trait | `iron-git/lib.rs` L91 | 10 | 4 methods: is_unlocked, unlock, lock, list_encrypted |
| DefaultGitManager | `iron-git/lib.rs` L103 | ~150 | Git ops with CommandExecutor circuit breaker |
| DefaultSecretsManager | `iron-git/lib.rs` L258 | ~60 | git-crypt wrapper with CommandExecutor |
| SyncAction enum | `cli.rs` L362 | 18 | Status, Push{message}, Pull{stash} |
| CLI sync command | `commands/sync.rs` L1 | 247 | execute → status/push/pull dispatch |
| TUI render_sync | `ui/update.rs` L390 | 82 | Sync view rendering |
| TUI sync handlers | `handlers.rs` L345 | 19 | p=push, f=pull, s=status |
| TUI sync actions | `actions.rs` L593 | 68 | refresh_sync_status, sync_push, sync_pull |
| TUI sync state | `mod.rs` L116 | 1 | `sync_info: Option<SyncInfo>` |

### Sync Data Flow — Push (TUI)

```
User presses 'p' from Sync view
    │
    ▼
View::Sync → KeyCode::Char('p')                    [handlers.rs L347]
    │
    ▼
sync_push()                                         [actions.rs L617]
    │  DefaultSyncService::new(&config_dir, state_manager)
    │  sync_service.push()                          [services/sync.rs L224]
    │    ├─ is_repo() check
    │    ├─ git(&["push"])                          ← raw Command, no circuit breaker
    │    └─ record_operation("git_push", Success)
    │
    ├─ Ok  → set_status("Changes pushed") + refresh_sync_status()
    └─ Err → set_error("Push failed: …")
```

**Critical observation**: Push goes directly to `git push` without first committing.
Uncommitted changes are NOT auto-committed in the TUI path. The CLI path auto-commits
(see Section 5), but the TUI does not. The spec says push should "stage all changes"
first (UC-19 step 2).

### Sync Data Flow — Pull (TUI)

```
User presses 'f' from Sync view
    │
    ▼
View::Sync → KeyCode::Char('f')                    [handlers.rs L351]
    │
    ▼
sync_pull()                                         [actions.rs L643]
    │  DefaultSyncService::new(&config_dir, state_manager)
    │  sync_service.pull()                          [services/sync.rs L210]
    │    ├─ is_repo() check
    │    ├─ git(&["pull", "--rebase"])              ← always rebase, no merge option
    │    └─ record_operation("git_pull", Success)
    │
    ├─ Ok  → set_status("Changes pulled") + refresh_sync_status()
    └─ Err → set_error("Pull failed: …")
```

**Critical observations**:
- No pre-pull dirty check — if working tree is dirty, `git pull --rebase` will fail
- No auto-stash — CLI has `--stash` flag but TUI doesn't offer it
- No post-pull config detection — spec says "Iron detects and applies them" (UC-20 step 3)
- No confirm dialog — push/pull execute immediately without user confirmation

---

## 2. SyncService (iron-core) — Deep Dive

**File**: `crates/iron-core/src/services/sync.rs` — 717 lines, 23 tests

### 2.1 SyncStatus Enum (L14)

Six states with clear priority ordering:

```rust
pub enum SyncStatus {
    UpToDate,   // Clean, synced with remote
    Ahead,      // Local commits not yet pushed
    Behind,     // Remote commits not yet pulled
    Diverged,   // Both local and remote have new commits
    Dirty,      // Uncommitted working tree changes
    NotARepo,   // Not a git repository
}
```

`status()` determines which state to return using this priority at L192:
```
Dirty > Diverged > Ahead > Behind > UpToDate
```

**Issue**: `Dirty` always takes priority over `Diverged`. If the repo is both diverged
AND dirty (common after a failed rebase), the user only sees "Dirty" and never learns
about the diverged state.

### 2.2 DefaultSyncService (L79)

Constructor takes `repo_root` (Iron config dir path) and `StateManager`.

**Helper methods** (private):

| Method | Line | Git Command | Purpose |
|--------|------|-------------|---------|
| `git()` | L90 | `Command::new("git").args(args)` | Run any git command, return stdout |
| `is_repo()` | L112 | `.git` exists OR `rev-parse --git-dir` | Check git repo |
| `current_branch()` | L123 | `branch --show-current` | Get current branch name |
| `tracking_branch()` | L129 | `rev-parse --abbrev-ref @{upstream}` | Get upstream ref |
| `ahead_behind()` | L135 | `rev-list --left-right --count @{upstream}...HEAD` | Count divergence |
| `dirty_count()` | L148 | `status --porcelain` | Count changed files |
| `fetch()` | L154 | `fetch --quiet` | Fetch from remote |

**Error handling**: The `git()` helper at L90 maps any I/O error to `GitError::NotARepository`.
This is misleading — a network timeout during `git push` would be reported as "not a repository"
rather than a network error. Non-zero exit codes are mapped to `GitError::CommandFailed` with
stderr content, which is more appropriate.

### 2.3 Trait Methods Analysis

| Method | Line | Git Commands | Records Op | Notes |
|--------|------|-------------|------------|-------|
| `status()` | L160 | fetch + branch + rev-parse + rev-list + status | No | Fetches every call (slow on bad networks) |
| `pull()` | L210 | `pull --rebase` | `git_pull` | Always rebase, never merge |
| `push()` | L224 | `push` | `git_push` | No remote/branch params — uses defaults |
| `sync()` | L238 | check_conflicts + pull + push | `sync` (via maintenance) | Aborts on conflicts |
| `commit()` | L261 | `add -A` + `commit -m` | `git_commit` | Stages ALL files (`-A`) — potentially dangerous |
| `check_conflicts()` | L282 | fetch + `diff --name-only --diff-filter=U` | No | Only finds already-unmerged files |
| `stash()` | L297 | `stash push -m "iron auto-stash"` | No | No record_operation |
| `stash_pop()` | L309 | `stash pop` | No | No error recovery if pop conflicts |

**Key issue with `check_conflicts()` (L282)**: The `--diff-filter=U` flag only finds files
that are currently in an unmerged state (i.e., a merge/rebase is already in progress).
It does NOT predict whether a pull will cause conflicts. For conflict *prediction*, you would
need `git merge-base` + `git diff` between HEAD and FETCH_HEAD. This means `sync()` will
only abort if there's an *ongoing* merge conflict, not if a pull *would* create one.

**Key issue with `commit()` (L261)**: `git add -A` stages everything in the repo root,
including untracked files the user may not intend to commit. No `.gitignore` awareness
is enforced at the Iron level.

### 2.4 Tests (L321–717, 23 tests)

Tests use `tempfile::TempDir` with real git repos (via `git init`):

| Test | What It Covers |
|------|---------------|
| `test_sync_status_*` (5 tests) | SyncStatus equality, serialization, all variants |
| `test_sync_info_*` (5 tests) | SyncInfo creation, clone, debug, partial eq, serialization |
| `test_default_sync_service_not_a_repo` | NotARepo status for non-git dirs |
| `test_default_sync_service_clean_repo` | UpToDate on empty repo |
| `test_default_sync_service_dirty_repo` | Dirty status after file modification |
| `test_default_sync_service_multiple_dirty` | Multiple dirty file count |
| `test_default_sync_service_is_repo` | is_repo() on real git repo |
| `test_default_sync_service_commit` | Commit flow with real repo |
| `test_default_sync_service_check_conflicts_empty` | No unmerged files |
| `test_default_sync_service_stash_clean` | Stash on clean repo |
| `test_default_sync_service_status_has_branch` | Branch name in status |

**What's NOT tested**:
- `push()` — would need a remote
- `pull()` — would need a remote with commits
- `sync()` — compound operation untested
- `stash()`/`stash_pop()` with dirty files — only tested on clean repo
- Error paths (network failure, auth failure, merge conflict during pull)
- Ahead/Behind counts with real remotes

---

## 3. iron-git Layer — Deep Dive

**File**: `crates/iron-git/src/lib.rs` — 1,505 lines, 72 tests
**Test Fixtures**: `crates/iron-git/src/test_fixtures.rs` — 781 lines

### 3.1 Why This Layer Exists But Is Unused by Sync

`iron-git` provides a complete git abstraction with two key advantages over
`DefaultSyncService`:

1. **Circuit breaker resilience** via `CommandExecutor` (120s timeout, automatic
   circuit-breaking after repeated failures)
2. **Richer return types** — `PullResult` with `has_conflicts` and `conflict_files`,
   `GitStatus` with file-level detail (modified/untracked/staged lists)

Yet the sync workflow (both TUI and CLI) uses `DefaultSyncService` from `iron-core`
exclusively. `iron-git` is a dependency of the workspace but is not wired into the sync
path.

### 3.2 GitManager Trait (L77)

```rust
pub trait GitManager {
    fn status(&self) -> IronResult<GitStatus>;
    fn diff(&self) -> IronResult<String>;
    fn commit(&self, message: &str) -> IronResult<()>;
    fn push(&self, remote: &str, branch: &str) -> IronResult<()>;
    fn pull(&self, remote: &str, branch: &str) -> IronResult<PullResult>;
    fn has_changes(&self) -> IronResult<bool>;
}
```

Notable differences from `SyncService`:
- `push(remote, branch)` takes explicit remote/branch parameters
- `pull()` returns `PullResult` with conflict info, not just `()`
- No `sync()`, `stash()`, `stash_pop()` methods
- No `check_conflicts()` — conflict detection is embedded in `pull()` return

### 3.3 DefaultGitManager (L103)

- Stores `root: PathBuf` + optional `Arc<dyn CommandExecutor>`
- `new()` always creates a `RealCommandExecutor::with_defaults()` (circuit breaker enabled)
- Validates `.git` directory exists at construction time (unlike `SyncService` which
  checks at each method call)
- `run_git()` (L157): Routes through executor if available, falling back to direct
  `Command` if not

### 3.4 Conflict Detection in pull() (L218)

```rust
fn pull(&self, remote: &str, branch: &str) -> IronResult<PullResult> {
    // ... runs git pull ...
    // On error, checks for merge conflicts:
    //   - Runs git status --porcelain
    //   - Looks for "UU" or "AA" status lines
    //   - Returns GitError::MergeConflict { files }
}
```

This is **more sophisticated** than `SyncService::check_conflicts()` which only looks for
already-unmerged files. `GitManager::pull()` detects conflicts that *just occurred* during
the pull operation.

### 3.5 SecretsManager (L91, L258)

```rust
pub trait SecretsManager {
    fn is_unlocked(&self) -> bool;
    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()>;
    fn lock(&self) -> IronResult<()>;
    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>>;
}
```

`DefaultSecretsManager` wraps `git-crypt` commands. `is_unlocked()` checks for the
`\x00GITCRYPT` magic header in files under `secrets/`. Has `is_initialized()` checking
for `.git-crypt/` directory.

**Integration with Sync**: There is NO integration. The sync workflow does not check
whether secrets are locked before pushing, meaning encrypted secrets could be pushed
while unlocked (exposing plaintext in commits). The user-workflow spec says `secrets/`
should be synced "via git-crypt" but no enforcement exists.

### 3.6 Utility Functions

- `parse_git_status(output)` (L~413): Comprehensive porcelain parser — branch, ahead/behind,
  modified/untracked/staged/conflict categories. Handles UU/AA/DD conflict markers, renamed
  files, "No commits yet" branches.
- `parse_encrypted_files(output)` (L506): Parses `git-crypt status -e` output.
- `is_gitcrypt_encrypted(content)` (L541): Checks for `\x00GITCRYPT` magic header.

### 3.7 Test Infrastructure (72 tests + 781-line fixture file)

The `test_fixtures` module provides `GitMockBuilder` — a configurable mock `CommandExecutor`
that returns canned git output for specific commands. Tests cover:

- GitStatus construction and default values (3 tests)
- PullResult construction (2 tests)
- `parse_git_status()` — 15 tests covering clean, modified, untracked, staged, mixed,
  ahead/behind, no tracking, deleted, renamed, conflicts, new repo, empty, short lines,
  unrecognized indicators, spaces in filenames, large ahead/behind
- `parse_encrypted_files()` — 6 tests covering empty, single, multiple, whitespace,
  mixed output, nested paths, special chars
- `is_gitcrypt_encrypted()` — 7 tests covering true/false/edge cases
- MockGitManager — 7 tests covering clean/changes/diff/commit/push/pull
- Circuit breaker integration — 6 tests (with_resilience, with_executor, new)
- DefaultGitManager with MockExecutor — 6 tests (status, diff, has_changes, push, pull, root)
- DefaultGitManager edge cases — 2 tests (not_a_repository)
- DefaultSecretsManager — 7 tests (not_initialized, initialized, is_unlocked variants,
  unlock/lock/list when not initialized)
- Additional edge cases — 5 tests (MM, AM, DD, short line, parse status edge cases)

**What's NOT tested in iron-git**:
- Real git operations against actual remotes (all mocked)
- Conflict resolution flows
- SecretsManager unlock/lock with actual git-crypt

---

## 4. TUI Sync View — Deep Dive

### 4.1 Sync View Rendering (`ui/update.rs` L390, 82 lines)

The Sync view is rendered by `render_sync()` in the **update** module (not its own file).

**Two states**:

1. **sync_info is Some** — Shows status badge, branch, ahead/behind, dirty count, last sync
2. **sync_info is None** — Shows "Press [s] to check git sync status" with keybind hints

Status icon mapping:
```
UpToDate → ✓ (green)     Ahead   → ↑ (yellow)    Behind → ↓ (blue)
Diverged → ⇅ (red)       Dirty   → ● (peach)     NotARepo → ✗ (red)
```

**Missing from render vs spec**:
- Remote URL — spec shows "Remote: origin (git@github.com:user/iron-config.git)" but
  `SyncInfo` has no remote URL field
- Conflict file list — spec says "Status: ⚠ Diverged (3 conflicts)" with file listing,
  but no conflict data is stored in app state
- Sync progress indicator — push/pull are synchronous (block the UI thread)

### 4.2 Sync Handlers (`handlers.rs` L345, 19 lines)

Three keybinds in `View::Sync`:

| Key | Handler | Action |
|-----|---------|--------|
| `p` | `sync_push()` | Push to remote |
| `f` | `sync_pull()` | Pull from remote |
| `s` | `refresh_sync_status()` | Refresh status display |

**Missing handlers**:
- No `c` for commit (TUI push does NOT auto-commit like CLI push does)
- No confirm dialog before push/pull (other destructive actions like Update use
  `request_confirm()` with risk-differentiated dialogs)
- No conflict resolution keybinds

### 4.3 Sync Actions (`actions.rs` L593, 68 lines)

All three actions follow the same pattern:
1. Check `self.state_manager.is_some()`
2. Create `DefaultSyncService::new(&self.config_dir, sm.clone())`
3. Call service method
4. Set status/error message

**Key observations**:

- `sync_push()` calls `sync_service.push()` directly — NO auto-commit. If files are
  dirty, the push completes but uncommitted changes are NOT included. The CLI path
  (`commands/sync.rs` L125) checks for dirty status and auto-commits first.
- `sync_pull()` calls `sync_service.pull()` directly — NO dirty check, NO auto-stash,
  NO post-pull config application.
- All three actions create a **new DefaultSyncService every call** instead of caching it.
  Each `refresh_sync_status()` call runs `git fetch --quiet` which is a network call.

### 4.4 Navigation and Auto-Refresh

`navigate(View::Sync)` does NOT auto-refresh sync status (L345 in `mod.rs`). Only
`ModuleDetail` triggers auto-load on navigation. The user must manually press `s`
every time they enter the Sync view to see current status.

### 4.5 Sync State in App (`mod.rs` L116)

```rust
pub sync_info: Option<SyncInfo>,
```

Single `Option<SyncInfo>` field. Initialized to `None`. No conflict state, no push/pull
progress, no error history. The absolute minimum for a sync view.

---

## 5. CLI Sync Command — Deep Dive

**File**: `crates/iron-cli/src/commands/sync.rs` — 247 lines, **0 tests**

### 5.1 SyncAction Enum (`cli.rs` L362)

```rust
pub enum SyncAction {
    Status,
    Push { message: Option<String> },
    Pull { stash: bool },
}
```

CLI definition tests at `cli.rs` L721: 2 tests (parse status, parse push with message).

### 5.2 Status (`commands/sync.rs` L35)

Calls `sync_service.status()` and displays:
- Branch name, remote branch, status badge (emoji + color), ahead/behind counts,
  dirty file count.
- Supports `--json` output via a local `SyncInfo` struct (re-serializes the core one).

### 5.3 Push (`commands/sync.rs` L125)

```
1. sync_service.status()
2. If Behind or Diverged → error("Push rejected")
3. If Dirty → sync_service.commit(message.unwrap_or("Iron sync"))
4. sync_service.push()
```

**This is significantly smarter than the TUI push**:
- Checks status first and blocks push if behind/diverged
- Auto-commits dirty files with a default or custom message
- The TUI just calls `push()` blindly without any pre-checks

### 5.4 Pull (`commands/sync.rs` L173)

```
1. sync_service.status()
2. If Dirty + stash flag → stash + pull + stash_pop
3. If Dirty + no stash → interactive y/N prompt
4. If Diverged → "Attempting to pull with rebase..."
5. sync_service.pull()
6. On error → "Manual intervention may be required"
```

**Also significantly smarter than TUI pull**:
- Handles dirty state via stash or interactive prompt
- Warns about diverged state
- Provides error recovery guidance

### 5.5 CLI ↔ TUI Parity Analysis

| Feature | CLI | TUI | Gap |
|---------|-----|-----|-----|
| Status display | ✓ With JSON | ✓ Visual | CLI has JSON output, TUI has colored icons |
| Pre-push status check | ✓ Blocks if behind | ✗ | **P1 gap** |
| Auto-commit on push | ✓ With message | ✗ | **P1 gap** — TUI pushes without committing |
| Pull dirty check | ✓ Prompt or stash | ✗ | **P1 gap** |
| Pull stash support | ✓ `--stash` flag | ✗ | **P2 gap** |
| Post-pull config apply | ✗ | ✗ | Both missing per spec |
| Conflict resolution | ✗ | ✗ | Both missing per spec |
| Confirm dialog | N/A (interactive y/N) | ✗ | **P2 gap** |

---

## 6. Task S1-P8-001 — Sync Conflict Resolution UI

### 6.1 Original Task Definition

From `docs/TODO-scenario1.md` L241:

```
S1-P8-001 | P2 | Sync conflict resolution UI
  Why: user-workflow describes a merge conflict resolution flow in the TUI.
  Action: Add conflict detection to Sync view, show conflicted files with
    options: "keep local" / "keep remote" / "open diff".
  Files: crates/iron-tui/src/ui/sync.rs, crates/iron-git/src/lib.rs
  Test: Render test with conflict state.
```

**Note**: The file reference `ui/sync.rs` does not exist. The sync view is rendered in
`ui/update.rs`.

### 6.2 Current State

**What exists**:
- `SyncStatus::Diverged` is defined and rendered (⇅ red icon)
- `SyncService::check_conflicts()` can list unmerged files (but only after a merge fails)
- `GitManager::pull()` returns `PullResult` with `conflict_files` (but iron-git is unused)
- TUI shows "Diverged" but no conflict details

**What's completely missing**:
- No conflict state in `App` (no field for conflict files)
- No conflict file list rendering
- No "keep local" / "keep remote" / "open diff" actions
- No git checkout --ours/--theirs integration
- No post-conflict-resolution merge continue flow

### 6.3 Implementation Approach

To implement this task properly:

**Phase A — Conflict Detection** (prerequisite):
1. Add `conflict_files: Vec<String>` to `App` state
2. After `sync_pull()` fails, run `check_conflicts()` to populate the list
3. Alternatively, switch to using `iron-git::GitManager::pull()` which returns
   `PullResult` with conflicts already detected

**Phase B — Conflict Display**:
1. Expand `render_sync()` to show conflict file list when `Diverged`
2. Add file-level navigation (j/k to select conflicted files)
3. Show per-file status (UU = both modified, AA = both added, etc.)

**Phase C — Resolution Actions**:
1. `o` — Keep ours: `git checkout --ours <file>` + `git add <file>`
2. `t` — Keep theirs: `git checkout --theirs <file>` + `git add <file>`
3. `d` — Open diff: shell out to configured diff tool or show inline
4. After all conflicts resolved: `git rebase --continue`

**Phase D — Wire iron-git**:
1. Consider routing sync operations through `GitManager` instead of raw commands
2. This would provide circuit breaker resilience and richer return types
3. Alternatively, port resilience features into `SyncService`

### 6.4 Dependencies and Risk

- Requires extending `SyncService` trait with conflict resolution methods
  (`resolve_ours`, `resolve_theirs`, `continue_rebase`)
- The `--rebase` mode in `pull()` means conflicts appear differently than merge conflicts
  (rebase conflicts show one commit at a time)
- `stash_pop()` can also conflict — this is unhandled entirely

---

## 7. Discovered Issues — Outside Original Phase 8 Scope

### D-P8-001 (P1) — TUI Push Does Not Auto-Commit

**Location**: `actions.rs` L617–636  
**Problem**: `sync_push()` calls `sync_service.push()` directly without checking for
dirty files or auto-committing. If the user has uncommitted changes, push succeeds but
those changes are NOT included.  
**Spec says**: UC-19 step 2 — "`SyncService::commit()` stages all changes"  
**CLI behavior**: CLI `push()` at `commands/sync.rs` L125 checks dirty status and
auto-commits with message.  
**Fix**: Add dirty check → auto-commit → push sequence to `sync_push()`, matching CLI.

### D-P8-002 (P1) — TUI Pull Has No Dirty Check or Stash

**Location**: `actions.rs` L643–660  
**Problem**: `sync_pull()` calls `sync_service.pull()` without checking if the working
tree is dirty. `git pull --rebase` on a dirty tree fails with an error.  
**Spec says**: UC-20 implies clean pull. CLI has `--stash` flag and interactive prompt.  
**Fix**: Check dirty status before pull. If dirty, either auto-stash or show confirm
dialog with stash option.

### D-P8-003 (P2) — No Confirm Dialog for Push/Pull

**Location**: `handlers.rs` L345–361  
**Problem**: Push and pull execute **immediately** when the key is pressed. Other
destructive operations (system update, cleanup) use `request_confirm()` with
risk-differentiated confirmation dialogs.  
**Risk**: Accidental push could push unwanted changes. Accidental pull could rebase
on top of unexpected remote changes.  
**Fix**: Add confirm dialog at minimum for push (since it modifies remote). Pull could
use Simple confirmation.

### D-P8-004 (P2) — No Auto-Refresh on Sync View Navigation

**Location**: `mod.rs` L345 (`navigate()`)  
**Problem**: Entering the Sync view shows stale or empty state. The user must manually
press `s` to see sync status. Other views like `ModuleDetail` auto-load data.  
**Fix**: Add `View::Sync` case to `navigate()` that calls `refresh_sync_status()`.

### D-P8-005 (P2) — SyncService Does Not Use iron-git (Code Duplication)

**Location**: `services/sync.rs` vs `iron-git/src/lib.rs`  
**Problem**: Two independent git layers doing the same things. `SyncService` uses raw
`Command::new("git")` without circuit breaker resilience. `iron-git::GitManager` has
circuit breaker, richer types, better conflict detection — but is unused by sync.  
**Impact**: Git operations in sync path have no timeout protection. A `git push` to an
unresponsive remote blocks the TUI thread indefinitely.  
**Fix**: Either route `SyncService` through `GitManager`, or add `CommandExecutor`
resilience to `SyncService`.

### D-P8-006 (P2) — No Post-Pull Config Application

**Location**: `actions.rs` L643, `commands/sync.rs` L173  
**Problem**: After a successful pull, neither TUI nor CLI detects changed configs or
re-links dotfiles. The spec says (UC-20 step 3): "If the pull contains config changes,
Iron detects and applies them — updating state and re-linking dotfiles as needed."  
**Fix**: After pull, diff the pulled changes for config files. If any TOML configs or
dotfiles changed, trigger re-linking via `FileService::deploy()`.

### D-P8-007 (P2) — Misleading Error Mapping in git() Helper

**Location**: `services/sync.rs` L97  
**Problem**: Any `std::io::Error` (command not found, permission denied, timeout) is
mapped to `GitError::NotARepository`. A network error during push would show "not a
git repository" to the user.  
**Fix**: Map I/O errors to `GitError::CommandFailed` instead, or add a new
`GitError::IoError` variant.

### D-P8-008 (P3) — commit() Uses git add -A (Stages Everything)

**Location**: `services/sync.rs` L271  
**Problem**: `git add -A` stages ALL files including new untracked files. If the user
created a temporary file in the Iron config dir, it gets committed.  
**Mitigation**: Iron's `.gitignore` should catch most cases, but the service doesn't
verify `.gitignore` coverage.  
**Fix**: Consider `git add .` with explicit paths, or show a preview of what will be
staged before committing.

### D-P8-009 (P3) — check_conflicts() Only Finds Existing Unmerged Files

**Location**: `services/sync.rs` L282  
**Problem**: `git diff --name-only --diff-filter=U` only lists files that are already
in an unmerged state (merge/rebase in progress). It cannot predict whether a pull will
cause conflicts. The `sync()` method calls `check_conflicts()` first but this is a
no-op for a clean repo about to pull diverged changes.  
**Fix**: For predictive conflict detection, compare `HEAD` with `FETCH_HEAD` after fetch:
`git diff --name-only HEAD FETCH_HEAD` to find files that differ.

### D-P8-010 (P3) — Secrets Not Locked Before Push

**Location**: `actions.rs` L617, `commands/sync.rs` L125  
**Problem**: Neither push path checks whether secrets are unlocked before pushing.
If the user unlocked secrets (via git-crypt unlock), modified a secret file, and pushed,
the plaintext secret would be committed.  
**Note**: git-crypt's clean/smudge filters should handle this transparently if properly
configured, but the Iron workflow doesn't verify the git-crypt filter is active.  
**Fix**: Add a pre-push check via `SecretsManager::is_unlocked()` with a warning.

### D-P8-011 (P3) — Push/Pull Block TUI Thread

**Location**: `actions.rs` L617–660  
**Problem**: `sync_push()` and `sync_pull()` are synchronous. A slow network connection
blocks the entire TUI until the git operation completes (or hangs indefinitely for
unresponsive remotes).  
**Impact**: UI becomes unresponsive during git operations.  
**Fix**: Either use `iron-git`'s `CommandExecutor` (120s timeout), or spawn git operations
in a background thread with progress feedback.

### D-P8-012 (P3) — SyncService Creates Fresh Instance Per Action

**Location**: `actions.rs` L598, L618, L643  
**Problem**: Each action creates `DefaultSyncService::new(&self.config_dir, sm.clone())`.
The `refresh_sync_status()` call inside `sync_push()` and `sync_pull()` creates ANOTHER
instance. Each `status()` call does `git fetch --quiet` — so a push-then-refresh does
two fetches.  
**Fix**: Cache the `DefaultSyncService` in `App` state, or at minimum share within a
single action sequence.

---

## 8. Integration Map

### Cross-Crate Dependencies

```
 iron-cli ──────────────────── iron-core ──────────── iron-git
    │                              │                      │
    ├─ commands/sync.rs            ├─ services/sync.rs    ├─ lib.rs (GitManager)
    │   uses SyncService trait     │   raw Command calls  │   CommandExecutor
    │   uses DefaultSyncService    │   NO iron-git dep    │   circuit breaker
    │                              │                      │
    │                              │                      ├─ lib.rs (SecretsManager)
    │                              │                      │   git-crypt wrapper
    │                              │                      │
 iron-tui                          │                      ├─ test_fixtures.rs
    │                              │                      │   GitMockBuilder
    ├─ app/actions.rs              │                      │
    │   uses DefaultSyncService ───┘                      │
    │   NO iron-git dep                                   │
    │                                                     │
    └─ (iron-tui has NO dependency on iron-git)           │
                                                          │
                    Unused by Sync ◄──────────────────────┘
```

### Feature Parity Matrix

| Feature | SyncService | GitManager | Notes |
|---------|------------|------------|-------|
| Status | `status()` → SyncInfo | `status()` → GitStatus | Different structs: SyncInfo has sync-level data, GitStatus has file-level |
| Push | `push()` → () | `push(remote, branch)` → () | SyncService uses defaults, GitManager takes params |
| Pull | `pull()` → () | `pull(remote, branch)` → PullResult | GitManager returns conflict info |
| Commit | `commit(msg)` → () | `commit(msg)` → () | Similar |
| Diff | N/A | `diff()` → String | Only in GitManager |
| Has Changes | N/A (dirty_count > 0) | `has_changes()` → bool | Different approach |
| Stash | `stash()`, `stash_pop()` | N/A | Only in SyncService |
| Conflicts | `check_conflicts()` (unmerged) | Embedded in pull() return | GitManager is better |
| Circuit Breaker | None | CommandExecutor with timeout | Major gap |
| Fetch | `fetch()` private helper | N/A (inside pull) | SyncService fetches explicitly |

### State Flow

```
                     ┌─────── App State ──────────┐
                     │                             │
                     │  sync_info: Option<SyncInfo>│
                     │  (single field — no         │
                     │   conflicts, progress,      │
                     │   or push/pull history)     │
                     │                             │
                     └─────────────┬───────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                     │
              ▼                    ▼                     ▼
    refresh_sync_status()    sync_push()           sync_pull()
              │                    │                     │
              ▼                    ▼                     ▼
    DefaultSyncService ──── DefaultSyncService ── DefaultSyncService
    (new instance)          (new instance)        (new instance)
              │                    │                     │
              ▼                    ▼                     ▼
         status()              push()               pull()
      ↓ git fetch           ↓ git push          ↓ git pull --rebase
      ↓ git branch
      ↓ git rev-list
      ↓ git status
```

---

## 9. Test Coverage Analysis

### Summary Table

| Component | File | LOC | Tests | Test Coverage |
|-----------|------|-----|-------|---------------|
| SyncService | `services/sync.rs` | 717 | 23 | Good struct coverage, no push/pull/sync integration |
| iron-git | `lib.rs` | 1,505 | 72 | Excellent parsing + mock coverage, no real remote ops |
| iron-git fixtures | `test_fixtures.rs` | 781 | — | Infrastructure only |
| CLI sync | `commands/sync.rs` | 247 | **0** | **No tests at all** |
| CLI definition | `cli.rs` | — | 2 | Parse-only (sync status, sync push) |
| TUI render | `ui/update.rs` (sync portion) | 82 | 1 | Basic render smoke test |
| TUI handlers | `handlers.rs` | 19 | 0 (sync-specific) | Covered by general Tab/navigation tests |
| TUI actions | `actions.rs` | 68 | 0 (sync-specific) | **No tests for sync actions** |

### Total Phase 8 Test Count

| Layer | Count | Notes |
|-------|-------|-------|
| iron-core SyncService | 23 | Struct, serialization, real git repos (no remotes) |
| iron-git | 72 | Comprehensive: parsing, mocks, circuit breaker, edge cases |
| CLI parsing | 2 | SyncAction parse tests only |
| TUI render | 1 | Smoke test: renders without panic |
| **Total** | **98** | |

### Critical Test Gaps

1. **CLI sync commands/sync.rs has ZERO tests** — All logic (pre-push status check,
   auto-commit, stash handling, interactive prompts) is completely untested.

2. **TUI sync actions have ZERO tests** — `refresh_sync_status()`, `sync_push()`,
   `sync_pull()` are untested. Since these have bugs (no auto-commit, no dirty check),
   tests would catch the divergence from CLI behavior.

3. **No integration tests for push/pull** — Both `SyncService` and `GitManager` test
   suites only test against local repos with no remotes. Real sync scenarios (push to
   remote, pull with conflicts, diverged repos) are untested.

4. **No conflict flow tests** — The task S1-P8-001 specifically mentions conflict
   resolution, but there are zero tests for any conflict detection or resolution path.

5. **iron-git is heavily tested but unused** — 72 tests for a module that contributes
   nothing to the sync workflow. These tests validate good code that should be wired in.

### Recommendations

**Immediate (P1)**:
- File new tasks for D-P8-001 and D-P8-002 (auto-commit + dirty check)
- Add tests for CLI `commands/sync.rs` covering push and pull flows
- Add tests for TUI sync actions

**Short-term (P2)**:
- Implement S1-P8-001 (conflict resolution UI)
- Wire `iron-git::GitManager` into sync path for circuit breaker resilience
- Add auto-refresh on Sync view navigation
- Add confirm dialog for push/pull

**Long-term (P3)**:
- Implement post-pull config detection and re-linking
- Add background threading for git operations (unblock TUI)
- Add secrets lock check before push
- Consolidate the two git layers into one
