//! Iron Services - Core business logic services
//!
//! This module contains the application services that orchestrate
//! domain operations and coordinate between infrastructure crates.

pub mod apply;
pub mod bundle;
pub mod clean;
pub mod doctor;
pub mod drift;
pub mod history;
pub mod host;
pub mod module;
pub mod profile;
pub mod recovery;
pub mod scan;
pub mod secrets;
pub mod security;
pub mod snapshot_service;
pub mod state;
pub mod sync;
pub mod update;

// Re-export service traits and implementations
pub use apply::{
    ApplyAction, ApplyPlan, ApplyResult, ApplyService, DEFAULT_HOOK_TIMEOUT, DefaultApplyService,
    DesiredState, HookOutput, builtin_variables, resolve_desired_state,
};
pub use bundle::{BundleService, DefaultBundleService};
pub use clean::{
    CleanupCategory, CleanupPreview, CleanupResult, CleanupService, CleanupSummary,
    DefaultCleanupService,
};
pub use doctor::{
    CheckStatus, DefaultDoctorService, DoctorConfig, DoctorService, HealthCheck, HealthReport,
};
pub use drift::{
    ConfigDrift, DefaultDriftService, DriftReport, DriftService, DriftSummary, PackageDrift,
    ServiceDrift,
};
pub use history::{DefaultHistoryService, HistoryEntry, HistoryService};
pub use host::{DefaultHostService, HostService};
pub use module::{DefaultModuleService, ModuleService};
pub use profile::{DefaultProfileService, ProfileService};
pub use recovery::{DefaultRecoveryService, RecoveryService, VerificationResult};
pub use scan::{
    ConflictSeverity, DefaultScanService, DiscoveredConfig, ScanConflict, ScanReport, ScanService,
    ScanSummary,
};
pub use secrets::{DefaultSecretsService, SecretsBackend, SecretsService};
pub use security::{
    DefaultSecurityService, SecurityLevel, SecurityModuleInfo, SecurityReport, SecurityService,
};
pub use snapshot_service::{DefaultSnapshotService, SnapshotRecord, SnapshotService};
pub use state::{StateManager, Transaction, TransactionGuard};
pub use sync::{DefaultSyncService, SyncService};
pub use update::{
    ConfigConflict, ConfigConflictType, DefaultUpdateService, FailedService, PostUpdateResult,
    UpdateService,
};
