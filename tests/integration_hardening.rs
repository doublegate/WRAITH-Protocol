//! Integration tests for production hardening features
//!
//! Tests for:
//! - Rate limiting and DoS protection
//! - Health monitoring and graceful degradation
//! - Circuit breaker and error recovery

use std::net::IpAddr;
use std::time::Duration;
use wraith_core::node::health::HealthStatus;
use wraith_core::node::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthConfig, HealthMonitor,
    RateLimitConfig, RateLimiter,
};

#[tokio::test]
async fn test_rate_limiter_protects_against_dos() {
    let config = RateLimitConfig {
        max_connections_per_ip_per_minute: 5,
        max_packets_per_session_per_second: 100,
        max_bytes_per_session_per_second: 1_000_000,
        max_concurrent_sessions: 10,
        refill_interval: Duration::from_millis(100),
    };

    let limiter = RateLimiter::new(config);
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // First 5 connections should be allowed
    for _ in 0..5 {
        assert!(limiter.check_connection(ip).await);
    }

    // 6th connection should be blocked
    assert!(!limiter.check_connection(ip).await);

    // Metrics should reflect this
    let metrics = limiter.metrics().await;
    assert_eq!(metrics.connections_allowed, 5);
    assert_eq!(metrics.connections_blocked, 1);
}

#[tokio::test]
async fn test_rate_limiter_session_limit() {
    let config = RateLimitConfig {
        max_concurrent_sessions: 3,
        ..Default::default()
    };

    let limiter = RateLimiter::new(config);

    // Add 3 sessions
    for _ in 0..3 {
        assert!(limiter.check_session_limit().await);
        limiter.increment_sessions().await;
    }

    // 4th session should be blocked
    assert!(!limiter.check_session_limit().await);

    // Remove a session
    limiter.decrement_sessions().await;

    // Now should be able to add another
    assert!(limiter.check_session_limit().await);
}

#[tokio::test]
async fn test_health_monitor_transitions() {
    let config = HealthConfig {
        degraded_session_threshold: 5,
        critical_session_threshold: 10,
        transition_cooldown: Duration::from_millis(10),
        ..Default::default()
    };

    let monitor = HealthMonitor::new(config);

    // Start healthy
    monitor.update(2, 0).await;
    assert_eq!(monitor.status().await, HealthStatus::Healthy);
    assert!(monitor.should_accept_connection().await);
    assert!(monitor.should_accept_transfer().await);

    // Wait for cooldown
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Transition to degraded
    monitor.update(7, 0).await;
    assert_eq!(monitor.status().await, HealthStatus::Degraded);
    assert!(monitor.should_accept_connection().await); // Still accepts connections
    assert!(!monitor.should_accept_transfer().await); // But not new transfers

    // Wait for cooldown
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Transition to critical
    monitor.update(12, 0).await;
    assert_eq!(monitor.status().await, HealthStatus::Critical);
    assert!(!monitor.should_accept_connection().await);
    assert!(!monitor.should_accept_transfer().await);
    assert!(monitor.needs_emergency_cleanup().await);
}

#[tokio::test]
async fn test_circuit_breaker_prevents_cascade_failures() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        timeout: Duration::from_millis(100),
        success_threshold: 2,
        reset_on_success: true,
    };

    let breaker = CircuitBreaker::new(config);
    let peer_id = [1u8; 32];

    // Initially closed
    assert_eq!(breaker.state(&peer_id).await, CircuitState::Closed);
    assert!(breaker.allows_request(&peer_id).await);

    // Record failures to open circuit
    for _ in 0..3 {
        breaker.record_failure(&peer_id).await;
    }

    // Circuit should be open
    assert_eq!(breaker.state(&peer_id).await, CircuitState::Open);
    assert!(!breaker.allows_request(&peer_id).await);

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Should transition to half-open
    assert!(breaker.allows_request(&peer_id).await);

    // Record successes to close circuit
    breaker.record_success(&peer_id).await;
    breaker.record_success(&peer_id).await;

    // Circuit should be closed
    assert_eq!(breaker.state(&peer_id).await, CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_metrics() {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        ..Default::default()
    };

    let breaker = CircuitBreaker::new(config);
    let peer_id = [2u8; 32];

    // Record some operations
    breaker.record_success(&peer_id).await;
    breaker.record_success(&peer_id).await;
    breaker.record_failure(&peer_id).await;
    breaker.record_failure(&peer_id).await;

    let metrics = breaker.metrics(&peer_id).await.unwrap();
    assert_eq!(metrics.total_successes, 2);
    assert_eq!(metrics.total_failures, 2);
    assert_eq!(metrics.failure_count, 2);
    assert_eq!(metrics.state, CircuitState::Closed);
}

#[tokio::test]
async fn test_combined_protection_mechanisms() {
    // Simulate a scenario where all protection mechanisms work together
    let rate_config = RateLimitConfig {
        max_concurrent_sessions: 5,
        ..Default::default()
    };

    let health_config = HealthConfig {
        critical_session_threshold: 4,
        transition_cooldown: Duration::from_millis(10),
        ..Default::default()
    };

    let circuit_config = CircuitBreakerConfig {
        failure_threshold: 3,
        ..Default::default()
    };

    let limiter = RateLimiter::new(rate_config);
    let health = HealthMonitor::new(health_config);
    let circuit = CircuitBreaker::new(circuit_config);

    let peer_id = [3u8; 32];
    let _ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Add sessions up to limit
    for i in 0..4 {
        assert!(limiter.check_session_limit().await);
        limiter.increment_sessions().await;

        // Update health monitor
        health.update(i + 1, 0).await;
    }

    // Health should transition to critical
    tokio::time::sleep(Duration::from_millis(20)).await;
    health.update(4, 0).await;
    assert_eq!(health.status().await, HealthStatus::Critical);

    // New connections should be rejected by health monitor
    assert!(!health.should_accept_connection().await);

    // Circuit breaker should prevent requests to failing peer
    for _ in 0..3 {
        circuit.record_failure(&peer_id).await;
    }
    assert_eq!(circuit.state(&peer_id).await, CircuitState::Open);
    assert!(!circuit.allows_request(&peer_id).await);
}

#[tokio::test]
async fn test_rate_limiter_bandwidth_control() {
    let config = RateLimitConfig {
        max_bytes_per_session_per_second: 10_000,
        ..Default::default()
    };

    let limiter = RateLimiter::new(config);
    let session_id = [4u8; 32];

    // Transfer 8KB - should succeed
    assert!(limiter.check_bandwidth(&session_id, 8_000).await);

    // Transfer another 2KB - should succeed (total 10KB)
    assert!(limiter.check_bandwidth(&session_id, 2_000).await);

    // Transfer 1 more byte - should fail
    assert!(!limiter.check_bandwidth(&session_id, 1).await);
}

#[tokio::test]
async fn test_health_monitor_recovery() {
    let config = HealthConfig {
        degraded_session_threshold: 5,
        critical_session_threshold: 10,
        transition_cooldown: Duration::from_millis(10),
        ..Default::default()
    };

    let monitor = HealthMonitor::new(config);

    // Go critical
    monitor.update(12, 0).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    monitor.update(12, 0).await;
    assert_eq!(monitor.status().await, HealthStatus::Critical);

    // Recover to healthy
    tokio::time::sleep(Duration::from_millis(20)).await;
    monitor.update(2, 0).await;
    assert_eq!(monitor.status().await, HealthStatus::Healthy);

    // Check recovery metrics
    let metrics = monitor.metrics().await;
    assert_eq!(metrics.recovery_count, 1);
    assert_eq!(metrics.critical_count, 1);
}
