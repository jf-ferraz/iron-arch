//! Validation - Check configuration integrity

use thiserror::Error;
use std::path::PathBuf;

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required field: {field} in {file}")]
    MissingField { field: String, file: PathBuf },

    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Conflict detected: {module_a} conflicts with {module_b}")]
    ModuleConflict { module_a: String, module_b: String },

    #[error("Bundle conflict: {bundle_a} conflicts with {bundle_b}")]
    BundleConflict { bundle_a: String, bundle_b: String },

    #[error("Dotfile target conflict: {target} claimed by {module_a} and {module_b}")]
    DotfileConflict { target: String, module_a: String, module_b: String },

    #[error("Missing dependency: {module} requires {dependency}")]
    MissingDependency { module: String, dependency: String },

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
}

/// Validation warning (non-fatal)
#[derive(Debug)]
pub struct ValidationWarning {
    pub message: String,
    pub path: Option<PathBuf>,
}

/// Validation result
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, message: &str, path: Option<PathBuf>) {
        self.warnings.push(ValidationWarning {
            message: message.to_string(),
            path,
        });
    }
}

/// Validate entire Iron configuration
pub fn validate_config(root: &PathBuf) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Check required directories exist
    let required_dirs = ["bundles", "profiles", "modules", "hosts"];
    for dir in required_dirs {
        let path = root.join(dir);
        if !path.exists() {
            result.add_warning(&format!("Directory not found: {}", dir), Some(path));
        }
    }

    // TODO: Validate each bundle, profile, module
    // TODO: Check for conflicts
    // TODO: Verify dotfile targets don't overlap

    result
}
