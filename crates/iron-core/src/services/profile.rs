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
    fn build_inheritance_chain(&self, id: &str, visited: &mut HashSet<String>) -> IronResult<Vec<String>> {
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

        for entry in fs::read_dir(&self.profiles_dir).into_iter().flatten().flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(id) = entry.file_name().to_str() {
                    if let Ok(profile) = self.load(id) {
                        profiles.push(profile);
                    }
                }
            }
        }

        Ok(profiles)
    }

    fn load(&self, id: &str) -> IronResult<Profile> {
        let path = self.profile_path(id);
        if !path.exists() {
            return Err(StateError::ProfileNotFound { id: id.to_string() }.into());
        }

        let content = fs::read_to_string(&path).map_err(|_| StateError::ProfileNotFound {
            id: id.to_string(),
        })?;

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
            if let Ok(state) = self.module_service.status(module_id) {
                if state == ModuleState::Installed {
                    continue;
                }
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
            if let Ok(state) = self.module_service.status(module_id) {
                if state == ModuleState::Installed {
                    self.module_service.disable(module_id)?;
                }
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
            if let Ok(state) = self.module_service.status(module_id) {
                if state == ModuleState::Installed {
                    installed_count += 1;
                }
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
            .filter(|p| p.for_bundle.as_ref().map(|b| b == bundle_id).unwrap_or(true))
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
}
