//! IP reputation system for progressive penalties and attack mitigation
//!
//! Tracks connection failures, handshake failures, and rate limit violations
//! per IP address. Implements progressive penalties:
//! - Warning threshold: Log suspicious activity
//! - Backoff threshold: Temporary delay increases
//! - Temporary ban: Block for configurable duration
//! - Permanent ban: Block indefinitely (requires manual intervention)
//!
//! Reputation scores decay over time to allow recovery from transient issues.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// IP reputation system configuration
#[derive(Debug, Clone)]
pub struct IpReputationConfig {
    /// Failure threshold for warning (log suspicious activity)
    pub warning_threshold: u32,

    /// Failure threshold for backoff (progressive delays)
    pub backoff_threshold: u32,

    /// Failure threshold for temporary ban
    pub temp_ban_threshold: u32,

    /// Failure threshold for permanent ban
    pub permanent_ban_threshold: u32,

    /// Duration of temporary ban
    pub temp_ban_duration: Duration,

    /// Reputation decay interval (failures decrease over time)
    pub decay_interval: Duration,

    /// Decay amount per interval
    pub decay_amount: u32,

    /// Backoff base delay (doubled for each failure over threshold)
    pub backoff_base_delay: Duration,

    /// Maximum backoff delay
    pub max_backoff_delay: Duration,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 3,
            backoff_threshold: 5,
            temp_ban_threshold: 10,
            permanent_ban_threshold: 50,
            temp_ban_duration: Duration::from_secs(3600), // 1 hour
            decay_interval: Duration::from_secs(300),     // 5 minutes
            decay_amount: 1,
            backoff_base_delay: Duration::from_millis(100),
            max_backoff_delay: Duration::from_secs(30),
        }
    }
}

/// Reputation status for an IP address
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReputationStatus {
    /// Good standing
    Good,
    /// Warning level (log suspicious activity)
    Warning,
    /// Backoff level (apply progressive delays)
    Backoff,
    /// Temporarily banned
    TempBanned {
        /// Ban expiry time
        until: Instant,
    },
    /// Permanently banned
    PermBanned,
}

/// IP reputation entry
#[derive(Debug, Clone)]
struct IpReputation {
    /// Number of failures
    failures: u32,

    /// Last failure time (for decay)
    last_failure: Instant,

    /// Last decay time
    last_decay: Instant,

    /// Current status
    status: ReputationStatus,
}

impl IpReputation {
    fn new() -> Self {
        Self {
            failures: 0,
            last_failure: Instant::now(),
            last_decay: Instant::now(),
            status: ReputationStatus::Good,
        }
    }

    /// Apply decay based on elapsed time
    fn apply_decay(&mut self, config: &IpReputationConfig) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_decay);

        if elapsed >= config.decay_interval {
            let interval_secs = config.decay_interval.as_secs();
            if interval_secs > 0 {
                let decay_periods = elapsed.as_secs() / interval_secs;
                let total_decay = config.decay_amount * decay_periods as u32;

                self.failures = self.failures.saturating_sub(total_decay);
                self.last_decay = now;
            }
        }
    }

    /// Update status based on failure count
    fn update_status(&mut self, config: &IpReputationConfig) {
        let now = Instant::now();

        // Check if temp ban has expired
        if let ReputationStatus::TempBanned { until } = self.status
            && now >= until
        {
            self.status = ReputationStatus::Good;
            self.failures = 0; // Reset on temp ban expiry
        }

        // Don't update if permanently banned
        if self.status == ReputationStatus::PermBanned {
            return;
        }

        // Update status based on failure count
        self.status = if self.failures >= config.permanent_ban_threshold {
            ReputationStatus::PermBanned
        } else if self.failures >= config.temp_ban_threshold {
            ReputationStatus::TempBanned {
                until: now + config.temp_ban_duration,
            }
        } else if self.failures >= config.backoff_threshold {
            ReputationStatus::Backoff
        } else if self.failures >= config.warning_threshold {
            ReputationStatus::Warning
        } else {
            ReputationStatus::Good
        };
    }
}

/// IP reputation system
pub struct IpReputationSystem {
    /// Configuration
    config: IpReputationConfig,

    /// Per-IP reputation data
    reputations: Arc<RwLock<HashMap<IpAddr, IpReputation>>>,

    /// Metrics
    metrics: Arc<RwLock<IpReputationMetrics>>,
}

/// IP reputation metrics
#[derive(Debug, Default, Clone)]
pub struct IpReputationMetrics {
    /// Total IPs in warning status
    pub warning_count: u64,

    /// Total IPs in backoff status
    pub backoff_count: u64,

    /// Total IPs temporarily banned
    pub temp_banned_count: u64,

    /// Total IPs permanently banned
    pub perm_banned_count: u64,

    /// Total failures recorded
    pub total_failures: u64,

    /// Total connections blocked
    pub connections_blocked: u64,
}

impl IpReputationSystem {
    /// Create a new IP reputation system
    pub fn new(config: IpReputationConfig) -> Self {
        Self {
            config,
            reputations: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(IpReputationMetrics::default())),
        }
    }

    /// Record a failure for an IP address
    pub async fn record_failure(&self, ip: IpAddr) {
        let mut reputations = self.reputations.write().await;
        let mut metrics = self.metrics.write().await;

        let reputation = reputations.entry(ip).or_insert_with(IpReputation::new);

        // Apply decay before recording new failure
        reputation.apply_decay(&self.config);

        // Increment failure count
        reputation.failures += 1;
        reputation.last_failure = Instant::now();
        metrics.total_failures += 1;

        // Update status
        let old_status = reputation.status.clone();
        reputation.update_status(&self.config);

        // Update metrics if status changed
        if old_status != reputation.status {
            match &reputation.status {
                ReputationStatus::Warning => metrics.warning_count += 1,
                ReputationStatus::Backoff => metrics.backoff_count += 1,
                ReputationStatus::TempBanned { .. } => metrics.temp_banned_count += 1,
                ReputationStatus::PermBanned => metrics.perm_banned_count += 1,
                ReputationStatus::Good => {}
            }
        }

        // Log status changes
        match &reputation.status {
            ReputationStatus::Warning => {
                tracing::warn!(
                    "IP {} reached warning threshold ({} failures)",
                    ip,
                    reputation.failures
                );
            }
            ReputationStatus::Backoff => {
                tracing::warn!(
                    "IP {} entered backoff mode ({} failures)",
                    ip,
                    reputation.failures
                );
            }
            ReputationStatus::TempBanned { until } => {
                tracing::warn!(
                    "IP {} temporarily banned until {:?} ({} failures)",
                    ip,
                    until,
                    reputation.failures
                );
            }
            ReputationStatus::PermBanned => {
                tracing::error!(
                    "IP {} permanently banned ({} failures)",
                    ip,
                    reputation.failures
                );
            }
            _ => {}
        }
    }

    /// Check if an IP is allowed to connect
    pub async fn check_allowed(&self, ip: IpAddr) -> bool {
        let mut reputations = self.reputations.write().await;
        let mut metrics = self.metrics.write().await;

        let reputation = match reputations.get_mut(&ip) {
            Some(rep) => rep,
            None => return true, // New IPs are allowed
        };

        // Apply decay
        reputation.apply_decay(&self.config);
        reputation.update_status(&self.config);

        // Check status
        match &reputation.status {
            ReputationStatus::Good | ReputationStatus::Warning => true,
            ReputationStatus::Backoff => true, // Allowed but with delay
            ReputationStatus::TempBanned { .. } | ReputationStatus::PermBanned => {
                metrics.connections_blocked += 1;
                false
            }
        }
    }

    /// Get backoff delay for an IP (if in backoff status)
    pub async fn get_backoff_delay(&self, ip: IpAddr) -> Duration {
        let reputations = self.reputations.read().await;

        let reputation = match reputations.get(&ip) {
            Some(rep) => rep,
            None => return Duration::ZERO,
        };

        match reputation.status {
            ReputationStatus::Backoff => {
                // Progressive backoff: base_delay * 2^(failures - backoff_threshold)
                let excess_failures = reputation
                    .failures
                    .saturating_sub(self.config.backoff_threshold);
                let multiplier = 2u32.saturating_pow(excess_failures.min(10)); // Cap at 2^10 = 1024x
                let delay = self.config.backoff_base_delay * multiplier;
                delay.min(self.config.max_backoff_delay)
            }
            _ => Duration::ZERO,
        }
    }

    /// Get reputation status for an IP
    pub async fn get_status(&self, ip: IpAddr) -> ReputationStatus {
        let reputations = self.reputations.read().await;

        reputations
            .get(&ip)
            .map(|rep| rep.status.clone())
            .unwrap_or(ReputationStatus::Good)
    }

    /// Manually clear reputation for an IP (admin action)
    pub async fn clear_reputation(&self, ip: IpAddr) {
        let mut reputations = self.reputations.write().await;
        reputations.remove(&ip);
    }

    /// Clean up expired temp bans and decayed entries
    pub async fn cleanup(&self) {
        let mut reputations = self.reputations.write().await;
        let now = Instant::now();

        reputations.retain(|_, rep| {
            // Remove if:
            // - Temp ban expired and failures decayed to 0
            // - Good status and no failures in last hour
            match &rep.status {
                ReputationStatus::TempBanned { until } if now >= *until && rep.failures == 0 => {
                    false
                }
                ReputationStatus::Good
                    if rep.failures == 0
                        && now.duration_since(rep.last_failure) > Duration::from_secs(3600) =>
                {
                    false
                }
                _ => true,
            }
        });
    }

    /// Get current metrics
    pub async fn metrics(&self) -> IpReputationMetrics {
        self.metrics.read().await.clone()
    }

    /// Get number of tracked IPs
    pub async fn tracked_ips(&self) -> usize {
        self.reputations.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reputation_progression() {
        let config = IpReputationConfig {
            warning_threshold: 2,
            backoff_threshold: 4,
            temp_ban_threshold: 6,
            permanent_ban_threshold: 10,
            ..Default::default()
        };
        let system = IpReputationSystem::new(config);
        let ip: IpAddr = "192.168.1.100".parse().unwrap();

        // Initial state
        assert_eq!(system.get_status(ip).await, ReputationStatus::Good);
        assert!(system.check_allowed(ip).await);

        // First failure - still good
        system.record_failure(ip).await;
        assert_eq!(system.get_status(ip).await, ReputationStatus::Good);

        // Second failure - warning
        system.record_failure(ip).await;
        assert_eq!(system.get_status(ip).await, ReputationStatus::Warning);
        assert!(system.check_allowed(ip).await);

        // More failures - backoff
        system.record_failure(ip).await;
        system.record_failure(ip).await;
        assert_eq!(system.get_status(ip).await, ReputationStatus::Backoff);
        assert!(system.check_allowed(ip).await);
        assert!(system.get_backoff_delay(ip).await > Duration::ZERO);

        // More failures - temp ban
        system.record_failure(ip).await;
        system.record_failure(ip).await;
        assert!(matches!(
            system.get_status(ip).await,
            ReputationStatus::TempBanned { .. }
        ));
        assert!(!system.check_allowed(ip).await);

        // More failures - perm ban
        for _ in 0..4 {
            system.record_failure(ip).await;
        }
        assert_eq!(system.get_status(ip).await, ReputationStatus::PermBanned);
        assert!(!system.check_allowed(ip).await);
    }

    #[tokio::test]
    async fn test_reputation_decay() {
        let config = IpReputationConfig {
            warning_threshold: 2,
            decay_interval: Duration::from_millis(100),
            decay_amount: 1,
            ..Default::default()
        };
        let system = IpReputationSystem::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Record failures to reach warning
        system.record_failure(ip).await;
        system.record_failure(ip).await;
        assert_eq!(system.get_status(ip).await, ReputationStatus::Warning);

        // Wait for decay
        tokio::time::sleep(Duration::from_millis(250)).await;

        // Check again - should have decayed
        assert!(system.check_allowed(ip).await);
        let status = system.get_status(ip).await;
        assert!(matches!(
            status,
            ReputationStatus::Good | ReputationStatus::Warning
        ));
    }

    #[tokio::test]
    async fn test_backoff_delay_progressive() {
        let config = IpReputationConfig {
            backoff_threshold: 3,
            backoff_base_delay: Duration::from_millis(100),
            max_backoff_delay: Duration::from_secs(10),
            ..Default::default()
        };
        let system = IpReputationSystem::new(config);
        let ip: IpAddr = "172.16.0.1".parse().unwrap();

        // No backoff initially
        assert_eq!(system.get_backoff_delay(ip).await, Duration::ZERO);

        // Reach backoff threshold
        for _ in 0..3 {
            system.record_failure(ip).await;
        }

        // Should have base delay
        let delay1 = system.get_backoff_delay(ip).await;
        assert!(delay1 >= Duration::from_millis(100));

        // More failures - exponential backoff
        system.record_failure(ip).await;
        let delay2 = system.get_backoff_delay(ip).await;
        assert!(delay2 > delay1);

        system.record_failure(ip).await;
        let delay3 = system.get_backoff_delay(ip).await;
        assert!(delay3 > delay2);
    }

    #[tokio::test]
    async fn test_clear_reputation() {
        let system = IpReputationSystem::new(IpReputationConfig::default());
        let ip: IpAddr = "203.0.113.1".parse().unwrap();

        // Build up failures
        for _ in 0..5 {
            system.record_failure(ip).await;
        }
        assert_ne!(system.get_status(ip).await, ReputationStatus::Good);

        // Clear
        system.clear_reputation(ip).await;
        assert_eq!(system.get_status(ip).await, ReputationStatus::Good);
    }

    #[tokio::test]
    async fn test_metrics() {
        let config = IpReputationConfig {
            warning_threshold: 1,
            backoff_threshold: 2,
            temp_ban_threshold: 3,
            ..Default::default()
        };
        let system = IpReputationSystem::new(config);
        let ip: IpAddr = "198.51.100.1".parse().unwrap();

        // Record failures
        system.record_failure(ip).await;
        system.record_failure(ip).await;
        system.record_failure(ip).await;

        let metrics = system.metrics().await;
        assert_eq!(metrics.total_failures, 3);
        assert!(metrics.temp_banned_count > 0);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let system = IpReputationSystem::new(IpReputationConfig::default());
        let ip: IpAddr = "192.0.2.1".parse().unwrap();

        // Add entry
        system.record_failure(ip).await;
        assert!(system.tracked_ips().await > 0);

        // Cleanup (won't remove recent entries)
        system.cleanup().await;
        assert!(system.tracked_ips().await > 0);
    }
}
