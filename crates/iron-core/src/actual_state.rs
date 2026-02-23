//! Actual State — system reality as queried from real sources
//!
//! `ActualState` is the counterpart to `DesiredState`. The apply plan
//! and drift report are computed from their diff. This module defines
//! the struct, supporting types, and the `scan()` method that captures
//! a consistent snapshot of the system.

use crate::IronResult;
use crate::packages::PackageManager;
use crate::system_service::SystemService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Represents the current state of the system as queried from real sources.
/// Counterpart to DesiredState -- the plan is their diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualState {
    /// System hostname
    pub hostname: String,

    /// Explicitly installed packages (from pacman -Qqe equivalent)
    pub installed_packages: HashSet<String>,

    /// AUR/foreign packages (from pacman -Qqm equivalent)
    #[serde(default)]
    pub aur_packages: HashSet<String>,

    /// State of declared services
    #[serde(default)]
    pub services: Vec<ActualServiceState>,

    /// State of managed dotfiles/config files
    #[serde(default)]
    pub managed_files: Vec<ActualFileState>,

    /// When this snapshot was captured
    pub scanned_at: DateTime<Utc>,
}

/// State of a service as observed on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualServiceState {
    /// Service unit name (e.g., "bluetooth.service")
    pub name: String,
    /// Whether the service is enabled at boot
    pub enabled: bool,
    /// Whether the service is currently running
    #[serde(default)]
    pub running: bool,
}

/// State of a managed file/symlink as observed on the filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualFileState {
    /// The target path (e.g., ~/.config/nvim/init.lua)
    pub target: String,
    /// Whether the file/symlink exists at target
    pub exists: bool,
    /// If symlink, where it points
    #[serde(default)]
    pub symlink_target: Option<String>,
    /// SHA256 of the file content (regular files only)
    #[serde(default)]
    pub checksum: Option<String>,
    /// Type of entry at target path
    #[serde(default)]
    pub file_type: FileStateType,
}

/// Type of filesystem entry at a managed path.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileStateType {
    Symlink,
    Regular,
    #[default]
    Missing,
    Directory,
}

/// Specification for a file to check during scan.
/// Constructed from DesiredState dotfile mappings.
#[derive(Debug, Clone)]
pub struct ManagedFileSpec {
    /// The target path to check (e.g., ~/.config/nvim)
    pub target: String,
    /// The expected source for symlinks (for reference, not checked by scan)
    #[allow(dead_code)]
    pub expected_source: Option<String>,
}

/// Specification for a service to check during scan.
#[derive(Debug, Clone)]
pub struct ManagedServiceSpec {
    /// Service name (e.g., "bluetooth.service")
    pub name: String,
}

impl ActualState {
    /// Scan the system and capture all relevant state.
    ///
    /// This is the single source of system truth. Both `compute_plan()`
    /// and `detect()` consume this instead of querying independently.
    ///
    /// # Arguments
    /// * `package_manager` - trait object for querying installed packages
    /// * `service_manager` - trait object for querying service status
    /// * `managed_services` - list of service names to check
    /// * `managed_files` - list of file paths to check
    pub fn scan(
        package_manager: &dyn PackageManager,
        service_manager: &dyn SystemService,
        managed_services: &[ManagedServiceSpec],
        managed_files: &[ManagedFileSpec],
    ) -> IronResult<Self> {
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        // Query all installed packages
        let all_packages = package_manager.query_installed()?;

        let mut installed_packages = HashSet::new();
        let mut aur_packages = HashSet::new();

        for pkg in &all_packages {
            if pkg.is_aur {
                aur_packages.insert(pkg.name.clone());
            }
            // All packages go into installed_packages
            // (AUR packages are also installed)
            installed_packages.insert(pkg.name.clone());
        }

        let services = Self::scan_services(service_manager, managed_services)?;
        let files = Self::scan_files(managed_files)?;

        Ok(Self {
            hostname,
            installed_packages,
            aur_packages,
            services,
            managed_files: files,
            scanned_at: Utc::now(),
        })
    }

    /// Scan services for their enabled/running status.
    fn scan_services(
        service_manager: &dyn SystemService,
        services: &[ManagedServiceSpec],
    ) -> IronResult<Vec<ActualServiceState>> {
        let mut result = Vec::with_capacity(services.len());
        for spec in services {
            let enabled = service_manager.is_enabled(&spec.name).unwrap_or(false);
            result.push(ActualServiceState {
                name: spec.name.clone(),
                enabled,
                // TODO: Add is_running() to SystemService trait
                // when needed. For now, we only track enabled state.
                running: false,
            });
        }
        Ok(result)
    }

    /// Scan managed files for existence, symlink targets, checksums.
    fn scan_files(files: &[ManagedFileSpec]) -> IronResult<Vec<ActualFileState>> {
        let mut result = Vec::with_capacity(files.len());
        for spec in files {
            let path = Path::new(&spec.target);
            let (exists, file_type, symlink_target, checksum) = if path.is_symlink() {
                let target = std::fs::read_link(path)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
                (true, FileStateType::Symlink, target, None)
            } else if path.is_dir() {
                (true, FileStateType::Directory, None, None)
            } else if path.is_file() {
                let checksum = Self::checksum_file(path);
                (true, FileStateType::Regular, None, checksum)
            } else {
                (false, FileStateType::Missing, None, None)
            };

            result.push(ActualFileState {
                target: spec.target.clone(),
                exists,
                symlink_target,
                checksum,
                file_type,
            });
        }
        Ok(result)
    }

    /// Compute SHA256 checksum of a file.
    fn checksum_file(path: &Path) -> Option<String> {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path).ok()?;
        let hash = Sha256::digest(&data);
        Some(format!("{:x}", hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IronResult;
    use crate::packages::{InstalledPackage, NoopPackageManager, PackageManager};
    use crate::system_service::SystemService;
    use tempfile::TempDir;

    // ── Mock implementations for testing ─────────────────────────────

    /// Mock package manager that returns configurable packages.
    struct MockPackageManager {
        packages: Vec<InstalledPackage>,
    }

    impl PackageManager for MockPackageManager {
        fn query_installed(&self) -> IronResult<Vec<InstalledPackage>> {
            Ok(self.packages.clone())
        }
        fn check_updates(&self) -> IronResult<Vec<crate::packages::PackageUpdate>> {
            Ok(vec![])
        }
        fn install(&self, _: &[String]) -> IronResult<()> {
            Ok(())
        }
        fn remove(&self, _: &[String], _: bool) -> IronResult<()> {
            Ok(())
        }
        fn is_installed(&self, pkg: &str) -> IronResult<bool> {
            Ok(self.packages.iter().any(|p| p.name == pkg))
        }
        fn search(&self, _: &str) -> IronResult<Vec<String>> {
            Ok(vec![])
        }
        fn info(&self, _: &str) -> IronResult<Option<InstalledPackage>> {
            Ok(None)
        }
        fn sync_database(&self) -> IronResult<()> {
            Ok(())
        }
        fn upgrade(&self, _: bool) -> IronResult<crate::packages::UpdatePreview> {
            Ok(crate::packages::UpdatePreview::default())
        }
    }

    /// Mock service manager that returns configurable enabled state.
    struct MockServiceManager {
        enabled_services: Vec<String>,
    }

    impl SystemService for MockServiceManager {
        fn enable_service(&self, _: &str) -> IronResult<()> {
            Ok(())
        }
        fn disable_service(&self, _: &str) -> IronResult<()> {
            Ok(())
        }
        fn start_service(&self, _: &str) -> IronResult<()> {
            Ok(())
        }
        fn stop_service(&self, _: &str) -> IronResult<()> {
            Ok(())
        }
        fn is_enabled(&self, name: &str) -> IronResult<bool> {
            Ok(self.enabled_services.contains(&name.to_string()))
        }
    }

    // ── Construction & field tests ───────────────────────────────────

    #[test]
    fn test_actual_state_construction() {
        let state = ActualState {
            hostname: "testhost".to_string(),
            installed_packages: HashSet::from(["git".to_string(), "neovim".to_string()]),
            aur_packages: HashSet::from(["yay".to_string()]),
            services: vec![ActualServiceState {
                name: "bluetooth.service".to_string(),
                enabled: true,
                running: false,
            }],
            managed_files: vec![ActualFileState {
                target: "/home/user/.config/nvim".to_string(),
                exists: true,
                symlink_target: Some("/config/modules/nvim".to_string()),
                checksum: None,
                file_type: FileStateType::Symlink,
            }],
            scanned_at: Utc::now(),
        };

        assert_eq!(state.hostname, "testhost");
        assert_eq!(state.installed_packages.len(), 2);
        assert!(state.installed_packages.contains("git"));
        assert_eq!(state.aur_packages.len(), 1);
        assert_eq!(state.services.len(), 1);
        assert!(state.services[0].enabled);
        assert_eq!(state.managed_files.len(), 1);
        assert!(state.managed_files[0].exists);
    }

    #[test]
    fn test_file_state_type_default() {
        assert_eq!(FileStateType::default(), FileStateType::Missing);
    }

    #[test]
    fn test_file_state_type_equality() {
        assert_eq!(FileStateType::Symlink, FileStateType::Symlink);
        assert_eq!(FileStateType::Regular, FileStateType::Regular);
        assert_ne!(FileStateType::Symlink, FileStateType::Regular);
        assert_ne!(FileStateType::Missing, FileStateType::Directory);
    }

    // ── Serialization roundtrip tests ────────────────────────────────

    #[test]
    fn test_actual_state_serde_roundtrip() {
        let state = ActualState {
            hostname: "archbox".to_string(),
            installed_packages: HashSet::from(["base".to_string(), "linux".to_string()]),
            aur_packages: HashSet::new(),
            services: vec![],
            managed_files: vec![],
            scanned_at: Utc::now(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ActualState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.hostname, "archbox");
        assert_eq!(deserialized.installed_packages.len(), 2);
        assert!(deserialized.installed_packages.contains("base"));
    }

    #[test]
    fn test_actual_state_deserialize_missing_optional_fields() {
        // Simulates loading a JSON with missing optional/defaulted fields
        let json = r#"{
            "hostname": "minimal",
            "installed_packages": ["git"],
            "scanned_at": "2026-02-22T12:00:00Z"
        }"#;

        let state: ActualState = serde_json::from_str(json).unwrap();
        assert_eq!(state.hostname, "minimal");
        assert!(state.aur_packages.is_empty());
        assert!(state.services.is_empty());
        assert!(state.managed_files.is_empty());
    }

    // ── Scan tests with mocks ────────────────────────────────────────

    #[test]
    fn test_scan_with_noop_managers() {
        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let state = ActualState::scan(&pm, &sm, &[], &[]).unwrap();

        assert!(state.installed_packages.is_empty());
        assert!(state.aur_packages.is_empty());
        assert!(state.services.is_empty());
        assert!(state.managed_files.is_empty());
        assert!(!state.hostname.is_empty());
    }

    #[test]
    fn test_scan_with_mock_packages() {
        let pm = MockPackageManager {
            packages: vec![
                InstalledPackage {
                    name: "git".to_string(),
                    version: "2.44.0".to_string(),
                    explicit: true,
                    is_aur: false,
                    ..Default::default()
                },
                InstalledPackage {
                    name: "neovim".to_string(),
                    version: "0.10.0".to_string(),
                    explicit: true,
                    is_aur: false,
                    ..Default::default()
                },
                InstalledPackage {
                    name: "yay".to_string(),
                    version: "12.3.0".to_string(),
                    explicit: true,
                    is_aur: true,
                    ..Default::default()
                },
            ],
        };
        let sm = crate::system_service::NoopSystemService;

        let state = ActualState::scan(&pm, &sm, &[], &[]).unwrap();

        assert_eq!(state.installed_packages.len(), 3);
        assert!(state.installed_packages.contains("git"));
        assert!(state.installed_packages.contains("neovim"));
        assert!(state.installed_packages.contains("yay"));
        assert_eq!(state.aur_packages.len(), 1);
        assert!(state.aur_packages.contains("yay"));
    }

    #[test]
    fn test_scan_services() {
        let pm = NoopPackageManager;
        let sm = MockServiceManager {
            enabled_services: vec!["bluetooth.service".to_string()],
        };

        let services = vec![
            ManagedServiceSpec {
                name: "bluetooth.service".to_string(),
            },
            ManagedServiceSpec {
                name: "sshd.service".to_string(),
            },
        ];

        let state = ActualState::scan(&pm, &sm, &services, &[]).unwrap();

        assert_eq!(state.services.len(), 2);
        assert!(state.services[0].enabled); // bluetooth
        assert!(!state.services[1].enabled); // sshd
    }

    #[test]
    fn test_scan_files_missing() {
        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let files = vec![ManagedFileSpec {
            target: "/nonexistent/path/that/does/not/exist".to_string(),
            expected_source: None,
        }];

        let state = ActualState::scan(&pm, &sm, &[], &files).unwrap();

        assert_eq!(state.managed_files.len(), 1);
        assert!(!state.managed_files[0].exists);
        assert_eq!(state.managed_files[0].file_type, FileStateType::Missing);
    }

    #[test]
    fn test_scan_files_regular_with_checksum() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let files = vec![ManagedFileSpec {
            target: file_path.to_string_lossy().to_string(),
            expected_source: None,
        }];

        let state = ActualState::scan(&pm, &sm, &[], &files).unwrap();

        assert_eq!(state.managed_files.len(), 1);
        assert!(state.managed_files[0].exists);
        assert_eq!(state.managed_files[0].file_type, FileStateType::Regular);
        assert!(state.managed_files[0].checksum.is_some());
        assert!(state.managed_files[0].symlink_target.is_none());
    }

    #[test]
    fn test_scan_files_symlink() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let link = temp.path().join("link.txt");
        std::fs::write(&source, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &link).unwrap();

        #[cfg(not(unix))]
        return; // Skip on non-Unix

        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let files = vec![ManagedFileSpec {
            target: link.to_string_lossy().to_string(),
            expected_source: Some(source.to_string_lossy().to_string()),
        }];

        let state = ActualState::scan(&pm, &sm, &[], &files).unwrap();

        assert_eq!(state.managed_files.len(), 1);
        assert!(state.managed_files[0].exists);
        assert_eq!(state.managed_files[0].file_type, FileStateType::Symlink);
        assert!(state.managed_files[0].symlink_target.is_some());
        // Symlinks do not get checksums
        assert!(state.managed_files[0].checksum.is_none());
    }

    #[test]
    fn test_scan_files_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();

        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let files = vec![ManagedFileSpec {
            target: dir_path.to_string_lossy().to_string(),
            expected_source: None,
        }];

        let state = ActualState::scan(&pm, &sm, &[], &files).unwrap();

        assert_eq!(state.managed_files.len(), 1);
        assert!(state.managed_files[0].exists);
        assert_eq!(state.managed_files[0].file_type, FileStateType::Directory);
    }

    #[test]
    fn test_checksum_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("checksum_test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let checksum = ActualState::checksum_file(&file_path);
        assert!(checksum.is_some());
        // SHA256 of "hello world" is known
        assert_eq!(
            checksum.unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_checksum_file_nonexistent() {
        let checksum = ActualState::checksum_file(Path::new("/nonexistent/file"));
        assert!(checksum.is_none());
    }

    #[test]
    fn test_scan_roundtrip_serialization() {
        let pm = MockPackageManager {
            packages: vec![InstalledPackage {
                name: "git".to_string(),
                version: "2.44.0".to_string(),
                explicit: true,
                is_aur: false,
                ..Default::default()
            }],
        };
        let sm = MockServiceManager {
            enabled_services: vec!["sshd.service".to_string()],
        };
        let services = vec![ManagedServiceSpec {
            name: "sshd.service".to_string(),
        }];

        let state = ActualState::scan(&pm, &sm, &services, &[]).unwrap();

        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: ActualState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.hostname, deserialized.hostname);
        assert_eq!(state.installed_packages, deserialized.installed_packages);
        assert_eq!(state.services.len(), deserialized.services.len());
        assert_eq!(state.services[0].enabled, deserialized.services[0].enabled);
    }

    #[test]
    fn test_scan_combined_packages_services_files() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("managed.conf");
        std::fs::write(&file_path, "config content").unwrap();

        let pm = MockPackageManager {
            packages: vec![
                InstalledPackage {
                    name: "base".to_string(),
                    version: "1.0".to_string(),
                    explicit: true,
                    is_aur: false,
                    ..Default::default()
                },
                InstalledPackage {
                    name: "paru".to_string(),
                    version: "2.0".to_string(),
                    explicit: true,
                    is_aur: true,
                    ..Default::default()
                },
            ],
        };
        let sm = MockServiceManager {
            enabled_services: vec!["NetworkManager.service".to_string()],
        };
        let services = vec![
            ManagedServiceSpec {
                name: "NetworkManager.service".to_string(),
            },
            ManagedServiceSpec {
                name: "cups.service".to_string(),
            },
        ];
        let files = vec![
            ManagedFileSpec {
                target: file_path.to_string_lossy().to_string(),
                expected_source: None,
            },
            ManagedFileSpec {
                target: "/does/not/exist".to_string(),
                expected_source: None,
            },
        ];

        let state = ActualState::scan(&pm, &sm, &services, &files).unwrap();

        // Packages
        assert_eq!(state.installed_packages.len(), 2);
        assert!(state.installed_packages.contains("base"));
        assert!(state.installed_packages.contains("paru"));
        assert_eq!(state.aur_packages.len(), 1);
        assert!(state.aur_packages.contains("paru"));

        // Services
        assert_eq!(state.services.len(), 2);
        assert!(state.services[0].enabled); // NetworkManager
        assert!(!state.services[1].enabled); // cups

        // Files
        assert_eq!(state.managed_files.len(), 2);
        assert!(state.managed_files[0].exists);
        assert_eq!(state.managed_files[0].file_type, FileStateType::Regular);
        assert!(state.managed_files[0].checksum.is_some());
        assert!(!state.managed_files[1].exists);
        assert_eq!(state.managed_files[1].file_type, FileStateType::Missing);
    }

    #[test]
    fn test_scan_with_empty_packages() {
        let pm = MockPackageManager { packages: vec![] };
        let sm = crate::system_service::NoopSystemService;

        let state = ActualState::scan(&pm, &sm, &[], &[]).unwrap();

        assert!(state.installed_packages.is_empty());
        assert!(state.aur_packages.is_empty());
    }

    #[test]
    fn test_scan_hostname_is_populated() {
        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;

        let state = ActualState::scan(&pm, &sm, &[], &[]).unwrap();

        // hostname should be a non-empty string from gethostname
        assert!(!state.hostname.is_empty());
    }

    #[test]
    fn test_scan_scanned_at_is_recent() {
        let pm = NoopPackageManager;
        let sm = crate::system_service::NoopSystemService;
        let before = Utc::now();

        let state = ActualState::scan(&pm, &sm, &[], &[]).unwrap();

        let after = Utc::now();
        assert!(state.scanned_at >= before);
        assert!(state.scanned_at <= after);
    }
}
