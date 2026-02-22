//! System Scan Service — detects existing configs, packages, and conflicts
//!
//! Scans the user's system for existing dotfiles and installed packages,
//! then compares against bundle/module definitions to identify conflicts
//! and generate actionable recommendations.

use crate::packages::PackageManager;
use crate::{Bundle, IronResult, Module};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// =========================================================================
// Models (S1-P1.5-002)
// =========================================================================

/// A report produced by scanning the system for existing state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanReport {
    /// Existing config files/directories discovered on the system
    pub existing_configs: Vec<DiscoveredConfig>,
    /// Packages already installed that overlap with bundle/module definitions
    pub installed_packages: Vec<String>,
    /// Potential conflicts between existing state and managed configs
    pub potential_conflicts: Vec<ScanConflict>,
    /// Human-readable recommendations based on scan results
    pub recommendations: Vec<String>,
    /// Summary statistics
    pub summary: ScanSummary,
}

/// A configuration file or directory discovered during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredConfig {
    /// Absolute path on disk
    pub path: PathBuf,
    /// What application/tool this config belongs to (e.g. "nvim", "kitty")
    pub app_name: String,
    /// Whether this path is already a symlink (managed by iron or another tool)
    pub is_symlink: bool,
    /// If a symlink, where it points
    pub symlink_target: Option<PathBuf>,
}

/// A conflict between existing system state and a managed resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConflict {
    /// Path that conflicts
    pub path: PathBuf,
    /// Which bundle or module owns this path
    pub managed_by: String,
    /// Description of the conflict
    pub description: String,
    /// Severity of the conflict
    pub severity: ConflictSeverity,
}

/// How severe a scan conflict is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    /// Informational — existing config can be safely replaced or backed up
    Info,
    /// Warning — existing config may contain custom changes worth preserving
    Warning,
    /// Error — cannot proceed without resolution (e.g. non-symlink blocking path)
    Error,
}

/// Summary statistics for a scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanSummary {
    /// Number of config paths scanned
    pub configs_scanned: usize,
    /// Number of managed packages already installed
    pub packages_already_installed: usize,
    /// Number of conflicts found
    pub conflicts_found: usize,
    /// Number of recommendations
    pub recommendations_count: usize,
}

// =========================================================================
// Well-known config paths to scan
// =========================================================================

/// Well-known XDG config directory names and their application association.
const KNOWN_CONFIGS: &[(&str, &str)] = &[
    ("nvim", "Neovim"),
    ("kitty", "Kitty terminal"),
    ("alacritty", "Alacritty terminal"),
    ("hypr", "Hyprland"),
    ("niri", "Niri compositor"),
    ("waybar", "Waybar"),
    ("sway", "Sway"),
    ("fish", "Fish shell"),
    ("starship.toml", "Starship prompt"),
    ("tmux", "Tmux"),
    ("rofi", "Rofi launcher"),
    ("wofi", "Wofi launcher"),
    ("dunst", "Dunst notifications"),
    ("mako", "Mako notifications"),
    ("foot", "Foot terminal"),
    ("wezterm", "WezTerm"),
    ("zsh", "Zsh shell"),
    ("i3", "i3 window manager"),
    ("polybar", "Polybar"),
    ("picom", "Picom compositor"),
    ("gtk-3.0", "GTK3"),
    ("gtk-4.0", "GTK4"),
];

/// Well-known dotfiles in $HOME.
const KNOWN_HOME_DOTFILES: &[(&str, &str)] = &[
    (".bashrc", "Bash shell"),
    (".zshrc", "Zsh shell"),
    (".tmux.conf", "Tmux"),
    (".gitconfig", "Git"),
    (".vimrc", "Vim"),
];

// =========================================================================
// Service trait (S1-P1.5-001)
// =========================================================================

/// System scan service — discovers existing system state.
pub trait ScanService {
    /// Run a full system scan, comparing against the provided bundles and modules.
    fn scan(
        &self,
        bundles: &[Bundle],
        modules: &[Module],
    ) -> IronResult<ScanReport>;
}

/// Default implementation of `ScanService`.
pub struct DefaultScanService {
    /// The user's home directory (for scanning configs)
    home_dir: PathBuf,
    /// Package manager for checking installed packages
    package_manager: Arc<dyn PackageManager>,
}

impl DefaultScanService {
    /// Create a new scan service.
    pub fn new(home_dir: &Path, package_manager: Arc<dyn PackageManager>) -> Self {
        Self {
            home_dir: home_dir.to_path_buf(),
            package_manager,
        }
    }

    /// Discover existing config files in `$HOME/.config` and `$HOME`.
    fn discover_configs(&self) -> Vec<DiscoveredConfig> {
        let mut configs = Vec::new();
        let xdg_config = self.home_dir.join(".config");

        // Scan XDG config dir
        if xdg_config.is_dir() {
            for &(name, app) in KNOWN_CONFIGS {
                let path = xdg_config.join(name);
                if path.exists() {
                    let (is_symlink, symlink_target) = Self::check_symlink(&path);
                    configs.push(DiscoveredConfig {
                        path,
                        app_name: app.to_string(),
                        is_symlink,
                        symlink_target,
                    });
                }
            }
        }

        // Scan home dotfiles
        for &(name, app) in KNOWN_HOME_DOTFILES {
            let path = self.home_dir.join(name);
            if path.exists() {
                let (is_symlink, symlink_target) = Self::check_symlink(&path);
                configs.push(DiscoveredConfig {
                    path,
                    app_name: app.to_string(),
                    is_symlink,
                    symlink_target,
                });
            }
        }

        configs
    }

    /// Check whether a path is a symlink and resolve its target.
    fn check_symlink(path: &Path) -> (bool, Option<PathBuf>) {
        match std::fs::read_link(path) {
            Ok(target) => (true, Some(target)),
            Err(_) => (false, None),
        }
    }

    /// Find packages from bundle/module definitions that are already installed.
    fn find_installed_overlap(
        &self,
        bundles: &[Bundle],
        modules: &[Module],
    ) -> Vec<String> {
        // Collect all managed package names
        let mut managed: std::collections::HashSet<String> = std::collections::HashSet::new();
        for b in bundles {
            managed.extend(b.packages.iter().cloned());
            managed.extend(b.aur_packages.iter().cloned());
        }
        for m in modules {
            managed.extend(m.packages.iter().cloned());
            managed.extend(m.aur_packages.iter().cloned());
        }

        if managed.is_empty() {
            return Vec::new();
        }

        // Check which are already installed via pacman
        let mut already_installed = Vec::new();
        for pkg in &managed {
            if self.package_manager.is_installed(pkg).unwrap_or(false) {
                already_installed.push(pkg.clone());
            }
        }
        already_installed.sort();
        already_installed
    }

    /// Detect conflicts between discovered configs and module dotfile mappings.
    fn detect_conflicts(
        &self,
        configs: &[DiscoveredConfig],
        modules: &[Module],
    ) -> Vec<ScanConflict> {
        let mut conflicts = Vec::new();

        for module in modules {
            for dotfile in &module.dotfiles {
                let target = crate::expand_home(Path::new(&dotfile.target));
                // See if any discovered config overlaps
                for config in configs {
                    if config.path == target || target.starts_with(&config.path) {
                        let severity = if config.is_symlink {
                            ConflictSeverity::Info
                        } else {
                            ConflictSeverity::Warning
                        };
                        let description = if config.is_symlink {
                            format!(
                                "{} is a symlink (may already be managed)",
                                config.path.display()
                            )
                        } else {
                            format!(
                                "{} exists as a regular file/dir — will be overwritten by module '{}'",
                                config.path.display(),
                                module.id
                            )
                        };
                        conflicts.push(ScanConflict {
                            path: config.path.clone(),
                            managed_by: module.id.clone(),
                            description,
                            severity,
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// Generate recommendations from scan results.
    fn generate_recommendations(
        configs: &[DiscoveredConfig],
        conflicts: &[ScanConflict],
        installed_count: usize,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        let unmanaged_count = configs.iter().filter(|c| !c.is_symlink).count();
        if unmanaged_count > 0 {
            recs.push(format!(
                "Found {} unmanaged config(s) — consider backing up before activation",
                unmanaged_count
            ));
        }

        let error_conflicts = conflicts
            .iter()
            .filter(|c| c.severity == ConflictSeverity::Error)
            .count();
        if error_conflicts > 0 {
            recs.push(format!(
                "{} blocking conflict(s) must be resolved before activation",
                error_conflicts
            ));
        }

        let warning_conflicts = conflicts
            .iter()
            .filter(|c| c.severity == ConflictSeverity::Warning)
            .count();
        if warning_conflicts > 0 {
            recs.push(format!(
                "{} config(s) will be overwritten — review before proceeding",
                warning_conflicts
            ));
        }

        if installed_count > 0 {
            recs.push(format!(
                "{} managed package(s) already installed — activation will be faster",
                installed_count
            ));
        }

        if conflicts.is_empty() && unmanaged_count == 0 {
            recs.push("Clean system — ready for bundle activation".to_string());
        }

        recs
    }
}

impl ScanService for DefaultScanService {
    fn scan(
        &self,
        bundles: &[Bundle],
        modules: &[Module],
    ) -> IronResult<ScanReport> {
        let existing_configs = self.discover_configs();
        let installed_packages = self.find_installed_overlap(bundles, modules);
        let potential_conflicts = self.detect_conflicts(&existing_configs, modules);
        let recommendations = Self::generate_recommendations(
            &existing_configs,
            &potential_conflicts,
            installed_packages.len(),
        );

        let summary = ScanSummary {
            configs_scanned: existing_configs.len(),
            packages_already_installed: installed_packages.len(),
            conflicts_found: potential_conflicts.len(),
            recommendations_count: recommendations.len(),
        };

        Ok(ScanReport {
            existing_configs,
            installed_packages,
            potential_conflicts,
            recommendations,
            summary,
        })
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::DotfileMapping;
    use crate::packages::NoopPackageManager;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup_scan_service(home: &Path) -> DefaultScanService {
        DefaultScanService::new(home, Arc::new(NoopPackageManager))
    }

    // --- ScanReport model tests ---

    #[test]
    fn test_scan_report_default() {
        let report = ScanReport::default();
        assert!(report.existing_configs.is_empty());
        assert!(report.installed_packages.is_empty());
        assert!(report.potential_conflicts.is_empty());
        assert!(report.recommendations.is_empty());
        assert_eq!(report.summary.configs_scanned, 0);
    }

    #[test]
    fn test_scan_report_serialization() {
        let report = ScanReport {
            existing_configs: vec![DiscoveredConfig {
                path: PathBuf::from("/home/user/.config/nvim"),
                app_name: "Neovim".to_string(),
                is_symlink: false,
                symlink_target: None,
            }],
            installed_packages: vec!["neovim".to_string()],
            potential_conflicts: vec![],
            recommendations: vec!["Clean system".to_string()],
            summary: ScanSummary {
                configs_scanned: 1,
                packages_already_installed: 1,
                conflicts_found: 0,
                recommendations_count: 1,
            },
        };

        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ScanReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.existing_configs.len(), 1);
        assert_eq!(deserialized.installed_packages.len(), 1);
    }

    #[test]
    fn test_conflict_severity_values() {
        assert_ne!(ConflictSeverity::Info, ConflictSeverity::Warning);
        assert_ne!(ConflictSeverity::Warning, ConflictSeverity::Error);
    }

    // --- Config discovery tests ---

    #[test]
    fn test_discover_configs_finds_xdg_dirs() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".config");
        std::fs::create_dir_all(config_dir.join("nvim")).unwrap();
        std::fs::create_dir_all(config_dir.join("kitty")).unwrap();

        let svc = setup_scan_service(tmp.path());
        let configs = svc.discover_configs();

        assert!(configs.iter().any(|c| c.app_name == "Neovim"));
        assert!(configs.iter().any(|c| c.app_name == "Kitty terminal"));
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_discover_configs_finds_home_dotfiles() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bashrc"), "# bash config").unwrap();
        std::fs::write(tmp.path().join(".gitconfig"), "[user]").unwrap();

        let svc = setup_scan_service(tmp.path());
        let configs = svc.discover_configs();

        assert!(configs.iter().any(|c| c.app_name == "Bash shell"));
        assert!(configs.iter().any(|c| c.app_name == "Git"));
    }

    #[test]
    fn test_discover_configs_detects_symlinks() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".config");
        let target_dir = tmp.path().join("target_nvim");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&target_dir).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_dir, config_dir.join("nvim")).unwrap();

        let svc = setup_scan_service(tmp.path());
        let configs = svc.discover_configs();

        #[cfg(unix)]
        {
            let nvim = configs.iter().find(|c| c.app_name == "Neovim").unwrap();
            assert!(nvim.is_symlink);
            assert!(nvim.symlink_target.is_some());
        }
    }

    #[test]
    fn test_discover_configs_empty_home() {
        let tmp = TempDir::new().unwrap();
        let svc = setup_scan_service(tmp.path());
        let configs = svc.discover_configs();
        assert!(configs.is_empty());
    }

    // --- Conflict detection tests ---

    #[test]
    fn test_detect_conflicts_regular_file() {
        let tmp = TempDir::new().unwrap();
        let nvim_path = tmp.path().join(".config").join("nvim");
        std::fs::create_dir_all(&nvim_path).unwrap();

        let configs = vec![DiscoveredConfig {
            path: nvim_path.clone(),
            app_name: "Neovim".to_string(),
            is_symlink: false,
            symlink_target: None,
        }];

        let modules = vec![Module {
            id: "nvim-ide".to_string(),
            name: "Neovim IDE".to_string(),
            description: None,
            kind: crate::module::ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: nvim_path.to_string_lossy().to_string(),
                link: true,
            }],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
        }];

        let svc = setup_scan_service(tmp.path());
        let conflicts = svc.detect_conflicts(&configs, &modules);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Warning);
        assert_eq!(conflicts[0].managed_by, "nvim-ide");
    }

    #[test]
    fn test_detect_conflicts_symlink_is_info() {
        let configs = vec![DiscoveredConfig {
            path: PathBuf::from("/home/user/.config/nvim"),
            app_name: "Neovim".to_string(),
            is_symlink: true,
            symlink_target: Some(PathBuf::from("/some/target")),
        }];

        let modules = vec![Module {
            id: "nvim-ide".to_string(),
            name: "Neovim IDE".to_string(),
            description: None,
            kind: crate::module::ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "/home/user/.config/nvim".to_string(),
                link: true,
            }],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
        }];

        let svc = setup_scan_service(Path::new("/home/user"));
        let conflicts = svc.detect_conflicts(&configs, &modules);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Info);
    }

    #[test]
    fn test_detect_no_conflicts_when_no_overlap() {
        let configs = vec![DiscoveredConfig {
            path: PathBuf::from("/home/user/.config/kitty"),
            app_name: "Kitty terminal".to_string(),
            is_symlink: false,
            symlink_target: None,
        }];

        let modules = vec![Module {
            id: "nvim-ide".to_string(),
            name: "Neovim IDE".to_string(),
            description: None,
            kind: crate::module::ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: "/home/user/.config/nvim".to_string(),
                link: true,
            }],
            conflicts: vec![],
            depends: vec![],
            pre_install: None,
            post_install: None,
            pre_uninstall: None,
            status_check: None,
            priority: None,
            requires_root: false,
        }];

        let svc = setup_scan_service(Path::new("/home/user"));
        let conflicts = svc.detect_conflicts(&configs, &modules);
        assert!(conflicts.is_empty());
    }

    // --- Recommendation generation tests ---

    #[test]
    fn test_recommendations_clean_system() {
        let recs =
            DefaultScanService::generate_recommendations(&[], &[], 0);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Clean system"));
    }

    #[test]
    fn test_recommendations_unmanaged_configs() {
        let configs = vec![DiscoveredConfig {
            path: PathBuf::from("/home/user/.bashrc"),
            app_name: "Bash shell".to_string(),
            is_symlink: false,
            symlink_target: None,
        }];
        let recs = DefaultScanService::generate_recommendations(&configs, &[], 0);
        assert!(recs.iter().any(|r| r.contains("unmanaged")));
    }

    #[test]
    fn test_recommendations_installed_packages() {
        let recs = DefaultScanService::generate_recommendations(&[], &[], 5);
        assert!(recs.iter().any(|r| r.contains("already installed")));
    }

    // --- Full scan integration test ---

    #[test]
    fn test_full_scan_empty_system() {
        let tmp = TempDir::new().unwrap();
        let svc = setup_scan_service(tmp.path());
        let report = svc.scan(&[], &[]).unwrap();

        assert!(report.existing_configs.is_empty());
        assert!(report.installed_packages.is_empty());
        assert!(report.potential_conflicts.is_empty());
        assert_eq!(report.summary.configs_scanned, 0);
        assert!(report.recommendations.iter().any(|r| r.contains("Clean system")));
    }

    #[test]
    fn test_full_scan_with_existing_configs() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".config").join("nvim")).unwrap();
        std::fs::write(tmp.path().join(".bashrc"), "#!/bin/bash").unwrap();

        let svc = setup_scan_service(tmp.path());
        let report = svc.scan(&[], &[]).unwrap();

        assert_eq!(report.summary.configs_scanned, 2);
        assert!(report.recommendations.iter().any(|r| r.contains("unmanaged")));
    }
}
