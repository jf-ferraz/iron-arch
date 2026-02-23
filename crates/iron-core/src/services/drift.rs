//! Drift Service — Detect differences between declared and actual state
//!
//! F1-011: DriftService trait + DefaultDriftService
//! F1-012: Package drift detection
//! F1-013: Service drift detection
//! F1-014: Config drift detection (symlink + checksum)

use crate::IronResult;
use crate::actual_state::{ActualState, ManagedFileSpec, ManagedServiceSpec};
use crate::module::Module;
use crate::packages::PackageManager;
use crate::services::apply::{DesiredState, resolve_desired_state};
use crate::services::state::StateManager;
use crate::system_service::SystemService;
use crate::validation::expand_home;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ==========================================================================
// F1-011: DriftReport models
// ==========================================================================

/// Full drift report comparing declared vs actual state.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DriftReport {
    pub package_drift: Vec<PackageDrift>,
    pub service_drift: Vec<ServiceDrift>,
    pub config_drift: Vec<ConfigDrift>,
    pub summary: DriftSummary,
}

/// Package-level drift.
#[derive(Debug, Clone, Serialize)]
pub enum PackageDrift {
    /// Declared but not installed
    Missing { name: String },
    /// Previously installed by Iron but no longer declared
    Extra { name: String },
}

/// Service-level drift.
#[derive(Debug, Clone, Serialize)]
pub enum ServiceDrift {
    /// Declared but not enabled
    NotEnabled { name: String },
    /// Enabled by Iron but no longer declared
    ExtraEnabled { name: String },
}

/// Config/dotfile-level drift.
#[derive(Debug, Clone, Serialize)]
pub enum ConfigDrift {
    /// Dotfile symlink doesn't exist
    MissingSymlink { source: String, target: String },
    /// Symlink exists but is broken
    BrokenSymlink { target: String },
    /// Symlink points to wrong target
    WrongTarget {
        target: String,
        expected: String,
        actual: String,
    },
}

/// Summary counts for the drift report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DriftSummary {
    pub total_drifts: usize,
    pub packages_missing: usize,
    pub packages_extra: usize,
    pub configs_drifted: usize,
    pub services_drifted: usize,
}

impl DriftReport {
    /// Check if the system matches declared state perfectly.
    pub fn is_clean(&self) -> bool {
        self.package_drift.is_empty()
            && self.service_drift.is_empty()
            && self.config_drift.is_empty()
    }

    /// Compute summary from current drift data.
    pub fn compute_summary(&mut self) {
        let pkg_missing = self
            .package_drift
            .iter()
            .filter(|d| matches!(d, PackageDrift::Missing { .. }))
            .count();
        let pkg_extra = self
            .package_drift
            .iter()
            .filter(|d| matches!(d, PackageDrift::Extra { .. }))
            .count();
        self.summary = DriftSummary {
            total_drifts: self.package_drift.len()
                + self.service_drift.len()
                + self.config_drift.len(),
            packages_missing: pkg_missing,
            packages_extra: pkg_extra,
            configs_drifted: self.config_drift.len(),
            services_drifted: self.service_drift.len(),
        };
    }
}

// ==========================================================================
// F1-011: DriftService trait + implementation
// ==========================================================================

/// Service for detecting drift between declared and actual state.
pub trait DriftService {
    /// Detect all drift for a host.
    fn detect(&self, host_id: &str) -> IronResult<DriftReport>;
}

/// Default implementation.
pub struct DefaultDriftService {
    iron_root: PathBuf,
    state_manager: StateManager,
    package_manager: Arc<dyn PackageManager>,
    service_manager: Arc<dyn SystemService>,
}

impl DefaultDriftService {
    pub fn new(
        iron_root: &Path,
        state_manager: StateManager,
        package_manager: Arc<dyn PackageManager>,
        service_manager: Arc<dyn SystemService>,
    ) -> Self {
        Self {
            iron_root: iron_root.to_path_buf(),
            state_manager,
            package_manager,
            service_manager,
        }
    }
}

impl DriftService for DefaultDriftService {
    fn detect(&self, host_id: &str) -> IronResult<DriftReport> {
        // Load host and resolve desired state
        let host_svc = crate::services::host::DefaultHostService::new(&self.iron_root);
        use crate::services::host::HostService;
        let host = host_svc.load_host(host_id)?;

        if host.bundle.is_none() && host.profile.is_none() && host.extra_modules.is_empty() {
            // No desired state declared — nothing to drift from
            return Ok(DriftReport::default());
        }

        let desired = resolve_desired_state(&self.iron_root, &host)?;

        // F3-002b: Scan actual state once, pass to all detect methods
        let actual = self.scan_actual_state(&desired)?;

        // F1-012/013/014: Detect all drift categories
        let package_drift = self.detect_package_drift(&desired, &actual)?;
        let service_drift = self.detect_service_drift(&desired, &actual)?;
        let config_drift = self.detect_config_drift(&desired, &actual)?;

        let mut report = DriftReport {
            package_drift,
            service_drift,
            config_drift,
            summary: DriftSummary::default(),
        };
        report.compute_summary();
        Ok(report)
    }
}

impl DefaultDriftService {
    /// Build managed file/service specs from desired state and scan the system.
    fn scan_actual_state(&self, desired: &DesiredState) -> IronResult<ActualState> {
        let managed_services: Vec<ManagedServiceSpec> = desired
            .services
            .iter()
            .map(|s| ManagedServiceSpec { name: s.clone() })
            .collect();

        let managed_files: Vec<ManagedFileSpec> = desired
            .dotfiles
            .iter()
            .map(|d| ManagedFileSpec {
                target: expand_home(Path::new(&d.target))
                    .to_string_lossy()
                    .to_string(),
                expected_source: Some(d.source.clone()),
            })
            .collect();

        ActualState::scan(
            self.package_manager.as_ref(),
            self.service_manager.as_ref(),
            &managed_services,
            &managed_files,
        )
    }

    /// F1-012: Package drift — desired packages vs installed.
    ///
    /// F3-002b: Reads installed packages from `ActualState` instead of
    /// querying the package manager directly.
    fn detect_package_drift(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> IronResult<Vec<PackageDrift>> {
        let desired_pkgs: HashSet<String> = desired
            .packages
            .iter()
            .chain(desired.aur_packages.iter())
            .cloned()
            .collect();

        let mut drift = Vec::new();

        // Missing: desired but not installed
        for pkg in &desired_pkgs {
            if !actual.installed_packages.contains(pkg) {
                drift.push(PackageDrift::Missing { name: pkg.clone() });
            }
        }

        // Extra: packages that Iron installed (tracked in active modules' packages)
        // but are no longer in the desired set
        let iron_managed: HashSet<String> = self
            .state_manager
            .active_modules()
            .iter()
            .filter_map(|mid| {
                let mdir = self.iron_root.join("modules").join(mid);
                Module::load(&mdir).ok()
            })
            .flat_map(|m| m.packages.into_iter().chain(m.aur_packages))
            .collect();

        for pkg in &iron_managed {
            if !desired_pkgs.contains(pkg) && actual.installed_packages.contains(pkg) {
                drift.push(PackageDrift::Extra { name: pkg.clone() });
            }
        }

        Ok(drift)
    }

    /// F1-013: Service drift — desired services vs enabled.
    ///
    /// F3-002b: Reads service enabled state from `ActualState` instead of
    /// querying the service manager directly.
    fn detect_service_drift(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> IronResult<Vec<ServiceDrift>> {
        let mut drift = Vec::new();

        for service in &desired.services {
            let is_enabled = actual
                .services
                .iter()
                .find(|s| s.name == *service)
                .map(|s| s.enabled)
                .unwrap_or(false);

            if !is_enabled {
                drift.push(ServiceDrift::NotEnabled {
                    name: service.clone(),
                });
            }
        }

        Ok(drift)
    }

    /// F1-014: Config drift — dotfile symlinks.
    ///
    /// F3-002b: Reads file state from `ActualState` instead of querying
    /// the filesystem directly.
    fn detect_config_drift(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> IronResult<Vec<ConfigDrift>> {
        let mut drift = Vec::new();

        for dotfile in &desired.dotfiles {
            let target_expanded = expand_home(Path::new(&dotfile.target))
                .to_string_lossy()
                .to_string();

            // Look up this file in the actual state scan results
            let actual_file = actual
                .managed_files
                .iter()
                .find(|f| f.target == target_expanded);

            match actual_file {
                Some(af) => match af.file_type {
                    crate::actual_state::FileStateType::Missing => {
                        drift.push(ConfigDrift::MissingSymlink {
                            source: dotfile.source.clone(),
                            target: dotfile.target.clone(),
                        });
                    }
                    crate::actual_state::FileStateType::Symlink => {
                        // Find the expected source path
                        let expected_source = desired
                            .modules
                            .iter()
                            .find_map(|mid| {
                                let mdir = self.iron_root.join("modules").join(mid);
                                if let Ok(m) = Module::load(&mdir) {
                                    if m.dotfiles.iter().any(|d| d.target == dotfile.target) {
                                        Some(mdir.join(&dotfile.source))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| PathBuf::from(&dotfile.source));

                        match &af.symlink_target {
                            Some(actual_target) => {
                                let actual_path = PathBuf::from(actual_target);
                                if actual_path != expected_source {
                                    if !actual_path.exists() {
                                        drift.push(ConfigDrift::BrokenSymlink {
                                            target: dotfile.target.clone(),
                                        });
                                    } else {
                                        drift.push(ConfigDrift::WrongTarget {
                                            target: dotfile.target.clone(),
                                            expected: expected_source.display().to_string(),
                                            actual: actual_target.clone(),
                                        });
                                    }
                                }
                            }
                            None => {
                                drift.push(ConfigDrift::BrokenSymlink {
                                    target: dotfile.target.clone(),
                                });
                            }
                        }
                    }
                    // Regular file or directory at target — not a symlink,
                    // so the dotfile link is "missing"
                    _ => {}
                },
                // File not in scan results — treat as missing
                None => {
                    let target_path = expand_home(Path::new(&dotfile.target));
                    if !target_path.exists() && !target_path.is_symlink() {
                        drift.push(ConfigDrift::MissingSymlink {
                            source: dotfile.source.clone(),
                            target: dotfile.target.clone(),
                        });
                    }
                }
            }
        }

        Ok(drift)
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_report_is_clean() {
        let report = DriftReport::default();
        assert!(report.is_clean());
    }

    #[test]
    fn test_drift_report_not_clean() {
        let mut report = DriftReport::default();
        report.package_drift.push(PackageDrift::Missing {
            name: "neovim".to_string(),
        });
        report.compute_summary();
        assert!(!report.is_clean());
        assert_eq!(report.summary.total_drifts, 1);
        assert_eq!(report.summary.packages_missing, 1);
    }

    #[test]
    fn test_drift_summary_counts() {
        let mut report = DriftReport {
            package_drift: vec![
                PackageDrift::Missing {
                    name: "a".to_string(),
                },
                PackageDrift::Extra {
                    name: "b".to_string(),
                },
            ],
            service_drift: vec![ServiceDrift::NotEnabled {
                name: "svc".to_string(),
            }],
            config_drift: vec![ConfigDrift::MissingSymlink {
                source: "s".to_string(),
                target: "t".to_string(),
            }],
            summary: DriftSummary::default(),
        };
        report.compute_summary();
        assert_eq!(report.summary.total_drifts, 4);
        assert_eq!(report.summary.packages_missing, 1);
        assert_eq!(report.summary.packages_extra, 1);
        assert_eq!(report.summary.services_drifted, 1);
        assert_eq!(report.summary.configs_drifted, 1);
    }

    #[test]
    fn test_config_drift_variants_serializable() {
        let drift = ConfigDrift::WrongTarget {
            target: "/home/user/.config/nvim".to_string(),
            expected: "/iron/modules/nvim/config".to_string(),
            actual: "/other/path".to_string(),
        };
        let json = serde_json::to_string(&drift).unwrap();
        assert!(json.contains("WrongTarget"));
    }
}
