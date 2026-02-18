//! Resilience patterns for fault-tolerant operations
//!
//! This module provides resilience patterns for handling failures gracefully:
//!
//! - **Circuit Breaker**: Prevents cascading failures by failing fast when services are unavailable
//! - **Command Executor**: Fault-tolerant command execution with timeout and retry logic
//! - **Mock Executor**: Configurable mock for testing command execution
//!
//! # Circuit Breaker Example
//!
//! ```
//! use iron_core::resilience::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
//!
//! // Create a circuit breaker for pacman commands
//! let config = CircuitBreakerConfig::default()
//!     .with_failure_threshold(3);
//! let breaker = CircuitBreaker::new("pacman", config);
//!
//! // Check if we can execute
//! if breaker.can_execute() {
//!     // Execute the command
//!     // On success: breaker.record_success();
//!     // On failure: breaker.record_failure();
//! }
//! ```
//!
//! # Command Executor Example
//!
//! ```no_run
//! use iron_core::resilience::{CommandExecutor, CommandConfig, RealCommandExecutor};
//!
//! // Create executor with default 120s timeout
//! let executor = RealCommandExecutor::with_defaults();
//!
//! // Execute a command
//! let output = executor.execute("pacman", &["-Qi", "linux"]);
//! ```
//!
//! # Mock Executor Example (for testing)
//!
//! ```
//! use iron_core::resilience::{MockCommandExecutor, MockResponse, CommandExecutor};
//!
//! let mock = MockCommandExecutor::new();
//! mock.add_response("pacman", &["-Qi", "linux"], MockResponse::success("Name: linux"));
//!
//! let result = mock.execute("pacman", &["-Qi", "linux"]);
//! assert!(result.is_ok());
//! ```

mod circuit_breaker;
mod command_executor;
mod mock_executor;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitOpenError, CircuitState,
};
pub use command_executor::{
    CommandConfig, CommandError, CommandExecutor, CommandOutput, RealCommandExecutor,
};
pub use mock_executor::{CallRecord, FailureMode, MockCommandExecutor, MockResponse};
