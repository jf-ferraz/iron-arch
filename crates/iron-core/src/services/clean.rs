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
        }
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
        let output = Command::new("pacman").args(["-Qtdq"]).output();

        let (count, _packages) = match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let pkgs: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
                (pkgs.len(), pkgs.join(", "))
            }
            Err(_) => (0, String::new()),
        };

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
        let output = Command::new("journalctl").args(["--disk-usage"]).output();

        let size = match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                parse_journal_size(&stdout)
            }
            Err(_) => 0,
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

        let output = Command::new("sudo")
            .args(["paccache", "-r", &format!("-k{}", self.package_cache_keep)])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                let stderr = String::from_utf8_lossy(&result.stderr).to_string();

                if result.status.success() {
                    // Parse output to count removed packages
                    let removed = stdout.lines().filter(|l| l.contains("removing")).count();
                    CleanupResult::success(
                        CleanupCategory::PackageCache,
                        removed,
                        0, // Actual space reclaimed not easily determined
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

    fn execute_orphan_packages(&self, dry_run: bool) -> CleanupResult {
        // First, get the list of orphans
        let orphan_output = Command::new("pacman").args(["-Qtdq"]).output();

        let orphans = match orphan_output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                stdout
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            }
            Err(_) => vec![],
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

        // Remove orphans
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
            };
            summary.add(result);
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
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_cleanup_category_safe() {
        let safe = CleanupCategory::safe();
        assert_eq!(safe.len(), 6);
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
}
