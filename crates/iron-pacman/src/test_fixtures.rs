//! Test fixtures for mocking pacman and AUR helper commands.
//!
//! This module provides pre-configured mock responses for common pacman commands,
//! enabling comprehensive testing of `DefaultPackageManager` without requiring
//! actual pacman execution or root privileges.
//!
//! # Usage
//!
//! ```rust,ignore
//! use iron_core::resilience::{MockCommandExecutor, CommandConfig};
//! use iron_pacman::test_fixtures::PacmanMockBuilder;
//!
//! let executor = PacmanMockBuilder::new()
//!     .with_installed_packages(&[("hyprland", "0.40.0-1"), ("waybar", "0.10.0-1")])
//!     .with_updates(&[("hyprland", "0.40.0-1", "0.41.0-1")])
//!     .build();
//!
//! let pm = DefaultPackageManager::with_executor(Arc::new(executor));
//! let packages = pm.query_installed().unwrap();
//! assert!(!packages.is_empty());
//! ```

use iron_core::resilience::{MockCommandExecutor, MockResponse};
use std::collections::HashMap;

/// Builder for creating configured `MockCommandExecutor` with pacman-specific responses.
///
/// Provides a fluent API for setting up mock responses for pacman commands,
/// enabling isolated testing of package management operations.
#[derive(Debug, Default)]
pub struct PacmanMockBuilder {
    /// Packages to return for `pacman -Q` (all installed)
    installed_packages: Vec<(String, String)>,
    /// Packages to return for `pacman -Qe` (explicitly installed)
    explicit_packages: Vec<String>,
    /// Package info responses keyed by package name
    package_info: HashMap<String, PackageInfoFixture>,
    /// Updates to return for `checkupdates`
    pending_updates: Vec<(String, String, String)>,
    /// AUR updates to return for `paru -Qua` / `yay -Qua`
    aur_updates: Vec<(String, String, String)>,
    /// Search results keyed by query
    search_results: HashMap<String, Vec<SearchResultFixture>>,
    /// Orphan packages
    orphans: Vec<String>,
    /// Whether database sync should succeed
    sync_succeeds: bool,
    /// Whether upgrade should succeed
    upgrade_succeeds: bool,
    /// Whether install should succeed
    install_succeeds: bool,
    /// Whether remove should succeed
    remove_succeeds: bool,
    /// Simulated AUR helper (for aur helper command responses)
    aur_helper: Option<String>,
}

/// Fixture data for package info (`pacman -Qi` output)
#[derive(Debug, Clone)]
pub struct PackageInfoFixture {
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed_size: String,
    pub install_reason: String,
    pub architecture: String,
    pub url: String,
    pub licenses: String,
}

impl Default for PackageInfoFixture {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: String::new(),
            installed_size: "1.0 MiB".to_string(),
            install_reason: "Explicitly installed".to_string(),
            architecture: "x86_64".to_string(),
            url: String::new(),
            licenses: "MIT".to_string(),
        }
    }
}

impl PackageInfoFixture {
    /// Create a new package info fixture
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("{} package", name),
            ..Default::default()
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Set the installed size
    pub fn with_size(mut self, size: &str) -> Self {
        self.installed_size = size.to_string();
        self
    }

    /// Set as dependency (not explicitly installed)
    pub fn as_dependency(mut self) -> Self {
        self.install_reason = "Installed as a dependency for another package".to_string();
        self
    }

    /// Format as pacman -Qi output
    pub fn to_pacman_qi_output(&self) -> String {
        format!(
            "Name            : {}\n\
             Version         : {}\n\
             Description     : {}\n\
             Architecture    : {}\n\
             URL             : {}\n\
             Licenses        : {}\n\
             Groups          : None\n\
             Provides        : None\n\
             Depends On      : glibc\n\
             Optional Deps   : None\n\
             Required By     : None\n\
             Optional For    : None\n\
             Conflicts With  : None\n\
             Replaces        : None\n\
             Installed Size  : {}\n\
             Packager        : Arch Linux Team\n\
             Build Date      : Mon 01 Jan 2024 12:00:00 AM UTC\n\
             Install Date    : Mon 01 Jan 2024 01:00:00 AM UTC\n\
             Install Reason  : {}\n\
             Install Script  : No\n\
             Validated By    : Signature\n",
            self.name,
            self.version,
            self.description,
            self.architecture,
            self.url,
            self.licenses,
            self.installed_size,
            self.install_reason
        )
    }
}

/// Fixture data for search results (`pacman -Ss` output)
#[derive(Debug, Clone)]
pub struct SearchResultFixture {
    pub repository: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

impl SearchResultFixture {
    /// Create a new search result fixture
    pub fn new(repo: &str, name: &str, version: &str, desc: &str) -> Self {
        Self {
            repository: repo.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: desc.to_string(),
            installed: false,
        }
    }

    /// Mark as installed
    pub fn installed(mut self) -> Self {
        self.installed = true;
        self
    }

    /// Format as a single search result line
    pub fn to_pacman_ss_output(&self) -> String {
        let installed_marker = if self.installed { " [installed]" } else { "" };
        format!(
            "{}/{} {}{}\n    {}",
            self.repository, self.name, self.version, installed_marker, self.description
        )
    }
}

impl PacmanMockBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            sync_succeeds: true,
            upgrade_succeeds: true,
            install_succeeds: true,
            remove_succeeds: true,
            ..Default::default()
        }
    }

    /// Add installed packages (for `pacman -Q` and `pacman -Qe`)
    ///
    /// All packages added here are treated as explicitly installed.
    pub fn with_installed_packages(mut self, packages: &[(&str, &str)]) -> Self {
        for (name, version) in packages {
            self.installed_packages
                .push((name.to_string(), version.to_string()));
            self.explicit_packages.push(name.to_string());
        }
        self
    }

    /// Add dependency packages (appear in `pacman -Q` but not `pacman -Qe`)
    pub fn with_dependency_packages(mut self, packages: &[(&str, &str)]) -> Self {
        for (name, version) in packages {
            self.installed_packages
                .push((name.to_string(), version.to_string()));
            // Not added to explicit_packages
        }
        self
    }

    /// Add package info for specific packages
    pub fn with_package_info(mut self, info: PackageInfoFixture) -> Self {
        self.package_info.insert(info.name.clone(), info);
        self
    }

    /// Add pending updates (for `checkupdates`)
    pub fn with_updates(mut self, updates: &[(&str, &str, &str)]) -> Self {
        for (name, current, new) in updates {
            self.pending_updates
                .push((name.to_string(), current.to_string(), new.to_string()));
        }
        self
    }

    /// Add AUR updates (for `paru -Qua` / `yay -Qua`)
    pub fn with_aur_updates(mut self, updates: &[(&str, &str, &str)]) -> Self {
        for (name, current, new) in updates {
            self.aur_updates
                .push((name.to_string(), current.to_string(), new.to_string()));
        }
        self
    }

    /// Add search results for a query
    pub fn with_search_results(mut self, query: &str, results: Vec<SearchResultFixture>) -> Self {
        self.search_results.insert(query.to_string(), results);
        self
    }

    /// Add orphan packages
    pub fn with_orphans(mut self, orphans: &[&str]) -> Self {
        self.orphans = orphans.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set whether database sync should succeed
    pub fn sync_succeeds(mut self, succeeds: bool) -> Self {
        self.sync_succeeds = succeeds;
        self
    }

    /// Set whether upgrade should succeed
    pub fn upgrade_succeeds(mut self, succeeds: bool) -> Self {
        self.upgrade_succeeds = succeeds;
        self
    }

    /// Set whether install should succeed
    pub fn install_succeeds(mut self, succeeds: bool) -> Self {
        self.install_succeeds = succeeds;
        self
    }

    /// Set whether remove should succeed
    pub fn remove_succeeds(mut self, succeeds: bool) -> Self {
        self.remove_succeeds = succeeds;
        self
    }

    /// Set the AUR helper to simulate
    pub fn with_aur_helper(mut self, helper: &str) -> Self {
        self.aur_helper = Some(helper.to_string());
        self
    }

    /// Build the configured `MockCommandExecutor`
    pub fn build(self) -> MockCommandExecutor {
        let executor = MockCommandExecutor::new();

        // Configure pacman -Q (all installed packages)
        let q_output = self
            .installed_packages
            .iter()
            .map(|(name, version)| format!("{} {}", name, version))
            .collect::<Vec<_>>()
            .join("\n");
        executor.add_response("pacman", &["-Q"], MockResponse::success(&q_output));

        // Configure pacman -Qe (explicitly installed)
        let qe_output = self
            .installed_packages
            .iter()
            .filter(|(name, _)| self.explicit_packages.contains(name))
            .map(|(name, version)| format!("{} {}", name, version))
            .collect::<Vec<_>>()
            .join("\n");
        executor.add_response("pacman", &["-Qe"], MockResponse::success(&qe_output));

        // Configure pacman -Qi <package> for each package info
        for (name, info) in &self.package_info {
            executor.add_response(
                "pacman",
                &["-Qi", name],
                MockResponse::success(&info.to_pacman_qi_output()),
            );
        }

        // Configure checkupdates
        let updates_output = self
            .pending_updates
            .iter()
            .map(|(name, current, new)| format!("{} {} -> {}", name, current, new))
            .collect::<Vec<_>>()
            .join("\n");
        executor.add_response(
            "checkupdates",
            &[],
            MockResponse::success(&updates_output),
        );

        // Configure AUR updates for both paru and yay
        let aur_output = self
            .aur_updates
            .iter()
            .map(|(name, current, new)| format!("{} {} -> {}", name, current, new))
            .collect::<Vec<_>>()
            .join("\n");

        for helper in &["paru", "yay"] {
            executor.add_response(helper, &["-Qua"], MockResponse::success(&aur_output));
        }

        // Configure search results
        for (query, results) in &self.search_results {
            let search_output = results
                .iter()
                .map(|r| r.to_pacman_ss_output())
                .collect::<Vec<_>>()
                .join("\n");

            executor.add_response("pacman", &["-Ss", query], MockResponse::success(&search_output));

            // Also configure for AUR helpers
            for helper in &["paru", "yay", "pikaur", "trizen"] {
                executor.add_response(
                    helper,
                    &["-Ss", query],
                    MockResponse::success(&search_output),
                );
            }
        }

        // Configure pacman -Qtdq (orphans)
        let orphans_output = self.orphans.join("\n");
        executor.add_response(
            "pacman",
            &["-Qtdq"],
            if self.orphans.is_empty() {
                MockResponse::exit_error(1, "")
            } else {
                MockResponse::success(&orphans_output)
            },
        );

        // Configure pacman -Sy (database sync)
        if self.sync_succeeds {
            executor.add_response("pacman", &["-Sy"], MockResponse::success(""));
        } else {
            executor.add_response(
                "pacman",
                &["-Sy"],
                MockResponse::exit_error(1, "error: failed to synchronize all databases"),
            );
        }

        // Configure upgrade commands
        for helper in &["pacman", "paru", "yay", "pikaur", "trizen"] {
            if self.upgrade_succeeds {
                executor.add_response(
                    helper,
                    &["-Syu", "--noconfirm"],
                    MockResponse::success(""),
                );
            } else {
                executor.add_response(
                    helper,
                    &["-Syu", "--noconfirm"],
                    MockResponse::exit_error(1, "error: failed to upgrade"),
                );
            }
        }

        // Configure which command for AUR helper detection
        if let Some(ref helper) = self.aur_helper {
            executor.add_response("which", &[helper], MockResponse::success(helper));
            // Set other helpers as not found
            for other in &["paru", "yay", "pikaur", "trizen"] {
                if *other != helper {
                    executor.add_response(
                        "which",
                        &[*other],
                        MockResponse::exit_error(1, ""),
                    );
                }
            }
        }

        // Configure paccache
        executor.add_response(
            "paccache",
            &["-rk", "3"],
            MockResponse::success("==> finished: 5 packages removed"),
        );
        executor.add_response(
            "paccache",
            &["-rk", "1"],
            MockResponse::success("==> finished: 10 packages removed"),
        );

        // Add additional commands to the existing commands list
        executor.add_existing_command("checkupdates");
        executor.add_existing_command("paccache");
        executor.add_existing_command("paru");
        executor.add_existing_command("yay");
        executor.add_existing_command("pikaur");
        executor.add_existing_command("trizen");

        executor
    }
}

// =============================================================================
// Pre-built Fixture Sets
// =============================================================================

/// Common package sets for testing
pub mod fixtures {
    use super::*;

    /// Create a minimal Hyprland desktop fixture
    pub fn hyprland_desktop() -> PacmanMockBuilder {
        PacmanMockBuilder::new()
            .with_installed_packages(&[
                ("hyprland", "0.40.0-1"),
                ("waybar", "0.10.0-1"),
                ("wofi", "1.4-1"),
                ("kitty", "0.31.0-1"),
                ("mako", "1.8.0-1"),
            ])
            .with_dependency_packages(&[
                ("glibc", "2.39-1"),
                ("mesa", "24.0.1-1"),
                ("wayland", "1.22.0-1"),
            ])
            .with_package_info(
                PackageInfoFixture::new("hyprland", "0.40.0-1")
                    .with_description("A highly customizable dynamic tiling Wayland compositor")
                    .with_size("12.50 MiB"),
            )
            .with_package_info(
                PackageInfoFixture::new("waybar", "0.10.0-1")
                    .with_description("Highly customizable Wayland bar for Sway and Wlroots based compositors")
                    .with_size("2.50 MiB"),
            )
            .with_aur_helper("paru")
    }

    /// Create a fixture with pending updates
    pub fn pending_updates() -> PacmanMockBuilder {
        hyprland_desktop()
            .with_updates(&[
                ("hyprland", "0.40.0-1", "0.41.0-1"),
                ("linux", "6.7.0-1", "6.7.1-1"),
                ("systemd", "255-1", "255.1-1"),
            ])
            .with_aur_updates(&[("paru-bin", "2.0.0-1", "2.0.1-1")])
    }

    /// Create a fixture with critical updates (kernel + systemd)
    pub fn critical_updates() -> PacmanMockBuilder {
        hyprland_desktop().with_updates(&[
            ("linux", "6.6.0-1", "6.7.0-1"),
            ("systemd", "254-1", "255-1"),
            ("glibc", "2.38-1", "2.39-1"),
        ])
    }

    /// Create an empty system fixture
    pub fn empty_system() -> PacmanMockBuilder {
        PacmanMockBuilder::new()
    }

    /// Create a fixture with orphan packages
    pub fn with_orphans() -> PacmanMockBuilder {
        hyprland_desktop().with_orphans(&["unused-lib", "old-dependency", "removed-pkg-dep"])
    }

    /// Create a fixture with search results
    pub fn with_search_results() -> PacmanMockBuilder {
        hyprland_desktop().with_search_results(
            "hypr",
            vec![
                SearchResultFixture::new(
                    "extra",
                    "hyprland",
                    "0.40.0-1",
                    "A highly customizable dynamic tiling Wayland compositor",
                )
                .installed(),
                SearchResultFixture::new(
                    "extra",
                    "hyprpaper",
                    "0.6.0-1",
                    "A blazing fast Wayland wallpaper utility",
                ),
                SearchResultFixture::new(
                    "aur",
                    "hyprshot",
                    "1.0.0-1",
                    "Screenshot utility for Hyprland",
                ),
            ],
        )
    }

    /// Create a fixture simulating failure scenarios
    pub fn failing_operations() -> PacmanMockBuilder {
        hyprland_desktop()
            .sync_succeeds(false)
            .upgrade_succeeds(false)
            .install_succeeds(false)
            .remove_succeeds(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::resilience::CommandExecutor;

    #[test]
    fn test_builder_creates_executor() {
        let executor = PacmanMockBuilder::new()
            .with_installed_packages(&[("vim", "9.0-1")])
            .build();

        // Verify the executor was created
        assert!(executor.total_call_count() == 0);
    }

    #[test]
    fn test_package_info_fixture_format() {
        let info = PackageInfoFixture::new("hyprland", "0.40.0-1")
            .with_description("A compositor")
            .with_size("12.5 MiB");

        let output = info.to_pacman_qi_output();
        assert!(output.contains("Name            : hyprland"));
        assert!(output.contains("Version         : 0.40.0-1"));
        assert!(output.contains("Description     : A compositor"));
        assert!(output.contains("Installed Size  : 12.5 MiB"));
    }

    #[test]
    fn test_package_info_fixture_dependency() {
        let info = PackageInfoFixture::new("glibc", "2.39-1").as_dependency();

        let output = info.to_pacman_qi_output();
        assert!(output.contains("Install Reason  : Installed as a dependency"));
    }

    #[test]
    fn test_search_result_fixture_format() {
        let result =
            SearchResultFixture::new("extra", "hyprland", "0.40.0-1", "A compositor").installed();

        let output = result.to_pacman_ss_output();
        assert!(output.contains("extra/hyprland 0.40.0-1 [installed]"));
        assert!(output.contains("A compositor"));
    }

    #[test]
    fn test_search_result_not_installed() {
        let result = SearchResultFixture::new("aur", "hyprshot", "1.0.0-1", "Screenshot utility");

        let output = result.to_pacman_ss_output();
        assert!(output.contains("aur/hyprshot 1.0.0-1\n"));
        assert!(!output.contains("[installed]"));
    }

    #[test]
    fn test_builder_with_updates() {
        let executor = PacmanMockBuilder::new()
            .with_updates(&[("linux", "6.7.0-1", "6.7.1-1")])
            .build();

        let output = executor
            .execute("checkupdates", &[])
            .expect("should execute");
        assert!(output.contains("linux 6.7.0-1 -> 6.7.1-1"));
    }

    #[test]
    fn test_builder_with_aur_updates() {
        let executor = PacmanMockBuilder::new()
            .with_aur_updates(&[("paru-bin", "2.0.0-1", "2.0.1-1")])
            .build();

        let output = executor
            .execute("paru", &["-Qua"])
            .expect("should execute");
        assert!(output.contains("paru-bin 2.0.0-1 -> 2.0.1-1"));
    }

    #[test]
    fn test_builder_installed_vs_explicit() {
        let executor = PacmanMockBuilder::new()
            .with_installed_packages(&[("vim", "9.0-1")])
            .with_dependency_packages(&[("glibc", "2.39-1")])
            .build();

        // pacman -Q should show both
        let q_output = executor.execute("pacman", &["-Q"]).expect("should execute");
        assert!(q_output.contains("vim 9.0-1"));
        assert!(q_output.contains("glibc 2.39-1"));

        // pacman -Qe should only show vim
        let qe_output = executor
            .execute("pacman", &["-Qe"])
            .expect("should execute");
        assert!(qe_output.contains("vim 9.0-1"));
        assert!(!qe_output.contains("glibc"));
    }

    #[test]
    fn test_builder_orphans() {
        let executor = PacmanMockBuilder::new()
            .with_orphans(&["old-lib", "unused-pkg"])
            .build();

        let output = executor
            .execute("pacman", &["-Qtdq"])
            .expect("should execute");
        assert!(output.contains("old-lib"));
        assert!(output.contains("unused-pkg"));
    }

    #[test]
    fn test_builder_no_orphans() {
        let executor = PacmanMockBuilder::new().build();

        // No orphans should result in failure (exit code 1 with no output)
        let result = executor.execute("pacman", &["-Qtdq"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_sync_failure() {
        let executor = PacmanMockBuilder::new().sync_succeeds(false).build();

        let result = executor.execute("pacman", &["-Sy"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fixtures_hyprland_desktop() {
        let executor = fixtures::hyprland_desktop().build();

        let output = executor.execute("pacman", &["-Q"]).expect("should execute");
        assert!(output.contains("hyprland"));
        assert!(output.contains("waybar"));
        assert!(output.contains("glibc"));
    }

    #[test]
    fn test_fixtures_pending_updates() {
        let executor = fixtures::pending_updates().build();

        let output = executor
            .execute("checkupdates", &[])
            .expect("should execute");
        assert!(output.contains("hyprland"));
        assert!(output.contains("linux"));
    }

    #[test]
    fn test_fixtures_critical_updates() {
        let executor = fixtures::critical_updates().build();

        let output = executor
            .execute("checkupdates", &[])
            .expect("should execute");
        assert!(output.contains("linux"));
        assert!(output.contains("systemd"));
        assert!(output.contains("glibc"));
    }

    #[test]
    fn test_fixtures_with_search() {
        let executor = fixtures::with_search_results().build();

        let output = executor
            .execute("pacman", &["-Ss", "hypr"])
            .expect("should execute");
        assert!(output.contains("hyprland"));
        assert!(output.contains("hyprpaper"));
    }

    #[test]
    fn test_paccache_response() {
        let executor = PacmanMockBuilder::new().build();

        let output = executor
            .execute("paccache", &["-rk", "3"])
            .expect("should execute");
        assert!(output.contains("5 packages removed"));
    }

    #[test]
    fn test_package_info_response() {
        let executor = PacmanMockBuilder::new()
            .with_package_info(PackageInfoFixture::new("vim", "9.0-1").with_description("Vi IMproved"))
            .build();

        let output = executor
            .execute("pacman", &["-Qi", "vim"])
            .expect("should execute");
        assert!(output.contains("Name            : vim"));
        assert!(output.contains("Version         : 9.0-1"));
        assert!(output.contains("Vi IMproved"));
    }
}
