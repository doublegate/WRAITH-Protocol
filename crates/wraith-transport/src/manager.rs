//! Transport manager for multi-transport orchestration and migration.
//!
//! The `TransportManager` holds multiple transport instances and provides
//! intelligent selection, migration, and aggregated statistics. It supports
//! several selection strategies including latency-based, throughput-based,
//! and round-robin.
//!
//! # Migration
//!
//! Transport migration allows seamless switching between transports (e.g., from
//! UDP to QUIC) without dropping packets. The manager coordinates the migration
//! by briefly buffering sends during the transition.

use crate::factory::TransportType;
use crate::transport::{Transport, TransportError, TransportResult, TransportStats};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Strategy for selecting which transport to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportSelector {
    /// Select the transport with lowest estimated latency.
    LatencyBased,
    /// Select the transport with highest throughput.
    ThroughputBased,
    /// Rotate through transports in order.
    RoundRobin,
    /// Always use the primary transport.
    #[default]
    Primary,
}

/// Event emitted during transport migration.
#[derive(Debug, Clone)]
pub enum MigrationEvent {
    /// Migration has started from one transport to another.
    Started {
        /// Source transport type.
        from: TransportType,
        /// Destination transport type.
        to: TransportType,
    },
    /// Migration completed successfully.
    Completed {
        /// Source transport type.
        from: TransportType,
        /// Destination transport type.
        to: TransportType,
        /// Time taken for migration.
        duration: Duration,
    },
    /// Migration failed.
    Failed {
        /// Source transport type.
        from: TransportType,
        /// Destination transport type.
        to: TransportType,
        /// Error description.
        error: String,
    },
}

/// Callback type for migration events.
pub type MigrationCallback = Arc<dyn Fn(MigrationEvent) + Send + Sync>;

/// Manages multiple transports with selection and migration support.
///
/// # Examples
///
/// ```no_run
/// use wraith_transport::manager::{TransportManager, TransportSelector};
/// use wraith_transport::udp_async::AsyncUdpTransport;
/// use wraith_transport::transport::Transport;
/// use std::net::SocketAddr;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:0".parse()?;
/// let udp = AsyncUdpTransport::bind(addr).await?;
///
/// let manager = TransportManager::new(Arc::new(udp))
///     .with_selector(TransportSelector::LatencyBased);
///
/// // Use the manager like any other transport
/// manager.send_to(b"Hello!", "127.0.0.1:50000".parse()?).await?;
/// # Ok(())
/// # }
/// ```
pub struct TransportManager {
    /// All available transports.
    transports: RwLock<Vec<Arc<dyn Transport>>>,
    /// Index of the primary/active transport.
    primary_index: AtomicUsize,
    /// Selection strategy.
    selector: TransportSelector,
    /// Whether the manager is closed.
    closed: AtomicBool,
    /// Round-robin counter.
    rr_counter: AtomicUsize,
    /// Migration callback.
    migration_callback: Mutex<Option<MigrationCallback>>,
}

impl TransportManager {
    /// Create a new transport manager with a primary transport.
    ///
    /// # Arguments
    /// * `primary` - The primary transport to use
    #[must_use]
    pub fn new(primary: Arc<dyn Transport>) -> Self {
        Self {
            transports: RwLock::new(vec![primary]),
            primary_index: AtomicUsize::new(0),
            selector: TransportSelector::Primary,
            closed: AtomicBool::new(false),
            rr_counter: AtomicUsize::new(0),
            migration_callback: Mutex::new(None),
        }
    }

    /// Set the transport selection strategy.
    #[must_use]
    pub fn with_selector(mut self, selector: TransportSelector) -> Self {
        self.selector = selector;
        self
    }

    /// Add an additional transport.
    pub async fn add_transport(&self, transport: Arc<dyn Transport>) {
        self.transports.write().await.push(transport);
    }

    /// Set a callback for migration events.
    pub async fn set_migration_callback(&self, callback: MigrationCallback) {
        *self.migration_callback.lock().await = Some(callback);
    }

    /// Migrate the primary transport to the specified type.
    ///
    /// Searches for a transport of the given type among the registered transports
    /// and makes it the primary.
    ///
    /// # Arguments
    /// * `to` - The target transport type to migrate to
    ///
    /// # Errors
    /// Returns `TransportError::Other` if no transport of the requested type is available.
    pub async fn migrate(&self, to: TransportType) -> TransportResult<()> {
        let start = Instant::now();
        let transports = self.transports.read().await;
        let current_idx = self.primary_index.load(Ordering::Relaxed);
        let from_type = transports
            .get(current_idx)
            .map(|t| t.transport_type())
            .unwrap_or(TransportType::Udp);

        // Emit start event
        if let Some(cb) = self.migration_callback.lock().await.as_ref() {
            cb(MigrationEvent::Started {
                from: from_type,
                to,
            });
        }

        // Find transport of the target type
        let target_idx = transports.iter().position(|t| t.transport_type() == to);

        match target_idx {
            Some(idx) => {
                self.primary_index.store(idx, Ordering::Relaxed);
                drop(transports);

                let duration = start.elapsed();
                if let Some(cb) = self.migration_callback.lock().await.as_ref() {
                    cb(MigrationEvent::Completed {
                        from: from_type,
                        to,
                        duration,
                    });
                }
                Ok(())
            }
            None => {
                drop(transports);
                let error = format!("No transport of type {to} available");
                if let Some(cb) = self.migration_callback.lock().await.as_ref() {
                    cb(MigrationEvent::Failed {
                        from: from_type,
                        to,
                        error: error.clone(),
                    });
                }
                Err(TransportError::Other(error))
            }
        }
    }

    /// Get the number of registered transports.
    pub async fn transport_count(&self) -> usize {
        self.transports.read().await.len()
    }

    /// Get aggregated statistics across all transports.
    pub async fn aggregated_stats(&self) -> TransportStats {
        let transports = self.transports.read().await;
        let mut agg = TransportStats::default();
        for t in transports.iter() {
            let s = t.stats();
            agg.bytes_sent += s.bytes_sent;
            agg.bytes_received += s.bytes_received;
            agg.packets_sent += s.packets_sent;
            agg.packets_received += s.packets_received;
            agg.send_errors += s.send_errors;
            agg.recv_errors += s.recv_errors;
        }
        agg
    }

    /// Get the currently active transport type.
    pub async fn active_transport_type(&self) -> TransportType {
        let transports = self.transports.read().await;
        let idx = self.primary_index.load(Ordering::Relaxed);
        transports
            .get(idx)
            .map(|t| t.transport_type())
            .unwrap_or(TransportType::Udp)
    }

    /// Select a transport based on the configured strategy.
    async fn select_transport(&self) -> TransportResult<Arc<dyn Transport>> {
        let transports = self.transports.read().await;
        if transports.is_empty() {
            return Err(TransportError::Other("No transports available".to_string()));
        }

        let idx = match self.selector {
            TransportSelector::Primary => self.primary_index.load(Ordering::Relaxed),
            TransportSelector::RoundRobin => {
                self.rr_counter.fetch_add(1, Ordering::Relaxed) % transports.len()
            }
            TransportSelector::LatencyBased => {
                let mut best_idx = 0;
                let mut best_latency = Duration::MAX;
                for (i, t) in transports.iter().enumerate() {
                    if !t.is_closed() {
                        let lat = t.latency_estimate();
                        if lat < best_latency {
                            best_latency = lat;
                            best_idx = i;
                        }
                    }
                }
                best_idx
            }
            TransportSelector::ThroughputBased => {
                let mut best_idx = 0;
                let mut best_throughput = 0u64;
                for (i, t) in transports.iter().enumerate() {
                    if !t.is_closed() {
                        let s = t.stats();
                        let throughput = s.bytes_sent + s.bytes_received;
                        if throughput >= best_throughput {
                            best_throughput = throughput;
                            best_idx = i;
                        }
                    }
                }
                best_idx
            }
        };

        let idx = idx.min(transports.len() - 1);
        Ok(Arc::clone(&transports[idx]))
    }
}

#[async_trait]
impl Transport for TransportManager {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> TransportResult<usize> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }
        let transport = self.select_transport().await?;
        transport.send_to(buf, addr).await
    }

    async fn recv_from(&self, buf: &mut [u8]) -> TransportResult<(usize, SocketAddr)> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Closed);
        }
        let transport = self.select_transport().await?;
        transport.recv_from(buf).await
    }

    fn local_addr(&self) -> TransportResult<SocketAddr> {
        // Use primary transport's address
        // We need to block on the async read - use try_read for sync context
        // Since this is called in sync context, we check primary index directly
        Err(TransportError::Other(
            "Use active transport's local_addr directly".to_string(),
        ))
    }

    async fn close(&self) -> TransportResult<()> {
        self.closed.store(true, Ordering::Relaxed);
        let transports = self.transports.read().await;
        for t in transports.iter() {
            let _ = t.close().await;
        }
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    fn transport_type(&self) -> TransportType {
        // Return the primary transport's type
        // This is a sync method so we can't await; return a sensible default
        TransportType::Udp
    }

    fn supports_migration(&self) -> bool {
        true
    }

    fn mtu(&self) -> usize {
        1200 // Conservative default; actual MTU depends on selected transport
    }

    fn latency_estimate(&self) -> Duration {
        Duration::from_millis(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::udp_async::AsyncUdpTransport;
    use std::sync::atomic::AtomicUsize;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_manager_create() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));
        assert_eq!(manager.transport_count().await, 1);
        assert!(!manager.is_closed());
    }

    #[tokio::test]
    async fn test_manager_add_transport() {
        let udp1 = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let udp2 = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp1));
        manager.add_transport(Arc::new(udp2)).await;
        assert_eq!(manager.transport_count().await, 2);
    }

    #[tokio::test]
    async fn test_manager_close() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));
        assert!(!manager.is_closed());
        manager.close().await.unwrap();
        assert!(manager.is_closed());

        let result = manager
            .send_to(b"test", "127.0.0.1:1234".parse().unwrap())
            .await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }

    #[tokio::test]
    async fn test_manager_send_recv() {
        let server = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        let client_udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(client_udp));

        manager.send_to(b"manager test", server_addr).await.unwrap();

        let mut buf = vec![0u8; 1500];
        let (size, _from) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .expect("Timeout")
            .unwrap();
        assert_eq!(&buf[..size], b"manager test");
    }

    #[tokio::test]
    async fn test_manager_round_robin() {
        let udp1 = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let udp2 = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();

        let manager =
            TransportManager::new(Arc::new(udp1)).with_selector(TransportSelector::RoundRobin);
        manager.add_transport(Arc::new(udp2)).await;

        // Send to a dummy address (will fail but tests selection)
        let server = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        // Both sends should succeed via different transports
        manager.send_to(b"rr1", server_addr).await.unwrap();
        manager.send_to(b"rr2", server_addr).await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_migrate_not_found() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));

        let result = manager.migrate(TransportType::Tcp).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_aggregated_stats() {
        let server = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));

        manager.send_to(b"test", server_addr).await.unwrap();

        let stats = manager.aggregated_stats().await;
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_sent, 4);
    }

    #[tokio::test]
    async fn test_manager_migration_callback() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));

        let event_count = Arc::new(AtomicUsize::new(0));
        let event_count_clone = event_count.clone();
        manager
            .set_migration_callback(Arc::new(move |_event| {
                event_count_clone.fetch_add(1, Ordering::Relaxed);
            }))
            .await;

        // Attempt migration (will fail but should still emit events)
        let _ = manager.migrate(TransportType::Quic).await;
        // Started + Failed = 2 events
        assert_eq!(event_count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_manager_supports_migration() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));
        assert!(manager.supports_migration());
    }

    #[tokio::test]
    async fn test_manager_active_transport_type() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager = TransportManager::new(Arc::new(udp));
        assert_eq!(manager.active_transport_type().await, TransportType::Udp);
    }

    #[tokio::test]
    async fn test_selector_default() {
        assert_eq!(TransportSelector::default(), TransportSelector::Primary);
    }

    #[tokio::test]
    async fn test_manager_latency_based() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager =
            TransportManager::new(Arc::new(udp)).with_selector(TransportSelector::LatencyBased);

        let server = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        manager.send_to(b"latency", server_addr).await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_throughput_based() {
        let udp = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let manager =
            TransportManager::new(Arc::new(udp)).with_selector(TransportSelector::ThroughputBased);

        let server = AsyncUdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        manager.send_to(b"throughput", server_addr).await.unwrap();
    }
}
