//! Sync Service - Git synchronization and repository management
//!
//! Provides git sync status, push/pull workflows, and conflict detection.

use crate::services::state::StateManager;
use crate::state::OperationStatus;
use crate::{GitError, IronResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Repository is up to date
    UpToDate,
    /// Local changes need to be pushed
    Ahead,
    /// Remote changes need to be pulled
    Behind,
    /// Both local and remote have changes
    Diverged,
    /// Working tree has uncommitted changes
    Dirty,
    /// Not a git repository
    NotARepo,
}

/// Detailed sync information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInfo {
    /// Current status
    pub status: SyncStatus,
    /// Current branch
    pub branch: Option<String>,
    /// Remote tracking branch
    pub remote_branch: Option<String>,
    /// Number of commits ahead
    pub commits_ahead: usize,
    /// Number of commits behind
    pub commits_behind: usize,
    /// Uncommitted file count
    pub dirty_files: usize,
    /// Last sync timestamp
    pub last_sync: Option<chrono::DateTime<Utc>>,
}

/// Sync service trait
pub trait SyncService {
    /// Get sync status
    fn status(&self) -> IronResult<SyncInfo>;

    /// Pull remote changes
    fn pull(&self) -> IronResult<()>;

    /// Push local changes
    fn push(&self) -> IronResult<()>;

    /// Full sync (pull then push)
    fn sync(&self) -> IronResult<()>;

    /// Commit all changes with message
    fn commit(&self, message: &str) -> IronResult<()>;

    /// Check for conflicts
    fn check_conflicts(&self) -> IronResult<Vec<String>>;

    /// Stash uncommitted changes
    fn stash(&self) -> IronResult<()>;

    /// Pop stashed changes
    fn stash_pop(&self) -> IronResult<()>;
}

/// Default sync service implementation
pub struct DefaultSyncService {
    /// Repository root
    repo_root: PathBuf,
    /// State manager
    state_manager: StateManager,
}

impl DefaultSyncService {
    /// Create a new sync service
    pub fn new(repo_root: &Path, state_manager: StateManager) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            state_manager,
        }
    }

    /// Run git command
    fn git(&self, args: &[&str]) -> IronResult<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|_| GitError::NotARepository {
                path: self.repo_root.clone(),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitError::CommandFailed { message: stderr }.into())
        }
    }

    /// Check if this is a git repository
    fn is_repo(&self) -> bool {
        self.repo_root.join(".git").exists()
            || Command::new("git")
                .args(["rev-parse", "--git-dir"])
                .current_dir(&self.repo_root)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    /// Get current branch name
    fn current_branch(&self) -> Option<String> {
        self.git(&["branch", "--show-current"])
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Get upstream tracking branch
    fn tracking_branch(&self) -> Option<String> {
        self.git(&["rev-parse", "--abbrev-ref", "@{upstream}"])
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Get ahead/behind counts
    fn ahead_behind(&self) -> (usize, usize) {
        if let Ok(output) = self.git(&["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        {
            let parts: Vec<&str> = output.split_whitespace().collect();
            if parts.len() == 2 {
                let behind = parts[0].parse().unwrap_or(0);
                let ahead = parts[1].parse().unwrap_or(0);
                return (ahead, behind);
            }
        }
        (0, 0)
    }

    /// Count dirty files
    fn dirty_count(&self) -> usize {
        self.git(&["status", "--porcelain"])
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    /// Fetch from remote
    fn fetch(&self) -> IronResult<()> {
        self.git(&["fetch", "--quiet"])?;
        Ok(())
    }
}

impl SyncService for DefaultSyncService {
    fn status(&self) -> IronResult<SyncInfo> {
        if !self.is_repo() {
            return Ok(SyncInfo {
                status: SyncStatus::NotARepo,
                branch: None,
                remote_branch: None,
                commits_ahead: 0,
                commits_behind: 0,
                dirty_files: 0,
                last_sync: None,
            });
        }

        // Fetch to get latest remote state
        let _ = self.fetch();

        let branch = self.current_branch();
        let remote_branch = self.tracking_branch();
        let (ahead, behind) = self.ahead_behind();
        let dirty = self.dirty_count();

        let status = if dirty > 0 {
            SyncStatus::Dirty
        } else if ahead > 0 && behind > 0 {
            SyncStatus::Diverged
        } else if ahead > 0 {
            SyncStatus::Ahead
        } else if behind > 0 {
            SyncStatus::Behind
        } else {
            SyncStatus::UpToDate
        };

        Ok(SyncInfo {
            status,
            branch,
            remote_branch,
            commits_ahead: ahead,
            commits_behind: behind,
            dirty_files: dirty,
            last_sync: self.state_manager.maintenance().last_sync,
        })
    }

    fn pull(&self) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        self.git(&["pull", "--rebase"])?;

        self.state_manager
            .record_operation("git_pull", OperationStatus::Success, None)?;

        Ok(())
    }

    fn push(&self) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        self.git(&["push"])?;

        self.state_manager
            .record_operation("git_push", OperationStatus::Success, None)?;

        Ok(())
    }

    fn sync(&self) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        // Check for conflicts first
        let conflicts = self.check_conflicts()?;
        if !conflicts.is_empty() {
            return Err(GitError::MergeConflict { files: conflicts }.into());
        }

        // Pull first
        self.pull()?;

        // Then push
        self.push()?;

        // Update sync timestamp
        self.state_manager.update_maintenance("sync")?;

        Ok(())
    }

    fn commit(&self, message: &str) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        // Stage all changes
        self.git(&["add", "-A"])?;

        // Commit
        self.git(&["commit", "-m", message])?;

        self.state_manager.record_operation(
            "git_commit",
            OperationStatus::Success,
            Some(message.to_string()),
        )?;

        Ok(())
    }

    fn check_conflicts(&self) -> IronResult<Vec<String>> {
        if !self.is_repo() {
            return Ok(vec![]);
        }

        // Fetch to check for potential conflicts
        let _ = self.fetch();

        // Check for unmerged files
        let unmerged = self
            .git(&["diff", "--name-only", "--diff-filter=U"])
            .unwrap_or_default();

        Ok(unmerged.lines().map(|s| s.to_string()).collect())
    }

    fn stash(&self) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        self.git(&["stash", "push", "-m", "iron auto-stash"])?;
        Ok(())
    }

    fn stash_pop(&self) -> IronResult<()> {
        if !self.is_repo() {
            return Err(GitError::NotARepository {
                path: self.repo_root.clone(),
            }
            .into());
        }

        self.git(&["stash", "pop"])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultSyncService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let service = DefaultSyncService::new(temp_dir.path(), state_manager);
        (service, temp_dir)
    }

    fn init_git_repo(path: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    // ==========================================================================
    // SyncStatus Tests
    // ==========================================================================

    #[test]
    fn test_sync_status_equality() {
        assert_eq!(SyncStatus::UpToDate, SyncStatus::UpToDate);
        assert_eq!(SyncStatus::Ahead, SyncStatus::Ahead);
        assert_eq!(SyncStatus::Behind, SyncStatus::Behind);
        assert_eq!(SyncStatus::Diverged, SyncStatus::Diverged);
        assert_eq!(SyncStatus::Dirty, SyncStatus::Dirty);
        assert_eq!(SyncStatus::NotARepo, SyncStatus::NotARepo);
    }

    #[test]
    fn test_sync_status_inequality() {
        assert_ne!(SyncStatus::UpToDate, SyncStatus::Ahead);
        assert_ne!(SyncStatus::Behind, SyncStatus::Diverged);
        assert_ne!(SyncStatus::Dirty, SyncStatus::NotARepo);
    }

    #[test]
    fn test_sync_status_clone() {
        let status = SyncStatus::Diverged;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_sync_status_copy() {
        let status = SyncStatus::Behind;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_sync_status_debug() {
        let statuses = vec![
            SyncStatus::UpToDate,
            SyncStatus::Ahead,
            SyncStatus::Behind,
            SyncStatus::Diverged,
            SyncStatus::Dirty,
            SyncStatus::NotARepo,
        ];

        for status in statuses {
            let debug_str = format!("{:?}", status);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_sync_status_serialization() {
        let statuses = vec![
            SyncStatus::UpToDate,
            SyncStatus::Ahead,
            SyncStatus::Behind,
            SyncStatus::Diverged,
            SyncStatus::Dirty,
            SyncStatus::NotARepo,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: SyncStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    // ==========================================================================
    // SyncInfo Tests
    // ==========================================================================

    #[test]
    fn test_sync_info_creation() {
        let info = SyncInfo {
            status: SyncStatus::UpToDate,
            branch: Some("main".to_string()),
            remote_branch: Some("origin/main".to_string()),
            commits_ahead: 0,
            commits_behind: 0,
            dirty_files: 0,
            last_sync: None,
        };

        assert_eq!(info.status, SyncStatus::UpToDate);
        assert_eq!(info.branch, Some("main".to_string()));
        assert_eq!(info.commits_ahead, 0);
    }

    #[test]
    fn test_sync_info_ahead() {
        let info = SyncInfo {
            status: SyncStatus::Ahead,
            branch: Some("feature".to_string()),
            remote_branch: Some("origin/feature".to_string()),
            commits_ahead: 5,
            commits_behind: 0,
            dirty_files: 0,
            last_sync: None,
        };

        assert_eq!(info.status, SyncStatus::Ahead);
        assert_eq!(info.commits_ahead, 5);
        assert_eq!(info.commits_behind, 0);
    }

    #[test]
    fn test_sync_info_diverged() {
        let info = SyncInfo {
            status: SyncStatus::Diverged,
            branch: Some("develop".to_string()),
            remote_branch: Some("origin/develop".to_string()),
            commits_ahead: 3,
            commits_behind: 2,
            dirty_files: 0,
            last_sync: None,
        };

        assert_eq!(info.status, SyncStatus::Diverged);
        assert_eq!(info.commits_ahead, 3);
        assert_eq!(info.commits_behind, 2);
    }

    #[test]
    fn test_sync_info_dirty() {
        let info = SyncInfo {
            status: SyncStatus::Dirty,
            branch: Some("main".to_string()),
            remote_branch: None,
            commits_ahead: 0,
            commits_behind: 0,
            dirty_files: 5,
            last_sync: None,
        };

        assert_eq!(info.status, SyncStatus::Dirty);
        assert_eq!(info.dirty_files, 5);
    }

    #[test]
    fn test_sync_info_clone() {
        let info = SyncInfo {
            status: SyncStatus::UpToDate,
            branch: Some("main".to_string()),
            remote_branch: None,
            commits_ahead: 1,
            commits_behind: 1,
            dirty_files: 2,
            last_sync: Some(Utc::now()),
        };

        let cloned = info.clone();
        assert_eq!(cloned.status, SyncStatus::UpToDate);
        assert_eq!(cloned.branch, Some("main".to_string()));
        assert_eq!(cloned.commits_ahead, 1);
    }

    #[test]
    fn test_sync_info_debug() {
        let info = SyncInfo {
            status: SyncStatus::Behind,
            branch: Some("test".to_string()),
            remote_branch: Some("origin/test".to_string()),
            commits_ahead: 0,
            commits_behind: 3,
            dirty_files: 0,
            last_sync: None,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("Behind"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_sync_info_serialization() {
        let info = SyncInfo {
            status: SyncStatus::Ahead,
            branch: Some("serial-test".to_string()),
            remote_branch: Some("origin/serial-test".to_string()),
            commits_ahead: 2,
            commits_behind: 1,
            dirty_files: 3,
            last_sync: Some(Utc::now()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SyncInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, SyncStatus::Ahead);
        assert_eq!(deserialized.branch, Some("serial-test".to_string()));
        assert_eq!(deserialized.commits_ahead, 2);
    }

    #[test]
    fn test_sync_info_no_branch() {
        let info = SyncInfo {
            status: SyncStatus::NotARepo,
            branch: None,
            remote_branch: None,
            commits_ahead: 0,
            commits_behind: 0,
            dirty_files: 0,
            last_sync: None,
        };

        assert!(info.branch.is_none());
        assert!(info.remote_branch.is_none());
        assert!(info.last_sync.is_none());
    }

    // ==========================================================================
    // DefaultSyncService Tests
    // ==========================================================================

    #[test]
    fn test_status_not_a_repo() {
        let (service, _temp) = create_test_service();
        let info = service.status().unwrap();
        assert_eq!(info.status, SyncStatus::NotARepo);
    }

    #[test]
    fn test_status_clean_repo() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create initial commit
        std::fs::write(temp_dir.path().join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let info = service.status().unwrap();
        // Without remote, status should be UpToDate
        assert!(info.status == SyncStatus::UpToDate || info.status == SyncStatus::Ahead);
    }

    #[test]
    fn test_status_dirty_repo() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create uncommitted file
        std::fs::write(temp_dir.path().join("dirty.txt"), "dirty").unwrap();

        let info = service.status().unwrap();
        assert_eq!(info.status, SyncStatus::Dirty);
        assert_eq!(info.dirty_files, 1);
    }

    #[test]
    fn test_status_multiple_dirty_files() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create multiple uncommitted files
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
        std::fs::write(temp_dir.path().join("file3.txt"), "content3").unwrap();

        let info = service.status().unwrap();
        assert_eq!(info.status, SyncStatus::Dirty);
        assert_eq!(info.dirty_files, 3);
    }

    #[test]
    fn test_is_repo() {
        let (service, temp_dir) = create_test_service();

        assert!(!service.is_repo());

        init_git_repo(temp_dir.path());

        assert!(service.is_repo());
    }

    #[test]
    fn test_commit_in_git_repo() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create a file and commit it
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let result = service.commit("Test commit");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_conflicts_empty() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        let conflicts = service.check_conflicts().unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_stash_clean_repo() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create initial commit
        std::fs::write(temp_dir.path().join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Stash on clean repo should work (no-op)
        let result = service.stash();
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_has_branch() {
        let (service, temp_dir) = create_test_service();
        init_git_repo(temp_dir.path());

        // Create initial commit
        std::fs::write(temp_dir.path().join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let info = service.status().unwrap();
        assert!(info.branch.is_some());
    }
}
