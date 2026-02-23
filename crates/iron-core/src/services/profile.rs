//! Profile Service - Profile management and inheritance
//!
//! Provides profile discovery, inheritance resolution, and activation.

use crate::module::ModuleState;
use crate::profile::{Profile, ProfileState};
use crate::services::module::ModuleService;
use crate::services::state::StateManager;
use crate::{IronResult, StateError};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Profile service trait
pub trait ProfileService {
    /// Discover all profiles
    fn discover(&self) -> IronResult<Vec<Profile>>;

    /// Load a specific profile
    fn load(&self, id: &str) -> IronResult<Profile>;

    /// Get active profile for current host
    fn active(&self) -> IronResult<Option<Profile>>;

    /// Apply a profile (enable its modules)
    fn apply(&self, id: &str) -> IronResult<()>;

    /// Unapply a profile (disable its modules)
    fn unapply(&self, id: &str) -> IronResult<()>;

    /// Get profile state
    fn state(&self, id: &str) -> IronResult<ProfileState>;

    /// Resolve inheritance chain
    fn resolve_inheritance(&self, id: &str) -> IronResult<Vec<String>>;

    /// Get all modules for a profile (including inherited)
    fn effective_modules(&self, id: &str) -> IronResult<Vec<String>>;

    /// Find profiles compatible with a bundle
    fn for_bundle(&self, bundle_id: &str) -> IronResult<Vec<Profile>>;
}

/// Default profile service implementation
pub struct DefaultProfileService<M: ModuleService> {
    /// Profiles directory
    profiles_dir: PathBuf,
    /// State manager
    state_manager: StateManager,
    /// Module service for enabling/disabling modules
    module_service: M,
}

impl<M: ModuleService> DefaultProfileService<M> {
    /// Create a new profile service
    pub fn new(iron_root: &Path, state_manager: StateManager, module_service: M) -> Self {
        Self {
            profiles_dir: iron_root.join("profiles"),
            state_manager,
            module_service,
        }
    }

    /// Get profile config path
    fn profile_path(&self, id: &str) -> PathBuf {
        self.profiles_dir.join(id).join("profile.toml")
    }

    /// Get current host ID
    fn current_host(&self) -> IronResult<String> {
        self.state_manager
            .current_host()
            .ok_or_else(|| StateError::NoActiveHost.into())
    }

    /// Build full inheritance chain
    fn build_inheritance_chain(
        &self,
        id: &str,
        visited: &mut HashSet<String>,
    ) -> IronResult<Vec<String>> {
        if visited.contains(id) {
            // Circular inheritance detected, stop
            return Ok(vec![]);
        }
        visited.insert(id.to_string());

        let profile = self.load(id)?;
        let mut chain = vec![id.to_string()];

        if let Some(parent_id) = &profile.extends {
            let parent_chain = self.build_inheritance_chain(parent_id, visited)?;
            chain.extend(parent_chain);
        }

        Ok(chain)
    }
}

impl<M: ModuleService> ProfileService for DefaultProfileService<M> {
    fn discover(&self) -> IronResult<Vec<Profile>> {
        let mut profiles = Vec::new();

        if !self.profiles_dir.exists() {
            return Ok(profiles);
        }

        for entry in fs::read_dir(&self.profiles_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && let Some(id) = entry.file_name().to_str()
                && let Ok(profile) = self.load(id)
            {
                profiles.push(profile);
            }
        }

        Ok(profiles)
    }

    fn load(&self, id: &str) -> IronResult<Profile> {
        let path = self.profile_path(id);
        if !path.exists() {
            return Err(StateError::ProfileNotFound { id: id.to_string() }.into());
        }

        let content = fs::read_to_string(&path)
            .map_err(|_| StateError::ProfileNotFound { id: id.to_string() })?;

        toml::from_str(&content).map_err(|e| {
            crate::ConfigError::ParseError {
                path,
                message: e.to_string(),
            }
            .into()
        })
    }

    fn active(&self) -> IronResult<Option<Profile>> {
        let host_id = self.current_host()?;
        if let Some(profile_id) = self.state_manager.active_profile(&host_id) {
            Ok(Some(self.load(&profile_id)?))
        } else {
            Ok(None)
        }
    }

    fn apply(&self, id: &str) -> IronResult<()> {
        let host_id = self.current_host()?;

        // Get all modules for this profile
        let modules = self.effective_modules(id)?;

        // Enable each module
        for module_id in &modules {
            // Skip if already installed
            if let Ok(state) = self.module_service.status(module_id)
                && state == ModuleState::Installed
            {
                continue;
            }
            self.module_service.enable(module_id)?;
        }

        // Update state
        self.state_manager.set_active_profile(&host_id, id)?;

        Ok(())
    }

    fn unapply(&self, id: &str) -> IronResult<()> {
        let _host_id = self.current_host()?;

        // Get all modules for this profile
        let modules = self.effective_modules(id)?;

        // Disable each module (in reverse order)
        for module_id in modules.iter().rev() {
            if let Ok(state) = self.module_service.status(module_id)
                && state == ModuleState::Installed
            {
                self.module_service.disable(module_id)?;
            }
        }

        // Note: We don't clear active profile here as another profile might be applied
        Ok(())
    }

    fn state(&self, id: &str) -> IronResult<ProfileState> {
        let _ = self.load(id)?; // Verify profile exists

        let modules = self.effective_modules(id)?;
        if modules.is_empty() {
            return Ok(ProfileState::Inactive);
        }

        let mut installed_count = 0;
        for module_id in &modules {
            if let Ok(state) = self.module_service.status(module_id)
                && state == ModuleState::Installed
            {
                installed_count += 1;
            }
        }

        if installed_count == modules.len() {
            Ok(ProfileState::Active)
        } else if installed_count > 0 {
            Ok(ProfileState::Partial)
        } else {
            Ok(ProfileState::Inactive)
        }
    }

    fn resolve_inheritance(&self, id: &str) -> IronResult<Vec<String>> {
        let mut visited = HashSet::new();
        self.build_inheritance_chain(id, &mut visited)
    }

    fn effective_modules(&self, id: &str) -> IronResult<Vec<String>> {
        let chain = self.resolve_inheritance(id)?;
        let mut modules = Vec::new();
        let mut seen = HashSet::new();

        // Collect modules from all profiles in inheritance chain
        // Child modules override parent modules
        for profile_id in &chain {
            if let Ok(profile) = self.load(profile_id) {
                for module in &profile.modules {
                    if !seen.contains(module) {
                        seen.insert(module.clone());
                        modules.push(module.clone());
                    }
                }
            }
        }

        Ok(modules)
    }

    fn for_bundle(&self, bundle_id: &str) -> IronResult<Vec<Profile>> {
        let all_profiles = self.discover()?;
        Ok(all_profiles
            .into_iter()
            .filter(|p| {
                p.for_bundle
                    .as_ref()
                    .map(|b| b == bundle_id)
                    .unwrap_or(true)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{DotfileMapping, Module, ModuleKind};
    use crate::services::module::DefaultModuleService;
    use tempfile::TempDir;

    fn create_test_profile(dir: &Path, id: &str, extends: Option<&str>) {
        let profile_dir = dir.join("profiles").join(id);
        fs::create_dir_all(&profile_dir).unwrap();

        let profile = Profile {
            id: id.to_string(),
            name: format!("Test Profile {}", id),
            description: Some("A test profile".to_string()),
            modules: vec![format!("{}-module", id)],
            theme: None,
            shell: None,
            extends: extends.map(|s| s.to_string()),
            for_bundle: None,
        };

        let config_path = profile_dir.join("profile.toml");
        let content = toml::to_string_pretty(&profile).unwrap();
        fs::write(config_path, content).unwrap();
    }

    fn create_test_module(dir: &Path, id: &str) {
        let module_dir = dir.join("modules").join(id);
        fs::create_dir_all(&module_dir).unwrap();

        let module = Module {
            id: id.to_string(),
            name: format!("Test Module {}", id),
            description: None,
            kind: ModuleKind::AppConfig,
            packages: vec![],
            aur_packages: vec![],
            dotfiles: vec![DotfileMapping {
                source: "config".to_string(),
                target: format!("~/.config/{}", id),
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
            security_points: 0,
            hook_behavior: crate::module::HookBehavior::default(),
            dotfiles_sync: false,
            dotfiles_sync_target: None,
        };

        let config_path = module_dir.join("module.toml");
        let content = toml::to_string_pretty(&module).unwrap();
        fs::write(config_path, content).unwrap();
    }

    fn create_test_service() -> (DefaultProfileService<DefaultModuleService>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        state_manager.set_current_host("test-host").unwrap();
        let module_service = DefaultModuleService::new(temp_dir.path(), state_manager.clone());
        let service = DefaultProfileService::new(temp_dir.path(), state_manager, module_service);
        (service, temp_dir)
    }

    #[test]
    fn test_discover_profiles() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "minimal", None);
        create_test_profile(temp_dir.path(), "full", None);

        let profiles = service.discover().unwrap();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_load_profile() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "test", None);

        let profile = service.load("test").unwrap();
        assert_eq!(profile.id, "test");
    }

    #[test]
    fn test_profile_inheritance() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "base", None);
        create_test_profile(temp_dir.path(), "extended", Some("base"));

        let chain = service.resolve_inheritance("extended").unwrap();
        assert_eq!(chain, vec!["extended", "base"]);
    }

    #[test]
    fn test_effective_modules() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "base", None);
        create_test_profile(temp_dir.path(), "child", Some("base"));

        let modules = service.effective_modules("child").unwrap();
        assert!(modules.contains(&"child-module".to_string()));
        assert!(modules.contains(&"base-module".to_string()));
    }

    #[test]
    fn test_profile_state_inactive() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "test", None);
        create_test_module(temp_dir.path(), "test-module");

        let state = service.state("test").unwrap();
        assert_eq!(state, ProfileState::Inactive);
    }

    #[test]
    fn test_profile_not_found() {
        let (service, _temp) = create_test_service();

        let result = service.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_active_profile_none() {
        let (service, _temp) = create_test_service();

        let active = service.active().unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn test_discover_empty() {
        let (service, _temp) = create_test_service();

        let profiles = service.discover().unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_circular_inheritance() {
        let (service, temp_dir) = create_test_service();

        // Create profiles that extend each other (circular)
        let profile_a_dir = temp_dir.path().join("profiles").join("a");
        let profile_b_dir = temp_dir.path().join("profiles").join("b");
        fs::create_dir_all(&profile_a_dir).unwrap();
        fs::create_dir_all(&profile_b_dir).unwrap();

        let profile_a = Profile {
            id: "a".to_string(),
            name: "Profile A".to_string(),
            description: None,
            modules: vec!["mod-a".to_string()],
            theme: None,
            shell: None,
            extends: Some("b".to_string()),
            for_bundle: None,
        };

        let profile_b = Profile {
            id: "b".to_string(),
            name: "Profile B".to_string(),
            description: None,
            modules: vec!["mod-b".to_string()],
            theme: None,
            shell: None,
            extends: Some("a".to_string()),
            for_bundle: None,
        };

        fs::write(
            profile_a_dir.join("profile.toml"),
            toml::to_string_pretty(&profile_a).unwrap(),
        )
        .unwrap();
        fs::write(
            profile_b_dir.join("profile.toml"),
            toml::to_string_pretty(&profile_b).unwrap(),
        )
        .unwrap();

        // Should handle circular inheritance gracefully
        let chain = service.resolve_inheritance("a").unwrap();
        // Should not contain infinite loop
        assert!(chain.len() <= 2);
    }

    fn create_bundle_profile(dir: &Path, id: &str, bundle: &str) {
        let profile_dir = dir.join("profiles").join(id);
        fs::create_dir_all(&profile_dir).unwrap();

        let profile = Profile {
            id: id.to_string(),
            name: format!("Profile for {}", bundle),
            description: Some("Bundle-specific profile".to_string()),
            modules: vec![format!("{}-module", id)],
            theme: None,
            shell: None,
            extends: None,
            for_bundle: Some(bundle.to_string()),
        };

        let config_path = profile_dir.join("profile.toml");
        let content = toml::to_string_pretty(&profile).unwrap();
        fs::write(config_path, content).unwrap();
    }

    #[test]
    fn test_for_bundle() {
        let (service, temp_dir) = create_test_service();

        // Create profiles for different bundles
        create_bundle_profile(temp_dir.path(), "hyprland-profile", "hyprland");
        create_bundle_profile(temp_dir.path(), "niri-profile", "niri");
        create_test_profile(temp_dir.path(), "generic", None);

        // Get profiles for hyprland bundle
        let hyprland_profiles = service.for_bundle("hyprland").unwrap();

        // Should include hyprland-specific and generic (no for_bundle specified)
        assert!(hyprland_profiles.len() >= 1);

        let ids: Vec<&str> = hyprland_profiles.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"hyprland-profile") || ids.contains(&"generic"));
    }

    #[test]
    fn test_apply_profile() {
        let (service, temp_dir) = create_test_service();

        create_test_profile(temp_dir.path(), "test", None);
        create_test_module(temp_dir.path(), "test-module");

        // Apply should enable the modules
        service.apply("test").unwrap();

        // Active profile should be set
        let active = service.active().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "test");
    }

    #[test]
    fn test_profile_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        state_manager.set_current_host("test-host").unwrap();
        let module_service = DefaultModuleService::new(temp_dir.path(), state_manager.clone());
        let service = DefaultProfileService::new(temp_dir.path(), state_manager, module_service);

        assert!(service.profiles_dir.ends_with("profiles"));
    }

    #[test]
    fn test_deep_inheritance_chain() {
        let (service, temp_dir) = create_test_service();

        // Create deep inheritance: child -> parent -> grandparent
        create_test_profile(temp_dir.path(), "grandparent", None);
        create_test_profile(temp_dir.path(), "parent", Some("grandparent"));
        create_test_profile(temp_dir.path(), "child", Some("parent"));

        let chain = service.resolve_inheritance("child").unwrap();
        assert_eq!(chain, vec!["child", "parent", "grandparent"]);
    }

    #[test]
    fn test_effective_modules_no_duplicates() {
        let (service, temp_dir) = create_test_service();

        // Create profiles with overlapping modules
        let profile_dir = temp_dir.path().join("profiles").join("base");
        fs::create_dir_all(&profile_dir).unwrap();
        let base = Profile {
            id: "base".to_string(),
            name: "Base".to_string(),
            description: None,
            modules: vec!["shared".to_string(), "base-only".to_string()],
            theme: None,
            shell: None,
            extends: None,
            for_bundle: None,
        };
        fs::write(
            profile_dir.join("profile.toml"),
            toml::to_string_pretty(&base).unwrap(),
        )
        .unwrap();

        let child_dir = temp_dir.path().join("profiles").join("child");
        fs::create_dir_all(&child_dir).unwrap();
        let child = Profile {
            id: "child".to_string(),
            name: "Child".to_string(),
            description: None,
            modules: vec!["shared".to_string(), "child-only".to_string()],
            theme: None,
            shell: None,
            extends: Some("base".to_string()),
            for_bundle: None,
        };
        fs::write(
            child_dir.join("profile.toml"),
            toml::to_string_pretty(&child).unwrap(),
        )
        .unwrap();

        let modules = service.effective_modules("child").unwrap();

        // Should only have unique modules
        let unique: HashSet<_> = modules.iter().collect();
        assert_eq!(modules.len(), unique.len());
    }
}
