//! Iron Git - Git operations for Iron configuration management
//!
//! This crate provides git integration for Iron, including:
//! - Repository status and diff
//! - Commit, push, and pull operations
//! - git-crypt secrets management
//! - Sync workflow support
//!
//! # Testing Support
//!
//! The `test_fixtures` module provides mock responses for git commands,
//! enabling comprehensive testing without actual git repository operations:
//!
//! ```rust,ignore
//! use iron_git::test_fixtures::GitMockBuilder;
//! use std::sync::Arc;
//!
//! let executor = GitMockBuilder::new()
//!     .with_branch("main")
//!     .with_modified_files(&["src/lib.rs"])
//!     .build();
//!
//! // Use with DefaultGitManager::with_executor()
//! ```

pub mod test_fixtures;

use iron_core::resilience::{CommandExecutor, RealCommandExecutor};
use iron_core::{GitError, IronResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Git repository status
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Whether the repository is clean
    pub is_clean: bool,

    /// Modified files
    pub modified: Vec<PathBuf>,

    /// Untracked files
    pub untracked: Vec<PathBuf>,

    /// Staged files
    pub staged: Vec<PathBuf>,

    /// Current branch
    pub branch: Option<String>,

    /// Commits ahead of remote
    pub ahead: usize,

    /// Commits behind remote
    pub behind: usize,
}

/// Result of a pull operation
#[derive(Debug, Clone)]
pub struct PullResult {
    /// Whether the pull was successful
    pub success: bool,

    /// Files that were updated
    pub updated_files: Vec<PathBuf>,

    /// Whether there were conflicts
    pub has_conflicts: bool,

    /// Conflict files (if any)
    pub conflict_files: Vec<PathBuf>,
}

/// Git manager trait for repository operations
pub trait GitManager {
    /// Get the repository status
    fn status(&self) -> IronResult<GitStatus>;

    /// Get the diff of uncommitted changes
    fn diff(&self) -> IronResult<String>;

    /// Commit changes with a message
    fn commit(&self, message: &str) -> IronResult<()>;

    /// Push to remote
    fn push(&self, remote: &str, branch: &str) -> IronResult<()>;

    /// Pull from remote
    fn pull(&self, remote: &str, branch: &str) -> IronResult<PullResult>;

    /// Check if there are uncommitted changes
    fn has_changes(&self) -> IronResult<bool>;
}

/// Secrets manager trait for git-crypt operations
pub trait SecretsManager {
    /// Check if secrets are unlocked
    fn is_unlocked(&self) -> bool;

    /// Unlock secrets with a key
    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()>;

    /// Lock secrets
    fn lock(&self) -> IronResult<()>;

    /// List encrypted files
    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>>;
}

/// Default git manager implementation using git commands
pub struct DefaultGitManager {
    /// Repository root path
    root: PathBuf,
    /// Optional command executor for resilient command execution
    executor: Option<Arc<dyn CommandExecutor>>,
}

impl DefaultGitManager {
    /// Create a new git manager for the given repository with a default resilient executor.
    ///
    /// The circuit breaker opens after 3 consecutive failures and stays open
    /// for 60 seconds, preventing hangs from git operations in a broken environment.
    pub fn new(root: PathBuf) -> IronResult<Self> {
        Self::with_resilience(root)
    }

    /// Create a git manager with a command executor for resilient operations
    ///
    /// The executor provides circuit breaker patterns and timeout handling
    /// for git commands. When the circuit opens due to repeated failures,
    /// commands will fail fast without attempting execution.
    pub fn with_executor(root: PathBuf, executor: Arc<dyn CommandExecutor>) -> IronResult<Self> {
        if !root.join(".git").exists() {
            return Err(GitError::NotARepository { path: root }.into());
        }
        Ok(Self {
            root,
            executor: Some(executor),
        })
    }

    /// Create a git manager with default resilient executor
    ///
    /// Uses the default `RealCommandExecutor` with 120s timeout and circuit breaker.
    pub fn with_resilience(root: PathBuf) -> IronResult<Self> {
        Self::with_executor(root, Arc::new(RealCommandExecutor::with_defaults()))
    }

    /// Get the repository root
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Run a git command using executor if available, otherwise direct execution
    fn run_git(&self, args: &[&str]) -> IronResult<String> {
        if let Some(ref executor) = self.executor {
            // Build args with -C for working directory
            let mut full_args = vec!["-C", self.root.to_str().unwrap_or(".")];
            full_args.extend(args);

            let output =
                executor
                    .execute_full("git", &full_args)
                    .map_err(|e| GitError::PushFailed {
                        message: format!("Failed to run git: {}", e),
                    })?;

            if output.success() {
                Ok(output.stdout)
            } else {
                Err(GitError::PushFailed {
                    message: output.stderr,
                }
                .into())
            }
        } else {
            let output = Command::new("git")
                .args(args)
                .current_dir(&self.root)
                .output()
                .map_err(iron_core::IronError::Io)?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(GitError::PushFailed {
                    message: stderr.to_string(),
                }
                .into())
            }
        }
    }
}

impl GitManager for DefaultGitManager {
    fn status(&self) -> IronResult<GitStatus> {
        let output = self.run_git(&["status", "--porcelain", "-b"])?;
        Ok(parse_git_status(&output))
    }

    fn diff(&self) -> IronResult<String> {
        self.run_git(&["diff"])
    }

    fn commit(&self, message: &str) -> IronResult<()> {
        self.run_git(&["commit", "-m", message])?;
        Ok(())
    }

    fn push(&self, remote: &str, branch: &str) -> IronResult<()> {
        self.run_git(&["push", remote, branch])?;
        Ok(())
    }

    fn pull(&self, remote: &str, branch: &str) -> IronResult<PullResult> {
        let output = self.run_git(&["pull", remote, branch]);

        match output {
            Ok(_) => Ok(PullResult {
                success: true,
                updated_files: vec![],
                has_conflicts: false,
                conflict_files: vec![],
            }),
            Err(e) => {
                // Check if it's a merge conflict
                if let Ok(status) = self.run_git(&["status", "--porcelain"]) {
                    let conflicts: Vec<String> = status
                        .lines()
                        .filter(|l| l.starts_with("UU") || l.starts_with("AA"))
                        .map(|l| l[3..].trim().to_string())
                        .collect();

                    if !conflicts.is_empty() {
                        return Err(GitError::MergeConflict { files: conflicts }.into());
                    }
                }
                Err(e)
            }
        }
    }

    fn has_changes(&self) -> IronResult<bool> {
        let status = self.status()?;
        Ok(!status.is_clean)
    }
}

/// Default secrets manager using git-crypt
pub struct DefaultSecretsManager {
    root: PathBuf,
    /// Optional command executor for resilient command execution
    executor: Option<Arc<dyn CommandExecutor>>,
}

impl DefaultSecretsManager {
    /// Create a new secrets manager with a default resilient executor.
    pub fn new(root: PathBuf) -> Self {
        Self::with_resilience(root)
    }

    /// Create a secrets manager with a command executor for resilient operations
    pub fn with_executor(root: PathBuf, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            root,
            executor: Some(executor),
        }
    }

    /// Create a secrets manager with default resilient executor
    pub fn with_resilience(root: PathBuf) -> Self {
        Self::with_executor(root, Arc::new(RealCommandExecutor::with_defaults()))
    }

    /// Check if git-crypt is initialized
    pub fn is_initialized(&self) -> bool {
        self.root.join(".git-crypt").exists()
    }

    /// Run git-crypt command using executor if available, otherwise direct execution
    fn run_gitcrypt(&self, args: &[&str]) -> IronResult<String> {
        if let Some(ref executor) = self.executor {
            // For git-crypt, we need to run it from the repo directory
            // Use execute_with_env to set the working directory context
            let output =
                executor
                    .execute_full("git-crypt", args)
                    .map_err(|e| GitError::PushFailed {
                        message: format!("Failed to run git-crypt: {}", e),
                    })?;

            if output.success() {
                Ok(output.stdout)
            } else {
                Err(GitError::PushFailed {
                    message: output.stderr,
                }
                .into())
            }
        } else {
            let output = Command::new("git-crypt")
                .args(args)
                .current_dir(&self.root)
                .output()
                .map_err(iron_core::IronError::Io)?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(GitError::PushFailed {
                    message: stderr.to_string(),
                }
                .into())
            }
        }
    }
}

impl SecretsManager for DefaultSecretsManager {
    fn is_unlocked(&self) -> bool {
        if !self.is_initialized() {
            return true; // No git-crypt, nothing to unlock
        }

        // Check if secrets directory is readable
        let secrets_dir = self.root.join("secrets");
        if !secrets_dir.exists() {
            return true;
        }

        // Try to read a file in secrets to see if it's decrypted
        if let Ok(entries) = std::fs::read_dir(&secrets_dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read(entry.path()) {
                    // Use the helper function to check for encrypted content
                    if is_gitcrypt_encrypted(&content) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(GitError::GitCryptNotInitialized.into());
        }

        let mut args = vec!["unlock"];
        let key_str;
        if let Some(key) = key_path {
            key_str = key.to_str().unwrap_or_default().to_string();
            args.push(&key_str);
        }

        self.run_gitcrypt(&args)?;
        Ok(())
    }

    fn lock(&self) -> IronResult<()> {
        if !self.is_initialized() {
            return Ok(());
        }

        self.run_gitcrypt(&["lock"])?;
        Ok(())
    }

    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>> {
        if !self.is_initialized() {
            return Ok(vec![]);
        }

        let output = self.run_gitcrypt(&["status", "-e"])?;
        Ok(parse_encrypted_files(&output))
    }
}

// =========================================================================
// SecretsBackend impl — allows iron-core's DefaultSecretsService to
// delegate git-crypt operations to this resilient manager.
// =========================================================================

impl iron_core::services::secrets::SecretsBackend for DefaultSecretsManager {
    fn is_unlocked(&self) -> bool {
        SecretsManager::is_unlocked(self)
    }

    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()> {
        SecretsManager::unlock(self, key_path)
    }

    fn lock(&self) -> IronResult<()> {
        SecretsManager::lock(self)
    }

    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>> {
        SecretsManager::list_encrypted(self)
    }
}

/// Parse git status --porcelain -b output into GitStatus
///
/// Parses the output of `git status --porcelain -b` into a structured [`GitStatus`].
///
/// # Examples
///
/// ```
/// use iron_git::{parse_git_status, GitStatus};
///
/// // Parse a clean repository status
/// let output = "## main...origin/main\n";
/// let status = parse_git_status(output);
/// assert!(status.is_clean);
/// assert_eq!(status.branch, Some("main".to_string()));
///
/// // Parse status with modified files
/// let output = "## develop...origin/develop\n M src/lib.rs\n?? new_file.txt\n";
/// let status = parse_git_status(output);
/// assert!(!status.is_clean);
/// assert_eq!(status.modified.len(), 1);
/// assert_eq!(status.untracked.len(), 1);
///
/// // Parse ahead/behind tracking
/// let output = "## main...origin/main [ahead 3, behind 2]\n";
/// let status = parse_git_status(output);
/// assert_eq!(status.ahead, 3);
/// assert_eq!(status.behind, 2);
/// ```
pub fn parse_git_status(output: &str) -> GitStatus {
    let mut status = GitStatus::default();

    for line in output.lines() {
        if line.starts_with("##") {
            // Parse branch info: "## main...origin/main [ahead 1, behind 2]"
            if let Some(branch_part) = line.strip_prefix("## ") {
                // Handle "## No commits yet on main" case
                if branch_part.starts_with("No commits yet on ") {
                    status.branch = branch_part
                        .strip_prefix("No commits yet on ")
                        .map(|s| s.to_string());
                    continue;
                }

                // Parse branch name and ahead/behind
                let parts: Vec<&str> = branch_part.split("...").collect();
                if !parts.is_empty() {
                    status.branch = Some(parts[0].to_string());
                }

                // Parse ahead/behind if present
                if let Some(tracking_info) = branch_part.find('[') {
                    let info = &branch_part[tracking_info..];
                    if let Some(ahead_pos) = info.find("ahead ") {
                        let ahead_str = &info[ahead_pos + 6..];
                        if let Some(end) = ahead_str.find(|c: char| !c.is_ascii_digit()) {
                            status.ahead = ahead_str[..end].parse().unwrap_or(0);
                        } else {
                            status.ahead = ahead_str.trim_end_matches(']').parse().unwrap_or(0);
                        }
                    }
                    if let Some(behind_pos) = info.find("behind ") {
                        let behind_str = &info[behind_pos + 7..];
                        if let Some(end) = behind_str.find(|c: char| !c.is_ascii_digit()) {
                            status.behind = behind_str[..end].parse().unwrap_or(0);
                        } else {
                            status.behind = behind_str.trim_end_matches(']').parse().unwrap_or(0);
                        }
                    }
                }
            }
        } else if line.len() >= 3 {
            let indicator = &line[0..2];
            let file = PathBuf::from(line[3..].trim());

            match indicator {
                "??" => status.untracked.push(file),
                " M" | "MM" => status.modified.push(file),
                "M " | "A " | "AM" => status.staged.push(file),
                " D" => status.modified.push(file),
                "D " => status.staged.push(file),
                "R " => status.staged.push(file),
                "UU" | "AA" | "DD" => {
                    // Conflict markers - treat as modified
                    status.modified.push(file);
                }
                _ => {}
            }
        }
    }

    status.is_clean =
        status.modified.is_empty() && status.untracked.is_empty() && status.staged.is_empty();

    status
}

/// Parse git-crypt status output to extract encrypted files
///
/// Parses the output of `git-crypt status -e` to extract the list of encrypted files.
///
/// # Examples
///
/// ```
/// use iron_git::parse_encrypted_files;
/// use std::path::PathBuf;
///
/// // Parse single encrypted file
/// let output = "encrypted: secrets/api_key.txt\n";
/// let files = parse_encrypted_files(output);
/// assert_eq!(files.len(), 1);
/// assert_eq!(files[0], PathBuf::from("secrets/api_key.txt"));
///
/// // Parse multiple encrypted files
/// let output = "encrypted: secrets/api_key.txt\nencrypted: config/prod.yaml\n";
/// let files = parse_encrypted_files(output);
/// assert_eq!(files.len(), 2);
///
/// // Empty output returns empty vec
/// let files = parse_encrypted_files("");
/// assert!(files.is_empty());
/// ```
pub fn parse_encrypted_files(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            // Format: "encrypted: path/to/file" or "    encrypted: path/to/file"
            line.trim()
                .strip_prefix("encrypted:")
                .map(|path| PathBuf::from(path.trim()))
        })
        .collect()
}

/// Check if content appears to be git-crypt encrypted
///
/// Detects git-crypt encrypted content by checking for the magic header `\x00GITCRYPT`.
///
/// # Examples
///
/// ```
/// use iron_git::is_gitcrypt_encrypted;
///
/// // Encrypted content starts with GITCRYPT header
/// let encrypted = b"\x00GITCRYPT\x00\x10\x00\x00some_binary_data";
/// assert!(is_gitcrypt_encrypted(encrypted));
///
/// // Plain text is not encrypted
/// let plaintext = b"This is plaintext content";
/// assert!(!is_gitcrypt_encrypted(plaintext));
///
/// // Empty content is not encrypted
/// assert!(!is_gitcrypt_encrypted(b""));
/// ```
pub fn is_gitcrypt_encrypted(content: &[u8]) -> bool {
    content.starts_with(b"\x00GITCRYPT")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // GitStatus struct tests
    // ==========================================================================

    #[test]
    fn test_git_status_default() {
        let status = GitStatus::default();
        assert!(!status.is_clean);
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
        assert!(status.branch.is_none());
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn test_git_status_clean() {
        let status = GitStatus {
            is_clean: true,
            modified: vec![],
            untracked: vec![],
            staged: vec![],
            branch: Some("main".to_string()),
            ahead: 0,
            behind: 0,
        };
        assert!(status.is_clean);
        assert_eq!(status.branch, Some("main".to_string()));
    }

    #[test]
    fn test_git_status_with_changes() {
        let status = GitStatus {
            is_clean: false,
            modified: vec![PathBuf::from("src/main.rs")],
            untracked: vec![PathBuf::from("new_file.txt")],
            staged: vec![PathBuf::from("staged.rs")],
            branch: Some("feature".to_string()),
            ahead: 2,
            behind: 1,
        };
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
    }

    // ==========================================================================
    // PullResult struct tests
    // ==========================================================================

    #[test]
    fn test_pull_result_success() {
        let result = PullResult {
            success: true,
            updated_files: vec![PathBuf::from("updated.txt")],
            has_conflicts: false,
            conflict_files: vec![],
        };
        assert!(result.success);
        assert!(!result.has_conflicts);
        assert_eq!(result.updated_files.len(), 1);
    }

    #[test]
    fn test_pull_result_with_conflicts() {
        let result = PullResult {
            success: false,
            updated_files: vec![],
            has_conflicts: true,
            conflict_files: vec![
                PathBuf::from("conflict1.txt"),
                PathBuf::from("conflict2.txt"),
            ],
        };
        assert!(!result.success);
        assert!(result.has_conflicts);
        assert_eq!(result.conflict_files.len(), 2);
    }

    // ==========================================================================
    // Status parsing tests (mock git output)
    // ==========================================================================

    #[test]
    fn test_parse_status_clean_repo() {
        let output = "## main...origin/main\n";
        let status = parse_git_status(output);
        assert!(status.is_clean);
        assert_eq!(status.branch, Some("main".to_string()));
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
    }

    #[test]
    fn test_parse_status_with_modified_files() {
        let output = "## main...origin/main\n M src/lib.rs\n M Cargo.toml\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 2);
        assert!(status.modified.contains(&PathBuf::from("src/lib.rs")));
        assert!(status.modified.contains(&PathBuf::from("Cargo.toml")));
    }

    #[test]
    fn test_parse_status_with_untracked_files() {
        let output = "## main\n?? new_file.txt\n?? another_new.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.untracked.len(), 2);
        assert!(status.untracked.contains(&PathBuf::from("new_file.txt")));
        assert!(status.untracked.contains(&PathBuf::from("another_new.rs")));
    }

    #[test]
    fn test_parse_status_with_staged_files() {
        let output = "## feature\nM  staged_file.rs\nA  new_staged.txt\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.staged.len(), 2);
        assert!(status.staged.contains(&PathBuf::from("staged_file.rs")));
        assert!(status.staged.contains(&PathBuf::from("new_staged.txt")));
    }

    #[test]
    fn test_parse_status_mixed_changes() {
        let output = r#"## develop...origin/develop
 M modified.rs
M  staged.rs
?? untracked.txt
AM added_modified.rs
"#;
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.branch, Some("develop".to_string()));
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.staged.len(), 2); // M  and AM
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn test_parse_status_ahead_behind() {
        let output = "## main...origin/main [ahead 3, behind 2]\n";
        let status = parse_git_status(output);
        assert!(status.is_clean);
        assert_eq!(status.branch, Some("main".to_string()));
        assert_eq!(status.ahead, 3);
        assert_eq!(status.behind, 2);
    }

    #[test]
    fn test_parse_status_only_ahead() {
        let output = "## feature...origin/feature [ahead 5]\n";
        let status = parse_git_status(output);
        assert_eq!(status.ahead, 5);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn test_parse_status_only_behind() {
        let output = "## main...origin/main [behind 7]\n";
        let status = parse_git_status(output);
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 7);
    }

    #[test]
    fn test_parse_status_no_tracking_branch() {
        let output = "## my-local-branch\n M file.rs\n";
        let status = parse_git_status(output);
        assert_eq!(status.branch, Some("my-local-branch".to_string()));
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn test_parse_status_deleted_files() {
        let output = "## main\n D deleted_unstaged.rs\nD  deleted_staged.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 1); // unstaged delete
        assert_eq!(status.staged.len(), 1); // staged delete
    }

    #[test]
    fn test_parse_status_renamed_file() {
        let output = "## main\nR  old_name.rs -> new_name.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.staged.len(), 1);
    }

    #[test]
    fn test_parse_status_conflict_markers() {
        let output = "## main\nUU conflicted.rs\nAA both_added.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 2); // Conflicts treated as modified
    }

    #[test]
    fn test_parse_status_new_repo() {
        let output = "## No commits yet on main\n?? README.md\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.branch, Some("main".to_string()));
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn test_parse_status_empty_output() {
        let output = "";
        let status = parse_git_status(output);
        assert!(status.is_clean);
        assert!(status.branch.is_none());
    }

    // ==========================================================================
    // Encrypted files parsing tests
    // ==========================================================================

    #[test]
    fn test_parse_encrypted_files_empty() {
        let output = "";
        let files = parse_encrypted_files(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_encrypted_files_single() {
        let output = "encrypted: secrets/api_key.txt\n";
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("secrets/api_key.txt"));
    }

    #[test]
    fn test_parse_encrypted_files_multiple() {
        let output = r#"encrypted: secrets/api_key.txt
encrypted: secrets/database.env
encrypted: config/prod.yaml
"#;
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 3);
        assert!(files.contains(&PathBuf::from("secrets/api_key.txt")));
        assert!(files.contains(&PathBuf::from("secrets/database.env")));
        assert!(files.contains(&PathBuf::from("config/prod.yaml")));
    }

    #[test]
    fn test_parse_encrypted_files_with_whitespace() {
        let output = "    encrypted: secrets/key.txt\n";
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("secrets/key.txt"));
    }

    #[test]
    fn test_parse_encrypted_files_mixed_output() {
        let output = r#"not encrypted: README.md
encrypted: secrets/token.txt
    not encrypted: src/main.rs
    encrypted: config/secrets.yaml
"#;
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 2);
    }

    // ==========================================================================
    // Git-crypt encryption detection tests
    // ==========================================================================

    #[test]
    fn test_is_gitcrypt_encrypted_true() {
        let content = b"\x00GITCRYPT\x00\x10\x00\x00";
        assert!(is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_false_plaintext() {
        let content = b"This is plaintext content";
        assert!(!is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_false_empty() {
        let content = b"";
        assert!(!is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_false_partial_header() {
        let content = b"\x00GIT";
        assert!(!is_gitcrypt_encrypted(content));
    }

    // ==========================================================================
    // Mock GitManager for testing dependent code
    // ==========================================================================

    /// Mock GitManager for testing
    pub struct MockGitManager {
        pub status_result: Option<GitStatus>,
        pub diff_result: Option<String>,
        pub has_changes_result: bool,
    }

    impl Default for MockGitManager {
        fn default() -> Self {
            Self {
                status_result: Some(GitStatus {
                    is_clean: true,
                    ..Default::default()
                }),
                diff_result: Some(String::new()),
                has_changes_result: false,
            }
        }
    }

    impl GitManager for MockGitManager {
        fn status(&self) -> IronResult<GitStatus> {
            self.status_result.clone().ok_or_else(|| {
                GitError::NotARepository {
                    path: PathBuf::from("/mock"),
                }
                .into()
            })
        }

        fn diff(&self) -> IronResult<String> {
            self.diff_result.clone().ok_or_else(|| {
                GitError::NotARepository {
                    path: PathBuf::from("/mock"),
                }
                .into()
            })
        }

        fn commit(&self, _message: &str) -> IronResult<()> {
            Ok(())
        }

        fn push(&self, _remote: &str, _branch: &str) -> IronResult<()> {
            Ok(())
        }

        fn pull(&self, _remote: &str, _branch: &str) -> IronResult<PullResult> {
            Ok(PullResult {
                success: true,
                updated_files: vec![],
                has_conflicts: false,
                conflict_files: vec![],
            })
        }

        fn has_changes(&self) -> IronResult<bool> {
            Ok(self.has_changes_result)
        }
    }

    #[test]
    fn test_mock_git_manager_clean() {
        let mock = MockGitManager::default();
        let status = mock.status().unwrap();
        assert!(status.is_clean);
        assert!(!mock.has_changes().unwrap());
    }

    #[test]
    fn test_mock_git_manager_with_changes() {
        let mock = MockGitManager {
            status_result: Some(GitStatus {
                is_clean: false,
                modified: vec![PathBuf::from("changed.rs")],
                ..Default::default()
            }),
            has_changes_result: true,
            ..Default::default()
        };
        let status = mock.status().unwrap();
        assert!(!status.is_clean);
        assert!(mock.has_changes().unwrap());
    }

    #[test]
    fn test_mock_git_manager_diff() {
        let mock = MockGitManager {
            diff_result: Some("+ added line\n- removed line".to_string()),
            ..Default::default()
        };
        let diff = mock.diff().unwrap();
        assert!(diff.contains("+ added line"));
    }

    #[test]
    fn test_mock_git_manager_commit() {
        let mock = MockGitManager::default();
        assert!(mock.commit("test commit").is_ok());
    }

    #[test]
    fn test_mock_git_manager_push() {
        let mock = MockGitManager::default();
        assert!(mock.push("origin", "main").is_ok());
    }

    #[test]
    fn test_mock_git_manager_pull() {
        let mock = MockGitManager::default();
        let result = mock.pull("origin", "main").unwrap();
        assert!(result.success);
        assert!(!result.has_conflicts);
    }

    // ==========================================================================
    // Circuit Breaker Integration Tests
    // ==========================================================================

    #[test]
    fn test_git_manager_with_resilience() {
        use tempfile::TempDir;

        // Create a temporary git repository for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();

        // Initialize git repo
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        // Test that with_resilience creates a properly configured manager
        let manager = DefaultGitManager::with_resilience(root).expect("Failed to create manager");
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_git_manager_with_executor() {
        use iron_core::resilience::RealCommandExecutor;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_git_manager_new_has_resilient_executor() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        // new() always initializes with a circuit-breaker executor
        let manager = DefaultGitManager::new(root).expect("Failed to create manager");
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_secrets_manager_with_resilience() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();

        let manager = DefaultSecretsManager::with_resilience(root);
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_secrets_manager_with_executor() {
        use iron_core::resilience::RealCommandExecutor;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();

        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let manager = DefaultSecretsManager::with_executor(root, executor);
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_secrets_manager_new_has_resilient_executor() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();

        // new() always initializes with a circuit-breaker executor
        let manager = DefaultSecretsManager::new(root);
        assert!(manager.executor.is_some());
    }

    // ==========================================================================
    // DefaultGitManager with Mock Executor Integration Tests
    // ==========================================================================

    #[test]
    fn test_git_manager_status_with_executor() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .with_branch("develop")
                .with_modified_files(&["src/main.rs"])
                .with_ahead_behind(2, 1)
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        let status = manager.status().expect("status should succeed");
        assert_eq!(status.branch, Some("develop".to_string()));
        assert!(!status.is_clean);
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
    }

    #[test]
    fn test_git_manager_diff_with_executor() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let diff_content = "+added line\n-removed line";
        let executor = Arc::new(
            GitMockBuilder::new()
                .with_diff(diff_content)
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        let diff = manager.diff().expect("diff should succeed");
        assert_eq!(diff, diff_content);
    }

    #[test]
    fn test_git_manager_has_changes_clean() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        assert!(!manager.has_changes().expect("has_changes should succeed"));
    }

    #[test]
    fn test_git_manager_has_changes_dirty() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .with_modified_files(&["file.rs"])
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        assert!(manager.has_changes().expect("has_changes should succeed"));
    }

    #[test]
    fn test_git_manager_push_success_with_executor() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .push_succeeds(true)
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        assert!(manager.push("origin", "main").is_ok());
    }

    #[test]
    fn test_git_manager_push_failure_with_executor() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .push_succeeds(false)
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        let result = manager.push("origin", "main");
        assert!(result.is_err());
    }

    #[test]
    fn test_git_manager_pull_success_with_executor() {
        use crate::test_fixtures::GitMockBuilder;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let executor = Arc::new(
            GitMockBuilder::new()
                .pull_succeeds(true)
                .with_repo_root(root.to_str().unwrap())
                .build(),
        );

        let manager =
            DefaultGitManager::with_executor(root, executor).expect("Failed to create manager");

        let result = manager.pull("origin", "main").expect("pull should succeed");
        assert!(result.success);
        assert!(!result.has_conflicts);
    }

    #[test]
    fn test_git_manager_root_accessor() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).expect("Failed to create .git");

        let manager = DefaultGitManager::new(root.clone()).expect("Failed to create manager");

        assert_eq!(manager.root(), root.as_path());
    }

    #[test]
    fn test_git_manager_not_a_repository() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git directory created

        let result = DefaultGitManager::new(root);
        assert!(result.is_err());
    }

    #[test]
    fn test_git_manager_with_executor_not_a_repository() {
        use iron_core::resilience::RealCommandExecutor;
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git directory

        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let result = DefaultGitManager::with_executor(root, executor);
        assert!(result.is_err());
    }

    // ==========================================================================
    // DefaultSecretsManager Tests
    // ==========================================================================

    #[test]
    fn test_secrets_manager_not_initialized() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git-crypt directory

        let manager = DefaultSecretsManager::new(root);
        assert!(!manager.is_initialized());
    }

    #[test]
    fn test_secrets_manager_initialized() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git-crypt")).expect("Failed to create .git-crypt");

        let manager = DefaultSecretsManager::new(root);
        assert!(manager.is_initialized());
    }

    #[test]
    fn test_secrets_manager_is_unlocked_no_gitcrypt() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git-crypt = not initialized, so is_unlocked returns true

        let manager = DefaultSecretsManager::new(root);
        assert!(manager.is_unlocked());
    }

    #[test]
    fn test_secrets_manager_is_unlocked_no_secrets_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git-crypt")).expect("Failed to create .git-crypt");
        // No secrets directory

        let manager = DefaultSecretsManager::new(root);
        assert!(manager.is_unlocked());
    }

    #[test]
    fn test_secrets_manager_is_unlocked_plaintext_secrets() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git-crypt")).expect("Failed to create .git-crypt");
        std::fs::create_dir_all(root.join("secrets")).expect("Failed to create secrets dir");
        std::fs::write(root.join("secrets/api_key.txt"), "plaintext secret")
            .expect("Failed to write secret");

        let manager = DefaultSecretsManager::new(root);
        assert!(manager.is_unlocked());
    }

    #[test]
    fn test_secrets_manager_is_unlocked_encrypted_secrets() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git-crypt")).expect("Failed to create .git-crypt");
        std::fs::create_dir_all(root.join("secrets")).expect("Failed to create secrets dir");
        // Write git-crypt encrypted content
        std::fs::write(
            root.join("secrets/api_key.txt"),
            b"\x00GITCRYPT\x00\x10\x00\x00encrypted_data",
        )
        .expect("Failed to write secret");

        let manager = DefaultSecretsManager::new(root);
        assert!(!manager.is_unlocked());
    }

    #[test]
    fn test_secrets_manager_unlock_not_initialized() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git-crypt directory

        let manager = DefaultSecretsManager::new(root);
        let result = manager.unlock(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_manager_lock_not_initialized() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git-crypt = not initialized, lock returns Ok

        let manager = DefaultSecretsManager::new(root);
        assert!(manager.lock().is_ok());
    }

    #[test]
    fn test_secrets_manager_list_encrypted_not_initialized() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();
        // No .git-crypt

        let manager = DefaultSecretsManager::new(root);
        let files = manager
            .list_encrypted()
            .expect("list_encrypted should succeed");
        assert!(files.is_empty());
    }

    // ==========================================================================
    // Additional Git Status Parsing Edge Cases
    // ==========================================================================

    #[test]
    fn test_parse_status_modified_and_staged() {
        let output = "## main\nMM both_modified.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 1);
        assert!(status.modified.contains(&PathBuf::from("both_modified.rs")));
    }

    #[test]
    fn test_parse_status_added_modified() {
        let output = "## main\nAM new_file.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.staged.len(), 1);
        assert!(status.staged.contains(&PathBuf::from("new_file.rs")));
    }

    #[test]
    fn test_parse_status_deleted_deleted_conflict() {
        let output = "## main\nDD deleted_both.rs\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert_eq!(status.modified.len(), 1);
    }

    #[test]
    fn test_parse_status_short_line() {
        // Line shorter than 3 chars should be ignored
        let output = "## main\nX\n";
        let status = parse_git_status(output);
        assert!(status.is_clean);
        assert_eq!(status.branch, Some("main".to_string()));
    }

    #[test]
    fn test_parse_status_unrecognized_indicator() {
        let output = "## main\nXX unknown.rs\n";
        let status = parse_git_status(output);
        // Unrecognized indicator should be ignored
        assert!(status.is_clean);
    }

    #[test]
    fn test_parse_status_files_with_spaces() {
        let output = "## main\n M file with spaces.rs\n?? another file.txt\n";
        let status = parse_git_status(output);
        assert!(!status.is_clean);
        assert!(
            status
                .modified
                .contains(&PathBuf::from("file with spaces.rs"))
        );
        assert!(
            status
                .untracked
                .contains(&PathBuf::from("another file.txt"))
        );
    }

    #[test]
    fn test_parse_status_large_ahead_behind() {
        let output = "## main...origin/main [ahead 100, behind 50]\n";
        let status = parse_git_status(output);
        assert_eq!(status.ahead, 100);
        assert_eq!(status.behind, 50);
    }

    // ==========================================================================
    // Additional Encrypted Files Parsing Edge Cases
    // ==========================================================================

    #[test]
    fn test_parse_encrypted_files_with_nested_paths() {
        let output =
            "encrypted: secrets/prod/api_key.txt\nencrypted: secrets/dev/database/password.env\n";
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("secrets/prod/api_key.txt")));
        assert!(files.contains(&PathBuf::from("secrets/dev/database/password.env")));
    }

    #[test]
    fn test_parse_encrypted_files_with_special_chars() {
        let output =
            "encrypted: secrets/key-with-dash.txt\nencrypted: secrets/key_with_underscore.env\n";
        let files = parse_encrypted_files(output);
        assert_eq!(files.len(), 2);
    }

    // ==========================================================================
    // Git-crypt Encryption Detection Edge Cases
    // ==========================================================================

    #[test]
    fn test_is_gitcrypt_encrypted_exact_header() {
        let content = b"\x00GITCRYPT";
        assert!(is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_longer_content() {
        let content = b"\x00GITCRYPT\x00\x10\x00\x00this_is_some_encrypted_binary_data_that_goes_on_for_a_while";
        assert!(is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_similar_but_different() {
        // Similar prefix but not exactly GITCRYPT
        let content = b"\x00GITCRYP";
        assert!(!is_gitcrypt_encrypted(content));
    }

    #[test]
    fn test_is_gitcrypt_encrypted_null_in_plaintext() {
        // Content that happens to start with null but isn't gitcrypt
        let content = b"\x00NOTGITCRYPT";
        assert!(!is_gitcrypt_encrypted(content));
    }
}
