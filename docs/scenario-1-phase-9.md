# Scenario 1 — Phase 9: Security & Secrets

## Implementation Guideline (Deep Dive)

> **Scope**: Task S1-P9-001 from `docs/TODO-scenario1.md` + Security & Secrets (user-workflow Phase 9 / Workflows D & E)
> **Phase**: SecretsService, git-crypt integration, TUI Secrets/Security/Recovery views, CLI secrets/recover commands
> **Generated**: 2026-02-19
> **Based on**: Deep codebase analysis across iron-core, iron-tui, iron-cli, iron-git

---

## Table of Contents

1. [Phase 9 Architecture Overview](#1-phase-9-architecture-overview)
2. [SecretsService (iron-core) — Deep Dive](#2-secretsservice-iron-core--deep-dive)
3. [RecoveryService (iron-core) — Deep Dive](#3-recoveryservice-iron-core--deep-dive)
4. [iron-git SecretsManager — Deep Dive](#4-iron-git-secretsmanager--deep-dive)
5. [TUI Secrets View — Deep Dive](#5-tui-secrets-view--deep-dive)
6. [TUI Security Modules View — Deep Dive](#6-tui-security-modules-view--deep-dive)
7. [TUI Recovery View — Deep Dive](#7-tui-recovery-view--deep-dive)
8. [CLI Secrets Command — Deep Dive](#8-cli-secrets-command--deep-dive)
9. [CLI Recover Command — Deep Dive](#9-cli-recover-command--deep-dive)
10. [Task S1-P9-001 — Secrets Encrypt/Decrypt File Actions](#10-task-s1-p9-001)
11. [Discovered Issues — Outside Original Phase 9 Scope](#11-discovered-issues)
12. [Integration Map](#12-integration-map)
13. [Test Coverage Analysis](#13-test-coverage-analysis)

---

## 1. Phase 9 Architecture Overview

### What Phase 9 Covers

Phase 9 spans three related subsystems:

1. **Secrets Management** — git-crypt integration for encrypting/decrypting sensitive files
   (SSH keys, API tokens, GPG keys) within the Iron repo.
2. **Security Modules** — Dedicated TUI view for security-related modules (ufw, fail2ban,
   apparmor, etc.) with enable/disable toggles.
3. **Recovery** — State export/import, install script generation, backup/restore for
   disaster recovery and cross-machine replication.

### Key Architectural Finding: Two Independent Secrets Layers

Iron has **two separate secrets abstractions** that overlap significantly:

| Layer | Location | Purpose | Used By |
|-------|----------|---------|---------|
| `SecretsService` | `iron-core/services/secrets.rs` | 10-method trait wrapping `git-crypt` | CLI `iron secrets` |
| `SecretsManager` | `iron-git/src/lib.rs` | 4-method trait wrapping `git-crypt` | Nothing in current codebase |

**Neither layer is used by the TUI.** The TUI Secrets view renders UI chrome from
hardcoded state fields (`secrets_status: Option<String>`, `encrypted_files: Vec<PathBuf>`)
but has **zero action handlers** — no keybinds are wired. The `[i]`, `[u]`, `[l]`, `[a]`
keys shown in the UI do nothing.

### Key Architectural Finding: TUI Views Are Render-Only Shells

**This is the most critical Phase 9 discovery.** Three TUI views — Secrets, Recovery,
and (partially) Security — are visual shells with no backend wiring:

| View | Renders | Has Handlers | Has Actions | State Populated |
|------|---------|-------------|-------------|-----------------|
| Secrets | Status + file list + keybind hints | **NO** | **NO** | **NEVER** (always None/empty) |
| Recovery | Status + action list + keybind hints | **NO** | **NO** | **NEVER** (always None) |
| SecurityModules | Module list with toggle | **YES** (e/i toggle, j/k nav) | YES (via toggle_selected_module) | YES (from modules list) |

Secrets and Recovery views navigate correctly (`Shift+S`, `Shift+R`) and display
beautifully, but pressing any action key is a no-op because `View::Secrets` and
`View::Recovery` are not matched in the handler's view-specific `match` block.

### Key Components

| Component | File | Lines | Tests | Purpose |
|-----------|------|-------|-------|---------|
| SecretsStatus enum | `services/secrets.rs` L12 | 10 | 7 | 4 states: NotInitialized, Locked, Unlocked, NotAvailable |
| GpgKey struct | `services/secrets.rs` L23 | 8 | 5 | Key ID + user ID + trust level |
| SecretsService trait | `services/secrets.rs` L34 | 28 | — | 10 methods: status, init, unlock, lock, add_gpg_user, list_keys, export_key, is_encrypted, list_encrypted |
| DefaultSecretsService | `services/secrets.rs` L68 | 230 | 33 | Full impl wrapping `git-crypt` commands |
| RecoveryExport struct | `services/recovery.rs` L17 | 22 | 4 | Serializable state snapshot |
| InstallScriptOptions | `services/recovery.rs` L41 | 14 | 3 | Script generation config flags |
| RecoveryService trait | `services/recovery.rs` L58 | 18 | — | 8 methods: export, import, generate_install_script, save/load_export, create/restore_backup |
| DefaultRecoveryService | `services/recovery.rs` L83 | ~370 | 18 | Full impl with pacman/systemctl queries + tar backup |
| SecretsManager trait | `iron-git/lib.rs` L91 | 10 | — | 4 methods: is_unlocked, unlock, lock, list_encrypted |
| DefaultSecretsManager | `iron-git/lib.rs` L258 | 60 | 7 | git-crypt wrapper with CommandExecutor circuit breaker |
| SecurityCategory enum | `ui/security.rs` L17 | 8 | 1 | 4 categories: Firewall, IntrusionDetection, AuditLogging, AccessControl |
| TUI render_secrets | `ui/secrets.rs` | 122 | 0 | Secrets view rendering (status + file list + hints) |
| TUI render_security_modules | `ui/security.rs` | 234 | 3 | Security modules table view |
| TUI render_recovery | `ui/recovery.rs` | 151 | 0 | Recovery view rendering (status + action descriptions + hints) |
| CLI secrets command | `commands/secrets.rs` | 290 | 0 | status, unlock, lock, link subcommands |
| CLI recover command | `commands/recover.rs` | 250 | 0 | export, import, script subcommands |
| CLI SecretsAction enum | `cli.rs` L383 | 15 | 1 | Status, Unlock{key}, Lock, Link |
| CLI Recover args | `cli.rs` L166 | 12 | 2 | --export, --import, --script flags |

---

## 2. SecretsService (iron-core) — Deep Dive

**File**: `crates/iron-core/src/services/secrets.rs` — 908 lines, 45 tests

### 2.1 SecretsStatus Enum (L12)

```rust
pub enum SecretsStatus {
    NotInitialized,  // No .git-crypt directory
    Locked,          // Initialized but files encrypted
    Unlocked,        // Files decrypted and accessible
    NotAvailable,    // git-crypt binary not found
}
```

Detection at `status()` (L138):
```
git-crypt not available → NotAvailable
.git-crypt not exists → NotInitialized
.git/git-crypt/keys/ exists → Unlocked
else → Locked
```

### 2.2 DefaultSecretsService (L68)

Constructor takes only `repo_root`. No `StateManager` dependency (unlike most other
services). Operations are not recorded in the audit log.

**Helper methods** (private):

| Method | Line | Purpose |
|--------|------|---------|
| `git_crypt_available()` | L82 | `git-crypt --version` → bool |
| `git_crypt()` | L90 | Run git-crypt command, return stdout |
| `is_initialized()` | L120 | Check `.git-crypt` directory exists |
| `is_unlocked()` | L125 | Check `.git/git-crypt/keys/` directory exists |

### 2.3 Trait Methods Analysis

| Method | Line | Implementation | Notes |
|--------|------|---------------|-------|
| `status()` | L138 | Check available → initialized → unlocked | Clean logic |
| `init()` | L152 | `git-crypt init` | Errors if already initialized |
| `unlock()` | L167 | `git-crypt unlock [key_path]` | GPG or key file |
| `lock()` | L185 | `git-crypt lock` | Errors if not initialized |
| `add_gpg_user()` | L196 | `git-crypt add-gpg-user <id>` | Errors if not initialized |
| `list_keys()` | L207 | Reads `.git-crypt/keys/default/0/*.gpg` + `gpg --list-keys` | Filesystem scan + GPG query |
| `export_key()` | L251 | `git-crypt export-key <path>` | Symmetric key export |
| `is_encrypted()` | L264 | Check file starts with `\x00GITCRYPT` (9 bytes) | Same magic header as iron-git |
| `list_encrypted()` | L277 | Parse `.gitattributes` for `filter=git-crypt` + walk `secrets/` dir | **Pattern ignored** — always lists all files in `secrets/` regardless of `.gitattributes` |

**Key issue with `list_encrypted()` (L277)**: The method parses `.gitattributes` to find
git-crypt patterns, but then ignores the parsed patterns entirely. It simply walks all
files under `secrets/` directory. Files outside `secrets/` that match git-crypt patterns
in `.gitattributes` are not listed. Files inside `secrets/` that are NOT encrypted (e.g.,
a README) are falsely listed as encrypted.

**Key issue: No audit logging**: Unlike `SyncService` which calls
`state_manager.record_operation()` for push/pull/commit, `SecretsService` has no
`StateManager` dependency and records nothing. An unlock/lock cycle leaves no trace in
the operation log.

### 2.4 Error Handling

All errors use `ServiceError::NotAvailable` or `ServiceError::OperationFailed` from
iron-core's error hierarchy. Error messages are specific and helpful (e.g., "Repository
not initialized with git-crypt").

The `git_crypt()` helper at L90 has the same I/O error misattribution as `SyncService`:
any I/O error (permission denied, command not found) becomes `ServiceError::NotAvailable`
rather than a more descriptive error.

### 2.5 Tests (L310–908, 45 tests)

Tests use `tempfile::TempDir` to create mock filesystem structures:

| Category | Count | Coverage |
|----------|-------|---------|
| SecretsStatus (equality, clone, copy, debug, serialization) | 7 | Struct completeness |
| GpgKey (creation, clone, debug, serialization, empty) | 5 | Struct completeness |
| DefaultSecretsService (status, is_initialized, is_encrypted) | 7 | Core detection |
| is_unlocked (no git dir, with keys, partial path) | 3 | Unlock detection |
| status path tests (locked, unlocked) | 2 | Status determination |
| list_encrypted (no dir, empty, with files, nested, gitattributes) | 6 | File listing |
| list_keys (empty, with gpg files, ignores non-gpg) | 3 | Key listing |
| is_encrypted edge cases (nonexistent, short, exact, empty, binary, trailing, directory) | 7 | Encryption detection |
| Error paths (init/unlock/lock/add_gpg/export_key when not initialized) | 5 | Error handling |

**What's NOT tested**:
- Actual `git-crypt` operations (all tests use filesystem mocking, never run git-crypt)
- `unlock()` with a key file argument
- `export_key()` success path
- Interaction between `status()` changes after `lock()`/`unlock()` calls
- `.gitattributes` pattern matching (tests show patterns are parsed but never used)

---

## 3. RecoveryService (iron-core) — Deep Dive

**File**: `crates/iron-core/src/services/recovery.rs` — 969 lines, 25 tests

### 3.1 RecoveryExport Struct (L17)

```rust
pub struct RecoveryExport {
    pub version: String,              // "1.0"
    pub timestamp: DateTime<Utc>,
    pub host_id: String,
    pub active_bundle: Option<String>,
    pub active_profile: Option<String>,
    pub active_modules: Vec<String>,
    pub packages: Vec<String>,        // pacman -Qqe output
    pub aur_packages: Vec<String>,    // pacman -Qqm output
    pub services: Vec<String>,        // systemctl --user enabled units
}
```

### 3.2 DefaultRecoveryService (L83)

Generic over `S: SnapshotManager`. In practice, always instantiated with `NoopManager`.

**System queries** (raw `Command` calls):

| Method | Line | Command | Purpose |
|--------|------|---------|---------|
| `get_installed_packages()` | L100 | `pacman -Qqe` | List explicitly installed packages |
| `get_aur_packages()` | L113 | `pacman -Qqm` | List foreign (AUR) packages |
| `get_enabled_services()` | L126 | `systemctl --user list-unit-files --state=enabled` | List enabled user services |

### 3.3 Trait Methods Analysis

| Method | Line | Implementation | Notes |
|--------|------|---------------|-------|
| `export()` | L148 | Reads state + queries pacman/systemctl | Live system snapshot |
| `import()` | L176 | Sets host/bundle/profile/modules in StateManager | State-only, no package install |
| `generate_install_script()` | L199 | Builds bash script from export data | Interactive/non-interactive modes |
| `save_export()` | L345 | JSON serialize to file | `serde_json::to_string_pretty` |
| `load_export()` | L358 | JSON deserialize from file | With error handling |
| `create_backup()` | L370 | Snapshot + copy dirs + tar archive | Creates `.tar.gz` with hosts/bundles/profiles/modules + state.json |
| `restore_backup()` | L415 | Extract tar + load state + copy dirs back | Full restore flow |

**Key issue with `import()` (L176)**: Import only restores state metadata (host, bundle,
profile, modules) via `StateManager`. It does NOT install packages, enable services, or
deploy dotfiles. The user must run the generated install script or manually apply changes.
The spec's 4-step recovery flow (Install → Bundle → Profile → Verify) is NOT implemented
in `import()`.

**Key issue with `create_backup()` (L370)**: Calls `self.snapshot_manager.create("pre-backup")`
but `NoopManager` is always used, so no actual snapshot is taken. The backup only includes
state.json and config directories — not actual dotfiles from `~/.config`, system packages,
or service state.

### 3.4 Install Script Generation (L199)

generates a bash script with sections:
1. Official packages: `sudo pacman -S --needed --noconfirm <packages>`
2. AUR packages: `<helper> -S --needed --noconfirm <aur-packages>` (defaults to `paru`)
3. User services: `systemctl --user enable <service>`
4. Iron commands: `iron bundle activate <bundle>`, `iron module enable <module>`

In interactive mode, adds `confirm()` function and wraps each section in `if confirm`.

**Missing from script**: No dotfile deployment, no git-crypt unlock, no post-install
verification, no secrets linking. The spec says the 4-step recovery should include
"Verify: check drivers, services, permissions, symlinks" but this is absent.

### 3.5 Tests (L500–969, 25 tests)

| Category | Count | Coverage |
|----------|-------|---------|
| RecoveryExport (serialization, clone, debug, empty) | 4 | Struct completeness |
| InstallScriptOptions (default, clone, debug) | 3 | Struct completeness |
| export() (basic, with bundle/profile) | 2 | Export logic |
| generate_install_script() (basic, interactive, AUR default, services only) | 4 | Script generation |
| save/load export (success, not found, invalid JSON) | 3 | File I/O |
| import() (full, no bundle, modules only) | 3 | State restoration |
| copy_dir_recursive (simple, nested, empty, creates dst) | 4 | Utility |
| Backup/restore (creates dirs, nonexistent restore) | 2 | Limited — tar may not be available |

---

## 4. iron-git SecretsManager — Deep Dive

**File**: `crates/iron-git/src/lib.rs` L91 + L258 — ~70 lines, 7 tests

### 4.1 SecretsManager Trait (L91)

```rust
pub trait SecretsManager {
    fn is_unlocked(&self) -> bool;
    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()>;
    fn lock(&self) -> IronResult<()>;
    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>>;
}
```

Only 4 methods vs SecretsService's 10 methods. Missing: `status()`, `init()`,
`add_gpg_user()`, `list_keys()`, `export_key()`, `is_encrypted()`.

### 4.2 DefaultSecretsManager (L258)

- Has optional `Arc<dyn CommandExecutor>` for circuit breaker resilience
- `is_unlocked()` checks files in `secrets/` for the `\x00GITCRYPT` magic header
  (different approach than `SecretsService` which checks `.git/git-crypt/keys/`)
- `unlock()`/`lock()` wraps `git-crypt` commands via `CommandExecutor`
- `list_encrypted()` runs `git-crypt status -e` (vs SecretsService which walks filesystem)

### 4.3 Comparison: SecretsService vs SecretsManager

| Feature | SecretsService (iron-core) | SecretsManager (iron-git) |
|---------|--------------------------|--------------------------|
| `is_unlocked()` detection | `.git/git-crypt/keys/` dir exists | Reads file headers for `\x00GITCRYPT` |
| `list_encrypted()` | Walks `secrets/` dir (ignores patterns) | `git-crypt status -e` (accurate) |
| Circuit breaker | None | `CommandExecutor` with timeout |
| Init support | `init()` method | None |
| GPG key management | `add_gpg_user()`, `list_keys()` | None |
| Key export | `export_key()` | None |
| `is_encrypted()` per file | Yes (reads file header) | No direct method |
| Used by | CLI `iron secrets` | **Nothing** |

**The two detection methods can disagree**: `SecretsService.is_unlocked()` checks for the
presence of the keys directory, while `SecretsManager.is_unlocked()` reads actual file
content. If git-crypt state is inconsistent (e.g., keys directory removed but files remain
decrypted), the two would report different statuses.

---

## 5. TUI Secrets View — Deep Dive

**File**: `crates/iron-tui/src/ui/secrets.rs` — 122 lines, 0 tests

### 5.1 Rendering (`render_secrets`, L9)

Four-panel layout:
1. Title bar — "Secrets - git-crypt encrypted file management"
2. Status panel — Shows status icon based on `app.secrets_status` string matching
3. Encrypted files list — Shows files from `app.encrypted_files` or empty message
4. Footer hints — `[i] Init [u] Unlock [l] Lock [Esc] Back`

Status matching at L56 uses string comparison:
```rust
match app.secrets_status.as_deref() {
    Some("Unlocked") => ("[OK]", "Unlocked — secrets are decrypted", GREEN),
    Some("Locked") => ("[--]", "Locked — secrets are encrypted", YELLOW),
    Some("NotInitialized") => ("[!!]", "Not initialized", YELLOW),
    Some("NotAvailable") => ("[XX]", "git-crypt not installed", RED),
    _ => ("[ ?]", "Unknown — press [r] to refresh", OVERLAY),
}
```

Footer also shows `[a]` for "Add GPG key" in the help overlay but NOT in the footer
bar itself.

### 5.2 The Critical Gap: No Handlers, No Actions

**There is NO `View::Secrets` match arm in the handler.** The view-specific handler
`match` in `handlers.rs` goes from `View::SecurityModules` to `View::Settings`.
`View::Secrets` falls through to the default `_ => false` case, meaning ALL keybinds
except global ones (q, ?, Tab, Esc) are ignored.

**There are no action methods** in `actions.rs` for secrets operations. No
`refresh_secrets_status()`, `secrets_init()`, `secrets_unlock()`, `secrets_lock()`, or
`secrets_add_gpg_key()` methods exist.

**The state fields are never populated**:
- `app.secrets_status` — initialized to `None`, never set → always shows "Unknown"
- `app.encrypted_files` — initialized to `Vec::new()`, never populated → always shows "No encrypted files"

**Bottom line**: The Secrets view is a **pure visual mockup**. It renders a beautiful UI
with keybind hints but every key shown is dead.

---

## 6. TUI Security Modules View — Deep Dive

**File**: `crates/iron-tui/src/ui/security.rs` — 234 lines, 3 tests

### 6.1 Overview

Unlike Secrets and Recovery, the SecurityModules view is **partially functional**:
- Has view-specific handlers at `handlers.rs` L291: j/k navigation, Enter/e toggle, i install
- Uses `toggle_selected_module()` to enable/disable modules
- Filters module list to known security IDs: `ufw`, `firewalld`, `fail2ban`, `auditd`,
  `apparmor`, `selinux`, `clamav`

### 6.2 Rendering

Two-panel layout:
1. Header — "Security Modules" with enabled/total count
2. Module table — Status (●/○), Name, Description with selection highlighting

Uses `SECURITY_MODULE_IDS` constant (L43) to filter `app.modules` to only security-related
ones. Also includes modules whose ID contains "security", "firewall", or "audit".

### 6.3 What Works vs What's Missing

| Feature | Status |
|---------|--------|
| Navigation (j/k) | ✓ Works |
| Toggle enable/disable | ✓ Works (via general `toggle_selected_module()`) |
| Module count in header | ✓ Works |
| Install packages on enable | ✗ Missing — toggle only changes state, no package installation |
| Category grouping | ✗ Missing — `SecurityCategory` enum defined but unused |
| Per-module detail view | ✗ Missing — Enter goes to general ModuleDetail |

### 6.4 Tests (3 tests)

- `test_render_security_modules_no_panic` — Smoke test
- `test_security_category_names` — Category name strings
- `test_security_module_ids` — Constant array contains expected IDs

---

## 7. TUI Recovery View — Deep Dive

**File**: `crates/iron-tui/src/ui/recovery.rs` — 151 lines, 0 tests

### 7.1 Rendering (`render_recovery`, L10)

Four-panel layout:
1. Title bar — "Recovery - Backup, export, and restore your configuration"
2. Status panel — Last Backup (relative time), Bundle, Profile, Module count
3. Actions panel — 5 action descriptions with keybind hints
4. Footer hints — `[g] install.sh [e] Export [s] Snapshot [Esc] Back`

The status panel uses `app.last_backup` for relative time formatting with color coding:
- ≤7 days → green
- ≤30 days → yellow
- >30 days → red
- None → gray

Action descriptions (render_recovery_actions, L113):
```
[g] Generate install.sh   — Bootstraps a fresh machine from your config
[e] Export config bundle   — Creates a portable archive
[i] Import from backup     — Restores configuration from an archive
[r] Recovery wizard        — Step-by-step system restoration guide
[s] Create snapshot now    — Take a timeshift/snapper snapshot immediately
```

### 7.2 The Critical Gap: No Handlers, No Actions

**Identical problem to Secrets view.** There is NO `View::Recovery` match arm in the
handler. All action keybinds ([g], [e], [i], [r], [s]) are dead.

**No action methods exist** for recovery operations. No `export_state()`,
`import_backup()`, `generate_install_script()`, `create_snapshot()`, or
`recovery_wizard()` methods in `actions.rs`.

**State never populated**:
- `app.last_backup` — initialized to `None`, never set → always shows "Never"

**Bottom line**: The Recovery view is also a **pure visual mockup**.

---

## 8. CLI Secrets Command — Deep Dive

**File**: `crates/iron-cli/src/commands/secrets.rs` — 290 lines, 0 tests

### 8.1 Subcommands

```rust
pub enum SecretsAction {
    Status,              // Show git-crypt status
    Unlock { key: Option<String> },  // Decrypt secrets
    Lock,                // Re-encrypt secrets
    Link,                // Symlink secrets to target locations
}
```

### 8.2 Command Analysis

**`status()` (L45)** — Comprehensive:
- Shows status badge (Unlocked/Locked/Inactive/Error)
- Lists encrypted files with per-file Locked/Unlocked badge
- Lists authorized GPG keys
- JSON output support via local `SecretsInfo` struct

**`unlock()` (L145)** — Smart flow:
- Pre-checks: already unlocked → skip, not initialized → error, not available → error
- Supports GPG key (default) or explicit key file
- Post-unlock: shows count of accessible files

**`lock()` (L210)** — Similar pre-check pattern:
- Already locked → skip, not initialized/available → warning

**`link()` (L240)** — Unique to CLI (not in TUI):
- Checks secrets are unlocked first
- Walks `secrets/` directory
- Creates symlinks: `secrets/ssh/id_ed25519` → `~/.ssh/id_ed25519`
- Uses `iron_core::validation::expand_home()` for `~` expansion
- Skips existing non-symlink files, replaces existing symlinks
- Reports count of linked files

**Missing**: No `add-gpg-user` or `export-key` CLI subcommands (service methods exist
but no CLI wiring).

### 8.3 The `link()` Convention

The `link()` command uses a convention where files under `secrets/` map to `~/.<path>`:
```
secrets/ssh/id_ed25519     → ~/.ssh/id_ed25519
secrets/gpg/private-key.asc → ~/.gpg/private-key.asc
secrets/tokens/github-token → ~/.tokens/github-token
```

This prefix convention (`~/.` + relative path) is undocumented and potentially surprising.
A file at `secrets/foo.txt` would link to `~/.foo.txt`.

---

## 9. CLI Recover Command — Deep Dive

**File**: `crates/iron-cli/src/commands/recover.rs` — 250 lines, 0 tests

### 9.1 Subcommands (via flags, not subcommands)

```rust
Recover {
    export: bool,           // --export
    import: Option<String>, // --import <file>
    script: bool,           // --script
}
```

### 9.2 Command Analysis

**`export_state()` (L55)** — Straightforward:
- Calls `recovery_service.export()` (queries pacman/systemctl)
- Saves to `iron-export-<timestamp>.json`
- Verbose mode shows summary (host, bundle, packages count)
- JSON output support

**`import_state()` (L103)** — With confirmation:
- Shows import preview (host, bundle, profile, modules, packages)
- Interactive y/N confirmation
- Calls `recovery_service.import()` (state-only, no package install)
- **Notable**: Only restores Iron state. Does NOT install packages or services.

**`generate_script()` (L185)** — Full featured:
- Uses `InstallScriptOptions` with all flags enabled
- Defaults to `paru` for AUR helper, interactive mode enabled
- Writes to `iron-install.sh` with executable permissions
- Verbose mode shows first 20 lines preview

**`show_help()` (L34)** — Default when no flags provided:
- Shows usage examples

### 9.3 Missing CLI Recovery Features

- No `--backup` / `--restore` flags (create_backup/restore_backup methods exist but
  are not wired to CLI)
- No `--verify` flag (RecoveryService has no verify method despite spec mentioning
  `RecoveryService::verify_installation()`)
- No package installation during import (import is state-only)

---

## 10. Task S1-P9-001 — Secrets Encrypt/Decrypt File Actions

### 10.1 Original Task Definition

From `docs/TODO-scenario1.md` L253:

```
S1-P9-001 | P2 | Secrets — encrypt/decrypt file actions
  Why: Secrets view renders status and file list but action keys may not
    trigger actual git-crypt operations.
  Action: Verify 'e' (encrypt) and 'd' (decrypt) keys actually invoke
    git-crypt lock/unlock. Wire if missing.
  Files: crates/iron-tui/src/app/handlers.rs, crates/iron-core/src/services/secrets.rs
  Test: Integration test with git-crypt.
```

### 10.2 Current State

The task correctly identified the problem but understated it. The issue is not just
that `e` and `d` keys "may not trigger" — it's that **NO keys in the Secrets view trigger
ANY action at all**. There is no `View::Secrets` handler block.

Additionally, the task mentions `e` (encrypt) and `d` (decrypt), but the actual UI
shows different keys:
- `[i]` Init (in both footer and help overlay)
- `[u]` Unlock (decrypt) — not `d`
- `[l]` Lock (encrypt) — not `e`
- `[a]` Add GPG key (help overlay only)

### 10.3 Implementation Approach

**Phase A — Wire the handler** (minimum fix for S1-P9-001):

Add `View::Secrets` to the handler match in `handlers.rs`:
```rust
View::Secrets => match key.code {
    KeyCode::Char('i') => { self.secrets_init(); true }
    KeyCode::Char('u') => { self.secrets_unlock(); true }
    KeyCode::Char('l') => { self.secrets_lock(); true }
    KeyCode::Char('a') => { self.secrets_add_gpg_key(); true }
    KeyCode::Char('r') => { self.refresh_secrets(); true }
    _ => false,
},
```

**Phase B — Wire the actions** (add to `actions.rs`):

```rust
pub fn refresh_secrets(&mut self) {
    let service = DefaultSecretsService::new(&self.config_dir);
    match service.status() {
        Ok(status) => {
            self.secrets_status = Some(format!("{:?}", status));
            self.encrypted_files = service.list_encrypted().unwrap_or_default();
            self.set_status("Secrets status refreshed");
        }
        Err(e) => self.set_error(format!("Failed: {}", e)),
    }
}

pub fn secrets_init(&mut self) { ... }
pub fn secrets_unlock(&mut self) { ... }
pub fn secrets_lock(&mut self) { ... }
pub fn secrets_add_gpg_key(&mut self) { ... }  // needs text input for key ID
```

**Phase C — Auto-refresh on navigation**:

Add to `navigate()` in `mod.rs`:
```rust
if matches!(view, View::Secrets) {
    self.refresh_secrets();
}
```

**Phase D — Add confirmation for destructive operations**:

`lock()` is destructive (re-encrypts files). Should use `request_confirm()`.
`init()` is one-time setup. Could use Simple confirmation.

### 10.4 Dependencies

- `iron-core::services::secrets::DefaultSecretsService` is already usable from TUI
  (iron-core is a dependency)
- No new crate dependencies needed
- The `add_gpg_key()` action requires text input — the TUI currently has no general
  text input widget outside of `ProfileBuilder` and `ModuleCreator`. Would need a
  simple input dialog or reuse of the `confirm_typed_input` pattern from
  `ConfirmStyle::TypedConfirmation`.

---

## 11. Discovered Issues — Outside Original Phase 9 Scope

### D-P9-001 (P0) — TUI Secrets View Has Zero Action Wiring

**Location**: `handlers.rs` — no `View::Secrets` match arm  
**Problem**: The Secrets view shows keybind hints ([i], [u], [l], [a]) but none of
them do anything. `View::Secrets` is not matched in the handler, so all keys fall
through to the `_ => false` default.  
**Impact**: Users see a functional-looking UI but cannot interact with git-crypt at all.  
**Fix**: Add `View::Secrets` handler block + action methods. This IS the core of
S1-P9-001 but the severity should be elevated to P0 since the view is effectively broken.

### D-P9-002 (P0) — TUI Recovery View Has Zero Action Wiring

**Location**: `handlers.rs` — no `View::Recovery` match arm  
**Problem**: Same as Secrets. The Recovery view shows keybind hints ([g], [e], [i],
[r], [s]) but none do anything.  
**Impact**: Users cannot export state, generate install scripts, or create backups
from the TUI.  
**Fix**: Add `View::Recovery` handler block + action methods.

### D-P9-003 (P1) — Secrets State Never Populated

**Location**: `app/mod.rs` L325-326  
**Problem**: `secrets_status` starts as `None`, `encrypted_files` starts empty. No
code ever sets these values.  
**Impact**: Even if the user HAS git-crypt configured and secrets encrypted, the
Secrets view always shows "Unknown" status and "No encrypted files".  
**Fix**: Auto-refresh on Secrets navigation, or populate during `App::default()` if
git-crypt is available.

### D-P9-004 (P1) — Recovery State Never Populated

**Location**: `app/mod.rs` L327  
**Problem**: `last_backup` starts as `None` and is never set.  
**Impact**: Recovery view always shows "Never" for last backup even if the user has
exported state before.  
**Fix**: Read from StateManager's operation log (check for last "create_backup" or
"export_recovery" operation timestamp).

### D-P9-005 (P1) — list_encrypted() Ignores .gitattributes Patterns

**Location**: `services/secrets.rs` L277  
**Problem**: The method parses `.gitattributes` to find git-crypt patterns but then
ignores the result. It simply walks `secrets/` directory.  
**Impact**: Files outside `secrets/` that are encrypted by git-crypt are not listed.
Files inside `secrets/` that are NOT encrypted are falsely listed.  
**Fix**: Either use the parsed patterns to filter, or use `git-crypt status -e` (like
`iron-git::DefaultSecretsManager` does) for accurate detection.

### D-P9-006 (P2) — Two Independent Secrets Layers (Code Duplication)

**Location**: `services/secrets.rs` vs `iron-git/lib.rs`  
**Problem**: `SecretsService` (iron-core, 10 methods) and `SecretsManager` (iron-git,
4 methods) both wrap git-crypt but with different detection approaches:
- `is_unlocked()`: directory check vs file header check
- `list_encrypted()`: filesystem walk vs `git-crypt status -e`  
**Impact**: Can disagree on status. Code duplication increases maintenance burden.
`SecretsManager` has circuit breaker resilience but is unused.  
**Fix**: Route `SecretsService` through `SecretsManager` for the overlapping methods,
or consolidate into one layer.

### D-P9-007 (P2) — No Audit Logging for Secrets Operations

**Location**: `services/secrets.rs` — no StateManager  
**Problem**: `SecretsService` has no `StateManager` dependency. `unlock()`, `lock()`,
`init()`, and `add_gpg_user()` leave no trace in the operation audit log.  
**Other services**: SyncService records git_push/git_pull/git_commit. UpdateService
records updates. RecoveryService records import_recovery/create_backup/restore_backup.  
**Fix**: Add `StateManager` parameter to `DefaultSecretsService::new()` and record
operations.

### D-P9-008 (P2) — CLI Missing add-gpg-user and export-key Subcommands

**Location**: `cli.rs` L383 / `commands/secrets.rs`  
**Problem**: `SecretsService` has `add_gpg_user()` and `export_key()` methods, but
there are no CLI subcommands for them. The `list_keys()` output is shown in `status`
but keys can only be added via raw `git-crypt add-gpg-user`.  
**Fix**: Add `iron secrets add-key <gpg-id>` and `iron secrets export-key <path>`
subcommands.

### D-P9-009 (P2) — CLI Missing backup/restore Subcommands

**Location**: `commands/recover.rs`  
**Problem**: `RecoveryService` has `create_backup()` and `restore_backup()` methods
that produce `.tar.gz` archives, but no CLI flags wire to them. Only
`--export`/`--import` (JSON state) and `--script` are available.  
**Fix**: Add `--backup <dir>` and `--restore <file.tar.gz>` flags to `iron recover`.

### D-P9-010 (P2) — import() Only Restores State, Not System

**Location**: `services/recovery.rs` L176  
**Problem**: `import()` calls `StateManager.set_current_host()`, `set_active_bundle()`,
etc. but does NOT install packages, enable services, or deploy dotfiles. The spec's
4-step recovery flow (Install → Bundle → Profile → Verify) is not implemented.  
**Impact**: After import, the system state says packages are installed but they aren't.  
**Fix**: Either implement the full recovery flow or clearly document that import is
state-only and requires follow-up with `iron-install.sh`.

### D-P9-011 (P2) — CLI secrets link Convention Undocumented

**Location**: `commands/secrets.rs` L240  
**Problem**: `link()` maps `secrets/<path>` → `~/.<path>`. This `~/.` prefix convention
is not documented anywhere. A file at `secrets/ssh/config` becomes `~/.ssh/config`
(correct), but `secrets/myfile` becomes `~/.myfile` (surprising).  
**Fix**: Document the convention, or use a TOML mapping file to specify link targets.

### D-P9-012 (P3) — snapshot_manager Always NoopManager

**Location**: `context.rs` L95, `services/recovery.rs` L83  
**Problem**: `DefaultRecoveryService` takes a generic `SnapshotManager` but is always
instantiated with `NoopManager`. The `create_backup()` method calls
`self.snapshot_manager.create("pre-backup")` which does nothing.  
**Spec says**: Recovery view shows `[s] Create snapshot now` and spec mentions
"timeshift/snapper snapshot immediately".  
**Fix**: Detect and use timeshift or snapper if available. The TODO comment in
`context.rs` L91 already notes: "TODO: Detect and use timeshift/snapper".

### D-P9-013 (P3) — verify_installation() Missing From RecoveryService

**Location**: user-workflow spec vs `services/recovery.rs`  
**Problem**: The spec (Workflow C, Step 4) says
"RecoveryService::verify_installation() checks drivers, services, permissions, symlinks"
but no such method exists on the `RecoveryService` trait.  
**Fix**: Add `verify_installation()` method that checks package presence, service status,
symlink validity, and driver loading.

---

## 12. Integration Map

### Cross-Crate Dependencies

```
 iron-cli ──────────────────── iron-core ──────────── iron-git
    │                              │                      │
    ├─ commands/secrets.rs         ├─ services/secrets.rs │
    │   uses SecretsService        │   raw git-crypt Cmd  ├─ SecretsManager (L91)
    │   uses DefaultSecretsService │   no StateManager    │   CommandExecutor
    │   4 subcommands              │   10-method trait    │   circuit breaker
    │                              │                      │   4-method trait
    ├─ commands/recover.rs         ├─ services/recovery.rs│
    │   uses RecoveryService       │   raw pacman/systemctl│  UNUSED by any
    │   uses DefaultRecoveryService│   + StateManager     │  secrets consumer
    │   3 flags                    │   8-method trait     │
    │                              │                      │
 iron-tui                          │                      │
    │                              │                      │
    ├─ ui/secrets.rs               │                      │
    │   RENDER ONLY                │                      │
    │   reads: secrets_status,     │                      │
    │          encrypted_files     │                      │
    │   NO ACTIONS, NO HANDLERS    │                      │
    │                              │                      │
    ├─ ui/recovery.rs              │                      │
    │   RENDER ONLY                │                      │
    │   reads: last_backup         │                      │
    │   NO ACTIONS, NO HANDLERS    │                      │
    │                              │                      │
    ├─ ui/security.rs              │                      │
    │   PARTIALLY FUNCTIONAL       │                      │
    │   Has handlers (toggle)      │                      │
    │   Filters modules by ID      │                      │
    │                              │                      │
    └─ NO dependency on iron-git ──┘                      │
                                                          │
                 Unused by any consumer ◄─────────────────┘
```

### CLI ↔ TUI Parity Matrix (Secrets)

| Feature | CLI | TUI | Gap |
|---------|-----|-----|-----|
| Status display | ✓ Detailed + JSON | ✓ Rendered (but always "Unknown") | **P0** — TUI never populates |
| Init git-crypt | ✗ Missing subcommand | ✗ Keybind shown but dead | Both missing |
| Unlock secrets | ✓ With key file support | ✗ Keybind shown but dead | **P0** — TUI not wired |
| Lock secrets | ✓ Working | ✗ Keybind shown but dead | **P0** — TUI not wired |
| Link secrets | ✓ Full linking with report | ✗ Not in TUI | CLI only |
| Add GPG user | ✗ Missing subcommand | ✗ Keybind shown but dead | Both missing |
| List keys | ✓ In status output | ✗ Not rendered | TUI incomplete |
| Export key | ✗ Missing subcommand | ✗ Not shown | Both missing |
| Per-file encrypt status | ✓ Locked/Unlocked badge | ✗ Always shows "[enc]" | TUI simplified |

### CLI ↔ TUI Parity Matrix (Recovery)

| Feature | CLI | TUI | Gap |
|---------|-----|-----|-----|
| Export state | ✓ JSON file with timestamp | ✗ Keybind shown but dead | **P0** — TUI not wired |
| Import state | ✓ With preview + confirm | ✗ Keybind shown but dead | **P0** — TUI not wired |
| Generate script | ✓ install.sh with permissions | ✗ Keybind shown but dead | **P0** — TUI not wired |
| Create backup | ✗ Missing flag | ✗ Not in footer hints | Both missing (service exists) |
| Restore backup | ✗ Missing flag | ✗ Not in footer hints | Both missing (service exists) |
| Create snapshot | N/A | ✗ Keybind shown but dead + NoopManager | TUI dead + service noop |
| Recovery wizard | N/A | ✗ Keybind shown but dead | TUI dead |
| Verify installation | ✗ Missing | ✗ Missing | Both missing (spec mentions it) |

---

## 13. Test Coverage Analysis

### Summary Table

| Component | File | LOC | Tests | Coverage Notes |
|-----------|------|-----|-------|---------------|
| SecretsService | `services/secrets.rs` | 908 | 45 | Good: struct, detection, errors. Missing: actual git-crypt ops |
| RecoveryService | `services/recovery.rs` | 969 | 25 | Good: struct, export/import, scripts. Missing: backup/restore integration |
| iron-git SecretsManager | `lib.rs` (partial) | ~70 | 7 | Basic: initialization, unlock detection |
| TUI secrets render | `ui/secrets.rs` | 122 | **0** | **No tests at all** |
| TUI security render | `ui/security.rs` | 234 | 3 | Smoke test + constants |
| TUI recovery render | `ui/recovery.rs` | 151 | **0** | **No tests at all** |
| CLI secrets command | `commands/secrets.rs` | 290 | **0** | **No tests at all** |
| CLI recover command | `commands/recover.rs` | 250 | **0** | **No tests at all** |
| CLI parse (secrets) | `cli.rs` | — | 1 | Secrets status parse |
| CLI parse (recover) | `cli.rs` | — | 2 | Export + import parse |

### Total Phase 9 Test Count

| Layer | Count | Notes |
|-------|-------|-------|
| iron-core SecretsService | 45 | Comprehensive struct/detection tests |
| iron-core RecoveryService | 25 | Good export/import + script generation |
| iron-git SecretsManager | 7 | Basic circuit breaker + detection |
| TUI security render | 3 | Smoke + constants |
| CLI parsing | 3 | Parse-only tests |
| **Total** | **83** | |

### Critical Test Gaps

1. **TUI Secrets view has ZERO tests** — Not even a smoke render test. Compare to
   sync view which has 1, or security which has 3.

2. **TUI Recovery view has ZERO tests** — Same as Secrets.

3. **CLI `commands/secrets.rs` has ZERO tests** — 290 lines of logic (status display,
   unlock flow, lock flow, link flow) completely untested.

4. **CLI `commands/recover.rs` has ZERO tests** — 250 lines of logic (export, import
   with y/N confirmation, script generation) completely untested.

5. **No integration tests for git-crypt** — All SecretsService tests use filesystem
   mocking. No test ever runs `git-crypt init/unlock/lock`. Since S1-P9-001 specifically
   asks for "integration test with git-crypt", this is a known gap.

6. **The two secrets layers are never tested together** — No test verifies that
   `SecretsService` and `SecretsManager` agree on status.

### Recommendations

**Immediate (P0)**:
- Wire Secrets view handlers + actions (D-P9-001, this IS S1-P9-001)
- Wire Recovery view handlers + actions (D-P9-002)
- Elevate severity of S1-P9-001 from P2 to P0

**Short-term (P1)**:
- Populate secrets_status/encrypted_files on navigation (D-P9-003)
- Populate last_backup from operation log (D-P9-004)
- Fix list_encrypted() to use accurate detection (D-P9-005)
- Add smoke render tests for Secrets and Recovery TUI views
- Add tests for CLI secrets and recover commands

**Medium-term (P2)**:
- Consolidate SecretsService and SecretsManager (D-P9-006)
- Add audit logging to secrets operations (D-P9-007)
- Add missing CLI subcommands: add-key, export-key, backup, restore (D-P9-008, D-P9-009)
- Implement full recovery flow in import() (D-P9-010)

**Long-term (P3)**:
- Detect and integrate timeshift/snapper (D-P9-012)
- Implement verify_installation() (D-P9-013)
- Document secrets linking convention (D-P9-011)
