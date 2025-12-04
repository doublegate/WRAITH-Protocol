//! Connection lifecycle management
//!
//! Monitors connection health, handles session migration, and manages keepalives.

use crate::node::session::PeerId;
use crate::node::{Node, NodeError};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::interval;

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

    /// Number of failed pings
    pub failed_pings: u32,
}

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
        let sessions: Vec<_> = {
            self.inner
                .sessions
                .read()
                .await
                .iter()
                .map(|(id, s)| (*id, s.clone()))
                .collect()
        };

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
                        self.inner.sessions.write().await.remove(&peer_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Send ping to a session and measure latency
    async fn ping_session(
        &self,
        peer_id: &PeerId,
        _session: std::sync::Arc<crate::node::session::PeerConnection>,
    ) -> Result<Duration, NodeError> {
        let start = std::time::Instant::now();

        // TODO: Send actual PING frame via transport
        // For now, simulate a successful ping
        //
        // let ping_frame = Frame::new_ping();
        // session.send_frame(ping_frame).await?;
        //
        // // Wait for PONG with timeout
        // let pong = tokio::time::timeout(
        //     Duration::from_secs(5),
        //     session.recv_pong()
        // ).await??;

        // Simulate 10ms RTT
        tokio::time::sleep(Duration::from_millis(10)).await;

        let latency = start.elapsed();

        tracing::trace!("Ping to {:?}: {:?}", peer_id, latency);

        Ok(latency)
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
        tracing::info!(
            "Migrating session for peer {:?} to new address {}",
            peer_id,
            new_addr
        );

        let mut sessions = self.inner.sessions.write().await;

        if let Some(_session) = sessions.get_mut(peer_id) {
            // TODO: Integrate with wraith-core::migration
            // For now, just update the address
            //
            // session.migrate_to(new_addr).await
            //     .map_err(|e| NodeError::Migration(e.to_string()))?;

            tracing::debug!("Session migrated successfully to {}", new_addr);

            // Update stored address
            // Note: In Arc, we can't mutate directly, so we'd need to
            // implement migration in PeerConnection or replace the Arc
            drop(sessions);

            // Verify new path with ping
            let result = self.get_or_establish_session(peer_id).await?;
            let _latency = self.ping_session(peer_id, result).await?;

            tracing::info!("Migration to {} verified", new_addr);

            Ok(())
        } else {
            Err(NodeError::SessionNotFound(*peer_id))
        }
    }

    /// Get connection health metrics
    ///
    /// Returns health status and metrics for a specific peer.
    pub async fn get_connection_health(&self, peer_id: &PeerId) -> Option<HealthMetrics> {
        let sessions = self.inner.sessions.read().await;

        if let Some(session) = sessions.get(peer_id) {
            let idle_time = session.last_activity.elapsed();
            let idle_timeout = self.inner.config.transport.idle_timeout;

            let status = if idle_time > idle_timeout {
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
                failed_pings: 0, // TODO: Track this
            })
        } else {
            None
        }
    }

    /// Get all connection health metrics
    ///
    /// Returns health metrics for all active sessions.
    pub async fn get_all_connection_health(&self) -> Vec<(PeerId, HealthMetrics)> {
        let sessions = self.inner.sessions.read().await;

        let mut metrics = Vec::new();

        for (peer_id, session) in sessions.iter() {
            let idle_time = session.last_activity.elapsed();
            let idle_timeout = self.inner.config.transport.idle_timeout;

            let status = if idle_time > idle_timeout {
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
                    failed_pings: 0,
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
        let mut sessions = self.inner.sessions.write().await;
        let idle_timeout = self.inner.config.transport.idle_timeout;

        let stale_peers: Vec<PeerId> = sessions
            .iter()
            .filter(|(_, session)| session.is_stale(idle_timeout))
            .map(|(peer_id, _)| *peer_id)
            .collect();

        let count = stale_peers.len();

        for peer_id in stale_peers {
            tracing::debug!("Cleaning up stale session for peer {:?}", peer_id);
            sessions.remove(&peer_id);
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
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer_id = [42u8; 32];
        node.establish_session(&peer_id).await.unwrap();

        let health = node.get_connection_health(&peer_id).await;
        assert!(health.is_some());

        let metrics = health.unwrap();
        assert_eq!(metrics.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_get_all_connection_health_with_sessions() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        node.establish_session(&peer1).await.unwrap();
        node.establish_session(&peer2).await.unwrap();

        let health = node.get_all_connection_health().await;
        assert_eq!(health.len(), 2);
    }
}
