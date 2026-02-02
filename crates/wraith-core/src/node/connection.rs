//! Connection lifecycle management
//!
//! Monitors connection health, handles session migration, and manages keepalives.

use crate::node::session::PeerId;
use crate::node::{Node, NodeError};
use std::borrow::Cow;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::interval;
use wraith_transport::transport::Transport;

/// Connection health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Connection is healthy
    Healthy,

    /// Connection is degraded (high latency/loss)
    Degraded,

    /// Connection is stale (no recent activity)
    Stale,

    /// Connection is dead (unresponsive)
    Dead,
}

/// Connection health metrics
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    /// Current status
    pub status: HealthStatus,

    /// Round-trip time (microseconds)
    pub rtt_us: Option<u64>,

    /// Packet loss rate (0.0 to 1.0)
    pub loss_rate: f64,

    /// Time since last activity
    pub idle_time: Duration,

    /// Number of consecutive failed pings
    pub failed_pings: u32,
}

/// Maximum consecutive failed pings before considering connection dead
const MAX_FAILED_PINGS: u32 = 3;

impl Node {
    /// Start the connection manager background task
    ///
    /// Monitors all active sessions and performs:
    /// - Health checks (ping/pong)
    /// - Stale session cleanup
    /// - DHT announcements
    ///
    /// Returns a join handle for the background task.
    pub fn start_connection_manager(&self) -> tokio::task::JoinHandle<()> {
        let node = self.clone();
        tokio::spawn(async move {
            node.connection_manager_loop().await;
        })
    }

    /// Main connection manager event loop
    async fn connection_manager_loop(&self) {
        let health_check_interval = Duration::from_secs(30);
        let announce_interval = self.inner.config.discovery.announcement_interval;

        let mut health_timer = interval(health_check_interval);
        let mut announce_timer = interval(announce_interval);

        tracing::info!("Connection manager started");

        loop {
            tokio::select! {
                _ = health_timer.tick() => {
                    if let Err(e) = self.health_check_all_sessions().await {
                        tracing::warn!("Health check failed: {}", e);
                    }
                }
                _ = announce_timer.tick() => {
                    if let Err(e) = self.announce().await {
                        tracing::warn!("DHT announcement failed: {}", e);
                    }
                }
            }
        }
    }

    /// Health check all active sessions
    async fn health_check_all_sessions(&self) -> Result<(), NodeError> {
        let sessions: Vec<_> = self
            .inner
            .sessions
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();

        tracing::trace!("Health checking {} sessions", sessions.len());

        let idle_timeout = self.inner.config.transport.idle_timeout;

        for (peer_id, session) in sessions {
            if session.is_stale(idle_timeout) {
                // Send ping to check if connection is alive
                match self.ping_session(&peer_id, session).await {
                    Ok(latency) => {
                        tracing::trace!("Ping to {:?}: {} µs", peer_id, latency.as_micros());
                    }
                    Err(e) => {
                        // Connection is dead, remove it
                        tracing::info!("Removing dead session for peer {:?}: {}", peer_id, e);
                        self.inner.sessions.remove(&peer_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Send ping to a session and measure latency
    ///
    /// Sends a PING frame and waits for the corresponding PONG response.
    /// Uses the `pending_pings` map to coordinate with `handle_pong_frame()`.
    async fn ping_session(
        &self,
        peer_id: &PeerId,
        session: std::sync::Arc<crate::node::session::PeerConnection>,
    ) -> Result<Duration, NodeError> {
        use crate::frame::{FrameBuilder, FrameType};

        let start = std::time::Instant::now();

        // Build PING frame with a unique sequence number for matching PONG
        let sequence = (start.elapsed().as_nanos() & 0xFFFFFFFF) as u32;

        let frame = FrameBuilder::new()
            .frame_type(FrameType::Ping)
            .stream_id(0) // Connection-level (stream 0)
            .sequence(sequence)
            .build(128) // Minimum size with padding
            .map_err(|e| NodeError::Other(format!("Failed to build PING frame: {e}").into()))?;

        // Create oneshot channel for PONG response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending ping before sending (so handle_pong_frame can find it)
        self.inner.pending_pings.insert((*peer_id, sequence), tx);

        // Encrypt frame
        let encrypted = session.encrypt_frame(&frame).await.inspect_err(|_| {
            self.inner.pending_pings.remove(&(*peer_id, sequence));
        })?;

        // Send via transport
        let transport_guard = self.inner.transport.lock().await;
        if let Some(transport) = transport_guard.as_ref() {
            transport
                .send_to(&encrypted, session.peer_addr())
                .await
                .map_err(|e| {
                    self.inner.pending_pings.remove(&(*peer_id, sequence));
                    NodeError::Transport(format!("Failed to send PING: {e}").into())
                })?;
        } else {
            self.inner.pending_pings.remove(&(*peer_id, sequence));
            return Err(NodeError::Transport("Transport not initialized".into()));
        }
        drop(transport_guard);

        // Wait for PONG response with timeout
        let ping_timeout = Duration::from_secs(5);
        match tokio::time::timeout(ping_timeout, rx).await {
            Ok(Ok(pong_time)) => {
                // PONG received - calculate RTT
                let latency = pong_time.duration_since(start);

                // Reset failed ping counter on successful PONG
                session.reset_failed_pings();
                session.touch(); // Update last activity

                tracing::trace!(
                    "PONG received from {:?}: {} µs RTT",
                    peer_id,
                    latency.as_micros()
                );

                Ok(latency)
            }
            Ok(Err(_)) => {
                // Channel closed (sender dropped without sending)
                self.inner.pending_pings.remove(&(*peer_id, sequence));
                session.increment_failed_pings();
                Err(NodeError::Other(
                    format!("PING to {peer_id:?} failed: channel closed").into(),
                ))
            }
            Err(_) => {
                // Timeout - no PONG received
                self.inner.pending_pings.remove(&(*peer_id, sequence));
                session.increment_failed_pings();

                tracing::debug!(
                    "PING to {:?} timed out after {:?} (failed pings: {})",
                    peer_id,
                    ping_timeout,
                    session.failed_ping_count()
                );

                Err(NodeError::Timeout(
                    format!("PING to {peer_id:?} timed out").into(),
                ))
            }
        }
    }

    /// Migrate a session to a new address
    ///
    /// Used when a peer's IP address changes (e.g., mobile network switch).
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer whose session to migrate
    /// * `new_addr` - The new address to migrate to
    ///
    /// # Errors
    ///
    /// Returns error if session not found or migration fails.
    pub async fn migrate_session(
        &self,
        peer_id: &PeerId,
        new_addr: SocketAddr,
    ) -> Result<(), NodeError> {
        use crate::frame::{FrameBuilder, FrameType};
        use crate::migration::PathValidator;
        use crate::node::node::MigrationState;

        tracing::info!(
            "Migrating session for peer {:?} to new address {}",
            peer_id,
            new_addr
        );

        // Get existing session
        let session = self
            .inner
            .sessions
            .get(peer_id)
            .ok_or(NodeError::SessionNotFound(*peer_id))?;
        let session = session.clone();

        // Create path validator
        let mut path_validator = PathValidator::new(Duration::from_secs(3));

        // Generate path ID from new address (simple hash)
        let path_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            new_addr.hash(&mut hasher);
            hasher.finish()
        };

        // Initiate PATH_CHALLENGE
        let challenge = path_validator.initiate_challenge(path_id);

        // Create channel for PATH_RESPONSE
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Register pending migration
        let migration_state = MigrationState {
            peer_id: *peer_id,
            new_addr,
            challenge,
            sender: response_tx,
            initiated_at: std::time::Instant::now(),
        };
        self.inner
            .pending_migrations
            .insert(path_id, migration_state);

        // Build PATH_CHALLENGE frame
        let frame = FrameBuilder::new()
            .frame_type(FrameType::PathChallenge)
            .stream_id(0) // Connection-level
            .sequence(0)
            .payload(&challenge)
            .build(128)
            .map_err(|e| {
                NodeError::Migration(format!("Failed to build PATH_CHALLENGE: {e}").into())
            })?;

        // Encrypt and send to new address
        let encrypted = session.encrypt_frame(&frame).await?;

        let transport_guard = self.inner.transport.lock().await;
        if let Some(transport) = transport_guard.as_ref() {
            transport.send_to(&encrypted, new_addr).await.map_err(|e| {
                self.inner.pending_migrations.remove(&path_id);
                NodeError::Migration(format!("Failed to send PATH_CHALLENGE: {e}").into())
            })?;
        } else {
            self.inner.pending_migrations.remove(&path_id);
            return Err(NodeError::Migration("Transport not initialized".into()));
        }
        drop(transport_guard);

        tracing::debug!(
            "PATH_CHALLENGE sent to {}, awaiting PATH_RESPONSE",
            new_addr
        );

        // Wait for PATH_RESPONSE with timeout
        let timeout = Duration::from_secs(5);
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(Ok(latency))) => {
                // Migration successful - update session peer address
                session.update_peer_addr(new_addr);
                session.touch(); // Update last activity

                tracing::info!(
                    "Migration to {} verified with {}µs RTT",
                    new_addr,
                    latency.as_micros()
                );
                Ok(())
            }
            Ok(Ok(Err(e))) => {
                tracing::error!("Migration validation failed: {}", e);
                Err(e)
            }
            Ok(Err(_)) => {
                self.inner.pending_migrations.remove(&path_id);
                Err(NodeError::Migration(Cow::Borrowed(
                    "Migration channel closed",
                )))
            }
            Err(_) => {
                self.inner.pending_migrations.remove(&path_id);
                Err(NodeError::Timeout(Cow::Borrowed("Migration timed out")))
            }
        }
    }

    /// Get connection health metrics
    ///
    /// Returns health status and metrics for a specific peer.
    pub async fn get_connection_health(&self, peer_id: &PeerId) -> Option<HealthMetrics> {
        if let Some(session) = self.inner.sessions.get(peer_id) {
            let idle_time_ms = session.idle_duration_ms();
            let idle_time = std::time::Duration::from_millis(idle_time_ms);
            let idle_timeout = self.inner.config.transport.idle_timeout;
            let failed_pings = session.failed_ping_count();

            let status = if failed_pings >= MAX_FAILED_PINGS || idle_time > idle_timeout {
                HealthStatus::Dead
            } else if idle_time > idle_timeout / 2 {
                HealthStatus::Stale
            } else if session.stats.loss_rate > 0.05 {
                // >5% loss
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };

            Some(HealthMetrics {
                status,
                rtt_us: session.stats.rtt_us,
                loss_rate: session.stats.loss_rate,
                idle_time,
                failed_pings,
            })
        } else {
            None
        }
    }

    /// Get all connection health metrics
    ///
    /// Returns health metrics for all active sessions.
    pub async fn get_all_connection_health(&self) -> Vec<(PeerId, HealthMetrics)> {
        let mut metrics = Vec::new();

        for entry in self.inner.sessions.iter() {
            let (peer_id, session) = entry.pair();
            let idle_time_ms = session.idle_duration_ms();
            let idle_time = std::time::Duration::from_millis(idle_time_ms);
            let idle_timeout = self.inner.config.transport.idle_timeout;
            let failed_pings = session.failed_ping_count();

            let status = if failed_pings >= MAX_FAILED_PINGS || idle_time > idle_timeout {
                HealthStatus::Dead
            } else if idle_time > idle_timeout / 2 {
                HealthStatus::Stale
            } else if session.stats.loss_rate > 0.05 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };

            metrics.push((
                *peer_id,
                HealthMetrics {
                    status,
                    rtt_us: session.stats.rtt_us,
                    loss_rate: session.stats.loss_rate,
                    idle_time,
                    failed_pings,
                },
            ));
        }

        metrics
    }

    /// Close all stale sessions
    ///
    /// Removes sessions that have exceeded the idle timeout.
    ///
    /// Returns the number of sessions closed.
    pub async fn cleanup_stale_sessions(&self) -> usize {
        let idle_timeout = self.inner.config.transport.idle_timeout;

        let stale_peers: Vec<PeerId> = self
            .inner
            .sessions
            .iter()
            .filter(|entry| entry.value().is_stale(idle_timeout))
            .map(|entry| *entry.key())
            .collect();

        let count = stale_peers.len();

        for peer_id in stale_peers {
            tracing::debug!("Cleaning up stale session for peer {:?}", peer_id);
            self.inner.sessions.remove(&peer_id);
        }

        if count > 0 {
            tracing::info!("Cleaned up {} stale sessions", count);
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
        assert_ne!(HealthStatus::Stale, HealthStatus::Dead);
    }

    #[test]
    fn test_health_metrics_creation() {
        let metrics = HealthMetrics {
            status: HealthStatus::Healthy,
            rtt_us: Some(1000),
            loss_rate: 0.01,
            idle_time: Duration::from_secs(10),
            failed_pings: 0,
        };

        assert_eq!(metrics.status, HealthStatus::Healthy);
        assert_eq!(metrics.rtt_us, Some(1000));
        assert_eq!(metrics.loss_rate, 0.01);
    }

    #[tokio::test]
    async fn test_get_connection_health_not_found() {
        let node = Node::new_random().await.unwrap();
        let peer_id = [42u8; 32];

        let health = node.get_connection_health(&peer_id).await;
        assert!(health.is_none());
    }

    #[tokio::test]
    async fn test_get_all_connection_health_empty() {
        let node = Node::new_random().await.unwrap();
        let health = node.get_all_connection_health().await;

        assert_eq!(health.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_stale_sessions_empty() {
        let node = Node::new_random().await.unwrap();
        let count = node.cleanup_stale_sessions().await;

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_migrate_session_not_found() {
        let node = Node::new_random().await.unwrap();
        let peer_id = [42u8; 32];
        let new_addr = "192.168.1.100:8420".parse().unwrap();

        let result = node.migrate_session(&peer_id, new_addr).await;
        assert!(result.is_err());

        match result {
            Err(NodeError::SessionNotFound(id)) => {
                assert_eq!(id, peer_id);
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_health_check_all_sessions_empty() {
        let node = Node::new_random().await.unwrap();
        let result = node.health_check_all_sessions().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_connection_health_with_session() {
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        let node = Node::new_random().await.unwrap();

        // Create a mock session by directly inserting into sessions map
        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));
        node.inner.sessions.insert(peer_id, mock_session);

        // Test that health check finds the session
        let health = node.get_connection_health(&peer_id).await;
        assert!(health.is_some());

        let metrics = health.unwrap();
        assert_eq!(metrics.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_cleanup_stale_sessions_no_stale() {
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        let node = Node::new_random().await.unwrap();

        // Insert a fresh session (not stale)
        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));
        // touch() it to make it fresh
        mock_session.touch();
        node.inner.sessions.insert(peer_id, mock_session);

        let count = node.cleanup_stale_sessions().await;
        assert_eq!(count, 0);
        assert_eq!(node.inner.sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_get_connection_health_dead_from_failed_pings() {
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        let node = Node::new_random().await.unwrap();

        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));

        // Exceed MAX_FAILED_PINGS (3)
        mock_session.increment_failed_pings();
        mock_session.increment_failed_pings();
        mock_session.increment_failed_pings();

        node.inner.sessions.insert(peer_id, mock_session);

        let health = node.get_connection_health(&peer_id).await.unwrap();
        assert_eq!(health.status, HealthStatus::Dead);
        assert_eq!(health.failed_pings, 3);
    }

    #[tokio::test]
    async fn test_health_status_debug_clone() {
        let status = HealthStatus::Healthy;
        let cloned = status;
        assert_eq!(format!("{:?}", cloned), "Healthy");

        let metrics = HealthMetrics {
            status: HealthStatus::Degraded,
            rtt_us: Some(5000),
            loss_rate: 0.06,
            idle_time: Duration::from_secs(5),
            failed_pings: 1,
        };
        let cloned = metrics.clone();
        assert_eq!(cloned.status, HealthStatus::Degraded);
        assert_eq!(cloned.rtt_us, Some(5000));
        assert_eq!(cloned.failed_pings, 1);
    }

    #[tokio::test]
    async fn test_cleanup_stale_sessions_removes_stale() {
        use crate::node::config::{NodeConfig, TransportConfig};
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        // Create node with very short idle timeout so sessions appear stale
        let config = NodeConfig {
            transport: TransportConfig {
                idle_timeout: Duration::from_millis(1),
                ..Default::default()
            },
            ..Default::default()
        };
        let node = Node::new_with_config(config).await.unwrap();

        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));

        node.inner.sessions.insert(peer_id, mock_session);

        // Wait a bit so the session becomes stale
        tokio::time::sleep(Duration::from_millis(10)).await;

        let count = node.cleanup_stale_sessions().await;
        assert_eq!(count, 1);
        assert_eq!(node.inner.sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_get_connection_health_stale() {
        use crate::node::config::{NodeConfig, TransportConfig};
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        // Create node with very short idle timeout
        let config = NodeConfig {
            transport: TransportConfig {
                idle_timeout: Duration::from_millis(20),
                ..Default::default()
            },
            ..Default::default()
        };
        let node = Node::new_with_config(config).await.unwrap();

        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));
        node.inner.sessions.insert(peer_id, mock_session);

        // Wait for half the idle timeout to get Stale status
        tokio::time::sleep(Duration::from_millis(15)).await;

        let health = node.get_connection_health(&peer_id).await.unwrap();
        // Should be Stale (idle > half of timeout) or Dead (idle > timeout)
        assert!(
            health.status == HealthStatus::Stale || health.status == HealthStatus::Dead,
            "Expected Stale or Dead, got {:?}",
            health.status
        );
    }

    #[tokio::test]
    async fn test_get_connection_health_dead_from_idle() {
        use crate::node::config::{NodeConfig, TransportConfig};
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        let config = NodeConfig {
            transport: TransportConfig {
                idle_timeout: Duration::from_millis(5),
                ..Default::default()
            },
            ..Default::default()
        };
        let node = Node::new_with_config(config).await.unwrap();

        let peer_id = [42u8; 32];
        let mock_session = Arc::new(PeerConnection::new_for_test(
            peer_id,
            "127.0.0.1:8420".parse().unwrap(),
        ));
        node.inner.sessions.insert(peer_id, mock_session);

        tokio::time::sleep(Duration::from_millis(20)).await;

        let health = node.get_connection_health(&peer_id).await.unwrap();
        assert_eq!(health.status, HealthStatus::Dead);
    }

    #[tokio::test]
    async fn test_get_all_connection_health_with_sessions() {
        use crate::node::session::PeerConnection;
        use std::sync::Arc;

        let node = Node::new_random().await.unwrap();

        // Insert two mock sessions
        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        let session1 = Arc::new(PeerConnection::new_for_test(
            peer1,
            "127.0.0.1:8421".parse().unwrap(),
        ));
        let session2 = Arc::new(PeerConnection::new_for_test(
            peer2,
            "127.0.0.1:8422".parse().unwrap(),
        ));

        node.inner.sessions.insert(peer1, session1);
        node.inner.sessions.insert(peer2, session2);

        let health = node.get_all_connection_health().await;
        assert_eq!(health.len(), 2);
    }
}
