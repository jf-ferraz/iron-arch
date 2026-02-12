//! Iron Pacman - Package management for Iron
//!
//! This crate provides package management abstractions for Iron:
//! - PackageManager trait for package operations
//! - Pacman command wrapper
//! - AUR helper integration (paru/yay)
//! - Update risk assessment
//! - Arch News RSS parsing

use chrono::{DateTime, Utc};
use iron_core::{IronResult, PackageError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Risk level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Safe to update
    Low,
    /// Review before updating
    Medium,
    /// Requires attention and possible manual intervention
    High,
    /// Critical - snapshot recommended before update
    Critical,
}

impl RiskLevel {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Safe to update",
            RiskLevel::Medium => "Review recommended",
            RiskLevel::High => "Attention required",
            RiskLevel::Critical => "Create snapshot before updating",
        }
    }
}

/// Update preview information
#[derive(Debug, Clone)]
pub struct UpdatePreview {
    /// Packages to be updated
    pub packages: Vec<PackageUpdate>,
    /// Relevant Arch news items
    pub arch_news: Vec<ArchNewsItem>,
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Reasons for the risk assessment
    pub risk_reasons: Vec<String>,
    /// Download size in bytes
    pub download_size: u64,
    /// Installed size delta in bytes
    pub install_size_delta: i64,
}

/// Package update details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUpdate {
    /// Package name
    pub name: String,
    /// Currently installed version
    pub current_version: String,
    /// New version available
    pub new_version: String,
    /// Whether this is an AUR package
    pub is_aur: bool,
    /// Whether the package is flagged out-of-date
    pub is_flagged: bool,
    /// Package repository
    pub repository: String,
}

/// Installed package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Package description
    pub description: String,
    /// Whether explicitly installed (vs dependency)
    pub explicit: bool,
    /// Whether this is an AUR package
    pub is_aur: bool,
    /// Install date
    pub install_date: Option<DateTime<Utc>>,
    /// Package size in bytes
    pub size: u64,
}

/// Arch Linux news item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchNewsItem {
    /// News title
    pub title: String,
    /// Publication date
    pub date: String,
    /// URL to the full article
    pub url: String,
    /// Brief description/summary
    pub description: String,
    /// Whether manual intervention is required
    pub requires_manual: bool,
}

/// Package manager trait for abstraction
pub trait PackageManager {
    /// Check for available updates
    fn check_updates(&self) -> IronResult<Vec<PackageUpdate>>;

    /// Install packages
    fn install(&self, packages: &[String]) -> IronResult<()>;

    /// Remove packages
    fn remove(&self, packages: &[String], remove_deps: bool) -> IronResult<()>;

    /// Query installed packages
    fn query_installed(&self) -> IronResult<Vec<InstalledPackage>>;

    /// Check if a package is installed
    fn is_installed(&self, package: &str) -> IronResult<bool>;

    /// Search for packages
    fn search(&self, query: &str) -> IronResult<Vec<String>>;

    /// Get package info
    fn info(&self, package: &str) -> IronResult<Option<InstalledPackage>>;

    /// Sync database
    fn sync_database(&self) -> IronResult<()>;

    /// Perform full system upgrade
    fn upgrade(&self, preview: bool) -> IronResult<UpdatePreview>;
}

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
}

impl DefaultPackageManager {
    /// Create a new package manager
    pub fn new() -> Self {
        Self {
            aur_helper: detect_aur_helper(),
            dry_run: false,
        }
    }

    /// Create a package manager with specific options
    pub fn with_options(aur_helper: AurHelper, dry_run: bool) -> Self {
        Self { aur_helper, dry_run }
    }

    /// Get the detected AUR helper
    pub fn aur_helper(&self) -> AurHelper {
        self.aur_helper
    }

    /// Run pacman command
    fn run_pacman(&self, args: &[&str]) -> IronResult<String> {
        let output = Command::new("pacman")
            .args(args)
            .output()
            .map_err(|e| PackageError::PacmanError {
                message: format!("Failed to run pacman: {}", e),
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

    /// Run AUR helper command
    fn run_aur_helper(&self, args: &[&str]) -> IronResult<String> {
        let cmd = self.aur_helper.command();
        let output = Command::new(cmd)
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

    /// Parse pacman -Qu output
    fn parse_updates_output(&self, output: &str, is_aur: bool) -> Vec<PackageUpdate> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    Some(PackageUpdate {
                        name: parts[0].to_string(),
                        current_version: parts[1].to_string(),
                        new_version: parts[3].to_string(),
                        is_aur,
                        is_flagged: false,
                        repository: if is_aur { "aur".to_string() } else { "".to_string() },
                    })
                } else {
                    None
                }
            })
            .collect()
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
        if let Ok(output) = Command::new("checkupdates").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                updates.extend(self.parse_updates_output(&stdout, false));
            }
        }

        // Check AUR updates if helper available
        if self.aur_helper != AurHelper::None {
            let aur_args = match self.aur_helper {
                AurHelper::Paru => vec!["-Qua"],
                AurHelper::Yay => vec!["-Qua"],
                _ => vec![],
            };

            if !aur_args.is_empty() {
                if let Ok(output) = self.run_aur_helper(&aur_args) {
                    updates.extend(self.parse_updates_output(&output, true));
                }
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

        let status = Command::new(cmd)
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

        let status = Command::new("pacman")
            .args(&args)
            .status()
            .map_err(|e| PackageError::RemoveFailed {
                message: e.to_string(),
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
                        if let Some(size_str) = value.split_whitespace().next() {
                            if let Ok(size) = size_str.parse::<f64>() {
                                let unit = value.split_whitespace().nth(1).unwrap_or("B");
                                pkg.size = match unit {
                                    "KiB" => (size * 1024.0) as u64,
                                    "MiB" => (size * 1024.0 * 1024.0) as u64,
                                    "GiB" => (size * 1024.0 * 1024.0 * 1024.0) as u64,
                                    _ => size as u64,
                                };
                            }
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
        let status = Command::new("pacman")
            .args(["-Sy"])
            .status()
            .map_err(|e| PackageError::PacmanError {
                message: e.to_string(),
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

/// Assess risk level for updates
pub fn assess_risk(updates: &[PackageUpdate], news: &[ArchNewsItem]) -> (RiskLevel, Vec<String>) {
    let mut reasons = Vec::new();
    let mut risk = RiskLevel::Low;

    // Critical packages that warrant higher risk
    let critical_packages = [
        "linux",
        "linux-lts",
        "linux-zen",
        "linux-hardened",
        "systemd",
        "glibc",
        "gcc",
        "grub",
        "mkinitcpio",
    ];

    let high_risk_packages = [
        "nvidia",
        "nvidia-dkms",
        "mesa",
        "xorg-server",
        "wayland",
        "plasma-desktop",
        "gnome-shell",
        "sddm",
        "gdm",
        "lightdm",
    ];

    // Check for kernel updates
    let kernel_updates: Vec<_> = updates
        .iter()
        .filter(|u| u.name.starts_with("linux") && !u.name.contains("headers"))
        .collect();

    if !kernel_updates.is_empty() {
        reasons.push(format!(
            "Kernel update: {}",
            kernel_updates
                .iter()
                .map(|u| u.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        risk = RiskLevel::Medium;
    }

    // Check for critical package updates
    for pkg in &critical_packages {
        if updates.iter().any(|u| u.name == *pkg) {
            reasons.push(format!("Critical package update: {}", pkg));
            if risk < RiskLevel::High {
                risk = RiskLevel::High;
            }
        }
    }

    // Check for high-risk package updates
    for pkg in &high_risk_packages {
        if updates.iter().any(|u| u.name.starts_with(pkg)) {
            reasons.push(format!("Display/driver update: {}", pkg));
            if risk < RiskLevel::Medium {
                risk = RiskLevel::Medium;
            }
        }
    }

    // Check for flagged AUR packages
    let flagged: Vec<_> = updates.iter().filter(|u| u.is_flagged).collect();
    if !flagged.is_empty() {
        reasons.push(format!(
            "Flagged out-of-date: {}",
            flagged
                .iter()
                .map(|u| u.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        risk = RiskLevel::High;
    }

    // Check for manual intervention news
    let manual_news: Vec<_> = news.iter().filter(|n| n.requires_manual).collect();
    if !manual_news.is_empty() {
        for item in manual_news {
            reasons.push(format!("Manual intervention: {}", item.title));
        }
        risk = RiskLevel::Critical;
    }

    // Large number of updates
    if updates.len() > 100 {
        reasons.push(format!("{} packages to update - consider updating more frequently", updates.len()));
        if risk < RiskLevel::Medium {
            risk = RiskLevel::Medium;
        }
    }

    (risk, reasons)
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
fn parse_arch_news_rss(xml: &str) -> IronResult<Vec<ArchNewsItem>> {
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

    #[test]
    fn test_risk_level_description() {
        assert_eq!(RiskLevel::Low.description(), "Safe to update");
        assert_eq!(RiskLevel::Critical.description(), "Create snapshot before updating");
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
        }];

        let (risk, _) = assess_risk(&updates, &[]);
        assert_eq!(risk, RiskLevel::Low);
    }

    #[test]
    fn test_parse_arch_news_rss() {
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
}
