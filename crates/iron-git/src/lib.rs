//! Iron Git - Git operations for Iron configuration management
//!
//! This crate provides git integration for Iron, including:
//! - Repository status and diff
//! - Commit, push, and pull operations
//! - git-crypt secrets management
//! - Sync workflow support

use iron_core::{GitError, IronResult};
use std::path::{Path, PathBuf};
use std::process::Command;

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
}

impl DefaultGitManager {
    /// Create a new git manager for the given repository
    pub fn new(root: PathBuf) -> IronResult<Self> {
        // Verify this is a git repository
        if !root.join(".git").exists() {
            return Err(GitError::NotARepository { path: root }.into());
        }
        Ok(Self { root })
    }

    /// Get the repository root
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Run a git command and return output
    fn run_git(&self, args: &[&str]) -> IronResult<String> {
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
}

impl DefaultSecretsManager {
    /// Create a new secrets manager
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Check if git-crypt is initialized
    pub fn is_initialized(&self) -> bool {
        self.root.join(".git-crypt").exists()
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

        let mut args = vec!["git-crypt", "unlock"];
        if let Some(key) = key_path {
            args.push(key.to_str().unwrap_or_default());
        }

        let output = Command::new(args[0])
            .args(&args[1..])
            .current_dir(&self.root)
            .output()
            .map_err(iron_core::IronError::Io)?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GitError::PushFailed {
                message: format!("Failed to unlock secrets: {}", stderr),
            }
            .into())
        }
    }

    fn lock(&self) -> IronResult<()> {
        if !self.is_initialized() {
            return Ok(());
        }

        let output = Command::new("git-crypt")
            .args(["lock"])
            .current_dir(&self.root)
            .output()
            .map_err(iron_core::IronError::Io)?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GitError::PushFailed {
                message: format!("Failed to lock secrets: {}", stderr),
            }
            .into())
        }
    }

    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>> {
        if !self.is_initialized() {
            return Ok(vec![]);
        }

        let output = Command::new("git-crypt")
            .args(["status", "-e"])
            .current_dir(&self.root)
            .output()
            .map_err(iron_core::IronError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_encrypted_files(&stdout))
    }
}

/// Parse git status --porcelain -b output into GitStatus
///
/// This is exposed for testing purposes.
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
                            status.ahead = ahead_str
                                .trim_end_matches(']')
                                .parse()
                                .unwrap_or(0);
                        }
                    }
                    if let Some(behind_pos) = info.find("behind ") {
                        let behind_str = &info[behind_pos + 7..];
                        if let Some(end) = behind_str.find(|c: char| !c.is_ascii_digit()) {
                            status.behind = behind_str[..end].parse().unwrap_or(0);
                        } else {
                            status.behind = behind_str
                                .trim_end_matches(']')
                                .parse()
                                .unwrap_or(0);
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
pub fn parse_encrypted_files(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            // Format: "encrypted: path/to/file" or "    encrypted: path/to/file"
            let trimmed = line.trim();
            if trimmed.starts_with("encrypted:") {
                Some(PathBuf::from(trimmed[10..].trim()))
            } else {
                None
            }
        })
        .collect()
}

/// Check if content appears to be git-crypt encrypted
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
            self.status_result
                .clone()
                .ok_or_else(|| GitError::NotARepository {
                    path: PathBuf::from("/mock"),
                }
                .into())
        }

        fn diff(&self) -> IronResult<String> {
            self.diff_result
                .clone()
                .ok_or_else(|| GitError::NotARepository {
                    path: PathBuf::from("/mock"),
                }
                .into())
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
}
