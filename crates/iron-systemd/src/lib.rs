//! Iron Systemd - Service management for Iron configuration management
//!
//! This crate provides systemd integration for Iron, including:
//! - Service enable/disable
//! - Service start/stop
//! - Service status queries
//! - User vs system service handling
//!
//! # Testing Support
//!
//! The `test_fixtures` module provides mock responses for systemctl commands,
//! enabling comprehensive testing without actual systemd operations:
//!
//! ```rust,ignore
//! use iron_systemd::test_fixtures::SystemdMockBuilder;
//! use iron_systemd::{ServiceState, EnabledState};
//! use std::sync::Arc;
//!
//! let executor = SystemdMockBuilder::new()
//!     .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
//!     .build();
//!
//! // Use with DefaultServiceManager::with_executor()
//! ```

pub mod test_fixtures;

use iron_core::resilience::{CommandExecutor, RealCommandExecutor};
use iron_core::{IronResult, ServiceError};
use std::process::Command;
use std::sync::Arc;

/// Service state from systemctl
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Service is running
    Active,

    /// Service is not running
    Inactive,

    /// Service failed to start
    Failed,

    /// Service state is unknown
    Unknown,
}

/// Whether a service is enabled at boot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnabledState {
    /// Service starts at boot
    Enabled,

    /// Service does not start at boot
    Disabled,

    /// Service is masked (cannot be started)
    Masked,

    /// Service is statically enabled
    Static,

    /// Unknown state
    Unknown,
}

/// Information about a systemd service
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,

    /// Current state
    pub state: ServiceState,

    /// Whether enabled at boot
    pub enabled: EnabledState,

    /// Service description
    pub description: Option<String>,
}

/// Scope for service operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScope {
    /// System-wide services (requires sudo)
    System,

    /// User services (no sudo needed)
    User,
}

/// Service manager trait for systemd operations
pub trait ServiceManager {
    /// Get the status of a service
    fn status(&self, name: &str) -> IronResult<ServiceInfo>;

    /// Enable a service
    fn enable(&self, name: &str) -> IronResult<()>;

    /// Disable a service
    fn disable(&self, name: &str) -> IronResult<()>;

    /// Start a service
    fn start(&self, name: &str) -> IronResult<()>;

    /// Stop a service
    fn stop(&self, name: &str) -> IronResult<()>;

    /// Restart a service
    fn restart(&self, name: &str) -> IronResult<()>;

    /// Check if a service exists
    fn exists(&self, name: &str) -> bool;

    /// List all services matching a pattern
    fn list(&self, pattern: Option<&str>) -> IronResult<Vec<ServiceInfo>>;
}

/// Default service manager using systemctl
pub struct DefaultServiceManager {
    scope: ServiceScope,
    /// Optional command executor for resilient command execution
    executor: Option<Arc<dyn CommandExecutor>>,
}

impl DefaultServiceManager {
    /// Create a new service manager with a default resilient executor.
    ///
    /// The circuit breaker opens after 3 consecutive failures and stays open
    /// for 60 seconds, preventing hangs from a broken systemd environment.
    pub fn new(scope: ServiceScope) -> Self {
        Self::with_resilience(scope)
    }

    /// Create a service manager for user services
    pub fn user() -> Self {
        Self::new(ServiceScope::User)
    }

    /// Create a service manager for system services
    pub fn system() -> Self {
        Self::new(ServiceScope::System)
    }

    /// Create a service manager with a command executor for resilient operations
    ///
    /// The executor provides circuit breaker patterns and timeout handling
    /// for systemctl commands. When the circuit opens due to repeated failures,
    /// commands will fail fast without attempting execution.
    pub fn with_executor(scope: ServiceScope, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            scope,
            executor: Some(executor),
        }
    }

    /// Create a service manager with default resilient executor
    ///
    /// Uses the default `RealCommandExecutor` with 120s timeout and circuit breaker.
    pub fn with_resilience(scope: ServiceScope) -> Self {
        Self::with_executor(scope, Arc::new(RealCommandExecutor::with_defaults()))
    }

    /// Run systemctl command using executor if available, otherwise direct execution
    fn run_systemctl(&self, args: &[&str]) -> IronResult<String> {
        let mut cmd_args = vec![];

        if self.scope == ServiceScope::User {
            cmd_args.push("--user");
        }

        cmd_args.extend(args);

        if let Some(ref executor) = self.executor {
            let args_refs: Vec<&str> = cmd_args.to_vec();
            let output = executor
                .execute_full("systemctl", &args_refs)
                .map_err(|_| ServiceError::SystemctlNotFound)?;

            if output.success() {
                Ok(output.stdout)
            } else {
                // Check for specific errors
                if output.stderr.contains("not found") || output.stderr.contains("No such file") {
                    return Err(ServiceError::NotFound {
                        name: args.last().unwrap_or(&"unknown").to_string(),
                    }
                    .into());
                }

                Err(ServiceError::EnableFailed {
                    name: args.last().unwrap_or(&"unknown").to_string(),
                    message: output.stderr,
                }
                .into())
            }
        } else {
            let output = Command::new("systemctl")
                .args(&cmd_args)
                .output()
                .map_err(|_| ServiceError::SystemctlNotFound)?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Check for specific errors
                if stderr.contains("not found") || stderr.contains("No such file") {
                    return Err(ServiceError::NotFound {
                        name: args.last().unwrap_or(&"unknown").to_string(),
                    }
                    .into());
                }

                Err(ServiceError::EnableFailed {
                    name: args.last().unwrap_or(&"unknown").to_string(),
                    message: stderr.to_string(),
                }
                .into())
            }
        }
    }

    /// Parse service state from systemctl output
    fn parse_state(output: &str) -> ServiceState {
        if output.contains("active (running)") {
            ServiceState::Active
        } else if output.contains("inactive") {
            ServiceState::Inactive
        } else if output.contains("failed") {
            ServiceState::Failed
        } else {
            ServiceState::Unknown
        }
    }

    /// Parse enabled state from systemctl output
    fn parse_enabled(output: &str) -> EnabledState {
        if output.contains("enabled") {
            EnabledState::Enabled
        } else if output.contains("disabled") {
            EnabledState::Disabled
        } else if output.contains("masked") {
            EnabledState::Masked
        } else if output.contains("static") {
            EnabledState::Static
        } else {
            EnabledState::Unknown
        }
    }
}

impl ServiceManager for DefaultServiceManager {
    fn status(&self, name: &str) -> IronResult<ServiceInfo> {
        let output = self.run_systemctl(&["status", name])?;

        let state = Self::parse_state(&output);
        let enabled_output = self
            .run_systemctl(&["is-enabled", name])
            .unwrap_or_default();
        let enabled = Self::parse_enabled(&enabled_output);

        // Extract description from status output
        let description = output
            .lines()
            .find(|l| l.contains("Description:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string());

        Ok(ServiceInfo {
            name: name.to_string(),
            state,
            enabled,
            description,
        })
    }

    fn enable(&self, name: &str) -> IronResult<()> {
        self.run_systemctl(&["enable", name])?;
        Ok(())
    }

    fn disable(&self, name: &str) -> IronResult<()> {
        self.run_systemctl(&["disable", name])?;
        Ok(())
    }

    fn start(&self, name: &str) -> IronResult<()> {
        self.run_systemctl(&["start", name])?;
        Ok(())
    }

    fn stop(&self, name: &str) -> IronResult<()> {
        self.run_systemctl(&["stop", name])?;
        Ok(())
    }

    fn restart(&self, name: &str) -> IronResult<()> {
        self.run_systemctl(&["restart", name])?;
        Ok(())
    }

    fn exists(&self, name: &str) -> bool {
        self.run_systemctl(&["cat", name]).is_ok()
    }

    fn list(&self, pattern: Option<&str>) -> IronResult<Vec<ServiceInfo>> {
        let mut args = vec!["list-units", "--type=service", "--all", "--no-legend"];
        if let Some(p) = pattern {
            args.push(p);
        }

        let output = self.run_systemctl(&args)?;
        let mut services = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].trim_end_matches(".service").to_string();
                let state = match parts[2] {
                    "active" => ServiceState::Active,
                    "inactive" => ServiceState::Inactive,
                    "failed" => ServiceState::Failed,
                    _ => ServiceState::Unknown,
                };

                services.push(ServiceInfo {
                    name,
                    state,
                    enabled: EnabledState::Unknown, // Would need separate query
                    description: None,
                });
            }
        }

        Ok(services)
    }
}

// ==========================================================================
// SystemService adapter — bridges iron-systemd → iron-core::SystemService
// ==========================================================================

/// Adapter that implements `iron_core::SystemService` by delegating to
/// `DefaultServiceManager`. This bridges the rich `ServiceManager` trait
/// (8 methods, status, restart, exists, list) down to the narrow 4-method
/// trait that `BundleService` and `ModuleService` require.
pub struct SystemdServiceAdapter {
    inner: DefaultServiceManager,
}

impl SystemdServiceAdapter {
    /// Create an adapter for the given scope with default resilient executor.
    pub fn new(scope: ServiceScope) -> Self {
        Self {
            inner: DefaultServiceManager::new(scope),
        }
    }

    /// Create an adapter for user services.
    pub fn user() -> Self {
        Self::new(ServiceScope::User)
    }

    /// Create an adapter for system services.
    pub fn system() -> Self {
        Self::new(ServiceScope::System)
    }

    /// Create an adapter with a custom command executor.
    pub fn with_executor(scope: ServiceScope, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            inner: DefaultServiceManager::with_executor(scope, executor),
        }
    }
}

impl iron_core::SystemService for SystemdServiceAdapter {
    fn enable_service(&self, name: &str) -> IronResult<()> {
        self.inner.enable(name)
    }

    fn disable_service(&self, name: &str) -> IronResult<()> {
        self.inner.disable(name)
    }

    fn start_service(&self, name: &str) -> IronResult<()> {
        self.inner.start(name)
    }

    fn stop_service(&self, name: &str) -> IronResult<()> {
        self.inner.stop(name)
    }
}

/// Helper to enable multiple services at once
pub fn enable_services(services: &[&str], scope: ServiceScope) -> IronResult<Vec<String>> {
    let manager = DefaultServiceManager::new(scope);
    let mut failed = Vec::new();

    for service in services {
        if manager.enable(service).is_err() {
            failed.push(service.to_string());
        }
    }

    Ok(failed)
}

/// Helper to disable multiple services at once
pub fn disable_services(services: &[&str], scope: ServiceScope) -> IronResult<Vec<String>> {
    let manager = DefaultServiceManager::new(scope);
    let mut failed = Vec::new();

    for service in services {
        if manager.disable(service).is_err() {
            failed.push(service.to_string());
        }
    }

    Ok(failed)
}

/// Parse service state from systemctl status output
///
/// Parses the `systemctl status` output to determine the current service state.
///
/// # Examples
///
/// ```
/// use iron_systemd::{parse_service_state, ServiceState};
///
/// // Active service
/// let output = "● ssh.service - OpenSSH Daemon\n   Active: active (running)";
/// assert_eq!(parse_service_state(output), ServiceState::Active);
///
/// // Inactive service
/// let output = "● test.service\n   Active: inactive (dead)";
/// assert_eq!(parse_service_state(output), ServiceState::Inactive);
///
/// // Failed service
/// let output = "● broken.service\n   Active: failed (Result: exit-code)";
/// assert_eq!(parse_service_state(output), ServiceState::Failed);
/// ```
pub fn parse_service_state(output: &str) -> ServiceState {
    DefaultServiceManager::parse_state(output)
}

/// Parse enabled state from systemctl is-enabled output
///
/// Parses the output of `systemctl is-enabled` to determine boot behavior.
///
/// # Examples
///
/// ```
/// use iron_systemd::{parse_enabled_state, EnabledState};
///
/// assert_eq!(parse_enabled_state("enabled"), EnabledState::Enabled);
/// assert_eq!(parse_enabled_state("disabled"), EnabledState::Disabled);
/// assert_eq!(parse_enabled_state("masked"), EnabledState::Masked);
/// assert_eq!(parse_enabled_state("static"), EnabledState::Static);
/// assert_eq!(parse_enabled_state("unknown-value"), EnabledState::Unknown);
/// ```
pub fn parse_enabled_state(output: &str) -> EnabledState {
    DefaultServiceManager::parse_enabled(output)
}

/// Parse description from systemctl status output
///
/// Extracts the service description from `systemctl status` output.
///
/// # Examples
///
/// ```
/// use iron_systemd::parse_description;
///
/// let output = "● ssh.service\n   Description: OpenSSH server daemon\n   Active: active";
/// assert_eq!(parse_description(output), Some("OpenSSH server daemon".to_string()));
///
/// // No description line
/// let output = "● test.service\n   Active: active (running)";
/// assert_eq!(parse_description(output), None);
/// ```
pub fn parse_description(output: &str) -> Option<String> {
    output
        .lines()
        .find(|l| l.trim().starts_with("Description:"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, desc)| desc.trim().to_string())
}

/// Parse list-units output into service info entries
///
/// Parses the output of `systemctl list-units --type=service` into structured data.
///
/// # Examples
///
/// ```
/// use iron_systemd::{parse_list_units, ServiceState};
///
/// let output = "ssh.service    loaded active running OpenSSH Daemon";
/// let services = parse_list_units(output);
/// assert_eq!(services.len(), 1);
/// assert_eq!(services[0].name, "ssh");
/// assert_eq!(services[0].state, ServiceState::Active);
///
/// // Multiple services
/// let output = "docker.service loaded active running Docker\ncups.service   loaded inactive dead CUPS";
/// let services = parse_list_units(output);
/// assert_eq!(services.len(), 2);
/// assert_eq!(services[0].state, ServiceState::Active);
/// assert_eq!(services[1].state, ServiceState::Inactive);
/// ```
pub fn parse_list_units(output: &str) -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let name = parts[0].trim_end_matches(".service").to_string();
            let state = match parts[2] {
                "active" => ServiceState::Active,
                "inactive" => ServiceState::Inactive,
                "failed" => ServiceState::Failed,
                _ => ServiceState::Unknown,
            };

            // Description is everything after the 4th column
            let description = if parts.len() > 4 {
                Some(parts[4..].join(" "))
            } else {
                None
            };

            services.push(ServiceInfo {
                name,
                state,
                enabled: EnabledState::Unknown,
                description,
            });
        }
    }

    services
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Enum tests
    // ==========================================================================

    #[test]
    fn test_service_state_equality() {
        assert_eq!(ServiceState::Active, ServiceState::Active);
        assert_ne!(ServiceState::Active, ServiceState::Inactive);
        assert_ne!(ServiceState::Failed, ServiceState::Unknown);
    }

    #[test]
    fn test_enabled_state_equality() {
        assert_eq!(EnabledState::Enabled, EnabledState::Enabled);
        assert_ne!(EnabledState::Enabled, EnabledState::Disabled);
        assert_ne!(EnabledState::Masked, EnabledState::Static);
    }

    #[test]
    fn test_service_scope_equality() {
        assert_eq!(ServiceScope::User, ServiceScope::User);
        assert_ne!(ServiceScope::User, ServiceScope::System);
    }

    // ==========================================================================
    // ServiceInfo tests
    // ==========================================================================

    #[test]
    fn test_service_info_creation() {
        let info = ServiceInfo {
            name: "ssh".to_string(),
            state: ServiceState::Active,
            enabled: EnabledState::Enabled,
            description: Some("OpenSSH Daemon".to_string()),
        };
        assert_eq!(info.name, "ssh");
        assert_eq!(info.state, ServiceState::Active);
        assert_eq!(info.enabled, EnabledState::Enabled);
        assert_eq!(info.description, Some("OpenSSH Daemon".to_string()));
    }

    #[test]
    fn test_service_info_without_description() {
        let info = ServiceInfo {
            name: "test".to_string(),
            state: ServiceState::Inactive,
            enabled: EnabledState::Disabled,
            description: None,
        };
        assert!(info.description.is_none());
    }

    // ==========================================================================
    // Service state parsing tests
    // ==========================================================================

    #[test]
    fn test_parse_state_active_running() {
        let output =
            "● ssh.service - OpenSSH Daemon\n   Loaded: loaded\n   Active: active (running)";
        assert_eq!(parse_service_state(output), ServiceState::Active);
    }

    #[test]
    fn test_parse_state_inactive_dead() {
        let output = "● test.service\n   Active: inactive (dead)";
        assert_eq!(parse_service_state(output), ServiceState::Inactive);
    }

    #[test]
    fn test_parse_state_failed() {
        let output = "● broken.service\n   Active: failed (Result: exit-code)";
        assert_eq!(parse_service_state(output), ServiceState::Failed);
    }

    #[test]
    fn test_parse_state_unknown() {
        let output = "some random output";
        assert_eq!(parse_service_state(output), ServiceState::Unknown);
    }

    #[test]
    fn test_parse_state_empty() {
        assert_eq!(parse_service_state(""), ServiceState::Unknown);
    }

    #[test]
    fn test_parse_state_activating() {
        // Edge case: service is starting
        let output = "Active: activating (start)";
        assert_eq!(parse_service_state(output), ServiceState::Unknown);
    }

    // ==========================================================================
    // Enabled state parsing tests
    // ==========================================================================

    #[test]
    fn test_parse_enabled_enabled() {
        assert_eq!(parse_enabled_state("enabled"), EnabledState::Enabled);
    }

    #[test]
    fn test_parse_enabled_disabled() {
        assert_eq!(parse_enabled_state("disabled"), EnabledState::Disabled);
    }

    #[test]
    fn test_parse_enabled_masked() {
        assert_eq!(parse_enabled_state("masked"), EnabledState::Masked);
    }

    #[test]
    fn test_parse_enabled_static() {
        assert_eq!(parse_enabled_state("static"), EnabledState::Static);
    }

    #[test]
    fn test_parse_enabled_unknown() {
        assert_eq!(parse_enabled_state("something else"), EnabledState::Unknown);
    }

    #[test]
    fn test_parse_enabled_empty() {
        assert_eq!(parse_enabled_state(""), EnabledState::Unknown);
    }

    #[test]
    fn test_parse_enabled_with_extra_text() {
        // is-enabled can output additional info
        assert_eq!(parse_enabled_state("enabled\n"), EnabledState::Enabled);
        assert_eq!(parse_enabled_state("disabled\n"), EnabledState::Disabled);
    }

    // ==========================================================================
    // Description parsing tests
    // ==========================================================================

    #[test]
    fn test_parse_description_present() {
        let output = r#"● ssh.service - OpenSSH Daemon
     Loaded: loaded (/usr/lib/systemd/system/sshd.service; enabled; preset: disabled)
     Active: active (running) since Mon 2024-01-01 10:00:00 UTC
   Description: OpenSSH server daemon"#;
        assert_eq!(
            parse_description(output),
            Some("OpenSSH server daemon".to_string())
        );
    }

    #[test]
    fn test_parse_description_none() {
        let output = "● test.service\n   Active: active (running)";
        assert_eq!(parse_description(output), None);
    }

    #[test]
    fn test_parse_description_empty_value() {
        let output = "Description:";
        assert_eq!(parse_description(output), Some("".to_string()));
    }

    // ==========================================================================
    // List-units parsing tests
    // ==========================================================================

    #[test]
    fn test_parse_list_units_empty() {
        let output = "";
        let services = parse_list_units(output);
        assert!(services.is_empty());
    }

    #[test]
    fn test_parse_list_units_single() {
        let output = "ssh.service                loaded active running OpenSSH Daemon";
        let services = parse_list_units(output);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "ssh");
        assert_eq!(services[0].state, ServiceState::Active);
    }

    #[test]
    fn test_parse_list_units_multiple() {
        let output = r#"bluetooth.service          loaded active running Bluetooth service
cups.service               loaded inactive dead    CUPS Scheduler
docker.service             loaded active running Docker Application Container Engine
sshd.service               loaded active running OpenSSH Daemon"#;
        let services = parse_list_units(output);
        assert_eq!(services.len(), 4);

        assert_eq!(services[0].name, "bluetooth");
        assert_eq!(services[0].state, ServiceState::Active);

        assert_eq!(services[1].name, "cups");
        assert_eq!(services[1].state, ServiceState::Inactive);

        assert_eq!(services[2].name, "docker");
        assert_eq!(services[2].state, ServiceState::Active);

        assert_eq!(services[3].name, "sshd");
        assert_eq!(services[3].state, ServiceState::Active);
    }

    #[test]
    fn test_parse_list_units_failed_service() {
        let output = "broken.service             loaded failed failed Broken Service";
        let services = parse_list_units(output);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "broken");
        assert_eq!(services[0].state, ServiceState::Failed);
    }

    #[test]
    fn test_parse_list_units_with_description() {
        let output = "pipewire.service           loaded active running PipeWire Multimedia Service";
        let services = parse_list_units(output);
        assert_eq!(services.len(), 1);
        assert_eq!(
            services[0].description,
            Some("PipeWire Multimedia Service".to_string())
        );
    }

    #[test]
    fn test_parse_list_units_strips_service_suffix() {
        let output = "test.service               loaded active running Test";
        let services = parse_list_units(output);
        assert_eq!(services[0].name, "test");
    }

    #[test]
    fn test_parse_list_units_whitespace_handling() {
        let output = "  service.service    loaded    active    running    Description  ";
        let services = parse_list_units(output);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "service");
    }

    // ==========================================================================
    // DefaultServiceManager tests
    // ==========================================================================

    #[test]
    fn test_service_manager_user() {
        let manager = DefaultServiceManager::user();
        assert_eq!(manager.scope, ServiceScope::User);
    }

    #[test]
    fn test_service_manager_system() {
        let manager = DefaultServiceManager::system();
        assert_eq!(manager.scope, ServiceScope::System);
    }

    #[test]
    fn test_service_manager_new() {
        let user = DefaultServiceManager::new(ServiceScope::User);
        assert_eq!(user.scope, ServiceScope::User);

        let system = DefaultServiceManager::new(ServiceScope::System);
        assert_eq!(system.scope, ServiceScope::System);
    }

    // ==========================================================================
    // Mock ServiceManager for testing dependent code
    // ==========================================================================

    /// Mock service manager for testing
    pub struct MockServiceManager {
        pub services: std::collections::HashMap<String, ServiceInfo>,
        pub fail_operations: bool,
    }

    impl Default for MockServiceManager {
        fn default() -> Self {
            Self {
                services: std::collections::HashMap::new(),
                fail_operations: false,
            }
        }
    }

    impl MockServiceManager {
        pub fn with_services(services: Vec<ServiceInfo>) -> Self {
            let map = services.into_iter().map(|s| (s.name.clone(), s)).collect();
            Self {
                services: map,
                fail_operations: false,
            }
        }
    }

    impl ServiceManager for MockServiceManager {
        fn status(&self, name: &str) -> IronResult<ServiceInfo> {
            self.services.get(name).cloned().ok_or_else(|| {
                ServiceError::NotFound {
                    name: name.to_string(),
                }
                .into()
            })
        }

        fn enable(&self, _name: &str) -> IronResult<()> {
            if self.fail_operations {
                Err(ServiceError::EnableFailed {
                    name: _name.to_string(),
                    message: "Mock failure".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }

        fn disable(&self, _name: &str) -> IronResult<()> {
            if self.fail_operations {
                Err(ServiceError::DisableFailed {
                    name: _name.to_string(),
                    message: "Mock failure".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }

        fn start(&self, _name: &str) -> IronResult<()> {
            if self.fail_operations {
                Err(ServiceError::StartFailed {
                    name: _name.to_string(),
                    message: "Mock failure".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }

        fn stop(&self, _name: &str) -> IronResult<()> {
            if self.fail_operations {
                Err(ServiceError::StopFailed {
                    name: _name.to_string(),
                    message: "Mock failure".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }

        fn restart(&self, _name: &str) -> IronResult<()> {
            if self.fail_operations {
                // Use StartFailed for restart failures (no RestartFailed variant)
                Err(ServiceError::StartFailed {
                    name: _name.to_string(),
                    message: "Mock restart failure".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }

        fn exists(&self, name: &str) -> bool {
            self.services.contains_key(name)
        }

        fn list(&self, pattern: Option<&str>) -> IronResult<Vec<ServiceInfo>> {
            let services: Vec<ServiceInfo> = self
                .services
                .values()
                .filter(|s| match pattern {
                    Some(p) => s.name.contains(p),
                    None => true,
                })
                .cloned()
                .collect();
            Ok(services)
        }
    }

    #[test]
    fn test_mock_service_manager_status() {
        let mock = MockServiceManager::with_services(vec![ServiceInfo {
            name: "ssh".to_string(),
            state: ServiceState::Active,
            enabled: EnabledState::Enabled,
            description: Some("SSH Server".to_string()),
        }]);

        let info = mock.status("ssh").unwrap();
        assert_eq!(info.name, "ssh");
        assert_eq!(info.state, ServiceState::Active);
    }

    #[test]
    fn test_mock_service_manager_status_not_found() {
        let mock = MockServiceManager::default();
        let result = mock.status("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_service_manager_exists() {
        let mock = MockServiceManager::with_services(vec![ServiceInfo {
            name: "docker".to_string(),
            state: ServiceState::Active,
            enabled: EnabledState::Enabled,
            description: None,
        }]);

        assert!(mock.exists("docker"));
        assert!(!mock.exists("nonexistent"));
    }

    #[test]
    fn test_mock_service_manager_list() {
        let mock = MockServiceManager::with_services(vec![
            ServiceInfo {
                name: "pipewire".to_string(),
                state: ServiceState::Active,
                enabled: EnabledState::Enabled,
                description: None,
            },
            ServiceInfo {
                name: "pipewire-pulse".to_string(),
                state: ServiceState::Active,
                enabled: EnabledState::Enabled,
                description: None,
            },
            ServiceInfo {
                name: "wireplumber".to_string(),
                state: ServiceState::Active,
                enabled: EnabledState::Enabled,
                description: None,
            },
        ]);

        let all = mock.list(None).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = mock.list(Some("pipewire")).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_mock_service_manager_operations() {
        let mock = MockServiceManager::default();
        assert!(mock.enable("test").is_ok());
        assert!(mock.disable("test").is_ok());
        assert!(mock.start("test").is_ok());
        assert!(mock.stop("test").is_ok());
        assert!(mock.restart("test").is_ok());
    }

    #[test]
    fn test_mock_service_manager_failing_operations() {
        let mock = MockServiceManager {
            services: std::collections::HashMap::new(),
            fail_operations: true,
        };
        assert!(mock.enable("test").is_err());
        assert!(mock.disable("test").is_err());
        assert!(mock.start("test").is_err());
        assert!(mock.stop("test").is_err());
        assert!(mock.restart("test").is_err());
    }

    // ==========================================================================
    // Circuit Breaker Integration Tests
    // ==========================================================================

    #[test]
    fn test_service_manager_with_resilience_user() {
        let manager = DefaultServiceManager::with_resilience(ServiceScope::User);
        assert!(manager.executor.is_some());
        assert_eq!(manager.scope, ServiceScope::User);
    }

    #[test]
    fn test_service_manager_with_resilience_system() {
        let manager = DefaultServiceManager::with_resilience(ServiceScope::System);
        assert!(manager.executor.is_some());
        assert_eq!(manager.scope, ServiceScope::System);
    }

    #[test]
    fn test_service_manager_with_executor() {
        use iron_core::resilience::RealCommandExecutor;
        use std::sync::Arc;

        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let manager = DefaultServiceManager::with_executor(ServiceScope::User, executor);
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_service_manager_new_has_resilient_executor() {
        // new() always initializes with a circuit-breaker executor
        let manager = DefaultServiceManager::new(ServiceScope::User);
        assert!(manager.executor.is_some());
    }

    #[test]
    fn test_service_manager_user_has_resilient_executor() {
        let manager = DefaultServiceManager::user();
        assert!(manager.executor.is_some());
        assert_eq!(manager.scope, ServiceScope::User);
    }

    #[test]
    fn test_service_manager_system_has_resilient_executor() {
        let manager = DefaultServiceManager::system();
        assert!(manager.executor.is_some());
        assert_eq!(manager.scope, ServiceScope::System);
    }

    // ==========================================================================
    // SystemdServiceAdapter tests
    // ==========================================================================

    #[test]
    fn test_adapter_user_constructor() {
        let _adapter = SystemdServiceAdapter::user();
        // Verifies it can be constructed without panic
    }

    #[test]
    fn test_adapter_system_constructor() {
        let _adapter = SystemdServiceAdapter::system();
    }

    #[test]
    fn test_adapter_with_executor() {
        use iron_core::resilience::RealCommandExecutor;
        let executor = Arc::new(RealCommandExecutor::with_defaults());
        let _adapter = SystemdServiceAdapter::with_executor(ServiceScope::User, executor);
    }

    #[test]
    fn test_adapter_implements_system_service() {
        // Ensure the adapter can be used as Arc<dyn SystemService>
        use iron_core::SystemService;
        let adapter = SystemdServiceAdapter::user();
        let _arc: Arc<dyn SystemService> = Arc::new(adapter);
    }

    #[test]
    fn test_adapter_delegates_to_mock() {
        // Use test_fixtures to verify delegation works end-to-end
        use crate::test_fixtures::SystemdMockBuilder;
        use iron_core::SystemService;

        let executor = SystemdMockBuilder::new()
            .user_scope()
            .with_service("sshd", ServiceState::Active, EnabledState::Enabled)
            .build();
        let adapter = SystemdServiceAdapter::with_executor(ServiceScope::User, Arc::new(executor));

        // enable/disable/start/stop should all succeed (mock always succeeds for known services)
        assert!(adapter.enable_service("sshd").is_ok());
        assert!(adapter.disable_service("sshd").is_ok());
        assert!(adapter.start_service("sshd").is_ok());
        assert!(adapter.stop_service("sshd").is_ok());
    }
}
