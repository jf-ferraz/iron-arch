//! Status Message System
//!
//! Provides timed status messages with automatic expiration.

use std::time::{Duration, Instant};

/// Default duration for status messages (3 seconds)
pub const DEFAULT_MESSAGE_DURATION: Duration = Duration::from_secs(3);

/// Extended duration for important messages (5 seconds)
pub const EXTENDED_MESSAGE_DURATION: Duration = Duration::from_secs(5);

/// Short duration for quick feedback (1.5 seconds)
pub const SHORT_MESSAGE_DURATION: Duration = Duration::from_millis(1500);

/// Message severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// Success - operation completed successfully (green)
    Success,
    /// Info - informational message (blue/cyan)
    Info,
    /// Warning - something needs attention (yellow)
    Warning,
    /// Error - operation failed (red)
    Error,
}

impl MessageLevel {
    /// Get the display symbol for this level
    pub fn symbol(&self) -> &'static str {
        match self {
            MessageLevel::Success => "✓",
            MessageLevel::Info => "ℹ",
            MessageLevel::Warning => "⚠",
            MessageLevel::Error => "✗",
        }
    }

    /// Get the default duration for this level
    pub fn default_duration(&self) -> Duration {
        match self {
            MessageLevel::Success => DEFAULT_MESSAGE_DURATION,
            MessageLevel::Info => DEFAULT_MESSAGE_DURATION,
            MessageLevel::Warning => EXTENDED_MESSAGE_DURATION,
            MessageLevel::Error => EXTENDED_MESSAGE_DURATION,
        }
    }
}

/// A status message with expiration time
#[derive(Debug, Clone)]
pub struct StatusMessage {
    /// The message text
    pub text: String,
    /// Message severity level
    pub level: MessageLevel,
    /// When this message expires
    pub expires_at: Instant,
}

impl StatusMessage {
    /// Create a new status message with default duration for its level
    pub fn new(text: impl Into<String>, level: MessageLevel) -> Self {
        let duration = level.default_duration();
        Self {
            text: text.into(),
            level,
            expires_at: Instant::now() + duration,
        }
    }

    /// Create a new status message with a custom duration
    pub fn with_duration(text: impl Into<String>, level: MessageLevel, duration: Duration) -> Self {
        Self {
            text: text.into(),
            level,
            expires_at: Instant::now() + duration,
        }
    }

    /// Create a success message
    pub fn success(text: impl Into<String>) -> Self {
        Self::new(text, MessageLevel::Success)
    }

    /// Create an info message
    pub fn info(text: impl Into<String>) -> Self {
        Self::new(text, MessageLevel::Info)
    }

    /// Create a warning message
    pub fn warning(text: impl Into<String>) -> Self {
        Self::new(text, MessageLevel::Warning)
    }

    /// Create an error message
    pub fn error(text: impl Into<String>) -> Self {
        Self::new(text, MessageLevel::Error)
    }

    /// Check if this message has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get time remaining until expiration
    pub fn time_remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }

    /// Get the message text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the message level
    pub fn level(&self) -> MessageLevel {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_message_level_symbol() {
        assert_eq!(MessageLevel::Success.symbol(), "✓");
        assert_eq!(MessageLevel::Info.symbol(), "ℹ");
        assert_eq!(MessageLevel::Warning.symbol(), "⚠");
        assert_eq!(MessageLevel::Error.symbol(), "✗");
    }

    #[test]
    fn test_message_level_default_duration() {
        assert_eq!(
            MessageLevel::Success.default_duration(),
            DEFAULT_MESSAGE_DURATION
        );
        assert_eq!(
            MessageLevel::Info.default_duration(),
            DEFAULT_MESSAGE_DURATION
        );
        assert_eq!(
            MessageLevel::Warning.default_duration(),
            EXTENDED_MESSAGE_DURATION
        );
        assert_eq!(
            MessageLevel::Error.default_duration(),
            EXTENDED_MESSAGE_DURATION
        );
    }

    #[test]
    fn test_status_message_new() {
        let msg = StatusMessage::new("Test message", MessageLevel::Success);
        assert_eq!(msg.text(), "Test message");
        assert_eq!(msg.level(), MessageLevel::Success);
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_status_message_success() {
        let msg = StatusMessage::success("Operation completed");
        assert_eq!(msg.level(), MessageLevel::Success);
    }

    #[test]
    fn test_status_message_info() {
        let msg = StatusMessage::info("Information");
        assert_eq!(msg.level(), MessageLevel::Info);
    }

    #[test]
    fn test_status_message_warning() {
        let msg = StatusMessage::warning("Warning");
        assert_eq!(msg.level(), MessageLevel::Warning);
    }

    #[test]
    fn test_status_message_error() {
        let msg = StatusMessage::error("Error occurred");
        assert_eq!(msg.level(), MessageLevel::Error);
    }

    #[test]
    fn test_status_message_with_duration() {
        let duration = Duration::from_millis(100);
        let msg = StatusMessage::with_duration("Quick message", MessageLevel::Info, duration);
        assert!(!msg.is_expired());

        // Wait for expiration
        sleep(Duration::from_millis(150));
        assert!(msg.is_expired());
    }

    #[test]
    fn test_status_message_time_remaining() {
        let msg = StatusMessage::with_duration("Test", MessageLevel::Info, Duration::from_secs(10));
        let remaining = msg.time_remaining();
        assert!(remaining.as_secs() >= 9);
    }

    #[test]
    fn test_status_message_clone() {
        let msg = StatusMessage::success("Test");
        let cloned = msg.clone();
        assert_eq!(msg.text(), cloned.text());
        assert_eq!(msg.level(), cloned.level());
    }

    #[test]
    fn test_message_level_equality() {
        assert_eq!(MessageLevel::Success, MessageLevel::Success);
        assert_ne!(MessageLevel::Success, MessageLevel::Error);
    }

    #[test]
    fn test_message_level_copy() {
        let level = MessageLevel::Warning;
        let copied = level;
        assert_eq!(level, copied);
    }

    #[test]
    fn test_message_level_debug() {
        let debug = format!("{:?}", MessageLevel::Error);
        assert!(debug.contains("Error"));
    }
}
