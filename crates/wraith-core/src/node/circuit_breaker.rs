//! Circuit breaker pattern for error recovery and resilience
//!
//! Prevents cascade failures by tracking consecutive errors and opening the
//! circuit when a threshold is exceeded. Implements automatic recovery testing
//! through half-open state.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - requests pass through normally
    Closed,

    /// Circuit is open - requests fail immediately without trying
    Open,

    /// Circuit is half-open - testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,

    /// Duration to wait before transitioning from Open to HalfOpen
    pub timeout: Duration,

    /// Number of successful requests in HalfOpen before closing circuit
    pub success_threshold: u32,

    /// Reset failure count after successful request
    pub reset_on_success: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 2,
            reset_on_success: true,
        }
    }
}

/// Circuit breaker for a single peer
#[derive(Debug, Clone)]
struct PeerCircuit {
    /// Current state
    state: CircuitState,

    /// Consecutive failure count
    failure_count: u32,

    /// Consecutive success count (in HalfOpen state)
    success_count: u32,

    /// Last state transition time
    last_transition: Instant,

    /// Last failure time
    last_failure: Option<Instant>,

    /// Total failures
    total_failures: u64,

    /// Total successes
    total_successes: u64,

    /// Times circuit opened
    open_count: u64,
}

impl PeerCircuit {
    /// Create a new circuit in closed state
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_transition: Instant::now(),
            last_failure: None,
            total_failures: 0,
            total_successes: 0,
            open_count: 0,
        }
    }

    /// Check if circuit allows requests
    fn allows_request(&self, config: &CircuitBreakerConfig) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout expired
                let now = Instant::now();
                now.duration_since(self.last_transition) >= config.timeout
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Transition to half-open state if timeout expired
    fn try_transition_to_half_open(&mut self, config: &CircuitBreakerConfig) {
        if self.state == CircuitState::Open {
            let now = Instant::now();
            if now.duration_since(self.last_transition) >= config.timeout {
                self.state = CircuitState::HalfOpen;
                self.success_count = 0;
                self.last_transition = now;
            }
        }
    }

    /// Record a successful request
    fn record_success(&mut self, config: &CircuitBreakerConfig) {
        self.total_successes += 1;

        match self.state {
            CircuitState::Closed => {
                if config.reset_on_success {
                    self.failure_count = 0;
                }
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= config.success_threshold {
                    // Enough successes - close the circuit
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_transition = Instant::now();
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed request
    fn record_failure(&mut self, config: &CircuitBreakerConfig) {
        let now = Instant::now();
        self.total_failures += 1;
        self.last_failure = Some(now);

        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= config.failure_threshold {
                    // Open the circuit
                    self.state = CircuitState::Open;
                    self.last_transition = now;
                    self.open_count += 1;
                }
            }
            CircuitState::HalfOpen => {
                // Failed during testing - reopen circuit
                self.state = CircuitState::Open;
                self.failure_count = config.failure_threshold;
                self.success_count = 0;
                self.last_transition = now;
                self.open_count += 1;
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }
}

/// Circuit breaker manager for all peers
pub struct CircuitBreaker {
    /// Configuration
    config: CircuitBreakerConfig,

    /// Per-peer circuits
    circuits: Arc<RwLock<HashMap<[u8; 32], PeerCircuit>>>,
}

/// Circuit breaker metrics for a peer
#[derive(Debug, Clone)]
pub struct CircuitMetrics {
    /// Current state
    pub state: CircuitState,

    /// Consecutive failures
    pub failure_count: u32,

    /// Total failures
    pub total_failures: u64,

    /// Total successes
    pub total_successes: u64,

    /// Times circuit opened
    pub open_count: u64,

    /// Last failure time
    pub last_failure: Option<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            circuits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if requests are allowed for a peer
    pub async fn allows_request(&self, peer_id: &[u8; 32]) -> bool {
        let mut circuits = self.circuits.write().await;
        let circuit = circuits.entry(*peer_id).or_insert_with(PeerCircuit::new);

        circuit.try_transition_to_half_open(&self.config);
        circuit.allows_request(&self.config)
    }

    /// Record a successful request for a peer
    pub async fn record_success(&self, peer_id: &[u8; 32]) {
        let mut circuits = self.circuits.write().await;
        let circuit = circuits.entry(*peer_id).or_insert_with(PeerCircuit::new);
        circuit.record_success(&self.config);
    }

    /// Record a failed request for a peer
    pub async fn record_failure(&self, peer_id: &[u8; 32]) {
        let mut circuits = self.circuits.write().await;
        let circuit = circuits.entry(*peer_id).or_insert_with(PeerCircuit::new);
        circuit.record_failure(&self.config);
    }

    /// Get current state for a peer
    pub async fn state(&self, peer_id: &[u8; 32]) -> CircuitState {
        let circuits = self.circuits.read().await;
        circuits
            .get(peer_id)
            .map(|c| c.state)
            .unwrap_or(CircuitState::Closed)
    }

    /// Get metrics for a peer
    pub async fn metrics(&self, peer_id: &[u8; 32]) -> Option<CircuitMetrics> {
        let circuits = self.circuits.read().await;
        circuits.get(peer_id).map(|c| CircuitMetrics {
            state: c.state,
            failure_count: c.failure_count,
            total_failures: c.total_failures,
            total_successes: c.total_successes,
            open_count: c.open_count,
            last_failure: c.last_failure,
        })
    }

    /// Remove circuit for a peer
    pub async fn remove(&self, peer_id: &[u8; 32]) {
        let mut circuits = self.circuits.write().await;
        circuits.remove(peer_id);
    }

    /// Reset circuit for a peer (force close)
    pub async fn reset(&self, peer_id: &[u8; 32]) {
        let mut circuits = self.circuits.write().await;
        if let Some(circuit) = circuits.get_mut(peer_id) {
            circuit.state = CircuitState::Closed;
            circuit.failure_count = 0;
            circuit.success_count = 0;
            circuit.last_transition = Instant::now();
        }
    }
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial backoff duration
    pub initial_backoff: Duration,

    /// Maximum backoff duration
    pub max_backoff: Duration,

    /// Backoff multiplier
    pub multiplier: f64,

    /// Add random jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for attempt number
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let base = self.initial_backoff.as_millis() as f64 * self.multiplier.powi(attempt as i32);
        let capped = base.min(self.max_backoff.as_millis() as f64);

        let duration = if self.jitter {
            // Add up to 25% jitter
            use getrandom::fill as getrandom;
            let mut buf = [0u8; 4];
            let _ = getrandom(&mut buf);
            let jitter_factor = (u32::from_le_bytes(buf) % 25) as f64 / 100.0;
            capped * (1.0 + jitter_factor)
        } else {
            capped
        };

        Duration::from_millis(duration as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new(config);
        let peer_id = [1u8; 32];

        assert_eq!(breaker.state(&peer_id).await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [2u8; 32];

        // Record failures
        for _ in 0..3 {
            breaker.record_failure(&peer_id).await;
        }

        // Circuit should be open
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);

        // Requests should be blocked
        assert!(!breaker.allows_request(&peer_id).await);
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [3u8; 32];

        // Open the circuit
        breaker.record_failure(&peer_id).await;
        breaker.record_failure(&peer_id).await;
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should allow request (transition to HalfOpen)
        assert!(breaker.allows_request(&peer_id).await);
    }

    #[tokio::test]
    async fn test_circuit_closes_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(50),
            success_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [4u8; 32];

        // Open the circuit
        breaker.record_failure(&peer_id).await;
        breaker.record_failure(&peer_id).await;
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Transition to HalfOpen
        assert!(breaker.allows_request(&peer_id).await);

        // Record successes
        breaker.record_success(&peer_id).await;
        breaker.record_success(&peer_id).await;

        // Circuit should be closed
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_reopens_on_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [5u8; 32];

        // Open the circuit
        breaker.record_failure(&peer_id).await;
        breaker.record_failure(&peer_id).await;

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Transition to HalfOpen
        assert!(breaker.allows_request(&peer_id).await);

        // Fail in HalfOpen
        breaker.record_failure(&peer_id).await;

        // Should reopen
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_reset_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_on_success: true,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [6u8; 32];

        // Record some failures
        breaker.record_failure(&peer_id).await;
        breaker.record_failure(&peer_id).await;

        // Record success
        breaker.record_success(&peer_id).await;

        // Failure count should be reset
        let metrics = breaker.metrics(&peer_id).await.unwrap();
        assert_eq!(metrics.failure_count, 0);
    }

    #[tokio::test]
    async fn test_circuit_metrics() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new(config);
        let peer_id = [7u8; 32];

        // Record some operations
        breaker.record_failure(&peer_id).await;
        breaker.record_success(&peer_id).await;
        breaker.record_failure(&peer_id).await;

        let metrics = breaker.metrics(&peer_id).await.unwrap();
        assert_eq!(metrics.total_failures, 2);
        assert_eq!(metrics.total_successes, 1);
        assert!(metrics.last_failure.is_some());
    }

    #[tokio::test]
    async fn test_circuit_remove() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new(config);
        let peer_id = [8u8; 32];

        breaker.record_failure(&peer_id).await;
        assert!(breaker.metrics(&peer_id).await.is_some());

        breaker.remove(&peer_id).await;
        assert!(breaker.metrics(&peer_id).await.is_none());
    }

    #[tokio::test]
    async fn test_circuit_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        let peer_id = [9u8; 32];

        // Open circuit
        breaker.record_failure(&peer_id).await;
        breaker.record_failure(&peer_id).await;
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);

        // Reset
        breaker.reset(&peer_id).await;
        assert_eq!(breaker.state(&peer_id).await, CircuitState::Closed);
    }

    #[test]
    fn test_retry_backoff_calculation() {
        let config = RetryConfig {
            max_retries: 10,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: false,
        };

        // Attempt 0: 100ms
        let backoff0 = config.backoff_duration(0);
        assert_eq!(backoff0.as_millis(), 100);

        // Attempt 1: 200ms
        let backoff1 = config.backoff_duration(1);
        assert_eq!(backoff1.as_millis(), 200);

        // Attempt 2: 400ms
        let backoff2 = config.backoff_duration(2);
        assert_eq!(backoff2.as_millis(), 400);

        // Attempt 10: should be capped at max
        let backoff10 = config.backoff_duration(10);
        assert_eq!(backoff10.as_millis(), 10_000);
    }

    #[test]
    fn test_retry_backoff_with_jitter() {
        let config = RetryConfig {
            max_retries: 10,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: true,
        };

        // With jitter, should be within 25% of base
        let backoff0 = config.backoff_duration(0);
        assert!(backoff0.as_millis() >= 100);
        assert!(backoff0.as_millis() <= 125);
    }
}
