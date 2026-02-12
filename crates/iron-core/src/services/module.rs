//! Module Service - Module management and dotfile linking
//!
//! Provides module discovery, enable/disable, and hook execution.

use crate::module::{Module, ModuleState};
use crate::services::state::StateManager;
use crate::validation::expand_home;
use crate::{IronResult, StateError, ValidationError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Module service trait
pub trait ModuleService {
    /// Discover all modules in the repository
    fn discover(&self) -> IronResult<Vec<Module>>;

    /// Load a specific module by ID
    fn load(&self, id: &str) -> IronResult<Module>;

    /// Enable a module (link dotfiles, run hooks)
    fn enable(&self, id: &str) -> IronResult<()>;

    /// Disable a module (unlink dotfiles, run hooks)
    fn disable(&self, id: &str) -> IronResult<()>;

    /// Check if enabling a module would cause conflicts
    fn check_conflicts(&self, id: &str) -> IronResult<Vec<String>>;

    /// Get module status
    fn status(&self, id: &str) -> IronResult<ModuleState>;

    /// List all enabled modules
    fn list_enabled(&self) -> IronResult<Vec<Module>>;

    /// Get effective modules (from profile + explicit)
    fn effective_modules(&self, profile_modules: &[String]) -> IronResult<Vec<Module>>;
}

/// Default module service implementation
pub struct DefaultModuleService {
    /// Modules directory
    modules_dir: PathBuf,
    /// State manager reference
    state_manager: StateManager,
}

impl DefaultModuleService {
    /// Create a new module service
    pub fn new(iron_root: &Path, state_manager: StateManager) -> Self {
        Self {
            modules_dir: iron_root.join("modules"),
            state_manager,
        }
    }

    /// Get module config path
    fn module_path(&self, id: &str) -> PathBuf {
        self.modules_dir.join(id).join("module.toml")
    }

    /// Get module directory
    fn module_dir(&self, id: &str) -> PathBuf {
        self.modules_dir.join(id)
    }

    /// Create a symlink
    fn create_symlink(&self, source: &Path, target: &Path) -> IronResult<()> {
        // Create parent directories
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok();
        }

        // Backup existing file if needed
        if target.exists() && !target.is_symlink() {
            let backup = target.with_extension("iron-backup");
            fs::rename(target, &backup).ok();
        }

        // Remove existing symlink
        if target.is_symlink() {
            fs::remove_file(target).ok();
        }

        // Create symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(source, target).map_err(|_| {
            crate::FsError::PermissionDenied {
                path: target.to_path_buf(),
            }
        })?;

        Ok(())
    }

    /// Remove a symlink
    fn remove_symlink(&self, target: &Path, restore_backup: bool) -> IronResult<()> {
        if target.is_symlink() {
            fs::remove_file(target).ok();
        }

        if restore_backup {
            let backup = target.with_extension("iron-backup");
            if backup.exists() {
                fs::rename(&backup, target).ok();
            }
        }

        Ok(())
    }

    /// Execute a hook script
    fn run_hook(&self, module: &Module, hook_name: &str) -> IronResult<bool> {
        let hook_path = self.module_dir(&module.id).join("hooks").join(hook_name);

        if !hook_path.exists() {
            return Ok(true); // No hook = success
        }

        let status = Command::new("bash")
            .arg(&hook_path)
            .current_dir(self.module_dir(&module.id))
            .env("IRON_MODULE_ID", &module.id)
            .env("IRON_MODULE_DIR", self.module_dir(&module.id))
            .status();

        match status {
            Ok(s) => Ok(s.success()),
            Err(_) => Ok(false),
        }
    }

    /// Get all dotfile mappings with resolved paths
    fn resolve_dotfiles(&self, module: &Module) -> Vec<(PathBuf, PathBuf)> {
        let module_dir = self.module_dir(&module.id);

        module
            .dotfiles
            .iter()
            .map(|df| {
                let source = module_dir.join(&df.source);
                let target = expand_home(Path::new(&df.target));
                (source, target)
            })
            .collect()
    }
}

impl ModuleService for DefaultModuleService {
    fn discover(&self) -> IronResult<Vec<Module>> {
        let mut modules = Vec::new();

        if !self.modules_dir.exists() {
            return Ok(modules);
        }

        for entry in fs::read_dir(&self.modules_dir).into_iter().flatten().flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(id) = entry.file_name().to_str() {
                    if let Ok(module) = self.load(id) {
                        modules.push(module);
                    }
                }
            }
        }

        Ok(modules)
    }

    fn load(&self, id: &str) -> IronResult<Module> {
        let path = self.module_path(id);
        if !path.exists() {
            return Err(StateError::ModuleNotFound { id: id.to_string() }.into());
        }

        let content = fs::read_to_string(&path).map_err(|_| StateError::ModuleNotFound {
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

    fn enable(&self, id: &str) -> IronResult<()> {
        let module = self.load(id)?;

        // Check conflicts first
        let conflicts = self.check_conflicts(id)?;
        if !conflicts.is_empty() {
            return Err(ValidationError::ModuleConflict {
                module_a: id.to_string(),
                module_b: conflicts.join(", "),
            }
            .into());
        }

        // Run pre-install hook
        if let Some(script) = &module.pre_install {
            let hook_path = self.module_dir(&module.id).join(script);
            if hook_path.exists() {
                let _ = Command::new("bash")
                    .arg(&hook_path)
                    .current_dir(self.module_dir(&module.id))
                    .status();
            }
        }

        // Link dotfiles
        for (source, target) in self.resolve_dotfiles(&module) {
            if source.exists() {
                self.create_symlink(&source, &target)?;
            }
        }

        // Update state
        self.state_manager.enable_module(id)?;

        // Run post-install hook
        if let Some(script) = &module.post_install {
            let hook_path = self.module_dir(&module.id).join(script);
            if hook_path.exists() {
                let _ = Command::new("bash")
                    .arg(&hook_path)
                    .current_dir(self.module_dir(&module.id))
                    .status();
            }
        }

        Ok(())
    }

    fn disable(&self, id: &str) -> IronResult<()> {
        let module = self.load(id)?;

        // Unlink dotfiles
        for (_, target) in self.resolve_dotfiles(&module) {
            self.remove_symlink(&target, true)?;
        }

        // Update state
        self.state_manager.disable_module(id)?;

        Ok(())
    }

    fn check_conflicts(&self, id: &str) -> IronResult<Vec<String>> {
        let module = self.load(id)?;
        let enabled = self.list_enabled()?;

        let mut conflicts = Vec::new();

        // Check explicit conflicts
        for enabled_mod in &enabled {
            if module.conflicts.contains(&enabled_mod.id) {
                conflicts.push(enabled_mod.id.clone());
            }
        }

        // Check dotfile target conflicts
        let module_targets: Vec<String> = module.dotfiles.iter().map(|d| d.target.clone()).collect();

        for enabled_mod in &enabled {
            for df in &enabled_mod.dotfiles {
                if module_targets.contains(&df.target) {
                    conflicts.push(format!("{}:{}", enabled_mod.id, df.target));
                }
            }
        }

        Ok(conflicts)
    }

    fn status(&self, id: &str) -> IronResult<ModuleState> {
        let module = self.load(id)?;

        if !self.state_manager.is_module_active(id) {
            return Ok(ModuleState::NotInstalled);
        }

        // Check if all dotfiles are properly linked
        let mut all_linked = true;
        let mut any_linked = false;

        for (source, target) in self.resolve_dotfiles(&module) {
            if target.is_symlink() {
                if let Ok(link_target) = fs::read_link(&target) {
                    if link_target == source {
                        any_linked = true;
                        continue;
                    }
                }
            }
            all_linked = false;
        }

        if all_linked && any_linked {
            Ok(ModuleState::Installed)
        } else if any_linked {
            Ok(ModuleState::Partial)
        } else {
            Ok(ModuleState::NotInstalled)
        }
    }

    fn list_enabled(&self) -> IronResult<Vec<Module>> {
        let active_ids = self.state_manager.active_modules();
        let mut enabled = Vec::new();

        for id in active_ids {
            if let Ok(module) = self.load(&id) {
                enabled.push(module);
            }
        }

        Ok(enabled)
    }

    fn effective_modules(&self, profile_modules: &[String]) -> IronResult<Vec<Module>> {
        let active_ids = self.state_manager.active_modules();
        let mut effective_ids: Vec<String> = profile_modules.to_vec();

        // Add explicitly enabled modules not in profile
        for id in &active_ids {
            if !effective_ids.contains(id) {
                effective_ids.push(id.clone());
            }
        }

        let mut modules = Vec::new();
        for id in effective_ids {
            if let Ok(module) = self.load(&id) {
                modules.push(module);
            }
        }

        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{DotfileMapping, ModuleKind};
    use tempfile::TempDir;

    fn create_test_module(dir: &Path, id: &str) {
        let module_dir = dir.join("modules").join(id);
        fs::create_dir_all(&module_dir).unwrap();

        let module = Module {
            id: id.to_string(),
            name: format!("Test Module {}", id),
            description: Some("A test module".to_string()),
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

        // Create dotfile source
        let config_dir = module_dir.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("test.conf"), "test content").unwrap();
    }

    fn create_test_service() -> (DefaultModuleService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_manager = StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let service = DefaultModuleService::new(temp_dir.path(), state_manager);
        (service, temp_dir)
    }

    #[test]
    fn test_discover_modules() {
        let (service, temp_dir) = create_test_service();

        create_test_module(temp_dir.path(), "mod1");
        create_test_module(temp_dir.path(), "mod2");

        let modules = service.discover().unwrap();
        assert_eq!(modules.len(), 2);
    }

    #[test]
    fn test_load_module() {
        let (service, temp_dir) = create_test_service();

        create_test_module(temp_dir.path(), "test-mod");

        let module = service.load("test-mod").unwrap();
        assert_eq!(module.id, "test-mod");
    }

    #[test]
    fn test_module_not_found() {
        let (service, _temp) = create_test_service();

        let result = service.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_conflicts() {
        let (service, _temp) = create_test_service();

        // No conflicts when nothing enabled
        let conflicts = service.check_conflicts("test-mod");
        assert!(conflicts.is_err()); // Module doesn't exist
    }

    #[test]
    fn test_status_not_installed() {
        let (service, temp_dir) = create_test_service();

        create_test_module(temp_dir.path(), "test-mod");

        let status = service.status("test-mod").unwrap();
        assert_eq!(status, ModuleState::NotInstalled);
    }
}
