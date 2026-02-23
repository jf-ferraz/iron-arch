//! Response Envelope — standard wrapper for all --json CLI output
//!
//! `IronEnvelope<T>` wraps command results with metadata, status, and
//! error information. All Iron CLI commands producing JSON output should
//! use this format for consistency and machine-readability.

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Standard response envelope for all --json CLI output.
///
/// Wraps command results with metadata, status, and error information.
/// All Iron CLI commands producing JSON output use this format.
#[derive(Debug, Clone)]
pub struct IronEnvelope<T> {
    /// Whether the command succeeded
    pub ok: bool,
    /// The command that produced this response
    pub command: String,
    /// The response payload (present on success)
    pub data: Option<T>,
    /// Error details (present on failure)
    pub error: Option<EnvelopeError>,
    /// Response metadata
    pub meta: EnvelopeMeta,
}

/// Error detail in an envelope.
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeError {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Suggested action to resolve the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Additional structured error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Metadata attached to every envelope response.
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeMeta {
    /// When the response was generated
    pub timestamp: DateTime<Utc>,
    /// How long the command took in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Hostname of the machine
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Iron version
    pub version: String,
}

// Manual Serialize impl to keep the T: Serialize bound off the struct
// definition itself (AQ-3 sub-decision in architect.md).
impl<T: Serialize> Serialize for IronEnvelope<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("IronEnvelope", 5)?;
        state.serialize_field("ok", &self.ok)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("error", &self.error)?;
        state.serialize_field("meta", &self.meta)?;
        state.end()
    }
}

impl<T> IronEnvelope<T> {
    /// Create a success envelope wrapping data.
    pub fn success(command: &str, data: T, duration_ms: Option<u64>) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            data: Some(data),
            error: None,
            meta: EnvelopeMeta::now(duration_ms),
        }
    }
}

impl IronEnvelope<()> {
    /// Create an error envelope.
    pub fn error(command: &str, code: &str, message: &str, duration_ms: Option<u64>) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            data: None,
            error: Some(EnvelopeError {
                code: code.to_string(),
                message: message.to_string(),
                suggestion: None,
                details: None,
            }),
            meta: EnvelopeMeta::now(duration_ms),
        }
    }

    /// Create an error envelope with a recovery suggestion.
    pub fn error_with_suggestion(
        command: &str,
        code: &str,
        message: &str,
        suggestion: &str,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            data: None,
            error: Some(EnvelopeError {
                code: code.to_string(),
                message: message.to_string(),
                suggestion: Some(suggestion.to_string()),
                details: None,
            }),
            meta: EnvelopeMeta::now(duration_ms),
        }
    }
}

impl EnvelopeMeta {
    /// Create metadata with current timestamp.
    pub fn now(duration_ms: Option<u64>) -> Self {
        Self {
            timestamp: Utc::now(),
            duration_ms,
            host: gethostname::gethostname()
                .to_string_lossy()
                .into_owned()
                .into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_envelope() {
        let env = IronEnvelope::success("scan", vec!["git", "neovim"], Some(42));

        assert!(env.ok);
        assert_eq!(env.command, "scan");
        assert!(env.data.is_some());
        assert_eq!(env.data.as_ref().unwrap().len(), 2);
        assert!(env.error.is_none());
        assert_eq!(env.meta.duration_ms, Some(42));
    }

    #[test]
    fn test_error_envelope() {
        let env =
            IronEnvelope::<()>::error("apply", "NOT_INITIALIZED", "Iron not initialized", Some(5));

        assert!(!env.ok);
        assert_eq!(env.command, "apply");
        assert!(env.data.is_none());
        assert!(env.error.is_some());
        let err = env.error.as_ref().unwrap();
        assert_eq!(err.code, "NOT_INITIALIZED");
        assert_eq!(err.message, "Iron not initialized");
        assert!(err.suggestion.is_none());
    }

    #[test]
    fn test_error_envelope_with_suggestion() {
        let env = IronEnvelope::<()>::error_with_suggestion(
            "apply",
            "NOT_INITIALIZED",
            "Iron not initialized",
            "Run 'iron init' first",
            None,
        );

        assert!(!env.ok);
        let err = env.error.as_ref().unwrap();
        assert_eq!(err.suggestion, Some("Run 'iron init' first".to_string()));
    }

    #[test]
    fn test_envelope_meta_has_fields() {
        let meta = EnvelopeMeta::now(Some(100));

        assert_eq!(meta.duration_ms, Some(100));
        assert!(meta.host.is_some());
        assert!(!meta.host.as_ref().unwrap().is_empty());
        assert!(!meta.version.is_empty());
    }

    #[test]
    fn test_envelope_meta_none_duration() {
        let meta = EnvelopeMeta::now(None);
        assert!(meta.duration_ms.is_none());
    }

    #[test]
    fn test_success_envelope_serialization() {
        let env = IronEnvelope::success("status", "all good", Some(10));

        let json = serde_json::to_string_pretty(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["command"], "status");
        assert_eq!(parsed["data"], "all good");
        assert!(parsed["error"].is_null());
        assert!(parsed["meta"]["timestamp"].is_string());
        assert_eq!(parsed["meta"]["duration_ms"], 10);
        assert!(parsed["meta"]["version"].is_string());
    }

    #[test]
    fn test_error_envelope_serialization() {
        let env =
            IronEnvelope::<()>::error("diff", "HOST_NOT_FOUND", "No host configured", Some(3));

        let json = serde_json::to_string_pretty(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["command"], "diff");
        assert!(parsed["data"].is_null());
        assert_eq!(parsed["error"]["code"], "HOST_NOT_FOUND");
        assert_eq!(parsed["error"]["message"], "No host configured");
    }

    #[test]
    fn test_envelope_with_struct_data() {
        #[derive(Debug, Clone, Serialize)]
        struct StatusData {
            host: String,
            modules: usize,
        }

        let data = StatusData {
            host: "desktop".to_string(),
            modules: 14,
        };
        let env = IronEnvelope::success("status", data, Some(50));

        let json = serde_json::to_string_pretty(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["data"]["host"], "desktop");
        assert_eq!(parsed["data"]["modules"], 14);
    }

    #[test]
    fn test_envelope_has_exactly_five_top_level_keys() {
        let env = IronEnvelope::success("test", "payload", Some(1));
        let json = serde_json::to_string(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let obj = parsed.as_object().unwrap();
        assert_eq!(
            obj.len(),
            5,
            "Envelope should have exactly 5 top-level keys"
        );
        assert!(obj.contains_key("ok"));
        assert!(obj.contains_key("command"));
        assert!(obj.contains_key("data"));
        assert!(obj.contains_key("error"));
        assert!(obj.contains_key("meta"));
    }

    #[test]
    fn test_error_envelope_with_suggestion_serialization() {
        let env = IronEnvelope::<()>::error_with_suggestion(
            "apply",
            "HOST_MISSING",
            "No host found",
            "Run iron init first",
            Some(12),
        );

        let json = serde_json::to_string_pretty(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["error"]["code"], "HOST_MISSING");
        assert_eq!(parsed["error"]["message"], "No host found");
        assert_eq!(parsed["error"]["suggestion"], "Run iron init first");
        // details should be omitted (skip_serializing_if)
        assert!(parsed["error"].get("details").is_none());
    }

    #[test]
    fn test_success_envelope_none_duration() {
        let env = IronEnvelope::success("test", 42, None);

        let json = serde_json::to_string(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // duration_ms should be omitted when None (skip_serializing_if)
        assert!(parsed["meta"].get("duration_ms").is_none());
    }

    #[test]
    fn test_envelope_with_vec_data() {
        let data = vec!["git", "neovim", "fish"];
        let env = IronEnvelope::success("module.list", data, Some(5));

        let json = serde_json::to_string(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["data"].is_array());
        assert_eq!(parsed["data"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_envelope_with_empty_data() {
        let env = IronEnvelope::success("test", serde_json::Value::Null, Some(0));

        let json = serde_json::to_string(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ok"], true);
        assert!(parsed["data"].is_null());
    }
}
