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

    /// No active update to resume (FR-5.10)
    #[error("No active or interrupted update to resume")]
    NoActiveUpdate,

    /// Failed to save state (FR-5.10)
    #[error("Failed to save state: {0}")]
    SaveFailed(String),
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

    /// IO error running git (command not found, permission denied, etc.)
    #[error("Git IO error: {message}")]
    IoError { message: String },
}

/// Filesystem operation errors
#[derive(Debug, Error, Clone)]
pub enum FsError {
    /// Path not found
    #[error("Path not found: {path}")]
    NotFound { path: PathBuf },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    /// Path already exists
    #[error("Path already exists: {path}")]
    AlreadyExists { path: PathBuf },

    /// Generic I/O error
    #[error("I/O error: {message}")]
    IoError { message: String },

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

    // ==========================================================================
    // IronError Tests
    // ==========================================================================

    #[test]
    fn test_iron_error_display() {
        let err = IronError::Config(ConfigError::NotFound {
            path: PathBuf::from("/test/path"),
        });
        assert!(err.to_string().contains("/test/path"));
    }

    #[test]
    fn test_iron_error_cancelled() {
        let err = IronError::Cancelled;
        assert_eq!(err.to_string(), "Operation cancelled by user");
    }

    #[test]
    fn test_iron_error_operation_failed() {
        let err = IronError::OperationFailed {
            message: "Something went wrong".to_string(),
        };
        assert!(err.to_string().contains("Something went wrong"));
    }

    // ==========================================================================
    // Error Conversion Tests
    // ==========================================================================

    #[test]
    fn test_error_conversion() {
        let config_err = ConfigError::MissingField {
            field: "id".to_string(),
        };
        let iron_err: IronError = config_err.into();
        assert!(matches!(iron_err, IronError::Config(_)));
    }

    #[test]
    fn test_state_error_conversion() {
        let state_err = StateError::NoActiveHost;
        let iron_err: IronError = state_err.into();
        assert!(matches!(iron_err, IronError::State(_)));
    }

    #[test]
    fn test_package_error_conversion() {
        let pkg_err = PackageError::NotFound {
            name: "test-pkg".to_string(),
        };
        let iron_err: IronError = pkg_err.into();
        assert!(matches!(iron_err, IronError::Package(_)));
    }

    #[test]
    fn test_git_error_conversion() {
        let git_err = GitError::UncommittedChanges;
        let iron_err: IronError = git_err.into();
        assert!(matches!(iron_err, IronError::Git(_)));
    }

    #[test]
    fn test_fs_error_conversion() {
        let fs_err = FsError::NotFound {
            path: PathBuf::from("/test"),
        };
        let iron_err: IronError = fs_err.into();
        assert!(matches!(iron_err, IronError::Filesystem(_)));
    }

    #[test]
    fn test_validation_error_conversion() {
        let val_err = ValidationError::InvalidIdFormat {
            id: "bad".to_string(),
        };
        let iron_err: IronError = val_err.into();
        assert!(matches!(iron_err, IronError::Validation(_)));
    }

    #[test]
    fn test_service_error_conversion() {
        let svc_err = ServiceError::SystemctlNotFound;
        let iron_err: IronError = svc_err.into();
        assert!(matches!(iron_err, IronError::Service(_)));
    }

    #[test]
    fn test_snapshot_error_conversion() {
        let snap_err = SnapshotError::NoSnapshotTool;
        let iron_err: IronError = snap_err.into();
        assert!(matches!(iron_err, IronError::Snapshot(_)));
    }

    // ==========================================================================
    // ConfigError Tests
    // ==========================================================================

    #[test]
    fn test_config_error_not_found() {
        let err = ConfigError::NotFound {
            path: PathBuf::from("/etc/config.toml"),
        };
        assert!(err.to_string().contains("/etc/config.toml"));
    }

    #[test]
    fn test_config_error_parse_error() {
        let err = ConfigError::ParseError {
            path: PathBuf::from("/test.toml"),
            message: "invalid syntax".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/test.toml"));
        assert!(msg.contains("invalid syntax"));
    }

    #[test]
    fn test_config_error_invalid_value() {
        let err = ConfigError::InvalidValue {
            field: "port".to_string(),
            message: "must be positive".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("port"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_config_error_duplicate_id() {
        let err = ConfigError::DuplicateId {
            id: "nvim".to_string(),
            path: PathBuf::from("/modules"),
        };
        let msg = err.to_string();
        assert!(msg.contains("nvim"));
        assert!(msg.contains("/modules"));
    }

    // ==========================================================================
    // StateError Tests
    // ==========================================================================

    #[test]
    fn test_state_error_variants() {
        let errors = vec![
            StateError::NoActiveHost,
            StateError::HostNotFound {
                id: "laptop".to_string(),
            },
            StateError::NoActiveBundle {
                host: "desktop".to_string(),
            },
            StateError::BundleNotFound {
                id: "hyprland".to_string(),
            },
            StateError::BundleAlreadyActive {
                id: "hyprland".to_string(),
            },
            StateError::BundleNotInstalled {
                id: "niri".to_string(),
            },
            StateError::ProfileNotFound {
                id: "developer".to_string(),
            },
            StateError::ModuleNotFound {
                id: "nvim-ide".to_string(),
            },
            StateError::ModuleAlreadyEnabled {
                id: "fish".to_string(),
            },
            StateError::Conflict {
                message: "test conflict".to_string(),
            },
            StateError::Corrupted {
                path: PathBuf::from("/state.json"),
            },
            StateError::TransactionFailed {
                message: "rollback needed".to_string(),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // PackageError Tests
    // ==========================================================================

    #[test]
    fn test_package_error_variants() {
        let errors = vec![
            PackageError::NotFound {
                name: "missing-pkg".to_string(),
            },
            PackageError::InstallFailed {
                message: "disk full".to_string(),
            },
            PackageError::RemoveFailed {
                message: "dependency".to_string(),
            },
            PackageError::UpdateFailed {
                message: "network".to_string(),
            },
            PackageError::DependencyConflict {
                message: "version mismatch".to_string(),
            },
            PackageError::NoAurHelper,
            PackageError::AurFlagged {
                name: "outdated-pkg".to_string(),
            },
            PackageError::PacmanError {
                message: "lock file".to_string(),
            },
            PackageError::PacmanFailed {
                message: "exit 1".to_string(),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // GitError Tests
    // ==========================================================================

    #[test]
    fn test_git_error_variants() {
        let errors = vec![
            GitError::NotARepository {
                path: PathBuf::from("/tmp"),
            },
            GitError::NoRemote {
                name: "origin".to_string(),
            },
            GitError::UncommittedChanges,
            GitError::MergeConflict {
                files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
            },
            GitError::PushFailed {
                message: "rejected".to_string(),
            },
            GitError::PullFailed {
                message: "network".to_string(),
            },
            GitError::GitCryptNotInitialized,
            GitError::SecretsLocked,
            GitError::CloneFailed {
                message: "timeout".to_string(),
            },
            GitError::CommandFailed {
                message: "exit 128".to_string(),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // FsError Tests
    // ==========================================================================

    #[test]
    fn test_fs_error_clone() {
        let err = FsError::NotFound {
            path: PathBuf::from("/test/path"),
        };
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_fs_error_variants() {
        let errors = vec![
            FsError::NotFound {
                path: PathBuf::from("/missing"),
            },
            FsError::PermissionDenied {
                path: PathBuf::from("/root"),
            },
            FsError::AlreadyExists {
                path: PathBuf::from("/exists"),
            },
            FsError::IoError {
                message: "disk error".to_string(),
            },
            FsError::SymlinkExists {
                path: PathBuf::from("/link"),
            },
            FsError::SymlinkConflict {
                target: PathBuf::from("/target"),
            },
            FsError::NotASymlink {
                path: PathBuf::from("/regular"),
            },
            FsError::BackupFailed {
                path: PathBuf::from("/backup"),
                message: "no space".to_string(),
            },
            FsError::RestoreFailed {
                path: PathBuf::from("/restore"),
                message: "corrupted".to_string(),
            },
            FsError::PathEscapesRoot {
                path: PathBuf::from("../../../etc"),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // ValidationError Tests
    // ==========================================================================

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

    #[test]
    fn test_validation_error_variants() {
        let errors = vec![
            ValidationError::InvalidIdFormat {
                id: "BAD_ID".to_string(),
            },
            ValidationError::IdTooLong {
                id: "very_long_id".to_string(),
                max: 64,
            },
            ValidationError::MissingField {
                field: "name".to_string(),
                context: "module.toml".to_string(),
            },
            ValidationError::InvalidValue {
                field: "priority".to_string(),
                message: "must be 1-10".to_string(),
            },
            ValidationError::ModuleConflict {
                module_a: "a".to_string(),
                module_b: "b".to_string(),
            },
            ValidationError::BundleConflict {
                bundle_a: "x".to_string(),
                bundle_b: "y".to_string(),
            },
            ValidationError::DotfileConflict {
                target: "~/.config/nvim".to_string(),
                module_a: "nvim".to_string(),
                module_b: "neovim".to_string(),
            },
            ValidationError::MissingDependency {
                module: "child".to_string(),
                dependency: "parent".to_string(),
            },
            ValidationError::CircularDependency {
                chain: "a -> b -> a".to_string(),
            },
            ValidationError::FileNotFound {
                path: PathBuf::from("/missing.toml"),
            },
            ValidationError::InvalidPath {
                path: PathBuf::from("../escape"),
                message: "path traversal".to_string(),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // ServiceError Tests
    // ==========================================================================

    #[test]
    fn test_service_error_variants() {
        let errors = vec![
            ServiceError::NotFound {
                name: "missing.service".to_string(),
            },
            ServiceError::EnableFailed {
                name: "test.service".to_string(),
                message: "permission denied".to_string(),
            },
            ServiceError::DisableFailed {
                name: "test.service".to_string(),
                message: "in use".to_string(),
            },
            ServiceError::StartFailed {
                name: "test.service".to_string(),
                message: "exit code 1".to_string(),
            },
            ServiceError::StopFailed {
                name: "test.service".to_string(),
                message: "timeout".to_string(),
            },
            ServiceError::SystemctlNotFound,
            ServiceError::NotAvailable {
                service: "nvidia.service".to_string(),
            },
            ServiceError::OperationFailed {
                service: "docker.service".to_string(),
                message: "unknown".to_string(),
            },
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // SnapshotError Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_error_variants() {
        let errors = vec![
            SnapshotError::NoSnapshotTool,
            SnapshotError::CreateFailed {
                message: "no space".to_string(),
            },
            SnapshotError::NotFound {
                id: "snap-123".to_string(),
            },
            SnapshotError::RestoreFailed {
                id: "snap-456".to_string(),
                message: "corrupted".to_string(),
            },
            SnapshotError::NoSnapshots,
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    // ==========================================================================
    // RecoveryAction Tests
    // ==========================================================================

    #[test]
    fn test_recovery_action_debug() {
        let actions = vec![
            RecoveryAction::Rollback {
                description: "undo last change".to_string(),
            },
            RecoveryAction::Retry {
                with_changes: "increase timeout".to_string(),
            },
            RecoveryAction::Skip {
                item: "failing module".to_string(),
            },
            RecoveryAction::Abort,
            RecoveryAction::RunCommand {
                command: "iron init".to_string(),
            },
            RecoveryAction::EditConfig {
                path: PathBuf::from("/config.toml"),
            },
        ];

        for action in actions {
            let debug_str = format!("{:?}", action);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_recovery_action_clone() {
        let action = RecoveryAction::RunCommand {
            command: "test".to_string(),
        };
        let cloned = action.clone();
        match cloned {
            RecoveryAction::RunCommand { command } => assert_eq!(command, "test"),
            _ => panic!("Clone failed"),
        }
    }

    // ==========================================================================
    // Recoverable Trait Tests
    // ==========================================================================

    #[test]
    fn test_recoverable_trait() {
        let err = IronError::State(StateError::NoActiveHost);
        assert!(err.auto_recovery().is_some());
        assert!(!err.manual_recovery_steps().is_empty());
    }

    #[test]
    fn test_recoverable_secrets_locked() {
        let err = IronError::Git(GitError::SecretsLocked);
        let recovery = err.auto_recovery();
        assert!(recovery.is_some());
        match recovery.unwrap() {
            RecoveryAction::RunCommand { command } => {
                assert!(command.contains("unlock"));
            }
            _ => panic!("Expected RunCommand"),
        }
    }

    #[test]
    fn test_recoverable_no_snapshot_tool() {
        let err = IronError::Snapshot(SnapshotError::NoSnapshotTool);
        let recovery = err.auto_recovery();
        assert!(recovery.is_some());
        match recovery.unwrap() {
            RecoveryAction::RunCommand { command } => {
                assert!(command.contains("timeshift"));
            }
            _ => panic!("Expected RunCommand"),
        }
    }

    #[test]
    fn test_recoverable_no_auto_recovery() {
        let err = IronError::Cancelled;
        assert!(err.auto_recovery().is_none());
    }

    #[test]
    fn test_manual_recovery_parse_error() {
        let err = IronError::Config(ConfigError::ParseError {
            path: PathBuf::from("/test.toml"),
            message: "syntax error".to_string(),
        });
        let steps = err.manual_recovery_steps();
        assert!(steps.len() >= 2);
        assert!(steps[0].contains("/test.toml"));
    }

    #[test]
    fn test_manual_recovery_merge_conflict() {
        let err = IronError::Git(GitError::MergeConflict {
            files: vec!["a.rs".to_string(), "b.rs".to_string()],
        });
        let steps = err.manual_recovery_steps();
        assert!(steps.len() >= 3);
        assert!(steps.iter().any(|s| s.contains("a.rs")));
        assert!(steps.iter().any(|s| s.contains("b.rs")));
    }

    #[test]
    fn test_manual_recovery_dependency_conflict() {
        let err = IronError::Package(PackageError::DependencyConflict {
            message: "libfoo requires bar >= 2.0".to_string(),
        });
        let steps = err.manual_recovery_steps();
        assert!(!steps.is_empty());
        assert!(steps[0].contains("libfoo"));
    }

    #[test]
    fn test_is_retriable_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let err = IronError::Io(io_err);
        assert!(err.is_retriable());
    }

    #[test]
    fn test_is_retriable_update_failed() {
        let err = IronError::Package(PackageError::UpdateFailed {
            message: "network error".to_string(),
        });
        assert!(err.is_retriable());
    }

    #[test]
    fn test_is_not_retriable() {
        let err = IronError::Config(ConfigError::NotFound {
            path: PathBuf::from("/missing"),
        });
        assert!(!err.is_retriable());
    }
}
