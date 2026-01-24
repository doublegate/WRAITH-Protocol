//! Packet routing infrastructure for WRAITH Protocol
//!
//! This module provides the routing table that maps Connection IDs to peer connections,
//! enabling incoming packets to be routed to the correct session for processing.
//!
//! # Architecture
//!
//! ```text
//! Incoming Packet
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │ Extract Connection  │
//! │ ID (first 8 bytes)  │
//! └──────────┬──────────┘
//!            │
//!            ▼
//! ┌─────────────────────┐     ┌───────────────────────┐
//! │   RoutingTable      │────▶│ PeerConnection lookup │
//! │   (DashMap)         │     │ by Connection ID      │
//! └──────────┬──────────┘     └───────────────────────┘
//!            │
//!            ▼
//! ┌─────────────────────┐
//! │ Decrypt & dispatch  │
//! │ to frame handler    │
//! └─────────────────────┘
//! ```
//!
//! # Performance
//!
//! Uses DashMap for lock-free concurrent access, enabling O(1) lookups with
//! minimal contention under high packet rates.

use crate::node::session::PeerConnection;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Packet routing table: Connection ID → PeerConnection
///
/// Provides fast, lock-free routing of incoming packets to the appropriate
/// session based on the Connection ID extracted from the packet header.
///
/// # Thread Safety
///
/// Uses DashMap internally for lock-free concurrent access. All operations
/// are safe to call from multiple threads simultaneously.
///
/// # Example
///
/// ```no_run
/// use wraith_core::node::routing::RoutingTable;
/// use wraith_core::node::session::PeerConnection;
/// use std::sync::Arc;
///
/// let routing = RoutingTable::new();
///
/// // Add route when session is established
/// // let connection = Arc::new(PeerConnection::new(...));
/// // routing.add_route(connection_id, connection);
///
/// // Lookup route for incoming packet
/// // let conn = routing.lookup(connection_id);
/// ```
pub struct RoutingTable {
    /// Map Connection ID (8 bytes as u64) to session
    routes: DashMap<u64, Arc<PeerConnection>>,

    /// Statistics: total lookups performed
    total_lookups: AtomicU64,

    /// Statistics: successful lookups (route found)
    successful_lookups: AtomicU64,

    /// Statistics: failed lookups (route not found)
    failed_lookups: AtomicU64,
}

impl RoutingTable {
    /// Create a new empty routing table
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
            total_lookups: AtomicU64::new(0),
            successful_lookups: AtomicU64::new(0),
            failed_lookups: AtomicU64::new(0),
        }
    }

    /// Add route for new session
    ///
    /// Associates a Connection ID with a peer connection. If a route already
    /// exists for this Connection ID, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - 8-byte Connection ID as u64
    /// * `connection` - Arc-wrapped peer connection handle
    pub fn add_route(&self, connection_id: u64, connection: Arc<PeerConnection>) {
        tracing::debug!(
            "Adding route: connection_id={:016x} -> peer_id={}",
            connection_id,
            hex::encode(&connection.peer_id[..8])
        );
        self.routes.insert(connection_id, connection);
    }

    /// Remove route when session closes
    ///
    /// Removes the routing entry for the given Connection ID.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - 8-byte Connection ID as u64
    ///
    /// # Returns
    ///
    /// Returns the removed connection if it existed, or `None` if not found.
    pub fn remove_route(&self, connection_id: u64) -> Option<Arc<PeerConnection>> {
        tracing::debug!("Removing route: connection_id={:016x}", connection_id);
        self.routes.remove(&connection_id).map(|(_, v)| v)
    }

    /// Lookup session by Connection ID
    ///
    /// Finds the peer connection associated with the given Connection ID.
    /// This is the hot path for packet routing - designed for minimal latency.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - 8-byte Connection ID as u64
    ///
    /// # Returns
    ///
    /// Returns the peer connection if found, or `None` if not routed.
    pub fn lookup(&self, connection_id: u64) -> Option<Arc<PeerConnection>> {
        self.total_lookups.fetch_add(1, Ordering::Relaxed);

        match self.routes.get(&connection_id) {
            Some(entry) => {
                self.successful_lookups.fetch_add(1, Ordering::Relaxed);
                Some(Arc::clone(entry.value()))
            }
            None => {
                self.failed_lookups.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Check if a route exists for the given Connection ID
    pub fn has_route(&self, connection_id: u64) -> bool {
        self.routes.contains_key(&connection_id)
    }

    /// Get all active Connection IDs
    ///
    /// Returns a list of all Connection IDs currently in the routing table.
    /// Useful for monitoring and debugging.
    pub fn active_routes(&self) -> Vec<u64> {
        self.routes.iter().map(|entry| *entry.key()).collect()
    }

    /// Get number of active routes
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Get routing statistics
    pub fn stats(&self) -> RoutingStats {
        RoutingStats {
            active_routes: self.routes.len(),
            total_lookups: self.total_lookups.load(Ordering::Relaxed),
            successful_lookups: self.successful_lookups.load(Ordering::Relaxed),
            failed_lookups: self.failed_lookups.load(Ordering::Relaxed),
        }
    }

    /// Clear all routes
    ///
    /// Removes all entries from the routing table. Used during shutdown.
    pub fn clear(&self) {
        tracing::debug!("Clearing all routes");
        self.routes.clear();
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Routing table statistics
#[derive(Debug, Clone, Copy)]
pub struct RoutingStats {
    /// Number of active routes
    pub active_routes: usize,

    /// Total lookup operations
    pub total_lookups: u64,

    /// Successful lookups (route found)
    pub successful_lookups: u64,

    /// Failed lookups (route not found)
    pub failed_lookups: u64,
}

impl RoutingStats {
    /// Calculate hit rate (percentage of successful lookups)
    pub fn hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            self.successful_lookups as f64 / self.total_lookups as f64 * 100.0
        }
    }
}

/// Extract Connection ID from packet header
///
/// The Connection ID is the first 8 bytes of the outer packet, used
/// to route packets to the correct session.
///
/// # Arguments
///
/// * `packet` - Raw packet bytes
///
/// # Returns
///
/// Returns the Connection ID as `u64`, or `None` if packet is too short.
pub fn extract_connection_id(packet: &[u8]) -> Option<u64> {
    if packet.len() < 8 {
        return None;
    }
    Some(u64::from_be_bytes(packet[0..8].try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionId;
    use crate::node::session::PeerConnection;
    use wraith_crypto::aead::SessionCrypto;

    /// Helper to create a test PeerConnection
    fn create_test_connection(id: u8) -> Arc<PeerConnection> {
        let session_id = [id; 32];
        let peer_id = [id + 1; 32];
        let peer_addr = format!("127.0.0.1:{}", 5000 + id as u16).parse().unwrap();
        let connection_id = ConnectionId::from_bytes([id; 8]);
        let crypto = SessionCrypto::new([id + 2; 32], [id + 3; 32], &[id + 4; 32]);

        Arc::new(PeerConnection::new(
            session_id,
            peer_id,
            peer_addr,
            connection_id,
            crypto,
        ))
    }

    #[test]
    fn test_routing_table_creation() {
        let routing = RoutingTable::new();
        assert_eq!(routing.route_count(), 0);
        assert!(routing.active_routes().is_empty());
    }

    #[test]
    fn test_add_and_lookup_route() {
        let routing = RoutingTable::new();
        let connection = create_test_connection(1);
        let connection_id = 0x0101010101010101u64;

        routing.add_route(connection_id, Arc::clone(&connection));

        assert_eq!(routing.route_count(), 1);
        assert!(routing.has_route(connection_id));

        let looked_up = routing.lookup(connection_id).unwrap();
        assert_eq!(looked_up.session_id, connection.session_id);
    }

    #[test]
    fn test_remove_route() {
        let routing = RoutingTable::new();
        let connection = create_test_connection(1);
        let connection_id = 0x0101010101010101u64;

        routing.add_route(connection_id, connection);
        assert!(routing.has_route(connection_id));

        let removed = routing.remove_route(connection_id);
        assert!(removed.is_some());
        assert!(!routing.has_route(connection_id));
        assert_eq!(routing.route_count(), 0);
    }

    #[test]
    fn test_lookup_nonexistent_route() {
        let routing = RoutingTable::new();
        let result = routing.lookup(0xDEADBEEF);
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_routes() {
        let routing = RoutingTable::new();

        let connection1 = create_test_connection(1);
        let connection2 = create_test_connection(2);
        let connection3 = create_test_connection(3);

        routing.add_route(0x1111111111111111, connection1);
        routing.add_route(0x2222222222222222, connection2);
        routing.add_route(0x3333333333333333, connection3);

        assert_eq!(routing.route_count(), 3);

        let conn1 = routing.lookup(0x1111111111111111).unwrap();
        assert_eq!(conn1.peer_id[0], 2);

        let conn2 = routing.lookup(0x2222222222222222).unwrap();
        assert_eq!(conn2.peer_id[0], 3);

        let conn3 = routing.lookup(0x3333333333333333).unwrap();
        assert_eq!(conn3.peer_id[0], 4);
    }

    #[test]
    fn test_active_routes() {
        let routing = RoutingTable::new();

        routing.add_route(0x1111111111111111, create_test_connection(1));
        routing.add_route(0x2222222222222222, create_test_connection(2));
        routing.add_route(0x3333333333333333, create_test_connection(3));

        let active = routing.active_routes();
        assert_eq!(active.len(), 3);
        assert!(active.contains(&0x1111111111111111));
        assert!(active.contains(&0x2222222222222222));
        assert!(active.contains(&0x3333333333333333));
    }

    #[test]
    fn test_routing_statistics() {
        let routing = RoutingTable::new();
        let connection = create_test_connection(1);
        let connection_id = 0x0101010101010101u64;

        routing.add_route(connection_id, connection);

        // Perform some lookups
        let _ = routing.lookup(connection_id); // Success
        let _ = routing.lookup(connection_id); // Success
        let _ = routing.lookup(0xDEADBEEF); // Fail
        let _ = routing.lookup(connection_id); // Success
        let _ = routing.lookup(0xCAFEBABE); // Fail

        let stats = routing.stats();
        assert_eq!(stats.active_routes, 1);
        assert_eq!(stats.total_lookups, 5);
        assert_eq!(stats.successful_lookups, 3);
        assert_eq!(stats.failed_lookups, 2);
        assert!((stats.hit_rate() - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_clear_routes() {
        let routing = RoutingTable::new();

        routing.add_route(0x1111111111111111, create_test_connection(1));
        routing.add_route(0x2222222222222222, create_test_connection(2));
        routing.add_route(0x3333333333333333, create_test_connection(3));

        assert_eq!(routing.route_count(), 3);

        routing.clear();

        assert_eq!(routing.route_count(), 0);
        assert!(routing.active_routes().is_empty());
    }

    #[test]
    fn test_route_replacement() {
        let routing = RoutingTable::new();
        let connection1 = create_test_connection(1);
        let connection2 = create_test_connection(2);
        let connection_id = 0x0101010101010101u64;

        // Add first connection
        routing.add_route(connection_id, Arc::clone(&connection1));
        let looked_up = routing.lookup(connection_id).unwrap();
        assert_eq!(looked_up.peer_id[0], 2); // connection1 has peer_id[0] = 2

        // Replace with second connection
        routing.add_route(connection_id, connection2);
        let looked_up = routing.lookup(connection_id).unwrap();
        assert_eq!(looked_up.peer_id[0], 3); // connection2 has peer_id[0] = 3

        // Should still be only one route
        assert_eq!(routing.route_count(), 1);
    }

    #[test]
    fn test_extract_connection_id() {
        // Valid packet with Connection ID
        let packet = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let cid = extract_connection_id(&packet).unwrap();
        assert_eq!(cid, 0x0102030405060708);

        // Packet too short
        let short_packet = [0x01, 0x02, 0x03];
        assert!(extract_connection_id(&short_packet).is_none());

        // Empty packet
        assert!(extract_connection_id(&[]).is_none());
    }

    #[test]
    fn test_concurrent_route_operations() {
        use std::thread;

        let routing = Arc::new(RoutingTable::new());

        let mut handles = Vec::new();

        // Spawn multiple threads adding routes
        for i in 0..10 {
            let routing_clone = Arc::clone(&routing);
            handles.push(thread::spawn(move || {
                let connection = create_test_connection(i as u8);
                let connection_id = (i as u64) << 56;
                routing_clone.add_route(connection_id, connection);
            }));
        }

        // Spawn multiple threads doing lookups
        for i in 0..10 {
            let routing_clone = Arc::clone(&routing);
            handles.push(thread::spawn(move || {
                let connection_id = (i as u64) << 56;
                for _ in 0..100 {
                    let _ = routing_clone.lookup(connection_id);
                }
            }));
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // All routes should be added
        assert_eq!(routing.route_count(), 10);

        // Stats should reflect the lookups
        let stats = routing.stats();
        assert!(stats.total_lookups >= 1000);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let stats = RoutingStats {
            active_routes: 5,
            total_lookups: 100,
            successful_lookups: 75,
            failed_lookups: 25,
        };
        assert!((stats.hit_rate() - 75.0).abs() < 0.001);

        // Edge case: no lookups
        let empty_stats = RoutingStats {
            active_routes: 0,
            total_lookups: 0,
            successful_lookups: 0,
            failed_lookups: 0,
        };
        assert!((empty_stats.hit_rate() - 0.0).abs() < 0.001);
    }
}
