//! Command Executor with Timeout and Retry Logic
//!
//! Provides fault-tolerant command execution with:
//! - Configurable timeout (default 120s per architecture spec)
//! - Retry logic with exponential backoff
//! - Circuit breaker integration for graceful degradation
//!
//! # Example
//!
//! ```no_run
//! use iron_core::resilience::{CommandExecutor, CommandConfig, RealCommandExecutor};
//!
//! let executor = RealCommandExecutor::new(CommandConfig::default());
//!
//! // Execute a simple command
//! if let Ok(output) = executor.execute("pacman", &["-Qi", "linux"]) {
//!     println!("Output: {}", output);
//! }
//!
//! // Execute with custom config
//! let config = CommandConfig::default()
//!     .with_timeout_secs(30)
//!     .with_max_retries(2);
//! let executor = RealCommandExecutor::new(config);
//! ```

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::{CircuitBreaker, CircuitBreakerConfig, CircuitOpenError};

/// Configuration for command execution
#[derive(Debug, Clone)]
pub struct CommandConfig {
    /// Command execution timeout (default: 120s per FR-5.9)
    pub timeout: Duration,
    /// Maximum retry attempts (default: 3)
    pub max_retries: u32,
    /// Initial retry delay (default: 1s)
    pub initial_retry_delay: Duration,
    /// Maximum retry delay (default: 30s)
    pub max_retry_delay: Duration,
    /// Backoff multiplier (default: 2.0)
    pub backoff_multiplier: f64,
    /// Whether to use circuit breaker (default: true)
    pub use_circuit_breaker: bool,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120), // FR-5.9: 120s timeout
            max_retries: 3,
            initial_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_circuit_breaker: true,
        }
    }
}

impl CommandConfig {
    /// Create a new config with specified timeout in seconds
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Create a new config with specified timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Create a new config with specified max retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Create a new config with specified initial retry delay
    pub fn with_initial_retry_delay(mut self, delay: Duration) -> Self {
        self.initial_retry_delay = delay;
        self
    }

    /// Create a new config with circuit breaker enabled/disabled
    pub fn with_circuit_breaker(mut self, enabled: bool) -> Self {
        self.use_circuit_breaker = enabled;
        self
    }

    /// Create a strict config for critical operations
    pub fn strict() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_retries: 1,
            initial_retry_delay: Duration::from_secs(2),
            max_retry_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            use_circuit_breaker: true,
        }
    }

    /// Create a lenient config for non-critical operations
    pub fn lenient() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            max_retries: 5,
            initial_retry_delay: Duration::from_millis(500),
            max_retry_delay: Duration::from_secs(60),
            backoff_multiplier: 1.5,
            use_circuit_breaker: true,
        }
    }

    /// Create a config without retries (single attempt)
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }
}

/// Error type for command execution
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Command timed out
    Timeout { command: String, timeout_secs: u64 },
    /// Command exited with non-zero status
    ExitError {
        command: String,
        exit_code: i32,
        stderr: String,
    },
    /// Command was terminated by signal
    Signaled { command: String, signal: i32 },
    /// Failed to spawn command
    SpawnFailed { command: String, message: String },
    /// IO error during command execution
    IoError { command: String, message: String },
    /// Circuit breaker is open
    CircuitOpen(CircuitOpenError),
    /// All retries exhausted
    RetriesExhausted {
        command: String,
        attempts: u32,
        last_error: String,
    },
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Timeout {
                command,
                timeout_secs,
            } => {
                write!(f, "Command '{}' timed out after {}s", command, timeout_secs)
            }
            CommandError::ExitError {
                command,
                exit_code,
                stderr,
            } => {
                write!(
                    f,
                    "Command '{}' exited with code {}: {}",
                    command, exit_code, stderr
                )
            }
            CommandError::Signaled { command, signal } => {
                write!(f, "Command '{}' terminated by signal {}", command, signal)
            }
            CommandError::SpawnFailed { command, message } => {
                write!(f, "Failed to spawn '{}': {}", command, message)
            }
            CommandError::IoError { command, message } => {
                write!(f, "IO error executing '{}': {}", command, message)
            }
            CommandError::CircuitOpen(err) => write!(f, "{}", err),
            CommandError::RetriesExhausted {
                command,
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "Command '{}' failed after {} attempts: {}",
                    command, attempts, last_error
                )
            }
        }
    }
}

impl std::error::Error for CommandError {}

impl CommandError {
    /// Check if this error is retriable
    pub fn is_retriable(&self) -> bool {
        match self {
            CommandError::Timeout { .. } => true,
            CommandError::IoError { .. } => true,
            CommandError::ExitError { exit_code, .. } => is_retriable_exit_code(*exit_code),
            _ => false,
        }
    }
}

/// Check if an exit code indicates a retriable error
fn is_retriable_exit_code(code: i32) -> bool {
    // Common retriable exit codes:
    // - Temporary failures, network issues, resource contention
    matches!(code, 5 | 75 | 69 | 70 | 73 | 74)
}

/// Result of a command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
}

impl CommandOutput {
    /// Check if command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined stdout and stderr
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Trait for executing external commands with fault tolerance
pub trait CommandExecutor: Send + Sync {
    /// Execute a command with the given arguments
    ///
    /// Returns the command output on success, or an error on failure.
    /// Respects timeout and retry configuration.
    fn execute(&self, command: &str, args: &[&str]) -> Result<String, CommandError>;

    /// Execute a command and return full output details
    fn execute_full(&self, command: &str, args: &[&str]) -> Result<CommandOutput, CommandError>;

    /// Execute a command with custom environment variables
    fn execute_with_env(
        &self,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String, CommandError>;

    /// Check if a command exists and is executable
    fn command_exists(&self, command: &str) -> bool;

    /// Get the current configuration
    fn config(&self) -> &CommandConfig;
}

/// Real implementation of CommandExecutor using std::process::Command
pub struct RealCommandExecutor {
    config: CommandConfig,
    circuit_breaker: Option<CircuitBreaker>,
}

impl RealCommandExecutor {
    /// Create a new executor with the given configuration
    pub fn new(config: CommandConfig) -> Self {
        let circuit_breaker = if config.use_circuit_breaker {
            Some(CircuitBreaker::new(
                "command_executor",
                CircuitBreakerConfig::default()
                    .with_command_timeout(config.timeout)
                    .with_failure_threshold(5),
            ))
        } else {
            None
        };

        Self {
            config,
            circuit_breaker,
        }
    }

    /// Create an executor with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CommandConfig::default())
    }

    /// Create an executor for a specific service with its own circuit breaker
    pub fn for_service(name: &str, config: CommandConfig) -> Self {
        let circuit_breaker = if config.use_circuit_breaker {
            Some(CircuitBreaker::new(
                name,
                CircuitBreakerConfig::default()
                    .with_command_timeout(config.timeout)
                    .with_failure_threshold(5),
            ))
        } else {
            None
        };

        Self {
            config,
            circuit_breaker,
        }
    }

    /// Get a reference to the circuit breaker if enabled
    pub fn circuit_breaker(&self) -> Option<&CircuitBreaker> {
        self.circuit_breaker.as_ref()
    }

    /// Execute a single attempt of the command with timeout
    fn execute_once(
        &self,
        command: &str,
        args: &[&str],
        env: Option<&[(&str, &str)]>,
    ) -> Result<CommandOutput, CommandError> {
        let mut cmd = Command::new(command);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        let mut child = cmd.spawn().map_err(|e| CommandError::SpawnFailed {
            command: command.to_string(),
            message: e.to_string(),
        })?;

        // Use channel-based timeout
        let timeout = self.config.timeout;
        let (tx, rx) = mpsc::channel();

        let child_stdout = child.stdout.take();
        let child_stderr = child.stderr.take();

        // Spawn thread to wait for child and read output
        thread::spawn(move || {
            let mut stdout_content = String::new();
            let mut stderr_content = String::new();

            if let Some(mut stdout) = child_stdout {
                let _ = stdout.read_to_string(&mut stdout_content);
            }
            if let Some(mut stderr) = child_stderr {
                let _ = stderr.read_to_string(&mut stderr_content);
            }

            let status = child.wait();
            let _ = tx.send((status, stdout_content, stderr_content));
        });

        // Wait for result with timeout
        match rx.recv_timeout(timeout) {
            Ok((status_result, stdout, stderr)) => {
                let status = status_result.map_err(|e| CommandError::IoError {
                    command: command.to_string(),
                    message: e.to_string(),
                })?;

                let exit_code = status.code().unwrap_or(-1);

                if status.success() {
                    Ok(CommandOutput {
                        stdout,
                        stderr,
                        exit_code,
                    })
                } else {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        if let Some(signal) = status.signal() {
                            return Err(CommandError::Signaled {
                                command: command.to_string(),
                                signal,
                            });
                        }
                    }

                    Err(CommandError::ExitError {
                        command: command.to_string(),
                        exit_code,
                        stderr: stderr.trim().to_string(),
                    })
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Command timed out
                // Note: The child process may still be running, but we can't easily kill it
                // from here without storing the child handle
                Err(CommandError::Timeout {
                    command: command.to_string(),
                    timeout_secs: timeout.as_secs(),
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // The sender thread panicked
                Err(CommandError::IoError {
                    command: command.to_string(),
                    message: "Command execution thread panicked".to_string(),
                })
            }
        }
    }

    /// Execute with retry logic
    fn execute_with_retry(
        &self,
        command: &str,
        args: &[&str],
        env: Option<&[(&str, &str)]>,
    ) -> Result<CommandOutput, CommandError> {
        let mut last_error = None;
        let mut delay = self.config.initial_retry_delay;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // Wait before retry with exponential backoff
                thread::sleep(delay);
                delay = std::cmp::min(
                    Duration::from_secs_f64(delay.as_secs_f64() * self.config.backoff_multiplier),
                    self.config.max_retry_delay,
                );
            }

            match self.execute_once(command, args, env) {
                Ok(output) => {
                    // Record success with circuit breaker
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_success();
                    }
                    return Ok(output);
                }
                Err(e) => {
                    // Record failure with circuit breaker
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_failure();
                    }

                    // Check if retriable
                    if e.is_retriable() && attempt < self.config.max_retries {
                        last_error = Some(e);
                        continue;
                    }

                    return Err(e);
                }
            }
        }

        // All retries exhausted
        Err(CommandError::RetriesExhausted {
            command: command.to_string(),
            attempts: self.config.max_retries + 1,
            last_error: last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }
}

impl CommandExecutor for RealCommandExecutor {
    fn execute(&self, command: &str, args: &[&str]) -> Result<String, CommandError> {
        // Check circuit breaker first
        if let Some(cb) = &self.circuit_breaker
            && !cb.can_execute() {
                return Err(CommandError::CircuitOpen(CircuitOpenError {
                    service: cb.name().to_string(),
                    time_until_reset: cb.time_until_reset(),
                }));
            }

        let output = self.execute_with_retry(command, args, None)?;
        Ok(output.stdout)
    }

    fn execute_full(&self, command: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
        // Check circuit breaker first
        if let Some(cb) = &self.circuit_breaker
            && !cb.can_execute() {
                return Err(CommandError::CircuitOpen(CircuitOpenError {
                    service: cb.name().to_string(),
                    time_until_reset: cb.time_until_reset(),
                }));
            }

        self.execute_with_retry(command, args, None)
    }

    fn execute_with_env(
        &self,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String, CommandError> {
        // Check circuit breaker first
        if let Some(cb) = &self.circuit_breaker
            && !cb.can_execute() {
                return Err(CommandError::CircuitOpen(CircuitOpenError {
                    service: cb.name().to_string(),
                    time_until_reset: cb.time_until_reset(),
                }));
            }

        let output = self.execute_with_retry(command, args, Some(env))?;
        Ok(output.stdout)
    }

    fn command_exists(&self, command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn config(&self) -> &CommandConfig {
        &self.config
    }
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockCommandExecutor {
    config: CommandConfig,
    responses: std::sync::RwLock<std::collections::HashMap<String, Result<String, CommandError>>>,
}

#[cfg(test)]
impl MockCommandExecutor {
    pub fn new() -> Self {
        Self {
            config: CommandConfig::default(),
            responses: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_config(config: CommandConfig) -> Self {
        Self {
            config,
            responses: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn add_response(&self, command: &str, response: Result<String, CommandError>) {
        self.responses
            .write()
            .unwrap()
            .insert(command.to_string(), response);
    }
}

#[cfg(test)]
impl CommandExecutor for MockCommandExecutor {
    fn execute(&self, command: &str, _args: &[&str]) -> Result<String, CommandError> {
        self.responses
            .read()
            .unwrap()
            .get(command)
            .cloned()
            .unwrap_or_else(|| {
                Err(CommandError::SpawnFailed {
                    command: command.to_string(),
                    message: "No mock response configured".to_string(),
                })
            })
    }

    fn execute_full(&self, command: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
        self.execute(command, args).map(|stdout| CommandOutput {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        })
    }

    fn execute_with_env(
        &self,
        command: &str,
        args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<String, CommandError> {
        self.execute(command, args)
    }

    fn command_exists(&self, command: &str) -> bool {
        self.responses.read().unwrap().contains_key(command)
    }

    fn config(&self) -> &CommandConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CommandConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.max_retries, 3);
        assert!(config.use_circuit_breaker);
    }

    #[test]
    fn test_config_builder() {
        let config = CommandConfig::default()
            .with_timeout_secs(60)
            .with_max_retries(5)
            .with_circuit_breaker(false);

        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert!(!config.use_circuit_breaker);
    }

    #[test]
    fn test_config_strict() {
        let config = CommandConfig::strict();
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 1);
    }

    #[test]
    fn test_config_lenient() {
        let config = CommandConfig::lenient();
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_config_no_retry() {
        let config = CommandConfig::no_retry();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_command_output_success() {
        let output = CommandOutput {
            stdout: "test".to_string(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert!(output.success());
    }

    #[test]
    fn test_command_output_combined() {
        let output = CommandOutput {
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
            exit_code: 0,
        };
        assert_eq!(output.combined(), "stdout\nstderr");
    }

    #[test]
    fn test_command_error_display() {
        let err = CommandError::Timeout {
            command: "pacman".to_string(),
            timeout_secs: 120,
        };
        assert!(err.to_string().contains("120"));
        assert!(err.to_string().contains("pacman"));
    }

    #[test]
    fn test_command_error_retriable() {
        let timeout_err = CommandError::Timeout {
            command: "test".to_string(),
            timeout_secs: 10,
        };
        assert!(timeout_err.is_retriable());

        let spawn_err = CommandError::SpawnFailed {
            command: "test".to_string(),
            message: "not found".to_string(),
        };
        assert!(!spawn_err.is_retriable());
    }

    #[test]
    fn test_mock_executor() {
        let mock = MockCommandExecutor::new();
        mock.add_response("echo", Ok("hello".to_string()));

        let result = mock.execute("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_mock_executor_error() {
        let mock = MockCommandExecutor::new();
        mock.add_response(
            "fail",
            Err(CommandError::ExitError {
                command: "fail".to_string(),
                exit_code: 1,
                stderr: "error".to_string(),
            }),
        );

        let result = mock.execute("fail", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_command_exists() {
        let executor =
            RealCommandExecutor::new(CommandConfig::no_retry().with_circuit_breaker(false));
        // 'ls' should exist on Unix systems
        #[cfg(unix)]
        assert!(executor.command_exists("ls"));
        // Non-existent command
        assert!(!executor.command_exists("definitely_not_a_real_command_12345"));
    }

    #[test]
    fn test_executor_simple_command() {
        let executor =
            RealCommandExecutor::new(CommandConfig::no_retry().with_circuit_breaker(false));

        #[cfg(unix)]
        {
            let result = executor.execute("echo", &["hello"]);
            assert!(result.is_ok());
            assert!(result.unwrap().contains("hello"));
        }
    }

    #[test]
    fn test_executor_with_circuit_breaker() {
        let executor = RealCommandExecutor::new(CommandConfig::default());
        assert!(executor.circuit_breaker().is_some());

        let executor_no_cb =
            RealCommandExecutor::new(CommandConfig::default().with_circuit_breaker(false));
        assert!(executor_no_cb.circuit_breaker().is_none());
    }

    #[test]
    fn test_executor_for_service() {
        let executor = RealCommandExecutor::for_service("pacman", CommandConfig::default());
        let cb = executor
            .circuit_breaker()
            .expect("should have circuit breaker");
        assert_eq!(cb.name(), "pacman");
    }
}
