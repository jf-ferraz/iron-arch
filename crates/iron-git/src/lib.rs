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
            .map_err(|e| iron_core::IronError::Io(e))?;

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
        let mut status = GitStatus::default();

        for line in output.lines() {
            if line.starts_with("##") {
                // Parse branch info
                if let Some(branch) = line.strip_prefix("## ") {
                    if let Some((branch_name, _)) = branch.split_once("...") {
                        status.branch = Some(branch_name.to_string());
                    } else {
                        status.branch = Some(branch.to_string());
                    }
                }
            } else if line.len() >= 3 {
                let indicator = &line[0..2];
                let file = PathBuf::from(line[3..].trim());

                match indicator {
                    "??" => status.untracked.push(file),
                    " M" | "MM" => status.modified.push(file),
                    "M " | "A " => status.staged.push(file),
                    _ => {}
                }
            }
        }

        status.is_clean =
            status.modified.is_empty() && status.untracked.is_empty() && status.staged.is_empty();

        Ok(status)
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
                if let Ok(content) = std::fs::read(&entry.path()) {
                    // Encrypted files start with specific git-crypt header
                    if content.starts_with(b"\x00GITCRYPT") {
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
            .map_err(|e| iron_core::IronError::Io(e))?;

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
            .map_err(|e| iron_core::IronError::Io(e))?;

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
            .map_err(|e| iron_core::IronError::Io(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<PathBuf> = stdout
            .lines()
            .filter_map(|line| {
                // Format: "encrypted: path/to/file"
                line.strip_prefix("encrypted: ")
                    .map(|p| PathBuf::from(p.trim()))
            })
            .collect();

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_default() {
        let status = GitStatus::default();
        // Default is_clean is false (not explicitly set to true)
        assert!(!status.is_clean);
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
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
    }

    #[test]
    fn test_pull_result() {
        let result = PullResult {
            success: true,
            updated_files: vec![],
            has_conflicts: false,
            conflict_files: vec![],
        };
        assert!(result.success);
        assert!(!result.has_conflicts);
    }
}
