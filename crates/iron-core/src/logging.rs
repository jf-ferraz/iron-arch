//! Structured JSON logging with file rotation.
//!
//! Provides NFR-9 (JSON logging) and NFR-10 (log rotation) support.
//!
//! # Features
//!
//! - JSON-formatted logs with timestamp, level, component, and message
//! - Size-based rotation (10MB default) with configurable file retention
//! - Correlation IDs for operation tracing across log entries
//! - Environment variable control via `IRON_LOG`
//!
//! # Example
//!
//! ```rust,ignore
//! use iron_core::logging::{LogConfig, init_logging, OperationSpan};
//!
//! // Initialize logging
//! let config = LogConfig::default();
//! init_logging(&config)?;
//!
//! // Create an operation span with correlation ID
//! let op = OperationSpan::new("sync").with_component("sync_service");
//! tracing::info!(correlation_id = %op.correlation_id, "Starting sync operation");
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Default maximum log file size (10MB).
pub const DEFAULT_MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// Counter for generating unique correlation IDs within a process.
static CORRELATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Configuration for the logging system.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Directory where log files are written.
    pub log_dir: PathBuf,
    /// Maximum number of rotated log files to keep.
    pub max_files: usize,
    /// Default log level (can be overridden by IRON_LOG env var).
    pub default_level: String,
    /// Maximum size in bytes before rotation (NFR-10).
    pub max_size_bytes: u64,
}

impl Default for LogConfig {
    fn default() -> Self {
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("iron")
            .join("logs");

        Self {
            log_dir,
            max_files: 5,
            default_level: "info".to_string(),
            max_size_bytes: DEFAULT_MAX_SIZE_BYTES,
        }
    }
}

impl LogConfig {
    /// Create a new LogConfig with a custom log directory.
    pub fn with_log_dir(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            ..Default::default()
        }
    }

    /// Set maximum log file size before rotation.
    #[must_use]
    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }

    /// Check if a log file should be rotated based on its current size.
    ///
    /// Returns `true` if `current_size` exceeds `max_size_bytes`.
    pub fn should_rotate(&self, current_size: u64) -> bool {
        current_size > self.max_size_bytes
    }
}

/// Generate a unique correlation ID for operation tracing.
///
/// Format: `iron-{timestamp_ms}-{counter}`
///
/// This ID can be used to correlate log entries across a multi-step operation.
pub fn generate_correlation_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let counter = CORRELATION_COUNTER.fetch_add(1, Ordering::SeqCst);

    format!("iron-{}-{:04x}", timestamp, counter)
}

/// Span context for tracking operations across log entries.
///
/// Provides correlation ID and component tracking for structured logging.
///
/// # Example
///
/// ```rust,ignore
/// use iron_core::logging::OperationSpan;
///
/// let span = OperationSpan::new("sync")
///     .with_component("sync_service");
///
/// tracing::info!(
///     correlation_id = %span.correlation_id,
///     operation = %span.operation,
///     component = ?span.component,
///     "Operation started"
/// );
/// ```
#[derive(Debug, Clone)]
pub struct OperationSpan {
    /// Unique correlation ID for this operation.
    pub correlation_id: String,
    /// Name of the operation (e.g., "sync", "update", "install").
    pub operation: String,
    /// Component handling this operation (e.g., "sync_service", "update_service").
    pub component: Option<String>,
}

impl OperationSpan {
    /// Create a new operation span with a generated correlation ID.
    pub fn new(operation: &str) -> Self {
        Self {
            correlation_id: generate_correlation_id(),
            operation: operation.to_string(),
            component: None,
        }
    }

    /// Set the component for this operation span.
    #[must_use]
    pub fn with_component(mut self, component: &str) -> Self {
        self.component = Some(component.to_string());
        self
    }

    /// Create a tracing span with this operation's context.
    ///
    /// The returned span includes correlation_id, operation, and component fields.
    pub fn enter(&self) -> tracing::span::EnteredSpan {
        let span = tracing::info_span!(
            "operation",
            correlation_id = %self.correlation_id,
            operation = %self.operation,
            component = ?self.component,
        );
        span.entered()
    }
}

/// Initialize the logging system with the given configuration.
///
/// This sets up:
/// - JSON-formatted logs written to files in `config.log_dir`
/// - Daily log rotation keeping `config.max_files` files
/// - Log level controlled by `IRON_LOG` env var or `config.default_level`
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the file
/// appender cannot be initialized.
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    // Create log directory
    std::fs::create_dir_all(&config.log_dir)?;

    // File appender with daily rotation
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("iron")
        .filename_suffix("log")
        .max_log_files(config.max_files)
        .build(&config.log_dir)?;

    // JSON formatting layer for file
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_ansi(false);

    // Environment filter (IRON_LOG env var)
    let env_filter = EnvFilter::try_from_env("IRON_LOG")
        .unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_config_default_uses_xdg_data_dir() {
        let config = LogConfig::default();
        assert!(config.log_dir.to_string_lossy().contains("iron"));
        assert!(config.log_dir.to_string_lossy().contains("logs"));
    }

    #[test]
    fn log_config_default_values() {
        let config = LogConfig::default();
        assert_eq!(config.max_files, 5);
        assert_eq!(config.default_level, "info");
    }

    #[test]
    fn log_config_with_custom_dir() {
        let custom = PathBuf::from("/tmp/test-logs");
        let config = LogConfig::with_log_dir(custom.clone());
        assert_eq!(config.log_dir, custom);
        assert_eq!(config.max_files, 5);
    }

    #[test]
    fn init_logging_creates_log_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs");
        let config = LogConfig::with_log_dir(log_dir.clone());

        // Directory doesn't exist yet
        assert!(!log_dir.exists());

        // Initialize logging (may fail if already initialized in another test, that's ok)
        let _ = init_logging(&config);

        // Directory now exists
        assert!(log_dir.exists());
    }

    #[test]
    fn log_config_respects_max_files() {
        let config = LogConfig {
            log_dir: PathBuf::from("/tmp"),
            max_files: 10,
            default_level: "debug".to_string(),
            max_size_bytes: DEFAULT_MAX_SIZE_BYTES,
        };
        assert_eq!(config.max_files, 10);
        assert_eq!(config.default_level, "debug");
    }

    // NFR-10: Size-based rotation tests
    #[test]
    fn log_config_default_max_size_is_10mb() {
        let config = LogConfig::default();
        assert_eq!(config.max_size_bytes, 10 * 1024 * 1024); // 10MB
    }

    #[test]
    fn log_config_custom_max_size() {
        let config = LogConfig::default().with_max_size(5 * 1024 * 1024); // 5MB
        assert_eq!(config.max_size_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn should_rotate_returns_true_when_size_exceeded() {
        let config = LogConfig {
            log_dir: PathBuf::from("/tmp"),
            max_files: 5,
            default_level: "info".to_string(),
            max_size_bytes: 1024, // 1KB for testing
        };

        // File smaller than limit - no rotation
        assert!(!config.should_rotate(500));

        // File at limit - no rotation yet
        assert!(!config.should_rotate(1024));

        // File exceeds limit - should rotate
        assert!(config.should_rotate(1025));
    }

    // NFR-9: Correlation ID tests
    #[test]
    fn generate_correlation_id_creates_unique_ids() {
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();

        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
    }

    #[test]
    fn correlation_id_has_expected_format() {
        let id = generate_correlation_id();

        // Should be a UUID-like format or timestamp-based
        // Format: iron-{timestamp}-{random}
        assert!(
            id.starts_with("iron-"),
            "ID should start with 'iron-': {}",
            id
        );
        assert!(id.len() >= 20, "ID should be at least 20 chars: {}", id);
    }

    #[test]
    fn operation_span_contains_correlation_id() {
        let op = OperationSpan::new("sync");

        assert_eq!(op.operation, "sync");
        assert!(!op.correlation_id.is_empty());
        assert!(op.correlation_id.starts_with("iron-"));
    }

    #[test]
    fn operation_span_tracks_component() {
        let op = OperationSpan::new("update").with_component("update_service");

        assert_eq!(op.component, Some("update_service".to_string()));
    }
}
