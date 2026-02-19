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

    /// Fetch Arch Linux news from RSS feed
    ///
    /// Returns recent news items that may affect system updates.
    /// Default implementation returns empty list (for testing/offline).
    fn fetch_news(&self) -> IronResult<Vec<ArchNewsItem>> {
        Ok(Vec::new())
    }

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
///
/// Evaluates package updates and Arch news to determine update risk level.
///
/// # Examples
///
/// ```
/// use iron_core::{assess_risk, PackageUpdate, ArchNewsItem, RiskLevel};
///
/// // Low risk - regular package update
/// let updates = vec![PackageUpdate {
///     name: "ripgrep".to_string(),
///     current_version: "14.0.0".to_string(),
///     new_version: "14.1.0".to_string(),
///     ..Default::default()
/// }];
/// let (risk, _reasons) = assess_risk(&updates, &[]);
/// assert_eq!(risk, RiskLevel::Low);
///
/// // Higher risk - kernel update
/// let updates = vec![PackageUpdate {
///     name: "linux".to_string(),
///     current_version: "6.17.0".to_string(),
///     new_version: "6.18.0".to_string(),
///     ..Default::default()
/// }];
/// let (risk, reasons) = assess_risk(&updates, &[]);
/// assert!(risk >= RiskLevel::Medium);
/// assert!(!reasons.is_empty());
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_package_update(name: &str, is_aur: bool, is_flagged: bool) -> PackageUpdate {
        PackageUpdate {
            name: name.to_string(),
            current_version: "1.0.0".to_string(),
            new_version: "1.1.0".to_string(),
            is_aur,
            is_flagged,
            repository: if is_aur {
                "aur".to_string()
            } else {
                "extra".to_string()
            },
        }
    }

    fn create_news_item(title: &str, requires_manual: bool) -> ArchNewsItem {
        ArchNewsItem {
            title: title.to_string(),
            date: "2025-02-13".to_string(),
            url: "https://archlinux.org/news/test".to_string(),
            description: "Test news item".to_string(),
            requires_manual,
        }
    }

    // Risk Level Tests
    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_default() {
        let risk = RiskLevel::default();
        assert_eq!(risk, RiskLevel::Low);
    }

    #[test]
    fn test_risk_level_descriptions() {
        assert_eq!(RiskLevel::Low.description(), "Safe to update");
        assert_eq!(RiskLevel::Medium.description(), "Review recommended");
        assert_eq!(RiskLevel::High.description(), "Attention required");
        assert_eq!(
            RiskLevel::Critical.description(),
            "Create snapshot before updating"
        );
    }

    #[test]
    fn test_risk_level_equality() {
        assert_eq!(RiskLevel::Low, RiskLevel::Low);
        assert_ne!(RiskLevel::Low, RiskLevel::High);
    }

    // UpdatePreview Tests
    #[test]
    fn test_update_preview_default() {
        let preview = UpdatePreview::default();

        assert!(preview.packages.is_empty());
        assert!(preview.arch_news.is_empty());
        assert_eq!(preview.risk_level, RiskLevel::Low);
        assert!(preview.risk_reasons.is_empty());
        assert_eq!(preview.download_size, 0);
        assert_eq!(preview.install_size_delta, 0);
    }

    // PackageUpdate Tests
    #[test]
    fn test_package_update_default() {
        let update = PackageUpdate::default();

        assert!(update.name.is_empty());
        assert!(update.current_version.is_empty());
        assert!(!update.is_aur);
        assert!(!update.is_flagged);
    }

    #[test]
    fn test_package_update_creation() {
        let update = create_package_update("neovim", false, false);

        assert_eq!(update.name, "neovim");
        assert_eq!(update.current_version, "1.0.0");
        assert_eq!(update.new_version, "1.1.0");
        assert!(!update.is_aur);
    }

    // InstalledPackage Tests
    #[test]
    fn test_installed_package_default() {
        let pkg = InstalledPackage::default();

        assert!(pkg.name.is_empty());
        assert!(!pkg.explicit);
        assert!(!pkg.is_aur);
        assert_eq!(pkg.size, 0);
    }

    // ArchNewsItem Tests
    #[test]
    fn test_arch_news_item_default() {
        let news = ArchNewsItem::default();

        assert!(news.title.is_empty());
        assert!(!news.requires_manual);
    }

    // assess_risk Tests
    #[test]
    fn test_assess_risk_empty() {
        let (risk, reasons) = assess_risk(&[], &[]);

        assert_eq!(risk, RiskLevel::Low);
        assert!(reasons.is_empty());
    }

    #[test]
    fn test_assess_risk_normal_packages() {
        let updates = vec![
            create_package_update("neovim", false, false),
            create_package_update("ripgrep", false, false),
        ];

        let (risk, reasons) = assess_risk(&updates, &[]);

        assert_eq!(risk, RiskLevel::Low);
        assert!(reasons.is_empty());
    }

    #[test]
    fn test_assess_risk_kernel_update() {
        let updates = vec![
            create_package_update("linux", false, false),
            create_package_update("linux-headers", false, false),
        ];

        let (risk, reasons) = assess_risk(&updates, &[]);

        // linux (not linux-headers) triggers kernel update detection
        assert!(risk >= RiskLevel::Medium);
        assert!(reasons.iter().any(|r| r.contains("Kernel update")));
    }

    #[test]
    fn test_assess_risk_critical_packages() {
        let critical_packages = ["systemd", "glibc", "grub", "mkinitcpio"];

        for pkg in critical_packages {
            let updates = vec![create_package_update(pkg, false, false)];
            let (risk, reasons) = assess_risk(&updates, &[]);

            assert!(
                risk >= RiskLevel::High,
                "Package {} should be High risk",
                pkg
            );
            assert!(reasons.iter().any(|r| r.contains("Critical package")));
        }
    }

    #[test]
    fn test_assess_risk_display_drivers() {
        let high_risk = ["nvidia", "nvidia-dkms", "mesa", "wayland"];

        for pkg in high_risk {
            let updates = vec![create_package_update(pkg, false, false)];
            let (risk, reasons) = assess_risk(&updates, &[]);

            assert!(
                risk >= RiskLevel::Medium,
                "Package {} should be at least Medium risk",
                pkg
            );
            assert!(reasons.iter().any(|r| r.contains("Display/driver")));
        }
    }

    #[test]
    fn test_assess_risk_flagged_packages() {
        let updates = vec![create_package_update("some-aur-pkg", true, true)];

        let (risk, reasons) = assess_risk(&updates, &[]);

        assert_eq!(risk, RiskLevel::High);
        assert!(reasons.iter().any(|r| r.contains("Flagged out-of-date")));
    }

    #[test]
    fn test_assess_risk_manual_intervention_news() {
        let updates = vec![create_package_update("pacman", false, false)];
        let news = vec![create_news_item(
            "Manual intervention required for pacman update",
            true,
        )];

        let (risk, reasons) = assess_risk(&updates, &news);

        assert_eq!(risk, RiskLevel::Critical);
        assert!(reasons.iter().any(|r| r.contains("Manual intervention")));
    }

    #[test]
    fn test_assess_risk_many_packages() {
        let updates: Vec<_> = (0..101)
            .map(|i| create_package_update(&format!("pkg-{}", i), false, false))
            .collect();

        let (risk, reasons) = assess_risk(&updates, &[]);

        assert!(risk >= RiskLevel::Medium);
        assert!(reasons.iter().any(|r| r.contains("101 packages")));
    }

    #[test]
    fn test_assess_risk_combined_risks() {
        let updates = vec![
            create_package_update("linux", false, false),
            create_package_update("nvidia", false, false),
            create_package_update("systemd", false, false),
        ];
        let news = vec![create_news_item("Critical update", true)];

        let (risk, reasons) = assess_risk(&updates, &news);

        // Should be Critical due to manual intervention news
        assert_eq!(risk, RiskLevel::Critical);
        // Should have multiple reasons
        assert!(reasons.len() >= 3);
    }

    // NoopPackageManager Tests
    #[test]
    fn test_noop_package_manager_check_updates() {
        let pm = NoopPackageManager;
        let updates = pm.check_updates().unwrap();
        assert!(updates.is_empty());
    }

    #[test]
    fn test_noop_package_manager_install() {
        let pm = NoopPackageManager;
        let result = pm.install(&["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_package_manager_remove() {
        let pm = NoopPackageManager;
        let result = pm.remove(&["test".to_string()], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_package_manager_query() {
        let pm = NoopPackageManager;
        let packages = pm.query_installed().unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_noop_package_manager_is_installed() {
        let pm = NoopPackageManager;
        assert!(!pm.is_installed("anything").unwrap());
    }

    #[test]
    fn test_noop_package_manager_search() {
        let pm = NoopPackageManager;
        let results = pm.search("test").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_noop_package_manager_info() {
        let pm = NoopPackageManager;
        let info = pm.info("test").unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn test_noop_package_manager_sync() {
        let pm = NoopPackageManager;
        assert!(pm.sync_database().is_ok());
    }

    #[test]
    fn test_noop_package_manager_upgrade() {
        let pm = NoopPackageManager;
        let preview = pm.upgrade(true).unwrap();
        assert!(preview.packages.is_empty());
    }

    #[test]
    fn test_noop_package_manager_installed_count() {
        let pm = NoopPackageManager;
        assert_eq!(pm.installed_count().unwrap(), 0);
    }
}

/// Property-based tests using proptest
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating package names
    fn package_name_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,30}").unwrap()
    }

    // Strategy for generating version strings
    fn version_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[0-9]+\\.[0-9]+\\.[0-9]+").unwrap()
    }

    // Strategy for generating PackageUpdate
    fn package_update_strategy() -> impl Strategy<Value = PackageUpdate> {
        (
            package_name_strategy(),
            version_strategy(),
            version_strategy(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(name, current, new, is_aur, is_flagged)| PackageUpdate {
                name,
                current_version: current,
                new_version: new,
                is_aur,
                is_flagged,
                repository: if is_aur {
                    "aur".to_string()
                } else {
                    "extra".to_string()
                },
            })
    }

    proptest! {
        // Property: RiskLevel ordering is transitive
        #[test]
        fn risk_level_ordering_transitive(a in 0u8..4, b in 0u8..4, c in 0u8..4) {
            let levels = [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical];
            let a = levels[a as usize % 4];
            let b = levels[b as usize % 4];
            let c = levels[c as usize % 4];

            if a <= b && b <= c {
                prop_assert!(a <= c, "Transitivity violated: {:?} <= {:?} <= {:?} but {:?} > {:?}", a, b, c, a, c);
            }
        }

        // Property: RiskLevel is reflexive
        #[test]
        fn risk_level_ordering_reflexive(idx in 0u8..4) {
            let levels = [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical];
            let level = levels[idx as usize % 4];
            prop_assert!(level == level, "Reflexivity violated for {:?}", level);
            prop_assert!(level <= level, "Reflexivity violated for <= on {:?}", level);
        }

        // Property: Adding any package doesn't decrease risk level
        #[test]
        fn assess_risk_monotonic_on_package_addition(
            base_updates in prop::collection::vec(package_update_strategy(), 0..10),
            extra_update in package_update_strategy()
        ) {
            let (base_risk, _) = assess_risk(&base_updates, &[]);

            let mut extended = base_updates.clone();
            extended.push(extra_update);
            let (extended_risk, _) = assess_risk(&extended, &[]);

            prop_assert!(
                extended_risk >= base_risk,
                "Risk should not decrease when adding packages: {:?} < {:?}",
                extended_risk,
                base_risk
            );
        }

        // Property: Adding manual intervention news always results in Critical
        #[test]
        fn assess_risk_manual_intervention_always_critical(
            updates in prop::collection::vec(package_update_strategy(), 0..10),
            title in "[A-Za-z ]{1,50}"
        ) {
            let news = vec![ArchNewsItem {
                title,
                date: "2025-02-13".to_string(),
                url: "https://archlinux.org/news/test".to_string(),
                description: "Test".to_string(),
                requires_manual: true,
            }];

            let (risk, _) = assess_risk(&updates, &news);
            prop_assert_eq!(risk, RiskLevel::Critical, "Manual intervention news should always result in Critical risk");
        }

        // Property: Empty updates and news always result in Low risk
        #[test]
        fn assess_risk_empty_is_low(_dummy in 0..100u8) {
            let (risk, reasons) = assess_risk(&[], &[]);
            prop_assert_eq!(risk, RiskLevel::Low);
            prop_assert!(reasons.is_empty());
        }

        // Property: PackageUpdate serialization roundtrip
        #[test]
        fn package_update_serialization_roundtrip(update in package_update_strategy()) {
            let serialized = serde_json::to_string(&update).unwrap();
            let deserialized: PackageUpdate = serde_json::from_str(&serialized).unwrap();

            prop_assert_eq!(update.name, deserialized.name);
            prop_assert_eq!(update.current_version, deserialized.current_version);
            prop_assert_eq!(update.new_version, deserialized.new_version);
            prop_assert_eq!(update.is_aur, deserialized.is_aur);
            prop_assert_eq!(update.is_flagged, deserialized.is_flagged);
        }

        // Property: Risk level description is never empty
        #[test]
        fn risk_level_description_non_empty(idx in 0u8..4) {
            let levels = [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical];
            let level = levels[idx as usize % 4];
            prop_assert!(!level.description().is_empty());
        }

        // Property: Kernel updates always trigger at least Medium risk
        #[test]
        fn kernel_update_triggers_medium_risk(suffix in "(|-(lts|zen|hardened))") {
            let name = format!("linux{}", suffix);
            // Skip linux-headers which is explicitly excluded
            if !name.contains("headers") {
                let updates = vec![PackageUpdate {
                    name,
                    current_version: "6.0.0".to_string(),
                    new_version: "6.1.0".to_string(),
                    is_aur: false,
                    is_flagged: false,
                    repository: "core".to_string(),
                }];

                let (risk, reasons) = assess_risk(&updates, &[]);
                prop_assert!(risk >= RiskLevel::Medium, "Kernel update should trigger at least Medium risk");
                prop_assert!(reasons.iter().any(|r| r.contains("Kernel") || r.contains("Critical")));
            }
        }

        // Property: More than 100 updates triggers at least Medium risk
        #[test]
        fn many_updates_trigger_medium_risk(count in 101usize..200) {
            let updates: Vec<_> = (0..count)
                .map(|i| PackageUpdate {
                    name: format!("pkg-{}", i),
                    current_version: "1.0.0".to_string(),
                    new_version: "1.1.0".to_string(),
                    is_aur: false,
                    is_flagged: false,
                    repository: "extra".to_string(),
                })
                .collect();

            let (risk, reasons) = assess_risk(&updates, &[]);
            prop_assert!(risk >= RiskLevel::Medium);
            prop_assert!(reasons.iter().any(|r| r.contains("packages to update")));
        }

        // Property: Flagged packages always trigger High risk
        #[test]
        fn flagged_package_triggers_high_risk(name in package_name_strategy()) {
            let updates = vec![PackageUpdate {
                name,
                current_version: "1.0.0".to_string(),
                new_version: "1.1.0".to_string(),
                is_aur: true,
                is_flagged: true,
                repository: "aur".to_string(),
            }];

            let (risk, reasons) = assess_risk(&updates, &[]);
            prop_assert!(risk >= RiskLevel::High, "Flagged package should trigger at least High risk");
            prop_assert!(reasons.iter().any(|r| r.contains("Flagged")));
        }
    }
}
