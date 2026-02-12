//! State management - Track active configurations

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::collections::HashMap;

/// Global Iron state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IronState {
    /// Current host ID
    pub current_host: Option<String>,

    /// Active bundle per host
    pub active_bundles: HashMap<String, String>,

    /// Active profile per host
    pub active_profiles: HashMap<String, String>,

    /// Active modules
    pub active_modules: Vec<String>,

    /// Last operations
    pub last_operations: Vec<OperationRecord>,

    /// Maintenance timestamps
    pub maintenance: MaintenanceState,
}

/// Record of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub status: OperationStatus,
    pub details: Option<String>,
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Success,
    Failed,
    Partial,
    Skipped,
}

/// Maintenance operation timestamps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaintenanceState {
    pub last_update: Option<DateTime<Utc>>,
    pub last_clean: Option<DateTime<Utc>>,
    pub last_doctor: Option<DateTime<Utc>>,
    pub last_snapshot: Option<DateTime<Utc>>,
    pub last_sync: Option<DateTime<Utc>>,
}

impl IronState {
    /// Load state from file
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let state: IronState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::default())
        }
    }

    /// Save state to file
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Record an operation
    pub fn record_operation(&mut self, operation: &str, status: OperationStatus, details: Option<String>) {
        self.last_operations.push(OperationRecord {
            operation: operation.to_string(),
            timestamp: Utc::now(),
            status,
            details,
        });

        // Keep only last 100 operations
        if self.last_operations.len() > 100 {
            self.last_operations = self.last_operations.split_off(self.last_operations.len() - 100);
        }
    }
}
