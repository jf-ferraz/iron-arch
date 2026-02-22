use crate::state::ModuleState;
use crate::{load_modules, ModuleDescriptor};
use std::io;
use std::path::Path;
use std::process::Command;

/// Toggle module activation state
pub struct ModuleToggle {
    pub module_id: String,
    pub currently_active: bool,
    pub will_be_active: bool,
}

/// Get list of all modules with their activation status
pub fn get_all_modules_status(root: &Path) -> io::Result<Vec<(ModuleDescriptor, bool)>> {
    let modules = load_modules(root)?;
    let active = crate::get_active_modules(root)?;

    let mut result = Vec::new();
    for module in modules {
        let is_active = active.iter().any(|id| id == &module.id);
        result.push((module, is_active));
    }

    Ok(result)
}

/// Prepare module toggles for batch application
pub fn prepare_toggles(
    root: &Path,
    desired_active: &[String],
) -> io::Result<Vec<ModuleToggle>> {
    let current_active = crate::get_active_modules(root)?;
    let modules = load_modules(root)?;
    let mut toggles = Vec::new();

    for module in modules {
        let currently_active = current_active.iter().any(|id| id == &module.id);
        let will_be_active = desired_active.iter().any(|id| id == &module.id);

        if currently_active != will_be_active {
            toggles.push(ModuleToggle {
                module_id: module.id,
                currently_active,
                will_be_active,
            });
        }
    }

    Ok(toggles)
}

/// Apply module toggles (activate or deactivate modules)
pub fn apply_toggles(root: &Path, toggles: &[ModuleToggle]) -> io::Result<Vec<(String, bool)>> {
    let mut results = Vec::new();

    for toggle in toggles {
        let success = if toggle.will_be_active {
            activate_module_impl(root, &toggle.module_id)?
        } else {
            deactivate_module_impl(root, &toggle.module_id)?
        };

        results.push((toggle.module_id.clone(), success));
    }

    // Update active modules list
    let modules = load_modules(root)?;
    let mut new_active = Vec::new();
    for module in modules {
        let should_be_active = toggles
            .iter()
            .find(|t| t.module_id == module.id)
            .map(|t| t.will_be_active)
            .unwrap_or_else(|| {
                // Keep current state if not in toggles
                crate::is_module_active(root, &module.id).unwrap_or(false)
            });

        if should_be_active {
            new_active.push(module.id);
        }
    }

    crate::set_active_modules(root, new_active)?;

    Ok(results)
}

fn activate_module_impl(root: &Path, module_id: &str) -> io::Result<bool> {
    let modules = load_modules(root)?;
    let Some(module) = modules.iter().find(|m| m.id == module_id) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("module not found: {module_id}"),
        ));
    };

    // Look for deploy operation
    let operation = module
        .operations
        .iter()
        .find(|op| op.id == "deploy")
        .or_else(|| module.operations.first());

    let Some(op) = operation else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("module {module_id} has no operations"),
        ));
    };

    // Execute deploy operation
    let status = Command::new("sh")
        .arg("-c")
        .arg(&op.command)
        .current_dir(root)
        .status()?;

    let success = status.success();

    // Record state
    if success {
        let mut state = ModuleState::load(root, module_id).unwrap_or_else(|_| {
            ModuleState::new(module_id.to_string())
        });
        state.record_operation(
            op.id.clone(),
            status.code().unwrap_or(-1),
            0, // Duration not tracked here
        );
        let _ = state.save(root);
    }

    Ok(success)
}

fn deactivate_module_impl(root: &Path, module_id: &str) -> io::Result<bool> {
    let modules = load_modules(root)?;
    let Some(module) = modules.iter().find(|m| m.id == module_id) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("module not found: {module_id}"),
        ));
    };

    // Look for rollback operation
    let operation = module.operations.iter().find(|op| op.id == "rollback");

    if let Some(op) = operation {
        // Execute rollback operation
        let status = Command::new("sh")
            .arg("-c")
            .arg(&op.command)
            .current_dir(root)
            .status()?;

        let success = status.success();

        // Record state
        if success {
            let mut state = ModuleState::load(root, module_id).unwrap_or_else(|_| {
                ModuleState::new(module_id.to_string())
            });
            state.record_operation(
                op.id.clone(),
                status.code().unwrap_or(-1),
                0, // Duration not tracked here
            );
            let _ = state.save(root);
        }

        Ok(success)
    } else {
        // No rollback operation - just mark as inactive (success)
        Ok(true)
    }
}

/// Get module activation count
pub fn get_activation_stats(root: &Path) -> io::Result<(usize, usize)> {
    let modules = load_modules(root)?;
    let active = crate::get_active_modules(root)?;

    let active_count = active.len();
    let total_count = modules.len();

    Ok((active_count, total_count))
}
