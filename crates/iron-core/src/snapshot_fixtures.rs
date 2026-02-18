//! Test fixtures for mocking timeshift and snapper commands.
//!
//! This module provides pre-configured mock responses for snapshot tool commands,
//! enabling comprehensive testing of `TimeshiftManager` and `SnapperManager`
//! without requiring actual snapshot tool execution or root privileges.
//!
//! # Usage
//!
//! ```rust,ignore
//! use iron_core::resilience::MockCommandExecutor;
//! use iron_core::snapshot_fixtures::SnapshotMockBuilder;
//!
//! let executor = SnapshotMockBuilder::timeshift()
//!     .with_snapshot("1", "Pre-update backup", "2024-01-15_10-30-00")
//!     .with_snapshot("2", "Post-update", "2024-01-15_11-00-00")
//!     .build();
//!
//! // Use with command execution tests
//! ```

use crate::resilience::{MockCommandExecutor, MockResponse};
use crate::snapshot::SnapshotType;
use chrono::{DateTime, NaiveDateTime, Utc};

/// Snapshot fixture data representing a single snapshot
#[derive(Debug, Clone)]
pub struct SnapshotFixture {
    /// Unique snapshot identifier
    pub id: String,
    /// Snapshot description/comment
    pub description: String,
    /// Creation timestamp as string (format depends on backend)
    pub date_str: String,
    /// Parsed creation timestamp
    pub created: DateTime<Utc>,
    /// Snapshot type (single, pre, post, boot)
    pub snapshot_type: SnapshotType,
}

impl SnapshotFixture {
    /// Create a new snapshot fixture with Timeshift date format
    ///
    /// Date format: `YYYY-MM-DD_HH-MM-SS`
    pub fn timeshift(id: &str, description: &str, date_str: &str) -> Self {
        let created = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d_%H-%M-%S")
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(|_| Utc::now());

        Self {
            id: id.to_string(),
            description: description.to_string(),
            date_str: date_str.to_string(),
            created,
            snapshot_type: SnapshotType::Single,
        }
    }

    /// Create a new snapshot fixture with Snapper date format
    ///
    /// Date format: `%a %b %d %H:%M:%S %Y` (e.g., "Mon Jan 15 10:30:00 2024")
    /// or `%Y-%m-%d %H:%M:%S`
    pub fn snapper(id: &str, description: &str, date_str: &str) -> Self {
        let created = NaiveDateTime::parse_from_str(date_str, "%a %b %d %H:%M:%S %Y")
            .or_else(|_| NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S"))
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(|_| Utc::now());

        Self {
            id: id.to_string(),
            description: description.to_string(),
            date_str: date_str.to_string(),
            created,
            snapshot_type: SnapshotType::Single,
        }
    }

    /// Set the snapshot type
    pub fn with_type(mut self, snapshot_type: SnapshotType) -> Self {
        self.snapshot_type = snapshot_type;
        self
    }

    /// Generate timeshift --list line for this snapshot
    pub fn to_timeshift_list_line(&self) -> String {
        // Timeshift list format:
        // Num     Name                  Tags  Description
        // -----------------------------------------------
        // 0    >  2024-01-15_10-30-00   O     Pre-update backup
        format!(
            "{:4}    {}   O     {}",
            self.id, self.date_str, self.description
        )
    }

    /// Generate snapper list line for this snapshot
    pub fn to_snapper_list_line(&self) -> String {
        // Snapper list format with columns number,date,description:
        // # | Date                     | Description
        // --+--------------------------+----------------------
        // 1 | Mon Jan 15 10:30:00 2024 | Pre-update backup
        format!("{} | {} | {}", self.id, self.date_str, self.description)
    }
}

/// Backend type for the mock builder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockBackend {
    /// Mock timeshift commands
    Timeshift,
    /// Mock snapper commands
    Snapper,
    /// No backend available
    None,
}

/// Builder for creating configured `MockCommandExecutor` with snapshot-specific responses.
///
/// Provides a fluent API for setting up mock responses for timeshift and snapper commands,
/// enabling isolated testing of snapshot operations.
#[derive(Debug, Default)]
pub struct SnapshotMockBuilder {
    /// Backend type to mock
    backend: Option<MockBackend>,
    /// Configured snapshots
    snapshots: Vec<SnapshotFixture>,
    /// Snapper config name (default: "root")
    snapper_config: String,
    /// Whether create operations succeed
    create_succeeds: bool,
    /// Whether delete operations succeed
    delete_succeeds: bool,
    /// Whether restore operations succeed
    restore_succeeds: bool,
    /// Custom create output
    create_output: Option<String>,
    /// Custom list output (overrides auto-generated)
    custom_list_output: Option<String>,
    /// ID counter for auto-generated snapshot IDs on create
    next_snapshot_id: u32,
}

impl SnapshotMockBuilder {
    /// Create a new builder for Timeshift backend
    pub fn timeshift() -> Self {
        Self {
            backend: Some(MockBackend::Timeshift),
            snapper_config: "root".to_string(),
            create_succeeds: true,
            delete_succeeds: true,
            restore_succeeds: true,
            next_snapshot_id: 1,
            ..Default::default()
        }
    }

    /// Create a new builder for Snapper backend
    pub fn snapper() -> Self {
        Self {
            backend: Some(MockBackend::Snapper),
            snapper_config: "root".to_string(),
            create_succeeds: true,
            delete_succeeds: true,
            restore_succeeds: true,
            next_snapshot_id: 1,
            ..Default::default()
        }
    }

    /// Create a new builder with no backend available
    pub fn none() -> Self {
        Self {
            backend: Some(MockBackend::None),
            snapper_config: "root".to_string(),
            create_succeeds: false,
            delete_succeeds: false,
            restore_succeeds: false,
            ..Default::default()
        }
    }

    /// Set the snapper config name (default: "root")
    pub fn with_snapper_config(mut self, config: &str) -> Self {
        self.snapper_config = config.to_string();
        self
    }

    /// Add a snapshot to the list
    pub fn with_snapshot(mut self, id: &str, description: &str, date_str: &str) -> Self {
        let fixture = match self.backend {
            Some(MockBackend::Timeshift) => SnapshotFixture::timeshift(id, description, date_str),
            Some(MockBackend::Snapper) => SnapshotFixture::snapper(id, description, date_str),
            _ => SnapshotFixture::timeshift(id, description, date_str),
        };
        self.snapshots.push(fixture);
        self
    }

    /// Add a snapshot fixture directly
    pub fn with_snapshot_fixture(mut self, fixture: SnapshotFixture) -> Self {
        self.snapshots.push(fixture);
        self
    }

    /// Set whether create operations succeed
    pub fn create_succeeds(mut self, succeeds: bool) -> Self {
        self.create_succeeds = succeeds;
        self
    }

    /// Set whether delete operations succeed
    pub fn delete_succeeds(mut self, succeeds: bool) -> Self {
        self.delete_succeeds = succeeds;
        self
    }

    /// Set whether restore operations succeed
    pub fn restore_succeeds(mut self, succeeds: bool) -> Self {
        self.restore_succeeds = succeeds;
        self
    }

    /// Set custom create output (for testing parsing)
    pub fn with_create_output(mut self, output: &str) -> Self {
        self.create_output = Some(output.to_string());
        self
    }

    /// Set custom list output (overrides auto-generated)
    pub fn with_custom_list_output(mut self, output: &str) -> Self {
        self.custom_list_output = Some(output.to_string());
        self
    }

    /// Set the next snapshot ID for create operations
    pub fn with_next_id(mut self, id: u32) -> Self {
        self.next_snapshot_id = id;
        self
    }

    /// Generate timeshift --list output
    fn generate_timeshift_list_output(&self) -> String {
        if let Some(ref custom) = self.custom_list_output {
            return custom.clone();
        }

        let mut output = String::new();
        output.push_str("Num     Name                  Tags  Description\n");
        output.push_str("-----------------------------------------------\n");

        for snapshot in &self.snapshots {
            output.push_str(&snapshot.to_timeshift_list_line());
            output.push('\n');
        }

        output.push_str("-----------------------------------------------\n");
        output
    }

    /// Generate snapper list output
    fn generate_snapper_list_output(&self) -> String {
        if let Some(ref custom) = self.custom_list_output {
            return custom.clone();
        }

        let mut output = String::new();
        output.push_str(" # | Date                     | Description\n");
        output.push_str("---+--------------------------+----------------------\n");

        // Always include snapshot 0 (current)
        output.push_str(" 0 |                          | current\n");

        for snapshot in &self.snapshots {
            output.push_str(&snapshot.to_snapper_list_line());
            output.push('\n');
        }

        output
    }

    /// Build the configured `MockCommandExecutor`
    pub fn build(self) -> MockCommandExecutor {
        let executor = MockCommandExecutor::new();

        match self.backend {
            Some(MockBackend::Timeshift) => {
                self.configure_timeshift(&executor);
            }
            Some(MockBackend::Snapper) => {
                self.configure_snapper(&executor);
            }
            Some(MockBackend::None) | None => {
                self.configure_no_backend(&executor);
            }
        }

        // Configure 'which' command for backend detection
        self.configure_which(&executor);

        executor
    }

    /// Configure timeshift mock responses
    fn configure_timeshift(&self, executor: &MockCommandExecutor) {
        // Add timeshift to existing commands
        executor.add_existing_command("timeshift");

        // timeshift --help (for is_available check)
        executor.add_response(
            "timeshift",
            &["--help"],
            MockResponse::success("Timeshift v22.11.2 by Tony George (teejee2008@gmail.com)\n"),
        );

        // timeshift --list
        executor.add_response(
            "timeshift",
            &["--list"],
            MockResponse::success(&self.generate_timeshift_list_output()),
        );

        // timeshift --create --comments <description>
        // We need to handle any description, so use fallback
        if self.create_succeeds {
            let create_output = self.create_output.clone().unwrap_or_else(|| {
                format!(
                    "Creating new snapshot...\n\
                     Tagged snapshot '{}': Iron pre-update\n\
                     Snapshot saved successfully.\n",
                    self.next_snapshot_id
                )
            });
            executor.add_fallback_response("timeshift", MockResponse::success(&create_output));
        } else {
            executor.add_fallback_response(
                "timeshift",
                MockResponse::exit_error(1, "Failed to create snapshot: insufficient space"),
            );
        }

        // Configure delete for each existing snapshot
        for snapshot in &self.snapshots {
            if self.delete_succeeds {
                executor.add_response(
                    "timeshift",
                    &["--delete", "--snapshot", &snapshot.id],
                    MockResponse::success(&format!(
                        "Deleting snapshot: {}\nSnapshot deleted successfully.\n",
                        snapshot.id
                    )),
                );
            } else {
                executor.add_response(
                    "timeshift",
                    &["--delete", "--snapshot", &snapshot.id],
                    MockResponse::exit_error(1, "Failed to delete snapshot"),
                );
            }
        }

        // Configure restore for each existing snapshot
        for snapshot in &self.snapshots {
            if self.restore_succeeds {
                executor.add_response(
                    "timeshift",
                    &["--restore", "--snapshot", &snapshot.id, "--skip-grub"],
                    MockResponse::success(&format!(
                        "Restoring snapshot: {}\nRestore completed. Please reboot.\n",
                        snapshot.id
                    )),
                );
            } else {
                executor.add_response(
                    "timeshift",
                    &["--restore", "--snapshot", &snapshot.id, "--skip-grub"],
                    MockResponse::exit_error(1, "Failed to restore snapshot"),
                );
            }
        }
    }

    /// Configure snapper mock responses
    fn configure_snapper(&self, executor: &MockCommandExecutor) {
        // Add snapper to existing commands
        executor.add_existing_command("snapper");

        let config = &self.snapper_config;

        // snapper -c <config> list --columns number,date,description
        executor.add_response(
            "snapper",
            &["-c", config, "list", "--columns", "number,date,description"],
            MockResponse::success(&self.generate_snapper_list_output()),
        );

        // snapper -c <config> list (for is_available check)
        executor.add_response(
            "snapper",
            &["-c", config, "list"],
            MockResponse::success(&self.generate_snapper_list_output()),
        );

        // snapper -c <config> create -d <description> --print-number
        // Using fallback since description varies
        if self.create_succeeds {
            executor.add_fallback_response(
                "snapper",
                MockResponse::success(&format!("{}\n", self.next_snapshot_id)),
            );
        } else {
            executor.add_fallback_response(
                "snapper",
                MockResponse::exit_error(1, "Creating snapshot failed."),
            );
        }

        // Configure delete for each existing snapshot
        for snapshot in &self.snapshots {
            if self.delete_succeeds {
                executor.add_response(
                    "snapper",
                    &["-c", config, "delete", &snapshot.id],
                    MockResponse::success(""),
                );
            } else {
                executor.add_response(
                    "snapper",
                    &["-c", config, "delete", &snapshot.id],
                    MockResponse::exit_error(1, "Deleting snapshot failed."),
                );
            }
        }

        // Configure restore (undochange) for each existing snapshot
        for snapshot in &self.snapshots {
            let undochange_arg = format!("{}..0", snapshot.id);
            if self.restore_succeeds {
                executor.add_response(
                    "snapper",
                    &["-c", config, "undochange", &undochange_arg],
                    MockResponse::success("create:0 modify:15 delete:3\n"),
                );
            } else {
                executor.add_response(
                    "snapper",
                    &["-c", config, "undochange", &undochange_arg],
                    MockResponse::exit_error(1, "Undoing changes failed."),
                );
            }
        }
    }

    /// Configure responses for no backend available
    fn configure_no_backend(&self, executor: &MockCommandExecutor) {
        // Remove timeshift and snapper from existing commands (they don't exist)
        executor.remove_existing_command("timeshift");
        executor.remove_existing_command("snapper");

        // 'which' will fail for both
        executor.add_response("which", &["timeshift"], MockResponse::exit_error(1, ""));
        executor.add_response("which", &["snapper"], MockResponse::exit_error(1, ""));
    }

    /// Configure 'which' command for backend detection
    fn configure_which(&self, executor: &MockCommandExecutor) {
        match self.backend {
            Some(MockBackend::Timeshift) => {
                executor.add_response(
                    "which",
                    &["timeshift"],
                    MockResponse::success("/usr/bin/timeshift\n"),
                );
                executor.add_response("which", &["snapper"], MockResponse::exit_error(1, ""));
            }
            Some(MockBackend::Snapper) => {
                executor.add_response("which", &["timeshift"], MockResponse::exit_error(1, ""));
                executor.add_response(
                    "which",
                    &["snapper"],
                    MockResponse::success("/usr/bin/snapper\n"),
                );
            }
            Some(MockBackend::None) | None => {
                executor.add_response("which", &["timeshift"], MockResponse::exit_error(1, ""));
                executor.add_response("which", &["snapper"], MockResponse::exit_error(1, ""));
            }
        }
    }
}

// =============================================================================
// Pre-built Fixture Sets
// =============================================================================

/// Common snapshot scenarios for testing
pub mod fixtures {
    use super::*;

    /// Empty timeshift (no snapshots)
    pub fn timeshift_empty() -> SnapshotMockBuilder {
        SnapshotMockBuilder::timeshift()
    }

    /// Timeshift with multiple snapshots
    pub fn timeshift_with_snapshots() -> SnapshotMockBuilder {
        SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "Pre-update backup", "2024-01-15_10-30-00")
            .with_snapshot("2", "Post-update", "2024-01-15_11-00-00")
            .with_snapshot("3", "Manual backup", "2024-01-20_14-45-30")
    }

    /// Timeshift with various snapshot types
    pub fn timeshift_mixed_types() -> SnapshotMockBuilder {
        SnapshotMockBuilder::timeshift()
            .with_snapshot_fixture(
                SnapshotFixture::timeshift("1", "Boot snapshot", "2024-01-01_00-00-00")
                    .with_type(SnapshotType::Boot),
            )
            .with_snapshot_fixture(
                SnapshotFixture::timeshift("2", "Pre-update", "2024-01-15_10-00-00")
                    .with_type(SnapshotType::Pre),
            )
            .with_snapshot_fixture(
                SnapshotFixture::timeshift("3", "Post-update", "2024-01-15_11-00-00")
                    .with_type(SnapshotType::Post),
            )
    }

    /// Timeshift where operations fail
    pub fn timeshift_failing() -> SnapshotMockBuilder {
        SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "Existing snapshot", "2024-01-15_10-30-00")
            .create_succeeds(false)
            .delete_succeeds(false)
            .restore_succeeds(false)
    }

    /// Empty snapper (no user snapshots)
    pub fn snapper_empty() -> SnapshotMockBuilder {
        SnapshotMockBuilder::snapper()
    }

    /// Snapper with multiple snapshots
    pub fn snapper_with_snapshots() -> SnapshotMockBuilder {
        SnapshotMockBuilder::snapper()
            .with_snapshot("1", "First snapshot", "2024-01-15 10:30:00")
            .with_snapshot("2", "Second snapshot", "2024-01-15 11:00:00")
            .with_snapshot("3", "Third snapshot", "2024-01-20 14:45:30")
    }

    /// Snapper with home config
    pub fn snapper_home_config() -> SnapshotMockBuilder {
        SnapshotMockBuilder::snapper()
            .with_snapper_config("home")
            .with_snapshot("1", "Home backup", "2024-01-15 10:30:00")
    }

    /// Snapper where operations fail
    pub fn snapper_failing() -> SnapshotMockBuilder {
        SnapshotMockBuilder::snapper()
            .with_snapshot("1", "Existing snapshot", "2024-01-15 10:30:00")
            .create_succeeds(false)
            .delete_succeeds(false)
            .restore_succeeds(false)
    }

    /// No snapshot backend available
    pub fn no_backend() -> SnapshotMockBuilder {
        SnapshotMockBuilder::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resilience::CommandExecutor;

    // ==========================================================================
    // SnapshotFixture Tests
    // ==========================================================================

    #[test]
    fn test_snapshot_fixture_timeshift_creation() {
        let fixture = SnapshotFixture::timeshift("1", "Test backup", "2024-01-15_10-30-00");

        assert_eq!(fixture.id, "1");
        assert_eq!(fixture.description, "Test backup");
        assert_eq!(fixture.date_str, "2024-01-15_10-30-00");
        assert_eq!(fixture.snapshot_type, SnapshotType::Single);
    }

    #[test]
    fn test_snapshot_fixture_snapper_creation() {
        let fixture = SnapshotFixture::snapper("5", "Snapper backup", "2024-01-15 10:30:00");

        assert_eq!(fixture.id, "5");
        assert_eq!(fixture.description, "Snapper backup");
        assert_eq!(fixture.snapshot_type, SnapshotType::Single);
    }

    #[test]
    fn test_snapshot_fixture_with_type() {
        let fixture = SnapshotFixture::timeshift("1", "Pre-update", "2024-01-15_10-30-00")
            .with_type(SnapshotType::Pre);

        assert_eq!(fixture.snapshot_type, SnapshotType::Pre);
    }

    #[test]
    fn test_snapshot_fixture_to_timeshift_list_line() {
        let fixture = SnapshotFixture::timeshift("1", "Test backup", "2024-01-15_10-30-00");
        let line = fixture.to_timeshift_list_line();

        assert!(line.contains("1"));
        assert!(line.contains("2024-01-15_10-30-00"));
        assert!(line.contains("Test backup"));
    }

    #[test]
    fn test_snapshot_fixture_to_snapper_list_line() {
        let fixture = SnapshotFixture::snapper("5", "Snapper backup", "2024-01-15 10:30:00");
        let line = fixture.to_snapper_list_line();

        assert!(line.contains("5"));
        assert!(line.contains("2024-01-15 10:30:00"));
        assert!(line.contains("Snapper backup"));
    }

    // ==========================================================================
    // SnapshotMockBuilder Timeshift Tests
    // ==========================================================================

    #[test]
    fn test_timeshift_builder_creates_executor() {
        let executor = SnapshotMockBuilder::timeshift().build();
        assert_eq!(executor.total_call_count(), 0);
    }

    #[test]
    fn test_timeshift_list_empty() {
        let executor = SnapshotMockBuilder::timeshift().build();

        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");

        assert!(output.contains("Num"));
        assert!(output.contains("Name"));
    }

    #[test]
    fn test_timeshift_list_with_snapshots() {
        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "First backup", "2024-01-15_10-30-00")
            .with_snapshot("2", "Second backup", "2024-01-16_10-30-00")
            .build();

        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");

        assert!(output.contains("First backup"));
        assert!(output.contains("Second backup"));
        assert!(output.contains("2024-01-15_10-30-00"));
        assert!(output.contains("2024-01-16_10-30-00"));
    }

    #[test]
    fn test_timeshift_create_success() {
        let executor = SnapshotMockBuilder::timeshift().build();

        // Fallback handles any --create command
        let result = executor.execute("timeshift", &["--create", "--comments", "Test"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timeshift_create_failure() {
        let executor = SnapshotMockBuilder::timeshift()
            .create_succeeds(false)
            .build();

        let result = executor.execute("timeshift", &["--create", "--comments", "Test"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_timeshift_delete_success() {
        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "To delete", "2024-01-15_10-30-00")
            .build();

        let result = executor.execute("timeshift", &["--delete", "--snapshot", "1"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timeshift_delete_failure() {
        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "Protected", "2024-01-15_10-30-00")
            .delete_succeeds(false)
            .build();

        let result = executor.execute("timeshift", &["--delete", "--snapshot", "1"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_timeshift_restore_success() {
        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "To restore", "2024-01-15_10-30-00")
            .build();

        let result = executor.execute("timeshift", &["--restore", "--snapshot", "1", "--skip-grub"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timeshift_restore_failure() {
        let executor = SnapshotMockBuilder::timeshift()
            .with_snapshot("1", "Corrupted", "2024-01-15_10-30-00")
            .restore_succeeds(false)
            .build();

        let result = executor.execute("timeshift", &["--restore", "--snapshot", "1", "--skip-grub"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_timeshift_help() {
        let executor = SnapshotMockBuilder::timeshift().build();

        let output = executor
            .execute("timeshift", &["--help"])
            .expect("should execute");

        assert!(output.contains("Timeshift"));
    }

    // ==========================================================================
    // SnapshotMockBuilder Snapper Tests
    // ==========================================================================

    #[test]
    fn test_snapper_builder_creates_executor() {
        let executor = SnapshotMockBuilder::snapper().build();
        assert_eq!(executor.total_call_count(), 0);
    }

    #[test]
    fn test_snapper_list_empty() {
        let executor = SnapshotMockBuilder::snapper().build();

        let output = executor
            .execute("snapper", &["-c", "root", "list", "--columns", "number,date,description"])
            .expect("should execute");

        // Should contain header and snapshot 0 (current)
        assert!(output.contains("Date"));
        assert!(output.contains("Description"));
        assert!(output.contains("current"));
    }

    #[test]
    fn test_snapper_list_with_snapshots() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("1", "First backup", "2024-01-15 10:30:00")
            .with_snapshot("2", "Second backup", "2024-01-16 10:30:00")
            .build();

        let output = executor
            .execute("snapper", &["-c", "root", "list", "--columns", "number,date,description"])
            .expect("should execute");

        assert!(output.contains("First backup"));
        assert!(output.contains("Second backup"));
    }

    #[test]
    fn test_snapper_create_success() {
        let executor = SnapshotMockBuilder::snapper()
            .with_next_id(5)
            .build();

        let result = executor.execute("snapper", &["-c", "root", "create", "-d", "Test", "--print-number"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "5");
    }

    #[test]
    fn test_snapper_create_failure() {
        let executor = SnapshotMockBuilder::snapper()
            .create_succeeds(false)
            .build();

        let result = executor.execute("snapper", &["-c", "root", "create", "-d", "Test", "--print-number"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapper_delete_success() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("1", "To delete", "2024-01-15 10:30:00")
            .build();

        let result = executor.execute("snapper", &["-c", "root", "delete", "1"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_snapper_delete_failure() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("1", "Protected", "2024-01-15 10:30:00")
            .delete_succeeds(false)
            .build();

        let result = executor.execute("snapper", &["-c", "root", "delete", "1"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapper_restore_success() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("5", "To restore", "2024-01-15 10:30:00")
            .build();

        let result = executor.execute("snapper", &["-c", "root", "undochange", "5..0"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_snapper_restore_failure() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapshot("5", "Corrupted", "2024-01-15 10:30:00")
            .restore_succeeds(false)
            .build();

        let result = executor.execute("snapper", &["-c", "root", "undochange", "5..0"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapper_with_custom_config() {
        let executor = SnapshotMockBuilder::snapper()
            .with_snapper_config("home")
            .with_snapshot("1", "Home backup", "2024-01-15 10:30:00")
            .build();

        let output = executor
            .execute("snapper", &["-c", "home", "list", "--columns", "number,date,description"])
            .expect("should execute");

        assert!(output.contains("Home backup"));
    }

    // ==========================================================================
    // Backend Detection Tests
    // ==========================================================================

    #[test]
    fn test_which_timeshift_available() {
        let executor = SnapshotMockBuilder::timeshift().build();

        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("/usr/bin/timeshift"));

        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_which_snapper_available() {
        let executor = SnapshotMockBuilder::snapper().build();

        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("/usr/bin/snapper"));

        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_which_no_backend() {
        let executor = SnapshotMockBuilder::none().build();

        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_err());

        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_err());
    }

    // ==========================================================================
    // Pre-built Fixture Tests
    // ==========================================================================

    #[test]
    fn test_fixtures_timeshift_empty() {
        let executor = fixtures::timeshift_empty().build();
        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");
        assert!(output.contains("Num"));
    }

    #[test]
    fn test_fixtures_timeshift_with_snapshots() {
        let executor = fixtures::timeshift_with_snapshots().build();
        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");
        assert!(output.contains("Pre-update backup"));
        assert!(output.contains("Post-update"));
        assert!(output.contains("Manual backup"));
    }

    #[test]
    fn test_fixtures_timeshift_failing() {
        let executor = fixtures::timeshift_failing().build();

        let result = executor.execute("timeshift", &["--create", "--comments", "Test"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fixtures_snapper_empty() {
        let executor = fixtures::snapper_empty().build();
        let output = executor
            .execute("snapper", &["-c", "root", "list", "--columns", "number,date,description"])
            .expect("should execute");
        assert!(output.contains("current"));
    }

    #[test]
    fn test_fixtures_snapper_with_snapshots() {
        let executor = fixtures::snapper_with_snapshots().build();
        let output = executor
            .execute("snapper", &["-c", "root", "list", "--columns", "number,date,description"])
            .expect("should execute");
        assert!(output.contains("First snapshot"));
        assert!(output.contains("Second snapshot"));
    }

    #[test]
    fn test_fixtures_snapper_failing() {
        let executor = fixtures::snapper_failing().build();

        let result = executor.execute("snapper", &["-c", "root", "create", "-d", "Test", "--print-number"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fixtures_no_backend() {
        let executor = fixtures::no_backend().build();

        let result = executor.execute("which", &["timeshift"]);
        assert!(result.is_err());

        let result = executor.execute("which", &["snapper"]);
        assert!(result.is_err());
    }

    // ==========================================================================
    // Custom Output Tests
    // ==========================================================================

    #[test]
    fn test_custom_list_output() {
        let custom = "Custom timeshift output\nLine 2\n";
        let executor = SnapshotMockBuilder::timeshift()
            .with_custom_list_output(custom)
            .build();

        let output = executor
            .execute("timeshift", &["--list"])
            .expect("should execute");

        assert_eq!(output, custom);
    }

    #[test]
    fn test_custom_create_output() {
        let custom = "Tagged snapshot 'custom-id': Test snapshot\n";
        let executor = SnapshotMockBuilder::timeshift()
            .with_create_output(custom)
            .build();

        let output = executor
            .execute("timeshift", &["--create", "--comments", "Test"])
            .expect("should execute");

        assert!(output.contains("custom-id"));
    }
}
