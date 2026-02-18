//! Service availability detection for graceful degradation.
//!
//! Implements NFR-11: System remains usable when optional components fail.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of an optional service's availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum AvailabilityStatus {
    /// Service is fully available.
    Available,
    /// Service is available but using a fallback.
    Degraded(String),
    /// Service is not available.
    Unavailable(String),
}

impl AvailabilityStatus {
    /// Returns true if the service is fully available.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns true if the service can be used (available or degraded).
    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unavailable(_))
    }

    /// Returns the reason string if degraded or unavailable.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Degraded(r) | Self::Unavailable(r) => Some(r),
        }
    }
}

impl fmt::Display for AvailabilityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::Degraded(reason) => write!(f, "degraded: {}", reason),
            Self::Unavailable(reason) => write!(f, "unavailable: {}", reason),
        }
    }
}

/// Availability status of all optional services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAvailability {
    /// git-crypt for secrets management.
    pub secrets: AvailabilityStatus,
    /// Git remote for sync operations.
    pub sync: AvailabilityStatus,
    /// Snapshot tool (timeshift or snapper).
    pub snapshots: AvailabilityStatus,
    /// AUR helper (paru or yay).
    pub aur: AvailabilityStatus,
}

impl ServiceAvailability {
    /// Check availability of all optional services.
    pub fn check() -> Self {
        Self {
            secrets: Self::check_secrets(),
            sync: Self::check_sync(),
            snapshots: Self::check_snapshots(),
            aur: Self::check_aur(),
        }
    }

    fn check_secrets() -> AvailabilityStatus {
        if which::which("git-crypt").is_ok() {
            AvailabilityStatus::Available
        } else {
            AvailabilityStatus::Unavailable("git-crypt not installed".into())
        }
    }

    fn check_sync() -> AvailabilityStatus {
        if which::which("git").is_ok() {
            AvailabilityStatus::Available
        } else {
            AvailabilityStatus::Unavailable("git not installed".into())
        }
    }

    fn check_snapshots() -> AvailabilityStatus {
        if which::which("timeshift").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("snapper").is_ok() {
            AvailabilityStatus::Degraded("using snapper (timeshift preferred)".into())
        } else {
            AvailabilityStatus::Unavailable("no snapshot tool (timeshift or snapper)".into())
        }
    }

    fn check_aur() -> AvailabilityStatus {
        if which::which("paru").is_ok() {
            AvailabilityStatus::Available
        } else if which::which("yay").is_ok() {
            AvailabilityStatus::Degraded("using yay (paru preferred)".into())
        } else {
            AvailabilityStatus::Unavailable("no AUR helper (paru or yay)".into())
        }
    }

    /// Get warning messages for unavailable or degraded services.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        match &self.secrets {
            AvailabilityStatus::Unavailable(r) | AvailabilityStatus::Degraded(r) => {
                warnings.push(format!("Secrets: {}", r));
            }
            _ => {}
        }
        match &self.sync {
            AvailabilityStatus::Unavailable(r) | AvailabilityStatus::Degraded(r) => {
                warnings.push(format!("Sync: {}", r));
            }
            _ => {}
        }
        match &self.snapshots {
            AvailabilityStatus::Unavailable(r) | AvailabilityStatus::Degraded(r) => {
                warnings.push(format!("Snapshots: {}", r));
            }
            _ => {}
        }
        match &self.aur {
            AvailabilityStatus::Unavailable(r) | AvailabilityStatus::Degraded(r) => {
                warnings.push(format!("AUR: {}", r));
            }
            _ => {}
        }

        warnings
    }

    /// Returns true if any service is unavailable or degraded.
    pub fn has_warnings(&self) -> bool {
        !self.secrets.is_available()
            || !self.sync.is_available()
            || !self.snapshots.is_available()
            || !self.aur.is_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_status_is_available() {
        assert!(AvailabilityStatus::Available.is_available());
        assert!(!AvailabilityStatus::Degraded("test".into()).is_available());
        assert!(!AvailabilityStatus::Unavailable("test".into()).is_available());
    }

    #[test]
    fn availability_status_is_usable() {
        assert!(AvailabilityStatus::Available.is_usable());
        assert!(AvailabilityStatus::Degraded("test".into()).is_usable());
        assert!(!AvailabilityStatus::Unavailable("test".into()).is_usable());
    }

    #[test]
    fn availability_status_reason() {
        assert_eq!(AvailabilityStatus::Available.reason(), None);
        assert_eq!(
            AvailabilityStatus::Degraded("using fallback".into()).reason(),
            Some("using fallback")
        );
        assert_eq!(
            AvailabilityStatus::Unavailable("not installed".into()).reason(),
            Some("not installed")
        );
    }

    #[test]
    fn availability_status_display() {
        assert_eq!(format!("{}", AvailabilityStatus::Available), "available");
        assert_eq!(
            format!("{}", AvailabilityStatus::Degraded("yay".into())),
            "degraded: yay"
        );
        assert_eq!(
            format!("{}", AvailabilityStatus::Unavailable("missing".into())),
            "unavailable: missing"
        );
    }

    #[test]
    fn availability_status_serializes_to_json() {
        let available = serde_json::to_string(&AvailabilityStatus::Available).unwrap();
        assert!(available.contains("Available"));

        let degraded = serde_json::to_string(&AvailabilityStatus::Degraded("test".into())).unwrap();
        assert!(degraded.contains("Degraded"));
        assert!(degraded.contains("test"));
    }

    #[test]
    fn service_availability_check_returns_all_services() {
        let availability = ServiceAvailability::check();
        // All fields should be set
        let _ = &availability.secrets;
        let _ = &availability.sync;
        let _ = &availability.snapshots;
        let _ = &availability.aur;
    }

    #[test]
    fn service_availability_warnings_collects_unavailable() {
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Unavailable("no remote".into()),
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Unavailable("no helper".into()),
        };
        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("Sync")));
        assert!(warnings.iter().any(|w| w.contains("AUR")));
    }

    #[test]
    fn service_availability_has_warnings() {
        let all_available = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Available,
        };
        assert!(!all_available.has_warnings());

        let some_unavailable = ServiceAvailability {
            secrets: AvailabilityStatus::Unavailable("test".into()),
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Available,
        };
        assert!(some_unavailable.has_warnings());
    }

    // NFR-11: Graceful degradation scenario tests

    #[test]
    fn degradation_scenario_all_unavailable() {
        // Simulates a minimal system with no optional services
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Unavailable("git-crypt not installed".into()),
            sync: AvailabilityStatus::Unavailable("no remote configured".into()),
            snapshots: AvailabilityStatus::Unavailable("no snapshot tool".into()),
            aur: AvailabilityStatus::Unavailable("no AUR helper".into()),
        };

        // System should still be usable
        assert!(availability.has_warnings());
        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 4);

        // Warnings should be informative
        assert!(warnings.iter().any(|w| w.contains("git-crypt")));
        assert!(warnings.iter().any(|w| w.contains("remote")));
        assert!(warnings.iter().any(|w| w.contains("snapshot")));
        assert!(warnings.iter().any(|w| w.contains("AUR")));
    }

    #[test]
    fn degradation_scenario_fallback_tools() {
        // Simulates using fallback tools (yay instead of paru, snapper instead of timeshift)
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Degraded("using snapper (timeshift preferred)".into()),
            aur: AvailabilityStatus::Degraded("using yay (paru preferred)".into()),
        };

        // Degraded services should be usable
        assert!(availability.snapshots.is_usable());
        assert!(availability.aur.is_usable());

        // But should report warnings
        assert!(availability.has_warnings());
        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn degradation_scenario_partial_availability() {
        // Simulates typical scenario: git available but secrets/snapshot not configured
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Unavailable("git-crypt not initialized".into()),
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Unavailable("timeshift not installed".into()),
            aur: AvailabilityStatus::Available,
        };

        // Should have exactly 2 warnings
        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 2);

        // Core operations (sync, AUR) should work
        assert!(availability.sync.is_available());
        assert!(availability.aur.is_available());
    }

    #[test]
    fn degradation_status_json_serialization_roundtrip() {
        // Verify degradation status survives JSON serialization
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Available,
            sync: AvailabilityStatus::Degraded("offline mode".into()),
            snapshots: AvailabilityStatus::Unavailable("not installed".into()),
            aur: AvailabilityStatus::Available,
        };

        let json = serde_json::to_string(&availability).expect("serialize");
        let restored: ServiceAvailability = serde_json::from_str(&json).expect("deserialize");

        assert!(restored.secrets.is_available());
        assert!(restored.sync.is_usable());
        assert!(!restored.sync.is_available());
        assert!(!restored.snapshots.is_usable());
        assert!(restored.aur.is_available());
    }

    #[test]
    fn degradation_warning_messages_are_actionable() {
        // Verify warning messages provide actionable information
        let availability = ServiceAvailability {
            secrets: AvailabilityStatus::Unavailable("git-crypt not installed".into()),
            sync: AvailabilityStatus::Available,
            snapshots: AvailabilityStatus::Available,
            aur: AvailabilityStatus::Available,
        };

        let warnings = availability.warnings();
        assert_eq!(warnings.len(), 1);

        // Warning should identify the service and include the reason
        let warning = &warnings[0];
        assert!(warning.contains("Secrets"), "Should identify service: {}", warning);
        assert!(warning.contains("git-crypt"), "Should include reason: {}", warning);
    }
}
