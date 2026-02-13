//! Iron Systemd - Service management for Iron configuration management
//!
//! This crate provides systemd integration for Iron, including:
//! - Service enable/disable
//! - Service start/stop
//! - Service status queries
//! - User vs system service handling

use iron_core::{IronResult, ServiceError};
use std::process::Command;

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
}

impl DefaultServiceManager {
    /// Create a new service manager
    pub fn new(scope: ServiceScope) -> Self {
        Self { scope }
    }

    /// Create a service manager for user services
    pub fn user() -> Self {
        Self::new(ServiceScope::User)
    }

    /// Create a service manager for system services
    pub fn system() -> Self {
        Self::new(ServiceScope::System)
    }

    /// Run systemctl command
    fn run_systemctl(&self, args: &[&str]) -> IronResult<String> {
        let mut cmd_args = vec![];

        if self.scope == ServiceScope::User {
            cmd_args.push("--user");
        }

        cmd_args.extend(args);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state() {
        assert_eq!(
            DefaultServiceManager::parse_state("active (running)"),
            ServiceState::Active
        );
        assert_eq!(
            DefaultServiceManager::parse_state("inactive (dead)"),
            ServiceState::Inactive
        );
        assert_eq!(
            DefaultServiceManager::parse_state("failed"),
            ServiceState::Failed
        );
    }

    #[test]
    fn test_enabled_state() {
        assert_eq!(
            DefaultServiceManager::parse_enabled("enabled"),
            EnabledState::Enabled
        );
        assert_eq!(
            DefaultServiceManager::parse_enabled("disabled"),
            EnabledState::Disabled
        );
        assert_eq!(
            DefaultServiceManager::parse_enabled("masked"),
            EnabledState::Masked
        );
    }

    #[test]
    fn test_service_scope() {
        let user = DefaultServiceManager::user();
        assert_eq!(user.scope, ServiceScope::User);

        let system = DefaultServiceManager::system();
        assert_eq!(system.scope, ServiceScope::System);
    }
}
