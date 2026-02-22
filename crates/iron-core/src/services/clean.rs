//! Cleanup Service - Comprehensive system cleanup operations
//!
//! Provides 8 cleanup categories for reclaiming disk space:
//! - Package cache management
//! - Orphan package removal
//! - Journal log vacuum
//! - User cache cleanup
//! - Thumbnail cache
//! - Application logs
//! - Browser cache (aggressive)
//! - Developer cache (aggressive)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// ==========================================================================
// Cleanup Categories
// ==========================================================================

/// Cleanup category identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CleanupCategory {
    /// Package cache - old package versions
    PackageCache,
    /// Orphan packages - unused dependencies
    OrphanPackages,
    /// Systemd journal - log vacuum
    SystemdJournal,
    /// User cache - ~/.cache by age
    UserCache,
    /// Thumbnail cache - ~/.cache/thumbnails
    Thumbnails,
    /// Application logs - ~/.local/share logs
    AppLogs,
    /// Browser cache - Firefox/Chrome (aggressive)
    BrowserCache,
    /// Developer cache - npm/yarn/pip/cargo/go (aggressive)
    DevCache,
    /// Broken symlinks in ~/.config (F-006)
    BrokenSymlinks,
}

impl CleanupCategory {
    /// Get all available categories
    pub fn all() -> &'static [CleanupCategory] {
        &[
            CleanupCategory::PackageCache,
            CleanupCategory::OrphanPackages,
            CleanupCategory::SystemdJournal,
            CleanupCategory::UserCache,
            CleanupCategory::Thumbnails,
            CleanupCategory::AppLogs,
            CleanupCategory::BrowserCache,
            CleanupCategory::DevCache,
            CleanupCategory::BrokenSymlinks,
        ]
    }

    /// Get non-aggressive (safe) categories
    pub fn safe() -> &'static [CleanupCategory] {
        &[
            CleanupCategory::PackageCache,
            CleanupCategory::OrphanPackages,
            CleanupCategory::SystemdJournal,
            CleanupCategory::UserCache,
            CleanupCategory::Thumbnails,
            CleanupCategory::AppLogs,
            CleanupCategory::BrokenSymlinks,
        ]
    }

    /// Get aggressive categories that require explicit opt-in
    pub fn aggressive() -> &'static [CleanupCategory] {
        &[CleanupCategory::BrowserCache, CleanupCategory::DevCache]
    }

    /// Check if this category is aggressive (requires explicit opt-in)
    pub fn is_aggressive(&self) -> bool {
        matches!(
            self,
            CleanupCategory::BrowserCache | CleanupCategory::DevCache
        )
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            CleanupCategory::PackageCache => "Package Cache",
            CleanupCategory::OrphanPackages => "Orphan Packages",
            CleanupCategory::SystemdJournal => "Systemd Journal",
            CleanupCategory::UserCache => "User Cache",
            CleanupCategory::Thumbnails => "Thumbnails",
            CleanupCategory::AppLogs => "Application Logs",
            CleanupCategory::BrowserCache => "Browser Cache",
            CleanupCategory::DevCache => "Developer Cache",
            CleanupCategory::BrokenSymlinks => "Broken Symlinks",
        }
    }

    /// Get the description
    pub fn description(&self) -> &'static str {
        match self {
            CleanupCategory::PackageCache => "Old package versions (keeps 3 latest)",
            CleanupCategory::OrphanPackages => "Unused dependency packages",
            CleanupCategory::SystemdJournal => "System logs (vacuum to 100MB)",
            CleanupCategory::UserCache => "Files older than 30 days in ~/.cache",
            CleanupCategory::Thumbnails => "Thumbnail cache in ~/.cache/thumbnails",
            CleanupCategory::AppLogs => "Old logs in ~/.local/share",
            CleanupCategory::BrowserCache => "Firefox and Chrome cache (aggressive)",
            CleanupCategory::DevCache => "npm, yarn, pip, cargo, go cache (aggressive)",
            CleanupCategory::BrokenSymlinks => "Broken symlinks in ~/.config",
        }
    }

    /// Get the short ID for serialization
    pub fn id(&self) -> &'static str {
        match self {
            CleanupCategory::PackageCache => "package_cache",
            CleanupCategory::OrphanPackages => "orphan_packages",
            CleanupCategory::SystemdJournal => "systemd_journal",
            CleanupCategory::UserCache => "user_cache",
            CleanupCategory::Thumbnails => "thumbnails",
            CleanupCategory::AppLogs => "app_logs",
            CleanupCategory::BrowserCache => "browser_cache",
            CleanupCategory::DevCache => "dev_cache",
            CleanupCategory::BrokenSymlinks => "broken_symlinks",
        }
    }
}

// ==========================================================================
// Preview and Result Types
// ==========================================================================

/// Preview information for a cleanup category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPreview {
    /// The cleanup category
    pub category: CleanupCategory,
    /// Number of items that would be cleaned
    pub items_count: usize,
    /// Estimated space that can be reclaimed (in bytes)
    pub space_reclaimable: u64,
    /// Human-readable details (e.g., "12 packages", "150 files")
    pub details: String,
    /// Warnings for this category
    pub warnings: Vec<String>,
}

impl CleanupPreview {
    /// Format the space as human-readable string
    pub fn space_formatted(&self) -> String {
        format_bytes(self.space_reclaimable)
    }
}

/// Result of a cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// The cleanup category
    pub category: CleanupCategory,
    /// Whether the cleanup succeeded
    pub success: bool,
    /// Number of items cleaned
    pub items_cleaned: usize,
    /// Space reclaimed (in bytes)
    pub space_reclaimed: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Detailed output from the operation
    pub output: String,
}

impl CleanupResult {
    /// Create a successful result
    pub fn success(
        category: CleanupCategory,
        items_cleaned: usize,
        space_reclaimed: u64,
        output: String,
    ) -> Self {
        Self {
            category,
            success: true,
            items_cleaned,
            space_reclaimed,
            error: None,
            output,
        }
    }

    /// Create a failed result
    pub fn failure(category: CleanupCategory, error: String) -> Self {
        Self {
            category,
            success: false,
            items_cleaned: 0,
            space_reclaimed: 0,
            error: Some(error),
            output: String::new(),
        }
    }

    /// Format the space as human-readable string
    pub fn space_formatted(&self) -> String {
        format_bytes(self.space_reclaimed)
    }
}

/// Aggregated cleanup summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupSummary {
    /// Individual category results
    pub results: Vec<CleanupResult>,
    /// Total items cleaned
    pub total_items: usize,
    /// Total space reclaimed (in bytes)
    pub total_space: u64,
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
}

impl CleanupSummary {
    /// Add a result to the summary
    pub fn add(&mut self, result: CleanupResult) {
        if result.success {
            self.successful += 1;
            self.total_items += result.items_cleaned;
            self.total_space += result.space_reclaimed;
        } else {
            self.failed += 1;
        }
        self.results.push(result);
    }

    /// Format the total space as human-readable string
    pub fn space_formatted(&self) -> String {
        format_bytes(self.total_space)
    }
}

// ==========================================================================
// Cleanup Service Trait
// ==========================================================================

/// Service for system cleanup operations
pub trait CleanupService: Send + Sync {
    /// Preview what would be cleaned for the given categories
    fn preview(&self, categories: &[CleanupCategory]) -> Vec<CleanupPreview>;

    /// Execute cleanup for the given categories
    fn execute(&self, categories: &[CleanupCategory], dry_run: bool) -> CleanupSummary;

    /// Preview all safe (non-aggressive) categories
    fn preview_safe(&self) -> Vec<CleanupPreview> {
        self.preview(CleanupCategory::safe())
    }

    /// Preview all categories including aggressive ones
    fn preview_all(&self) -> Vec<CleanupPreview> {
        self.preview(CleanupCategory::all())
    }

    /// Execute cleanup for all safe categories
    fn execute_safe(&self, dry_run: bool) -> CleanupSummary {
        self.execute(CleanupCategory::safe(), dry_run)
    }

    /// Get total estimated space for categories
    fn total_space(&self, categories: &[CleanupCategory]) -> u64 {
        self.preview(categories)
            .iter()
            .map(|p| p.space_reclaimable)
            .sum()
    }
}

// ==========================================================================
// Default Implementation
// ==========================================================================

/// Default cleanup service implementation
pub struct DefaultCleanupService {
    /// Home directory path
    home_dir: PathBuf,
    /// Cache max age in days for UserCache
    cache_max_age_days: u64,
    /// Journal max size in MB
    journal_max_size_mb: u64,
    /// Package cache versions to keep
    package_cache_keep: u32,
    /// State manager for audit logging (F-003)
    state_manager: Option<crate::services::StateManager>,
    /// F-005: Injected package manager for orphan/cache operations
    package_manager: Option<Arc<dyn crate::PackageManager>>,
    /// F0-008: Command executor for journalctl/sudo commands (timeout + circuit breaker)
    executor: Option<Arc<dyn crate::resilience::CommandExecutor>>,
}

impl Default for DefaultCleanupService {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultCleanupService {
    /// Create a new cleanup service with default settings
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        Self {
            home_dir,
            cache_max_age_days: 30,
            journal_max_size_mb: 100,
            package_cache_keep: 3,
            state_manager: None,
            package_manager: None,
            executor: None,
        }
    }

    /// Create with custom settings
    pub fn with_settings(
        cache_max_age_days: u64,
        journal_max_size_mb: u64,
        package_cache_keep: u32,
    ) -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        Self {
            home_dir,
            cache_max_age_days,
            journal_max_size_mb,
            package_cache_keep,
            state_manager: None,
            package_manager: None,
            executor: None,
        }
    }

    /// Add state manager for audit logging (F-003)
    pub fn with_state_manager(mut self, sm: crate::services::StateManager) -> Self {
        self.state_manager = Some(sm);
        self
    }

    /// F-005: Inject a PackageManager for orphan/cache operations
    pub fn with_package_manager(mut self, pm: Arc<dyn crate::PackageManager>) -> Self {
        self.package_manager = Some(pm);
        self
    }

    /// F0-008: Inject a CommandExecutor for journalctl/sudo commands
    pub fn with_executor(mut self, executor: Arc<dyn crate::resilience::CommandExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }

    // -----------------------------------------------------------------------
    // Preview Methods
    // -----------------------------------------------------------------------

    fn preview_package_cache(&self) -> CleanupPreview {
        let cache_path = Path::new("/var/cache/pacman/pkg");
        let (count, size) = if cache_path.exists() {
            count_files_and_size(cache_path)
        } else {
            (0, 0)
        };

        // Estimate ~30% can be cleaned (keeping 3 versions)
        let reclaimable = size * 30 / 100;

        CleanupPreview {
            category: CleanupCategory::PackageCache,
            items_count: count,
            space_reclaimable: reclaimable,
            details: format!(
                "{} files, keeps {} versions",
                count, self.package_cache_keep
            ),
            warnings: vec![],
        }
    }

    fn preview_orphan_packages(&self) -> CleanupPreview {
        // F-005: Delegate to injected PackageManager when available
        let orphans = if let Some(ref pm) = self.package_manager {
            pm.get_orphans().unwrap_or_default()
        } else {
            let output = Command::new("pacman").args(["-Qtdq"]).output();
            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    stdout
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|s| s.to_string())
                        .collect()
                }
                Err(_) => Vec::new(),
            }
        };
        let count = orphans.len();

        // Estimate 50MB per orphan package on average
        let reclaimable = count as u64 * 50 * 1024 * 1024;

        CleanupPreview {
            category: CleanupCategory::OrphanPackages,
            items_count: count,
            space_reclaimable: reclaimable,
            details: if count > 0 {
                format!("{} packages", count)
            } else {
                "No orphans found".to_string()
            },
            warnings: if count > 5 {
                vec!["Review packages before removal".to_string()]
            } else {
                vec![]
            },
        }
    }

    fn preview_systemd_journal(&self) -> CleanupPreview {
        // F0-008: Use executor when available, fallback to raw Command
        let size = if let Some(ref executor) = self.executor {
            executor
                .execute("journalctl", &["--disk-usage"])
                .ok()
                .map(|stdout| parse_journal_size(&stdout))
                .unwrap_or(0)
        } else {
            let output = Command::new("journalctl").args(["--disk-usage"]).output();
            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    parse_journal_size(&stdout)
                }
                Err(_) => 0,
            }
        };

        let target_size = self.journal_max_size_mb * 1024 * 1024;
        let reclaimable = size.saturating_sub(target_size);

        CleanupPreview {
            category: CleanupCategory::SystemdJournal,
            items_count: 1,
            space_reclaimable: reclaimable,
            details: format!(
                "Current: {}, Target: {}MB",
                format_bytes(size),
                self.journal_max_size_mb
            ),
            warnings: vec![],
        }
    }

    fn preview_user_cache(&self) -> CleanupPreview {
        let cache_path = self.home_dir.join(".cache");
        let max_age = Duration::from_secs(self.cache_max_age_days * 24 * 60 * 60);

        let (count, size) = if cache_path.exists() {
            count_old_files(&cache_path, max_age)
        } else {
            (0, 0)
        };

        CleanupPreview {
            category: CleanupCategory::UserCache,
            items_count: count,
            space_reclaimable: size,
            details: format!(
                "{} files older than {} days",
                count, self.cache_max_age_days
            ),
            warnings: vec![],
        }
    }

    fn preview_thumbnails(&self) -> CleanupPreview {
        let thumb_path = self.home_dir.join(".cache/thumbnails");
        let (count, size) = if thumb_path.exists() {
            count_files_and_size(&thumb_path)
        } else {
            (0, 0)
        };

        CleanupPreview {
            category: CleanupCategory::Thumbnails,
            items_count: count,
            space_reclaimable: size,
            details: format!("{} thumbnail files", count),
            warnings: vec![],
        }
    }

    fn preview_app_logs(&self) -> CleanupPreview {
        let local_share = self.home_dir.join(".local/share");
        let max_age = Duration::from_secs(30 * 24 * 60 * 60); // 30 days

        let (count, size) = if local_share.exists() {
            count_log_files(&local_share, max_age)
        } else {
            (0, 0)
        };

        CleanupPreview {
            category: CleanupCategory::AppLogs,
            items_count: count,
            space_reclaimable: size,
            details: format!("{} old log files", count),
            warnings: vec![],
        }
    }

    fn preview_browser_cache(&self) -> CleanupPreview {
        let mut total_size = 0u64;
        let mut total_count = 0usize;

        // Firefox cache
        let firefox_cache = self.home_dir.join(".cache/mozilla/firefox");
        if firefox_cache.exists() {
            let (c, s) = count_files_and_size(&firefox_cache);
            total_count += c;
            total_size += s;
        }

        // Chrome/Chromium cache
        for browser in ["google-chrome", "chromium"] {
            let chrome_cache = self.home_dir.join(format!(".cache/{}", browser));
            if chrome_cache.exists() {
                let (c, s) = count_files_and_size(&chrome_cache);
                total_count += c;
                total_size += s;
            }
        }

        CleanupPreview {
            category: CleanupCategory::BrowserCache,
            items_count: total_count,
            space_reclaimable: total_size,
            details: format!("{} browser cache files", total_count),
            warnings: vec![
                "This will clear browser cache".to_string(),
                "You may need to re-login to some websites".to_string(),
            ],
        }
    }

    fn preview_dev_cache(&self) -> CleanupPreview {
        let mut total_size = 0u64;
        let mut total_count = 0usize;
        let mut details = Vec::new();

        // npm cache
        let npm_cache = self.home_dir.join(".npm/_cacache");
        if npm_cache.exists() {
            let (c, s) = count_files_and_size(&npm_cache);
            total_count += c;
            total_size += s;
            if s > 0 {
                details.push(format!("npm: {}", format_bytes(s)));
            }
        }

        // yarn cache
        let yarn_cache = self.home_dir.join(".cache/yarn");
        if yarn_cache.exists() {
            let (c, s) = count_files_and_size(&yarn_cache);
            total_count += c;
            total_size += s;
            if s > 0 {
                details.push(format!("yarn: {}", format_bytes(s)));
            }
        }

        // pip cache
        let pip_cache = self.home_dir.join(".cache/pip");
        if pip_cache.exists() {
            let (c, s) = count_files_and_size(&pip_cache);
            total_count += c;
            total_size += s;
            if s > 0 {
                details.push(format!("pip: {}", format_bytes(s)));
            }
        }

        // cargo cache
        let cargo_cache = self.home_dir.join(".cargo/registry/cache");
        if cargo_cache.exists() {
            let (c, s) = count_files_and_size(&cargo_cache);
            total_count += c;
            total_size += s;
            if s > 0 {
                details.push(format!("cargo: {}", format_bytes(s)));
            }
        }

        // go cache
        let go_cache = self.home_dir.join("go/pkg/mod/cache");
        if go_cache.exists() {
            let (c, s) = count_files_and_size(&go_cache);
            total_count += c;
            total_size += s;
            if s > 0 {
                details.push(format!("go: {}", format_bytes(s)));
            }
        }

        CleanupPreview {
            category: CleanupCategory::DevCache,
            items_count: total_count,
            space_reclaimable: total_size,
            details: if details.is_empty() {
                "No dev caches found".to_string()
            } else {
                details.join(", ")
            },
            warnings: vec![
                "This will clear developer tool caches".to_string(),
                "Next build may take longer".to_string(),
            ],
        }
    }

    // F-006: Broken symlinks in ~/.config
    fn preview_broken_symlinks(&self) -> CleanupPreview {
        let config_dir = self.home_dir.join(".config");
        let broken = if config_dir.exists() {
            find_broken_symlinks(&config_dir)
        } else {
            vec![]
        };

        let count = broken.len();
        CleanupPreview {
            category: CleanupCategory::BrokenSymlinks,
            items_count: count,
            space_reclaimable: 0, // symlinks use negligible space
            details: if count == 0 {
                "No broken symlinks found".to_string()
            } else {
                format!("{} broken symlinks in ~/.config", count)
            },
            warnings: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Execute Methods
    // -----------------------------------------------------------------------

    fn execute_package_cache(&self, dry_run: bool) -> CleanupResult {
        if dry_run {
            let preview = self.preview_package_cache();
            return CleanupResult::success(
                CleanupCategory::PackageCache,
                preview.items_count,
                preview.space_reclaimable,
                "[DRY RUN] Would run: paccache -rk3".to_string(),
            );
        }

        // F-005: Delegate to injected PackageManager when available
        if let Some(ref pm) = self.package_manager {
            match pm.clean_cache(self.package_cache_keep) {
                Ok(result) => CleanupResult::success(
                    CleanupCategory::PackageCache,
                    result.removed_count,
                    0,
                    result.output,
                ),
                Err(e) => CleanupResult::failure(
                    CleanupCategory::PackageCache,
                    format!("Package cache clean failed: {}", e),
                ),
            }
        } else {
            let output = Command::new("sudo")
                .args(["paccache", "-r", &format!("-k{}", self.package_cache_keep)])
                .output();

            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

                    if result.status.success() {
                        let removed =
                            stdout.lines().filter(|l| l.contains("removing")).count();
                        CleanupResult::success(
                            CleanupCategory::PackageCache,
                            removed,
                            0,
                            stdout,
                        )
                    } else {
                        CleanupResult::failure(
                            CleanupCategory::PackageCache,
                            format!("paccache failed: {}", stderr),
                        )
                    }
                }
                Err(e) => CleanupResult::failure(
                    CleanupCategory::PackageCache,
                    format!("Failed to run paccache: {}", e),
                ),
            }
        }
    }

    fn execute_orphan_packages(&self, dry_run: bool) -> CleanupResult {
        // F-005: Delegate orphan listing to injected PackageManager when available
        let orphans = if let Some(ref pm) = self.package_manager {
            pm.get_orphans().unwrap_or_default()
        } else {
            let orphan_output = Command::new("pacman").args(["-Qtdq"]).output();
            match orphan_output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    stdout
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                }
                Err(_) => vec![],
            }
        };

        if orphans.is_empty() {
            return CleanupResult::success(
                CleanupCategory::OrphanPackages,
                0,
                0,
                "No orphan packages found".to_string(),
            );
        }

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::OrphanPackages,
                orphans.len(),
                orphans.len() as u64 * 50 * 1024 * 1024, // Estimate
                format!(
                    "[DRY RUN] Would remove {} packages: {}",
                    orphans.len(),
                    orphans.join(", ")
                ),
            );
        }

        // F-005: Delegate removal to injected PackageManager when available
        if let Some(ref pm) = self.package_manager {
            match pm.remove(&orphans, true) {
                Ok(()) => CleanupResult::success(
                    CleanupCategory::OrphanPackages,
                    orphans.len(),
                    orphans.len() as u64 * 50 * 1024 * 1024,
                    format!("Removed {} orphan packages", orphans.len()),
                ),
                Err(e) => CleanupResult::failure(
                    CleanupCategory::OrphanPackages,
                    format!("Orphan removal failed: {}", e),
                ),
            }
        } else {
            // Fallback: raw Command
            let output = Command::new("sudo")
                .args(["pacman", "-Rns", "--noconfirm"])
                .args(&orphans)
                .output();

            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

                    if result.status.success() {
                        CleanupResult::success(
                            CleanupCategory::OrphanPackages,
                            orphans.len(),
                            orphans.len() as u64 * 50 * 1024 * 1024,
                            stdout,
                        )
                    } else {
                        CleanupResult::failure(
                            CleanupCategory::OrphanPackages,
                            format!("pacman removal failed: {}", stderr),
                        )
                    }
                }
                Err(e) => CleanupResult::failure(
                    CleanupCategory::OrphanPackages,
                    format!("Failed to remove orphans: {}", e),
                ),
            }
        }
    }

    fn execute_systemd_journal(&self, dry_run: bool) -> CleanupResult {
        if dry_run {
            let preview = self.preview_systemd_journal();
            return CleanupResult::success(
                CleanupCategory::SystemdJournal,
                1,
                preview.space_reclaimable,
                format!(
                    "[DRY RUN] Would run: journalctl --vacuum-size={}M",
                    self.journal_max_size_mb
                ),
            );
        }

        // F0-008: Use executor when available, fallback to raw Command
        if let Some(ref executor) = self.executor {
            let vacuum_arg = format!("--vacuum-size={}M", self.journal_max_size_mb);
            match executor.execute_full("sudo", &["journalctl", &vacuum_arg]) {
                Ok(output) => {
                    if output.success() {
                        let freed = parse_journal_freed(&output.stdout);
                        CleanupResult::success(
                            CleanupCategory::SystemdJournal,
                            1,
                            freed,
                            output.stdout,
                        )
                    } else {
                        CleanupResult::failure(
                            CleanupCategory::SystemdJournal,
                            format!("journalctl vacuum failed: {}", output.stderr),
                        )
                    }
                }
                Err(e) => CleanupResult::failure(
                    CleanupCategory::SystemdJournal,
                    format!("Failed to vacuum journal: {}", e),
                ),
            }
        } else {
            let output = Command::new("sudo")
                .args([
                    "journalctl",
                    &format!("--vacuum-size={}M", self.journal_max_size_mb),
                ])
                .output();

            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

                    if result.status.success() {
                        // Parse freed space from output
                        let freed = parse_journal_freed(&stdout);
                        CleanupResult::success(CleanupCategory::SystemdJournal, 1, freed, stdout)
                    } else {
                        CleanupResult::failure(
                            CleanupCategory::SystemdJournal,
                            format!("journalctl vacuum failed: {}", stderr),
                        )
                    }
                }
                Err(e) => CleanupResult::failure(
                    CleanupCategory::SystemdJournal,
                    format!("Failed to vacuum journal: {}", e),
                ),
            }
        }
    }

    fn execute_user_cache(&self, dry_run: bool) -> CleanupResult {
        let cache_path = self.home_dir.join(".cache");
        let max_age = Duration::from_secs(self.cache_max_age_days * 24 * 60 * 60);

        if !cache_path.exists() {
            return CleanupResult::success(
                CleanupCategory::UserCache,
                0,
                0,
                "User cache directory not found".to_string(),
            );
        }

        let (count, size) = count_old_files(&cache_path, max_age);

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::UserCache,
                count,
                size,
                format!(
                    "[DRY RUN] Would remove {} files ({}) older than {} days",
                    count,
                    format_bytes(size),
                    self.cache_max_age_days
                ),
            );
        }

        let removed = remove_old_files(&cache_path, max_age);
        CleanupResult::success(
            CleanupCategory::UserCache,
            removed,
            size,
            format!("Removed {} old cache files", removed),
        )
    }

    fn execute_thumbnails(&self, dry_run: bool) -> CleanupResult {
        let thumb_path = self.home_dir.join(".cache/thumbnails");

        if !thumb_path.exists() {
            return CleanupResult::success(
                CleanupCategory::Thumbnails,
                0,
                0,
                "Thumbnail directory not found".to_string(),
            );
        }

        let (count, size) = count_files_and_size(&thumb_path);

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::Thumbnails,
                count,
                size,
                format!(
                    "[DRY RUN] Would remove {} thumbnail files ({})",
                    count,
                    format_bytes(size)
                ),
            );
        }

        // Remove all files in thumbnail directory
        let removed = remove_directory_contents(&thumb_path);
        CleanupResult::success(
            CleanupCategory::Thumbnails,
            removed,
            size,
            format!("Removed {} thumbnail files", removed),
        )
    }

    fn execute_app_logs(&self, dry_run: bool) -> CleanupResult {
        let local_share = self.home_dir.join(".local/share");
        let max_age = Duration::from_secs(30 * 24 * 60 * 60);

        if !local_share.exists() {
            return CleanupResult::success(
                CleanupCategory::AppLogs,
                0,
                0,
                "Local share directory not found".to_string(),
            );
        }

        let (count, size) = count_log_files(&local_share, max_age);

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::AppLogs,
                count,
                size,
                format!(
                    "[DRY RUN] Would remove {} old log files ({})",
                    count,
                    format_bytes(size)
                ),
            );
        }

        let removed = remove_log_files(&local_share, max_age);
        CleanupResult::success(
            CleanupCategory::AppLogs,
            removed,
            size,
            format!("Removed {} old log files", removed),
        )
    }

    fn execute_browser_cache(&self, dry_run: bool) -> CleanupResult {
        let preview = self.preview_browser_cache();

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::BrowserCache,
                preview.items_count,
                preview.space_reclaimable,
                format!(
                    "[DRY RUN] Would remove {} browser cache files ({})",
                    preview.items_count,
                    preview.space_formatted()
                ),
            );
        }

        let mut removed = 0;
        let mut freed = 0u64;

        // Firefox cache
        let firefox_cache = self.home_dir.join(".cache/mozilla/firefox");
        if firefox_cache.exists() {
            let (_, size) = count_files_and_size(&firefox_cache);
            removed += remove_directory_contents(&firefox_cache);
            freed += size;
        }

        // Chrome/Chromium cache
        for browser in ["google-chrome", "chromium"] {
            let chrome_cache = self.home_dir.join(format!(".cache/{}", browser));
            if chrome_cache.exists() {
                let (_, size) = count_files_and_size(&chrome_cache);
                removed += remove_directory_contents(&chrome_cache);
                freed += size;
            }
        }

        CleanupResult::success(
            CleanupCategory::BrowserCache,
            removed,
            freed,
            format!("Removed {} browser cache files", removed),
        )
    }

    fn execute_dev_cache(&self, dry_run: bool) -> CleanupResult {
        let preview = self.preview_dev_cache();

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::DevCache,
                preview.items_count,
                preview.space_reclaimable,
                format!(
                    "[DRY RUN] Would remove {} dev cache files ({})\n{}",
                    preview.items_count,
                    preview.space_formatted(),
                    preview.details
                ),
            );
        }

        let mut removed = 0;
        let mut freed = 0u64;
        let mut details = Vec::new();

        // npm cache - use npm cache clean
        let npm_cache = self.home_dir.join(".npm/_cacache");
        if npm_cache.exists() {
            let (_, size) = count_files_and_size(&npm_cache);
            let r = remove_directory_contents(&npm_cache);
            removed += r;
            freed += size;
            details.push(format!("npm: {} files", r));
        }

        // yarn cache
        let yarn_cache = self.home_dir.join(".cache/yarn");
        if yarn_cache.exists() {
            let (_, size) = count_files_and_size(&yarn_cache);
            let r = remove_directory_contents(&yarn_cache);
            removed += r;
            freed += size;
            details.push(format!("yarn: {} files", r));
        }

        // pip cache
        let pip_cache = self.home_dir.join(".cache/pip");
        if pip_cache.exists() {
            let (_, size) = count_files_and_size(&pip_cache);
            let r = remove_directory_contents(&pip_cache);
            removed += r;
            freed += size;
            details.push(format!("pip: {} files", r));
        }

        // cargo cache (only registry cache, not sources)
        let cargo_cache = self.home_dir.join(".cargo/registry/cache");
        if cargo_cache.exists() {
            let (_, size) = count_files_and_size(&cargo_cache);
            let r = remove_directory_contents(&cargo_cache);
            removed += r;
            freed += size;
            details.push(format!("cargo: {} files", r));
        }

        // go mod cache
        let go_cache = self.home_dir.join("go/pkg/mod/cache");
        if go_cache.exists() {
            let (_, size) = count_files_and_size(&go_cache);
            let r = remove_directory_contents(&go_cache);
            removed += r;
            freed += size;
            details.push(format!("go: {} files", r));
        }

        CleanupResult::success(
            CleanupCategory::DevCache,
            removed,
            freed,
            format!("Cleaned: {}", details.join(", ")),
        )
    }

    // F-006: Remove broken symlinks in ~/.config
    fn execute_broken_symlinks(&self, dry_run: bool) -> CleanupResult {
        let config_dir = self.home_dir.join(".config");
        if !config_dir.exists() {
            return CleanupResult::success(
                CleanupCategory::BrokenSymlinks,
                0,
                0,
                "~/.config directory not found".to_string(),
            );
        }

        let broken = find_broken_symlinks(&config_dir);
        let count = broken.len();

        if dry_run {
            return CleanupResult::success(
                CleanupCategory::BrokenSymlinks,
                count,
                0,
                if count == 0 {
                    "[DRY RUN] No broken symlinks found".to_string()
                } else {
                    format!(
                        "[DRY RUN] Would remove {} broken symlinks:\n{}",
                        count,
                        broken
                            .iter()
                            .map(|p| format!("  {}", p.display()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                },
            );
        }

        let mut removed = 0;
        for path in &broken {
            if fs::remove_file(path).is_ok() {
                removed += 1;
            }
        }

        CleanupResult::success(
            CleanupCategory::BrokenSymlinks,
            removed,
            0,
            format!("Removed {} broken symlinks", removed),
        )
    }
}

impl CleanupService for DefaultCleanupService {
    fn preview(&self, categories: &[CleanupCategory]) -> Vec<CleanupPreview> {
        categories
            .iter()
            .map(|cat| match cat {
                CleanupCategory::PackageCache => self.preview_package_cache(),
                CleanupCategory::OrphanPackages => self.preview_orphan_packages(),
                CleanupCategory::SystemdJournal => self.preview_systemd_journal(),
                CleanupCategory::UserCache => self.preview_user_cache(),
                CleanupCategory::Thumbnails => self.preview_thumbnails(),
                CleanupCategory::AppLogs => self.preview_app_logs(),
                CleanupCategory::BrowserCache => self.preview_browser_cache(),
                CleanupCategory::DevCache => self.preview_dev_cache(),
                CleanupCategory::BrokenSymlinks => self.preview_broken_symlinks(),
            })
            .collect()
    }

    fn execute(&self, categories: &[CleanupCategory], dry_run: bool) -> CleanupSummary {
        let mut summary = CleanupSummary::default();

        for cat in categories {
            let result = match cat {
                CleanupCategory::PackageCache => self.execute_package_cache(dry_run),
                CleanupCategory::OrphanPackages => self.execute_orphan_packages(dry_run),
                CleanupCategory::SystemdJournal => self.execute_systemd_journal(dry_run),
                CleanupCategory::UserCache => self.execute_user_cache(dry_run),
                CleanupCategory::Thumbnails => self.execute_thumbnails(dry_run),
                CleanupCategory::AppLogs => self.execute_app_logs(dry_run),
                CleanupCategory::BrowserCache => self.execute_browser_cache(dry_run),
                CleanupCategory::DevCache => self.execute_dev_cache(dry_run),
                CleanupCategory::BrokenSymlinks => self.execute_broken_symlinks(dry_run),
            };
            summary.add(result);
        }

        // F-003: Record cleanup operation in audit log
        if !dry_run
            && let Some(ref sm) = self.state_manager
        {
            let cat_names: Vec<&str> = categories.iter().map(|c| c.name()).collect();
            let details = format!(
                "categories: [{}], reclaimed: {}",
                cat_names.join(", "),
                summary.space_formatted()
            );
            let _ = sm.record_operation(
                "cleanup",
                crate::OperationStatus::Success,
                Some(details),
            );
        }

        summary
    }
}

// ==========================================================================
// Helper Functions
// ==========================================================================

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// F-006: Recursively find broken symlinks in a directory
fn find_broken_symlinks(dir: &Path) -> Vec<PathBuf> {
    let mut broken = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return broken,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Check if it's a symlink (use symlink_metadata to avoid following the link)
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() {
                // Broken if the target doesn't exist
                if !path.exists() {
                    broken.push(path);
                }
            } else if meta.is_dir() {
                broken.extend(find_broken_symlinks(&path));
            }
        }
    }
    broken
}

/// Count files and total size in a directory
fn count_files_and_size(path: &Path) -> (usize, u64) {
    let mut count = 0usize;
    let mut size = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let (c, s) = count_files_and_size(&entry_path);
                count += c;
                size += s;
            } else if let Ok(metadata) = entry.metadata() {
                count += 1;
                size += metadata.len();
            }
        }
    }

    (count, size)
}

/// Count files older than max_age and their total size
fn count_old_files(path: &Path, max_age: Duration) -> (usize, u64) {
    let now = SystemTime::now();
    let mut count = 0usize;
    let mut size = 0u64;

    fn walk(path: &Path, now: SystemTime, max_age: Duration, count: &mut usize, size: &mut u64) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    walk(&entry_path, now, max_age, count, size);
                } else if let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                        && let Ok(age) = now.duration_since(modified)
                            && age > max_age {
                                *count += 1;
                                *size += metadata.len();
                            }
            }
        }
    }

    walk(path, now, max_age, &mut count, &mut size);
    (count, size)
}

/// Count log files older than max_age
fn count_log_files(path: &Path, max_age: Duration) -> (usize, u64) {
    let now = SystemTime::now();
    let mut count = 0usize;
    let mut size = 0u64;

    fn walk(path: &Path, now: SystemTime, max_age: Duration, count: &mut usize, size: &mut u64) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    walk(&entry_path, now, max_age, count, size);
                } else if let Ok(metadata) = entry.metadata() {
                    let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
                    // Only count log files
                    if (name.ends_with(".log")
                        || name.ends_with(".log.old")
                        || name.contains(".log."))
                        && let Ok(modified) = metadata.modified()
                            && let Ok(age) = now.duration_since(modified)
                                && age > max_age {
                                    *count += 1;
                                    *size += metadata.len();
                                }
                }
            }
        }
    }

    walk(path, now, max_age, &mut count, &mut size);
    (count, size)
}

/// Remove files older than max_age, returns count of removed files
fn remove_old_files(path: &Path, max_age: Duration) -> usize {
    let now = SystemTime::now();
    let mut removed = 0usize;

    fn walk(path: &Path, now: SystemTime, max_age: Duration, removed: &mut usize) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    walk(&entry_path, now, max_age, removed);
                } else if let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                        && let Ok(age) = now.duration_since(modified)
                            && age > max_age
                                && fs::remove_file(&entry_path).is_ok() {
                                    *removed += 1;
                                }
            }
        }
    }

    walk(path, now, max_age, &mut removed);
    removed
}

/// Remove log files older than max_age
fn remove_log_files(path: &Path, max_age: Duration) -> usize {
    let now = SystemTime::now();
    let mut removed = 0usize;

    fn walk(path: &Path, now: SystemTime, max_age: Duration, removed: &mut usize) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    walk(&entry_path, now, max_age, removed);
                } else if let Ok(metadata) = entry.metadata() {
                    let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
                    if (name.ends_with(".log")
                        || name.ends_with(".log.old")
                        || name.contains(".log."))
                        && let Ok(modified) = metadata.modified()
                            && let Ok(age) = now.duration_since(modified)
                                && age > max_age
                                    && fs::remove_file(&entry_path).is_ok() {
                                        *removed += 1;
                                    }
                }
            }
        }
    }

    walk(path, now, max_age, &mut removed);
    removed
}

/// Remove all contents of a directory (but not the directory itself)
fn remove_directory_contents(path: &Path) -> usize {
    let mut removed = 0usize;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if fs::remove_dir_all(&entry_path).is_ok() {
                    removed += 1;
                }
            } else if fs::remove_file(&entry_path).is_ok() {
                removed += 1;
            }
        }
    }

    removed
}

/// Parse journal disk usage from journalctl --disk-usage output
fn parse_journal_size(output: &str) -> u64 {
    // Output format: "Archived and active journals take up 234.5M in the file system."
    for line in output.lines() {
        if line.contains("take up") || line.contains("takes up") {
            // Find the size value
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "up" && i + 1 < parts.len() {
                    let size_str = parts[i + 1];
                    return parse_size_string(size_str);
                }
            }
        }
    }
    0
}

/// Parse journal freed space from vacuum output
fn parse_journal_freed(output: &str) -> u64 {
    // Output format: "Vacuuming done, freed 123.4M of archived journals"
    for line in output.lines() {
        if line.contains("freed") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "freed" && i + 1 < parts.len() {
                    let size_str = parts[i + 1];
                    return parse_size_string(size_str);
                }
            }
        }
    }
    0
}

/// Parse size string like "234.5M" or "1.2G" into bytes
fn parse_size_string(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }

    let last_char = s.chars().last().unwrap_or('B');
    let num_str = &s[..s.len() - 1];

    let multiplier = match last_char {
        'B' | 'b' => 1u64,
        'K' | 'k' => 1024,
        'M' | 'm' => 1024 * 1024,
        'G' | 'g' => 1024 * 1024 * 1024,
        'T' | 't' => 1024 * 1024 * 1024 * 1024,
        _ => {
            // No unit suffix, try parsing as bytes
            return s.parse().unwrap_or(0);
        }
    };

    num_str
        .parse::<f64>()
        .map(|n| (n * multiplier as f64) as u64)
        .unwrap_or(0)
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_category_all() {
        let all = CleanupCategory::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn test_cleanup_category_safe() {
        let safe = CleanupCategory::safe();
        assert_eq!(safe.len(), 7);
        for cat in safe {
            assert!(!cat.is_aggressive());
        }
    }

    #[test]
    fn test_cleanup_category_aggressive() {
        let aggressive = CleanupCategory::aggressive();
        assert_eq!(aggressive.len(), 2);
        for cat in aggressive {
            assert!(cat.is_aggressive());
        }
    }

    #[test]
    fn test_cleanup_category_metadata() {
        let cat = CleanupCategory::PackageCache;
        assert_eq!(cat.name(), "Package Cache");
        assert_eq!(cat.id(), "package_cache");
        assert!(!cat.description().is_empty());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_parse_size_string() {
        assert_eq!(parse_size_string("100B"), 100);
        assert_eq!(parse_size_string("1K"), 1024);
        assert_eq!(parse_size_string("1M"), 1024 * 1024);
        assert_eq!(parse_size_string("1G"), 1024 * 1024 * 1024);
        assert_eq!(parse_size_string("1.5M"), (1.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(
            parse_size_string("234.5M"),
            (234.5 * 1024.0 * 1024.0) as u64
        );
    }

    #[test]
    fn test_cleanup_result_success() {
        let result = CleanupResult::success(
            CleanupCategory::Thumbnails,
            100,
            1024 * 1024,
            "Cleaned".to_string(),
        );
        assert!(result.success);
        assert_eq!(result.items_cleaned, 100);
        assert_eq!(result.space_formatted(), "1.0 MB");
    }

    #[test]
    fn test_cleanup_result_failure() {
        let result = CleanupResult::failure(
            CleanupCategory::PackageCache,
            "Permission denied".to_string(),
        );
        assert!(!result.success);
        assert_eq!(result.error, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_cleanup_summary() {
        let mut summary = CleanupSummary::default();

        summary.add(CleanupResult::success(
            CleanupCategory::Thumbnails,
            50,
            1024 * 1024,
            "OK".to_string(),
        ));
        summary.add(CleanupResult::success(
            CleanupCategory::UserCache,
            100,
            2 * 1024 * 1024,
            "OK".to_string(),
        ));
        summary.add(CleanupResult::failure(
            CleanupCategory::PackageCache,
            "Failed".to_string(),
        ));

        assert_eq!(summary.successful, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.total_items, 150);
        assert_eq!(summary.total_space, 3 * 1024 * 1024);
    }

    #[test]
    fn test_cleanup_preview_space_formatted() {
        let preview = CleanupPreview {
            category: CleanupCategory::UserCache,
            items_count: 100,
            space_reclaimable: 500 * 1024 * 1024,
            details: "100 files".to_string(),
            warnings: vec![],
        };
        assert_eq!(preview.space_formatted(), "500.0 MB");
    }

    #[test]
    fn test_default_cleanup_service_creation() {
        let service = DefaultCleanupService::new();
        assert!(service.home_dir.exists() || service.home_dir == PathBuf::from("/home"));
    }

    #[test]
    fn test_cleanup_service_with_settings() {
        let service = DefaultCleanupService::with_settings(60, 200, 5);
        assert_eq!(service.cache_max_age_days, 60);
        assert_eq!(service.journal_max_size_mb, 200);
        assert_eq!(service.package_cache_keep, 5);
    }

    #[test]
    fn test_preview_empty_categories() {
        let service = DefaultCleanupService::new();
        let previews = service.preview(&[]);
        assert!(previews.is_empty());
    }

    #[test]
    fn test_execute_dry_run_empty_categories() {
        let service = DefaultCleanupService::new();
        let summary = service.execute(&[], true);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert!(summary.results.is_empty());
    }

    #[test]
    fn test_cleanup_category_id_roundtrip() {
        for cat in CleanupCategory::all() {
            let id = cat.id();
            assert!(!id.is_empty());
            assert!(id.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
    }

    #[test]
    fn test_cleanup_category_name_not_empty() {
        for cat in CleanupCategory::all() {
            assert!(!cat.name().is_empty());
        }
    }

    #[test]
    fn test_cleanup_category_description_not_empty() {
        for cat in CleanupCategory::all() {
            assert!(!cat.description().is_empty());
        }
    }

    #[test]
    fn test_parse_journal_size_formats() {
        // Test various output formats
        assert_eq!(
            parse_journal_size("Archived and active journals take up 234.5M in the file system."),
            (234.5 * 1024.0 * 1024.0) as u64
        );
        assert_eq!(
            parse_journal_size("Journals take up 1.2G on disk."),
            (1.2 * 1024.0 * 1024.0 * 1024.0) as u64
        );
        assert_eq!(parse_journal_size("No match here"), 0);
    }

    #[test]
    fn test_parse_journal_freed_formats() {
        assert_eq!(
            parse_journal_freed("Vacuuming done, freed 100M of archived journals"),
            (100.0 * 1024.0 * 1024.0) as u64
        );
        assert_eq!(parse_journal_freed("Deleted 0B, no files were freed"), 0);
    }

    #[test]
    fn test_format_bytes_large_values() {
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1025), "1.0 KB");
        // Test terabyte range
        let tb = 1024u64 * 1024 * 1024 * 1024;
        assert!(format_bytes(tb).contains("TB") || format_bytes(tb).contains("GB"));
    }

    #[test]
    fn test_cleanup_preview_has_category() {
        let preview = CleanupPreview {
            category: CleanupCategory::Thumbnails,
            items_count: 100,
            space_reclaimable: 1024 * 1024,
            details: "100 files".to_string(),
            warnings: vec![],
        };
        assert_eq!(preview.category, CleanupCategory::Thumbnails);
        assert_eq!(preview.items_count, 100);
    }

    #[test]
    fn test_cleanup_result_preserves_category() {
        let result = CleanupResult::success(
            CleanupCategory::UserCache,
            50,
            512 * 1024,
            "Cleaned 50 files".to_string(),
        );
        assert_eq!(result.category, CleanupCategory::UserCache);
        assert!(result.success);
    }

    // -----------------------------------------------------------------------
    // E-011: Mock filesystem tests for CleanupService
    // -----------------------------------------------------------------------

    fn service_with_temp(home: &Path) -> DefaultCleanupService {
        DefaultCleanupService {
            home_dir: home.to_path_buf(),
            cache_max_age_days: 30,
            journal_max_size_mb: 100,
            package_cache_keep: 3,
            state_manager: None,
            package_manager: None,
            executor: None,
        }
    }

    #[test]
    fn test_preview_thumbnails_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        let thumb_dir = tmp.path().join(".cache/thumbnails/normal");
        fs::create_dir_all(&thumb_dir).unwrap();
        fs::write(thumb_dir.join("abc.png"), "data1234").unwrap();
        fs::write(thumb_dir.join("def.png"), "moredata").unwrap();

        let svc = service_with_temp(tmp.path());
        let preview = svc.preview_thumbnails();
        assert_eq!(preview.category, CleanupCategory::Thumbnails);
        assert_eq!(preview.items_count, 2);
        assert!(preview.space_reclaimable > 0);
    }

    #[test]
    fn test_preview_thumbnails_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = service_with_temp(tmp.path());
        let preview = svc.preview_thumbnails();
        assert_eq!(preview.items_count, 0);
        assert_eq!(preview.space_reclaimable, 0);
    }

    #[test]
    fn test_execute_thumbnails_removes_files() {
        let tmp = tempfile::tempdir().unwrap();
        let thumb_dir = tmp.path().join(".cache/thumbnails");
        fs::create_dir_all(&thumb_dir).unwrap();
        fs::write(thumb_dir.join("a.png"), "x").unwrap();
        fs::write(thumb_dir.join("b.png"), "y").unwrap();

        let svc = service_with_temp(tmp.path());
        let result = svc.execute_thumbnails(false);
        assert!(result.success);
        assert_eq!(result.items_cleaned, 2);
    }

    #[test]
    fn test_execute_thumbnails_dry_run_keeps_files() {
        let tmp = tempfile::tempdir().unwrap();
        let thumb_dir = tmp.path().join(".cache/thumbnails");
        fs::create_dir_all(&thumb_dir).unwrap();
        fs::write(thumb_dir.join("a.png"), "x").unwrap();

        let svc = service_with_temp(tmp.path());
        let result = svc.execute_thumbnails(true);
        assert!(result.success);
        assert!(result.output.contains("[DRY RUN]"));
        // File should still exist
        assert!(thumb_dir.join("a.png").exists());
    }

    #[test]
    fn test_preview_app_logs_old_files() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join(".local/share/myapp");
        fs::create_dir_all(&log_dir).unwrap();
        // Create a log file — it will be recent, so may not be picked up by age filter
        fs::write(log_dir.join("app.log"), "log content").unwrap();

        let svc = service_with_temp(tmp.path());
        let preview = svc.preview_app_logs();
        assert_eq!(preview.category, CleanupCategory::AppLogs);
        // Recent files won't match the 30-day age filter, so count may be 0
    }

    #[test]
    fn test_preview_user_cache_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join(".cache/some-app");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("data.bin"), "cached").unwrap();

        let svc = service_with_temp(tmp.path());
        let preview = svc.preview_user_cache();
        assert_eq!(preview.category, CleanupCategory::UserCache);
        // Recent files won't match age filter
    }

    #[test]
    fn test_preview_broken_symlinks_finds_dangling() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".config/myapp");
        fs::create_dir_all(&config_dir).unwrap();
        // Create a broken symlink
        std::os::unix::fs::symlink("/nonexistent/path/xyz", config_dir.join("broken.conf"))
            .unwrap();
        // Create a valid file (not a broken symlink)
        fs::write(config_dir.join("valid.conf"), "ok").unwrap();

        let svc = service_with_temp(tmp.path());
        let preview = svc.preview_broken_symlinks();
        assert_eq!(preview.category, CleanupCategory::BrokenSymlinks);
        assert_eq!(preview.items_count, 1);
        assert!(preview.details.contains("1 broken symlink"));
    }

    #[test]
    fn test_execute_broken_symlinks_removes_dangling() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".config/testapp");
        fs::create_dir_all(&config_dir).unwrap();
        let broken_path = config_dir.join("broken.link");
        std::os::unix::fs::symlink("/no/such/target", &broken_path).unwrap();

        let svc = service_with_temp(tmp.path());
        let result = svc.execute_broken_symlinks(false);
        assert!(result.success);
        assert_eq!(result.items_cleaned, 1);
        // Symlink should be gone
        assert!(!broken_path.exists());
        // Also symlink_metadata should fail
        assert!(fs::symlink_metadata(&broken_path).is_err());
    }

    #[test]
    fn test_execute_broken_symlinks_dry_run_preserves() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".config");
        fs::create_dir_all(&config_dir).unwrap();
        let broken = config_dir.join("dangling");
        std::os::unix::fs::symlink("/does/not/exist", &broken).unwrap();

        let svc = service_with_temp(tmp.path());
        let result = svc.execute_broken_symlinks(true);
        assert!(result.success);
        assert!(result.output.contains("[DRY RUN]"));
        // Symlink should still exist
        assert!(fs::symlink_metadata(&broken).is_ok());
    }

    #[test]
    fn test_execute_broken_symlinks_no_config_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = service_with_temp(tmp.path());
        let result = svc.execute_broken_symlinks(false);
        assert!(result.success);
        assert_eq!(result.items_cleaned, 0);
    }

    #[test]
    fn test_find_broken_symlinks_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        std::os::unix::fs::symlink("/nowhere", nested.join("deep.link")).unwrap();

        let broken = find_broken_symlinks(tmp.path());
        assert_eq!(broken.len(), 1);
    }

    #[test]
    fn test_find_broken_symlinks_valid_symlink_not_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("real_file");
        fs::write(&target, "content").unwrap();
        std::os::unix::fs::symlink(&target, tmp.path().join("valid.link")).unwrap();

        let broken = find_broken_symlinks(tmp.path());
        assert!(broken.is_empty());
    }

    #[test]
    fn test_broken_symlinks_not_aggressive() {
        assert!(!CleanupCategory::BrokenSymlinks.is_aggressive());
    }

    #[test]
    fn test_broken_symlinks_metadata() {
        let cat = CleanupCategory::BrokenSymlinks;
        assert_eq!(cat.name(), "Broken Symlinks");
        assert_eq!(cat.id(), "broken_symlinks");
        assert!(!cat.description().is_empty());
    }

    #[test]
    fn test_service_preview_dispatches_home_categories() {
        // Use temp home to avoid scanning real filesystem (which can be huge)
        let tmp = tempfile::tempdir().unwrap();
        let svc = service_with_temp(tmp.path());
        // Only test categories that respect home_dir (skip system-path categories)
        let home_cats = vec![
            CleanupCategory::Thumbnails,
            CleanupCategory::UserCache,
            CleanupCategory::AppLogs,
            CleanupCategory::BrokenSymlinks,
            CleanupCategory::DevCache,
            CleanupCategory::BrowserCache,
        ];
        let previews = svc.preview(&home_cats);
        assert_eq!(previews.len(), home_cats.len());
        for (preview, cat) in previews.iter().zip(home_cats.iter()) {
            assert_eq!(preview.category, *cat);
            // All empty in temp dir
            assert_eq!(preview.items_count, 0);
        }
    }

    #[test]
    fn test_service_execute_dry_run_home_categories() {
        // Use temp home to avoid running system commands like pacman/journalctl
        let tmp = tempfile::tempdir().unwrap();
        let svc = service_with_temp(tmp.path());
        let home_cats = vec![
            CleanupCategory::Thumbnails,
            CleanupCategory::UserCache,
            CleanupCategory::AppLogs,
            CleanupCategory::BrokenSymlinks,
            CleanupCategory::DevCache,
            CleanupCategory::BrowserCache,
        ];
        let summary = svc.execute(&home_cats, true);
        for result in &summary.results {
            assert!(result.success, "Dry run failed for {:?}", result.category);
        }
    }
}
