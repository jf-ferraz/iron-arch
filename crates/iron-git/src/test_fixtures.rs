//! Test fixtures for mocking git and git-crypt commands.
//!
//! This module provides pre-configured mock responses for common git commands,
//! enabling comprehensive testing of `DefaultGitManager` and `DefaultSecretsManager`
//! without requiring actual git repository operations.
//!
//! # Usage
//!
//! ```rust,ignore
//! use iron_core::resilience::MockCommandExecutor;
//! use iron_git::test_fixtures::GitMockBuilder;
//!
//! let executor = GitMockBuilder::new()
//!     .with_branch("main")
//!     .with_modified_files(&["src/lib.rs"])
//!     .with_ahead_behind(2, 0)
//!     .build();
//!
//! // Use with DefaultGitManager::with_executor()
//! ```

use iron_core::resilience::{MockCommandExecutor, MockResponse};

/// Builder for creating configured `MockCommandExecutor` with git-specific responses.
///
/// Provides a fluent API for setting up mock responses for git commands,
/// enabling isolated testing of git operations.
#[derive(Debug, Default)]
pub struct GitMockBuilder {
    /// Current branch name
    branch: Option<String>,
    /// Tracking remote (e.g., "origin/main")
    tracking: Option<String>,
    /// Modified files
    modified_files: Vec<String>,
    /// Untracked files
    untracked_files: Vec<String>,
    /// Staged files
    staged_files: Vec<String>,
    /// Conflict files
    conflict_files: Vec<String>,
    /// Commits ahead of remote
    ahead: usize,
    /// Commits behind remote
    behind: usize,
    /// Diff content to return
    diff_content: String,
    /// Whether commit should succeed
    commit_succeeds: bool,
    /// Whether push should succeed
    push_succeeds: bool,
    /// Whether pull should succeed
    pull_succeeds: bool,
    /// Whether git-crypt is initialized
    gitcrypt_initialized: bool,
    /// Whether secrets are unlocked
    secrets_unlocked: bool,
    /// Encrypted files list
    encrypted_files: Vec<String>,
    /// Repository root path (for -C argument matching)
    repo_root: Option<String>,
}

impl GitMockBuilder {
    /// Create a new builder with default settings (clean repository on main)
    pub fn new() -> Self {
        Self {
            branch: Some("main".to_string()),
            tracking: Some("origin/main".to_string()),
            commit_succeeds: true,
            push_succeeds: true,
            pull_succeeds: true,
            gitcrypt_initialized: false,
            secrets_unlocked: true,
            ..Default::default()
        }
    }

    /// Set the current branch
    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self.tracking = Some(format!("origin/{}", branch));
        self
    }

    /// Set a custom tracking reference
    pub fn with_tracking(mut self, tracking: &str) -> Self {
        self.tracking = Some(tracking.to_string());
        self
    }

    /// Set no tracking (detached HEAD or no upstream)
    pub fn without_tracking(mut self) -> Self {
        self.tracking = None;
        self
    }

    /// Add modified files
    pub fn with_modified_files(mut self, files: &[&str]) -> Self {
        self.modified_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add untracked files
    pub fn with_untracked_files(mut self, files: &[&str]) -> Self {
        self.untracked_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add staged files
    pub fn with_staged_files(mut self, files: &[&str]) -> Self {
        self.staged_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add conflict files (for merge conflict scenarios)
    pub fn with_conflict_files(mut self, files: &[&str]) -> Self {
        self.conflict_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set ahead/behind counts
    pub fn with_ahead_behind(mut self, ahead: usize, behind: usize) -> Self {
        self.ahead = ahead;
        self.behind = behind;
        self
    }

    /// Set the diff content to return
    pub fn with_diff(mut self, diff: &str) -> Self {
        self.diff_content = diff.to_string();
        self
    }

    /// Set whether commit should succeed
    pub fn commit_succeeds(mut self, succeeds: bool) -> Self {
        self.commit_succeeds = succeeds;
        self
    }

    /// Set whether push should succeed
    pub fn push_succeeds(mut self, succeeds: bool) -> Self {
        self.push_succeeds = succeeds;
        self
    }

    /// Set whether pull should succeed
    pub fn pull_succeeds(mut self, succeeds: bool) -> Self {
        self.pull_succeeds = succeeds;
        self
    }

    /// Enable git-crypt with specified state
    pub fn with_gitcrypt(mut self, unlocked: bool) -> Self {
        self.gitcrypt_initialized = true;
        self.secrets_unlocked = unlocked;
        self
    }

    /// Add encrypted files for git-crypt status
    pub fn with_encrypted_files(mut self, files: &[&str]) -> Self {
        self.encrypted_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the repository root path for -C argument matching
    pub fn with_repo_root(mut self, path: &str) -> Self {
        self.repo_root = Some(path.to_string());
        self
    }

    /// Generate the git status --porcelain -b output
    fn generate_status_output(&self) -> String {
        let mut lines = Vec::new();

        // Branch line
        let branch_line = if let Some(ref branch) = self.branch {
            if let Some(ref tracking) = self.tracking {
                let tracking_info = match (self.ahead, self.behind) {
                    (0, 0) => String::new(),
                    (a, 0) => format!(" [ahead {}]", a),
                    (0, b) => format!(" [behind {}]", b),
                    (a, b) => format!(" [ahead {}, behind {}]", a, b),
                };
                format!("## {}...{}{}", branch, tracking, tracking_info)
            } else {
                format!("## {}", branch)
            }
        } else {
            "## HEAD (no branch)".to_string()
        };
        lines.push(branch_line);

        // Conflict files (UU prefix)
        for file in &self.conflict_files {
            lines.push(format!("UU {}", file));
        }

        // Staged files (A or M in first column)
        for file in &self.staged_files {
            lines.push(format!("A  {}", file));
        }

        // Modified files (M in second column)
        for file in &self.modified_files {
            lines.push(format!(" M {}", file));
        }

        // Untracked files (??)
        for file in &self.untracked_files {
            lines.push(format!("?? {}", file));
        }

        lines.join("\n")
    }

    /// Generate git-crypt status -e output
    fn generate_gitcrypt_status(&self) -> String {
        self.encrypted_files
            .iter()
            .map(|f| format!("    encrypted: {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build the configured `MockCommandExecutor`
    pub fn build(self) -> MockCommandExecutor {
        let executor = MockCommandExecutor::new();

        // Configure git status --porcelain -b
        let status_output = self.generate_status_output();
        executor.add_response(
            "git",
            &["-C", self.repo_root.as_deref().unwrap_or("."), "status", "--porcelain", "-b"],
            MockResponse::success(&status_output),
        );
        // Also add without -C for direct calls
        executor.add_response(
            "git",
            &["status", "--porcelain", "-b"],
            MockResponse::success(&status_output),
        );

        // Configure git status --porcelain (without -b, used for conflict check)
        let status_no_branch = self.generate_status_output()
            .lines()
            .skip(1)  // Skip branch line
            .collect::<Vec<_>>()
            .join("\n");
        executor.add_response(
            "git",
            &["-C", self.repo_root.as_deref().unwrap_or("."), "status", "--porcelain"],
            MockResponse::success(&status_no_branch),
        );
        executor.add_response(
            "git",
            &["status", "--porcelain"],
            MockResponse::success(&status_no_branch),
        );

        // Configure git diff
        executor.add_response(
            "git",
            &["-C", self.repo_root.as_deref().unwrap_or("."), "diff"],
            MockResponse::success(&self.diff_content),
        );
        executor.add_response(
            "git",
            &["diff"],
            MockResponse::success(&self.diff_content),
        );

        // Configure git commit
        if self.commit_succeeds {
            // Use fallback for commit since message varies
            executor.add_fallback_response(
                "git",
                MockResponse::success("[main abc1234] Commit message\n 1 file changed"),
            );
        } else {
            executor.add_fallback_response(
                "git",
                MockResponse::exit_error(1, "error: nothing to commit"),
            );
        }

        // Configure git push (use fallback since remote/branch vary)
        if self.push_succeeds {
            // We need specific responses for push commands
            for remote in &["origin"] {
                for branch in &["main", "master", "develop"] {
                    executor.add_response(
                        "git",
                        &["-C", self.repo_root.as_deref().unwrap_or("."), "push", remote, branch],
                        MockResponse::success("Everything up-to-date"),
                    );
                    executor.add_response(
                        "git",
                        &["push", remote, branch],
                        MockResponse::success("Everything up-to-date"),
                    );
                }
            }
        } else {
            for remote in &["origin"] {
                for branch in &["main", "master", "develop"] {
                    executor.add_response(
                        "git",
                        &["-C", self.repo_root.as_deref().unwrap_or("."), "push", remote, branch],
                        MockResponse::exit_error(1, "error: failed to push some refs"),
                    );
                    executor.add_response(
                        "git",
                        &["push", remote, branch],
                        MockResponse::exit_error(1, "error: failed to push some refs"),
                    );
                }
            }
        }

        // Configure git pull
        if self.pull_succeeds && self.conflict_files.is_empty() {
            for remote in &["origin"] {
                for branch in &["main", "master", "develop"] {
                    executor.add_response(
                        "git",
                        &["-C", self.repo_root.as_deref().unwrap_or("."), "pull", remote, branch],
                        MockResponse::success("Already up to date."),
                    );
                    executor.add_response(
                        "git",
                        &["pull", remote, branch],
                        MockResponse::success("Already up to date."),
                    );
                }
            }
        } else {
            for remote in &["origin"] {
                for branch in &["main", "master", "develop"] {
                    executor.add_response(
                        "git",
                        &["-C", self.repo_root.as_deref().unwrap_or("."), "pull", remote, branch],
                        MockResponse::exit_error(1, "CONFLICT (content): Merge conflict"),
                    );
                    executor.add_response(
                        "git",
                        &["pull", remote, branch],
                        MockResponse::exit_error(1, "CONFLICT (content): Merge conflict"),
                    );
                }
            }
        }

        // Configure git-crypt commands
        if self.gitcrypt_initialized {
            // git-crypt status -e
            let gitcrypt_status = self.generate_gitcrypt_status();
            executor.add_response(
                "git-crypt",
                &["status", "-e"],
                MockResponse::success(&gitcrypt_status),
            );

            // git-crypt unlock
            if self.secrets_unlocked {
                executor.add_response(
                    "git-crypt",
                    &["unlock"],
                    MockResponse::success(""),
                );
            } else {
                executor.add_response(
                    "git-crypt",
                    &["unlock"],
                    MockResponse::exit_error(1, "Error: no key"),
                );
            }

            // git-crypt lock
            executor.add_response(
                "git-crypt",
                &["lock"],
                MockResponse::success(""),
            );
        }

        // Add git and git-crypt to existing commands
        executor.add_existing_command("git-crypt");

        executor
    }
}

// =============================================================================
// Pre-built Fixture Sets
// =============================================================================

/// Common git scenarios for testing
pub mod fixtures {
    use super::*;

    /// Clean repository on main branch
    pub fn clean_repo() -> GitMockBuilder {
        GitMockBuilder::new()
    }

    /// Repository with uncommitted changes
    pub fn dirty_repo() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_modified_files(&["src/lib.rs", "src/main.rs"])
            .with_diff(
                "diff --git a/src/lib.rs b/src/lib.rs\n\
                 index abc1234..def5678 100644\n\
                 --- a/src/lib.rs\n\
                 +++ b/src/lib.rs\n\
                 @@ -1,3 +1,4 @@\n\
                 +// New comment\n\
                  fn main() {}\n",
            )
    }

    /// Repository with staged changes ready to commit
    pub fn staged_changes() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_staged_files(&["src/new_file.rs"])
            .with_modified_files(&["src/lib.rs"])
    }

    /// Repository with untracked files
    pub fn untracked_files() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_untracked_files(&["new_file.txt", "temp/", "*.log"])
    }

    /// Repository ahead of remote
    pub fn ahead_of_remote() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_ahead_behind(3, 0)
    }

    /// Repository behind remote (needs pull)
    pub fn behind_remote() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_ahead_behind(0, 5)
    }

    /// Repository with diverged history
    pub fn diverged() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_ahead_behind(2, 3)
    }

    /// Repository with merge conflicts
    pub fn merge_conflict() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_conflict_files(&["src/lib.rs", "Cargo.toml"])
            .pull_succeeds(false)
    }

    /// Repository on feature branch
    pub fn feature_branch() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_branch("feature/new-feature")
            .with_modified_files(&["src/feature.rs"])
            .with_ahead_behind(5, 0)
    }

    /// Repository with git-crypt initialized and unlocked
    pub fn gitcrypt_unlocked() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_gitcrypt(true)
            .with_encrypted_files(&["secrets/api_key.txt", "secrets/database.env"])
    }

    /// Repository with git-crypt locked
    pub fn gitcrypt_locked() -> GitMockBuilder {
        GitMockBuilder::new()
            .with_gitcrypt(false)
            .with_encrypted_files(&["secrets/api_key.txt", "secrets/database.env"])
    }

    /// Repository where push fails (e.g., no access)
    pub fn push_denied() -> GitMockBuilder {
        GitMockBuilder::new()
            .push_succeeds(false)
    }

    /// Repository where commit fails (nothing staged)
    pub fn nothing_to_commit() -> GitMockBuilder {
        GitMockBuilder::new()
            .commit_succeeds(false)
    }

    /// Detached HEAD state
    pub fn detached_head() -> GitMockBuilder {
        GitMockBuilder {
            branch: None,
            tracking: None,
            commit_succeeds: true,
            push_succeeds: false, // Can't push without branch
            pull_succeeds: false,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::resilience::CommandExecutor;

    #[test]
    fn test_builder_creates_executor() {
        let executor = GitMockBuilder::new().build();
        assert_eq!(executor.total_call_count(), 0);
    }

    #[test]
    fn test_clean_repo_status() {
        let executor = GitMockBuilder::new().build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("## main...origin/main"));
        // Clean repo has no file changes
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1); // Only branch line
    }

    #[test]
    fn test_dirty_repo_status() {
        let executor = GitMockBuilder::new()
            .with_modified_files(&["src/lib.rs"])
            .with_untracked_files(&["new.txt"])
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains(" M src/lib.rs"));
        assert!(output.contains("?? new.txt"));
    }

    #[test]
    fn test_ahead_behind_status() {
        let executor = GitMockBuilder::new()
            .with_ahead_behind(3, 2)
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("[ahead 3, behind 2]"));
    }

    #[test]
    fn test_ahead_only_status() {
        let executor = GitMockBuilder::new()
            .with_ahead_behind(5, 0)
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("[ahead 5]"));
        assert!(!output.contains("behind"));
    }

    #[test]
    fn test_behind_only_status() {
        let executor = GitMockBuilder::new()
            .with_ahead_behind(0, 3)
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("[behind 3]"));
        assert!(!output.contains("ahead"));
    }

    #[test]
    fn test_feature_branch() {
        let executor = GitMockBuilder::new()
            .with_branch("feature/my-feature")
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("## feature/my-feature...origin/feature/my-feature"));
    }

    #[test]
    fn test_conflict_files() {
        let executor = GitMockBuilder::new()
            .with_conflict_files(&["src/lib.rs", "Cargo.toml"])
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("UU src/lib.rs"));
        assert!(output.contains("UU Cargo.toml"));
    }

    #[test]
    fn test_staged_files() {
        let executor = GitMockBuilder::new()
            .with_staged_files(&["src/new.rs"])
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("A  src/new.rs"));
    }

    #[test]
    fn test_diff_output() {
        let diff = "diff --git a/file.rs b/file.rs\n+new line";
        let executor = GitMockBuilder::new()
            .with_diff(diff)
            .build();

        let output = executor
            .execute("git", &["diff"])
            .expect("should execute");

        assert_eq!(output, diff);
    }

    #[test]
    fn test_push_success() {
        let executor = GitMockBuilder::new()
            .push_succeeds(true)
            .build();

        let result = executor.execute("git", &["push", "origin", "main"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_push_failure() {
        let executor = GitMockBuilder::new()
            .push_succeeds(false)
            .build();

        let result = executor.execute("git", &["push", "origin", "main"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_pull_success() {
        let executor = GitMockBuilder::new()
            .pull_succeeds(true)
            .build();

        let result = executor.execute("git", &["pull", "origin", "main"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pull_conflict() {
        let executor = GitMockBuilder::new()
            .with_conflict_files(&["src/lib.rs"])
            .build();

        let result = executor.execute("git", &["pull", "origin", "main"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gitcrypt_status() {
        let executor = GitMockBuilder::new()
            .with_gitcrypt(true)
            .with_encrypted_files(&["secrets/key.txt"])
            .build();

        let output = executor
            .execute("git-crypt", &["status", "-e"])
            .expect("should execute");

        assert!(output.contains("encrypted: secrets/key.txt"));
    }

    #[test]
    fn test_gitcrypt_unlock() {
        let executor = GitMockBuilder::new()
            .with_gitcrypt(true)
            .build();

        let result = executor.execute("git-crypt", &["unlock"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gitcrypt_lock() {
        let executor = GitMockBuilder::new()
            .with_gitcrypt(true)
            .build();

        let result = executor.execute("git-crypt", &["lock"]);
        assert!(result.is_ok());
    }

    // Fixture tests
    #[test]
    fn test_fixtures_clean_repo() {
        let executor = fixtures::clean_repo().build();
        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_fixtures_dirty_repo() {
        let executor = fixtures::dirty_repo().build();
        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .unwrap();
        assert!(output.contains("src/lib.rs"));
    }

    #[test]
    fn test_fixtures_merge_conflict() {
        let executor = fixtures::merge_conflict().build();
        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .unwrap();
        assert!(output.contains("UU"));
    }

    #[test]
    fn test_fixtures_gitcrypt_unlocked() {
        let executor = fixtures::gitcrypt_unlocked().build();
        let output = executor
            .execute("git-crypt", &["status", "-e"])
            .unwrap();
        assert!(output.contains("secrets/"));
    }

    #[test]
    fn test_no_tracking_branch() {
        let executor = GitMockBuilder::new()
            .with_branch("local-only")
            .without_tracking()
            .build();

        let output = executor
            .execute("git", &["status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("## local-only"));
        assert!(!output.contains("..."));
    }

    #[test]
    fn test_with_repo_root() {
        let executor = GitMockBuilder::new()
            .with_repo_root("/home/user/project")
            .build();

        let output = executor
            .execute("git", &["-C", "/home/user/project", "status", "--porcelain", "-b"])
            .expect("should execute");

        assert!(output.contains("## main"));
    }
}
