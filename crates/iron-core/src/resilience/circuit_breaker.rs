//! Circuit Breaker Pattern Implementation
//!
//! Implements the circuit breaker pattern for fault-tolerant external command execution.
//! The circuit breaker prevents cascading failures by failing fast when a service is unavailable.
//!
//! # States
//!
//! - **Closed**: Normal operation, commands execute normally
//! - **Open**: Failing fast, commands return immediately with error
//! - **HalfOpen**: Testing if service has recovered
//!
//! # Usage
//!
//! ```
//! use iron_core::resilience::{CircuitBreaker, CircuitBreakerConfig};
//!
//! let config = CircuitBreakerConfig::default();
//! let breaker = CircuitBreaker::new("pacman", config);
//!
//! // Record success or failure
//! breaker.record_success();
//! breaker.record_failure();
//!
//! // Check if circuit allows execution
//! if breaker.can_execute() {
//!     // Execute command
//! }
//! ```

use std::sync::RwLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - commands execute normally
    Closed,
    /// Failing fast - commands return immediately with error
    Open,
    /// Testing recovery - allow one command through to test if service recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration the circuit stays open before transitioning to half-open
    pub reset_timeout: Duration,
    /// Number of successful calls in half-open state before closing
    pub success_threshold: u32,
    /// Command execution timeout
    pub command_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            command_timeout: Duration::from_secs(120),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new configuration with specified failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Create a new configuration with specified reset timeout
    pub fn with_reset_timeout(mut self, timeout: Duration) -> Self {
        self.reset_timeout = timeout;
        self
    }

    /// Create a new configuration with specified success threshold
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Create a new configuration with specified command timeout
    pub fn with_command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Create a strict configuration with low thresholds for critical services
    pub fn strict() -> Self {
        Self {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(60),
            success_threshold: 3,
            command_timeout: Duration::from_secs(120),
        }
    }

    /// Create a lenient configuration with higher thresholds for non-critical services
    pub fn lenient() -> Self {
        Self {
            failure_threshold: 10,
            reset_timeout: Duration::from_secs(15),
            success_threshold: 1,
            command_timeout: Duration::from_secs(120),
        }
    }
}

/// Circuit breaker for external command execution
///
/// Thread-safe implementation using atomic operations and RwLock.
pub struct CircuitBreaker {
    /// Name of the service (for logging and identification)
    name: String,
    /// Current state of the circuit
    state: RwLock<CircuitState>,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Consecutive failure count
    failure_count: AtomicU32,
    /// Consecutive success count (used in half-open state)
    success_count: AtomicU32,
    /// Timestamp when circuit opened (milliseconds since UNIX epoch)
    opened_at: AtomicU64,
    /// Total failures recorded
    total_failures: AtomicU32,
    /// Total successes recorded
    total_successes: AtomicU32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given name and configuration
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            state: RwLock::new(CircuitState::Closed),
            config,
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            opened_at: AtomicU64::new(0),
            total_failures: AtomicU32::new(0),
            total_successes: AtomicU32::new(0),
        }
    }

    /// Create a new circuit breaker with default configuration
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Get the name of the service this circuit breaker protects
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current state of the circuit
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Get the current configuration
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Get the current failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Get total failures recorded
    pub fn total_failures(&self) -> u32 {
        self.total_failures.load(Ordering::SeqCst)
    }

    /// Get total successes recorded
    pub fn total_successes(&self) -> u32 {
        self.total_successes.load(Ordering::SeqCst)
    }

    /// Check if the circuit breaker allows execution
    ///
    /// Returns `true` if the circuit is closed or has transitioned to half-open.
    pub fn can_execute(&self) -> bool {
        let current_state = self.state();

        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if reset timeout has elapsed
                if self.should_attempt_reset() {
                    self.transition_to_half_open();
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful execution
    ///
    /// In half-open state, may transition to closed after enough successes.
    pub fn record_success(&self) {
        self.total_successes.fetch_add(1, Ordering::SeqCst);

        let current_state = self.state();

        match current_state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                self.failure_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Record a failed execution
    ///
    /// In closed state, may transition to open after enough failures.
    /// In half-open state, immediately transitions back to open.
    pub fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::SeqCst);

        let current_state = self.state();

        match current_state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.failure_threshold {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open returns to open
                self.transition_to_open();
            }
            CircuitState::Open => {
                // Already open, just increment counter
                self.failure_count.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    /// Force the circuit to open state
    pub fn force_open(&self) {
        self.transition_to_open();
    }

    /// Force the circuit to closed state
    pub fn force_close(&self) {
        self.transition_to_closed();
    }

    /// Reset the circuit breaker to its initial state
    pub fn reset(&self) {
        self.transition_to_closed();
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
    }

    /// Get the time remaining before the circuit attempts to reset
    ///
    /// Returns `None` if the circuit is not open or if the reset timeout has elapsed.
    pub fn time_until_reset(&self) -> Option<Duration> {
        if self.state() != CircuitState::Open {
            return None;
        }

        let opened_at_millis = self.opened_at.load(Ordering::SeqCst);
        if opened_at_millis == 0 {
            return None;
        }

        let elapsed_millis = Self::current_time_millis().saturating_sub(opened_at_millis);
        let elapsed = Duration::from_millis(elapsed_millis);
        if elapsed >= self.config.reset_timeout {
            None
        } else {
            Some(self.config.reset_timeout - elapsed)
        }
    }

    /// Get statistics about the circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.name.clone(),
            state: self.state(),
            failure_count: self.failure_count(),
            total_failures: self.total_failures(),
            total_successes: self.total_successes(),
            time_until_reset: self.time_until_reset(),
        }
    }

    // Private helper methods

    fn should_attempt_reset(&self) -> bool {
        let opened_at_millis = self.opened_at.load(Ordering::SeqCst);
        if opened_at_millis == 0 {
            return true;
        }

        let elapsed_millis = Self::current_time_millis().saturating_sub(opened_at_millis);
        Duration::from_millis(elapsed_millis) >= self.config.reset_timeout
    }

    fn transition_to_open(&self) {
        let mut state = self.state.write().unwrap();
        *state = CircuitState::Open;
        self.opened_at
            .store(Self::current_time_millis(), Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
    }

    fn transition_to_half_open(&self) {
        let mut state = self.state.write().unwrap();
        *state = CircuitState::HalfOpen;
        self.success_count.store(0, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
    }

    fn transition_to_closed(&self) {
        let mut state = self.state.write().unwrap();
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        self.opened_at.store(0, Ordering::SeqCst);
    }

    /// Get current time as milliseconds since UNIX epoch
    fn current_time_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("name", &self.name)
            .field("state", &self.state())
            .field("failure_count", &self.failure_count())
            .field("config", &self.config)
            .finish()
    }
}

/// Statistics about a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Name of the service
    pub name: String,
    /// Current state
    pub state: CircuitState,
    /// Current consecutive failure count
    pub failure_count: u32,
    /// Total failures recorded
    pub total_failures: u32,
    /// Total successes recorded
    pub total_successes: u32,
    /// Time until reset attempt (if open)
    pub time_until_reset: Option<Duration>,
}

/// Error returned when circuit is open
#[derive(Debug, Clone)]
pub struct CircuitOpenError {
    /// Name of the service
    pub service: String,
    /// Time until reset attempt
    pub time_until_reset: Option<Duration>,
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.time_until_reset {
            Some(duration) => write!(
                f,
                "Circuit breaker for '{}' is open. Retry in {:.1}s",
                self.service,
                duration.as_secs_f64()
            ),
            None => write!(
                f,
                "Circuit breaker for '{}' is open. Ready for retry.",
                self.service
            ),
        }
    }
}

impl std::error::Error for CircuitOpenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_initial_state_is_closed() {
        let breaker = CircuitBreaker::with_defaults("test");
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());
    }

    #[test]
    fn test_opens_after_failure_threshold() {
        let config = CircuitBreakerConfig::default().with_failure_threshold(3);
        let breaker = CircuitBreaker::new("test", config);

        // Record failures up to threshold
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.record_failure();

        // Should now be open
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.can_execute());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig::default().with_failure_threshold(3);
        let breaker = CircuitBreaker::new("test", config);

        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 2);

        breaker.record_success();
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_after_reset_timeout() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_reset_timeout(Duration::from_millis(50));
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        thread::sleep(Duration::from_millis(60));

        // Should transition to half-open on next can_execute call
        assert!(breaker.can_execute());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_closes_after_success_threshold() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_success_threshold(2)
            .with_reset_timeout(Duration::from_millis(10));
        let breaker = CircuitBreaker::new("test", config);

        // Open and wait for half-open
        breaker.record_failure();
        breaker.record_failure();
        thread::sleep(Duration::from_millis(20));
        breaker.can_execute(); // Triggers half-open

        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record successes
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        breaker.record_success();

        // Should now be closed
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_reopens_on_failure() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_reset_timeout(Duration::from_millis(10));
        let breaker = CircuitBreaker::new("test", config);

        // Open and wait for half-open
        breaker.record_failure();
        breaker.record_failure();
        thread::sleep(Duration::from_millis(20));
        breaker.can_execute();

        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Failure in half-open reopens circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_force_open_and_close() {
        let breaker = CircuitBreaker::with_defaults("test");

        breaker.force_open();
        assert_eq!(breaker.state(), CircuitState::Open);

        breaker.force_close();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_reset_clears_all_counters() {
        let config = CircuitBreakerConfig::default().with_failure_threshold(2);
        let breaker = CircuitBreaker::new("test", config);

        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
    }

    #[test]
    fn test_stats() {
        let breaker = CircuitBreaker::with_defaults("test-service");

        breaker.record_success();
        breaker.record_failure();
        breaker.record_success();

        let stats = breaker.stats();
        assert_eq!(stats.name, "test-service");
        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.total_successes, 2);
        assert_eq!(stats.total_failures, 1);
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "Closed");
        assert_eq!(format!("{}", CircuitState::Open), "Open");
        assert_eq!(format!("{}", CircuitState::HalfOpen), "HalfOpen");
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(10)
            .with_reset_timeout(Duration::from_secs(60))
            .with_success_threshold(5)
            .with_command_timeout(Duration::from_secs(180));

        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.reset_timeout, Duration::from_secs(60));
        assert_eq!(config.success_threshold, 5);
        assert_eq!(config.command_timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_strict_config() {
        let config = CircuitBreakerConfig::strict();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.reset_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_lenient_config() {
        let config = CircuitBreakerConfig::lenient();
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.reset_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_circuit_open_error_display() {
        let error = CircuitOpenError {
            service: "pacman".to_string(),
            time_until_reset: Some(Duration::from_secs(10)),
        };
        assert!(error.to_string().contains("pacman"));
        assert!(error.to_string().contains("10"));

        let error_no_time = CircuitOpenError {
            service: "git".to_string(),
            time_until_reset: None,
        };
        assert!(error_no_time.to_string().contains("Ready for retry"));
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;

        let breaker = Arc::new(CircuitBreaker::with_defaults("concurrent"));
        let mut handles = vec![];

        // Spawn multiple threads recording successes and failures
        for i in 0..10 {
            let breaker_clone = Arc::clone(&breaker);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    if i % 2 == 0 {
                        breaker_clone.record_success();
                    } else {
                        breaker_clone.record_failure();
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Just verify it didn't panic and counters are reasonable
        let total = breaker.total_successes() + breaker.total_failures();
        assert_eq!(total, 1000);
    }

    #[test]
    fn test_name_getter() {
        let breaker = CircuitBreaker::with_defaults("my-service");
        assert_eq!(breaker.name(), "my-service");
    }

    #[test]
    fn test_config_getter() {
        let config = CircuitBreakerConfig::default().with_failure_threshold(7);
        let breaker = CircuitBreaker::new("test", config);
        assert_eq!(breaker.config().failure_threshold, 7);
    }
}
