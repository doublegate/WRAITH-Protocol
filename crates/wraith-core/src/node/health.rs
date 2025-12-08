//! Health monitoring and graceful degradation
//!
//! Monitors system resources and triggers graceful degradation when thresholds
//! are exceeded to prevent out-of-memory conditions and maintain stability.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Normal operation - all systems healthy
    Healthy,

    /// Reduced capacity - resource usage elevated (>75%)
    /// New connections accepted but rate limited more aggressively
    Degraded,

    /// Emergency mode - resource usage critical (>90%)
    /// New connections rejected, idle sessions closed
    Critical,
}

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Memory threshold for degraded state (percentage, 0.0-1.0)
    pub degraded_memory_threshold: f64,

    /// Memory threshold for critical state (percentage, 0.0-1.0)
    pub critical_memory_threshold: f64,

    /// Session count threshold for degraded state
    pub degraded_session_threshold: usize,

    /// Session count threshold for critical state
    pub critical_session_threshold: usize,

    /// Health check interval
    pub check_interval: Duration,

    /// Minimum time between state transitions
    pub transition_cooldown: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            degraded_memory_threshold: 0.75, // 75%
            critical_memory_threshold: 0.90, // 90%
            degraded_session_threshold: 800,
            critical_session_threshold: 950,
            check_interval: Duration::from_secs(5),
            transition_cooldown: Duration::from_secs(10),
        }
    }
}

/// Health metrics snapshot
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    /// Current health status
    pub status: HealthStatus,

    /// Memory usage percentage (0.0-1.0)
    pub memory_usage: f64,

    /// Total system memory in bytes
    pub total_memory: u64,

    /// Used system memory in bytes
    pub used_memory: u64,

    /// Current session count
    pub session_count: usize,

    /// Current transfer count
    pub transfer_count: usize,

    /// Last status transition time
    pub last_transition: Instant,

    /// Times degraded state was entered
    pub degraded_count: u64,

    /// Times critical state was entered
    pub critical_count: u64,

    /// Times recovered to healthy
    pub recovery_count: u64,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            status: HealthStatus::Healthy,
            memory_usage: 0.0,
            total_memory: 0,
            used_memory: 0,
            session_count: 0,
            transfer_count: 0,
            last_transition: Instant::now(),
            degraded_count: 0,
            critical_count: 0,
            recovery_count: 0,
        }
    }
}

/// Health monitor
pub struct HealthMonitor {
    /// Configuration
    config: HealthConfig,

    /// Current metrics
    metrics: Arc<RwLock<HealthMetrics>>,

    /// System information provider
    system_info: Arc<RwLock<SystemInfo>>,
}

/// System information (abstracted for testability)
#[derive(Debug)]
struct SystemInfo {
    /// Total system memory (cached)
    total_memory: u64,
}

impl SystemInfo {
    /// Create new system info
    fn new() -> Self {
        #[cfg(target_os = "linux")]
        let total_memory = Self::get_total_memory_linux();

        #[cfg(not(target_os = "linux"))]
        let total_memory = Self::get_total_memory_fallback();

        Self { total_memory }
    }

    /// Get total system memory on Linux
    #[cfg(target_os = "linux")]
    fn get_total_memory_linux() -> u64 {
        use std::fs;

        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }

        Self::get_total_memory_fallback()
    }

    /// Fallback memory detection
    fn get_total_memory_fallback() -> u64 {
        // Assume 8 GB if we can't detect
        8 * 1024 * 1024 * 1024
    }

    /// Get current memory usage
    fn get_memory_usage(&self) -> (u64, u64) {
        #[cfg(target_os = "linux")]
        {
            if let Some((used, total)) = Self::get_memory_usage_linux() {
                return (used, total);
            }
        }

        // Fallback: assume 50% usage
        (self.total_memory / 2, self.total_memory)
    }

    /// Get memory usage on Linux
    #[cfg(target_os = "linux")]
    fn get_memory_usage_linux() -> Option<(u64, u64)> {
        use std::fs;

        let meminfo = match fs::read_to_string("/proc/meminfo") {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to read /proc/meminfo: {}", e);
                return None;
            }
        };
        let mut mem_total = 0u64;
        let mut mem_available = 0u64;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    match kb_str.parse::<u64>() {
                        Ok(kb) => mem_total = kb * 1024,
                        Err(e) => {
                            warn!("Failed to parse MemTotal '{}': {}", kb_str, e);
                            return None;
                        }
                    }
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    match kb_str.parse::<u64>() {
                        Ok(kb) => mem_available = kb * 1024,
                        Err(e) => {
                            warn!("Failed to parse MemAvailable '{}': {}", kb_str, e);
                            return None;
                        }
                    }
                }
            }
        }

        if mem_total > 0 && mem_available > 0 {
            let used = mem_total.saturating_sub(mem_available);
            Some((used, mem_total))
        } else {
            warn!(
                "Memory info incomplete: mem_total={}, mem_available={}",
                mem_total, mem_available
            );
            None
        }
    }
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            system_info: Arc::new(RwLock::new(SystemInfo::new())),
        }
    }

    /// Update health status based on current metrics
    pub async fn update(&self, session_count: usize, transfer_count: usize) {
        let system_info = self.system_info.read().await;
        let (used_memory, total_memory) = system_info.get_memory_usage();
        drop(system_info);

        let memory_usage = used_memory as f64 / total_memory as f64;

        let mut metrics = self.metrics.write().await;

        // Determine new status
        let new_status = self.determine_status(memory_usage, session_count);

        // Check transition cooldown
        let now = Instant::now();
        let since_transition = now.duration_since(metrics.last_transition);

        if new_status != metrics.status && since_transition >= self.config.transition_cooldown {
            // State transition
            let old_status = metrics.status;
            match new_status {
                HealthStatus::Healthy => {
                    info!(
                        "Health status recovered: {:?} -> {:?} (memory: {:.1}%, sessions: {})",
                        old_status,
                        new_status,
                        memory_usage * 100.0,
                        session_count
                    );
                    metrics.recovery_count += 1;
                }
                HealthStatus::Degraded => {
                    warn!(
                        "Health status degraded: {:?} -> {:?} (memory: {:.1}%, sessions: {})",
                        old_status,
                        new_status,
                        memory_usage * 100.0,
                        session_count
                    );
                    metrics.degraded_count += 1;
                }
                HealthStatus::Critical => {
                    warn!(
                        "Health status CRITICAL: {:?} -> {:?} (memory: {:.1}%, sessions: {})",
                        old_status,
                        new_status,
                        memory_usage * 100.0,
                        session_count
                    );
                    metrics.critical_count += 1;
                }
            }

            metrics.status = new_status;
            metrics.last_transition = now;
        } else {
            debug!(
                "Health check: status={:?}, memory={:.1}%, sessions={}",
                metrics.status,
                memory_usage * 100.0,
                session_count
            );
        }

        // Update metrics
        metrics.memory_usage = memory_usage;
        metrics.total_memory = total_memory;
        metrics.used_memory = used_memory;
        metrics.session_count = session_count;
        metrics.transfer_count = transfer_count;
    }

    /// Determine health status from metrics
    fn determine_status(&self, memory_usage: f64, session_count: usize) -> HealthStatus {
        // Critical conditions
        if memory_usage >= self.config.critical_memory_threshold
            || session_count >= self.config.critical_session_threshold
        {
            return HealthStatus::Critical;
        }

        // Degraded conditions
        if memory_usage >= self.config.degraded_memory_threshold
            || session_count >= self.config.degraded_session_threshold
        {
            return HealthStatus::Degraded;
        }

        HealthStatus::Healthy
    }

    /// Get current health status
    pub async fn status(&self) -> HealthStatus {
        self.metrics.read().await.status
    }

    /// Get current health metrics
    pub async fn metrics(&self) -> HealthMetrics {
        self.metrics.read().await.clone()
    }

    /// Check if new connections should be accepted
    pub async fn should_accept_connection(&self) -> bool {
        let status = self.status().await;
        matches!(status, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Check if new transfers should be accepted
    pub async fn should_accept_transfer(&self) -> bool {
        let status = self.status().await;
        status == HealthStatus::Healthy
    }

    /// Check if emergency cleanup is needed
    pub async fn needs_emergency_cleanup(&self) -> bool {
        self.status().await == HealthStatus::Critical
    }

    /// Get recommended action based on current health
    pub async fn recommended_action(&self) -> HealthAction {
        match self.status().await {
            HealthStatus::Healthy => HealthAction::None,
            HealthStatus::Degraded => HealthAction::ReduceLoad,
            HealthStatus::Critical => HealthAction::EmergencyCleanup,
        }
    }
}

/// Recommended health action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthAction {
    /// No action needed
    None,

    /// Reduce load (reject new connections, pause transfers)
    ReduceLoad,

    /// Emergency cleanup (close idle sessions, cancel transfers)
    EmergencyCleanup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);

        let status = monitor.status().await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_status_determination() {
        let config = HealthConfig {
            degraded_memory_threshold: 0.75,
            critical_memory_threshold: 0.90,
            degraded_session_threshold: 800,
            critical_session_threshold: 950,
            ..Default::default()
        };
        let monitor = HealthMonitor::new(config.clone());

        // Healthy
        assert_eq!(monitor.determine_status(0.5, 100), HealthStatus::Healthy);

        // Degraded by memory
        assert_eq!(monitor.determine_status(0.80, 100), HealthStatus::Degraded);

        // Degraded by sessions
        assert_eq!(monitor.determine_status(0.5, 850), HealthStatus::Degraded);

        // Critical by memory
        assert_eq!(monitor.determine_status(0.95, 100), HealthStatus::Critical);

        // Critical by sessions
        assert_eq!(monitor.determine_status(0.5, 960), HealthStatus::Critical);
    }

    #[tokio::test]
    async fn test_health_monitor_update() {
        let config = HealthConfig {
            transition_cooldown: Duration::from_millis(10),
            ..Default::default()
        };
        let monitor = HealthMonitor::new(config);

        // Update with healthy metrics
        monitor.update(100, 5).await;
        let metrics = monitor.metrics().await;

        assert_eq!(metrics.session_count, 100);
        assert_eq!(metrics.transfer_count, 5);
    }

    #[tokio::test]
    async fn test_health_monitor_state_transitions() {
        let config = HealthConfig {
            degraded_session_threshold: 500,
            critical_session_threshold: 900,
            transition_cooldown: Duration::from_millis(10),
            ..Default::default()
        };
        let monitor = HealthMonitor::new(config);

        // Start healthy
        monitor.update(100, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Healthy);

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Transition to degraded
        monitor.update(600, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Degraded);

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.degraded_count, 1);

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Transition to critical
        monitor.update(950, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Critical);

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.critical_count, 1);

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Recover to healthy
        monitor.update(100, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Healthy);

        let metrics = monitor.metrics().await;
        assert_eq!(metrics.recovery_count, 1);
    }

    #[tokio::test]
    async fn test_health_monitor_transition_cooldown() {
        let config = HealthConfig {
            degraded_session_threshold: 500,
            transition_cooldown: Duration::from_secs(1),
            ..Default::default()
        };
        let monitor = HealthMonitor::new(config);

        // Start healthy
        monitor.update(100, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Healthy);

        // Try to transition immediately (should be blocked by cooldown)
        monitor.update(600, 5).await;
        assert_eq!(monitor.status().await, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_monitor_accept_connection() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);

        // Healthy - should accept
        assert!(monitor.should_accept_connection().await);

        // Set to degraded manually
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Degraded;
        }
        assert!(monitor.should_accept_connection().await);

        // Set to critical manually
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Critical;
        }
        assert!(!monitor.should_accept_connection().await);
    }

    #[tokio::test]
    async fn test_health_monitor_accept_transfer() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);

        // Healthy - should accept
        assert!(monitor.should_accept_transfer().await);

        // Degraded - should not accept
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Degraded;
        }
        assert!(!monitor.should_accept_transfer().await);

        // Critical - should not accept
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Critical;
        }
        assert!(!monitor.should_accept_transfer().await);
    }

    #[tokio::test]
    async fn test_health_monitor_recommended_action() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);

        // Healthy
        assert_eq!(monitor.recommended_action().await, HealthAction::None);

        // Degraded
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Degraded;
        }
        assert_eq!(monitor.recommended_action().await, HealthAction::ReduceLoad);

        // Critical
        {
            let mut metrics = monitor.metrics.write().await;
            metrics.status = HealthStatus::Critical;
        }
        assert_eq!(
            monitor.recommended_action().await,
            HealthAction::EmergencyCleanup
        );
    }
}
