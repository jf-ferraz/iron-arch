//! Doctor Service — unified system health checks
//!
//! Provides a shared health-check service consumed by both TUI and CLI.
//! Implements FR-10.1 through FR-10.8 health diagnostics.

use crate::IronResult;
use crate::availability::ServiceAvailability;
use crate::services::host::HostService;
use crate::snapshot::SnapshotBackend;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

// =========================================================================
// Shared types (extracted from CLI doctor.rs)
// =========================================================================

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Check raised a warning
    Warn,
    /// Check failed
    Fail,
}

impl CheckStatus {
    /// String representation for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }
}

/// A single health check result.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    /// Machine-readable check name (e.g. "state_file", "git", "symlinks")
    pub name: String,
    /// Pass / Warn / Fail
    pub status: CheckStatus,
    /// Human-readable message
    pub message: String,
    /// Optional sub-items (e.g. list of missing packages)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
}

/// Aggregated health report.
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    /// Individual check results
    pub checks: Vec<HealthCheck>,
    /// Overall status (derived from worst individual status)
    pub overall: CheckStatus,
    /// ISO-8601 timestamp
    pub timestamp: String,
}

impl HealthReport {
    /// Count of checks with a given status.
    pub fn count(&self, status: CheckStatus) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    /// Number of errors.
    pub fn errors(&self) -> usize {
        self.count(CheckStatus::Fail)
    }

    /// Number of warnings.
    pub fn warnings(&self) -> usize {
        self.count(CheckStatus::Warn)
    }
}

// =========================================================================
// Service trait
// =========================================================================

/// Doctor service — runs health checks against the system.
pub trait DoctorService {
    /// Run all health checks and return a report.
    fn check_all(&self) -> IronResult<HealthReport>;
}

/// Configuration for the default doctor service.
pub struct DoctorConfig {
    /// Repository / config root directory
    pub root: PathBuf,
    /// Currently active host ID (from state)
    pub current_host: Option<String>,
    /// Currently active bundle ID (from state)
    pub active_bundle: Option<String>,
    /// Detected snapshot backend
    pub snapshot_backend: SnapshotBackend,
}

/// Default implementation of `DoctorService`.
pub struct DefaultDoctorService {
    config: DoctorConfig,
}

impl DefaultDoctorService {
    /// Create a new doctor service.
    pub fn new(config: DoctorConfig) -> Self {
        Self { config }
    }

    // --- Individual check implementations ---

    /// Check 1: State file validation (FR-10.1)
    fn check_state_file(&self) -> HealthCheck {
        let state_path = self.config.root.join("state.json");
        if !state_path.exists() {
            return HealthCheck {
                name: "state_file".to_string(),
                status: CheckStatus::Fail,
                message: "State file missing".to_string(),
                details: vec![],
            };
        }

        match std::fs::read_to_string(&state_path) {
            Ok(content) => {
                if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                    HealthCheck {
                        name: "state_file".to_string(),
                        status: CheckStatus::Pass,
                        message: "state.json valid".to_string(),
                        details: vec![],
                    }
                } else {
                    HealthCheck {
                        name: "state_file".to_string(),
                        status: CheckStatus::Fail,
                        message: "state.json invalid JSON".to_string(),
                        details: vec![],
                    }
                }
            }
            Err(_) => HealthCheck {
                name: "state_file".to_string(),
                status: CheckStatus::Fail,
                message: "state.json unreadable".to_string(),
                details: vec![],
            },
        }
    }

    /// Check 2: Directory structure (FR-10.5)
    fn check_directories(&self) -> HealthCheck {
        let dirs = ["modules", "profiles", "bundles", "hosts"];
        let mut missing = Vec::new();
        for dir in &dirs {
            if !self.config.root.join(dir).exists() {
                missing.push(dir.to_string());
            }
        }

        if missing.is_empty() {
            HealthCheck {
                name: "directories".to_string(),
                status: CheckStatus::Pass,
                message: "All directories exist".to_string(),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "directories".to_string(),
                status: CheckStatus::Warn,
                message: format!("{} director(ies) missing", missing.len()),
                details: missing.iter().map(|d| format!("missing: {}", d)).collect(),
            }
        }
    }

    /// Check 3: Current host configured
    fn check_host(&self) -> HealthCheck {
        match &self.config.current_host {
            Some(host_id) => {
                let host_service =
                    crate::services::host::DefaultHostService::new(&self.config.root);
                if host_service.load_host(host_id).is_ok() {
                    HealthCheck {
                        name: "current_host".to_string(),
                        status: CheckStatus::Pass,
                        message: format!("Current host: {}", host_id),
                        details: vec![],
                    }
                } else {
                    HealthCheck {
                        name: "current_host".to_string(),
                        status: CheckStatus::Fail,
                        message: format!("Host '{}' config missing", host_id),
                        details: vec![],
                    }
                }
            }
            None => HealthCheck {
                name: "current_host".to_string(),
                status: CheckStatus::Warn,
                message: "No current host set".to_string(),
                details: vec![],
            },
        }
    }

    /// Check 4: Git repository (FR-10.6)
    fn check_git(&self) -> HealthCheck {
        if !self.config.root.join(".git").exists() {
            return HealthCheck {
                name: "git".to_string(),
                status: CheckStatus::Warn,
                message: "Not a git repository".to_string(),
                details: vec![],
            };
        }

        let git_status = Command::new("git")
            .args([
                "-C",
                self.config.root.to_str().unwrap_or("."),
                "status",
                "--porcelain",
            ])
            .output();

        match git_status {
            Ok(result) if result.status.success() => {
                let output = String::from_utf8_lossy(&result.stdout);
                if output.trim().is_empty() {
                    HealthCheck {
                        name: "git".to_string(),
                        status: CheckStatus::Pass,
                        message: "Repository clean".to_string(),
                        details: vec![],
                    }
                } else {
                    let changed = output.lines().count();
                    HealthCheck {
                        name: "git".to_string(),
                        status: CheckStatus::Warn,
                        message: format!("{} uncommitted changes", changed),
                        details: vec![],
                    }
                }
            }
            _ => HealthCheck {
                name: "git".to_string(),
                status: CheckStatus::Pass,
                message: "Repository initialized".to_string(),
                details: vec![],
            },
        }
    }

    /// Check 5: Required external tools
    fn check_tools(&self) -> HealthCheck {
        let tools = [("pacman", "Package manager"), ("git", "Version control")];
        let mut missing = Vec::new();

        for (tool, desc) in &tools {
            if !Self::tool_available(tool) {
                missing.push(format!("{} ({})", tool, desc));
            }
        }

        if missing.is_empty() {
            HealthCheck {
                name: "tools".to_string(),
                status: CheckStatus::Pass,
                message: "Required tools available".to_string(),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "tools".to_string(),
                status: CheckStatus::Fail,
                message: "Missing required tools".to_string(),
                details: missing,
            }
        }
    }

    /// Check 6: Package installation (FR-10.3)
    fn check_packages(&self) -> HealthCheck {
        let bundle_id = match &self.config.active_bundle {
            Some(b) => b,
            None => {
                return HealthCheck {
                    name: "packages".to_string(),
                    status: CheckStatus::Pass,
                    message: "No active bundle to verify".to_string(),
                    details: vec![],
                };
            }
        };

        // Load bundle TOML directly (avoids requiring a StateManager)
        let bundle_path = self
            .config
            .root
            .join("bundles")
            .join(bundle_id)
            .join("bundle.toml");
        let bundle: crate::Bundle = match std::fs::read_to_string(&bundle_path)
            .ok()
            .and_then(|c| toml::from_str(&c).ok())
        {
            Some(b) => b,
            None => {
                return HealthCheck {
                    name: "packages".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("Cannot load bundle '{}'", bundle_id),
                    details: vec![],
                };
            }
        };

        let all_packages: Vec<&str> = bundle
            .packages
            .iter()
            .chain(bundle.aur_packages.iter())
            .map(|s| s.as_str())
            .collect();

        if all_packages.is_empty() {
            return HealthCheck {
                name: "packages".to_string(),
                status: CheckStatus::Pass,
                message: "No packages to verify".to_string(),
                details: vec![],
            };
        }

        let mut missing = Vec::new();
        for pkg in &all_packages {
            let installed = Command::new("pacman")
                .args(["-Q", pkg])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !installed {
                missing.push(pkg.to_string());
            }
        }

        if missing.is_empty() {
            HealthCheck {
                name: "packages".to_string(),
                status: CheckStatus::Pass,
                message: format!("{} packages verified", all_packages.len()),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "packages".to_string(),
                status: CheckStatus::Warn,
                message: format!("{} missing packages", missing.len()),
                details: missing,
            }
        }
    }

    /// Check 7: Snapshot backend (FR-10.4)
    fn check_snapshot(&self) -> HealthCheck {
        match self.config.snapshot_backend {
            SnapshotBackend::Timeshift => HealthCheck {
                name: "snapshot".to_string(),
                status: CheckStatus::Pass,
                message: "Timeshift available".to_string(),
                details: vec![],
            },
            SnapshotBackend::Snapper => HealthCheck {
                name: "snapshot".to_string(),
                status: CheckStatus::Pass,
                message: "Snapper available".to_string(),
                details: vec![],
            },
            SnapshotBackend::None => HealthCheck {
                name: "snapshot".to_string(),
                status: CheckStatus::Warn,
                message: "No snapshot backend (install timeshift or snapper)".to_string(),
                details: vec![],
            },
        }
    }

    /// Check 8: Secrets status (FR-10.7)
    fn check_secrets(&self) -> HealthCheck {
        let secrets_dir = self.config.root.join("secrets");
        if !secrets_dir.exists() {
            return HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Pass,
                message: "No secrets configured (optional)".to_string(),
                details: vec![],
            };
        }

        if !Self::tool_available("git-crypt") {
            return HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Warn,
                message: "git-crypt not available".to_string(),
                details: vec![],
            };
        }

        let gitcrypt_dir = self.config.root.join(".git-crypt");
        if !gitcrypt_dir.exists() {
            return HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Warn,
                message: "Secrets dir exists, git-crypt not configured".to_string(),
                details: vec![],
            };
        }

        let keys_dir = gitcrypt_dir.join("keys");
        if keys_dir.exists() {
            HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Pass,
                message: "git-crypt configured".to_string(),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "secrets".to_string(),
                status: CheckStatus::Warn,
                message: "git-crypt not initialized".to_string(),
                details: vec![],
            }
        }
    }

    /// Check 9: Symlink integrity (FR-10.2)
    fn check_symlinks(&self) -> HealthCheck {
        // Discover modules by reading TOML files directly (avoids StateManager dependency)
        let modules_dir = self.config.root.join("modules");
        let modules: Vec<crate::Module> = if modules_dir.exists() {
            std::fs::read_dir(&modules_dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| {
                    let toml_path = e.path().join("module.toml");
                    std::fs::read_to_string(&toml_path)
                        .ok()
                        .and_then(|c| toml::from_str(&c).ok())
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut broken = 0;
        let mut total = 0;
        let mut broken_details = Vec::new();

        for module in &modules {
            for dotfile in &module.dotfiles {
                let target = crate::expand_home(Path::new(&dotfile.target));
                if target.is_symlink() {
                    total += 1;
                    if let Ok(link_target) = std::fs::read_link(&target)
                        && !link_target.exists()
                    {
                        broken += 1;
                        broken_details.push(format!("broken: {}", target.display()));
                    }
                }
            }
        }

        if broken == 0 {
            HealthCheck {
                name: "symlinks".to_string(),
                status: CheckStatus::Pass,
                message: format!("{} symlinks verified", total),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "symlinks".to_string(),
                status: CheckStatus::Warn,
                message: format!("{} broken symlinks found", broken),
                details: broken_details,
            }
        }
    }

    /// Check 11: Security modules status
    fn check_security_modules(&self) -> HealthCheck {
        let modules_dir = self.config.root.join("modules");
        if !modules_dir.exists() {
            return HealthCheck {
                name: "security_modules".to_string(),
                status: CheckStatus::Pass,
                message: "No modules directory".to_string(),
                details: vec![],
            };
        }

        let security_modules: Vec<crate::Module> = std::fs::read_dir(&modules_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let toml_path = e.path().join("module.toml");
                std::fs::read_to_string(&toml_path)
                    .ok()
                    .and_then(|c| toml::from_str::<crate::Module>(&c).ok())
            })
            .filter(|m| matches!(m.kind, crate::ModuleKind::SecurityHardening))
            .collect();

        if security_modules.is_empty() {
            return HealthCheck {
                name: "security_modules".to_string(),
                status: CheckStatus::Warn,
                message: "No security modules found".to_string(),
                details: vec![
                    "Consider enabling security modules (ufw, kernel-hardening, fail2ban)"
                        .to_string(),
                ],
            };
        }

        // Check which have status_check hooks
        let mut with_status = 0;
        let mut details = Vec::new();

        for module in &security_modules {
            if let Some(ref check) = module.status_check {
                let hook_path = modules_dir.join(&module.id).join(check);
                if hook_path.exists() {
                    with_status += 1;
                    let result = Command::new("bash")
                        .arg(&hook_path)
                        .current_dir(modules_dir.join(&module.id))
                        .output();

                    match result {
                        Ok(output) if output.status.success() => {
                            details.push(format!("{}: pass", module.id));
                        }
                        Ok(output) if output.status.code() == Some(2) => {
                            details.push(format!("{}: partial", module.id));
                        }
                        _ => {
                            details.push(format!("{}: not active", module.id));
                        }
                    }
                }
            }
        }

        HealthCheck {
            name: "security_modules".to_string(),
            status: CheckStatus::Pass,
            message: format!(
                "{} security modules found, {} with status checks",
                security_modules.len(),
                with_status
            ),
            details,
        }
    }

    /// Check 12: Firewall status
    fn check_firewall(&self) -> HealthCheck {
        let ufw_active = Command::new("ufw")
            .arg("status")
            .output()
            .map(|o| {
                o.status.success() && String::from_utf8_lossy(&o.stdout).contains("Status: active")
            })
            .unwrap_or(false);

        if ufw_active {
            HealthCheck {
                name: "firewall".to_string(),
                status: CheckStatus::Pass,
                message: "UFW firewall active".to_string(),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "firewall".to_string(),
                status: CheckStatus::Warn,
                message: "No active firewall detected".to_string(),
                details: vec!["Enable ufw module: iron module enable ufw".to_string()],
            }
        }
    }

    /// Check 10: Service availability (NFR-11)
    fn check_services(&self) -> HealthCheck {
        let availability = ServiceAvailability::check();
        let warnings = availability.warnings();

        if warnings.is_empty() {
            HealthCheck {
                name: "services".to_string(),
                status: CheckStatus::Pass,
                message: "All optional services available".to_string(),
                details: vec![],
            }
        } else {
            HealthCheck {
                name: "services".to_string(),
                status: CheckStatus::Warn,
                message: format!("{} service(s) degraded", warnings.len()),
                details: warnings,
            }
        }
    }

    /// Check if a CLI tool is available on PATH.
    fn tool_available(name: &str) -> bool {
        Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check N: Root partition disk space (F0-003)
    ///
    /// Warns at < 5 GB free, fails at < 1 GB free.
    fn check_disk_space(&self) -> HealthCheck {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem::MaybeUninit;

            let path = match CString::new("/") {
                Ok(p) => p,
                Err(_) => {
                    return HealthCheck {
                        name: "disk_space".to_string(),
                        status: CheckStatus::Warn,
                        message: "Unable to check disk space".to_string(),
                        details: vec![],
                    };
                }
            };

            let mut stat = MaybeUninit::<libc::statvfs>::uninit();
            let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };

            if result == 0 {
                let stat = unsafe { stat.assume_init() };
                let total_bytes = stat.f_blocks * stat.f_frsize;
                let free_bytes = stat.f_bavail * stat.f_frsize;

                if total_bytes == 0 {
                    return HealthCheck {
                        name: "disk_space".to_string(),
                        status: CheckStatus::Warn,
                        message: "Unable to determine disk size".to_string(),
                        details: vec![],
                    };
                }

                let free_gb = free_bytes as f64 / 1_073_741_824.0;
                let pct_used =
                    ((total_bytes - free_bytes) as f64 / total_bytes as f64 * 100.0) as u64;

                if free_gb < 1.0 {
                    return HealthCheck {
                        name: "disk_space".to_string(),
                        status: CheckStatus::Fail,
                        message: format!(
                            "Root: {:.1} GB free ({}% used) — critically low!",
                            free_gb, pct_used
                        ),
                        details: vec!["Run 'iron clean' to free space".to_string()],
                    };
                }

                if free_gb < 5.0 {
                    return HealthCheck {
                        name: "disk_space".to_string(),
                        status: CheckStatus::Warn,
                        message: format!(
                            "Root: {:.1} GB free ({}% used) — consider cleanup",
                            free_gb, pct_used
                        ),
                        details: vec!["Run 'iron clean' to free space".to_string()],
                    };
                }

                return HealthCheck {
                    name: "disk_space".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Root: {:.1} GB free ({}% used)", free_gb, pct_used),
                    details: vec![],
                };
            }
        }

        // Fallback if statvfs fails or non-unix
        HealthCheck {
            name: "disk_space".to_string(),
            status: CheckStatus::Warn,
            message: "Unable to check disk space".to_string(),
            details: vec![],
        }
    }
}

impl DoctorService for DefaultDoctorService {
    fn check_all(&self) -> IronResult<HealthReport> {
        let checks = vec![
            self.check_state_file(),
            self.check_directories(),
            self.check_host(),
            self.check_disk_space(),
            self.check_git(),
            self.check_tools(),
            self.check_packages(),
            self.check_snapshot(),
            self.check_secrets(),
            self.check_symlinks(),
            self.check_services(),
            self.check_security_modules(),
            self.check_firewall(),
        ];

        let overall = if checks.iter().any(|c| c.status == CheckStatus::Fail) {
            CheckStatus::Fail
        } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        };

        Ok(HealthReport {
            checks,
            overall,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(root: &Path) -> DoctorConfig {
        DoctorConfig {
            root: root.to_path_buf(),
            current_host: None,
            active_bundle: None,
            snapshot_backend: SnapshotBackend::None,
        }
    }

    #[test]
    fn test_check_status_values() {
        assert_eq!(CheckStatus::Pass.as_str(), "pass");
        assert_eq!(CheckStatus::Warn.as_str(), "warn");
        assert_eq!(CheckStatus::Fail.as_str(), "fail");
    }

    #[test]
    fn test_health_report_counting() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "a".to_string(),
                    status: CheckStatus::Pass,
                    message: "ok".to_string(),
                    details: vec![],
                },
                HealthCheck {
                    name: "b".to_string(),
                    status: CheckStatus::Warn,
                    message: "warn".to_string(),
                    details: vec![],
                },
                HealthCheck {
                    name: "c".to_string(),
                    status: CheckStatus::Fail,
                    message: "fail".to_string(),
                    details: vec![],
                },
            ],
            overall: CheckStatus::Fail,
            timestamp: "2026-02-20T00:00:00Z".to_string(),
        };
        assert_eq!(report.errors(), 1);
        assert_eq!(report.warnings(), 1);
        assert_eq!(report.count(CheckStatus::Pass), 1);
    }

    #[test]
    fn test_health_report_serialization() {
        let report = HealthReport {
            checks: vec![HealthCheck {
                name: "test".to_string(),
                status: CheckStatus::Pass,
                message: "ok".to_string(),
                details: vec![],
            }],
            overall: CheckStatus::Pass,
            timestamp: "2026-02-20T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"pass\""));
        assert!(json.contains("\"test\""));
    }

    #[test]
    fn test_check_state_file_missing() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_state_file();
        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.message.contains("missing"));
    }

    #[test]
    fn test_check_state_file_valid() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("state.json"), "{}").unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_state_file();
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn test_check_state_file_invalid_json() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("state.json"), "NOT JSON").unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_state_file();
        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.message.contains("invalid"));
    }

    #[test]
    fn test_check_directories_all_present() {
        let tmp = TempDir::new().unwrap();
        for dir in &["modules", "profiles", "bundles", "hosts"] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_directories();
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn test_check_directories_some_missing() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("modules")).unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_directories();
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(!check.details.is_empty());
    }

    #[test]
    fn test_check_host_no_host_set() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_host();
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.message.contains("No current host"));
    }

    #[test]
    fn test_check_host_with_host_missing_config() {
        let tmp = TempDir::new().unwrap();
        let mut config = create_test_config(tmp.path());
        config.current_host = Some("nonexistent".to_string());
        let svc = DefaultDoctorService::new(config);
        let check = svc.check_host();
        assert_eq!(check.status, CheckStatus::Fail);
    }

    #[test]
    fn test_check_snapshot_none() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_snapshot();
        assert_eq!(check.status, CheckStatus::Warn);
    }

    #[test]
    fn test_check_snapshot_timeshift() {
        let tmp = TempDir::new().unwrap();
        let mut config = create_test_config(tmp.path());
        config.snapshot_backend = SnapshotBackend::Timeshift;
        let svc = DefaultDoctorService::new(config);
        let check = svc.check_snapshot();
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.message.contains("Timeshift"));
    }

    #[test]
    fn test_check_secrets_no_secrets_dir() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_secrets();
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.message.contains("optional"));
    }

    #[test]
    fn test_check_git_no_git_repo() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_git();
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.message.contains("Not a git"));
    }

    #[test]
    fn test_check_symlinks_no_modules() {
        let tmp = TempDir::new().unwrap();
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let check = svc.check_symlinks();
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn test_check_all_returns_report() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("state.json"), "{}").unwrap();
        for dir in &["modules", "profiles", "bundles", "hosts"] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }

        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let report = svc.check_all().unwrap();

        assert_eq!(report.checks.len(), 13);
        assert!(!report.timestamp.is_empty());
        // At least state_file and directories should pass
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.name == "state_file" && c.status == CheckStatus::Pass)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.name == "directories" && c.status == CheckStatus::Pass)
        );
    }

    #[test]
    fn test_check_all_overall_derives_from_worst() {
        let tmp = TempDir::new().unwrap();
        // No state.json → Fail
        let svc = DefaultDoctorService::new(create_test_config(tmp.path()));
        let report = svc.check_all().unwrap();
        assert_eq!(report.overall, CheckStatus::Fail);
    }

    #[test]
    fn test_health_check_details_serialization() {
        let check = HealthCheck {
            name: "test".to_string(),
            status: CheckStatus::Pass,
            message: "ok".to_string(),
            details: vec![],
        };
        let json = serde_json::to_string(&check).unwrap();
        // details should be skipped when empty
        assert!(!json.contains("details"));
    }
}
