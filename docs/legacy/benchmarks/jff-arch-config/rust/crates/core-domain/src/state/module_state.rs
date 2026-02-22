use super::{OperationRecord, load_state, save_state};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

/// State for a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub module_id: String,
    pub last_applied: DateTime<Utc>,
    pub operations_executed: HashMap<String, OperationRecord>,
}

impl ModuleState {
    pub fn new(module_id: String) -> Self {
        ModuleState {
            module_id,
            last_applied: Utc::now(),
            operations_executed: HashMap::new(),
        }
    }

    pub fn load(root: &Path, module_id: &str) -> io::Result<ModuleState> {
        let path = module_state_path(root, module_id);
        load_state(&path)
    }

    pub fn save(&self, root: &Path) -> io::Result<()> {
        let path = module_state_path(root, &self.module_id);
        save_state(&path, self)
    }

    pub fn record_operation(
        &mut self,
        operation_id: String,
        exit_code: i32,
        duration_ms: u64,
    ) {
        self.last_applied = Utc::now();
        self.operations_executed.insert(
            operation_id.clone(),
            OperationRecord {
                operation_id,
                executed_at: Utc::now(),
                exit_code,
                duration_ms,
            },
        );
    }

    pub fn was_successful(&self, operation_id: &str) -> bool {
        self.operations_executed
            .get(operation_id)
            .map(|r| r.exit_code == 0)
            .unwrap_or(false)
    }
}

fn module_state_path(root: &Path, module_id: &str) -> PathBuf {
    root.join("app/state/tracking/module_states")
        .join(format!("{}.json", module_id))
}
