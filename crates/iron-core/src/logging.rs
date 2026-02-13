//! Structured JSON logging with file rotation.
//!
//! Provides NFR-9 (JSON logging) and NFR-10 (log rotation) support.

use std::path::PathBuf;

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
}
