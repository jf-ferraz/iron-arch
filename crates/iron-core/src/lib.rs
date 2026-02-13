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

pub mod availability;
pub mod bundle;
pub mod error;
pub mod fs_trait;
pub mod host;
pub mod logging;
pub mod module;
pub mod packages;
pub mod profile;
pub mod resilience;
pub mod services;
pub mod snapshot;
pub mod state;
pub mod validation;

#[cfg(test)]
pub mod test_helpers;

// Re-exports for convenience
pub use bundle::{Bundle, BundleState, BundleType};
pub use error::{
    ConfigError, FsError, GitError, IronError, IronResult, PackageError, Recoverable, ServiceError,
    SnapshotError, StateError, ValidationError,
};
pub use host::{BootloaderType, ChassisType, HardwareSpec, Host, InstallParams, MonitorConfig};
pub use module::{DotfileMapping, Module, ModuleKind, ModuleState};
pub use packages::{
    ArchNewsItem, InstalledPackage, NoopPackageManager, PackageManager, PackageUpdate, RiskLevel,
    UpdatePreview, assess_risk,
};
pub use profile::{Profile, ProfileState};
pub use snapshot::{
    NoopManager, SnapperManager, SnapshotBackend, SnapshotInfo, SnapshotManager, SnapshotType,
    TimeshiftManager, create_manager as create_snapshot_manager,
    detect_backend as detect_snapshot_backend,
};
pub use state::{IronState, MaintenanceState, OperationRecord, OperationStatus};
pub use validation::{
    MAX_ID_LENGTH, ValidationResult, ValidationWarning, WarningCode, check_dotfile_conflicts,
    check_module_conflicts, expand_home, resolve_dependencies, validate_config, validate_id,
    validate_module, validate_path, validate_path_safe,
};

// Filesystem abstraction for testing
pub use fs_trait::{FileSystem, FsResult, MockFileSystem, RealFileSystem};

// Resilience patterns
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitOpenError, CircuitState,
    CommandConfig, CommandError, CommandExecutor, CommandOutput, RealCommandExecutor,
};
