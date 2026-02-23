//! History Service -- Query operation history
//!
//! F3-016: Reads from IronState.last_operations via StateManager.
//! Provides a display model (HistoryEntry) with 1-based indexing.

use crate::IronResult;
use crate::services::state::StateManager;
use crate::state::{OperationRecord, OperationStatus};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// A history entry for display purposes.
/// Wraps OperationRecord with a 1-based display index.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    /// 1-based index (most recent = 1)
    pub index: usize,
    /// Operation name (e.g., "apply", "update", "snapshot restore")
    pub operation: String,
    /// When it happened
    pub timestamp: DateTime<Utc>,
    /// How long it took
    pub duration_secs: Option<f64>,
    /// Number of actions
    pub action_count: Option<usize>,
    /// Outcome
    pub status: OperationStatus,
    /// Detailed breakdown (action list, errors)
    pub details: Option<String>,
}

/// Service for querying operation history.
pub trait HistoryService {
    /// List recent operations, most recent first.
    fn list(&self, limit: usize) -> IronResult<Vec<HistoryEntry>>;

    /// Get a specific operation by 1-based index.
    fn show(&self, index: usize) -> IronResult<Option<HistoryEntry>>;

    /// Get the most recent operation.
    fn last(&self) -> IronResult<Option<HistoryEntry>>;
}

/// Default implementation reading from StateManager.
pub struct DefaultHistoryService {
    state_manager: StateManager,
}

impl DefaultHistoryService {
    pub fn new(state_manager: StateManager) -> Self {
        Self { state_manager }
    }

    /// Convert OperationRecord to HistoryEntry with 1-based indexing.
    fn to_entry(record: &OperationRecord, index: usize) -> HistoryEntry {
        HistoryEntry {
            index,
            operation: record.operation.clone(),
            timestamp: record.timestamp,
            duration_secs: record.duration_secs,
            action_count: record.action_count,
            status: record.status.clone(),
            details: record.details.clone(),
        }
    }
}

impl HistoryService for DefaultHistoryService {
    fn list(&self, limit: usize) -> IronResult<Vec<HistoryEntry>> {
        let state = self.state_manager.state();
        let ops = &state.last_operations;
        // Reverse chronological (most recent first)
        let entries: Vec<HistoryEntry> = ops
            .iter()
            .rev()
            .take(limit)
            .enumerate()
            .map(|(i, r)| Self::to_entry(r, i + 1))
            .collect();
        Ok(entries)
    }

    fn show(&self, index: usize) -> IronResult<Option<HistoryEntry>> {
        let state = self.state_manager.state();
        let ops = &state.last_operations;
        if index == 0 || index > ops.len() {
            return Ok(None);
        }
        // index 1 = most recent = last element
        let record_index = ops.len() - index;
        Ok(Some(Self::to_entry(&ops[record_index], index)))
    }

    fn last(&self) -> IronResult<Option<HistoryEntry>> {
        self.show(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::OperationStatus;
    use tempfile::TempDir;

    fn create_state_manager_with_ops(ops: Vec<(&str, OperationStatus)>) -> (TempDir, StateManager) {
        let temp = TempDir::new().unwrap();
        let sm = StateManager::new(temp.path().to_path_buf()).unwrap();
        for (op, status) in ops {
            sm.record_operation(op, status, None).unwrap();
        }
        (temp, sm)
    }

    #[test]
    fn test_history_empty() {
        let temp = TempDir::new().unwrap();
        let sm = StateManager::new(temp.path().to_path_buf()).unwrap();
        let svc = DefaultHistoryService::new(sm);

        let entries = svc.list(10).unwrap();
        assert!(entries.is_empty());

        let last = svc.last().unwrap();
        assert!(last.is_none());
    }

    #[test]
    fn test_history_list_reverse_chronological() {
        let (_temp, sm) = create_state_manager_with_ops(vec![
            ("apply", OperationStatus::Success),
            ("update", OperationStatus::Failed),
            ("snapshot", OperationStatus::Success),
        ]);
        let svc = DefaultHistoryService::new(sm);

        let entries = svc.list(10).unwrap();
        assert_eq!(entries.len(), 3);
        // Most recent first
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[0].operation, "snapshot");
        assert_eq!(entries[1].index, 2);
        assert_eq!(entries[1].operation, "update");
        assert_eq!(entries[2].index, 3);
        assert_eq!(entries[2].operation, "apply");
    }

    #[test]
    fn test_history_limit() {
        let (_temp, sm) = create_state_manager_with_ops(vec![
            ("op1", OperationStatus::Success),
            ("op2", OperationStatus::Success),
            ("op3", OperationStatus::Success),
            ("op4", OperationStatus::Success),
            ("op5", OperationStatus::Success),
        ]);
        let svc = DefaultHistoryService::new(sm);

        let entries = svc.list(3).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].operation, "op5");
        assert_eq!(entries[2].operation, "op3");
    }

    #[test]
    fn test_history_show_detail() {
        let (_temp, sm) = create_state_manager_with_ops(vec![
            ("apply", OperationStatus::Success),
            ("update", OperationStatus::Partial),
        ]);
        let svc = DefaultHistoryService::new(sm);

        // index 1 = most recent = "update"
        let entry = svc.show(1).unwrap().unwrap();
        assert_eq!(entry.operation, "update");
        assert_eq!(entry.index, 1);

        // index 2 = "apply"
        let entry = svc.show(2).unwrap().unwrap();
        assert_eq!(entry.operation, "apply");
        assert_eq!(entry.index, 2);

        // Out of range
        assert!(svc.show(3).unwrap().is_none());
        assert!(svc.show(0).unwrap().is_none());
    }

    #[test]
    fn test_history_last_shortcut() {
        let (_temp, sm) = create_state_manager_with_ops(vec![
            ("first", OperationStatus::Success),
            ("second", OperationStatus::Failed),
        ]);
        let svc = DefaultHistoryService::new(sm);

        let last = svc.last().unwrap().unwrap();
        assert_eq!(last.operation, "second");
        assert_eq!(last.index, 1);
    }
}
