//! Bundle Service - Desktop environment management
//!
//! Provides bundle discovery, installation, activation, and switching.

use crate::bundle::{Bundle, BundleState};
use crate::packages::{NoopPackageManager, PackageManager};
use crate::services::state::StateManager;
use crate::system_service::{NoopSystemService, SystemService};
use crate::validation::expand_home;
use crate::{IronResult, StateError};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Bundle service trait
pub trait BundleService {
    /// Discover all bundles
    fn discover(&self) -> IronResult<Vec<Bundle>>;

    /// Load a specific bundle
    fn load(&self, id: &str) -> IronResult<Bundle>;

    /// Get active bundle for current host
    fn active(&self) -> IronResult<Option<Bundle>>;

    /// Activate a bundle (install packages, link dotfiles)
    fn activate(&self, id: &str) -> IronResult<()>;

    /// Deactivate a bundle (move to dormant)
    fn deactivate(&self, id: &str) -> IronResult<()>;

    /// Switch from one bundle to another
    fn switch(&self, from: &str, to: &str) -> IronResult<()>;

    /// Get bundle state
    fn state(&self, id: &str) -> IronResult<BundleState>;

    /// Check for conflicts between bundles
    fn check_conflicts(&self, id: &str) -> IronResult<Vec<String>>;
}

/// Default bundle service implementation
pub struct DefaultBundleService {
    /// Bundles directory
    bundles_dir: PathBuf,
    /// State manager
    state_manager: StateManager,
    /// Package manager for installing/removing packages
    package_manager: Arc<dyn PackageManager>,
    /// Service manager for enabling/disabling system services
    service_manager: Arc<dyn SystemService>,
}

impl DefaultBundleService {
    /// Create a new bundle service with no-op package/service managers.
    ///
    /// Use [`with_package_manager`] and [`with_service_manager`] to inject
    /// real implementations before use.
    pub fn new(iron_root: &Path, state_manager: StateManager) -> Self {
        Self {
            bundles_dir: iron_root.join("bundles"),
            state_manager,
            package_manager: Arc::new(NoopPackageManager),
            service_manager: Arc::new(NoopSystemService),
        }
    }

    /// Inject a package manager (builder pattern).
    pub fn with_package_manager(mut self, pm: Arc<dyn PackageManager>) -> Self {
        self.package_manager = pm;
        self
    }

    /// Inject a service manager (builder pattern).
    pub fn with_service_manager(mut self, sm: Arc<dyn SystemService>) -> Self {
        self.service_manager = sm;
        self
    }

    /// Get bundle config path
    fn bundle_path(&self, id: &str) -> PathBuf {
        self.bundles_dir.join(id).join("bundle.toml")
    }

    /// Get bundle directory
    fn bundle_dir(&self, id: &str) -> PathBuf {
        self.bundles_dir.join(id)
    }

    /// Get current host ID
    fn current_host(&self) -> IronResult<String> {
        self.state_manager
            .current_host()
            .ok_or_else(|| StateError::NoActiveHost.into())
    }

    /// Install bundle packages via the injected package manager.
    fn install_packages(&self, bundle: &Bundle) -> IronResult<()> {
        let mut all_packages = bundle.packages.clone();
        all_packages.extend(bundle.aur_packages.iter().cloned());
        if !all_packages.is_empty() {
            self.package_manager.install(&all_packages)?;
        }
        Ok(())
    }

    /// Remove bundle packages via the injected package manager.
    ///
    /// Note: not called during deactivation by default — packages may be shared
    /// with other bundles. Called explicitly when the user asks to uninstall.
    #[allow(dead_code)]
    fn remove_packages(&self, bundle: &Bundle) -> IronResult<()> {
        let mut all_packages = bundle.packages.clone();
        all_packages.extend(bundle.aur_packages.iter().cloned());
        if !all_packages.is_empty() {
            // remove_deps=false: don't auto-remove dependencies (other bundles may need them)
            self.package_manager.remove(&all_packages, false)?;
        }
        Ok(())
    }

    /// Link bundle dotfiles
    fn link_dotfiles(&self, bundle: &Bundle) -> IronResult<()> {
        let bundle_dir = self.bundle_dir(&bundle.id);
        let dotfiles_dir = bundle_dir.join("dotfiles");

        if !dotfiles_dir.exists() {
            return Ok(());
        }

        // Walk dotfiles directory and create symlinks
        for entry in walkdir::WalkDir::new(&dotfiles_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let Ok(relative) = entry.path().strip_prefix(&dotfiles_dir) else {
                continue;
            };
            let relative_str = format!("~/.{}", relative.display());
            let target = expand_home(Path::new(&relative_str));

            // Create parent directories
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).ok();
            }

            // Backup existing and create symlink
            if target.exists() && !target.is_symlink() {
                let backup = target.with_extension("iron-backup");
                fs::rename(&target, &backup).ok();
            }

            if target.is_symlink() {
                fs::remove_file(&target).ok();
            }

            #[cfg(unix)]
            std::os::unix::fs::symlink(entry.path(), &target).ok();
        }

        Ok(())
    }

    /// Unlink bundle dotfiles
    fn unlink_dotfiles(&self, bundle: &Bundle) -> IronResult<()> {
        let bundle_dir = self.bundle_dir(&bundle.id);
        let dotfiles_dir = bundle_dir.join("dotfiles");

        if !dotfiles_dir.exists() {
            return Ok(());
        }

        // Walk dotfiles directory and remove symlinks
        for entry in walkdir::WalkDir::new(&dotfiles_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let Ok(relative) = entry.path().strip_prefix(&dotfiles_dir) else {
                continue;
            };
            let relative_str = format!("~/.{}", relative.display());
            let target = expand_home(Path::new(&relative_str));

            if target.is_symlink() {
                fs::remove_file(&target).ok();

                // Restore backup if exists
                let backup = target.with_extension("iron-backup");
                if backup.exists() {
                    fs::rename(&backup, &target).ok();
                }
            }
        }

        Ok(())
    }

    /// Enable and start bundle services via the injected service manager.
    ///
    /// Failures are logged as warnings rather than errors — systemd may not be
    /// available in containers/VMs and bundle activation should still succeed.
    fn enable_services(&self, bundle: &Bundle) -> IronResult<()> {
        for service in &bundle.services {
            if let Err(e) = self.service_manager.enable_service(service) {
                tracing::warn!("Failed to enable service '{}': {}", service, e);
            }
            if let Err(e) = self.service_manager.start_service(service) {
                tracing::warn!("Failed to start service '{}': {}", service, e);
            }
        }
        Ok(())
    }

    /// Stop and disable bundle services via the injected service manager.
    fn disable_services(&self, bundle: &Bundle) -> IronResult<()> {
        for service in &bundle.services {
            if let Err(e) = self.service_manager.stop_service(service) {
                tracing::warn!("Failed to stop service '{}': {}", service, e);
            }
            if let Err(e) = self.service_manager.disable_service(service) {
                tracing::warn!("Failed to disable service '{}': {}", service, e);
            }
        }
        Ok(())
    }
}

impl BundleService for DefaultBundleService {
    fn discover(&self) -> IronResult<Vec<Bundle>> {
        let mut bundles = Vec::new();

        if !self.bundles_dir.exists() {
            return Ok(bundles);
        }

        for entry in fs::read_dir(&self.bundles_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && let Some(id) = entry.file_name().to_str()
                && let Ok(bundle) = self.load(id)
            {
                bundles.push(bundle);
            }
        }

        Ok(bundles)
    }

    fn load(&self, id: &str) -> IronResult<Bundle> {
        let path = self.bundle_path(id);
        if !path.exists() {
            return Err(StateError::BundleNotFound { id: id.to_string() }.into());
        }

        let content = fs::read_to_string(&path)
            .map_err(|_| StateError::BundleNotFound { id: id.to_string() })?;

        toml::from_str(&content).map_err(|e| {
            crate::ConfigError::ParseError {
                path,
                message: e.to_string(),
            }
            .into()
        })
    }

    fn active(&self) -> IronResult<Option<Bundle>> {
        let host_id = self.current_host()?;
        if let Some(bundle_id) = self.state_manager.active_bundle(&host_id) {
            Ok(Some(self.load(&bundle_id)?))
        } else {
            Ok(None)
        }
    }

    fn activate(&self, id: &str) -> IronResult<()> {
        let bundle = self.load(id)?;
        let host_id = self.current_host()?;

        // Check if already active
        if let Some(active_id) = self.state_manager.active_bundle(&host_id) {
            if active_id == id {
                return Err(StateError::BundleAlreadyActive { id: id.to_string() }.into());
            }
            // Deactivate current bundle first
            self.deactivate(&active_id)?;
        }

        // Install packages
        self.install_packages(&bundle)?;

        // Link dotfiles
        self.link_dotfiles(&bundle)?;

        // Enable services
        self.enable_services(&bundle)?;

        // Update state
        self.state_manager.set_active_bundle(&host_id, id)?;

        Ok(())
    }

    fn deactivate(&self, id: &str) -> IronResult<()> {
        let bundle = self.load(id)?;
        let host_id = self.current_host()?;

        // Check if actually active
        if let Some(active_id) = self.state_manager.active_bundle(&host_id) {
            if active_id != id {
                return Err(StateError::BundleNotInstalled { id: id.to_string() }.into());
            }
        } else {
            return Err(StateError::BundleNotInstalled { id: id.to_string() }.into());
        }

        // Disable services
        self.disable_services(&bundle)?;

        // Unlink dotfiles
        self.unlink_dotfiles(&bundle)?;

        // Note: We typically don't remove packages on deactivation
        // as they might be shared with other bundles

        // Clear active bundle from state
        self.state_manager.clear_active_bundle(&host_id)?;

        Ok(())
    }

    fn switch(&self, from: &str, to: &str) -> IronResult<()> {
        // Deactivate current bundle
        self.deactivate(from)?;

        // Activate new bundle
        self.activate(to)?;

        Ok(())
    }

    fn state(&self, id: &str) -> IronResult<BundleState> {
        let _ = self.load(id)?; // Verify bundle exists
        let host_id = self.current_host()?;

        if let Some(active_id) = self.state_manager.active_bundle(&host_id)
            && active_id == id
        {
            return Ok(BundleState::Active);
        }

        // Check if dotfiles are linked (dormant)
        let bundle_dir = self.bundle_dir(id);
        let dotfiles_dir = bundle_dir.join("dotfiles");

        if dotfiles_dir.exists() {
            for entry in walkdir::WalkDir::new(&dotfiles_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let Ok(relative) = entry.path().strip_prefix(&dotfiles_dir) else {
                    continue;
                };
                let relative_str = format!("~/.{}", relative.display());
                let target = expand_home(Path::new(&relative_str));

                if target.is_symlink() {
                    // Some dotfiles are linked, might be dormant
                    return Ok(BundleState::Dormant);
                }
            }
        }

        Ok(BundleState::NotInstalled)
    }

    fn check_conflicts(&self, id: &str) -> IronResult<Vec<String>> {
        let bundle = self.load(id)?;
        let bundles = self.discover()?;

        let mut conflicts = Vec::new();

        // Check explicit conflicts
        for other in &bundles {
            if bundle.conflicts.contains(&other.id) {
                // Check if the conflicting bundle is active
                if let Ok(state) = self.state(&other.id)
                    && state == BundleState::Active
                {
                    conflicts.push(other.id.clone());
                }
            }
        }

        Ok(conflicts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleType;
    use tempfile::TempDir;

    fn create_test_bundle(dir: &Path, id: &str) {
        let bundle_dir = dir.join("bundles").join(id);
        fs::create_dir_all(&bundle_dir).unwrap();

        let bundle = Bundle {
            id: id.to_string(),
            name: format!("Test Bundle {}", id),
            description: Some("A test bundle".to_string()),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec!["pkg1".to_string()],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: vec![],
            services: vec![],
            post_install: None,
        };

        let config_path = bundle_dir.join("bundle.toml");
        let content = toml::to_string_pretty(&bundle).unwrap();
        fs::write(config_path, content).unwrap();
    }

    fn create_test_service() -> (DefaultBundleService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        state_manager.set_current_host("test-host").unwrap();
        let service = DefaultBundleService::new(temp_dir.path(), state_manager);
        (service, temp_dir)
    }

    #[test]
    fn test_discover_bundles() {
        let (service, temp_dir) = create_test_service();

        create_test_bundle(temp_dir.path(), "hyprland");
        create_test_bundle(temp_dir.path(), "plasma");

        let bundles = service.discover().unwrap();
        assert_eq!(bundles.len(), 2);
    }

    #[test]
    fn test_load_bundle() {
        let (service, temp_dir) = create_test_service();

        create_test_bundle(temp_dir.path(), "hyprland");

        let bundle = service.load("hyprland").unwrap();
        assert_eq!(bundle.id, "hyprland");
    }

    #[test]
    fn test_bundle_not_found() {
        let (service, _temp) = create_test_service();

        let result = service.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_bundle_state_not_installed() {
        let (service, temp_dir) = create_test_service();

        create_test_bundle(temp_dir.path(), "hyprland");

        let state = service.state("hyprland").unwrap();
        assert_eq!(state, BundleState::NotInstalled);
    }

    #[test]
    fn test_active_bundle_none() {
        let (service, _temp) = create_test_service();

        let active = service.active().unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn test_activate_bundle() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        service.activate("hyprland").unwrap();

        let active = service.active().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "hyprland");
    }

    #[test]
    fn test_activate_already_active_bundle() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        service.activate("hyprland").unwrap();

        // Trying to activate again should fail
        let result = service.activate("hyprland");
        assert!(result.is_err());
    }

    #[test]
    fn test_activate_switches_from_previous() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");
        create_test_bundle(temp_dir.path(), "niri");

        service.activate("hyprland").unwrap();
        service.activate("niri").unwrap();

        let active = service.active().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "niri");
    }

    #[test]
    fn test_deactivate_not_active_bundle() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        // Deactivating a non-active bundle should fail
        let result = service.deactivate("hyprland");
        assert!(result.is_err());
    }

    #[test]
    fn test_deactivate_clears_active_bundle_state() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        // Activate then deactivate
        service.activate("hyprland").unwrap();
        assert_eq!(service.state("hyprland").unwrap(), BundleState::Active);

        service.deactivate("hyprland").unwrap();

        // State should no longer show hyprland as active
        let active = service.active().unwrap();
        assert!(active.is_none());
        assert_ne!(service.state("hyprland").unwrap(), BundleState::Active);
    }

    #[test]
    fn test_switch_bundles() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");
        create_test_bundle(temp_dir.path(), "niri");

        // Activate first bundle
        service.activate("hyprland").unwrap();
        assert_eq!(service.state("hyprland").unwrap(), BundleState::Active);

        // Switch to second bundle
        service.switch("hyprland", "niri").unwrap();

        let active = service.active().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "niri");
    }

    #[test]
    fn test_bundle_state_active() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        service.activate("hyprland").unwrap();

        let state = service.state("hyprland").unwrap();
        assert_eq!(state, BundleState::Active);
    }

    #[test]
    fn test_check_conflicts_empty() {
        let (service, temp_dir) = create_test_service();
        create_test_bundle(temp_dir.path(), "hyprland");

        let conflicts = service.check_conflicts("hyprland").unwrap();
        assert!(conflicts.is_empty());
    }

    fn create_conflicting_bundle(dir: &Path, id: &str, conflicts_with: Vec<&str>) {
        let bundle_dir = dir.join("bundles").join(id);
        fs::create_dir_all(&bundle_dir).unwrap();

        let bundle = Bundle {
            id: id.to_string(),
            name: format!("Test Bundle {}", id),
            description: Some("A test bundle".to_string()),
            bundle_type: BundleType::WaylandCompositor,
            packages: vec!["pkg1".to_string()],
            aur_packages: vec![],
            profiles: vec![],
            default_profile: None,
            conflicts: conflicts_with.iter().map(|s| s.to_string()).collect(),
            services: vec![],
            post_install: None,
        };

        let config_path = bundle_dir.join("bundle.toml");
        let content = toml::to_string_pretty(&bundle).unwrap();
        fs::write(config_path, content).unwrap();
    }

    #[test]
    fn test_check_conflicts_with_active() {
        let (service, temp_dir) = create_test_service();

        // Create two bundles that conflict
        create_conflicting_bundle(temp_dir.path(), "hyprland", vec!["niri"]);
        create_test_bundle(temp_dir.path(), "niri");

        // Activate niri
        service.activate("niri").unwrap();

        // Check if hyprland conflicts with active niri
        let conflicts = service.check_conflicts("hyprland").unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], "niri");
    }

    #[test]
    fn test_discover_empty_dir() {
        let (service, _temp) = create_test_service();

        let bundles = service.discover().unwrap();
        assert!(bundles.is_empty());
    }

    #[test]
    fn test_bundle_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let service = DefaultBundleService::new(temp_dir.path(), state_manager);

        // Service should be created successfully
        assert!(service.bundles_dir.ends_with("bundles"));
    }

    #[test]
    fn test_activate_nonexistent_bundle() {
        let (service, _temp) = create_test_service();

        let result = service.activate("nonexistent");
        assert!(result.is_err());
    }
}
