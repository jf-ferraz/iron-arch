//! Package Management Types
//!
//! Domain types for package management operations.
//! Implementations are provided by iron-pacman.

use crate::IronResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Risk level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum RiskLevel {
    /// Safe to update
    #[default]
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
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
///
/// This trait defines the interface for package management operations.
/// The default implementation is provided by `iron_pacman::DefaultPackageManager`.
pub trait PackageManager: Send + Sync {
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

    /// Get installed package count
    fn installed_count(&self) -> IronResult<usize> {
        Ok(self.query_installed()?.len())
    }
}

/// No-op package manager for testing
#[derive(Debug, Clone, Default)]
pub struct NoopPackageManager;

impl PackageManager for NoopPackageManager {
    fn check_updates(&self) -> IronResult<Vec<PackageUpdate>> {
        Ok(Vec::new())
    }

    fn install(&self, _packages: &[String]) -> IronResult<()> {
        Ok(())
    }

    fn remove(&self, _packages: &[String], _remove_deps: bool) -> IronResult<()> {
        Ok(())
    }

    fn query_installed(&self) -> IronResult<Vec<InstalledPackage>> {
        Ok(Vec::new())
    }

    fn is_installed(&self, _package: &str) -> IronResult<bool> {
        Ok(false)
    }

    fn search(&self, _query: &str) -> IronResult<Vec<String>> {
        Ok(Vec::new())
    }

    fn info(&self, _package: &str) -> IronResult<Option<InstalledPackage>> {
        Ok(None)
    }

    fn sync_database(&self) -> IronResult<()> {
        Ok(())
    }

    fn upgrade(&self, _preview: bool) -> IronResult<UpdatePreview> {
        Ok(UpdatePreview::default())
    }
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
        reasons.push(format!(
            "{} packages to update - consider updating more frequently",
            updates.len()
        ));
        if risk < RiskLevel::Medium {
            risk = RiskLevel::Medium;
        }
    }

    (risk, reasons)
}
