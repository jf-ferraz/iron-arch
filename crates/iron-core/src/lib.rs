//! Iron Core - Domain logic for Iron configuration management
//!
//! This crate contains the core business logic for Iron, including:
//! - Host management (hardware catalog, system config)
//! - Bundle management (desktop environments)
//! - Profile management (dotfile collections)
//! - Module management (individual components)
//! - State tracking and persistence
//! - Comprehensive error handling
//! - Configuration validation
//! - Snapshot management

pub mod actual_state;
pub mod availability;
pub mod bundle;
pub mod envelope;
pub mod error;
pub mod fs_trait;
pub mod host;
pub mod install;
pub mod logging;
pub mod module;
pub mod packages;
pub mod profile;
pub mod resilience;
pub mod services;
pub mod snapshot;
pub mod state;
pub mod system_service;
pub mod templates;
pub mod validation;

#[cfg(test)]
pub mod test_helpers;

/// Test fixtures for snapshot tool mocking (timeshift/snapper)
#[cfg(test)]
pub mod snapshot_fixtures;

// Re-exports for convenience
pub use actual_state::{
    ActualFileState, ActualServiceState, ActualState, FileStateType, ManagedFileSpec,
    ManagedServiceSpec,
};
pub use bundle::{Bundle, BundleState, BundleType};
pub use error::{
    ConfigError, FsError, GitError, IronError, IronResult, PackageError, Recoverable, ServiceError,
    SnapshotError, StateError, ValidationError,
};
pub use host::{BootloaderType, ChassisType, HardwareSpec, Host, InstallParams, MonitorConfig};
pub use install::{
    InstallEvent, InstallPhase, InstallPhaseId, InstallPlan, InstallRunConfig, InstallRunMode,
    InstallRunner, InstallStatus, InstallStep,
};
pub use module::{DotfileMapping, Module, ModuleKind, ModuleState};
pub use packages::{
    ArchNewsItem, CleanCacheResult, InstalledPackage, NoopPackageManager, PackageManager,
    PackageUpdate, RiskLevel, UpdatePreview, assess_risk,
};
pub use profile::{Profile, ProfileState};
pub use snapshot::{
    NoopManager, SnapperManager, SnapshotBackend, SnapshotInfo, SnapshotManager, SnapshotType,
    TimeshiftManager, create_manager as create_snapshot_manager,
    detect_backend as detect_snapshot_backend,
};
pub use state::{
    CompletedPackage, IronState, MaintenanceState, NewsAcknowledgment, OperationRecord,
    OperationStatus, SavedPackage, SavedUpdatePlan, UpdatePhase, UpdateProgress,
};
pub use validation::{
    MAX_ID_LENGTH, ValidationResult, ValidationWarning, WarningCode, check_dotfile_conflicts,
    check_module_conflicts, expand_home, resolve_dependencies, validate_config, validate_id,
    validate_module, validate_path, validate_path_safe,
};

// Filesystem abstraction for testing
pub use fs_trait::{FileSystem, FsResult, MockFileSystem, RealFileSystem};

// System service abstraction
pub use system_service::{NoopSystemService, SystemService};

// Resilience patterns
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitOpenError, CircuitState,
    CommandConfig, CommandError, CommandExecutor, CommandOutput, RealCommandExecutor,
};

// Structured logging (NFR-9, NFR-10)
pub use logging::{
    DEFAULT_MAX_SIZE_BYTES, LogConfig, OperationSpan, generate_correlation_id, init_logging,
};
