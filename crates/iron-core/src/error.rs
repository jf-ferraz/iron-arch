//! Iron Error Types - Comprehensive error handling for Iron
//!
//! This module defines the error hierarchy for the Iron project:
//! - `IronError` - Top-level error type
//! - Domain-specific errors for each subsystem

use std::path::PathBuf;
use thiserror::Error;

/// Top-level error type for Iron operations
#[derive(Debug, Error)]
pub enum IronError {
    /// Configuration file errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// State management errors
    #[error("State error: {0}")]
    State(#[from] StateError),

    /// Package management errors
    #[error("Package error: {0}")]
    Package(#[from] PackageError),

    /// Git operation errors
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    /// Filesystem operation errors
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FsError),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Systemd service errors
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),

    /// Snapshot errors
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] SnapshotError),

    /// Operation cancelled by user
    #[error("Operation cancelled by user")]
    Cancelled,

    /// Generic operation failure
    #[error("Operation failed: {message}")]
    OperationFailed { message: String },

    /// IO error wrapper
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration-specific errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File not found
    #[error("Configuration file not found: {path}")]
    NotFound { path: PathBuf },

    /// Parse error (TOML syntax)
    #[error("Failed to parse {path}: {message}")]
    ParseError { path: PathBuf, message: String },

    /// Invalid value in configuration
    #[error("Invalid value for '{field}': {message}")]
    InvalidValue { field: String, message: String },

    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// Invalid ID format
    #[error("Invalid ID '{id}': {message}")]
    InvalidId { id: String, message: String },

    /// Duplicate ID
    #[error("Duplicate ID '{id}' found in {path}")]
    DuplicateId { id: String, path: PathBuf },
}

/// State management errors
#[derive(Debug, Error)]
pub enum StateError {
    /// No host configured
    #[error("No active host configured. Run 'iron init' first.")]
    NoActiveHost,

    /// Host not found
    #[error("Host '{id}' not found")]
    HostNotFound { id: String },

    /// No active bundle
    #[error("No active bundle for host '{host}'")]
    NoActiveBundle { host: String },

    /// Bundle not found
    #[error("Bundle '{id}' not found")]
    BundleNotFound { id: String },

    /// Bundle already active
    #[error("Bundle '{id}' is already active")]
    BundleAlreadyActive { id: String },

    /// Bundle not installed
    #[error("Bundle '{id}' is not installed")]
    BundleNotInstalled { id: String },

    /// Profile not found
    #[error("Profile '{id}' not found")]
    ProfileNotFound { id: String },

    /// Module not found
    #[error("Module '{id}' not found")]
    ModuleNotFound { id: String },

    /// Module already enabled
    #[error("Module '{id}' is already enabled")]
    ModuleAlreadyEnabled { id: String },

    /// Conflict detected
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// State file corruption
    #[error("State file corrupted: {path}")]
    Corrupted { path: PathBuf },

    /// Transaction failure
    #[error("Transaction failed: {message}")]
    TransactionFailed { message: String },
}

/// Package management errors
#[derive(Debug, Error)]
pub enum PackageError {
    /// Package not found
    #[error("Package '{name}' not found in repositories")]
    NotFound { name: String },

    /// Installation failed
    #[error("Failed to install packages: {message}")]
    InstallFailed { message: String },

    /// Removal failed
    #[error("Failed to remove packages: {message}")]
    RemoveFailed { message: String },

    /// Update failed
    #[error("Update failed: {message}")]
    UpdateFailed { message: String },

    /// Dependency conflict
    #[error("Dependency conflict: {message}")]
    DependencyConflict { message: String },

    /// AUR helper not found
    #[error("No AUR helper found. Install paru or yay.")]
    NoAurHelper,

    /// AUR package flagged
    #[error("AUR package '{name}' is flagged out-of-date")]
    AurFlagged { name: String },

    /// Pacman error
    #[error("Pacman error: {message}")]
    PacmanError { message: String },

    /// Pacman command failed
    #[error("Pacman failed: {message}")]
    PacmanFailed { message: String },
}

/// Git operation errors
#[derive(Debug, Error)]
pub enum GitError {
    /// Not a git repository
    #[error("Not a git repository: {path}")]
    NotARepository { path: PathBuf },

    /// No remote configured
    #[error("No remote '{name}' configured")]
    NoRemote { name: String },

    /// Uncommitted changes
    #[error("Uncommitted changes in repository")]
    UncommittedChanges,

    /// Merge conflict
    #[error("Merge conflict in {files:?}")]
    MergeConflict { files: Vec<String> },

    /// Push failed
    #[error("Push failed: {message}")]
    PushFailed { message: String },

    /// Pull failed
    #[error("Pull failed: {message}")]
    PullFailed { message: String },

    /// git-crypt not initialized
    #[error("git-crypt not initialized in repository")]
    GitCryptNotInitialized,

    /// Secrets locked
    #[error("Secrets are locked. Run 'iron secrets unlock' first.")]
    SecretsLocked,

    /// Clone failed
    #[error("Failed to clone repository: {message}")]
    CloneFailed { message: String },

    /// Git command failed
    #[error("Git command failed: {message}")]
    CommandFailed { message: String },
}

/// Filesystem operation errors
#[derive(Debug, Error)]
pub enum FsError {
    /// Path not found
    #[error("Path not found: {path}")]
    NotFound { path: PathBuf },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    /// Symlink already exists
    #[error("Symlink already exists: {path}")]
    SymlinkExists { path: PathBuf },

    /// Symlink target conflict
    #[error("Target '{target}' already linked to different source")]
    SymlinkConflict { target: PathBuf },

    /// Not a symlink
    #[error("Not a symlink: {path}")]
    NotASymlink { path: PathBuf },

    /// Backup failed
    #[error("Failed to backup {path}: {message}")]
    BackupFailed { path: PathBuf, message: String },

    /// Restore failed
    #[error("Failed to restore {path}: {message}")]
    RestoreFailed { path: PathBuf, message: String },

    /// Path escapes root
    #[error("Path '{path}' escapes allowed root")]
    PathEscapesRoot { path: PathBuf },
}

/// Validation errors (re-export with additions)
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Invalid ID format
    #[error("Invalid ID '{id}': must be lowercase alphanumeric with hyphens")]
    InvalidIdFormat { id: String },

    /// ID too long
    #[error("ID '{id}' exceeds maximum length of {max} characters")]
    IdTooLong { id: String, max: usize },

    /// Missing required field
    #[error("Missing required field: {field} in {context}")]
    MissingField { field: String, context: String },

    /// Invalid value
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    /// Module conflict
    #[error("Module '{module_a}' conflicts with '{module_b}'")]
    ModuleConflict { module_a: String, module_b: String },

    /// Bundle conflict
    #[error("Bundle '{bundle_a}' conflicts with '{bundle_b}'")]
    BundleConflict { bundle_a: String, bundle_b: String },

    /// Dotfile target conflict
    #[error("Dotfile target '{target}' claimed by both '{module_a}' and '{module_b}'")]
    DotfileConflict {
        target: String,
        module_a: String,
        module_b: String,
    },

    /// Missing dependency
    #[error("Module '{module}' requires dependency '{dependency}' which is not available")]
    MissingDependency { module: String, dependency: String },

    /// Circular dependency
    #[error("Circular dependency detected: {chain}")]
    CircularDependency { chain: String },

    /// File not found
    #[error("Required file not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Path validation failed
    #[error("Path validation failed for '{path}': {message}")]
    InvalidPath { path: PathBuf, message: String },
}

/// Systemd service errors
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Service not found
    #[error("Service '{name}' not found")]
    NotFound { name: String },

    /// Failed to enable service
    #[error("Failed to enable service '{name}': {message}")]
    EnableFailed { name: String, message: String },

    /// Failed to disable service
    #[error("Failed to disable service '{name}': {message}")]
    DisableFailed { name: String, message: String },

    /// Failed to start service
    #[error("Failed to start service '{name}': {message}")]
    StartFailed { name: String, message: String },

    /// Failed to stop service
    #[error("Failed to stop service '{name}': {message}")]
    StopFailed { name: String, message: String },

    /// Systemctl not found
    #[error("systemctl not found. Is systemd installed?")]
    SystemctlNotFound,

    /// Service not available
    #[error("Service '{service}' is not available")]
    NotAvailable { service: String },

    /// Service operation failed
    #[error("Service '{service}' operation failed: {message}")]
    OperationFailed { service: String, message: String },
}

/// Snapshot (timeshift/snapper) errors
#[derive(Debug, Error)]
pub enum SnapshotError {
    /// No snapshot tool available
    #[error("No snapshot tool found. Install timeshift or snapper.")]
    NoSnapshotTool,

    /// Snapshot creation failed
    #[error("Failed to create snapshot: {message}")]
    CreateFailed { message: String },

    /// Snapshot not found
    #[error("Snapshot '{id}' not found")]
    NotFound { id: String },

    /// Restore failed
    #[error("Failed to restore snapshot '{id}': {message}")]
    RestoreFailed { id: String, message: String },

    /// No snapshots available
    #[error("No snapshots available")]
    NoSnapshots,
}

/// Result type alias for Iron operations
pub type IronResult<T> = Result<T, IronError>;

/// Recovery action that can be suggested for errors
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Rollback to previous state
    Rollback { description: String },

    /// Retry the operation
    Retry { with_changes: String },

    /// Skip this item and continue
    Skip { item: String },

    /// Abort the entire operation
    Abort,

    /// Run a specific command
    RunCommand { command: String },

    /// Edit a configuration file
    EditConfig { path: PathBuf },
}

/// Trait for errors that can suggest recovery actions
pub trait Recoverable {
    /// Get possible automatic recovery actions
    fn auto_recovery(&self) -> Option<RecoveryAction>;

    /// Get manual recovery steps for the user
    fn manual_recovery_steps(&self) -> Vec<String>;

    /// Whether this error can be retried
    fn is_retriable(&self) -> bool;
}

impl Recoverable for IronError {
    fn auto_recovery(&self) -> Option<RecoveryAction> {
        match self {
            IronError::State(StateError::NoActiveHost) => Some(RecoveryAction::RunCommand {
                command: "iron init".to_string(),
            }),
            IronError::Git(GitError::SecretsLocked) => Some(RecoveryAction::RunCommand {
                command: "iron secrets unlock".to_string(),
            }),
            IronError::Snapshot(SnapshotError::NoSnapshotTool) => {
                Some(RecoveryAction::RunCommand {
                    command: "sudo pacman -S timeshift".to_string(),
                })
            }
            _ => None,
        }
    }

    fn manual_recovery_steps(&self) -> Vec<String> {
        match self {
            IronError::Config(ConfigError::ParseError { path, .. }) => {
                vec![
                    format!("Check the syntax of {}", path.display()),
                    "Ensure all required fields are present".to_string(),
                    "Validate TOML syntax with a linter".to_string(),
                ]
            }
            IronError::Git(GitError::MergeConflict { files }) => {
                let mut steps = vec!["Resolve the following merge conflicts:".to_string()];
                for file in files {
                    steps.push(format!("  - {}", file));
                }
                steps.push("Then run 'git add' and 'git commit'".to_string());
                steps
            }
            IronError::Package(PackageError::DependencyConflict { message }) => {
                vec![
                    format!("Dependency conflict: {}", message),
                    "Try removing conflicting packages first".to_string(),
                    "Or use 'pacman -Rdd' to force removal".to_string(),
                ]
            }
            _ => vec!["Check the error message for details".to_string()],
        }
    }

    fn is_retriable(&self) -> bool {
        matches!(
            self,
            IronError::Io(_) | IronError::Package(PackageError::UpdateFailed { .. })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_error_display() {
        let err = IronError::Config(ConfigError::NotFound {
            path: PathBuf::from("/test/path"),
        });
        assert!(err.to_string().contains("/test/path"));
    }

    #[test]
    fn test_error_conversion() {
        let config_err = ConfigError::MissingField {
            field: "id".to_string(),
        };
        let iron_err: IronError = config_err.into();
        assert!(matches!(iron_err, IronError::Config(_)));
    }

    #[test]
    fn test_recoverable_trait() {
        let err = IronError::State(StateError::NoActiveHost);
        assert!(err.auto_recovery().is_some());
        assert!(!err.manual_recovery_steps().is_empty());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::ModuleConflict {
            module_a: "nvim-ide".to_string(),
            module_b: "vim-minimal".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("nvim-ide"));
        assert!(msg.contains("vim-minimal"));
    }
}
