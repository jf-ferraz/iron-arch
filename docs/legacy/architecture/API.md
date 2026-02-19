# Iron API Reference

> **Version**: 1.0.0
> **Last Updated**: 2025-02-12

---

## Table of Contents

1. [Core Services](#1-core-services)
2. [Domain Types](#2-domain-types)
3. [Infrastructure Interfaces](#3-infrastructure-interfaces)
4. [Event System](#4-event-system)

---

## 1. Core Services

### 1.1 HostService

Manages host configurations and hardware detection.

```rust
pub trait HostService {
    /// Detect the current host by hostname or hardware fingerprint
    fn detect_current(&self) -> Result<Host>;

    /// Catalog hardware specifications of current machine
    fn catalog_hardware(&self) -> Result<HardwareSpec>;

    /// List all configured hosts
    fn list_hosts(&self) -> Result<Vec<Host>>;

    /// Get host by ID
    fn get_host(&self, id: &str) -> Result<Option<Host>>;

    /// Save host configuration
    fn save_host(&self, host: &Host) -> Result<()>;

    /// Delete host configuration
    fn delete_host(&self, id: &str) -> Result<()>;

    /// Generate install script for recovery
    fn generate_install_script(&self, host: &Host) -> Result<String>;

    /// Check if host has a recent snapshot
    fn has_snapshot(&self, host: &Host) -> Result<bool>;
}
```

### 1.2 BundleService

Manages desktop environment bundles.

```rust
pub trait BundleService {
    /// List all available bundles
    fn list_bundles(&self) -> Result<Vec<Bundle>>;

    /// Get bundle by ID
    fn get_bundle(&self, id: &str) -> Result<Option<Bundle>>;

    /// Get currently active bundle for current host
    fn get_active(&self) -> Result<Option<Bundle>>;

    /// Get bundle state (NotInstalled, Dormant, Active)
    fn get_state(&self, id: &str) -> Result<BundleState>;

    /// Install a bundle (packages only, doesn't activate)
    fn install(&self, id: &str) -> Result<InstallResult>;

    /// Activate a bundle (link dotfiles, start services)
    fn activate(&self, id: &str) -> Result<ActivationResult>;

    /// Deactivate current bundle (move to dormant)
    fn deactivate(&self) -> Result<()>;

    /// Switch from current bundle to another
    fn switch(&self, to: &str) -> Result<SwitchResult>;

    /// Uninstall a bundle
    fn uninstall(&self, id: &str) -> Result<()>;

    /// Check conflicts between bundles
    fn check_conflicts(&self, ids: &[&str]) -> Result<Vec<BundleConflict>>;
}

/// Result of bundle activation
pub struct ActivationResult {
    pub bundle_id: String,
    pub packages_installed: usize,
    pub dotfiles_linked: usize,
    pub services_enabled: Vec<String>,
    pub warnings: Vec<Warning>,
}

/// Result of bundle switch
pub struct SwitchResult {
    pub from_bundle: String,
    pub to_bundle: String,
    pub snapshot_id: Option<String>,
    pub activation_result: ActivationResult,
}
```

### 1.3 ProfileService

Manages dotfile profiles.

```rust
pub trait ProfileService {
    /// List all available profiles
    fn list_profiles(&self) -> Result<Vec<Profile>>;

    /// Get profile by ID
    fn get_profile(&self, id: &str) -> Result<Option<Profile>>;

    /// Get currently active profile
    fn get_active(&self) -> Result<Option<Profile>>;

    /// Get profiles compatible with a bundle
    fn get_for_bundle(&self, bundle_id: &str) -> Result<Vec<Profile>>;

    /// Select/activate a profile
    fn select(&self, id: &str) -> Result<SelectResult>;

    /// Create a new profile
    fn create(&self, profile: Profile) -> Result<()>;

    /// Update an existing profile
    fn update(&self, profile: Profile) -> Result<()>;

    /// Delete a profile
    fn delete(&self, id: &str) -> Result<()>;

    /// Get effective modules for a profile (including inherited)
    fn get_effective_modules(&self, id: &str) -> Result<Vec<Module>>;

    /// Preview profile application
    fn preview(&self, id: &str) -> Result<ProfilePreview>;
}

/// Result of profile selection
pub struct SelectResult {
    pub profile_id: String,
    pub modules_enabled: Vec<String>,
    pub dotfiles_linked: usize,
    pub conflicts_resolved: Vec<ConflictResolution>,
}

/// Profile application preview
pub struct ProfilePreview {
    pub modules_to_enable: Vec<String>,
    pub modules_to_disable: Vec<String>,
    pub dotfiles_to_link: Vec<DotfileChange>,
    pub potential_conflicts: Vec<Conflict>,
}
```

### 1.4 ModuleService

Manages individual configuration modules.

```rust
pub trait ModuleService {
    /// List all available modules
    fn list_modules(&self) -> Result<Vec<Module>>;

    /// Get module by ID
    fn get_module(&self, id: &str) -> Result<Option<Module>>;

    /// Get module state
    fn get_state(&self, id: &str) -> Result<ModuleState>;

    /// Get all enabled modules
    fn get_enabled(&self) -> Result<Vec<Module>>;

    /// Enable a module
    fn enable(&self, id: &str) -> Result<EnableResult>;

    /// Disable a module
    fn disable(&self, id: &str) -> Result<DisableResult>;

    /// Check conflicts for a set of modules
    fn check_conflicts(&self, ids: &[&str]) -> Result<Vec<ModuleConflict>>;

    /// Get modules that depend on the given module
    fn get_dependents(&self, id: &str) -> Result<Vec<Module>>;

    /// Get modules that the given module depends on
    fn get_dependencies(&self, id: &str) -> Result<Vec<Module>>;

    /// Run module hooks
    fn run_hooks(&self, id: &str, hook_type: HookType) -> Result<HookResult>;
}

/// Module conflict information
pub struct ModuleConflict {
    pub module_a: String,
    pub module_b: String,
    pub conflict_type: ConflictType,
    pub details: String,
}

/// Types of conflicts
pub enum ConflictType {
    /// Both modules target the same dotfile path
    DotfilePath { path: PathBuf },
    /// Explicit conflict declaration in module.toml
    Declared,
    /// Package conflict detected
    Package { package: String },
}
```

### 1.5 UpdateService

Manages system updates with risk assessment.

```rust
pub trait UpdateService {
    /// Check for available updates
    fn check_updates(&self) -> Result<UpdateCheck>;

    /// Fetch Arch Linux news
    fn fetch_arch_news(&self) -> Result<Vec<NewsItem>>;

    /// Check for flagged AUR packages
    fn check_aur_flags(&self) -> Result<Vec<FlaggedPackage>>;

    /// Calculate risk score for pending updates
    fn calculate_risk(&self, updates: &UpdateCheck) -> Result<RiskAssessment>;

    /// Preview update (dry-run)
    fn preview(&self) -> Result<UpdatePreview>;

    /// Execute update
    fn execute(&self, config: UpdateConfig) -> Result<UpdateResult>;

    /// Handle pacnew files
    fn handle_pacnew(&self) -> Result<Vec<PacnewAction>>;
}

/// Update check results
pub struct UpdateCheck {
    pub pacman_updates: Vec<PackageUpdate>,
    pub aur_updates: Vec<PackageUpdate>,
    pub total_download_size: u64,
    pub news_items: Vec<NewsItem>,
}

/// Risk assessment
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub score: u8,  // 0-100
    pub factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
}

/// Risk factors
pub struct RiskFactor {
    pub name: String,
    pub weight: u8,
    pub description: String,
    pub mitigations: Vec<String>,
}

/// Update configuration
pub struct UpdateConfig {
    pub create_snapshot: bool,
    pub include_aur: bool,
    pub non_interactive: bool,
    pub approved: bool,
}
```

### 1.6 SyncService

Manages git synchronization.

```rust
pub trait SyncService {
    /// Get sync status
    fn status(&self) -> Result<SyncStatus>;

    /// Check for local changes
    fn has_local_changes(&self) -> Result<bool>;

    /// Check for remote changes
    fn has_remote_changes(&self) -> Result<bool>;

    /// Commit local changes
    fn commit(&self, message: &str) -> Result<()>;

    /// Push to remote
    fn push(&self) -> Result<PushResult>;

    /// Pull from remote
    fn pull(&self) -> Result<PullResult>;

    /// Get diff of changes
    fn diff(&self) -> Result<String>;

    /// Resolve conflicts
    fn resolve_conflict(&self, path: &Path, resolution: ConflictResolution) -> Result<()>;
}

/// Sync status
pub struct SyncStatus {
    pub local_changes: usize,
    pub remote_changes: usize,
    pub last_sync: Option<DateTime<Utc>>,
    pub remote_url: Option<String>,
    pub branch: String,
}
```

### 1.7 SecretsService

Manages encrypted secrets.

```rust
pub trait SecretsService {
    /// Check if secrets are unlocked
    fn is_unlocked(&self) -> bool;

    /// Unlock secrets
    fn unlock(&self) -> Result<()>;

    /// Lock secrets
    fn lock(&self) -> Result<()>;

    /// List available secrets
    fn list(&self) -> Result<Vec<SecretInfo>>;

    /// Get secret status
    fn status(&self) -> Result<SecretsStatus>;

    /// Link secrets to target locations
    fn link(&self) -> Result<LinkResult>;

    /// Unlink secrets
    fn unlink(&self) -> Result<()>;
}

/// Secret information
pub struct SecretInfo {
    pub name: String,
    pub category: SecretCategory,
    pub target_path: PathBuf,
    pub linked: bool,
}

/// Secret categories
pub enum SecretCategory {
    Ssh,
    Gpg,
    Token,
    Other,
}
```

### 1.8 RecoveryService

Manages system recovery.

```rust
pub trait RecoveryService {
    /// Generate Arch install script from host config
    fn generate_install_script(&self, host: &Host) -> Result<String>;

    /// Export complete system state
    fn export_state(&self) -> Result<StateExport>;

    /// Import state from export
    fn import_state(&self, export: &StateExport) -> Result<()>;

    /// Run recovery wizard
    fn run_wizard(&self) -> Result<RecoveryResult>;

    /// Verify installation
    fn verify(&self) -> Result<VerificationResult>;
}

/// State export
pub struct StateExport {
    pub host: Host,
    pub active_bundle: Option<String>,
    pub active_profile: Option<String>,
    pub enabled_modules: Vec<String>,
    pub packages: Vec<String>,
    pub services: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Verification result
pub struct VerificationResult {
    pub passed: bool,
    pub checks: Vec<VerificationCheck>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
```

---

## 2. Domain Types

### 2.1 Core Entities

```rust
/// Host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub hardware: HardwareSpec,
    pub install_params: Option<InstallParams>,
    pub installed_bundles: Vec<String>,
    pub active_bundle: Option<String>,
}

/// Bundle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub bundle_type: BundleType,
    pub packages: Vec<String>,
    pub aur_packages: Vec<String>,
    pub profiles: Vec<String>,
    pub default_profile: Option<String>,
    pub conflicts: Vec<String>,
    pub services: Vec<String>,
    pub post_install: Option<String>,
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub modules: Vec<String>,
    pub theme: Option<String>,
    pub shell: Option<String>,
    pub extends: Option<String>,
    pub for_bundle: Option<String>,
}

/// Module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: ModuleKind,
    pub packages: Vec<String>,
    pub aur_packages: Vec<String>,
    pub dotfiles: Vec<DotfileMapping>,
    pub conflicts: Vec<String>,
    pub depends: Vec<String>,
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
}
```

### 2.2 Value Types

```rust
/// Hardware specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSpec {
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub ram_mb: Option<u64>,
    pub monitors: Vec<MonitorConfig>,
    pub chassis: Option<ChassisType>,
}

/// Dotfile mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotfileMapping {
    pub source: String,
    pub target: String,
    pub link: bool,
}

/// Installation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallParams {
    pub partitions: Vec<PartitionConfig>,
    pub bootloader: BootloaderType,
    pub kernel: String,
    pub microcode: Option<String>,
    pub gpu_drivers: Vec<String>,
    pub filesystem: String,
    pub encrypted: bool,
}
```

### 2.3 Enumerations

```rust
/// Bundle types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BundleType {
    WaylandCompositor,
    DesktopEnvironment,
    X11WindowManager,
}

/// Bundle states
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BundleState {
    NotInstalled,
    Dormant,
    Active,
}

/// Module kinds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModuleKind {
    AppConfig,
    Shell,
    DesktopComponent,
    Theme,
    SystemUtil,
    DevTools,
}

/// Module states
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModuleState {
    NotInstalled,
    Installed,
    Partial,
    Failed,
}

/// Risk levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Chassis types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChassisType {
    Desktop,
    Laptop,
    Server,
    Tablet,
    Convertible,
    Unknown,
}
```

---

## 3. Infrastructure Interfaces

### 3.1 PackageManager

```rust
/// Package manager abstraction
pub trait PackageManager {
    /// Query installed packages
    fn query_installed(&self) -> Result<Vec<InstalledPackage>>;

    /// Check if package is installed
    fn is_installed(&self, name: &str) -> Result<bool>;

    /// Search for packages
    fn search(&self, query: &str) -> Result<Vec<AvailablePackage>>;

    /// Get package info
    fn info(&self, name: &str) -> Result<Option<PackageInfo>>;

    /// Install packages
    fn install(&self, packages: &[&str], aur: bool) -> Result<InstallResult>;

    /// Remove packages
    fn remove(&self, packages: &[&str]) -> Result<RemoveResult>;

    /// Update packages
    fn update(&self, packages: Option<&[&str]>) -> Result<UpdateResult>;

    /// Check for updates
    fn check_updates(&self) -> Result<Vec<AvailableUpdate>>;

    /// Clean cache
    fn clean_cache(&self, keep: usize) -> Result<CleanResult>;
}
```

### 3.2 FileSystem

```rust
/// File system operations
pub trait FileSystem {
    /// Create symlink
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<()>;

    /// Remove symlink
    fn remove_symlink(&self, path: &Path) -> Result<()>;

    /// Check if path is valid symlink to expected source
    fn is_valid_symlink(&self, path: &Path, expected: &Path) -> Result<bool>;

    /// Copy file/directory
    fn copy(&self, from: &Path, to: &Path) -> Result<()>;

    /// Move file/directory
    fn move_path(&self, from: &Path, to: &Path) -> Result<()>;

    /// Create backup of file
    fn backup(&self, path: &Path) -> Result<PathBuf>;

    /// Read TOML file
    fn read_toml<T: DeserializeOwned>(&self, path: &Path) -> Result<T>;

    /// Write TOML file
    fn write_toml<T: Serialize>(&self, path: &Path, value: &T) -> Result<()>;
}
```

### 3.3 SnapshotManager

```rust
/// Snapshot management (Timeshift/Snapper)
pub trait SnapshotManager {
    /// Get snapshot tool type
    fn tool_type(&self) -> SnapshotTool;

    /// Create snapshot
    fn create(&self, description: &str) -> Result<Snapshot>;

    /// List snapshots
    fn list(&self) -> Result<Vec<Snapshot>>;

    /// Get snapshot by ID
    fn get(&self, id: &str) -> Result<Option<Snapshot>>;

    /// Restore snapshot
    fn restore(&self, id: &str) -> Result<()>;

    /// Delete snapshot
    fn delete(&self, id: &str) -> Result<()>;

    /// Check if tool is available
    fn is_available(&self) -> bool;
}

/// Snapshot tools
pub enum SnapshotTool {
    Timeshift,
    Snapper,
    None,
}
```

---

## 4. Event System

### 4.1 Events

```rust
/// Iron events
pub enum IronEvent {
    /// Bundle events
    Bundle(BundleEvent),

    /// Profile events
    Profile(ProfileEvent),

    /// Module events
    Module(ModuleEvent),

    /// Update events
    Update(UpdateEvent),

    /// Sync events
    Sync(SyncEvent),

    /// State change events
    State(StateEvent),
}

/// Bundle-related events
pub enum BundleEvent {
    Installing { id: String },
    Installed { id: String, result: InstallResult },
    Activating { id: String },
    Activated { id: String },
    Deactivating { id: String },
    Deactivated { id: String },
    Switching { from: String, to: String },
    Switched { from: String, to: String },
}

/// Update-related events
pub enum UpdateEvent {
    CheckingUpdates,
    UpdatesAvailable { count: usize },
    FetchingNews,
    CalculatingRisk,
    RiskCalculated { level: RiskLevel },
    AwaitingApproval,
    CreatingSnapshot,
    SnapshotCreated { id: String },
    Updating { progress: UpdateProgress },
    UpdateComplete { result: UpdateResult },
}

/// Progress information
pub struct UpdateProgress {
    pub current: usize,
    pub total: usize,
    pub current_package: String,
}
```

### 4.2 Event Handler

```rust
/// Event handler trait
pub trait EventHandler: Send + Sync {
    /// Handle an event
    fn handle(&self, event: &IronEvent);
}

/// Event dispatcher
pub struct EventDispatcher {
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventDispatcher {
    /// Register an event handler
    pub fn register(&mut self, handler: Box<dyn EventHandler>);

    /// Dispatch an event to all handlers
    pub fn dispatch(&self, event: IronEvent);
}
```

---

## Appendix: Error Types

See [ARCHITECTURE.md](./ARCHITECTURE.md#11-error-handling) for complete error type definitions.
