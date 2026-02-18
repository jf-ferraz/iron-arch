//! Mock Command Executor for Testing
//!
//! Provides a configurable mock implementation of [`CommandExecutor`] for testing
//! code that depends on external command execution without actually running commands.
//!
//! # Features
//!
//! - Pattern-based response matching (command + args)
//! - Timeout simulation
//! - Circuit breaker state manipulation
//! - Call history tracking for verification
//! - Failure mode injection
//!
//! # Example
//!
//! ```
//! use iron_core::resilience::{MockCommandExecutor, MockResponse, CommandExecutor};
//!
//! let mock = MockCommandExecutor::new();
//!
//! // Add a successful response
//! mock.add_response("pacman", &["-Qi", "linux"], MockResponse::success("Name: linux\nVersion: 6.7.0"));
//!
//! // Add a failure response
//! mock.add_response("pacman", &["-Qi", "nonexistent"], MockResponse::exit_error(1, "package not found"));
//!
//! // Execute and verify
//! let result = mock.execute("pacman", &["-Qi", "linux"]);
//! assert!(result.is_ok());
//! assert!(result.unwrap().contains("linux"));
//!
//! // Verify calls were made
//! assert_eq!(mock.call_count("pacman"), 1);
//! ```

use super::{CircuitOpenError, CommandConfig, CommandError, CommandExecutor, CommandOutput};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::Duration;

/// Mock response configuration for a command
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Successful execution with stdout content
    Success {
        stdout: String,
        stderr: String,
    },
    /// Failed execution with exit code and stderr
    Failure {
        exit_code: i32,
        stderr: String,
    },
    /// Simulate a timeout
    Timeout,
    /// Simulate circuit breaker open
    CircuitOpen {
        service: String,
    },
    /// Simulate spawn failure (command not found)
    SpawnFailed {
        message: String,
    },
    /// Simulate IO error
    IoError {
        message: String,
    },
    /// Simulate retries exhausted
    RetriesExhausted {
        attempts: u32,
        last_error: String,
    },
}

impl MockResponse {
    /// Create a successful response with stdout only
    pub fn success(stdout: &str) -> Self {
        Self::Success {
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    /// Create a successful response with both stdout and stderr
    pub fn success_with_stderr(stdout: &str, stderr: &str) -> Self {
        Self::Success {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    /// Create a failure response with exit code and error message
    pub fn exit_error(exit_code: i32, stderr: &str) -> Self {
        Self::Failure {
            exit_code,
            stderr: stderr.to_string(),
        }
    }

    /// Create a timeout response
    pub fn timeout() -> Self {
        Self::Timeout
    }

    /// Create a circuit open response
    pub fn circuit_open(service: &str) -> Self {
        Self::CircuitOpen {
            service: service.to_string(),
        }
    }

    /// Create a spawn failed response (command not found)
    pub fn not_found() -> Self {
        Self::SpawnFailed {
            message: "command not found".to_string(),
        }
    }

    /// Create an IO error response
    pub fn io_error(message: &str) -> Self {
        Self::IoError {
            message: message.to_string(),
        }
    }

    /// Convert to CommandError
    fn to_error(&self, command: &str) -> CommandError {
        match self {
            MockResponse::Success { .. } => panic!("Cannot convert success to error"),
            MockResponse::Failure { exit_code, stderr } => CommandError::ExitError {
                command: command.to_string(),
                exit_code: *exit_code,
                stderr: stderr.clone(),
            },
            MockResponse::Timeout => CommandError::Timeout {
                command: command.to_string(),
                timeout_secs: 120,
            },
            MockResponse::CircuitOpen { service } => {
                CommandError::CircuitOpen(CircuitOpenError {
                    service: service.clone(),
                    time_until_reset: Some(Duration::from_secs(30)),
                })
            }
            MockResponse::SpawnFailed { message } => CommandError::SpawnFailed {
                command: command.to_string(),
                message: message.clone(),
            },
            MockResponse::IoError { message } => CommandError::IoError {
                command: command.to_string(),
                message: message.clone(),
            },
            MockResponse::RetriesExhausted {
                attempts,
                last_error,
            } => CommandError::RetriesExhausted {
                command: command.to_string(),
                attempts: *attempts,
                last_error: last_error.clone(),
            },
        }
    }
}

/// Key for matching commands with their arguments
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CommandKey {
    command: String,
    args: Vec<String>,
}

impl CommandKey {
    fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[allow(dead_code)]
    fn command_only(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: vec![],
        }
    }
}

/// Call record for verification
#[derive(Debug, Clone)]
pub struct CallRecord {
    /// Command that was called
    pub command: String,
    /// Arguments passed
    pub args: Vec<String>,
    /// Environment variables if any
    pub env: Option<Vec<(String, String)>>,
}

/// Failure mode for injecting errors
#[derive(Debug, Clone)]
pub enum FailureMode {
    /// Fail all commands
    FailAll(MockResponse),
    /// Fail after N successful calls
    FailAfter {
        successes: usize,
        response: MockResponse,
    },
    /// Fail specific commands matching pattern
    FailPattern {
        pattern: String,
        response: MockResponse,
    },
}

/// Comprehensive mock implementation of CommandExecutor
///
/// Provides full control over command execution behavior for testing:
/// - Configure responses per command+args combination
/// - Simulate various failure modes
/// - Track call history for verification
/// - Manipulate circuit breaker state
pub struct MockCommandExecutor {
    config: CommandConfig,
    /// Responses keyed by command + args
    responses: RwLock<HashMap<CommandKey, MockResponse>>,
    /// Fallback responses keyed by command only (ignores args)
    fallback_responses: RwLock<HashMap<String, MockResponse>>,
    /// Default response when no match found
    default_response: RwLock<Option<MockResponse>>,
    /// Call history
    calls: RwLock<Vec<CallRecord>>,
    /// Total call count
    call_count: AtomicUsize,
    /// Failure mode
    failure_mode: RwLock<Option<FailureMode>>,
    /// Circuit breaker simulation state
    circuit_open: AtomicBool,
    /// Commands that "exist"
    existing_commands: RwLock<Vec<String>>,
}

impl Default for MockCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCommandExecutor {
    /// Create a new mock executor with default configuration
    pub fn new() -> Self {
        Self {
            config: CommandConfig::default(),
            responses: RwLock::new(HashMap::new()),
            fallback_responses: RwLock::new(HashMap::new()),
            default_response: RwLock::new(None),
            calls: RwLock::new(Vec::new()),
            call_count: AtomicUsize::new(0),
            failure_mode: RwLock::new(None),
            circuit_open: AtomicBool::new(false),
            existing_commands: RwLock::new(vec![
                "pacman".to_string(),
                "git".to_string(),
                "systemctl".to_string(),
                "which".to_string(),
            ]),
        }
    }

    /// Create a mock executor with custom configuration
    pub fn with_config(config: CommandConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Add a response for a specific command + args combination
    ///
    /// # Example
    /// ```
    /// use iron_core::resilience::{MockCommandExecutor, MockResponse};
    ///
    /// let mock = MockCommandExecutor::new();
    /// mock.add_response("pacman", &["-Qi", "linux"], MockResponse::success("Name: linux"));
    /// ```
    pub fn add_response(&self, command: &str, args: &[&str], response: MockResponse) {
        let key = CommandKey::new(command, args);
        self.responses.write().unwrap().insert(key, response);
    }

    /// Add a fallback response for a command (ignores args)
    ///
    /// Used when no exact command+args match is found.
    pub fn add_fallback_response(&self, command: &str, response: MockResponse) {
        self.fallback_responses
            .write()
            .unwrap()
            .insert(command.to_string(), response);
    }

    /// Set a default response for all unmatched commands
    pub fn set_default_response(&self, response: MockResponse) {
        *self.default_response.write().unwrap() = Some(response);
    }

    /// Set failure mode for error injection
    pub fn set_failure_mode(&self, mode: FailureMode) {
        *self.failure_mode.write().unwrap() = Some(mode);
    }

    /// Clear failure mode
    pub fn clear_failure_mode(&self) {
        *self.failure_mode.write().unwrap() = None;
    }

    /// Open the circuit breaker (simulate unavailable service)
    pub fn open_circuit(&self) {
        self.circuit_open.store(true, Ordering::SeqCst);
    }

    /// Close the circuit breaker (simulate recovery)
    pub fn close_circuit(&self) {
        self.circuit_open.store(false, Ordering::SeqCst);
    }

    /// Check if circuit is open
    pub fn is_circuit_open(&self) -> bool {
        self.circuit_open.load(Ordering::SeqCst)
    }

    /// Add a command to the "exists" list
    pub fn add_existing_command(&self, command: &str) {
        self.existing_commands
            .write()
            .unwrap()
            .push(command.to_string());
    }

    /// Remove a command from the "exists" list
    pub fn remove_existing_command(&self, command: &str) {
        self.existing_commands
            .write()
            .unwrap()
            .retain(|c| c != command);
    }

    /// Get total call count
    pub fn total_call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Get call count for a specific command
    pub fn call_count(&self, command: &str) -> usize {
        self.calls
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.command == command)
            .count()
    }

    /// Get all calls for a specific command
    pub fn calls_for(&self, command: &str) -> Vec<CallRecord> {
        self.calls
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.command == command)
            .cloned()
            .collect()
    }

    /// Get all recorded calls
    pub fn all_calls(&self) -> Vec<CallRecord> {
        self.calls.read().unwrap().clone()
    }

    /// Clear call history
    pub fn clear_calls(&self) {
        self.calls.write().unwrap().clear();
        self.call_count.store(0, Ordering::SeqCst);
    }

    /// Clear all responses
    pub fn clear_responses(&self) {
        self.responses.write().unwrap().clear();
        self.fallback_responses.write().unwrap().clear();
        *self.default_response.write().unwrap() = None;
    }

    /// Reset the mock to initial state
    pub fn reset(&self) {
        self.clear_calls();
        self.clear_responses();
        self.clear_failure_mode();
        self.close_circuit();
    }

    /// Record a call
    fn record_call(&self, command: &str, args: &[&str], env: Option<&[(&str, &str)]>) {
        let record = CallRecord {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            env: env.map(|e| e.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()),
        };
        self.calls.write().unwrap().push(record);
        self.call_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Check for failure mode
    fn check_failure_mode(&self, command: &str) -> Option<MockResponse> {
        let failure_mode = self.failure_mode.read().unwrap();
        match &*failure_mode {
            Some(FailureMode::FailAll(response)) => Some(response.clone()),
            Some(FailureMode::FailAfter { successes, response }) => {
                let count = self.total_call_count();
                if count >= *successes {
                    Some(response.clone())
                } else {
                    None
                }
            }
            Some(FailureMode::FailPattern { pattern, response }) => {
                if command.contains(pattern) {
                    Some(response.clone())
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// Get response for a command
    fn get_response(&self, command: &str, args: &[&str]) -> Option<MockResponse> {
        // Check exact match first
        let key = CommandKey::new(command, args);
        if let Some(response) = self.responses.read().unwrap().get(&key) {
            return Some(response.clone());
        }

        // Check fallback (command only)
        if let Some(response) = self.fallback_responses.read().unwrap().get(command) {
            return Some(response.clone());
        }

        // Check default
        self.default_response.read().unwrap().clone()
    }

    /// Internal execute implementation
    fn execute_internal(
        &self,
        command: &str,
        args: &[&str],
        env: Option<&[(&str, &str)]>,
    ) -> Result<CommandOutput, CommandError> {
        // Check circuit breaker first (before recording call)
        if self.is_circuit_open() {
            // Still record the call for tracking
            self.record_call(command, args, env);
            return Err(CommandError::CircuitOpen(CircuitOpenError {
                service: "mock".to_string(),
                time_until_reset: Some(Duration::from_secs(30)),
            }));
        }

        // Check failure mode BEFORE recording call (for FailAfter logic)
        // This way the count reflects completed calls, not including current
        if let Some(failure_response) = self.check_failure_mode(command) {
            self.record_call(command, args, env);
            return match failure_response {
                MockResponse::Success { stdout, stderr } => Ok(CommandOutput {
                    stdout,
                    stderr,
                    exit_code: 0,
                }),
                _ => Err(failure_response.to_error(command)),
            };
        }

        // Record the call
        self.record_call(command, args, env);

        // Get configured response
        if let Some(response) = self.get_response(command, args) {
            return match response {
                MockResponse::Success { stdout, stderr } => Ok(CommandOutput {
                    stdout,
                    stderr,
                    exit_code: 0,
                }),
                _ => Err(response.to_error(command)),
            };
        }

        // No response configured - return spawn failed
        Err(CommandError::SpawnFailed {
            command: command.to_string(),
            message: format!(
                "No mock response configured for '{}' with args {:?}",
                command, args
            ),
        })
    }
}

impl CommandExecutor for MockCommandExecutor {
    fn execute(&self, command: &str, args: &[&str]) -> Result<String, CommandError> {
        self.execute_internal(command, args, None)
            .map(|output| output.stdout)
    }

    fn execute_full(&self, command: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
        self.execute_internal(command, args, None)
    }

    fn execute_with_env(
        &self,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String, CommandError> {
        self.execute_internal(command, args, Some(env))
            .map(|output| output.stdout)
    }

    fn command_exists(&self, command: &str) -> bool {
        self.existing_commands.read().unwrap().contains(&command.to_string())
    }

    fn config(&self) -> &CommandConfig {
        &self.config
    }
}

// Make MockCommandExecutor Send + Sync safe
unsafe impl Send for MockCommandExecutor {}
unsafe impl Sync for MockCommandExecutor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_executor_success() {
        let mock = MockCommandExecutor::new();
        mock.add_response(
            "pacman",
            &["-Qi", "linux"],
            MockResponse::success("Name: linux\nVersion: 6.7.0"),
        );

        let result = mock.execute("pacman", &["-Qi", "linux"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("linux"));
    }

    #[test]
    fn test_mock_executor_failure() {
        let mock = MockCommandExecutor::new();
        mock.add_response(
            "pacman",
            &["-Qi", "nonexistent"],
            MockResponse::exit_error(1, "error: package 'nonexistent' was not found"),
        );

        let result = mock.execute("pacman", &["-Qi", "nonexistent"]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ExitError { exit_code, .. } => assert_eq!(exit_code, 1),
            _ => panic!("Expected ExitError"),
        }
    }

    #[test]
    fn test_mock_executor_timeout() {
        let mock = MockCommandExecutor::new();
        mock.add_response("slow", &[], MockResponse::timeout());

        let result = mock.execute("slow", &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::Timeout { .. }));
    }

    #[test]
    fn test_mock_executor_circuit_open() {
        let mock = MockCommandExecutor::new();
        mock.add_response("pacman", &["-Syu"], MockResponse::success("done"));
        mock.open_circuit();

        let result = mock.execute("pacman", &["-Syu"]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CommandError::CircuitOpen { .. }
        ));

        // Close circuit and retry
        mock.close_circuit();
        let result = mock.execute("pacman", &["-Syu"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_executor_call_tracking() {
        let mock = MockCommandExecutor::new();
        mock.add_fallback_response("git", MockResponse::success(""));

        mock.execute("git", &["status"]).unwrap();
        mock.execute("git", &["diff"]).unwrap();
        mock.execute("git", &["log"]).unwrap();

        assert_eq!(mock.call_count("git"), 3);
        assert_eq!(mock.total_call_count(), 3);

        let calls = mock.calls_for("git");
        assert_eq!(calls[0].args, vec!["status"]);
        assert_eq!(calls[1].args, vec!["diff"]);
        assert_eq!(calls[2].args, vec!["log"]);
    }

    #[test]
    fn test_mock_executor_fallback_response() {
        let mock = MockCommandExecutor::new();
        mock.add_fallback_response("echo", MockResponse::success("fallback"));

        let result = mock.execute("echo", &["anything", "here"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fallback");
    }

    #[test]
    fn test_mock_executor_default_response() {
        let mock = MockCommandExecutor::new();
        mock.set_default_response(MockResponse::success("default"));

        let result = mock.execute("any_command", &["any", "args"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "default");
    }

    #[test]
    fn test_mock_executor_failure_mode_all() {
        let mock = MockCommandExecutor::new();
        mock.add_response("cmd", &[], MockResponse::success("ok"));
        mock.set_failure_mode(FailureMode::FailAll(MockResponse::io_error("forced failure")));

        let result = mock.execute("cmd", &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::IoError { .. }));
    }

    #[test]
    fn test_mock_executor_failure_mode_after() {
        let mock = MockCommandExecutor::new();
        mock.add_fallback_response("cmd", MockResponse::success("ok"));
        mock.set_failure_mode(FailureMode::FailAfter {
            successes: 2,
            response: MockResponse::timeout(),
        });

        // First two calls succeed
        assert!(mock.execute("cmd", &[]).is_ok());
        assert!(mock.execute("cmd", &[]).is_ok());

        // Third call fails
        let result = mock.execute("cmd", &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::Timeout { .. }));
    }

    #[test]
    fn test_mock_executor_command_exists() {
        let mock = MockCommandExecutor::new();

        // Default commands exist
        assert!(mock.command_exists("pacman"));
        assert!(mock.command_exists("git"));

        // Non-default doesn't exist
        assert!(!mock.command_exists("nonexistent"));

        // Add custom command
        mock.add_existing_command("custom");
        assert!(mock.command_exists("custom"));

        // Remove command
        mock.remove_existing_command("pacman");
        assert!(!mock.command_exists("pacman"));
    }

    #[test]
    fn test_mock_executor_execute_full() {
        let mock = MockCommandExecutor::new();
        mock.add_response(
            "test",
            &[],
            MockResponse::success_with_stderr("stdout content", "stderr content"),
        );

        let result = mock.execute_full("test", &[]).unwrap();
        assert_eq!(result.stdout, "stdout content");
        assert_eq!(result.stderr, "stderr content");
        assert_eq!(result.exit_code, 0);
        assert!(result.success());
    }

    #[test]
    fn test_mock_executor_execute_with_env() {
        let mock = MockCommandExecutor::new();
        mock.add_response("env_cmd", &[], MockResponse::success("with env"));

        let result = mock.execute_with_env("env_cmd", &[], &[("KEY", "VALUE")]);
        assert!(result.is_ok());

        // Verify env was recorded
        let calls = mock.calls_for("env_cmd");
        assert_eq!(calls.len(), 1);
        assert!(calls[0].env.is_some());
        let env = calls[0].env.as_ref().unwrap();
        assert_eq!(env[0], ("KEY".to_string(), "VALUE".to_string()));
    }

    #[test]
    fn test_mock_executor_reset() {
        let mock = MockCommandExecutor::new();
        mock.add_response("cmd", &[], MockResponse::success("ok"));
        mock.open_circuit();
        let _ = mock.execute("cmd", &[]); // Will fail due to circuit

        mock.reset();

        assert!(!mock.is_circuit_open());
        assert_eq!(mock.total_call_count(), 0);

        // Response also cleared
        let result = mock.execute("cmd", &[]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CommandError::SpawnFailed { .. }
        ));
    }

    #[test]
    fn test_mock_response_constructors() {
        // success
        let r = MockResponse::success("output");
        assert!(matches!(r, MockResponse::Success { .. }));

        // exit_error
        let r = MockResponse::exit_error(127, "not found");
        assert!(matches!(r, MockResponse::Failure { .. }));

        // timeout
        let r = MockResponse::timeout();
        assert!(matches!(r, MockResponse::Timeout));

        // circuit_open
        let r = MockResponse::circuit_open("service");
        assert!(matches!(r, MockResponse::CircuitOpen { .. }));

        // not_found
        let r = MockResponse::not_found();
        assert!(matches!(r, MockResponse::SpawnFailed { .. }));

        // io_error
        let r = MockResponse::io_error("disk full");
        assert!(matches!(r, MockResponse::IoError { .. }));
    }

    #[test]
    fn test_mock_executor_pattern_priority() {
        let mock = MockCommandExecutor::new();

        // Add specific response
        mock.add_response("cmd", &["-a", "-b"], MockResponse::success("specific"));

        // Add fallback
        mock.add_fallback_response("cmd", MockResponse::success("fallback"));

        // Add default
        mock.set_default_response(MockResponse::success("default"));

        // Specific should match
        let result = mock.execute("cmd", &["-a", "-b"]).unwrap();
        assert_eq!(result, "specific");

        // Different args should get fallback
        let result = mock.execute("cmd", &["-c"]).unwrap();
        assert_eq!(result, "fallback");

        // Unknown command gets default
        let result = mock.execute("other", &[]).unwrap();
        assert_eq!(result, "default");
    }
}
