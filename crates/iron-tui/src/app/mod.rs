//! Iron TUI Application State
//!
//! Manages application state, navigation, and service integration.

mod actions;
mod handlers;

use crate::message::{MessageLevel, StatusMessage};
use crate::ui::operation_log::OperationFilter;
use crate::widgets::ProgressTracker;
use crate::wizard::{TextInput, WizardState};
use crate::install_wizard::InstallWizardState;
use iron_core::{
    ArchNewsItem, Bundle, Module, NoopPackageManager, NoopSystemService, PackageManager,
    PackageUpdate, Profile, RiskLevel, SystemService,
    services::StateManager,
    services::clean::{CleanupCategory, CleanupPreview, CleanupSummary},
    services::sync::{DefaultSyncService, SyncInfo},
    services::update::{PostUpdateResult, PreflightResult, UnacknowledgedNews},
};
use std::path::PathBuf;
use std::sync::Arc;

/// Update view section for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateSection {
    /// Pre-flight checks section
    #[default]
    PreflightChecks,
    /// News section
    News,
    /// Package list section
    Packages,
}

/// Application state
pub struct App {
    /// Current view
    pub view: View,
    /// Previous view for back navigation
    pub previous_view: Option<View>,
    /// Should quit
    pub should_quit: bool,
    /// Config directory path
    pub config_dir: PathBuf,
    /// State manager
    pub state_manager: Option<StateManager>,
    /// Current host ID
    pub current_host: Option<String>,
    /// Discovered hosts for HostSelection view (S1-P2-001)
    pub discovered_hosts: Vec<iron_core::host::Host>,
    /// Active bundle
    pub active_bundle: Option<Bundle>,
    /// Active profile ID
    pub active_profile: Option<String>,
    /// Available bundles
    pub bundles: Vec<Bundle>,
    /// Available profiles
    pub profiles: Vec<Profile>,
    /// Available modules
    pub modules: Vec<Module>,
    /// Active module IDs
    pub active_modules: Vec<String>,
    /// Selected index in list views
    pub selected_index: usize,
    /// Status message with expiration
    pub status_message: Option<StatusMessage>,
    /// Error message with expiration
    pub error_message: Option<StatusMessage>,
    /// Show help overlay
    pub show_help: bool,
    /// Show confirm dialog
    pub show_confirm: bool,
    /// Confirm action pending
    pub confirm_action: Option<ConfirmAction>,
    /// Confirmation dialog style (risk-differentiated)
    pub confirm_style: ConfirmStyle,
    /// Typed confirmation input buffer (for Critical risk)
    pub confirm_typed_input: String,
    /// Wizard state
    pub wizard: WizardState,
    /// Host name input
    pub host_input: TextInput,
    /// GPG key ID input (D-004)
    pub gpg_key_input: TextInput,
    /// Import file path input (D-003)
    pub import_path_input: TextInput,
    /// Package manager (injected)
    pub package_manager: Arc<dyn PackageManager>,
    /// Service manager for systemd operations (injected)
    pub service_manager: Arc<dyn SystemService>,
    /// Installed package count (cached)
    pub installed_count: usize,
    /// Pending updates (cached)
    pub pending_updates: Vec<PackageUpdate>,
    /// Update risk level
    pub update_risk: RiskLevel,
    /// Pre-flight check results (Phase 2.3)
    pub preflight_result: Option<PreflightResult>,
    /// Fetched Arch news items (Phase 2.3)
    pub arch_news: Vec<ArchNewsItem>,
    /// Current section in update view for navigation
    pub update_section: UpdateSection,
    /// Selected index within the current update section
    pub update_section_index: usize,
    /// Whether a reboot is recommended after updates
    pub reboot_required: bool,
    /// Post-update detection results (Phase 2.4)
    pub post_update_result: Option<PostUpdateResult>,
    // -------------------------------------------------------------------------
    // Phase 3: System Cleanup State
    // -------------------------------------------------------------------------
    /// Selected cleanup categories (Phase 3)
    pub cleanup_categories: Vec<CleanupCategory>,
    /// Cleanup preview results (Phase 3)
    pub cleanup_previews: Vec<CleanupPreview>,
    /// Cleanup execution summary (Phase 3)
    pub cleanup_summary: Option<CleanupSummary>,
    /// Whether cleanup is in preview mode (vs execution mode)
    pub cleanup_preview_mode: bool,
    // -------------------------------------------------------------------------
    // Sync State
    // -------------------------------------------------------------------------
    /// Cached sync info from git
    pub sync_info: Option<SyncInfo>,
    /// Conflicted files detected during sync (unmerged paths)
    pub sync_conflicts: Vec<String>,
    /// A-009: Reusable sync service (initialized after state_manager is set)
    pub sync_service: Option<DefaultSyncService>,
    /// D-009: Whether a background sync operation is running
    pub sync_in_progress: bool,
    /// D-009: Channel receiver for background sync result
    pub sync_result_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    // -------------------------------------------------------------------------
    // Operation Log State
    // -------------------------------------------------------------------------
    /// Active operation filter
    pub operation_filter: OperationFilter,
    // -------------------------------------------------------------------------
    // Progress Dialog State
    // -------------------------------------------------------------------------
    /// Active progress tracker for long-running operations
    pub progress: Option<ProgressTracker>,
    // -------------------------------------------------------------------------
    // Module Conflict State (Phase 2.4)
    // -------------------------------------------------------------------------
    /// Conflicts for the currently-selected module (populated on ModuleDetail nav)
    pub module_conflicts: Vec<String>,
    // -------------------------------------------------------------------------
    // Secrets State (Phase 4.1)
    // -------------------------------------------------------------------------
    /// Secrets encryption status string (matches SecretsStatus enum variant name)
    pub secrets_status: Option<String>,
    /// List of encrypted files tracked by git-crypt
    pub encrypted_files: Vec<std::path::PathBuf>,
    // -------------------------------------------------------------------------
    // Recovery State (Phase 4.2)
    // -------------------------------------------------------------------------
    /// Timestamp of the last backup/export, if known
    pub last_backup: Option<chrono::DateTime<chrono::Utc>>,
    // -------------------------------------------------------------------------
    // Snapshot State (Phase 2.2)
    // -------------------------------------------------------------------------
    /// Detected snapshot backend (Timeshift, Snapper, or None)
    pub snapshot_backend: iron_core::snapshot::SnapshotBackend,
    // -------------------------------------------------------------------------
    // Install Wizard State
    // -------------------------------------------------------------------------
    /// Integrated Arch install wizard state.
    pub install_wizard: Option<InstallWizardState>,
    // -------------------------------------------------------------------------
    // Profile Builder State (Phase 4.4)
    // -------------------------------------------------------------------------
    /// Current step in the profile builder wizard (0=name, 1=modules, 2=preview, 3=done)
    pub profile_builder_step: usize,
    /// Name being entered in the profile builder
    pub profile_builder_name: String,
    /// Description being entered in the profile builder
    pub profile_builder_description: String,
    /// Module IDs checked in the profile builder module selection step
    pub profile_builder_selected_modules: Vec<String>,
    /// Cursor position within the module checklist
    pub profile_builder_module_cursor: usize,
    /// Whether profile name input is in edit mode
    pub profile_builder_editing: bool,
    /// Whether description input is in focus (vs name)
    pub profile_builder_editing_desc: bool,
    // -------------------------------------------------------------------------
    // Module Creator State (Phase 5.1)
    // -------------------------------------------------------------------------
    /// Current step in the module creator wizard
    pub module_creator_step: usize,
    /// Module name being entered
    pub module_creator_name: String,
    /// Module description being entered
    pub module_creator_description: String,
    /// Packages being entered (comma-separated raw input)
    pub module_creator_packages: String,
    /// Whether name field is active (vs description/packages)
    pub module_creator_active_field: usize, // 0=name, 1=desc, 2=packages, 3=kind
    /// F-010: Selected ModuleKind index
    pub module_creator_kind_index: usize,
    /// D-012: Dotfile mappings (source, target) being built in the wizard
    pub module_creator_dotfiles: Vec<(String, String)>,
    /// D-012: 0 = editing source, 1 = editing target within current dotfile entry
    pub module_creator_dotfile_field: usize,
    // -------------------------------------------------------------------------
    // System Scan State (Sprint 3 / S1-P1.5-003)
    // -------------------------------------------------------------------------
    /// Latest system scan report (populated by the ScanService)
    pub scan_report: Option<iron_core::services::scan::ScanReport>,
    /// Scroll offset for the scan results view
    pub scan_scroll: usize,
    // -------------------------------------------------------------------------
    // Divergence Detection (Sprint 3 / S1-P3-001)
    // -------------------------------------------------------------------------
    /// Module IDs whose dotfile symlinks are broken or point to unexpected targets
    pub diverged_modules: Vec<String>,
    /// Whether the divergence popup is showing (S1-P3-002)
    pub show_divergence_popup: bool,
    /// Selected index within the divergence popup
    pub divergence_selected: usize,
}

/// Available views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    /// Dashboard home
    Dashboard,
    /// First-time setup wizard
    SetupWizard,
    /// Bundle list
    Bundles,
    /// Bundle detail
    BundleDetail,
    /// Profile list
    Profiles,
    /// Profile detail
    ProfileDetail,
    /// Module list
    Modules,
    /// Module detail
    ModuleDetail,
    /// Update preview
    UpdatePreview,
    /// Sync status
    Sync,
    /// Settings
    Settings,
    /// System maintenance hub (Clean/Update/Doctor)
    SystemMaintenance,
    /// System cleanup with category selection
    CleanSystem,
    /// Cleanup preview (detailed pre-execution view)
    CleanupPreview,
    /// Cleanup execution results
    CleanupResults,
    /// Security module management
    SecurityModules,
    /// Configuration and dotfile management
    ConfigManager,
    /// Operation log viewer (JSONL)
    OperationLog,
    /// System health checks (Doctor)
    Doctor,
    /// Secrets management (git-crypt/age)
    Secrets,
    /// Recovery and backup
    Recovery,
    /// Profile builder wizard ([n] from Profiles)
    ProfileBuilder,
    /// Module creator wizard ([n] from Modules)
    ModuleCreator,
    /// System scan results
    SystemScan,
    /// Host selection for multi-machine setups (S1-P2-001)
    HostSelection,
    /// Integrated Arch installation wizard
    InstallWizard,
}

/// Actions that require confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Switch to a bundle
    SwitchBundle(String),
    /// Remove a bundle
    RemoveBundle(String),
    /// Enable a module
    EnableModule(String),
    /// Disable a module
    DisableModule(String),
    /// Run system update
    RunUpdate,
    /// Run system cleanup (Phase 3)
    RunCleanup,
    /// Push to remote (D-006)
    SyncPush,
    /// Pull from remote (D-006)
    SyncPull,
    /// Quit application
    Quit,
}

/// Confirmation dialog style based on risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmStyle {
    /// Simple Y/N confirmation (Low/Medium risk)
    #[default]
    Simple,
    /// Enhanced warning with prominent risk display (High risk)
    EnhancedWarning,
    /// Requires typing "CONFIRM" to proceed (Critical risk)
    TypedConfirmation,
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
    Warning,
    Error,
}

impl Default for App {
    fn default() -> Self {
        Self::new(
            std::path::PathBuf::from("."),
            Arc::new(NoopPackageManager),
            Arc::new(NoopSystemService),
        )
    }
}

impl App {
    /// Create a new application instance with injected package and service managers
    pub fn new(
        config_dir: PathBuf,
        package_manager: Arc<dyn PackageManager>,
        service_manager: Arc<dyn SystemService>,
    ) -> Self {
        Self {
            view: View::Dashboard,
            previous_view: None,
            should_quit: false,
            config_dir,
            state_manager: None,
            current_host: None,
            discovered_hosts: Vec::new(),
            active_bundle: None,
            active_profile: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            modules: Vec::new(),
            active_modules: Vec::new(),
            selected_index: 0,
            status_message: None,
            error_message: None,
            show_help: false,
            show_confirm: false,
            confirm_action: None,
            confirm_style: ConfirmStyle::default(),
            confirm_typed_input: String::new(),
            wizard: WizardState::new(),
            host_input: TextInput::new(""),
            gpg_key_input: TextInput::new(""),
            import_path_input: TextInput::new(""),
            package_manager,
            service_manager,
            installed_count: 0,
            pending_updates: Vec::new(),
            update_risk: RiskLevel::Low,
            preflight_result: None,
            arch_news: Vec::new(),
            update_section: UpdateSection::default(),
            update_section_index: 0,
            reboot_required: false,
            post_update_result: None,
            // Phase 3: Cleanup state
            cleanup_categories: CleanupCategory::safe().to_vec(),
            cleanup_previews: Vec::new(),
            cleanup_summary: None,
            cleanup_preview_mode: true,
            // Sync state
            sync_info: None,
            sync_conflicts: Vec::new(),
            sync_service: None,
            sync_in_progress: false,
            sync_result_rx: None,
            // Operation log state
            operation_filter: OperationFilter::default(),
            // Progress dialog
            progress: None,
            module_conflicts: Vec::new(),
            secrets_status: None,
            encrypted_files: Vec::new(),
            last_backup: None,
            snapshot_backend: iron_core::snapshot::detect_backend(),
            install_wizard: None,
            profile_builder_step: 0,
            profile_builder_name: String::new(),
            profile_builder_description: String::new(),
            profile_builder_selected_modules: Vec::new(),
            profile_builder_module_cursor: 0,
            profile_builder_editing: false,
            profile_builder_editing_desc: false,
            module_creator_step: 0,
            module_creator_name: String::new(),
            module_creator_description: String::new(),
            module_creator_packages: String::new(),
            module_creator_active_field: 0,
            module_creator_kind_index: 0,
            module_creator_dotfiles: Vec::new(),
            module_creator_dotfile_field: 0,
            scan_report: None,
            scan_scroll: 0,
            diverged_modules: Vec::new(),
            show_divergence_popup: false,
            divergence_selected: 0,
        }
    }

    /// Navigate to a view
    pub fn navigate(&mut self, view: View) {
        self.previous_view = Some(self.view);
        self.view = view;
        self.selected_index = 0;
        self.clear_messages();
        // Pre-load conflict data whenever entering module detail
        if matches!(view, View::ModuleDetail) {
            self.load_module_conflicts();
        }
        // Auto-refresh secrets state when entering Secrets view
        if matches!(view, View::Secrets) {
            self.refresh_secrets();
        }
        // Auto-refresh recovery state when entering Recovery view
        if matches!(view, View::Recovery)
            && let Some(ref sm) = self.state_manager {
                // Populate last_backup from the most recent backup operation in audit log
                let ops = sm.recent_audit(50);
                self.last_backup = ops
                    .iter()
                    .filter(|op| {
                        op.operation == "create_backup" || op.operation == "recovery_export"
                    })
                    .map(|op| op.timestamp)
                    .next();
            }
        // Auto-refresh sync status when entering Sync view (hardening D-005)
        if matches!(view, View::Sync) {
            self.refresh_sync_status();
        }
        // Auto-refresh doctor checks when entering Doctor view (hardening F-004)
        if matches!(view, View::Doctor) {
            self.refresh_current_view();
        }
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_view.take() {
            self.view = prev;
            self.selected_index = 0;
        } else {
            self.view = View::Dashboard;
        }
        self.clear_messages();
    }

    /// Request confirmation for an action
    pub fn request_confirm(&mut self, action: ConfirmAction) {
        // Determine confirmation style based on action and risk level
        self.confirm_style = match &action {
            ConfirmAction::RunUpdate => match self.update_risk {
                RiskLevel::Critical => ConfirmStyle::TypedConfirmation,
                RiskLevel::High => ConfirmStyle::EnhancedWarning,
                _ => ConfirmStyle::Simple,
            },
            // F-002: Enhanced confirm for aggressive cleanup categories
            ConfirmAction::RunCleanup => {
                let has_aggressive = self
                    .cleanup_categories
                    .iter()
                    .any(|c| c.is_aggressive());
                if has_aggressive {
                    ConfirmStyle::EnhancedWarning
                } else {
                    ConfirmStyle::Simple
                }
            }
            _ => ConfirmStyle::Simple,
        };
        self.confirm_typed_input.clear();
        self.confirm_action = Some(action);
        self.show_confirm = true;
    }

    /// Clear status and error messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }

    /// Set error message with automatic expiration
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(StatusMessage::error(message));
    }

    /// Set status message with automatic expiration (success level)
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(StatusMessage::success(message));
    }

    /// Set an info message with automatic expiration
    pub fn set_info(&mut self, message: impl Into<String>) {
        self.status_message = Some(StatusMessage::info(message));
    }

    /// Set a warning message with automatic expiration
    pub fn set_warning(&mut self, message: impl Into<String>) {
        self.status_message = Some(StatusMessage::warning(message));
    }

    /// Get the current status message text (for backward compatibility)
    pub fn status_text(&self) -> Option<&str> {
        self.status_message.as_ref().map(|m| m.text())
    }

    /// Get the current error message text (for backward compatibility)
    pub fn error_text(&self) -> Option<&str> {
        self.error_message.as_ref().map(|m| m.text())
    }

    /// Get the current status message level
    pub fn status_level(&self) -> Option<MessageLevel> {
        self.status_message.as_ref().map(|m| m.level())
    }

    /// Get the current error message level
    pub fn error_level(&self) -> Option<MessageLevel> {
        self.error_message.as_ref().map(|m| m.level())
    }

    /// Get the selected bundle (if in bundle views)
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.bundles.get(self.selected_index)
    }

    /// Get the selected profile (if in profile views)
    pub fn selected_profile(&self) -> Option<&Profile> {
        self.profiles.get(self.selected_index)
    }

    /// Get the selected module (if in module views)
    pub fn selected_module(&self) -> Option<&Module> {
        self.modules.get(self.selected_index)
    }

    /// Check if a module is active
    pub fn is_module_active(&self, module_id: &str) -> bool {
        self.active_modules.contains(&module_id.to_string())
    }

    /// Check if state needs refresh (called on tick)
    ///
    /// Clears expired status and error messages.
    pub fn tick(&mut self) {
        // Clear expired status message
        if let Some(ref msg) = self.status_message
            && msg.is_expired() {
                self.status_message = None;
            }

        // Clear expired error message
        if let Some(ref msg) = self.error_message
            && msg.is_expired() {
                self.error_message = None;
            }

        // D-009: Poll background sync result
        self.poll_sync_result();
    }

    /// Get system health status based on update risk and pending updates
    pub fn system_health(&self) -> HealthStatus {
        match self.update_risk {
            RiskLevel::Critical => HealthStatus::Error,
            RiskLevel::High => HealthStatus::Warning,
            _ if self.pending_updates.len() > 50 => HealthStatus::Warning,
            _ => HealthStatus::Ok,
        }
    }

    /// Get installed package count
    pub fn package_count(&self) -> usize {
        self.installed_count
    }

    /// Get enabled module count
    pub fn enabled_module_count(&self) -> usize {
        self.active_modules.len()
    }

    /// Get pending update count
    pub fn pending_update_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Get update risk level
    pub fn update_risk_level(&self) -> RiskLevel {
        self.update_risk
    }

    /// Get pending updates list
    pub fn pending_updates_list(&self) -> &[PackageUpdate] {
        &self.pending_updates
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    // ==========================================================================
    // Update View Helpers (Phase 2.3)
    // ==========================================================================

    /// Check if pre-flight checks have been run
    pub fn has_preflight_results(&self) -> bool {
        self.preflight_result.is_some()
    }

    /// Get pre-flight check results
    pub fn preflight_checks(&self) -> Option<&PreflightResult> {
        self.preflight_result.as_ref()
    }

    /// Check if system is ready for update (all pre-flight checks pass)
    pub fn can_proceed_with_update(&self) -> bool {
        self.preflight_result
            .as_ref()
            .map(|r| r.can_proceed_with_news())
            .unwrap_or(false)
    }

    /// Get count of unacknowledged news
    pub fn unacknowledged_news_count(&self) -> usize {
        self.preflight_result
            .as_ref()
            .map(|r| r.unacknowledged_news.len())
            .unwrap_or(0)
    }

    /// Get unacknowledged news items
    pub fn unacknowledged_news(&self) -> &[UnacknowledgedNews] {
        self.preflight_result
            .as_ref()
            .map(|r| r.unacknowledged_news.as_slice())
            .unwrap_or(&[])
    }

    /// Check if there's critical news blocking updates
    pub fn has_critical_news(&self) -> bool {
        self.preflight_result
            .as_ref()
            .map(|r| r.news_blocks_update)
            .unwrap_or(false)
    }

    /// Get critical news count
    pub fn critical_news_count(&self) -> usize {
        self.preflight_result
            .as_ref()
            .map(|r| r.critical_news_count())
            .unwrap_or(0)
    }

    /// Navigate to next section in update view
    pub fn next_update_section(&mut self) {
        self.update_section = match self.update_section {
            UpdateSection::PreflightChecks => UpdateSection::News,
            UpdateSection::News => UpdateSection::Packages,
            UpdateSection::Packages => UpdateSection::PreflightChecks,
        };
        self.update_section_index = 0;
    }

    /// Navigate to previous section in update view
    pub fn prev_update_section(&mut self) {
        self.update_section = match self.update_section {
            UpdateSection::PreflightChecks => UpdateSection::Packages,
            UpdateSection::News => UpdateSection::PreflightChecks,
            UpdateSection::Packages => UpdateSection::News,
        };
        self.update_section_index = 0;
    }

    /// Get max items in current update section
    pub fn update_section_max_index(&self) -> usize {
        match self.update_section {
            UpdateSection::PreflightChecks => self
                .preflight_result
                .as_ref()
                .map(|r| r.checks.len())
                .unwrap_or(0),
            UpdateSection::News => self.unacknowledged_news_count(),
            UpdateSection::Packages => self.pending_updates.len().min(50),
        }
    }

    /// Move selection up in update section
    pub fn update_section_up(&mut self) {
        if self.update_section_index > 0 {
            self.update_section_index -= 1;
        }
    }

    /// Move selection down in update section
    pub fn update_section_down(&mut self) {
        let max = self.update_section_max_index();
        if max > 0 && self.update_section_index < max - 1 {
            self.update_section_index += 1;
        }
    }

    /// Acknowledge the currently selected news item
    pub fn acknowledge_selected_news(&mut self) -> Option<String> {
        if self.update_section != UpdateSection::News {
            return None;
        }

        let news = self.unacknowledged_news();
        if self.update_section_index < news.len() {
            let url = news[self.update_section_index].url.clone();

            // Acknowledge via state manager
            if let Some(ref state_manager) = self.state_manager
                && state_manager.acknowledge_news(&url).is_ok() {
                    // Remove from preflight result
                    if let Some(ref mut result) = self.preflight_result {
                        result.unacknowledged_news.retain(|n| n.url != url);
                        // Update news_blocks_update flag
                        result.news_blocks_update =
                            result.unacknowledged_news.iter().any(|n| n.requires_manual);
                    }
                    return Some(url);
                }
        }
        None
    }

    /// Acknowledge all news items
    pub fn acknowledge_all_news(&mut self) -> usize {
        let urls: Vec<String> = self
            .unacknowledged_news()
            .iter()
            .map(|n| n.url.clone())
            .collect();

        if urls.is_empty() {
            return 0;
        }

        let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();

        if let Some(ref state_manager) = self.state_manager
            && state_manager.acknowledge_all_news(&url_refs).is_ok() {
                let count = urls.len();
                if let Some(ref mut result) = self.preflight_result {
                    result.unacknowledged_news.clear();
                    result.news_blocks_update = false;
                }
                return count;
            }
        0
    }

    /// Check if any packages require a reboot (kernel, systemd, glibc)
    pub fn check_reboot_required(&self) -> bool {
        self.pending_updates.iter().any(|p| {
            let name = p.name.to_lowercase();
            name.starts_with("linux")
                || name == "systemd"
                || name.starts_with("systemd-")
                || name == "glibc"
                || name == "nvidia"
                || name == "nvidia-dkms"
        })
    }

    /// Reset update view state (called when entering update view)
    pub fn reset_update_view(&mut self) {
        self.update_section = UpdateSection::PreflightChecks;
        self.update_section_index = 0;
        self.reboot_required = self.check_reboot_required();
    }

    // ==========================================================================
    // Post-Update Detection Helpers (Phase 2.4)
    // ==========================================================================

    /// Check if post-update results are available
    pub fn has_post_update_results(&self) -> bool {
        self.post_update_result.is_some()
    }

    /// Get post-update results
    pub fn post_update_results(&self) -> Option<&PostUpdateResult> {
        self.post_update_result.as_ref()
    }

    /// Check if there are post-update issues requiring attention
    pub fn has_post_update_issues(&self) -> bool {
        self.post_update_result
            .as_ref()
            .map(|r| r.has_issues)
            .unwrap_or(false)
    }

    /// Get count of .pacnew files
    pub fn pacnew_count(&self) -> usize {
        self.post_update_result
            .as_ref()
            .map(|r| r.pacnew_count())
            .unwrap_or(0)
    }

    /// Get count of .pacsave files
    pub fn pacsave_count(&self) -> usize {
        self.post_update_result
            .as_ref()
            .map(|r| r.pacsave_count())
            .unwrap_or(0)
    }

    /// Check if reboot is required (from post-update checks)
    pub fn post_update_reboot_required(&self) -> bool {
        self.post_update_result
            .as_ref()
            .map(|r| r.reboot_required)
            .unwrap_or(false)
    }

    /// Get failed services count
    pub fn failed_services_count(&self) -> usize {
        self.post_update_result
            .as_ref()
            .map(|r| r.failed_services.len())
            .unwrap_or(0)
    }

    /// Clear post-update results
    pub fn clear_post_update_results(&mut self) {
        self.post_update_result = None;
    }

    // ==========================================================================
    // Cleanup Helpers (Phase 3)
    // ==========================================================================

    /// Toggle a cleanup category selection
    pub fn toggle_cleanup_category(&mut self, category: CleanupCategory) {
        if let Some(pos) = self.cleanup_categories.iter().position(|c| *c == category) {
            self.cleanup_categories.remove(pos);
        } else {
            self.cleanup_categories.push(category);
        }
    }

    /// Check if a cleanup category is selected
    pub fn is_cleanup_category_selected(&self, category: &CleanupCategory) -> bool {
        self.cleanup_categories.contains(category)
    }

    /// Get total reclaimable space from previews
    pub fn cleanup_total_space(&self) -> u64 {
        self.cleanup_previews
            .iter()
            .filter(|p| self.cleanup_categories.contains(&p.category))
            .map(|p| p.space_reclaimable)
            .sum()
    }

    /// Get preview for a specific category
    pub fn cleanup_preview_for(&self, category: &CleanupCategory) -> Option<&CleanupPreview> {
        self.cleanup_previews
            .iter()
            .find(|p| &p.category == category)
    }

    /// Check if cleanup has been executed
    pub fn has_cleanup_results(&self) -> bool {
        self.cleanup_summary.is_some()
    }

    /// Clear cleanup state
    pub fn clear_cleanup_state(&mut self) {
        self.cleanup_previews.clear();
        self.cleanup_summary = None;
        self.cleanup_preview_mode = true;
    }

    /// Reset cleanup view state
    pub fn reset_cleanup_view(&mut self) {
        self.selected_index = 0;
        self.cleanup_preview_mode = true;
        // Keep selected categories, but refresh previews
    }

    /// Select all safe cleanup categories
    pub fn select_safe_cleanup_categories(&mut self) {
        self.cleanup_categories = CleanupCategory::safe().to_vec();
    }

    /// Select all cleanup categories (including aggressive)
    pub fn select_all_cleanup_categories(&mut self) {
        self.cleanup_categories = CleanupCategory::all().to_vec();
    }

    /// Deselect all cleanup categories
    pub fn deselect_all_cleanup_categories(&mut self) {
        self.cleanup_categories.clear();
    }

    // ==========================================================================
    // Divergence Detection Helpers (S1-P3-001)
    // ==========================================================================

    /// Check active modules for dotfile symlink divergence.
    ///
    /// A module is considered "diverged" if any of its dotfile targets:
    /// - Is a symlink that points to a non-existent file (broken)
    /// - Is not a symlink when it should be (overwritten by user)
    /// - Does not exist at all (removed)
    pub fn check_divergence(&mut self) {
        let modules_dir = self.config_dir.join("modules");
        let mut diverged = Vec::new();

        for module in &self.modules {
            if !self.active_modules.contains(&module.id) {
                continue;
            }
            let mut module_diverged = false;
            for dotfile in &module.dotfiles {
                let target = iron_core::expand_home(std::path::Path::new(&dotfile.target));
                if dotfile.link {
                    // Expected to be a symlink
                    if target.is_symlink() {
                        // Check if symlink target still exists
                        if let Ok(link_dest) = std::fs::read_link(&target) {
                            if !link_dest.exists() {
                                module_diverged = true;
                                break;
                            }
                            // Optionally verify it points into the modules dir
                            let expected_source =
                                modules_dir.join(&module.id).join(&dotfile.source);
                            if expected_source.exists()
                                && link_dest.canonicalize().ok()
                                    != expected_source.canonicalize().ok()
                            {
                                module_diverged = true;
                                break;
                            }
                        }
                    } else if target.exists() {
                        // Target exists but is NOT a symlink → overwritten
                        module_diverged = true;
                        break;
                    }
                    // If target doesn't exist at all, the dotfile hasn't been applied yet
                    // (not diverged, just not deployed)
                }
            }
            if module_diverged {
                diverged.push(module.id.clone());
            }
        }

        self.diverged_modules = diverged;
    }

    /// Get count of diverged modules
    pub fn diverged_count(&self) -> usize {
        self.diverged_modules.len()
    }

    /// Check if a specific module has diverged
    pub fn is_module_diverged(&self, module_id: &str) -> bool {
        self.diverged_modules.iter().any(|id| id == module_id)
    }
}
