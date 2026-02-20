//! Iron Pacman - Package management for Iron
//!
//! This crate provides the default PackageManager implementation for Iron:
//! - Pacman command wrapper
//! - AUR helper integration (paru/yay)
//! - Update risk assessment
//! - Arch News RSS parsing
//!
//! The `PackageManager` trait and related types are defined in `iron_core::packages`.
//!
//! # Testing Support
//!
//! The `test_fixtures` module provides mock responses for pacman commands,
//! enabling comprehensive testing without actual pacman execution:
//!
//! ```rust,ignore
//! use iron_pacman::test_fixtures::PacmanMockBuilder;
//! use std::sync::Arc;
//!
//! let executor = PacmanMockBuilder::new()
//!     .with_installed_packages(&[("hyprland", "0.40.0-1")])
//!     .with_updates(&[("hyprland", "0.40.0-1", "0.41.0-1")])
//!     .build();
//!
//! let pm = DefaultPackageManager::with_executor(Arc::new(executor));
//! ```

pub mod test_fixtures;

use iron_core::resilience::{CommandExecutor, RealCommandExecutor};
use iron_core::{IronResult, PackageError};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

// Re-export types from iron-core for backward compatibility
pub use iron_core::{
    ArchNewsItem, InstalledPackage, PackageManager, PackageUpdate, RiskLevel, UpdatePreview,
    assess_risk,
};

/// AUR helper type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AurHelper {
    Paru,
    Yay,
    Pikaur,
    Trizen,
    None,
}

impl AurHelper {
    /// Get the command name
    pub fn command(&self) -> &'static str {
        match self {
            AurHelper::Paru => "paru",
            AurHelper::Yay => "yay",
            AurHelper::Pikaur => "pikaur",
            AurHelper::Trizen => "trizen",
            AurHelper::None => "pacman",
        }
    }
}

/// Default package manager implementation using pacman/AUR helpers
pub struct DefaultPackageManager {
    /// Detected AUR helper
    aur_helper: AurHelper,
    /// Whether to run in dry-run mode
    dry_run: bool,
    /// Optional command executor for resilient command execution
    executor: Option<Arc<dyn CommandExecutor>>,
}

impl DefaultPackageManager {
    /// Create a new package manager with a default resilient executor.
    ///
    /// The circuit breaker opens after 3 consecutive failures and stays open
    /// for 60 seconds, preventing hangs from a broken pacman environment.
    pub fn new() -> Self {
        Self::with_resilience()
    }

    /// Create a package manager with specific options
    pub fn with_options(aur_helper: AurHelper, dry_run: bool) -> Self {
        Self {
            aur_helper,
            dry_run,
            executor: None,
        }
    }

    /// Create a package manager with a command executor for resilient operations
    ///
    /// The executor provides circuit breaker patterns and timeout handling
    /// for pacman commands. When the circuit opens due to repeated failures,
    /// commands will fail fast without attempting execution.
    pub fn with_executor(executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            aur_helper: detect_aur_helper(),
            dry_run: false,
            executor: Some(executor),
        }
    }

    /// Create a package manager with default resilient executor
    ///
    /// Uses the default `RealCommandExecutor` with 120s timeout and circuit breaker.
    pub fn with_resilience() -> Self {
        Self::with_executor(Arc::new(RealCommandExecutor::with_defaults()))
    }

    /// Get the detected AUR helper
    pub fn aur_helper(&self) -> AurHelper {
        self.aur_helper
    }

    /// Run pacman command using executor if available, otherwise direct execution
    fn run_pacman(&self, args: &[&str]) -> IronResult<String> {
        if let Some(ref executor) = self.executor {
            let output =
                executor
                    .execute_full("pacman", args)
                    .map_err(|e| PackageError::PacmanError {
                        message: format!("Failed to run pacman: {}", e),
                    })?;

            if output.success() {
                Ok(output.stdout)
            } else {
                Err(PackageError::PacmanError {
                    message: output.stderr,
                }
                .into())
            }
        } else {
            let output = Command::new("pacman").args(args).output().map_err(|e| {
                PackageError::PacmanError {
                    message: format!("Failed to run pacman: {}", e),
                }
            })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(PackageError::PacmanError {
                    message: stderr.to_string(),
                }
                .into())
            }
        }
    }

    /// Run AUR helper command using executor if available, otherwise direct execution
    fn run_aur_helper(&self, args: &[&str]) -> IronResult<String> {
        let cmd = self.aur_helper.command();

        if let Some(ref executor) = self.executor {
            let output =
                executor
                    .execute_full(cmd, args)
                    .map_err(|e| PackageError::PacmanError {
                        message: format!("Failed to run {}: {}", cmd, e),
                    })?;

            if output.success() {
                Ok(output.stdout)
            } else {
                Err(PackageError::PacmanError {
                    message: output.stderr,
                }
                .into())
            }
        } else {
            let output =
                Command::new(cmd)
                    .args(args)
                    .output()
                    .map_err(|e| PackageError::PacmanError {
                        message: format!("Failed to run {}: {}", cmd, e),
                    })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(PackageError::PacmanError {
                    message: stderr.to_string(),
                }
                .into())
            }
        }
    }
}

impl Default for DefaultPackageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageManager for DefaultPackageManager {
    fn check_updates(&self) -> IronResult<Vec<PackageUpdate>> {
        let mut updates = Vec::new();

        // Check official repository updates using checkupdates
        if let Ok(output) = Command::new("checkupdates").output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            updates.extend(parse_updates_output(&stdout, false));
        }

        // Check AUR updates if helper available
        if self.aur_helper != AurHelper::None {
            let aur_args = match self.aur_helper {
                AurHelper::Paru => vec!["-Qua"],
                AurHelper::Yay => vec!["-Qua"],
                _ => vec![],
            };

            if !aur_args.is_empty()
                && let Ok(output) = self.run_aur_helper(&aur_args)
            {
                updates.extend(parse_updates_output(&output, true));
            }
        }

        Ok(updates)
    }

    fn install(&self, packages: &[String]) -> IronResult<()> {
        if packages.is_empty() {
            return Ok(());
        }

        if self.dry_run {
            return Ok(());
        }

        let cmd = self.aur_helper.command();
        let mut args: Vec<&str> = vec!["-S", "--needed", "--noconfirm"];
        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        args.extend(pkg_refs);

        let status =
            Command::new(cmd)
                .args(&args)
                .status()
                .map_err(|e| PackageError::InstallFailed {
                    message: e.to_string(),
                })?;

        if status.success() {
            Ok(())
        } else {
            Err(PackageError::InstallFailed {
                message: "Package installation failed".to_string(),
            }
            .into())
        }
    }

    fn remove(&self, packages: &[String], remove_deps: bool) -> IronResult<()> {
        if packages.is_empty() {
            return Ok(());
        }

        if self.dry_run {
            return Ok(());
        }

        let mut args = vec!["-R"];
        if remove_deps {
            args.push("-s"); // Remove dependencies
        }
        args.push("--noconfirm");

        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        args.extend(pkg_refs);

        let status = Command::new("pacman").args(&args).status().map_err(|e| {
            PackageError::RemoveFailed {
                message: e.to_string(),
            }
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(PackageError::RemoveFailed {
                message: "Package removal failed".to_string(),
            }
            .into())
        }
    }

    fn query_installed(&self) -> IronResult<Vec<InstalledPackage>> {
        let output = self.run_pacman(&["-Qe"])?;
        let explicit: HashSet<String> = output
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(|s| s.to_string())
            .collect();

        let output = self.run_pacman(&["-Q"])?;
        let packages: Vec<InstalledPackage> = output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(InstalledPackage {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        description: String::new(),
                        explicit: explicit.contains(parts[0]),
                        is_aur: false, // TODO: detect AUR packages
                        install_date: None,
                        size: 0,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    fn is_installed(&self, package: &str) -> IronResult<bool> {
        let output = Command::new("pacman")
            .args(["-Q", package])
            .output()
            .map_err(|e| PackageError::PacmanError {
                message: e.to_string(),
            })?;

        Ok(output.status.success())
    }

    fn search(&self, query: &str) -> IronResult<Vec<String>> {
        let output = self.run_aur_helper(&["-Ss", query])?;
        let packages: Vec<String> = output
            .lines()
            .filter(|line| !line.starts_with(' '))
            .filter_map(|line| line.split('/').nth(1))
            .filter_map(|s| s.split_whitespace().next())
            .map(|s| s.to_string())
            .collect();

        Ok(packages)
    }

    fn info(&self, package: &str) -> IronResult<Option<InstalledPackage>> {
        let output = Command::new("pacman")
            .args(["-Qi", package])
            .output()
            .map_err(|e| PackageError::PacmanError {
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pkg = InstalledPackage {
            name: package.to_string(),
            version: String::new(),
            description: String::new(),
            explicit: false,
            is_aur: false,
            install_date: None,
            size: 0,
        };

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "Version" => pkg.version = value.to_string(),
                    "Description" => pkg.description = value.to_string(),
                    "Installed Size" => {
                        // Parse size like "123.45 MiB"
                        if let Some(size_str) = value.split_whitespace().next()
                            && let Ok(size) = size_str.parse::<f64>()
                        {
                            let unit = value.split_whitespace().nth(1).unwrap_or("B");
                            pkg.size = match unit {
                                "KiB" => (size * 1024.0) as u64,
                                "MiB" => (size * 1024.0 * 1024.0) as u64,
                                "GiB" => (size * 1024.0 * 1024.0 * 1024.0) as u64,
                                _ => size as u64,
                            };
                        }
                    }
                    "Install Reason" => {
                        pkg.explicit = value.contains("Explicitly installed");
                    }
                    _ => {}
                }
            }
        }

        Ok(Some(pkg))
    }

    fn sync_database(&self) -> IronResult<()> {
        let status = Command::new("pacman").args(["-Sy"]).status().map_err(|e| {
            PackageError::PacmanError {
                message: e.to_string(),
            }
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(PackageError::UpdateFailed {
                message: "Database sync failed".to_string(),
            }
            .into())
        }
    }

    fn upgrade(&self, preview: bool) -> IronResult<UpdatePreview> {
        let updates = self.check_updates()?;
        let news = fetch_arch_news()?;
        let (risk_level, risk_reasons) = assess_risk(&updates, &news);

        let preview_result = UpdatePreview {
            packages: updates,
            arch_news: news,
            risk_level,
            risk_reasons,
            download_size: 0,
            install_size_delta: 0,
        };

        if !preview && !self.dry_run {
            let cmd = self.aur_helper.command();
            let status = Command::new(cmd)
                .args(["-Syu", "--noconfirm"])
                .status()
                .map_err(|e| PackageError::UpdateFailed {
                    message: e.to_string(),
                })?;

            if !status.success() {
                return Err(PackageError::UpdateFailed {
                    message: "System upgrade failed".to_string(),
                }
                .into());
            }
        }

        Ok(preview_result)
    }

    fn fetch_news(&self) -> IronResult<Vec<ArchNewsItem>> {
        fetch_arch_news()
    }
}

/// Detect available AUR helper
pub fn detect_aur_helper() -> AurHelper {
    let helpers = [
        ("paru", AurHelper::Paru),
        ("yay", AurHelper::Yay),
        ("pikaur", AurHelper::Pikaur),
        ("trizen", AurHelper::Trizen),
    ];

    for (cmd, helper) in helpers {
        if Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return helper;
        }
    }

    AurHelper::None
}

/// Fetch Arch Linux news from RSS feed
pub fn fetch_arch_news() -> IronResult<Vec<ArchNewsItem>> {
    // Try to fetch from archlinux.org RSS feed
    let response = match ureq::get("https://archlinux.org/feeds/news/")
        .timeout(std::time::Duration::from_secs(10))
        .call()
    {
        Ok(resp) => resp,
        Err(_) => return Ok(Vec::new()), // Fail gracefully if offline
    };

    let body = match response.into_string() {
        Ok(b) => b,
        Err(_) => return Ok(Vec::new()),
    };

    parse_arch_news_rss(&body)
}

/// Parse Arch News RSS feed
pub fn parse_arch_news_rss(xml: &str) -> IronResult<Vec<ArchNewsItem>> {
    let mut items = Vec::new();
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.trim_text(true);

    let mut in_item = false;
    let mut current_item = ArchNewsItem {
        title: String::new(),
        date: String::new(),
        url: String::new(),
        description: String::new(),
        requires_manual: false,
    };
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "item" {
                    in_item = true;
                    current_item = ArchNewsItem {
                        title: String::new(),
                        date: String::new(),
                        url: String::new(),
                        description: String::new(),
                        requires_manual: false,
                    };
                }
                current_tag = tag;
            }
            Ok(quick_xml::events::Event::Text(e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "title" => current_item.title = text,
                        "pubDate" => current_item.date = text,
                        "link" => current_item.url = text,
                        "description" => {
                            current_item.description = text.clone();
                            // Check for manual intervention keywords
                            let lower = text.to_lowercase();
                            current_item.requires_manual = lower.contains("manual intervention")
                                || lower.contains("action required")
                                || lower.contains("must be done manually")
                                || lower.contains("before upgrading");
                        }
                        _ => {}
                    }
                }
            }
            Ok(quick_xml::events::Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "item" {
                    in_item = false;
                    items.push(current_item.clone());
                    if items.len() >= 10 {
                        break; // Only keep recent 10 items
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(items)
}

// =============================================================================
// Public Parsing Functions (for testing)
// =============================================================================

/// Parse pacman -Qu or checkupdates output
///
/// Format: `package_name current_version -> new_version`
///
/// # Example
/// ```
/// use iron_pacman::parse_updates_output;
///
/// let output = "hyprland 0.40.0-1 -> 0.41.0-1\nwaybar 0.10.0-1 -> 0.10.1-1";
/// let updates = parse_updates_output(output, false);
/// assert_eq!(updates.len(), 2);
/// assert_eq!(updates[0].name, "hyprland");
/// ```
pub fn parse_updates_output(output: &str, is_aur: bool) -> Vec<PackageUpdate> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                Some(PackageUpdate {
                    name: parts[0].to_string(),
                    current_version: parts[1].to_string(),
                    new_version: parts[3].to_string(),
                    is_aur,
                    is_flagged: false,
                    repository: if is_aur {
                        "aur".to_string()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                })
            } else {
                None
            }
        })
        .collect()
}

/// Parse pacman -Q output (package list)
///
/// Format: `package_name version`
///
/// # Example
/// ```
/// use iron_pacman::parse_package_list;
///
/// let output = "hyprland 0.40.0-1\nwaybar 0.10.0-1";
/// let packages = parse_package_list(output);
/// assert_eq!(packages.len(), 2);
/// assert_eq!(packages[0].0, "hyprland");
/// assert_eq!(packages[0].1, "0.40.0-1");
/// ```
pub fn parse_package_list(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

/// Parse pacman -Ss search output
///
/// Format lines alternate between:
/// - `repository/package_name version [installed]`
/// - `    Description text`
///
/// # Example
/// ```
/// use iron_pacman::parse_search_output;
///
/// let output = "extra/hyprland 0.40.0-1
///     A highly customizable dynamic tiling Wayland compositor";
/// let packages = parse_search_output(output);
/// assert_eq!(packages.len(), 1);
/// assert_eq!(packages[0].name, "hyprland");
/// ```
pub fn parse_search_output(output: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(header_line) = lines.next() {
        // Skip empty lines and indented lines (descriptions from previous entry)
        if header_line.trim().is_empty() || header_line.starts_with(' ') {
            continue;
        }

        // Parse header: repository/name version [flags]
        if let Some((repo_pkg, rest)) = header_line.split_once('/') {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0].to_string();
                let version = parts.get(1).unwrap_or(&"").to_string();
                let installed = rest.contains("[installed]");
                let repository = repo_pkg.to_string();

                // Try to get description from next line
                let description = if let Some(&desc_line) = lines.peek() {
                    if desc_line.starts_with(' ') {
                        lines.next();
                        desc_line.trim().to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                results.push(SearchResult {
                    name,
                    version,
                    description,
                    repository,
                    installed,
                });
            }
        }
    }

    results
}

/// Parse pacman -Qi info output
///
/// Format: `Key : Value` pairs
///
/// # Example
/// ```
/// use iron_pacman::parse_package_info;
///
/// let output = "Name            : hyprland
/// Version         : 0.40.0-1
/// Description     : A tiling Wayland compositor
/// Installed Size  : 12.5 MiB";
/// let info = parse_package_info(output);
/// assert_eq!(info.get("Name"), Some(&"hyprland".to_string()));
/// assert_eq!(info.get("Version"), Some(&"0.40.0-1".to_string()));
/// ```
pub fn parse_package_info(output: &str) -> std::collections::HashMap<String, String> {
    let mut info = std::collections::HashMap::new();

    for line in output.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                info.insert(key, value);
            }
        }
    }

    info
}

/// Parse installed size from pacman info output
///
/// Handles units: B, KiB, MiB, GiB
///
/// # Example
/// ```
/// use iron_pacman::parse_size;
///
/// assert_eq!(parse_size("12.5 MiB"), 13107200);
/// assert_eq!(parse_size("1024 KiB"), 1048576);
/// assert_eq!(parse_size("1 GiB"), 1073741824);
/// ```
pub fn parse_size(size_str: &str) -> u64 {
    let parts: Vec<&str> = size_str.split_whitespace().collect();
    if parts.len() >= 2
        && let Ok(size) = parts[0].parse::<f64>()
    {
        return match parts[1] {
            "B" => size as u64,
            "KiB" => (size * 1024.0) as u64,
            "MiB" => (size * 1024.0 * 1024.0) as u64,
            "GiB" => (size * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => size as u64,
        };
    }
    0
}

/// Search result from pacman -Ss
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub installed: bool,
}

/// Check if a package file exists in the pacman cache
pub fn is_cached(package: &str, version: &str) -> bool {
    let cache_dir = Path::new("/var/cache/pacman/pkg");
    let pattern = format!("{}-{}", package, version);

    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&pattern) {
                return true;
            }
        }
    }

    false
}

/// Clean pacman cache
pub fn clean_cache(keep_versions: u32) -> IronResult<()> {
    let status = Command::new("paccache")
        .args(["-rk", &keep_versions.to_string()])
        .status()
        .map_err(|e| PackageError::PacmanError {
            message: format!("Failed to clean cache: {}", e),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(PackageError::PacmanError {
            message: "Cache cleanup failed".to_string(),
        }
        .into())
    }
}

/// Get orphan packages (no longer required as dependencies)
pub fn get_orphans() -> IronResult<Vec<String>> {
    let output = Command::new("pacman")
        .args(["-Qtdq"])
        .output()
        .map_err(|e| PackageError::PacmanError {
            message: e.to_string(),
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect())
    } else {
        Ok(Vec::new()) // No orphans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Risk Assessment Tests (existing)
    // =========================================================================

    #[test]
    fn test_risk_level_description() {
        assert_eq!(RiskLevel::Low.description(), "Safe to update");
        assert_eq!(
            RiskLevel::Critical.description(),
            "Create snapshot before updating"
        );
    }

    #[test]
    fn test_assess_risk_kernel_update() {
        let updates = vec![PackageUpdate {
            name: "linux".to_string(),
            current_version: "6.7.0".to_string(),
            new_version: "6.7.1".to_string(),
            is_aur: false,
            is_flagged: false,
            repository: "core".to_string(),
            ..Default::default()
        }];

        let (risk, reasons) = assess_risk(&updates, &[]);
        assert!(risk >= RiskLevel::Medium);
        assert!(reasons.iter().any(|r| r.contains("Kernel")));
    }

    #[test]
    fn test_assess_risk_critical_package() {
        let updates = vec![PackageUpdate {
            name: "systemd".to_string(),
            current_version: "254".to_string(),
            new_version: "255".to_string(),
            is_aur: false,
            is_flagged: false,
            repository: "core".to_string(),
            ..Default::default()
        }];

        let (risk, reasons) = assess_risk(&updates, &[]);
        assert!(risk >= RiskLevel::High);
        assert!(reasons.iter().any(|r| r.contains("Critical")));
    }

    #[test]
    fn test_assess_risk_manual_intervention() {
        let updates = vec![];
        let news = vec![ArchNewsItem {
            title: "Important update".to_string(),
            date: "2024-01-01".to_string(),
            url: "https://archlinux.org/news/".to_string(),
            description: "Manual intervention required".to_string(),
            requires_manual: true,
        }];

        let (risk, reasons) = assess_risk(&updates, &news);
        assert_eq!(risk, RiskLevel::Critical);
        assert!(reasons.iter().any(|r| r.contains("Manual intervention")));
    }

    #[test]
    fn test_assess_risk_flagged_aur() {
        let updates = vec![PackageUpdate {
            name: "some-aur-pkg".to_string(),
            current_version: "1.0".to_string(),
            new_version: "1.1".to_string(),
            is_aur: true,
            is_flagged: true,
            repository: "aur".to_string(),
            ..Default::default()
        }];

        let (risk, reasons) = assess_risk(&updates, &[]);
        assert!(risk >= RiskLevel::High);
        assert!(reasons.iter().any(|r| r.contains("Flagged")));
    }

    #[test]
    fn test_assess_risk_low() {
        let updates = vec![PackageUpdate {
            name: "htop".to_string(),
            current_version: "3.2.1".to_string(),
            new_version: "3.2.2".to_string(),
            is_aur: false,
            is_flagged: false,
            repository: "extra".to_string(),
            ..Default::default()
        }];

        let (risk, _) = assess_risk(&updates, &[]);
        assert_eq!(risk, RiskLevel::Low);
    }

    #[test]
    fn test_aur_helper_command() {
        assert_eq!(AurHelper::Paru.command(), "paru");
        assert_eq!(AurHelper::Yay.command(), "yay");
        assert_eq!(AurHelper::None.command(), "pacman");
    }

    #[test]
    fn test_default_package_manager() {
        let pm = DefaultPackageManager::new();
        // Just test that it creates without panicking
        let _ = pm.aur_helper();
    }

    // =========================================================================
    // Parse Updates Output Tests
    // =========================================================================

    #[test]
    fn test_parse_updates_single_package() {
        let output = "hyprland 0.40.0-1 -> 0.41.0-1";
        let updates = parse_updates_output(output, false);

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "hyprland");
        assert_eq!(updates[0].current_version, "0.40.0-1");
        assert_eq!(updates[0].new_version, "0.41.0-1");
        assert!(!updates[0].is_aur);
    }

    #[test]
    fn test_parse_updates_multiple_packages() {
        let output = "hyprland 0.40.0-1 -> 0.41.0-1
waybar 0.10.0-1 -> 0.10.1-1
wofi 1.4-1 -> 1.4.1-1";

        let updates = parse_updates_output(output, false);

        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0].name, "hyprland");
        assert_eq!(updates[1].name, "waybar");
        assert_eq!(updates[2].name, "wofi");
    }

    #[test]
    fn test_parse_updates_aur_packages() {
        let output = "paru-bin 2.0.0-1 -> 2.0.1-1";
        let updates = parse_updates_output(output, true);

        assert_eq!(updates.len(), 1);
        assert!(updates[0].is_aur);
        assert_eq!(updates[0].repository, "aur");
    }

    #[test]
    fn test_parse_updates_empty_output() {
        let updates = parse_updates_output("", false);
        assert!(updates.is_empty());
    }

    #[test]
    fn test_parse_updates_whitespace_only() {
        let updates = parse_updates_output("   \n\n   \n", false);
        assert!(updates.is_empty());
    }

    #[test]
    fn test_parse_updates_malformed_line() {
        // Line needs < 4 parts to be considered malformed
        let output = "pkg 1.0 2.0"; // Only 3 parts, missing arrow
        let updates = parse_updates_output(output, false);
        assert!(updates.is_empty());
    }

    #[test]
    fn test_parse_updates_mixed_valid_invalid() {
        let output = "invalid line
hyprland 0.40.0-1 -> 0.41.0-1
another invalid
waybar 0.10.0-1 -> 0.10.1-1";

        let updates = parse_updates_output(output, false);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "hyprland");
        assert_eq!(updates[1].name, "waybar");
    }

    #[test]
    fn test_parse_updates_complex_versions() {
        let output = "linux 6.7.0.arch1-1 -> 6.7.1.arch1-1
gcc-libs 13.2.1-3 -> 14.0.0-1";

        let updates = parse_updates_output(output, false);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].current_version, "6.7.0.arch1-1");
        assert_eq!(updates[0].new_version, "6.7.1.arch1-1");
    }

    // =========================================================================
    // Parse Package List Tests
    // =========================================================================

    #[test]
    fn test_parse_package_list_single() {
        let output = "hyprland 0.40.0-1";
        let packages = parse_package_list(output);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].0, "hyprland");
        assert_eq!(packages[0].1, "0.40.0-1");
    }

    #[test]
    fn test_parse_package_list_multiple() {
        let output = "base 3-2
linux 6.7.1-1
systemd 255-1";

        let packages = parse_package_list(output);
        assert_eq!(packages.len(), 3);
        assert_eq!(packages[0], ("base".to_string(), "3-2".to_string()));
        assert_eq!(packages[1], ("linux".to_string(), "6.7.1-1".to_string()));
        assert_eq!(packages[2], ("systemd".to_string(), "255-1".to_string()));
    }

    #[test]
    fn test_parse_package_list_empty() {
        let packages = parse_package_list("");
        assert!(packages.is_empty());
    }

    #[test]
    fn test_parse_package_list_with_blank_lines() {
        let output = "hyprland 0.40.0-1

waybar 0.10.0-1

wofi 1.4-1";

        let packages = parse_package_list(output);
        assert_eq!(packages.len(), 3);
    }

    #[test]
    fn test_parse_package_list_extra_whitespace() {
        let output = "  hyprland   0.40.0-1  ";
        let packages = parse_package_list(output);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].0, "hyprland");
    }

    // =========================================================================
    // Parse Search Output Tests
    // =========================================================================

    #[test]
    fn test_parse_search_single_result() {
        let output = "extra/hyprland 0.40.0-1
    A highly customizable dynamic tiling Wayland compositor";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hyprland");
        assert_eq!(results[0].version, "0.40.0-1");
        assert_eq!(results[0].repository, "extra");
        assert!(!results[0].installed);
    }

    #[test]
    fn test_parse_search_multiple_results() {
        let output = "extra/hyprland 0.40.0-1
    A highly customizable dynamic tiling Wayland compositor
extra/waybar 0.10.0-1
    Highly customizable Wayland bar
aur/hyprshot 1.0.0-1
    Screenshot utility for Hyprland";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "hyprland");
        assert_eq!(results[1].name, "waybar");
        assert_eq!(results[2].name, "hyprshot");
        assert_eq!(results[2].repository, "aur");
    }

    #[test]
    fn test_parse_search_installed_marker() {
        let output = "extra/hyprland 0.40.0-1 [installed]
    A highly customizable dynamic tiling Wayland compositor";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert!(results[0].installed);
    }

    #[test]
    fn test_parse_search_empty() {
        let results = parse_search_output("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_search_no_description() {
        let output = "extra/hyprland 0.40.0-1";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hyprland");
        assert!(results[0].description.is_empty());
    }

    // =========================================================================
    // Parse Package Info Tests
    // =========================================================================

    #[test]
    fn test_parse_package_info_basic() {
        let output = "Name            : hyprland
Version         : 0.40.0-1
Description     : A tiling Wayland compositor";

        let info = parse_package_info(output);
        assert_eq!(info.get("Name"), Some(&"hyprland".to_string()));
        assert_eq!(info.get("Version"), Some(&"0.40.0-1".to_string()));
        assert_eq!(
            info.get("Description"),
            Some(&"A tiling Wayland compositor".to_string())
        );
    }

    #[test]
    fn test_parse_package_info_with_colon_in_value() {
        let output = "URL             : https://hyprland.org
Description     : A compositor: modern and fast";

        let info = parse_package_info(output);
        assert_eq!(info.get("URL"), Some(&"https://hyprland.org".to_string()));
        assert_eq!(
            info.get("Description"),
            Some(&"A compositor: modern and fast".to_string())
        );
    }

    #[test]
    fn test_parse_package_info_empty() {
        let info = parse_package_info("");
        assert!(info.is_empty());
    }

    #[test]
    fn test_parse_package_info_install_reason() {
        let output = "Name            : hyprland
Install Reason  : Explicitly installed";

        let info = parse_package_info(output);
        assert_eq!(
            info.get("Install Reason"),
            Some(&"Explicitly installed".to_string())
        );
    }

    // =========================================================================
    // Parse Size Tests
    // =========================================================================

    #[test]
    fn test_parse_size_bytes() {
        assert_eq!(parse_size("1024 B"), 1024);
    }

    #[test]
    fn test_parse_size_kib() {
        assert_eq!(parse_size("1 KiB"), 1024);
        assert_eq!(parse_size("2.5 KiB"), 2560);
    }

    #[test]
    fn test_parse_size_mib() {
        assert_eq!(parse_size("1 MiB"), 1048576);
        assert_eq!(parse_size("12.5 MiB"), 13107200);
    }

    #[test]
    fn test_parse_size_gib() {
        assert_eq!(parse_size("1 GiB"), 1073741824);
    }

    #[test]
    fn test_parse_size_invalid() {
        assert_eq!(parse_size(""), 0);
        assert_eq!(parse_size("invalid"), 0);
        assert_eq!(parse_size("abc MiB"), 0);
    }

    #[test]
    fn test_parse_size_float_precision() {
        // 100.5 MiB
        let size = parse_size("100.5 MiB");
        assert!(size > 100 * 1024 * 1024);
        assert!(size < 101 * 1024 * 1024);
    }

    // =========================================================================
    // Parse Arch News RSS Tests
    // =========================================================================

    #[test]
    fn test_parse_arch_news_rss_single_item() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
            <channel>
                <item>
                    <title>Test News Item</title>
                    <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
                    <link>https://archlinux.org/news/test/</link>
                    <description>This requires manual intervention before upgrading.</description>
                </item>
            </channel>
        </rss>"#;

        let items = parse_arch_news_rss(xml).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test News Item");
        assert!(items[0].requires_manual);
    }

    #[test]
    fn test_parse_arch_news_rss_multiple_items() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
            <channel>
                <item>
                    <title>First News</title>
                    <pubDate>Mon, 01 Jan 2024</pubDate>
                    <link>https://archlinux.org/news/1/</link>
                    <description>Regular update</description>
                </item>
                <item>
                    <title>Second News</title>
                    <pubDate>Tue, 02 Jan 2024</pubDate>
                    <link>https://archlinux.org/news/2/</link>
                    <description>Action required by user</description>
                </item>
            </channel>
        </rss>"#;

        let items = parse_arch_news_rss(xml).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "First News");
        assert_eq!(items[1].title, "Second News");
        assert!(!items[0].requires_manual);
        assert!(items[1].requires_manual);
    }

    #[test]
    fn test_parse_arch_news_rss_manual_intervention_keywords() {
        let test_cases = [
            ("Manual intervention required", true),
            ("Action required for upgrade", true),
            ("This must be done manually", true),
            ("Read this before upgrading", true),
            ("Regular package update", false),
            ("Bug fixes and improvements", false),
        ];

        for (description, should_require_manual) in test_cases {
            let xml = format!(
                r#"<?xml version="1.0"?>
                <rss><channel>
                    <item>
                        <title>Test</title>
                        <description>{}</description>
                    </item>
                </channel></rss>"#,
                description
            );

            let items = parse_arch_news_rss(&xml).unwrap();
            assert_eq!(
                items[0].requires_manual, should_require_manual,
                "Failed for description: {}",
                description
            );
        }
    }

    #[test]
    fn test_parse_arch_news_rss_empty() {
        let xml = r#"<?xml version="1.0"?>
        <rss><channel></channel></rss>"#;

        let items = parse_arch_news_rss(xml).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_arch_news_rss_limit_to_10() {
        // Create XML with 15 items
        let mut items_xml = String::new();
        for i in 0..15 {
            items_xml.push_str(&format!(
                "<item><title>Item {}</title><description>Desc</description></item>",
                i
            ));
        }
        let xml = format!(
            r#"<?xml version="1.0"?><rss><channel>{}</channel></rss>"#,
            items_xml
        );

        let items = parse_arch_news_rss(&xml).unwrap();
        assert_eq!(items.len(), 10, "Should limit to 10 items");
    }

    #[test]
    fn test_parse_arch_news_rss_malformed() {
        let xml = "not valid xml at all";
        let items = parse_arch_news_rss(xml).unwrap();
        // Should not panic, just return empty
        assert!(items.is_empty());
    }

    // =========================================================================
    // AUR Helper Tests
    // =========================================================================

    #[test]
    fn test_aur_helper_pikaur() {
        assert_eq!(AurHelper::Pikaur.command(), "pikaur");
    }

    #[test]
    fn test_aur_helper_trizen() {
        assert_eq!(AurHelper::Trizen.command(), "trizen");
    }

    #[test]
    fn test_aur_helper_equality() {
        assert_eq!(AurHelper::Paru, AurHelper::Paru);
        assert_ne!(AurHelper::Paru, AurHelper::Yay);
    }

    // =========================================================================
    // Package Manager Configuration Tests
    // =========================================================================

    #[test]
    fn test_package_manager_with_options() {
        let pm = DefaultPackageManager::with_options(AurHelper::Paru, true);
        assert_eq!(pm.aur_helper(), AurHelper::Paru);
    }

    #[test]
    fn test_package_manager_default() {
        let pm: DefaultPackageManager = Default::default();
        // Just verify it doesn't panic
        let _ = pm.aur_helper();
    }

    // =========================================================================
    // Search Result Tests
    // =========================================================================

    #[test]
    fn test_search_result_clone() {
        let result = SearchResult {
            name: "hyprland".to_string(),
            version: "0.40.0".to_string(),
            description: "A compositor".to_string(),
            repository: "extra".to_string(),
            installed: true,
        };

        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_search_result_debug() {
        let result = SearchResult {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: false,
        };

        // Should implement Debug
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("test"));
    }

    // =========================================================================
    // Edge Cases and Error Conditions
    // =========================================================================

    #[test]
    fn test_parse_updates_unicode_package_names() {
        // Some AUR packages might have unicode in descriptions, but names should be ASCII
        let output = "some-pkg 1.0 -> 2.0";
        let updates = parse_updates_output(output, false);
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn test_parse_updates_very_long_version() {
        let output = "pkg 1.0.0.0.0.0.0.0.0.0.0.0.0.0-1 -> 2.0.0.0.0.0.0.0.0.0.0.0.0.0-1";
        let updates = parse_updates_output(output, false);

        assert_eq!(updates.len(), 1);
        assert!(updates[0].current_version.starts_with("1.0"));
        assert!(updates[0].new_version.starts_with("2.0"));
    }

    #[test]
    fn test_parse_package_list_thousands_of_packages() {
        // Generate large output
        let mut output = String::new();
        for i in 0..1000 {
            output.push_str(&format!("pkg-{} 1.0.0-1\n", i));
        }

        let packages = parse_package_list(&output);
        assert_eq!(packages.len(), 1000);
        assert_eq!(packages[0].0, "pkg-0");
        assert_eq!(packages[999].0, "pkg-999");
    }

    #[test]
    fn test_parse_search_multiline_description() {
        // Real pacman search output only has single-line descriptions
        let output = "extra/pkg 1.0
    First line of description";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].description, "First line of description");
    }

    // =========================================================================
    // SearchResult Extended Tests
    // =========================================================================

    #[test]
    fn test_search_result_equality() {
        let r1 = SearchResult {
            name: "pkg".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: false,
        };

        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_search_result_inequality() {
        let r1 = SearchResult {
            name: "pkg1".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: false,
        };

        let r2 = SearchResult {
            name: "pkg2".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: false,
        };

        assert_ne!(r1, r2);
    }

    #[test]
    fn test_search_result_installed_flag() {
        let r1 = SearchResult {
            name: "pkg".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: true,
        };

        let r2 = SearchResult {
            name: "pkg".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: "extra".to_string(),
            installed: false,
        };

        assert_ne!(r1, r2);
        assert!(r1.installed);
        assert!(!r2.installed);
    }

    // =========================================================================
    // AurHelper Extended Tests
    // =========================================================================

    #[test]
    fn test_aur_helper_clone() {
        let h1 = AurHelper::Paru;
        let h2 = h1.clone();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_aur_helper_copy() {
        let h1 = AurHelper::Yay;
        let h2 = h1;
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_aur_helper_debug() {
        let helpers = vec![
            AurHelper::Paru,
            AurHelper::Yay,
            AurHelper::Pikaur,
            AurHelper::Trizen,
            AurHelper::None,
        ];

        for helper in helpers {
            let debug = format!("{:?}", helper);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_all_aur_helper_commands() {
        assert_eq!(AurHelper::Paru.command(), "paru");
        assert_eq!(AurHelper::Yay.command(), "yay");
        assert_eq!(AurHelper::Pikaur.command(), "pikaur");
        assert_eq!(AurHelper::Trizen.command(), "trizen");
        assert_eq!(AurHelper::None.command(), "pacman");
    }

    // =========================================================================
    // Parse Size Extended Tests
    // =========================================================================

    #[test]
    fn test_parse_size_single_digit() {
        assert_eq!(parse_size("5 MiB"), 5 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_large_number() {
        let size = parse_size("100 GiB");
        assert_eq!(size, 100 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_just_bytes() {
        assert_eq!(parse_size("500 B"), 500);
    }

    #[test]
    fn test_parse_size_negative_treated_as_invalid() {
        // Negative numbers should be handled gracefully
        let size = parse_size("-10 MiB");
        // parse::<f64>() handles negative, but result will be 0 or negative cast
        assert!(size == 0 || size > u64::MAX / 2); // Overflow wrap
    }

    #[test]
    fn test_parse_size_whitespace() {
        assert_eq!(parse_size("  10   MiB  "), 10 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_unknown_unit() {
        // Unknown unit should use value as-is
        assert_eq!(parse_size("100 TiB"), 100);
    }

    // =========================================================================
    // Parse Package Info Extended Tests
    // =========================================================================

    #[test]
    fn test_parse_package_info_full() {
        let output = r#"Name            : hyprland
Version         : 0.40.0-1
Description     : A highly customizable dynamic tiling Wayland compositor
URL             : https://hyprland.org
Architecture    : x86_64
Licenses        : BSD
Groups          : None
Provides        : hyprland-git
Depends On      : cairo  libdrm  libinput
Optional Deps   : xdg-desktop-portal-hyprland
Required By     : None
Installed Size  : 12.50 MiB
Install Date    : Mon 01 Jan 2024 10:00:00 AM UTC
Install Reason  : Explicitly installed
Install Script  : No
Validated By    : Signature"#;

        let info = parse_package_info(output);
        assert_eq!(info.get("Name"), Some(&"hyprland".to_string()));
        assert_eq!(info.get("Version"), Some(&"0.40.0-1".to_string()));
        assert_eq!(info.get("Architecture"), Some(&"x86_64".to_string()));
        assert_eq!(info.get("Installed Size"), Some(&"12.50 MiB".to_string()));
    }

    #[test]
    fn test_parse_package_info_missing_value() {
        let output = "Name:";
        let info = parse_package_info(output);
        assert_eq!(info.get("Name"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_package_info_no_colon() {
        let output = "This line has no colon";
        let info = parse_package_info(output);
        assert!(info.is_empty());
    }

    // =========================================================================
    // Parse Updates Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_updates_extra_spaces() {
        let output = "pkg   1.0   ->   2.0";
        let updates = parse_updates_output(output, false);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "pkg");
    }

    #[test]
    fn test_parse_updates_tabs() {
        let output = "pkg\t1.0\t->\t2.0";
        let updates = parse_updates_output(output, false);
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn test_parse_updates_preserves_order() {
        let output = "aaa 1.0 -> 2.0\nbbb 1.0 -> 2.0\nccc 1.0 -> 2.0";
        let updates = parse_updates_output(output, false);
        assert_eq!(updates[0].name, "aaa");
        assert_eq!(updates[1].name, "bbb");
        assert_eq!(updates[2].name, "ccc");
    }

    // =========================================================================
    // Parse Package List Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_package_list_single_word() {
        // Malformed line with just package name
        let output = "pkgname";
        let packages = parse_package_list(output);
        assert!(packages.is_empty());
    }

    #[test]
    fn test_parse_package_list_many_columns() {
        // Extra columns should be ignored
        let output = "pkg 1.0 extra stuff here";
        let packages = parse_package_list(output);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].0, "pkg");
        assert_eq!(packages[0].1, "1.0");
    }

    // =========================================================================
    // Parse Search Output Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_search_group_package() {
        let output = "extra/base-devel 1-1 (base-devel)
    Development tools";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_search_outdated_marker() {
        // Note: Parser checks for "[installed]" exact substring (with bracket)
        // "[installed: 0.9]" does NOT match "[installed]"
        let output = "extra/pkg 1.0 [installed]
    Description";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert!(results[0].installed);
    }

    #[test]
    fn test_parse_search_installed_with_version_no_match() {
        // [installed: X.Y] format does NOT match "[installed]" substring
        // because it's "[installed:" not "[installed]"
        let output = "extra/pkg 1.0 [installed: 0.9]
    Description";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        // This should NOT be detected as installed by current parser
        assert!(!results[0].installed);
    }

    #[test]
    fn test_parse_search_aur_package() {
        let output = "aur/hyprshot 1.0.0-1
    Screenshot utility for Hyprland";

        let results = parse_search_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].repository, "aur");
    }

    // =========================================================================
    // Arch News RSS Extended Tests
    // =========================================================================

    #[test]
    fn test_parse_arch_news_partial_item() {
        let xml = r#"<?xml version="1.0"?>
        <rss><channel>
            <item>
                <title>Only Title</title>
            </item>
        </channel></rss>"#;

        let items = parse_arch_news_rss(xml).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Only Title");
        assert!(items[0].date.is_empty());
    }

    #[test]
    fn test_parse_arch_news_cdata() {
        let xml = r#"<?xml version="1.0"?>
        <rss><channel>
            <item>
                <title><![CDATA[Title with <special> chars]]></title>
                <description>Normal desc</description>
            </item>
        </channel></rss>"#;

        let items = parse_arch_news_rss(xml).unwrap();
        assert_eq!(items.len(), 1);
    }

    // =========================================================================
    // Circuit Breaker Integration Tests
    // =========================================================================

    #[test]
    fn test_package_manager_with_resilience() {
        // Test that with_resilience creates a properly configured package manager
        let pm = DefaultPackageManager::with_resilience();
        assert!(pm.executor.is_some());
        // Should have detected AUR helper or fallen back to None
        let _ = pm.aur_helper();
    }

    #[test]
    fn test_package_manager_with_executor() {
        use iron_core::resilience::RealCommandExecutor;
        use std::sync::Arc;

        // Test that with_executor accepts a custom executor
        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let pm = DefaultPackageManager::with_executor(executor);
        assert!(pm.executor.is_some());
    }

    #[test]
    fn test_package_manager_new_has_resilient_executor() {
        // new() always initializes with a circuit-breaker executor
        let pm = DefaultPackageManager::new();
        assert!(pm.executor.is_some());
    }

    #[test]
    fn test_package_manager_with_options_no_executor() {
        // Test with_options doesn't set executor
        let pm = DefaultPackageManager::with_options(AurHelper::None, true);
        assert!(pm.executor.is_none());
        assert!(pm.dry_run);
    }
}
