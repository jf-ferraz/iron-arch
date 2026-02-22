//! Iron Services - Core business logic services
//!
//! This module contains the application services that orchestrate
//! domain operations and coordinate between infrastructure crates.

pub mod bundle;
pub mod clean;
pub mod doctor;
pub mod host;
pub mod module;
pub mod profile;
pub mod recovery;
pub mod scan;
pub mod secrets;
pub mod state;
pub mod sync;
pub mod update;

// Re-export service traits and implementations
pub use bundle::{BundleService, DefaultBundleService};
pub use clean::{
    CleanupCategory, CleanupPreview, CleanupResult, CleanupService, CleanupSummary,
    DefaultCleanupService,
};
pub use doctor::{
    CheckStatus, DefaultDoctorService, DoctorConfig, DoctorService, HealthCheck, HealthReport,
};
pub use host::{DefaultHostService, HostService};
pub use module::{DefaultModuleService, ModuleService};
pub use profile::{DefaultProfileService, ProfileService};
pub use recovery::{DefaultRecoveryService, RecoveryService, VerificationResult};
pub use scan::{
    ConflictSeverity, DefaultScanService, DiscoveredConfig, ScanConflict, ScanReport, ScanService,
    ScanSummary,
};
pub use secrets::{DefaultSecretsService, SecretsBackend, SecretsService};
pub use state::{StateManager, Transaction, TransactionGuard};
pub use sync::{DefaultSyncService, SyncService};
pub use update::{
    ConfigConflict, ConfigConflictType, DefaultUpdateService, FailedService, PostUpdateResult,
    UpdateService,
};
