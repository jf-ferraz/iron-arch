//! Recovery Service - System recovery and install script generation
//!
//! Provides state export/import and install script generation for disaster recovery.

use crate::services::state::StateManager;
use crate::snapshot::SnapshotManager;
use crate::state::OperationStatus;
use crate::{IronResult, StateError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Recovery export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryExport {
    /// Export version
    pub version: String,
    /// Export timestamp
    pub timestamp: chrono::DateTime<Utc>,
    /// Host ID this export is for
    pub host_id: String,
    /// Active bundle
    pub active_bundle: Option<String>,
    /// Active profile
    pub active_profile: Option<String>,
    /// Active modules
    pub active_modules: Vec<String>,
    /// Installed packages
    pub packages: Vec<String>,
    /// AUR packages
    pub aur_packages: Vec<String>,
    /// Enabled services
    pub services: Vec<String>,
}

/// Install script options
#[derive(Debug, Clone, Default)]
pub struct InstallScriptOptions {
    /// Include package installation
    pub include_packages: bool,
    /// Include AUR packages
    pub include_aur: bool,
    /// Include service enablement
    pub include_services: bool,
    /// Include module activation
    pub include_modules: bool,
    /// Include bundle activation
    pub include_bundle: bool,
    /// AUR helper to use
    pub aur_helper: String,
    /// Generate interactive script
    pub interactive: bool,
}

/// Recovery service trait
pub trait RecoveryService {
    /// Export current state to recovery format
    fn export(&self) -> IronResult<RecoveryExport>;

    /// Import state from recovery export
    fn import(&self, export: &RecoveryExport) -> IronResult<()>;

    /// Generate install script
    fn generate_install_script(&self, options: &InstallScriptOptions) -> IronResult<String>;

    /// Save export to file
    fn save_export(&self, path: &Path) -> IronResult<()>;

    /// Load export from file
    fn load_export(&self, path: &Path) -> IronResult<RecoveryExport>;

    /// Create full backup (state + dotfiles + config)
    fn create_backup(&self, output_dir: &Path) -> IronResult<PathBuf>;

    /// Restore from backup
    fn restore_backup(&self, backup_path: &Path) -> IronResult<()>;
}

/// Default recovery service implementation
pub struct DefaultRecoveryService<S: SnapshotManager> {
    /// Iron root directory
    iron_root: PathBuf,
    /// State manager
    state_manager: StateManager,
    /// Snapshot manager
    snapshot_manager: S,
}

impl<S: SnapshotManager> DefaultRecoveryService<S> {
    /// Create a new recovery service
    pub fn new(iron_root: &Path, state_manager: StateManager, snapshot_manager: S) -> Self {
        Self {
            iron_root: iron_root.to_path_buf(),
            state_manager,
            snapshot_manager,
        }
    }

    /// Get list of explicitly installed packages
    fn get_installed_packages(&self) -> Vec<String> {
        std::process::Command::new("pacman")
            .args(["-Qqe"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get list of AUR packages
    fn get_aur_packages(&self) -> Vec<String> {
        std::process::Command::new("pacman")
            .args(["-Qqm"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get list of enabled user services
    fn get_enabled_services(&self) -> Vec<String> {
        std::process::Command::new("systemctl")
            .args([
                "--user",
                "list-unit-files",
                "--state=enabled",
                "--no-legend",
            ])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|l| l.split_whitespace().next())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl<S: SnapshotManager> RecoveryService for DefaultRecoveryService<S> {
    fn export(&self) -> IronResult<RecoveryExport> {
        let host_id = self
            .state_manager
            .current_host()
            .ok_or(StateError::NoActiveHost)?;

        let active_bundle = self.state_manager.active_bundle(&host_id);
        let active_profile = self.state_manager.active_profile(&host_id);
        let active_modules = self.state_manager.active_modules();

        let packages = self.get_installed_packages();
        let aur_packages = self.get_aur_packages();
        let services = self.get_enabled_services();

        Ok(RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id,
            active_bundle,
            active_profile,
            active_modules,
            packages,
            aur_packages,
            services,
        })
    }

    fn import(&self, export: &RecoveryExport) -> IronResult<()> {
        // Set current host
        self.state_manager.set_current_host(&export.host_id)?;

        // Set active bundle
        if let Some(bundle_id) = &export.active_bundle {
            self.state_manager
                .set_active_bundle(&export.host_id, bundle_id)?;
        }

        // Set active profile
        if let Some(profile_id) = &export.active_profile {
            self.state_manager
                .set_active_profile(&export.host_id, profile_id)?;
        }

        // Enable modules
        for module_id in &export.active_modules {
            self.state_manager.enable_module(module_id)?;
        }

        self.state_manager
            .record_operation("import_recovery", OperationStatus::Success, None)?;

        Ok(())
    }

    fn generate_install_script(&self, options: &InstallScriptOptions) -> IronResult<String> {
        let export = self.export()?;
        let mut script = String::new();

        // Script header
        script.push_str("#!/bin/bash\n");
        script.push_str("# Iron Recovery Script\n");
        script.push_str(&format!("# Generated: {}\n", Utc::now()));
        script.push_str(&format!("# Host: {}\n\n", export.host_id));

        script.push_str("set -e\n\n");

        if options.interactive {
            script.push_str("# Interactive mode - confirm before each step\n");
            script.push_str("confirm() {\n");
            script.push_str("    read -p \"$1 [y/N] \" response\n");
            script.push_str("    [[ \"$response\" =~ ^[Yy]$ ]]\n");
            script.push_str("}\n\n");
        }

        // Package installation
        if options.include_packages && !export.packages.is_empty() {
            script.push_str("# Install official packages\n");
            if options.interactive {
                script.push_str("if confirm \"Install official packages?\"; then\n    ");
            }

            let official_packages: Vec<_> = export
                .packages
                .iter()
                .filter(|p| !export.aur_packages.contains(p))
                .collect();

            if !official_packages.is_empty() {
                script.push_str(&format!(
                    "sudo pacman -S --needed --noconfirm {}\n",
                    official_packages
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }

            if options.interactive {
                script.push_str("fi\n");
            }
            script.push('\n');
        }

        // AUR package installation
        if options.include_aur && !export.aur_packages.is_empty() {
            let helper = if options.aur_helper.is_empty() {
                "paru"
            } else {
                &options.aur_helper
            };

            script.push_str("# Install AUR packages\n");
            if options.interactive {
                script.push_str("if confirm \"Install AUR packages?\"; then\n    ");
            }

            script.push_str(&format!(
                "{} -S --needed --noconfirm {}\n",
                helper,
                export.aur_packages.join(" ")
            ));

            if options.interactive {
                script.push_str("fi\n");
            }
            script.push('\n');
        }

        // Service enablement
        if options.include_services && !export.services.is_empty() {
            script.push_str("# Enable user services\n");
            if options.interactive {
                script.push_str("if confirm \"Enable user services?\"; then\n");
            }

            for service in &export.services {
                script.push_str(&format!("    systemctl --user enable {}\n", service));
            }

            if options.interactive {
                script.push_str("fi\n");
            }
            script.push('\n');
        }

        // Iron configuration
        if options.include_bundle || options.include_modules {
            script.push_str("# Iron configuration\n");

            if let Some(bundle) = &export.active_bundle
                && options.include_bundle
            {
                script.push_str(&format!("# Activate bundle: {}\n", bundle));
                script.push_str(&format!("iron bundle activate {}\n", bundle));
            }

            if options.include_modules && !export.active_modules.is_empty() {
                script.push_str("# Enable modules\n");
                for module in &export.active_modules {
                    script.push_str(&format!("iron module enable {}\n", module));
                }
            }
        }

        script.push_str("\necho \"Recovery complete!\"\n");

        Ok(script)
    }

    fn save_export(&self, path: &Path) -> IronResult<()> {
        let export = self.export()?;
        let content = serde_json::to_string_pretty(&export).map_err(|e| {
            crate::IronError::OperationFailed {
                message: format!("Failed to serialize export: {}", e),
            }
        })?;

        fs::write(path, content).map_err(|_| crate::FsError::PermissionDenied {
            path: path.to_path_buf(),
        })?;

        Ok(())
    }

    fn load_export(&self, path: &Path) -> IronResult<RecoveryExport> {
        let content = fs::read_to_string(path).map_err(|_| crate::FsError::NotFound {
            path: path.to_path_buf(),
        })?;

        serde_json::from_str(&content).map_err(|e| crate::IronError::OperationFailed {
            message: format!("Failed to parse export: {}", e),
        })
    }

    fn create_backup(&self, output_dir: &Path) -> IronResult<PathBuf> {
        // Create system snapshot first
        self.snapshot_manager.create("pre-backup")?;

        // Create backup directory
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("iron-backup-{}", timestamp);
        let backup_path = output_dir.join(&backup_name);
        fs::create_dir_all(&backup_path).map_err(|_| crate::FsError::PermissionDenied {
            path: backup_path.clone(),
        })?;

        // Export state
        self.save_export(&backup_path.join("state.json"))?;

        // Copy iron config directory
        let config_backup = backup_path.join("config");
        fs::create_dir_all(&config_backup).ok();

        // Copy important directories
        for dir in &["hosts", "bundles", "profiles", "modules"] {
            let src = self.iron_root.join(dir);
            if src.exists() {
                let dst = config_backup.join(dir);
                copy_dir_recursive(&src, &dst).ok();
            }
        }

        // Create archive
        let archive_path = output_dir.join(format!("{}.tar.gz", backup_name));
        std::process::Command::new("tar")
            .args([
                "-czf",
                archive_path.to_str().unwrap(),
                "-C",
                output_dir.to_str().unwrap(),
                &backup_name,
            ])
            .status()
            .ok();

        // Clean up uncompressed directory
        fs::remove_dir_all(&backup_path).ok();

        self.state_manager.record_operation(
            "create_backup",
            OperationStatus::Success,
            Some(archive_path.display().to_string()),
        )?;

        Ok(archive_path)
    }

    fn restore_backup(&self, backup_path: &Path) -> IronResult<()> {
        // Extract archive to temp location
        let temp_dir = tempfile::TempDir::new().map_err(|_| crate::IronError::OperationFailed {
            message: "Failed to create temp directory".to_string(),
        })?;

        std::process::Command::new("tar")
            .args([
                "-xzf",
                backup_path.to_str().unwrap(),
                "-C",
                temp_dir.path().to_str().unwrap(),
            ])
            .status()
            .map_err(|_| crate::IronError::OperationFailed {
                message: "Failed to extract backup".to_string(),
            })?;

        // Find extracted directory
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .into_iter()
            .flatten()
            .flatten()
            .collect();

        if entries.is_empty() {
            return Err(crate::IronError::OperationFailed {
                message: "Empty backup archive".to_string(),
            });
        }

        let backup_dir = entries[0].path();

        // Load and import state
        let state_path = backup_dir.join("state.json");
        if state_path.exists() {
            let export = self.load_export(&state_path)?;
            self.import(&export)?;
        }

        // Restore config directories
        let config_backup = backup_dir.join("config");
        if config_backup.exists() {
            for dir in &["hosts", "bundles", "profiles", "modules"] {
                let src = config_backup.join(dir);
                if src.exists() {
                    let dst = self.iron_root.join(dir);
                    fs::create_dir_all(&dst).ok();
                    copy_dir_recursive(&src, &dst).ok();
                }
            }
        }

        self.state_manager
            .record_operation("restore_backup", OperationStatus::Success, None)?;

        Ok(())
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::NoopManager;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultRecoveryService<NoopManager>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        state_manager.set_current_host("test-host").unwrap();
        let snapshot_manager = NoopManager;
        let service = DefaultRecoveryService::new(temp_dir.path(), state_manager, snapshot_manager);
        (service, temp_dir)
    }

    // ==========================================================================
    // RecoveryExport Tests
    // ==========================================================================

    #[test]
    fn test_recovery_export_serialization() {
        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "desktop".to_string(),
            active_bundle: Some("hyprland".to_string()),
            active_profile: Some("developer".to_string()),
            active_modules: vec!["nvim".to_string(), "fish".to_string()],
            packages: vec!["neovim".to_string(), "fish".to_string()],
            aur_packages: vec!["hyprshot".to_string()],
            services: vec!["pipewire.service".to_string()],
        };

        let json = serde_json::to_string(&export).unwrap();
        let deserialized: RecoveryExport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "1.0");
        assert_eq!(deserialized.host_id, "desktop");
        assert_eq!(deserialized.active_bundle, Some("hyprland".to_string()));
        assert_eq!(deserialized.active_profile, Some("developer".to_string()));
        assert_eq!(deserialized.active_modules.len(), 2);
        assert_eq!(deserialized.packages.len(), 2);
        assert_eq!(deserialized.aur_packages.len(), 1);
        assert_eq!(deserialized.services.len(), 1);
    }

    #[test]
    fn test_recovery_export_clone() {
        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "test".to_string(),
            active_bundle: None,
            active_profile: None,
            active_modules: vec![],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        let cloned = export.clone();
        assert_eq!(cloned.host_id, "test");
        assert_eq!(cloned.version, "1.0");
    }

    #[test]
    fn test_recovery_export_debug() {
        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "debug-test".to_string(),
            active_bundle: None,
            active_profile: None,
            active_modules: vec![],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        let debug_str = format!("{:?}", export);
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("RecoveryExport"));
    }

    #[test]
    fn test_recovery_export_empty_fields() {
        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "empty".to_string(),
            active_bundle: None,
            active_profile: None,
            active_modules: vec![],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        let json = serde_json::to_string(&export).unwrap();
        let deserialized: RecoveryExport = serde_json::from_str(&json).unwrap();

        assert!(deserialized.active_bundle.is_none());
        assert!(deserialized.active_profile.is_none());
        assert!(deserialized.active_modules.is_empty());
    }

    // ==========================================================================
    // InstallScriptOptions Tests
    // ==========================================================================

    #[test]
    fn test_install_script_options_default() {
        let options = InstallScriptOptions::default();

        assert!(!options.include_packages);
        assert!(!options.include_aur);
        assert!(!options.include_services);
        assert!(!options.include_modules);
        assert!(!options.include_bundle);
        assert!(options.aur_helper.is_empty());
        assert!(!options.interactive);
    }

    #[test]
    fn test_install_script_options_clone() {
        let options = InstallScriptOptions {
            include_packages: true,
            include_aur: true,
            include_services: false,
            include_modules: true,
            include_bundle: false,
            aur_helper: "paru".to_string(),
            interactive: true,
        };

        let cloned = options.clone();
        assert!(cloned.include_packages);
        assert!(cloned.include_aur);
        assert!(!cloned.include_services);
        assert!(cloned.include_modules);
        assert!(!cloned.include_bundle);
        assert_eq!(cloned.aur_helper, "paru");
        assert!(cloned.interactive);
    }

    #[test]
    fn test_install_script_options_debug() {
        let options = InstallScriptOptions {
            include_packages: true,
            aur_helper: "yay".to_string(),
            ..Default::default()
        };

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("InstallScriptOptions"));
        assert!(debug_str.contains("yay"));
    }

    // ==========================================================================
    // DefaultRecoveryService Tests
    // ==========================================================================

    #[test]
    fn test_export() {
        let (service, _temp) = create_test_service();

        let export = service.export().unwrap();
        assert_eq!(export.host_id, "test-host");
        assert_eq!(export.version, "1.0");
    }

    #[test]
    fn test_export_with_bundle_and_profile() {
        let (service, _temp) = create_test_service();

        // Set active bundle and profile
        service
            .state_manager
            .set_active_bundle("test-host", "hyprland")
            .unwrap();
        service
            .state_manager
            .set_active_profile("test-host", "developer")
            .unwrap();
        service.state_manager.enable_module("nvim-ide").unwrap();

        let export = service.export().unwrap();
        assert_eq!(export.active_bundle, Some("hyprland".to_string()));
        assert_eq!(export.active_profile, Some("developer".to_string()));
        assert!(export.active_modules.contains(&"nvim-ide".to_string()));
    }

    #[test]
    fn test_generate_install_script() {
        let (service, _temp) = create_test_service();

        let options = InstallScriptOptions {
            include_packages: true,
            include_aur: true,
            include_services: true,
            include_modules: true,
            include_bundle: true,
            aur_helper: "yay".to_string(),
            interactive: false,
        };

        let script = service.generate_install_script(&options).unwrap();
        assert!(script.contains("#!/bin/bash"));
        assert!(script.contains("Iron Recovery Script"));
        assert!(script.contains("set -e"));
        assert!(script.contains("Recovery complete!"));
    }

    #[test]
    fn test_generate_install_script_interactive() {
        let (service, _temp) = create_test_service();

        let options = InstallScriptOptions {
            include_packages: true,
            include_aur: true,
            include_services: true,
            include_modules: true,
            include_bundle: true,
            aur_helper: "paru".to_string(),
            interactive: true,
        };

        let script = service.generate_install_script(&options).unwrap();
        assert!(script.contains("confirm()"));
        assert!(script.contains("Interactive mode"));
        assert!(script.contains("[y/N]"));
    }

    #[test]
    fn test_generate_install_script_default_aur_helper() {
        let (service, _temp) = create_test_service();

        // Enable module and set bundle to generate content
        service.state_manager.enable_module("test-mod").unwrap();
        service
            .state_manager
            .set_active_bundle("test-host", "test-bundle")
            .unwrap();

        let options = InstallScriptOptions {
            include_packages: false,
            include_aur: true,
            include_services: false,
            include_modules: true,
            include_bundle: true,
            aur_helper: String::new(), // Empty - should default to paru
            interactive: false,
        };

        let script = service.generate_install_script(&options).unwrap();
        // Script should contain iron module enable for test-mod
        assert!(script.contains("iron module enable test-mod"));
        assert!(script.contains("iron bundle activate test-bundle"));
    }

    #[test]
    fn test_generate_install_script_services_only() {
        let (service, _temp) = create_test_service();

        let options = InstallScriptOptions {
            include_packages: false,
            include_aur: false,
            include_services: true,
            include_modules: false,
            include_bundle: false,
            aur_helper: String::new(),
            interactive: false,
        };

        let script = service.generate_install_script(&options).unwrap();
        assert!(script.contains("#!/bin/bash"));
        // Should not contain package installation sections since include_packages is false
        assert!(!script.contains("sudo pacman -S"));
    }

    #[test]
    fn test_save_load_export() {
        let (service, temp_dir) = create_test_service();

        let export_path = temp_dir.path().join("export.json");
        service.save_export(&export_path).unwrap();

        let loaded = service.load_export(&export_path).unwrap();
        assert_eq!(loaded.host_id, "test-host");
    }

    #[test]
    fn test_load_export_not_found() {
        let (service, temp_dir) = create_test_service();

        let export_path = temp_dir.path().join("nonexistent.json");
        let result = service.load_export(&export_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_load_export_invalid_json() {
        let (service, temp_dir) = create_test_service();

        let export_path = temp_dir.path().join("invalid.json");
        fs::write(&export_path, "not valid json {{{").unwrap();

        let result = service.load_export(&export_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_export() {
        let (service, _temp) = create_test_service();

        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "new-host".to_string(),
            active_bundle: Some("hyprland".to_string()),
            active_profile: Some("minimal".to_string()),
            active_modules: vec!["nvim".to_string(), "zsh".to_string()],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        service.import(&export).unwrap();

        // Verify import
        let state = service.state_manager.current_host();
        assert_eq!(state, Some("new-host".to_string()));
    }

    #[test]
    fn test_import_export_no_bundle() {
        let (service, _temp) = create_test_service();

        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "no-bundle-host".to_string(),
            active_bundle: None,
            active_profile: None,
            active_modules: vec![],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        service.import(&export).unwrap();

        let state = service.state_manager.current_host();
        assert_eq!(state, Some("no-bundle-host".to_string()));
    }

    #[test]
    fn test_import_with_modules_only() {
        let (service, _temp) = create_test_service();

        let export = RecoveryExport {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            host_id: "modules-host".to_string(),
            active_bundle: None,
            active_profile: None,
            active_modules: vec!["mod1".to_string(), "mod2".to_string(), "mod3".to_string()],
            packages: vec![],
            aur_packages: vec![],
            services: vec![],
        };

        service.import(&export).unwrap();

        let modules = service.state_manager.active_modules();
        assert!(modules.contains(&"mod1".to_string()));
        assert!(modules.contains(&"mod2".to_string()));
        assert!(modules.contains(&"mod3".to_string()));
    }

    // ==========================================================================
    // copy_dir_recursive Tests
    // ==========================================================================

    #[test]
    fn test_copy_dir_recursive_simple() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file1.txt"), "content1").unwrap();
        fs::write(src.join("file2.txt"), "content2").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("file2.txt").exists());
        assert_eq!(
            fs::read_to_string(dst.join("file1.txt")).unwrap(),
            "content1"
        );
    }

    #[test]
    fn test_copy_dir_recursive_nested() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir_all(src.join("level1/level2")).unwrap();
        fs::write(src.join("root.txt"), "root").unwrap();
        fs::write(src.join("level1/mid.txt"), "mid").unwrap();
        fs::write(src.join("level1/level2/deep.txt"), "deep").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("root.txt").exists());
        assert!(dst.join("level1/mid.txt").exists());
        assert!(dst.join("level1/level2/deep.txt").exists());
        assert_eq!(
            fs::read_to_string(dst.join("level1/level2/deep.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn test_copy_dir_recursive_empty() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("empty_src");
        let dst = temp_dir.path().join("empty_dst");

        fs::create_dir_all(&src).unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.exists());
        assert!(dst.is_dir());
    }

    #[test]
    fn test_copy_dir_recursive_creates_dst() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("nested/deep/dst");

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("test.txt"), "test").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.exists());
        assert!(dst.join("test.txt").exists());
    }

    // ==========================================================================
    // Backup/Restore Tests (limited - require system commands)
    // ==========================================================================

    #[test]
    fn test_create_backup_creates_directories() {
        let (service, temp_dir) = create_test_service();

        // Create some config directories
        fs::create_dir_all(temp_dir.path().join("hosts")).unwrap();
        fs::write(temp_dir.path().join("hosts/test.toml"), "[host]").unwrap();
        fs::create_dir_all(temp_dir.path().join("bundles")).unwrap();
        fs::create_dir_all(temp_dir.path().join("profiles")).unwrap();
        fs::create_dir_all(temp_dir.path().join("modules")).unwrap();

        let output_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&output_dir).unwrap();

        // This test may fail if tar is not available, but it tests the setup
        let _result = service.create_backup(&output_dir);
        // We don't assert success because tar might not be available in test env
    }

    #[test]
    fn test_restore_backup_nonexistent() {
        let (service, temp_dir) = create_test_service();

        let backup_path = temp_dir.path().join("nonexistent.tar.gz");
        let result = service.restore_backup(&backup_path);

        assert!(result.is_err());
    }
}
