//! System service management abstraction
//!
//! Defines the `SystemService` trait for enabling/disabling/starting/stopping
//! system services (e.g., via systemd). The concrete implementation is provided
//! by `iron-systemd`; `NoopSystemService` is used for testing and environments
//! without systemd.

use crate::IronResult;

/// Minimal service-management trait for dependency injection into bundle/module services.
///
/// This trait is intentionally narrow — only the operations needed by `BundleService`
/// are required. The full systemd API lives in `iron-systemd::ServiceManager`.
pub trait SystemService: Send + Sync {
    /// Enable a service (start at boot).
    fn enable_service(&self, name: &str) -> IronResult<()>;

    /// Disable a service (do not start at boot).
    fn disable_service(&self, name: &str) -> IronResult<()>;

    /// Start a service immediately.
    fn start_service(&self, name: &str) -> IronResult<()>;

    /// Stop a running service.
    fn stop_service(&self, name: &str) -> IronResult<()>;

    /// F1-013: Check if a service is enabled at boot.
    /// Default returns false (safe for NoopSystemService and tests).
    fn is_enabled(&self, _name: &str) -> IronResult<bool> {
        Ok(false)
    }
}

/// No-op system service for testing and environments without systemd.
#[derive(Debug, Clone, Default)]
pub struct NoopSystemService;

impl SystemService for NoopSystemService {
    fn enable_service(&self, _name: &str) -> IronResult<()> {
        Ok(())
    }

    fn disable_service(&self, _name: &str) -> IronResult<()> {
        Ok(())
    }

    fn start_service(&self, _name: &str) -> IronResult<()> {
        Ok(())
    }

    fn stop_service(&self, _name: &str) -> IronResult<()> {
        Ok(())
    }
}
