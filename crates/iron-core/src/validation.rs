//! Validation - Check configuration integrity
//!
//! This module provides comprehensive validation for Iron configurations:
//! - ID format validation
//! - Path safety validation
//! - Module conflict detection
//! - Dependency resolution
//! - Dotfile target conflict detection

use crate::error::ValidationError;
use crate::module::Module;
use crate::IronResult;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Maximum allowed length for identifiers
pub const MAX_ID_LENGTH: usize = 64;

/// Validation warning (non-fatal)
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,

    /// Related path (if any)
    pub path: Option<PathBuf>,

    /// Warning code for categorization
    pub code: WarningCode,
}

/// Warning codes for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    /// Directory doesn't exist
    MissingDirectory,

    /// Optional field not set
    MissingOptionalField,

    /// Deprecated feature used
    Deprecated,

    /// Potential issue detected
    PotentialIssue,
}

/// Validation result containing errors and warnings
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Fatal errors that prevent operation
    pub errors: Vec<ValidationError>,

    /// Non-fatal warnings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if validation is clean (no errors or warnings)
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, message: impl Into<String>, path: Option<PathBuf>, code: WarningCode) {
        self.warnings.push(ValidationWarning {
            message: message.into(),
            path,
            code,
        });
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Convert to Result, returning Err if there are any errors
    pub fn into_result(self) -> IronResult<Vec<ValidationWarning>> {
        if let Some(error) = self.errors.into_iter().next() {
            Err(error.into())
        } else {
            Ok(self.warnings)
        }
    }
}

/// Validate an identifier (bundle ID, profile ID, module ID, host ID)
///
/// IDs must be:
/// - Lowercase alphanumeric with hyphens
/// - Start with a letter
/// - Not end with a hyphen
/// - Maximum 64 characters
pub fn validate_id(id: &str) -> Result<(), ValidationError> {
    // Check length
    if id.is_empty() {
        return Err(ValidationError::InvalidIdFormat {
            id: id.to_string(),
        });
    }

    if id.len() > MAX_ID_LENGTH {
        return Err(ValidationError::IdTooLong {
            id: id.to_string(),
            max: MAX_ID_LENGTH,
        });
    }

    // Check first character is a letter
    if !id.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
        return Err(ValidationError::InvalidIdFormat {
            id: id.to_string(),
        });
    }

    // Check last character is not a hyphen
    if id.ends_with('-') {
        return Err(ValidationError::InvalidIdFormat {
            id: id.to_string(),
        });
    }

    // Check all characters are valid
    for c in id.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(ValidationError::InvalidIdFormat {
                id: id.to_string(),
            });
        }
    }

    // Check for double hyphens
    if id.contains("--") {
        return Err(ValidationError::InvalidIdFormat {
            id: id.to_string(),
        });
    }

    Ok(())
}

/// Validate a path is safe (doesn't escape allowed root)
pub fn validate_path(path: &Path, allowed_root: &Path) -> Result<(), ValidationError> {
    // Canonicalize paths to resolve .. and symlinks
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist yet, check components manually
            let mut components_count: i32 = 0;
            for component in path.components() {
                match component {
                    std::path::Component::ParentDir => {
                        components_count -= 1;
                        if components_count < 0 {
                            return Err(ValidationError::InvalidPath {
                                path: path.to_path_buf(),
                                message: "Path escapes root via parent directory references".to_string(),
                            });
                        }
                    }
                    std::path::Component::Normal(_) => {
                        components_count += 1;
                    }
                    _ => {}
                }
            }
            return Ok(());
        }
    };

    let canonical_root = allowed_root.canonicalize().map_err(|_| ValidationError::InvalidPath {
        path: allowed_root.to_path_buf(),
        message: "Root path does not exist".to_string(),
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(ValidationError::InvalidPath {
            path: path.to_path_buf(),
            message: format!(
                "Path escapes allowed root: {}",
                canonical_root.display()
            ),
        });
    }

    Ok(())
}

/// Validate that a path doesn't contain dangerous components
pub fn validate_path_safe(path: &Path) -> Result<(), ValidationError> {
    let path_str = path.to_string_lossy();

    // Check for null bytes
    if path_str.contains('\0') {
        return Err(ValidationError::InvalidPath {
            path: path.to_path_buf(),
            message: "Path contains null bytes".to_string(),
        });
    }

    // Check for path traversal attempts
    if path_str.contains("..") {
        return Err(ValidationError::InvalidPath {
            path: path.to_path_buf(),
            message: "Path contains parent directory reference (..)".to_string(),
        });
    }

    Ok(())
}

/// Expand ~ to home directory
pub fn expand_home(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with('~')
        && let Some(home) = dirs::home_dir() {
            return PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1));
        }
    path.to_path_buf()
}

/// Module dependency resolution
pub fn resolve_dependencies(
    modules: &[Module],
    requested: &[String],
) -> Result<Vec<String>, ValidationError> {
    let module_map: HashMap<&str, &Module> = modules.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut resolved: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();

    fn visit(
        module_id: &str,
        module_map: &HashMap<&str, &Module>,
        resolved: &mut Vec<String>,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<(), ValidationError> {
        if in_stack.contains(module_id) {
            path.push(module_id.to_string());
            return Err(ValidationError::CircularDependency {
                chain: path.join(" -> "),
            });
        }

        if visited.contains(module_id) {
            return Ok(());
        }

        let module = module_map.get(module_id).ok_or_else(|| ValidationError::MissingDependency {
            module: path.last().cloned().unwrap_or_else(|| "unknown".to_string()),
            dependency: module_id.to_string(),
        })?;

        in_stack.insert(module_id.to_string());
        path.push(module_id.to_string());

        for dep in &module.depends {
            visit(dep, module_map, resolved, visited, in_stack, path)?;
        }

        path.pop();
        in_stack.remove(module_id);
        visited.insert(module_id.to_string());
        resolved.push(module_id.to_string());

        Ok(())
    }

    for module_id in requested {
        visit(
            module_id,
            &module_map,
            &mut resolved,
            &mut visited,
            &mut in_stack,
            &mut Vec::new(),
        )?;
    }

    Ok(resolved)
}

/// Check for conflicts between modules
pub fn check_module_conflicts(
    modules: &[Module],
    enabled: &[String],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let module_map: HashMap<&str, &Module> = modules.iter().map(|m| (m.id.as_str(), m)).collect();

    // Check explicit conflicts
    for (i, module_id) in enabled.iter().enumerate() {
        if let Some(module) = module_map.get(module_id.as_str()) {
            for other_id in enabled.iter().skip(i + 1) {
                if module.conflicts.contains(other_id) {
                    errors.push(ValidationError::ModuleConflict {
                        module_a: module_id.clone(),
                        module_b: other_id.clone(),
                    });
                }
            }
        }
    }

    errors
}

/// Check for dotfile target conflicts
pub fn check_dotfile_conflicts(
    modules: &[Module],
    enabled: &[String],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut target_owners: HashMap<String, String> = HashMap::new();

    for module_id in enabled {
        if let Some(module) = modules.iter().find(|m| &m.id == module_id) {
            for dotfile in &module.dotfiles {
                let target = expand_home(Path::new(&dotfile.target))
                    .to_string_lossy()
                    .to_string();

                if let Some(existing_owner) = target_owners.get(&target) {
                    errors.push(ValidationError::DotfileConflict {
                        target,
                        module_a: existing_owner.clone(),
                        module_b: module_id.clone(),
                    });
                } else {
                    target_owners.insert(target, module_id.clone());
                }
            }
        }
    }

    errors
}

/// Validate entire Iron configuration directory
pub fn validate_config(root: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check required directories exist
    let required_dirs = ["bundles", "profiles", "modules", "hosts"];
    for dir in required_dirs {
        let path = root.join(dir);
        if !path.exists() {
            result.add_warning(
                format!("Directory not found: {}", dir),
                Some(path),
                WarningCode::MissingDirectory,
            );
        }
    }

    // Check .iron state directory
    let state_dir = root.join(".iron").join("state");
    if !state_dir.exists() {
        result.add_warning(
            "State directory not initialized. Run 'iron init'.",
            Some(state_dir),
            WarningCode::MissingDirectory,
        );
    }

    result
}

/// Validate a module configuration
pub fn validate_module(module: &Module, modules_root: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Validate ID
    if let Err(e) = validate_id(&module.id) {
        result.add_error(e);
    }

    // Check dotfile sources exist
    let module_dir = modules_root.join(&module.id);
    for dotfile in &module.dotfiles {
        let source_path = module_dir.join(&dotfile.source);
        if !source_path.exists() {
            result.add_warning(
                format!("Dotfile source not found: {}", dotfile.source),
                Some(source_path),
                WarningCode::PotentialIssue,
            );
        }

        // Validate target path
        if let Err(e) = validate_path_safe(Path::new(&dotfile.target)) {
            result.add_error(e);
        }
    }

    // Check for self-referencing dependencies
    if module.depends.contains(&module.id) {
        result.add_error(ValidationError::CircularDependency {
            chain: format!("{} -> {}", module.id, module.id),
        });
    }

    result
}

// Re-export ValidationError from error module (already imported at top)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{DotfileMapping, ModuleKind};

    #[test]
    fn test_valid_ids() {
        assert!(validate_id("nvim-ide").is_ok());
        assert!(validate_id("kitty-dev").is_ok());
        assert!(validate_id("fish").is_ok());
        assert!(validate_id("a").is_ok());
        assert!(validate_id("module123").is_ok());
    }

    #[test]
    fn test_invalid_ids() {
        // Empty
        assert!(validate_id("").is_err());

        // Starts with number
        assert!(validate_id("123abc").is_err());

        // Starts with hyphen
        assert!(validate_id("-foo").is_err());

        // Ends with hyphen
        assert!(validate_id("foo-").is_err());

        // Contains uppercase
        assert!(validate_id("FooBar").is_err());

        // Contains spaces
        assert!(validate_id("foo bar").is_err());

        // Contains underscore
        assert!(validate_id("foo_bar").is_err());

        // Double hyphen
        assert!(validate_id("foo--bar").is_err());

        // Too long
        let long_id = "a".repeat(MAX_ID_LENGTH + 1);
        assert!(validate_id(&long_id).is_err());
    }

    #[test]
    fn test_path_safety() {
        assert!(validate_path_safe(Path::new("/home/user/.config")).is_ok());
        assert!(validate_path_safe(Path::new("config/nvim")).is_ok());

        assert!(validate_path_safe(Path::new("../../../etc/passwd")).is_err());
        assert!(validate_path_safe(Path::new("/home/../etc")).is_err());
    }

    #[test]
    fn test_expand_home() {
        let path = Path::new("~/.config/nvim");
        let expanded = expand_home(path);

        // Should not start with ~
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn test_dependency_resolution() {
        let modules = vec![
            Module {
                id: "a".to_string(),
                name: "Module A".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec![],
                depends: vec!["b".to_string()],
                pre_install: None,
                post_install: None,
            },
            Module {
                id: "b".to_string(),
                name: "Module B".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec![],
                depends: vec![],
                pre_install: None,
                post_install: None,
            },
        ];

        let result = resolve_dependencies(&modules, &["a".to_string()]).unwrap();
        assert_eq!(result, vec!["b", "a"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let modules = vec![
            Module {
                id: "a".to_string(),
                name: "Module A".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec![],
                depends: vec!["b".to_string()],
                pre_install: None,
                post_install: None,
            },
            Module {
                id: "b".to_string(),
                name: "Module B".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec![],
                depends: vec!["a".to_string()],
                pre_install: None,
                post_install: None,
            },
        ];

        let result = resolve_dependencies(&modules, &["a".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_module_conflict_detection() {
        let modules = vec![
            Module {
                id: "vim".to_string(),
                name: "Vim".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec!["nvim".to_string()],
                depends: vec![],
                pre_install: None,
                post_install: None,
            },
            Module {
                id: "nvim".to_string(),
                name: "Neovim".to_string(),
                description: None,
                kind: ModuleKind::AppConfig,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![],
                conflicts: vec!["vim".to_string()],
                depends: vec![],
                pre_install: None,
                post_install: None,
            },
        ];

        let conflicts =
            check_module_conflicts(&modules, &["vim".to_string(), "nvim".to_string()]);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_dotfile_conflict_detection() {
        let modules = vec![
            Module {
                id: "fish-a".to_string(),
                name: "Fish A".to_string(),
                description: None,
                kind: ModuleKind::Shell,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![DotfileMapping {
                    source: "config".to_string(),
                    target: "~/.config/fish".to_string(),
                    link: true,
                }],
                conflicts: vec![],
                depends: vec![],
                pre_install: None,
                post_install: None,
            },
            Module {
                id: "fish-b".to_string(),
                name: "Fish B".to_string(),
                description: None,
                kind: ModuleKind::Shell,
                packages: vec![],
                aur_packages: vec![],
                dotfiles: vec![DotfileMapping {
                    source: "config".to_string(),
                    target: "~/.config/fish".to_string(),
                    link: true,
                }],
                conflicts: vec![],
                depends: vec![],
                pre_install: None,
                post_install: None,
            },
        ];

        let conflicts =
            check_dotfile_conflicts(&modules, &["fish-a".to_string(), "fish-b".to_string()]);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(result.is_clean());

        result.add_warning("test warning", None, WarningCode::PotentialIssue);
        assert!(result.is_valid());
        assert!(!result.is_clean());

        result.add_error(ValidationError::InvalidIdFormat {
            id: "bad".to_string(),
        });
        assert!(!result.is_valid());
    }
}
