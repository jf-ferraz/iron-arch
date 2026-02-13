//! Structured JSON logging with file rotation.
//!
//! Provides NFR-9 (JSON logging) and NFR-10 (log rotation) support.

use std::path::PathBuf;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Configuration for the logging system.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Directory where log files are written.
    pub log_dir: PathBuf,
    /// Maximum number of rotated log files to keep.
    pub max_files: usize,
    /// Default log level (can be overridden by IRON_LOG env var).
    pub default_level: String,
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
}
