//! Security monitoring and alerting
//!
//! Tracks security-relevant events and provides hooks for external monitoring:
//! - Failed handshakes
//! - Rate limit violations
//! - IP bans
//! - Suspicious patterns
//! - Anomaly detection
//!
//! Metrics can be exported to monitoring systems (Prometheus, etc.) and
//! alerts can trigger callbacks for incident response.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Per-IP event history for pattern detection
type IpEventMap = HashMap<IpAddr, Vec<(Instant, SecurityEventType)>>;

/// Security event types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecurityEventType {
    /// Handshake failed (crypto/protocol error)
    HandshakeFailed,

    /// Rate limit exceeded
    RateLimitExceeded,

    /// IP address banned (temporary)
    IpTempBanned,

    /// IP address banned (permanent)
    IpPermBanned,

    /// Invalid packet format
    InvalidPacket,

    /// Replay attack detected
    ReplayDetected,

    /// Connection limit exceeded
    ConnectionLimitExceeded,

    /// Bandwidth limit exceeded
    BandwidthLimitExceeded,

    /// Nonce overflow (rekey required)
    NonceOverflow,

    /// Suspicious pattern detected
    SuspiciousPattern,
}

/// Security event details
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: SecurityEventType,

    /// Source IP address
    pub source_ip: IpAddr,

    /// Timestamp
    pub timestamp: Instant,

    /// Optional message
    pub message: Option<String>,

    /// Optional session ID
    pub session_id: Option<[u8; 32]>,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(event_type: SecurityEventType, source_ip: IpAddr) -> Self {
        Self {
            event_type,
            source_ip,
            timestamp: Instant::now(),
            message: None,
            session_id: None,
        }
    }

    /// Add a message to the event
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add a session ID to the event
    pub fn with_session(mut self, session_id: [u8; 32]) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Security event callback type
pub type SecurityEventCallback = Arc<dyn Fn(SecurityEvent) + Send + Sync>;

/// Security metrics
#[derive(Debug, Default, Clone)]
pub struct SecurityMetrics {
    /// Total handshake failures
    pub handshake_failures: u64,

    /// Total rate limit violations
    pub rate_limit_violations: u64,

    /// Total temporary bans
    pub temp_bans: u64,

    /// Total permanent bans
    pub perm_bans: u64,

    /// Total invalid packets
    pub invalid_packets: u64,

    /// Total replay attacks detected
    pub replay_attacks: u64,

    /// Total connection limit hits
    pub connection_limit_hits: u64,

    /// Total bandwidth limit hits
    pub bandwidth_limit_hits: u64,

    /// Total nonce overflows
    pub nonce_overflows: u64,

    /// Total suspicious patterns
    pub suspicious_patterns: u64,

    /// Events per second (last minute)
    pub events_per_second: f64,

    /// Last metrics update time
    pub last_update: Option<Instant>,
}

/// Security monitor configuration
#[derive(Debug, Clone)]
pub struct SecurityMonitorConfig {
    /// Enable event logging
    pub enable_logging: bool,

    /// Event buffer size (for rate calculation)
    pub event_buffer_size: usize,

    /// Anomaly detection threshold (events per second)
    pub anomaly_threshold: f64,

    /// History retention duration
    pub history_retention: Duration,
}

impl Default for SecurityMonitorConfig {
    fn default() -> Self {
        Self {
            enable_logging: true,
            event_buffer_size: 1000,
            anomaly_threshold: 100.0, // 100 events/sec triggers anomaly alert
            history_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Security monitor
pub struct SecurityMonitor {
    /// Configuration
    config: SecurityMonitorConfig,

    /// Metrics
    metrics: Arc<RwLock<SecurityMetrics>>,

    /// Event history (for rate calculation)
    event_history: Arc<RwLock<Vec<(Instant, SecurityEventType)>>>,

    /// Per-IP event counts (for pattern detection)
    ip_events: Arc<RwLock<IpEventMap>>,

    /// Optional callback for events
    callback: Arc<RwLock<Option<SecurityEventCallback>>>,
}

impl SecurityMonitor {
    /// Create a new security monitor
    pub fn new(config: SecurityMonitorConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(SecurityMetrics::default())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            ip_events: Arc::new(RwLock::new(HashMap::new())),
            callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Set event callback
    pub async fn set_callback(&self, callback: SecurityEventCallback) {
        let mut cb = self.callback.write().await;
        *cb = Some(callback);
    }

    /// Record a security event
    pub async fn record_event(&self, event: SecurityEvent) {
        let mut metrics = self.metrics.write().await;
        let mut history = self.event_history.write().await;
        let mut ip_events = self.ip_events.write().await;

        // Update metrics
        match event.event_type {
            SecurityEventType::HandshakeFailed => metrics.handshake_failures += 1,
            SecurityEventType::RateLimitExceeded => metrics.rate_limit_violations += 1,
            SecurityEventType::IpTempBanned => metrics.temp_bans += 1,
            SecurityEventType::IpPermBanned => metrics.perm_bans += 1,
            SecurityEventType::InvalidPacket => metrics.invalid_packets += 1,
            SecurityEventType::ReplayDetected => metrics.replay_attacks += 1,
            SecurityEventType::ConnectionLimitExceeded => metrics.connection_limit_hits += 1,
            SecurityEventType::BandwidthLimitExceeded => metrics.bandwidth_limit_hits += 1,
            SecurityEventType::NonceOverflow => metrics.nonce_overflows += 1,
            SecurityEventType::SuspiciousPattern => metrics.suspicious_patterns += 1,
        }
        metrics.last_update = Some(Instant::now());

        // Update event history
        history.push((event.timestamp, event.event_type.clone()));

        // Trim history to buffer size
        if history.len() > self.config.event_buffer_size {
            let excess = history.len() - self.config.event_buffer_size;
            history.drain(0..excess);
        }

        // Update per-IP event counts
        let ip_entry = ip_events.entry(event.source_ip).or_default();
        ip_entry.push((event.timestamp, event.event_type.clone()));

        // Log event if enabled
        if self.config.enable_logging {
            let message = event.message.as_deref().unwrap_or("");
            match event.event_type {
                SecurityEventType::HandshakeFailed => {
                    tracing::warn!(
                        "Security: Handshake failed from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::RateLimitExceeded => {
                    tracing::warn!(
                        "Security: Rate limit exceeded from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::IpTempBanned => {
                    tracing::warn!(
                        "Security: IP {} temporarily banned: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::IpPermBanned => {
                    tracing::error!(
                        "Security: IP {} permanently banned: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::InvalidPacket => {
                    tracing::debug!(
                        "Security: Invalid packet from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::ReplayDetected => {
                    tracing::warn!(
                        "Security: Replay attack from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::ConnectionLimitExceeded => {
                    tracing::warn!(
                        "Security: Connection limit exceeded from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::BandwidthLimitExceeded => {
                    tracing::warn!(
                        "Security: Bandwidth limit exceeded from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::NonceOverflow => {
                    tracing::warn!(
                        "Security: Nonce overflow from {}: {}",
                        event.source_ip,
                        message
                    );
                }
                SecurityEventType::SuspiciousPattern => {
                    tracing::warn!(
                        "Security: Suspicious pattern from {}: {}",
                        event.source_ip,
                        message
                    );
                }
            }
        }

        // Trigger callback if set
        if let Some(callback) = self.callback.read().await.as_ref() {
            callback(event);
        }
    }

    /// Calculate events per second
    pub async fn calculate_event_rate(&self) -> f64 {
        let history = self.event_history.read().await;

        if history.is_empty() {
            return 0.0;
        }

        let now = Instant::now();

        // Use checked_sub to avoid underflow on Windows or early system uptime
        let one_minute_ago = match now.checked_sub(Duration::from_secs(60)) {
            Some(t) => t,
            None => {
                // Less than 60 seconds since epoch - count all events
                // and calculate rate based on actual elapsed time
                if let Some((first_timestamp, _)) = history.first() {
                    let elapsed = now.duration_since(*first_timestamp).as_secs_f64();
                    if elapsed > 0.0 {
                        return history.len() as f64 / elapsed;
                    }
                }
                return 0.0;
            }
        };

        // Count events in last minute
        let recent_events = history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_minute_ago)
            .count();

        // Calculate rate (events per second)
        recent_events as f64 / 60.0
    }

    /// Update event rate metric
    pub async fn update_event_rate(&self) {
        let rate = self.calculate_event_rate().await;
        let mut metrics = self.metrics.write().await;
        metrics.events_per_second = rate;

        // Check for anomaly
        if rate > self.config.anomaly_threshold {
            tracing::error!(
                "Security: Anomaly detected - {} events/sec (threshold: {})",
                rate,
                self.config.anomaly_threshold
            );
        }
    }

    /// Get current metrics
    pub async fn metrics(&self) -> SecurityMetrics {
        // Update event rate before returning metrics
        self.update_event_rate().await;
        self.metrics.read().await.clone()
    }

    /// Get event count for specific IP and event type
    pub async fn get_ip_event_count(&self, ip: IpAddr, event_type: SecurityEventType) -> u32 {
        let ip_events = self.ip_events.read().await;

        if let Some(events) = ip_events.get(&ip) {
            events.iter().filter(|(_, t)| *t == event_type).count() as u32
        } else {
            0
        }
    }

    /// Clean up old event history
    pub async fn cleanup(&self) {
        let mut history = self.event_history.write().await;
        let mut ip_events = self.ip_events.write().await;

        let now = Instant::now();

        // Use checked_sub to avoid underflow on Windows or early system uptime
        let cutoff = match now.checked_sub(self.config.history_retention) {
            Some(c) => c,
            None => {
                // Retention period is longer than time since epoch - keep all events
                return;
            }
        };

        // Remove old events from history
        history.retain(|(timestamp, _)| *timestamp >= cutoff);

        // Remove old events from IP history
        for events in ip_events.values_mut() {
            events.retain(|(timestamp, _)| *timestamp >= cutoff);
        }
        // Remove IPs with no events
        ip_events.retain(|_, events| !events.is_empty());
    }

    /// Detect suspicious patterns for an IP
    pub async fn detect_suspicious_pattern(&self, ip: IpAddr) -> bool {
        let ip_events = self.ip_events.read().await;

        if let Some(events) = ip_events.get(&ip) {
            // Pattern 1: High rate of handshake failures (> 5 in history)
            let handshake_fails = events
                .iter()
                .filter(|(_, t)| *t == SecurityEventType::HandshakeFailed)
                .count();
            if handshake_fails > 5 {
                return true;
            }

            // Pattern 2: Multiple different attack types
            let mut unique_types = std::collections::HashSet::new();
            for (_, t) in events {
                unique_types.insert(t);
            }
            if unique_types.len() >= 3 {
                return true;
            }

            // Pattern 3: Repeated rate limit violations
            let rate_limits = events
                .iter()
                .filter(|(_, t)| *t == SecurityEventType::RateLimitExceeded)
                .count();
            if rate_limits > 10 {
                return true;
            }
        }

        false
    }

    /// Get total events tracked
    pub async fn total_events(&self) -> u64 {
        let metrics = self.metrics.read().await;
        metrics.handshake_failures
            + metrics.rate_limit_violations
            + metrics.temp_bans
            + metrics.perm_bans
            + metrics.invalid_packets
            + metrics.replay_attacks
            + metrics.connection_limit_hits
            + metrics.bandwidth_limit_hits
            + metrics.nonce_overflows
            + metrics.suspicious_patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_event() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        let event = SecurityEvent::new(SecurityEventType::HandshakeFailed, ip)
            .with_message("Invalid public key");

        monitor.record_event(event).await;

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.handshake_failures, 1);
        assert_eq!(monitor.total_events().await, 1);
    }

    #[tokio::test]
    async fn test_event_rate_calculation() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Record multiple events
        for _ in 0..10 {
            let event = SecurityEvent::new(SecurityEventType::RateLimitExceeded, ip);
            monitor.record_event(event).await;
        }

        let rate = monitor.calculate_event_rate().await;
        assert!(rate > 0.0);
    }

    #[tokio::test]
    async fn test_ip_event_count() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "172.16.0.1".parse().unwrap();

        // Record specific events
        for _ in 0..3 {
            let event = SecurityEvent::new(SecurityEventType::HandshakeFailed, ip);
            monitor.record_event(event).await;
        }

        let count = monitor
            .get_ip_event_count(ip, SecurityEventType::HandshakeFailed)
            .await;
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_suspicious_pattern_detection() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "203.0.113.1".parse().unwrap();

        // Not suspicious initially
        assert!(!monitor.detect_suspicious_pattern(ip).await);

        // Record many handshake failures
        for _ in 0..6 {
            let event = SecurityEvent::new(SecurityEventType::HandshakeFailed, ip);
            monitor.record_event(event).await;
        }

        // Should detect pattern
        assert!(monitor.detect_suspicious_pattern(ip).await);
    }

    #[tokio::test]
    async fn test_event_callback() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "198.51.100.1".parse().unwrap();

        // Set callback
        let called = Arc::new(RwLock::new(false));
        let called_clone = Arc::clone(&called);

        monitor
            .set_callback(Arc::new(move |_event| {
                let called = Arc::clone(&called_clone);
                tokio::spawn(async move {
                    let mut flag = called.write().await;
                    *flag = true;
                });
            }))
            .await;

        // Record event
        let event = SecurityEvent::new(SecurityEventType::IpTempBanned, ip);
        monitor.record_event(event).await;

        // Give callback time to execute
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Callback should have been called
        assert!(*called.read().await);
    }

    #[tokio::test]
    async fn test_metrics_all_types() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "192.0.2.1".parse().unwrap();

        // Record one of each type
        let event_types = vec![
            SecurityEventType::HandshakeFailed,
            SecurityEventType::RateLimitExceeded,
            SecurityEventType::IpTempBanned,
            SecurityEventType::IpPermBanned,
            SecurityEventType::InvalidPacket,
            SecurityEventType::ReplayDetected,
            SecurityEventType::ConnectionLimitExceeded,
            SecurityEventType::BandwidthLimitExceeded,
            SecurityEventType::NonceOverflow,
            SecurityEventType::SuspiciousPattern,
        ];

        for event_type in event_types {
            let event = SecurityEvent::new(event_type, ip);
            monitor.record_event(event).await;
        }

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.handshake_failures, 1);
        assert_eq!(metrics.rate_limit_violations, 1);
        assert_eq!(metrics.temp_bans, 1);
        assert_eq!(metrics.perm_bans, 1);
        assert_eq!(metrics.invalid_packets, 1);
        assert_eq!(metrics.replay_attacks, 1);
        assert_eq!(metrics.connection_limit_hits, 1);
        assert_eq!(metrics.bandwidth_limit_hits, 1);
        assert_eq!(metrics.nonce_overflows, 1);
        assert_eq!(metrics.suspicious_patterns, 1);
        assert_eq!(monitor.total_events().await, 10);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let monitor = SecurityMonitor::new(SecurityMonitorConfig::default());
        let ip: IpAddr = "198.18.0.1".parse().unwrap();

        // Record events
        for _ in 0..5 {
            let event = SecurityEvent::new(SecurityEventType::InvalidPacket, ip);
            monitor.record_event(event).await;
        }

        assert!(monitor.total_events().await > 0);

        // Cleanup (won't remove recent events)
        monitor.cleanup().await;
        assert!(monitor.total_events().await > 0);
    }
}
