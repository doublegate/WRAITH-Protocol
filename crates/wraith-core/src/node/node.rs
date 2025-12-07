//! Node implementation - high-level protocol orchestrator
//!
//! The Node is the primary entry point for WRAITH Protocol applications.
//! It coordinates cryptographic handshakes, transport, discovery, and file transfers.
//!
//! # Example
//!
//! ```no_run
//! use wraith_core::node::Node;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let node = Node::new_random().await?;
//!     node.start().await?;
//!
//!     let peer_id = [0u8; 32];
//!     let transfer_id = node.send_file("file.txt", &peer_id).await?;
//!     node.wait_for_transfer(transfer_id).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::node::config::NodeConfig;
use crate::node::error::{NodeError, Result};
use crate::node::file_transfer::FileTransferContext;
use crate::node::identity::{Identity, TransferId};
use crate::node::routing::RoutingTable;
use crate::node::session::{HandshakePacket, PeerConnection, PeerId, SessionId};
use crate::transfer::TransferSession;
use crate::{ConnectionId, HandshakePhase, SessionState};
use dashmap::DashMap;
use getrandom::getrandom;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, oneshot};
use wraith_discovery::{DiscoveryConfig as DiscoveryConfigInternal, DiscoveryManager};
use wraith_files::tree_hash::compute_tree_hash;
use wraith_transport::transport::Transport;
use wraith_transport::udp_async::AsyncUdpTransport;

/// Node inner state
pub(crate) struct NodeInner {
    /// Node identity
    pub(crate) identity: Arc<Identity>,
    /// Node configuration
    pub(crate) config: NodeConfig,
    /// Active sessions (peer_id -> connection)
    pub(crate) sessions: Arc<DashMap<PeerId, Arc<PeerConnection>>>,
    /// Packet routing table (Connection ID -> PeerConnection)
    pub(crate) routing: Arc<RoutingTable>,
    /// Active file transfers (transfer_id -> transfer context)
    pub(crate) transfers: Arc<DashMap<TransferId, Arc<FileTransferContext>>>,
    /// Pending handshakes (peer_addr -> channel)
    pub(crate) pending_handshakes: Arc<DashMap<SocketAddr, oneshot::Sender<HandshakePacket>>>,
    /// Node running state
    pub(crate) running: Arc<AtomicBool>,
    /// Transport layer
    pub(crate) transport: Arc<Mutex<Option<Arc<AsyncUdpTransport>>>>,
    /// Discovery manager
    pub(crate) discovery: Arc<Mutex<Option<Arc<DiscoveryManager>>>>,
}

/// WRAITH Protocol Node
///
/// The Node is the high-level API for the WRAITH protocol. It coordinates:
/// - Cryptographic handshakes (Noise_XX)
/// - Transport layer (AF_XDP/UDP)
/// - Peer discovery (DHT)
/// - NAT traversal
/// - Obfuscation
/// - File transfers
#[derive(Clone)]
pub struct Node {
    pub(crate) inner: Arc<NodeInner>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Constructors
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Create node with random identity
    pub async fn new_random() -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, NodeConfig::default()).await
    }

    /// Create node with custom configuration
    pub async fn new_with_config(config: NodeConfig) -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, config).await
    }

    /// Create node with specific port (useful for testing)
    pub async fn new_random_with_port(port: u16) -> Result<Self> {
        let identity = Identity::generate()?;
        let config = NodeConfig {
            listen_addr: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..NodeConfig::default()
        };
        Self::new_from_identity(identity, config).await
    }

    /// Create node from existing identity
    pub async fn new_from_identity(identity: Identity, config: NodeConfig) -> Result<Self> {
        let inner = NodeInner {
            identity: Arc::new(identity),
            config,
            sessions: Arc::new(DashMap::new()),
            routing: Arc::new(RoutingTable::new()),
            transfers: Arc::new(DashMap::new()),
            pending_handshakes: Arc::new(DashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            transport: Arc::new(Mutex::new(None)),
            discovery: Arc::new(Mutex::new(None)),
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Identity Methods
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Get node's public key (Ed25519 node ID)
    pub fn node_id(&self) -> &[u8; 32] {
        self.inner.identity.public_key()
    }

    /// Get node's X25519 public key (used in Noise handshakes)
    pub fn x25519_public_key(&self) -> &[u8; 32] {
        self.inner.identity.x25519_public_key()
    }

    /// Get node's identity
    pub fn identity(&self) -> &Arc<Identity> {
        &self.inner.identity
    }

    /// Get node's actual listening address
    pub async fn listen_addr(&self) -> Result<SocketAddr> {
        let transport = self.inner.transport.lock().await;
        match transport.as_ref() {
            Some(t) => {
                let mut addr = t.local_addr().map_err(|e| {
                    NodeError::Transport(format!("Failed to get local address: {}", e))
                })?;
                if addr.ip().is_unspecified() {
                    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
                    let loopback = if addr.is_ipv4() {
                        IpAddr::V4(Ipv4Addr::LOCALHOST)
                    } else {
                        IpAddr::V6(Ipv6Addr::LOCALHOST)
                    };
                    addr.set_ip(loopback);
                }
                Ok(addr)
            }
            None => Err(NodeError::InvalidState(
                "Transport not initialized".to_string(),
            )),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Lifecycle Methods
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Start the node
    pub async fn start(&self) -> Result<()> {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(NodeError::InvalidState("Node already running".to_string()));
        }

        tracing::info!(
            "Starting node {} on {}",
            hex::encode(self.node_id()),
            self.inner.config.listen_addr
        );

        // Initialize transport
        let transport = AsyncUdpTransport::bind(self.inner.config.listen_addr)
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to bind transport: {}", e)))?;
        let transport = Arc::new(transport);
        *self.inner.transport.lock().await = Some(Arc::clone(&transport));

        // Initialize discovery
        let node_id_bytes = wraith_discovery::dht::NodeId::from_bytes(*self.node_id());
        let mut discovery_config =
            DiscoveryConfigInternal::new(node_id_bytes, self.inner.config.listen_addr);
        discovery_config.nat_detection_enabled = self.inner.config.discovery.enable_nat_traversal;
        discovery_config.relay_enabled = self.inner.config.discovery.enable_relay;

        let discovery = DiscoveryManager::new(discovery_config).await.map_err(|e| {
            NodeError::Discovery(format!("Failed to create discovery manager: {}", e))
        })?;
        let discovery = Arc::new(discovery);
        *self.inner.discovery.lock().await = Some(Arc::clone(&discovery));
        discovery
            .start()
            .await
            .map_err(|e| NodeError::Discovery(format!("Failed to start discovery: {}", e)))?;

        // Start packet receive loop (defined in packet_handler.rs)
        let node = self.clone();
        tokio::spawn(async move {
            node.packet_receive_loop().await;
        });

        // Start cover traffic if enabled
        if self.inner.config.obfuscation.cover_traffic.enabled {
            let node = self.clone();
            tokio::spawn(async move {
                node.cover_traffic_loop().await;
            });
        }

        tracing::info!("Node started: {:?}", hex::encode(self.node_id()));
        Ok(())
    }

    /// Stop the node
    pub async fn stop(&self) -> Result<()> {
        if self
            .inner
            .running
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(NodeError::InvalidState("Node not running".to_string()));
        }

        // Close all sessions
        for entry in self.inner.sessions.iter() {
            let (peer_id, connection) = entry.pair();
            tracing::debug!("Closing session with peer {:?}", peer_id);
            if let Err(e) = connection.transition_to(SessionState::Closed).await {
                tracing::warn!("Error closing session: {}", e);
            }
        }

        // Clear routing table
        self.inner.routing.clear();

        // Close transport
        if let Some(transport) = self.inner.transport.lock().await.take() {
            if let Err(e) = transport.close().await {
                tracing::warn!("Error closing transport: {}", e);
            }
        }

        tracing::info!("Node stopped");
        Ok(())
    }

    /// Check if node is running
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session Methods
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Establish session with peer (via DHT lookup)
    pub async fn establish_session(&self, peer_id: &PeerId) -> Result<SessionId> {
        if let Some(connection) = self.inner.sessions.get(peer_id) {
            return Ok(connection.session_id);
        }
        // TODO: Lookup peer address via DHT
        let peer_addr: SocketAddr = "127.0.0.1:8421".parse().unwrap();
        self.establish_session_with_addr(peer_id, peer_addr).await
    }

    /// Establish session with peer at known address
    pub async fn establish_session_with_addr(
        &self,
        _expected_peer_id: &PeerId,
        peer_addr: SocketAddr,
    ) -> Result<SessionId> {
        let transport = self.get_transport().await?;
        tracing::info!("Establishing session with peer at {}", peer_addr);

        let (msg2_tx, msg2_rx) = oneshot::channel();
        self.inner.pending_handshakes.insert(peer_addr, msg2_tx);

        let handshake_result = crate::node::session::perform_handshake_initiator(
            self.inner.identity.x25519_keypair(),
            peer_addr,
            transport.as_ref(),
            Some(msg2_rx),
        )
        .await;
        self.inner.pending_handshakes.remove(&peer_addr);
        let (crypto, session_id, peer_id) = handshake_result?;

        if let Some(connection) = self.inner.sessions.get(&peer_id) {
            return Ok(connection.session_id);
        }

        let mut connection_id_bytes = [0u8; 8];
        connection_id_bytes.copy_from_slice(&session_id[..8]);
        let connection_id = ConnectionId::from_bytes(connection_id_bytes);
        let connection = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .await?;
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        let connection_arc = Arc::new(connection);
        self.inner
            .sessions
            .insert(peer_id, Arc::clone(&connection_arc));
        let cid_u64 = u64::from_be_bytes(connection_id_bytes);
        self.inner.routing.add_route(cid_u64, connection_arc);

        tracing::info!(
            "Session established with peer {} (X25519), session: {}, route: {:016x}",
            hex::encode(&peer_id[..8]),
            hex::encode(&session_id[..8]),
            cid_u64
        );
        Ok(session_id)
    }

    /// Get or establish session with peer
    pub async fn get_or_establish_session(&self, peer_id: &PeerId) -> Result<Arc<PeerConnection>> {
        if let Some(connection) = self.inner.sessions.get(peer_id) {
            return Ok(Arc::clone(connection.value()));
        }
        let _session_id = self.establish_session(peer_id).await?;
        self.inner
            .sessions
            .get(peer_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(NodeError::SessionNotFound(*peer_id))
    }

    /// Close session with peer
    pub async fn close_session(&self, peer_id: &PeerId) -> Result<()> {
        if let Some((_, connection)) = self.inner.sessions.remove(peer_id) {
            let cid_u64 = connection.connection_id.as_u64();
            self.inner.routing.remove_route(cid_u64);
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

    /// List active sessions
    pub async fn active_sessions(&self) -> Vec<PeerId> {
        self.inner
            .sessions
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get routing statistics
    pub fn routing_stats(&self) -> crate::node::routing::RoutingStats {
        self.inner.routing.stats()
    }

    /// Get number of active routes
    pub fn active_route_count(&self) -> usize {
        self.inner.routing.route_count()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Transfer Methods
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Send file to peer
    pub async fn send_file(
        &self,
        file_path: impl AsRef<Path>,
        peer_id: &PeerId,
    ) -> Result<TransferId> {
        let file_path = file_path.as_ref();
        let file_size = std::fs::metadata(file_path).map_err(NodeError::Io)?.len();
        if file_size == 0 {
            return Err(NodeError::InvalidState(
                "Cannot send empty file".to_string(),
            ));
        }

        let chunk_size = self.inner.config.transfer.chunk_size;
        let tree_hash = compute_tree_hash(file_path, chunk_size).map_err(NodeError::Io)?;
        let transfer_id = Self::generate_transfer_id();

        let mut transfer =
            TransferSession::new_send(transfer_id, file_path.to_path_buf(), file_size, chunk_size);
        transfer.start();

        let transfer_arc = Arc::new(RwLock::new(transfer));
        let context = Arc::new(FileTransferContext::new_send(
            transfer_id,
            Arc::clone(&transfer_arc),
            tree_hash.clone(),
        ));
        self.inner
            .transfers
            .insert(transfer_id, Arc::clone(&context));

        let connection = self.get_or_establish_session(peer_id).await?;
        let stream_id = ((transfer_id[0] as u16) << 8) | (transfer_id[1] as u16);

        let metadata = crate::node::file_transfer::FileMetadata::from_path_and_hash(
            transfer_id,
            file_path,
            file_size,
            chunk_size,
            &tree_hash,
        )?;
        let metadata_frame =
            crate::node::file_transfer::build_metadata_frame(stream_id, &metadata)?;
        self.send_encrypted_frame(&connection, &metadata_frame)
            .await?;

        let node = self.clone();
        let file_path_buf = file_path.to_path_buf();
        tokio::spawn(async move {
            if let Err(e) = node
                .send_file_chunks(transfer_id, file_path_buf, stream_id, connection)
                .await
            {
                tracing::error!("Error sending file chunks: {}", e);
            }
        });

        Ok(transfer_id)
    }

    /// Wait for transfer to complete
    pub async fn wait_for_transfer(&self, transfer_id: TransferId) -> Result<()> {
        loop {
            if let Some(context) = self.inner.transfers.get(&transfer_id) {
                if context.transfer_session.read().await.is_complete() {
                    return Ok(());
                }
            } else {
                return Err(NodeError::TransferNotFound(transfer_id));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get transfer progress
    pub async fn get_transfer_progress(&self, transfer_id: &TransferId) -> Option<f64> {
        self.inner
            .transfers
            .get(transfer_id)
            .map(|context| context.transfer_session.clone())
            .map(|session| async move { session.read().await.progress() })
            .map(|fut| {
                tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
            })
    }

    /// List active transfers
    pub async fn active_transfers(&self) -> Vec<TransferId> {
        self.inner
            .transfers
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Generate random transfer ID
    pub(crate) fn generate_transfer_id() -> TransferId {
        let mut id = [0u8; 32];
        getrandom(&mut id).expect("Failed to generate transfer ID");
        id
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Internal Helpers
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Get transport layer
    pub(crate) async fn get_transport(&self) -> Result<Arc<AsyncUdpTransport>> {
        let guard = self.inner.transport.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| NodeError::InvalidState("Transport not initialized".to_string()))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate().unwrap();
        assert_eq!(identity.public_key().len(), 32);
    }

    #[tokio::test]
    async fn test_node_creation() {
        let node = Node::new_random().await.unwrap();
        assert_eq!(node.node_id().len(), 32);
        assert!(!node.is_running());
    }

    #[tokio::test]
    async fn test_node_start_stop() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();
        assert!(node.is_running());
        assert!(node.start().await.is_err());
        node.stop().await.unwrap();
        assert!(!node.is_running());
        assert!(node.stop().await.is_err());
    }

    #[tokio::test]
    async fn test_active_sessions_empty() {
        let node = Node::new_random().await.unwrap();
        assert!(node.active_sessions().await.is_empty());
    }

    #[tokio::test]
    async fn test_transfer_id_generation() {
        let id1 = Node::generate_transfer_id();
        let id2 = Node::generate_transfer_id();
        assert_eq!(id1.len(), 32);
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_frame_encryption_roundtrip() {
        use crate::FRAME_HEADER_SIZE;
        use crate::frame::{FrameBuilder, FrameType};
        use wraith_crypto::aead::SessionCrypto;

        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);

        let alice_crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);
        let bob_crypto = SessionCrypto::new([5u8; 32], [4u8; 32], &[6u8; 32]);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, bob_crypto);

        let payload = b"Hello, encrypted WRAITH!";
        let frame_bytes = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .offset(0)
            .payload(payload)
            .build(FRAME_HEADER_SIZE + payload.len())
            .unwrap();

        let encrypted = alice.encrypt_frame(&frame_bytes).await.unwrap();
        let decrypted = bob.decrypt_frame(&encrypted).await.unwrap();
        let parsed = crate::frame::Frame::parse(&decrypted).unwrap();

        assert_eq!(parsed.frame_type(), FrameType::Data);
        assert_eq!(parsed.payload(), payload);
    }

    #[tokio::test]
    async fn test_encrypted_frame_tampering_detection() {
        use crate::FRAME_HEADER_SIZE;
        use crate::frame::{FrameBuilder, FrameType};
        use wraith_crypto::aead::SessionCrypto;

        let alice_crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);
        let bob_crypto = SessionCrypto::new([5u8; 32], [4u8; 32], &[6u8; 32]);

        let alice = PeerConnection::new(
            [1u8; 32],
            [2u8; 32],
            "127.0.0.1:5000".parse().unwrap(),
            ConnectionId::from_bytes([3u8; 8]),
            alice_crypto,
        );
        let bob = PeerConnection::new(
            [1u8; 32],
            [2u8; 32],
            "127.0.0.1:5000".parse().unwrap(),
            ConnectionId::from_bytes([3u8; 8]),
            bob_crypto,
        );

        let payload = b"Secret data";
        let frame_bytes = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(100)
            .sequence(1)
            .payload(payload)
            .build(FRAME_HEADER_SIZE + payload.len())
            .unwrap();

        let mut encrypted = alice.encrypt_frame(&frame_bytes).await.unwrap();
        if let Some(byte) = encrypted.get_mut(10) {
            *byte ^= 0xFF;
        }
        assert!(bob.decrypt_frame(&encrypted).await.is_err());
    }
}
