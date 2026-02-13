//! Test fixtures for mocking systemctl commands.
//!
//! This module provides pre-configured mock responses for systemctl commands,
//! enabling comprehensive testing of `DefaultServiceManager` without requiring
//! actual systemd operations or root privileges.
//!
//! # Usage
//!
//! ```rust,ignore
//! use iron_core::resilience::MockCommandExecutor;
//! use iron_systemd::test_fixtures::SystemdMockBuilder;
//!
//! let executor = SystemdMockBuilder::new()
//!     .with_service("ssh", ServiceState::Active, EnabledState::Enabled)
//!     .with_service("docker", ServiceState::Inactive, EnabledState::Disabled)
//!     .build();
//!
//! // Use with DefaultServiceManager::with_executor()
//! ```

use super::{EnabledState, ServiceState};
use iron_core::resilience::{MockCommandExecutor, MockResponse};
use std::collections::HashMap;

/// Service fixture data
#[derive(Debug, Clone)]
pub struct ServiceFixture {
    /// Service name (without .service suffix)
    pub name: String,
    /// Current running state
    pub state: ServiceState,
    /// Boot-time enabled state
    pub enabled: EnabledState,
    /// Service description
    pub description: String,
    /// Whether service exists
    pub exists: bool,
}

impl ServiceFixture {
    /// Create a new service fixture
    pub fn new(name: &str, state: ServiceState, enabled: EnabledState) -> Self {
        Self {
            name: name.to_string(),
            state,
            enabled,
            description: format!("{} service", name),
            exists: true,
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Mark as non-existent (for testing not found scenarios)
    pub fn not_exists(mut self) -> Self {
        self.exists = false;
        self
    }

    /// Generate systemctl status output
    pub fn to_status_output(&self) -> String {
        let state_line = match self.state {
            ServiceState::Active => "     Active: active (running) since Mon 2024-01-01 00:00:00 UTC",
            ServiceState::Inactive => "     Active: inactive (dead)",
            ServiceState::Failed => "     Active: failed (Result: exit-code)",
            ServiceState::Unknown => "     Active: unknown",
        };

        format!(
            "● {}.service - {}\n\
             {}\n\
             Description: {}\n\
             Loaded: loaded (/usr/lib/systemd/system/{}.service; {})\n\
             Main PID: 1234 ({})\n",
            self.name,
            self.description,
            state_line,
            self.description,
            self.name,
            self.enabled_str(),
            self.name
        )
    }

    /// Generate systemctl is-enabled output
    pub fn to_enabled_output(&self) -> String {
        self.enabled_str().to_string()
    }

    /// Generate list-units line
    pub fn to_list_units_line(&self) -> String {
        let state_str = match self.state {
            ServiceState::Active => "active",
            ServiceState::Inactive => "inactive",
            ServiceState::Failed => "failed",
            ServiceState::Unknown => "unknown",
        };
        let sub_state = match self.state {
            ServiceState::Active => "running",
            ServiceState::Inactive => "dead",
            ServiceState::Failed => "failed",
            ServiceState::Unknown => "unknown",
        };

        format!(
            "{}.service    loaded {} {} {}",
            self.name, state_str, sub_state, self.description
        )
    }

    fn enabled_str(&self) -> &'static str {
        match self.enabled {
            EnabledState::Enabled => "enabled",
            EnabledState::Disabled => "disabled",
            EnabledState::Masked => "masked",
            EnabledState::Static => "static",
            EnabledState::Unknown => "unknown",
        }
    }
}

/// Builder for creating configured `MockCommandExecutor` with systemd-specific responses.
#[derive(Debug, Default)]
pub struct SystemdMockBuilder {
    /// Registered services
    services: HashMap<String, ServiceFixture>,
    /// Whether enable operations succeed
    enable_succeeds: bool,
    /// Whether disable operations succeed
    disable_succeeds: bool,
    /// Whether start operations succeed
    start_succeeds: bool,
    /// Whether stop operations succeed
    stop_succeeds: bool,
    /// Whether restart operations succeed
    restart_succeeds: bool,
    /// Whether to configure for user scope (--user flag)
    user_scope: bool,
}

impl SystemdMockBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            enable_succeeds: true,
            disable_succeeds: true,
            start_succeeds: true,
            stop_succeeds: true,
            restart_succeeds: true,
            ..Default::default()
        }
    }

    /// Add a service with the given state
    pub fn with_service(mut self, name: &str, state: ServiceState, enabled: EnabledState) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceFixture::new(name, state, enabled),
        );
        self
    }

    /// Add a fully configured service fixture
    pub fn with_service_fixture(mut self, fixture: ServiceFixture) -> Self {
        self.services.insert(fixture.name.clone(), fixture);
        self
    }

    /// Add a non-existent service (for testing "not found" scenarios)
    pub fn with_missing_service(mut self, name: &str) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceFixture::new(name, ServiceState::Unknown, EnabledState::Unknown).not_exists(),
        );
        self
    }

    /// Set whether enable operations succeed
    pub fn enable_succeeds(mut self, succeeds: bool) -> Self {
        self.enable_succeeds = succeeds;
        self
    }

    /// Set whether disable operations succeed
    pub fn disable_succeeds(mut self, succeeds: bool) -> Self {
        self.disable_succeeds = succeeds;
        self
    }

    /// Set whether start operations succeed
    pub fn start_succeeds(mut self, succeeds: bool) -> Self {
        self.start_succeeds = succeeds;
        self
    }

    /// Set whether stop operations succeed
    pub fn stop_succeeds(mut self, succeeds: bool) -> Self {
        self.stop_succeeds = succeeds;
        self
    }

    /// Set whether restart operations succeed
    pub fn restart_succeeds(mut self, succeeds: bool) -> Self {
        self.restart_succeeds = succeeds;
        self
    }

    /// Configure for user scope (--user flag on all commands)
    pub fn user_scope(mut self) -> Self {
        self.user_scope = true;
        self
    }

    /// Build the configured `MockCommandExecutor`
    pub fn build(self) -> MockCommandExecutor {
        let executor = MockCommandExecutor::new();

        // Build argument prefix for user scope
        let user_prefix: &[&str] = if self.user_scope { &["--user"] } else { &[] };

        // Configure responses for each service
        for (name, fixture) in &self.services {
            let service_name = format!("{}.service", name);

            if fixture.exists {
                // systemctl status <service>
                let status_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["status", name].iter())
                    .copied()
                    .collect();
                executor.add_response(
                    "systemctl",
                    &status_args,
                    MockResponse::success(&fixture.to_status_output()),
                );
                // Also with .service suffix
                let status_args_svc: Vec<&str> = user_prefix
                    .iter()
                    .chain(["status", &service_name].iter())
                    .copied()
                    .collect();
                executor.add_response(
                    "systemctl",
                    &status_args_svc,
                    MockResponse::success(&fixture.to_status_output()),
                );

                // systemctl is-enabled <service>
                let enabled_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["is-enabled", name].iter())
                    .copied()
                    .collect();
                executor.add_response(
                    "systemctl",
                    &enabled_args,
                    MockResponse::success(&fixture.to_enabled_output()),
                );

                // systemctl cat <service> (for exists check)
                let cat_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["cat", name].iter())
                    .copied()
                    .collect();
                executor.add_response(
                    "systemctl",
                    &cat_args,
                    MockResponse::success(&format!("[Unit]\nDescription={}", fixture.description)),
                );

                // systemctl enable <service>
                let enable_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["enable", name].iter())
                    .copied()
                    .collect();
                if self.enable_succeeds {
                    executor.add_response(
                        "systemctl",
                        &enable_args,
                        MockResponse::success(&format!("Created symlink for {}.service", name)),
                    );
                } else {
                    executor.add_response(
                        "systemctl",
                        &enable_args,
                        MockResponse::exit_error(1, "Failed to enable unit"),
                    );
                }

                // systemctl disable <service>
                let disable_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["disable", name].iter())
                    .copied()
                    .collect();
                if self.disable_succeeds {
                    executor.add_response(
                        "systemctl",
                        &disable_args,
                        MockResponse::success(&format!("Removed symlink for {}.service", name)),
                    );
                } else {
                    executor.add_response(
                        "systemctl",
                        &disable_args,
                        MockResponse::exit_error(1, "Failed to disable unit"),
                    );
                }

                // systemctl start <service>
                let start_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["start", name].iter())
                    .copied()
                    .collect();
                if self.start_succeeds {
                    executor.add_response(
                        "systemctl",
                        &start_args,
                        MockResponse::success(""),
                    );
                } else {
                    executor.add_response(
                        "systemctl",
                        &start_args,
                        MockResponse::exit_error(1, &format!("Failed to start {}.service", name)),
                    );
                }

                // systemctl stop <service>
                let stop_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["stop", name].iter())
                    .copied()
                    .collect();
                if self.stop_succeeds {
                    executor.add_response(
                        "systemctl",
                        &stop_args,
                        MockResponse::success(""),
                    );
                } else {
                    executor.add_response(
                        "systemctl",
                        &stop_args,
                        MockResponse::exit_error(1, &format!("Failed to stop {}.service", name)),
                    );
                }

                // systemctl restart <service>
                let restart_args: Vec<&str> = user_prefix
                    .iter()
                    .chain(["restart", name].iter())
                    .copied()
                    .collect();
                if self.restart_succeeds {
                    executor.add_response(
                        "systemctl",
                        &restart_args,
                        MockResponse::success(""),
                    );
                } else {
                    executor.add_response(
                        "systemctl",
                        &restart_args,
                        MockResponse::exit_error(1, &format!("Failed to restart {}.service", name)),
                    );
                }
            } else {
                // Service doesn't exist - return errors
                let not_found_msg = format!("Unit {}.service not found.", name);

                for cmd in &["status", "is-enabled", "enable", "disable", "start", "stop", "restart", "cat"] {
                    let args: Vec<&str> = user_prefix
                        .iter()
                        .chain([*cmd, name].iter())
                        .copied()
                        .collect();
                    executor.add_response(
                        "systemctl",
                        &args,
                        MockResponse::exit_error(4, &not_found_msg),
                    );
                }
            }
        }

        // Configure list-units
        let list_output: String = self
            .services
            .values()
            .filter(|s| s.exists)
            .map(|s| s.to_list_units_line())
            .collect::<Vec<_>>()
            .join("\n");

        let list_args: Vec<&str> = user_prefix
            .iter()
            .chain(["list-units", "--type=service", "--all", "--no-legend"].iter())
            .copied()
            .collect();
        executor.add_response(
            "systemctl",
            &list_args,
            MockResponse::success(&list_output),
        );

        // Add systemctl to existing commands
        executor.add_existing_command("systemctl");

        executor
    }
}

// =============================================================================
// Pre-built Fixture Sets
// =============================================================================

/// Common systemd service scenarios for testing
pub mod fixtures {
    use super::*;

    /// Common desktop services
    pub fn desktop_services() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("pipewire", ServiceState::Active, EnabledState::Enabled)
            .with_service("wireplumber", ServiceState::Active, EnabledState::Enabled)
            .with_service("xdg-desktop-portal", ServiceState::Active, EnabledState::Enabled)
            .with_service("xdg-desktop-portal-hyprland", ServiceState::Active, EnabledState::Enabled)
            .user_scope()
    }

    /// Common system services
    pub fn system_services() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .with_service("NetworkManager", ServiceState::Active, EnabledState::Enabled)
            .with_service("docker", ServiceState::Inactive, EnabledState::Disabled)
            .with_service("cups", ServiceState::Inactive, EnabledState::Disabled)
    }

    /// Services with various states
    pub fn mixed_states() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("active-enabled", ServiceState::Active, EnabledState::Enabled)
            .with_service("active-disabled", ServiceState::Active, EnabledState::Disabled)
            .with_service("inactive-enabled", ServiceState::Inactive, EnabledState::Enabled)
            .with_service("inactive-disabled", ServiceState::Inactive, EnabledState::Disabled)
            .with_service("failed-service", ServiceState::Failed, EnabledState::Enabled)
            .with_service("masked-service", ServiceState::Inactive, EnabledState::Masked)
            .with_service("static-service", ServiceState::Active, EnabledState::Static)
    }

    /// Failed services
    pub fn failed_services() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("crashed-app", ServiceState::Failed, EnabledState::Enabled)
            .with_service("broken-daemon", ServiceState::Failed, EnabledState::Disabled)
    }

    /// Services where operations fail
    pub fn failing_operations() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("protected", ServiceState::Active, EnabledState::Enabled)
            .enable_succeeds(false)
            .disable_succeeds(false)
            .start_succeeds(false)
            .stop_succeeds(false)
            .restart_succeeds(false)
    }

    /// With a missing/non-existent service
    pub fn with_missing() -> SystemdMockBuilder {
        SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .with_missing_service("nonexistent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::resilience::CommandExecutor;

    #[test]
    fn test_builder_creates_executor() {
        let executor = SystemdMockBuilder::new()
            .with_service("ssh", ServiceState::Active, EnabledState::Enabled)
            .build();

        assert_eq!(executor.total_call_count(), 0);
    }

    #[test]
    fn test_service_fixture_status_output() {
        let fixture = ServiceFixture::new("ssh", ServiceState::Active, EnabledState::Enabled)
            .with_description("OpenSSH server daemon");

        let output = fixture.to_status_output();
        assert!(output.contains("ssh.service"));
        assert!(output.contains("active (running)"));
        assert!(output.contains("OpenSSH server daemon"));
    }

    #[test]
    fn test_service_fixture_inactive() {
        let fixture = ServiceFixture::new("docker", ServiceState::Inactive, EnabledState::Disabled);
        let output = fixture.to_status_output();
        assert!(output.contains("inactive (dead)"));
    }

    #[test]
    fn test_service_fixture_failed() {
        let fixture = ServiceFixture::new("broken", ServiceState::Failed, EnabledState::Enabled);
        let output = fixture.to_status_output();
        assert!(output.contains("failed (Result: exit-code)"));
    }

    #[test]
    fn test_service_fixture_enabled_output() {
        let fixture = ServiceFixture::new("ssh", ServiceState::Active, EnabledState::Enabled);
        assert_eq!(fixture.to_enabled_output(), "enabled");

        let fixture = ServiceFixture::new("docker", ServiceState::Inactive, EnabledState::Disabled);
        assert_eq!(fixture.to_enabled_output(), "disabled");

        let fixture = ServiceFixture::new("masked", ServiceState::Inactive, EnabledState::Masked);
        assert_eq!(fixture.to_enabled_output(), "masked");
    }

    #[test]
    fn test_service_fixture_list_units_line() {
        let fixture = ServiceFixture::new("ssh", ServiceState::Active, EnabledState::Enabled);
        let line = fixture.to_list_units_line();
        assert!(line.contains("ssh.service"));
        assert!(line.contains("active"));
        assert!(line.contains("running"));
    }

    #[test]
    fn test_status_command() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();

        let output = executor
            .execute("systemctl", &["status", "sshd"])
            .expect("should execute");

        assert!(output.contains("sshd.service"));
        assert!(output.contains("active (running)"));
    }

    #[test]
    fn test_is_enabled_command() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();

        let output = executor
            .execute("systemctl", &["is-enabled", "sshd"])
            .expect("should execute");

        assert_eq!(output, "enabled");
    }

    #[test]
    fn test_enable_command_success() {
        let executor = SystemdMockBuilder::new()
            .with_service("docker", ServiceState::Inactive, EnabledState::Disabled)
            .build();

        let result = executor.execute("systemctl", &["enable", "docker"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enable_command_failure() {
        let executor = SystemdMockBuilder::new()
            .with_service("protected", ServiceState::Active, EnabledState::Enabled)
            .enable_succeeds(false)
            .build();

        let result = executor.execute("systemctl", &["enable", "protected"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_start_command() {
        let executor = SystemdMockBuilder::new()
            .with_service("docker", ServiceState::Inactive, EnabledState::Disabled)
            .build();

        let result = executor.execute("systemctl", &["start", "docker"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_command() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();

        let result = executor.execute("systemctl", &["stop", "sshd"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_restart_command() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();

        let result = executor.execute("systemctl", &["restart", "sshd"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cat_command_exists() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();

        let result = executor.execute("systemctl", &["cat", "sshd"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_service() {
        let executor = SystemdMockBuilder::new()
            .with_missing_service("nonexistent")
            .build();

        let result = executor.execute("systemctl", &["status", "nonexistent"]);
        assert!(result.is_err());

        let result = executor.execute("systemctl", &["cat", "nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_units() {
        let executor = SystemdMockBuilder::new()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .with_service("docker", ServiceState::Inactive, EnabledState::Disabled)
            .build();

        let output = executor
            .execute(
                "systemctl",
                &["list-units", "--type=service", "--all", "--no-legend"],
            )
            .expect("should execute");

        assert!(output.contains("sshd.service"));
        assert!(output.contains("docker.service"));
    }

    #[test]
    fn test_user_scope() {
        let executor = SystemdMockBuilder::new()
            .with_service("pipewire", ServiceState::Active, EnabledState::Enabled)
            .user_scope()
            .build();

        let output = executor
            .execute("systemctl", &["--user", "status", "pipewire"])
            .expect("should execute");

        assert!(output.contains("pipewire.service"));
    }

    // Fixture tests
    #[test]
    fn test_fixtures_desktop_services() {
        let executor = fixtures::desktop_services().build();

        let output = executor
            .execute("systemctl", &["--user", "status", "pipewire"])
            .expect("should execute");
        assert!(output.contains("active (running)"));
    }

    #[test]
    fn test_fixtures_system_services() {
        let executor = fixtures::system_services().build();

        let output = executor
            .execute("systemctl", &["status", "sshd"])
            .expect("should execute");
        assert!(output.contains("active (running)"));

        let output = executor
            .execute("systemctl", &["status", "docker"])
            .expect("should execute");
        assert!(output.contains("inactive (dead)"));
    }

    #[test]
    fn test_fixtures_mixed_states() {
        let executor = fixtures::mixed_states().build();

        let output = executor
            .execute("systemctl", &["status", "failed-service"])
            .expect("should execute");
        assert!(output.contains("failed"));

        let output = executor
            .execute("systemctl", &["is-enabled", "masked-service"])
            .expect("should execute");
        assert_eq!(output, "masked");
    }

    #[test]
    fn test_fixtures_failing_operations() {
        let executor = fixtures::failing_operations().build();

        let result = executor.execute("systemctl", &["enable", "protected"]);
        assert!(result.is_err());

        let result = executor.execute("systemctl", &["stop", "protected"]);
        assert!(result.is_err());
    }
}
