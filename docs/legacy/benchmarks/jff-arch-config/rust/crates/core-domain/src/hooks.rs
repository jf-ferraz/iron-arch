use crate::state::{HookBehavior, HookState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Hook execution tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTracker {
    pub hooks: HashMap<String, HookState>,
}

impl HookTracker {
    pub fn new() -> Self {
        HookTracker {
            hooks: HashMap::new(),
        }
    }

    pub fn load(root: &Path) -> io::Result<HookTracker> {
        let path = root.join("app/state/tracking/hook_hashes.json");
        match fs::read_to_string(&path) {
            Ok(content) => {
                let data: HashMap<String, HookState> = serde_json::from_str(&content)?;
                Ok(HookTracker { hooks: data })
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(HookTracker::new()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, root: &Path) -> io::Result<()> {
        let path = root.join("app/state/tracking/hook_hashes.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.hooks)?;
        fs::write(path, content)
    }

    pub fn get_or_create(
        &mut self,
        hook_id: String,
        script_path: PathBuf,
        behavior: HookBehavior,
    ) -> io::Result<&mut HookState> {
        if !self.hooks.contains_key(&hook_id) {
            let state = HookState::new(hook_id.clone(), script_path, behavior)?;
            self.hooks.insert(hook_id.clone(), state);
        }
        Ok(self.hooks.get_mut(&hook_id).unwrap())
    }

    pub fn should_execute(&mut self, hook_id: &str) -> io::Result<bool> {
        if let Some(state) = self.hooks.get_mut(hook_id) {
            // Check if script has changed
            let has_changed = state.has_changed()?;
            if has_changed {
                // Script changed - reset execution count for "once" behavior
                if state.behavior == HookBehavior::Once {
                    state.execution_count = 0;
                }
                state.update_hash()?;
            }

            Ok(state.should_execute())
        } else {
            // Hook not tracked yet - default to execute
            Ok(true)
        }
    }

    pub fn record_execution(&mut self, hook_id: &str) {
        if let Some(state) = self.hooks.get_mut(hook_id) {
            state.record_execution();
        }
    }

    pub fn set_behavior(&mut self, hook_id: &str, behavior: HookBehavior) -> io::Result<()> {
        if let Some(state) = self.hooks.get_mut(hook_id) {
            state.behavior = behavior;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("hook not found: {hook_id}"),
            ))
        }
    }

    pub fn list_hooks(&self) -> Vec<(String, &HookState)> {
        let mut hooks: Vec<_> = self.hooks.iter().map(|(id, state)| (id.clone(), state)).collect();
        hooks.sort_by(|a, b| a.0.cmp(&b.0));
        hooks
    }
}

impl Default for HookTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a hook with smart tracking
pub fn execute_hook(
    root: &Path,
    hook_id: &str,
    script_path: &Path,
    behavior: HookBehavior,
) -> io::Result<bool> {
    let mut tracker = HookTracker::load(root)?;

    // Register hook if not already tracked
    let _ = tracker.get_or_create(
        hook_id.to_string(),
        script_path.to_path_buf(),
        behavior,
    )?;

    // Check if should execute
    let should_exec = tracker.should_execute(hook_id)?;

    if should_exec {
        // Execute the hook (this would be done by the caller)
        // Here we just record that it will be executed
        tracker.record_execution(hook_id);
        tracker.save(root)?;
        Ok(true)
    } else {
        // Skip execution
        Ok(false)
    }
}

/// List all tracked hooks
pub fn list_tracked_hooks(root: &Path) -> io::Result<Vec<(String, HookState)>> {
    let tracker = HookTracker::load(root)?;
    Ok(tracker
        .list_hooks()
        .into_iter()
        .map(|(id, state)| (id, state.clone()))
        .collect())
}

/// Reset hook execution history
pub fn reset_hook(root: &Path, hook_id: &str) -> io::Result<()> {
    let mut tracker = HookTracker::load(root)?;

    if let Some(state) = tracker.hooks.get_mut(hook_id) {
        state.execution_count = 0;
        state.last_executed = None;
        tracker.save(root)?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("hook not found: {hook_id}"),
        ))
    }
}

/// Set hook behavior
pub fn set_hook_behavior(root: &Path, hook_id: &str, behavior: HookBehavior) -> io::Result<()> {
    let mut tracker = HookTracker::load(root)?;
    tracker.set_behavior(hook_id, behavior)?;
    tracker.save(root)
}
