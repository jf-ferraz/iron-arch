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

pub mod bundle;
pub mod error;
pub mod host;
pub mod module;
pub mod packages;
pub mod profile;
pub mod services;
pub mod snapshot;
pub mod state;
pub mod validation;

// Re-exports for convenience
pub use bundle::{Bundle, BundleState, BundleType};
pub use error::{
    ConfigError, FsError, GitError, IronError, IronResult, PackageError, Recoverable,
    ServiceError, SnapshotError, StateError, ValidationError,
};
pub use host::{BootloaderType, ChassisType, HardwareSpec, Host, InstallParams, MonitorConfig};
pub use module::{DotfileMapping, Module, ModuleKind, ModuleState};
pub use profile::{Profile, ProfileState};
pub use state::{IronState, MaintenanceState, OperationRecord, OperationStatus};
pub use snapshot::{
    create_manager as create_snapshot_manager, detect_backend as detect_snapshot_backend,
    NoopManager, SnapshotBackend, SnapshotInfo, SnapshotManager, SnapshotType, SnapperManager,
    TimeshiftManager,
};
pub use validation::{
    check_dotfile_conflicts, check_module_conflicts, expand_home, resolve_dependencies,
    validate_config, validate_id, validate_module, validate_path, validate_path_safe,
    ValidationResult, ValidationWarning, WarningCode, MAX_ID_LENGTH,
};
pub use packages::{
    assess_risk, ArchNewsItem, InstalledPackage, NoopPackageManager, PackageManager, PackageUpdate,
    RiskLevel, UpdatePreview,
};
