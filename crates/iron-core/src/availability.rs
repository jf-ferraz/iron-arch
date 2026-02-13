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
}
