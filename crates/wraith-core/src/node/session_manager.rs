//! Session management for WRAITH nodes
//!
//! This module provides session lifecycle management including:
//! - Session establishment (Noise_XX handshake)
//! - Session lookup and retrieval
//! - Session closure
//! - Handshake coordination
//!
//! # Architecture
//!
//! Sessions are stored in a concurrent DashMap keyed by peer ID (X25519 public key).
//! Each session has an associated route in the routing table for O(1) packet lookup.
//!
//! # Handshake Flow
//!
//! ```text
//! Initiator                     Responder
//!     |                              |
//!     |------ Noise msg1 (e) ------->|
//!     |                              |
//!     |<-- Noise msg2 (e,ee,s,es) ---|
//!     |                              |
//!     |------ Noise msg3 (s,se) ---->|
//!     |                              |
//!     |    [Session Established]     |
//! ```

use crate::node::error::{NodeError, Result};
use crate::node::session::{
    HandshakePacket, PeerConnection, PeerId, SessionId, perform_handshake_initiator,
    perform_handshake_responder,
};
use crate::{ConnectionId, HandshakePhase, SessionState};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use wraith_crypto::noise::NoiseKeypair;
use wraith_transport::udp_async::AsyncUdpTransport;

/// Session manager for WRAITH nodes
///
/// Coordinates session establishment, maintenance, and closure.
/// Thread-safe and designed for concurrent access.
pub struct SessionManager {
    /// Local X25519 keypair for Noise handshakes
    local_keypair: Arc<NoiseKeypair>,

    /// Active sessions (peer_id -> connection)
    sessions: Arc<DashMap<PeerId, Arc<PeerConnection>>>,

    /// Pending handshakes (peer_addr -> channel)
    pending_handshakes: Arc<DashMap<SocketAddr, oneshot::Sender<HandshakePacket>>>,

    /// Transport layer
    transport: Arc<Mutex<Option<Arc<AsyncUdpTransport>>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(
        local_keypair: Arc<NoiseKeypair>,
        sessions: Arc<DashMap<PeerId, Arc<PeerConnection>>>,
        pending_handshakes: Arc<DashMap<SocketAddr, oneshot::Sender<HandshakePacket>>>,
        transport: Arc<Mutex<Option<Arc<AsyncUdpTransport>>>>,
    ) -> Self {
        Self {
            local_keypair,
            sessions,
            pending_handshakes,
            transport,
        }
    }

    /// Get the transport layer
    async fn get_transport(&self) -> Result<Arc<AsyncUdpTransport>> {
        let guard = self.transport.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| NodeError::invalid_state("Transport not initialized"))
            .cloned()
    }

    /// Establish session with peer at known address
    ///
    /// Performs Noise_XX handshake and creates encrypted session.
    ///
    /// # Arguments
    ///
    /// * `_expected_peer_id` - Expected peer's public key (reserved for future validation)
    /// * `peer_addr` - The peer's network address
    /// * `routing` - Routing table for registering the new route
    ///
    /// # Returns
    ///
    /// Returns the session ID on success.
    ///
    /// # Note
    ///
    /// Sessions are stored using the peer's X25519 public key from the Noise handshake,
    /// not the passed-in Ed25519 key.
    pub async fn establish_session_with_addr(
        &self,
        _expected_peer_id: &PeerId,
        peer_addr: SocketAddr,
        routing: &crate::node::routing::RoutingTable,
    ) -> Result<SessionId> {
        let transport = self.get_transport().await?;

        tracing::info!("Establishing session with peer at {}", peer_addr);

        // Create channel for receiving msg2 (prevents recv_from racing with packet_receive_loop)
        let (msg2_tx, msg2_rx) = oneshot::channel();

        // Register pending handshake
        self.pending_handshakes.insert(peer_addr, msg2_tx);

        // Perform Noise_XX handshake as initiator
        let handshake_result = perform_handshake_initiator(
            &self.local_keypair,
            peer_addr,
            transport.as_ref(),
            Some(msg2_rx),
        )
        .await;

        // Clean up pending handshake entry
        self.pending_handshakes.remove(&peer_addr);

        // Propagate any handshake error
        let (crypto, session_id, peer_id) = handshake_result?;

        // Check if session already exists with this peer
        if let Some(connection) = self.sessions.get(&peer_id) {
            return Ok(connection.session_id);
        }

        // Derive connection ID from session ID
        let mut connection_id_bytes = [0u8; 8];
        connection_id_bytes.copy_from_slice(&session_id[..8]);
        let connection_id = ConnectionId::from_bytes(connection_id_bytes);

        // Create connection using X25519 peer_id from handshake
        let connection = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Transition through handshake states to established
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .await?;
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        // Store session using X25519 peer_id from handshake
        let connection_arc = Arc::new(connection);
        self.sessions.insert(peer_id, Arc::clone(&connection_arc));

        // Add route to routing table for Connection ID-based packet routing
        let cid_u64 = u64::from_be_bytes(connection_id_bytes);
        routing.add_route(cid_u64, connection_arc);

        tracing::info!(
            "Session established with peer {} (X25519), session: {}, route: {:016x}",
            hex::encode(&peer_id[..8]),
            hex::encode(&session_id[..8]),
            cid_u64
        );

        Ok(session_id)
    }

    /// Handle incoming handshake initiation (responder side)
    ///
    /// When a packet arrives with an unknown Connection ID, it could be a
    /// Noise_XX handshake initiation. This method processes msg1, completes
    /// the handshake, and creates a new session for the peer.
    ///
    /// # Arguments
    ///
    /// * `msg1` - The first handshake message from the initiator
    /// * `peer_addr` - The address the handshake came from
    /// * `routing` - Routing table for registering the new route
    ///
    /// # Returns
    ///
    /// Returns the session ID on success.
    pub async fn handle_handshake_initiation(
        &self,
        msg1: &[u8],
        peer_addr: SocketAddr,
        routing: &crate::node::routing::RoutingTable,
    ) -> Result<SessionId> {
        let transport = self.get_transport().await?;

        tracing::info!(
            "Handling handshake initiation from {} ({} bytes)",
            peer_addr,
            msg1.len()
        );

        // Create channel for receiving msg3 (prevents recv_from racing with packet_receive_loop)
        let (msg3_tx, msg3_rx) = oneshot::channel();

        // Register pending handshake
        self.pending_handshakes.insert(peer_addr, msg3_tx);

        // Perform Noise_XX handshake as responder
        let handshake_result = perform_handshake_responder(
            &self.local_keypair,
            msg1,
            peer_addr,
            transport.as_ref(),
            Some(msg3_rx),
        )
        .await;

        // Clean up pending handshake entry
        self.pending_handshakes.remove(&peer_addr);

        // Propagate any handshake error
        let (crypto, session_id, peer_id) = handshake_result?;

        // Derive connection ID from session ID
        let mut connection_id_bytes = [0u8; 8];
        connection_id_bytes.copy_from_slice(&session_id[..8]);
        let connection_id = ConnectionId::from_bytes(connection_id_bytes);

        // Create connection
        let connection = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Transition through handshake states to established
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::RespSent))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        // Store session (check if one already exists from initiator side)
        if self.sessions.contains_key(&peer_id) {
            tracing::warn!(
                "Session already exists for peer {} - race condition?",
                hex::encode(&peer_id[..8])
            );
            if let Some(existing) = self.sessions.get(&peer_id) {
                return Ok(existing.session_id);
            }
        }

        let connection_arc = Arc::new(connection);
        self.sessions.insert(peer_id, Arc::clone(&connection_arc));

        // Add route to routing table for Connection ID-based packet routing
        let cid_u64 = u64::from_be_bytes(connection_id_bytes);
        routing.add_route(cid_u64, connection_arc);

        tracing::info!(
            "Session established as responder with peer {}, session: {}, route: {:016x}",
            hex::encode(&peer_id[..8]),
            hex::encode(&session_id[..8]),
            cid_u64
        );

        Ok(session_id)
    }

    /// Get or establish session with peer
    ///
    /// Returns an existing session if one exists, otherwise establishes a new one.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer's public key
    /// * `peer_addr` - The peer's address (used if establishing new session)
    /// * `routing` - Routing table for registering new routes
    pub async fn get_or_establish_session(
        &self,
        peer_id: &PeerId,
        peer_addr: SocketAddr,
        routing: &crate::node::routing::RoutingTable,
    ) -> Result<Arc<PeerConnection>> {
        // Try to get existing session
        if let Some(connection) = self.sessions.get(peer_id) {
            return Ok(Arc::clone(connection.value()));
        }

        // Establish new session
        let _session_id = self
            .establish_session_with_addr(peer_id, peer_addr, routing)
            .await?;

        // Retrieve the newly created session
        self.sessions
            .get(peer_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(NodeError::SessionNotFound(*peer_id))
    }

    /// Close session with peer
    ///
    /// Removes the session from storage and the route from the routing table.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer's public key
    /// * `routing` - Routing table for removing the route
    pub async fn close_session(
        &self,
        peer_id: &PeerId,
        routing: &crate::node::routing::RoutingTable,
    ) -> Result<()> {
        if let Some((_, connection)) = self.sessions.remove(peer_id) {
            // Remove route from routing table
            let cid_u64 = connection.connection_id.as_u64();
            routing.remove_route(cid_u64);

            connection.transition_to(SessionState::Closed).await?;
            tracing::info!(
                "Session closed with peer {:?}, route {:016x} removed",
                peer_id,
                cid_u64
            );
            Ok(())
        } else {
            Err(NodeError::SessionNotFound(*peer_id))
        }
    }

    /// Get session by peer ID
    pub fn get_session(&self, peer_id: &PeerId) -> Option<Arc<PeerConnection>> {
        self.sessions.get(peer_id).map(|e| Arc::clone(e.value()))
    }

    /// Check if session exists
    pub fn has_session(&self, peer_id: &PeerId) -> bool {
        self.sessions.contains_key(peer_id)
    }

    /// List all active session peer IDs
    pub fn active_sessions(&self) -> Vec<PeerId> {
        self.sessions.iter().map(|entry| *entry.key()).collect()
    }

    /// Get number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Close all sessions
    ///
    /// Used during node shutdown to gracefully close all connections.
    pub async fn close_all_sessions(&self, routing: &crate::node::routing::RoutingTable) {
        for entry in self.sessions.iter() {
            let (peer_id, connection) = entry.pair();
            tracing::debug!("Closing session with peer {:?}", peer_id);

            // Remove route
            let cid_u64 = connection.connection_id.as_u64();
            routing.remove_route(cid_u64);

            // Transition to closed state
            if let Err(e) = connection.transition_to(SessionState::Closed).await {
                tracing::warn!("Error closing session: {}", e);
            }
        }

        // Clear all sessions
        self.sessions.clear();
    }

    /// Check pending handshake for address match
    ///
    /// Returns the matching address if there's a pending handshake for the given source.
    /// Handles INADDR_ANY (0.0.0.0) matching by port only.
    pub fn find_pending_handshake(&self, from: SocketAddr) -> Option<SocketAddr> {
        self.pending_handshakes
            .iter()
            .find(|entry| {
                let registered = entry.key();
                if registered.ip().is_unspecified() {
                    from.port() == registered.port()
                } else {
                    from == *registered
                }
            })
            .map(|entry| *entry.key())
    }

    /// Remove and return pending handshake sender
    pub fn take_pending_handshake(
        &self,
        addr: &SocketAddr,
    ) -> Option<oneshot::Sender<HandshakePacket>> {
        self.pending_handshakes.remove(addr).map(|(_, tx)| tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wraith_crypto::noise::NoiseKeypair;

    fn create_test_manager() -> SessionManager {
        let keypair = NoiseKeypair::generate().unwrap();
        SessionManager::new(
            Arc::new(keypair),
            Arc::new(DashMap::new()),
            Arc::new(DashMap::new()),
            Arc::new(Mutex::new(None)),
        )
    }

    #[test]
    fn test_session_manager_creation() {
        let manager = create_test_manager();
        assert_eq!(manager.session_count(), 0);
        assert!(manager.active_sessions().is_empty());
    }

    #[test]
    fn test_has_session() {
        let manager = create_test_manager();
        let peer_id = [42u8; 32];
        assert!(!manager.has_session(&peer_id));
    }

    #[test]
    fn test_get_session_not_found() {
        let manager = create_test_manager();
        let peer_id = [42u8; 32];
        assert!(manager.get_session(&peer_id).is_none());
    }

    #[test]
    fn test_find_pending_handshake_exact() {
        let manager = create_test_manager();
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        // Register a pending handshake
        let (tx, _rx) = oneshot::channel();
        manager.pending_handshakes.insert(addr, tx);

        // Should find exact match
        assert_eq!(manager.find_pending_handshake(addr), Some(addr));

        // Should not find different address
        let other_addr: SocketAddr = "192.168.1.200:5000".parse().unwrap();
        assert!(manager.find_pending_handshake(other_addr).is_none());
    }

    #[test]
    fn test_find_pending_handshake_inaddr_any() {
        let manager = create_test_manager();
        let inaddr_any: SocketAddr = "0.0.0.0:5000".parse().unwrap();

        // Register a pending handshake with INADDR_ANY
        let (tx, _rx) = oneshot::channel();
        manager.pending_handshakes.insert(inaddr_any, tx);

        // Should match any address with same port
        let loopback: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        assert_eq!(manager.find_pending_handshake(loopback), Some(inaddr_any));

        // Should not match different port
        let diff_port: SocketAddr = "127.0.0.1:6000".parse().unwrap();
        assert!(manager.find_pending_handshake(diff_port).is_none());
    }

    #[test]
    fn test_take_pending_handshake() {
        let manager = create_test_manager();
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();

        // Register a pending handshake
        let (tx, _rx) = oneshot::channel();
        manager.pending_handshakes.insert(addr, tx);

        // Should be able to take it once
        assert!(manager.take_pending_handshake(&addr).is_some());

        // Second take should return None
        assert!(manager.take_pending_handshake(&addr).is_none());
    }

    #[tokio::test]
    async fn test_close_session_not_found() {
        let manager = create_test_manager();
        let peer_id = [42u8; 32];
        let routing = crate::node::routing::RoutingTable::new();

        let result = manager.close_session(&peer_id, &routing).await;
        assert!(matches!(result, Err(NodeError::SessionNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_transport_not_initialized() {
        let manager = create_test_manager();
        let result = manager.get_transport().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_close_session_with_session() {
        use crate::node::session::PeerConnection;

        let manager = create_test_manager();
        let routing = crate::node::routing::RoutingTable::new();

        let peer_id = [42u8; 32];
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let conn = PeerConnection::new_for_test(peer_id, addr);

        // Transition to Established so we can close
        conn.transition_to(crate::SessionState::Handshaking(
            crate::HandshakePhase::RespSent,
        ))
        .await
        .unwrap();
        conn.transition_to(crate::SessionState::Established)
            .await
            .unwrap();

        let conn_arc = Arc::new(conn);
        let cid_u64 = conn_arc.connection_id.as_u64();

        // Insert session and route
        manager.sessions.insert(peer_id, Arc::clone(&conn_arc));
        routing.add_route(cid_u64, conn_arc);

        assert_eq!(manager.session_count(), 1);
        assert!(routing.has_route(cid_u64));

        // Close session
        let result = manager.close_session(&peer_id, &routing).await;
        assert!(result.is_ok());
        assert_eq!(manager.session_count(), 0);
        assert!(!routing.has_route(cid_u64));
    }

    #[tokio::test]
    async fn test_close_all_sessions() {
        use crate::node::session::PeerConnection;

        let manager = create_test_manager();
        let routing = crate::node::routing::RoutingTable::new();

        // Insert two sessions
        for i in 0..2u8 {
            let mut peer_id = [0u8; 32];
            peer_id[0] = i + 1;
            let addr: SocketAddr = format!("127.0.0.1:{}", 9000u16 + u16::from(i))
                .parse()
                .unwrap();
            let conn = Arc::new(PeerConnection::new_for_test(peer_id, addr));
            let cid = conn.connection_id.as_u64();
            routing.add_route(cid, Arc::clone(&conn));
            manager.sessions.insert(peer_id, conn);
        }

        assert_eq!(manager.session_count(), 2);

        manager.close_all_sessions(&routing).await;
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_get_session_found() {
        use crate::node::session::PeerConnection;

        let manager = create_test_manager();
        let peer_id = [42u8; 32];
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let conn = Arc::new(PeerConnection::new_for_test(peer_id, addr));
        manager.sessions.insert(peer_id, conn);

        let result = manager.get_session(&peer_id);
        assert!(result.is_some());
        assert_eq!(result.unwrap().peer_id, peer_id);
    }

    #[test]
    fn test_has_session_true() {
        use crate::node::session::PeerConnection;

        let manager = create_test_manager();
        let peer_id = [42u8; 32];
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        manager.sessions.insert(
            peer_id,
            Arc::new(PeerConnection::new_for_test(peer_id, addr)),
        );

        assert!(manager.has_session(&peer_id));
    }

    #[test]
    fn test_active_sessions_with_entries() {
        use crate::node::session::PeerConnection;

        let manager = create_test_manager();
        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        manager.sessions.insert(
            peer1,
            Arc::new(PeerConnection::new_for_test(
                peer1,
                "127.0.0.1:9001".parse().unwrap(),
            )),
        );
        manager.sessions.insert(
            peer2,
            Arc::new(PeerConnection::new_for_test(
                peer2,
                "127.0.0.1:9002".parse().unwrap(),
            )),
        );

        let active = manager.active_sessions();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&peer1));
        assert!(active.contains(&peer2));
    }

    #[test]
    fn test_find_pending_handshake_none() {
        let manager = create_test_manager();
        let addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
        assert!(manager.find_pending_handshake(addr).is_none());
    }
}
