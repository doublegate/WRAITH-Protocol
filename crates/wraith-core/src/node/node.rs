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
use crate::node::ip_reputation::IpReputationSystem;
use crate::node::obfuscation::ObfuscationStats;
use crate::node::rate_limiter::RateLimiter;
use crate::node::routing::RoutingTable;
use crate::node::security_monitor::SecurityMonitor;
use crate::node::session::{HandshakePacket, PeerConnection, PeerId, SessionId};
use crate::transfer::TransferSession;
use crate::{ConnectionId, HandshakePhase, SessionState};
use dashmap::DashMap;
use getrandom::getrandom;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, oneshot};
use wraith_discovery::{DiscoveryConfig as DiscoveryConfigInternal, DiscoveryManager};
use wraith_files::tree_hash::compute_tree_hash;
use wraith_obfuscation::{DohTunnel, TlsRecordWrapper, WebSocketFrameWrapper};
use wraith_transport::transport::Transport;
use wraith_transport::udp_async::AsyncUdpTransport;

/// Type alias for pending chunk request map: (stream_id, chunk_index) -> data sender
type PendingChunkMap = DashMap<(u16, u64), oneshot::Sender<Vec<u8>>>;

/// Migration state for tracking PATH_CHALLENGE/RESPONSE
#[allow(dead_code)]
pub(crate) struct MigrationState {
    /// Peer being migrated
    pub peer_id: PeerId,
    /// New address to migrate to
    pub new_addr: SocketAddr,
    /// Challenge data (8 bytes)
    pub challenge: [u8; 8],
    /// Channel to signal migration completion
    pub sender: oneshot::Sender<Result<Duration>>,
    /// Time when migration was initiated
    pub initiated_at: Instant,
}

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
    /// Pending pings (peer_id, sequence -> response channel)
    pub(crate) pending_pings: Arc<DashMap<(PeerId, u32), oneshot::Sender<Instant>>>,
    /// Pending migrations (path_id -> migration state)
    pub(crate) pending_migrations: Arc<DashMap<u64, MigrationState>>,
    /// Pending chunk requests ((stream_id, chunk_idx) -> data sender)
    pub(crate) pending_chunks: Arc<PendingChunkMap>,
    /// Node running state
    pub(crate) running: Arc<AtomicBool>,
    /// Transport layer
    pub(crate) transport: Arc<Mutex<Option<Arc<AsyncUdpTransport>>>>,
    /// Discovery manager
    pub(crate) discovery: Arc<Mutex<Option<Arc<DiscoveryManager>>>>,
    /// Rate limiter for DoS protection
    pub(crate) rate_limiter: Arc<RateLimiter>,
    /// IP reputation system
    pub(crate) ip_reputation: Arc<IpReputationSystem>,
    /// Security event monitor
    pub(crate) security_monitor: Arc<SecurityMonitor>,
    /// TLS record wrapper for protocol mimicry
    pub(crate) tls_wrapper: Arc<Mutex<TlsRecordWrapper>>,
    /// WebSocket frame wrapper for protocol mimicry
    pub(crate) websocket_wrapper: Arc<WebSocketFrameWrapper>,
    /// DNS-over-HTTPS tunnel for protocol mimicry
    pub(crate) doh_tunnel: Arc<DohTunnel>,
    /// Obfuscation statistics (padding bytes, timing delays, wrapped packets)
    pub(crate) obfuscation_stats: Arc<Mutex<ObfuscationStats>>,
    /// Available files for seeding (root_hash -> (metadata, file_path))
    pub(crate) available_files:
        Arc<DashMap<[u8; 32], (crate::node::transfer::FileMetadata, PathBuf)>>,
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
    ///
    /// # Errors
    ///
    /// Returns an error if key generation fails due to insufficient entropy.
    pub async fn new_random() -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, NodeConfig::default()).await
    }

    /// Create node with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if key generation fails due to insufficient entropy.
    pub async fn new_with_config(config: NodeConfig) -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, config).await
    }

    /// Create node with specific port (useful for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if key generation fails due to insufficient entropy.
    pub async fn new_random_with_port(port: u16) -> Result<Self> {
        use std::net::{Ipv4Addr, SocketAddrV4};

        let identity = Identity::generate()?;
        let config = NodeConfig {
            listen_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
            ..NodeConfig::default()
        };
        Self::new_from_identity(identity, config).await
    }

    /// Create node from existing identity
    pub async fn new_from_identity(identity: Identity, config: NodeConfig) -> Result<Self> {
        use crate::node::ip_reputation::IpReputationConfig;
        use crate::node::security_monitor::SecurityMonitorConfig;

        let rate_limiter = RateLimiter::new(config.rate_limiting.clone());
        let ip_reputation = IpReputationSystem::new(IpReputationConfig::default());
        let security_monitor = SecurityMonitor::new(SecurityMonitorConfig::default());

        // Initialize obfuscation wrappers
        let tls_wrapper = TlsRecordWrapper::new();
        let websocket_wrapper = WebSocketFrameWrapper::new(false); // Server mode (no masking)
        let doh_tunnel = DohTunnel::new("https://1.1.1.1/dns-query".to_string());
        let obfuscation_stats = ObfuscationStats::default();

        let inner = NodeInner {
            identity: Arc::new(identity),
            config,
            sessions: Arc::new(DashMap::new()),
            routing: Arc::new(RoutingTable::new()),
            transfers: Arc::new(DashMap::new()),
            pending_handshakes: Arc::new(DashMap::new()),
            pending_pings: Arc::new(DashMap::new()),
            pending_migrations: Arc::new(DashMap::new()),
            pending_chunks: Arc::new(DashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            transport: Arc::new(Mutex::new(None)),
            discovery: Arc::new(Mutex::new(None)),
            rate_limiter: Arc::new(rate_limiter),
            ip_reputation: Arc::new(ip_reputation),
            security_monitor: Arc::new(security_monitor),
            tls_wrapper: Arc::new(Mutex::new(tls_wrapper)),
            websocket_wrapper: Arc::new(websocket_wrapper),
            doh_tunnel: Arc::new(doh_tunnel),
            obfuscation_stats: Arc::new(Mutex::new(obfuscation_stats)),
            available_files: Arc::new(DashMap::new()),
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transport has not been initialized (node not started)
    /// - The transport fails to return a local address
    pub async fn listen_addr(&self) -> Result<SocketAddr> {
        let transport = self.inner.transport.lock().await;
        match transport.as_ref() {
            Some(t) => {
                let mut addr = t.local_addr().map_err(|e| {
                    NodeError::Transport(format!("Failed to get local address: {e}").into())
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
            None => Err(NodeError::InvalidState("Transport not initialized".into())),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Lifecycle Methods
// ═══════════════════════════════════════════════════════════════════════════

impl Node {
    /// Start the node
    ///
    /// Initializes the transport layer, discovery manager, and packet receive loop.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The node is already running
    /// - The transport fails to bind to the configured address
    /// - The discovery manager fails to initialize or start
    pub async fn start(&self) -> Result<()> {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(NodeError::InvalidState(std::borrow::Cow::Borrowed(
                "Node already running",
            )));
        }

        tracing::info!(
            "Starting node {} on {}",
            hex::encode(self.node_id()),
            self.inner.config.listen_addr
        );

        // Initialize transport
        let transport = AsyncUdpTransport::bind(self.inner.config.listen_addr)
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to bind transport: {e}").into()))?;
        let transport = Arc::new(transport);
        *self.inner.transport.lock().await = Some(Arc::clone(&transport));

        // Initialize discovery
        let node_id_bytes = wraith_discovery::dht::NodeId::from_bytes(*self.node_id());
        let mut discovery_config =
            DiscoveryConfigInternal::new(node_id_bytes, self.inner.config.listen_addr);
        discovery_config.nat_detection_enabled = self.inner.config.discovery.enable_nat_traversal;
        discovery_config.relay_enabled = self.inner.config.discovery.enable_relay;

        let discovery = DiscoveryManager::new(discovery_config).await.map_err(|e| {
            NodeError::Discovery(format!("Failed to create discovery manager: {e}").into())
        })?;
        let discovery = Arc::new(discovery);
        *self.inner.discovery.lock().await = Some(Arc::clone(&discovery));
        discovery
            .start()
            .await
            .map_err(|e| NodeError::Discovery(format!("Failed to start discovery: {e}").into()))?;

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
    ///
    /// Closes all sessions, clears the routing table, and shuts down the transport.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not currently running.
    pub async fn stop(&self) -> Result<()> {
        if self
            .inner
            .running
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(NodeError::InvalidState(std::borrow::Cow::Borrowed(
                "Node not running",
            )));
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
        if let Some(transport) = self.inner.transport.lock().await.take()
            && let Err(e) = transport.close().await
        {
            tracing::warn!("Error closing transport: {}", e);
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
    /// Discover peer addresses via DHT/STUN/relay
    pub async fn discover_peer(&self, peer_id: &PeerId) -> Result<Vec<SocketAddr>> {
        let discovery = self.inner.discovery.lock().await;
        let discovery =
            discovery
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?;

        // Convert PeerId (Ed25519 public key) to DHT NodeId
        let dht_node_id = wraith_discovery::dht::NodeId::from_bytes(*peer_id);

        // Use DiscoveryManager to find peer
        match discovery.connect_to_peer(dht_node_id).await {
            Ok(peer_connection) => {
                tracing::info!(
                    "Discovered peer {} at {} via {}",
                    hex::encode(&peer_id[..8]),
                    peer_connection.addr,
                    peer_connection.connection_type
                );
                Ok(vec![peer_connection.addr])
            }
            Err(e) => {
                tracing::warn!(
                    "DHT lookup failed for peer {}: {}",
                    hex::encode(&peer_id[..8]),
                    e
                );
                Err(NodeError::Discovery(
                    format!("Peer discovery failed: {e}").into(),
                ))
            }
        }
    }

    /// Establish session with peer (via DHT lookup)
    pub async fn establish_session(&self, peer_id: &PeerId) -> Result<SessionId> {
        if let Some(connection) = self.inner.sessions.get(peer_id) {
            return Ok(connection.session_id);
        }

        // Discover peer addresses via DHT/STUN/relay
        let addrs = self.discover_peer(peer_id).await?;
        if addrs.is_empty() {
            return Err(NodeError::PeerNotFound(*peer_id));
        }

        // Try first address (in future, could try multiple in parallel)
        let peer_addr = addrs[0];
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

        // Announce peer to DHT (best-effort, don't fail session if announcement fails)
        self.announce_peer_to_dht(&peer_id, peer_addr).await;

        Ok(session_id)
    }

    /// Announce peer to DHT (best-effort)
    async fn announce_peer_to_dht(&self, peer_id: &PeerId, peer_addr: SocketAddr) {
        if let Some(discovery) = self.inner.discovery.lock().await.as_ref() {
            let dht = discovery.dht();
            if let Ok(mut dht_write) = dht.try_write() {
                let node_id = wraith_discovery::dht::NodeId::from_bytes(*peer_id);
                let dht_peer = wraith_discovery::dht::DhtPeer::new(node_id, peer_addr);
                let routing_table = dht_write.routing_table_mut();
                if let Err(e) = routing_table.insert(dht_peer) {
                    tracing::debug!(
                        "Failed to announce peer {} to DHT: {}",
                        hex::encode(&peer_id[..8]),
                        e
                    );
                } else {
                    tracing::debug!("Announced peer {} to DHT", hex::encode(&peer_id[..8]));
                }
            }
        }
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

    /// Get connection statistics for a peer
    ///
    /// Returns the connection statistics for the specified peer, or `None` if
    /// no active session exists with that peer.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer ID (32-byte Ed25519 public key)
    ///
    /// # Returns
    ///
    /// `Some(ConnectionStats)` if a session exists, `None` otherwise.
    pub fn get_connection_stats(
        &self,
        peer_id: &PeerId,
    ) -> Option<crate::node::session::ConnectionStats> {
        self.inner
            .sessions
            .get(peer_id)
            .map(|connection| connection.stats.clone())
    }

    /// Get the timestamp when a session was established
    ///
    /// Returns the `SystemTime` when the session with the specified peer was established,
    /// or `None` if no active session exists with that peer.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer ID (32-byte Ed25519 public key)
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if a session exists, `None` otherwise.
    pub fn get_session_established_at(&self, peer_id: &PeerId) -> Option<std::time::SystemTime> {
        self.inner
            .sessions
            .get(peer_id)
            .map(|connection| connection.established_at)
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
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| NodeError::Io(e.to_string()))?
            .len();
        if file_size == 0 {
            return Err(NodeError::InvalidState("Cannot send empty file".into()));
        }

        let chunk_size = self.inner.config.transfer.chunk_size;
        let tree_hash =
            compute_tree_hash(file_path, chunk_size).map_err(|e| NodeError::Io(e.to_string()))?;
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

    /// Send file to multiple peers using multi-peer coordination
    ///
    /// Establishes sessions with all peers and uses the MultiPeerCoordinator
    /// to intelligently assign chunks for parallel upload.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to send
    /// * `peer_ids` - List of peer IDs to send to
    ///
    /// # Returns
    ///
    /// Transfer ID for tracking progress
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File doesn't exist or can't be read
    /// - No peers provided
    /// - Session establishment fails with all peers
    pub async fn send_file_to_peers(
        &self,
        file_path: impl AsRef<Path>,
        peer_ids: &[PeerId],
    ) -> Result<TransferId> {
        if peer_ids.is_empty() {
            return Err(NodeError::InvalidState(
                "No peers provided for multi-peer transfer".into(),
            ));
        }

        let file_path = file_path.as_ref();
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| NodeError::Io(e.to_string()))?
            .len();
        if file_size == 0 {
            return Err(NodeError::InvalidState("Cannot send empty file".into()));
        }

        tracing::info!(
            "Starting multi-peer send: {} to {} peers",
            file_path.display(),
            peer_ids.len()
        );

        // Create transfer ID and context
        let transfer_id = Node::generate_transfer_id();
        let chunk_size = self.inner.config.transfer.chunk_size;

        // Compute tree hash
        let tree_hash = wraith_files::tree_hash::compute_tree_hash(file_path, chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        // Create send transfer session
        let mut transfer_session = crate::transfer::TransferSession::new_send(
            transfer_id,
            file_path.to_path_buf(),
            file_size,
            chunk_size,
        );
        transfer_session.start();

        // Add all peers to transfer session
        for peer_id in peer_ids {
            transfer_session.add_peer(*peer_id);
        }

        // Store transfer context
        let context = Arc::new(FileTransferContext::new_send(
            transfer_id,
            Arc::new(tokio::sync::RwLock::new(transfer_session)),
            tree_hash,
        ));
        self.inner.transfers.insert(transfer_id, context.clone());

        // Create multi-peer coordinator
        let strategy = self.inner.config.transfer.chunk_assignment_strategy;
        let coordinator = crate::node::multi_peer::MultiPeerCoordinator::new(strategy);

        // Establish sessions with all peers and add to coordinator
        let mut sessions = Vec::new();
        for peer_id in peer_ids {
            match self.get_or_establish_session(peer_id).await {
                Ok(session) => {
                    coordinator.add_peer(*peer_id, session.peer_addr()).await;
                    sessions.push((*peer_id, session));
                }
                Err(e) => {
                    tracing::warn!("Failed to establish session with peer {:?}: {}", peer_id, e);
                }
            }
        }

        if sessions.is_empty() {
            return Err(NodeError::Transfer(
                "Failed to establish session with any peer".into(),
            ));
        }

        tracing::info!(
            "Established sessions with {} out of {} peers",
            sessions.len(),
            peer_ids.len()
        );

        // Spawn task to coordinate chunk uploads
        let node = self.clone();
        let file_path_buf = file_path.to_path_buf();
        tokio::spawn(async move {
            if let Err(e) = node
                .coordinate_multi_peer_upload(transfer_id, file_path_buf, sessions, coordinator)
                .await
            {
                tracing::error!("Error in multi-peer upload: {}", e);
            }
        });

        Ok(transfer_id)
    }

    /// Coordinate multi-peer upload (internal helper)
    async fn coordinate_multi_peer_upload(
        &self,
        transfer_id: TransferId,
        file_path: PathBuf,
        sessions: Vec<(PeerId, Arc<crate::node::session::PeerConnection>)>,
        coordinator: crate::node::multi_peer::MultiPeerCoordinator,
    ) -> Result<()> {
        let chunk_size = self.inner.config.transfer.chunk_size;
        let mut chunker = wraith_files::chunker::FileChunker::new(&file_path, chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        let total_chunks = chunker.num_chunks();
        let stream_id = ((transfer_id[0] as u16) << 8) | (transfer_id[1] as u16);

        tracing::debug!(
            "Uploading {} chunks across {} peers",
            total_chunks,
            sessions.len()
        );

        // Assign and upload chunks
        for chunk_index in 0..total_chunks {
            // Assign chunk to a peer
            let peer_id = match coordinator.assign_chunk(chunk_index as usize).await {
                Some(id) => id,
                None => {
                    tracing::warn!("No available peer for chunk {}", chunk_index);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };

            // Find the session for this peer
            let session = sessions
                .iter()
                .find(|(id, _)| *id == peer_id)
                .map(|(_, session)| session.clone());

            if let Some(session) = session {
                // Read chunk data
                let chunk_data = chunker
                    .read_chunk_at(chunk_index)
                    .map_err(|e| NodeError::Io(e.to_string()))?;
                let chunk_len = chunk_data.len();

                // Build and send chunk frame
                let chunk_frame = crate::node::file_transfer::build_chunk_frame(
                    stream_id,
                    chunk_index,
                    &chunk_data,
                )?;

                let start = Instant::now();
                if let Err(e) = self.send_encrypted_frame(&session, &chunk_frame).await {
                    tracing::warn!(
                        "Failed to send chunk {} to peer {:?}: {}",
                        chunk_index,
                        peer_id,
                        e
                    );
                    coordinator.reassign_chunk(chunk_index as usize).await;
                    continue;
                }

                // Record success
                let duration = start.elapsed();
                coordinator
                    .record_success(chunk_index as usize, chunk_len as u64, duration)
                    .await;

                // Update progress
                if let Some(context) = self.inner.transfers.get(&transfer_id) {
                    context
                        .transfer_session
                        .write()
                        .await
                        .mark_chunk_transferred(chunk_index, chunk_len);
                }
            }
        }

        tracing::info!(
            "Multi-peer upload complete: {:?} ({} chunks to {} peers)",
            transfer_id,
            total_chunks,
            sessions.len()
        );

        Ok(())
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

    /// Get detailed transfer progress
    pub async fn get_transfer_progress(
        &self,
        transfer_id: &TransferId,
    ) -> Option<crate::node::progress::TransferProgress> {
        let context = self.inner.transfers.get(transfer_id)?;
        let session = context.transfer_session.read().await;

        let bytes_sent = session.bytes_transferred();
        let bytes_total = session.file_size;
        let chunks_sent = session.transferred_count() as usize;
        let chunks_total = session.total_chunks as usize;
        let speed = session.speed().unwrap_or(0.0);

        let mut progress =
            crate::node::progress::TransferProgress::new(*transfer_id, bytes_total, chunks_total);

        progress.update(bytes_sent, chunks_sent, speed);

        // Set status based on session state
        if session.is_complete() {
            progress.status = crate::node::progress::TransferStatus::Complete;
        } else if bytes_sent > 0 {
            progress.status = crate::node::progress::TransferStatus::Transferring;
        }

        Some(progress)
    }

    /// List active transfers
    pub async fn active_transfers(&self) -> Vec<TransferId> {
        self.inner
            .transfers
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Cancel an in-progress transfer
    ///
    /// Removes the transfer from the active transfers map and sends a STREAM_CLOSE
    /// frame to the peer if the transfer has an active session.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - The ID of the transfer to cancel
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transfer was successfully cancelled, or `Err(NodeError::TransferNotFound)`
    /// if the transfer ID does not exist.
    ///
    /// # Errors
    ///
    /// Returns `TransferNotFound` if the transfer ID is not in the active transfers map.
    pub async fn cancel_transfer(&self, transfer_id: &TransferId) -> Result<()> {
        use crate::frame::{FrameBuilder, FrameType};

        // Remove transfer from map
        if let Some((_, _context)) = self.inner.transfers.remove(transfer_id) {
            // Get the session if transfer has one
            let session_opt = self
                .inner
                .sessions
                .iter()
                .find(|_entry| {
                    // Find session that matches the transfer's peer (if any)
                    // For now, we'll just send CLOSE to any active session
                    true
                })
                .map(|entry| entry.value().clone());

            // Send STREAM_CLOSE frame if we have a session
            if let Some(session) = session_opt {
                let stream_id = ((transfer_id[0] as u16) << 8) | (transfer_id[1] as u16);
                let close_frame = FrameBuilder::new()
                    .frame_type(FrameType::StreamClose)
                    .stream_id(stream_id)
                    .build(64) // Minimum size with padding
                    .ok();

                if let Some(frame) = close_frame
                    && let Ok(encrypted) = session.encrypt_frame(&frame).await
                {
                    // Send CLOSE frame (best-effort - don't fail cancellation if send fails)
                    if let Some(transport) = self.inner.transport.lock().await.as_ref() {
                        let _ = transport.send_to(&encrypted, session.peer_addr()).await;
                    }
                }
            }

            tracing::info!("Cancelled transfer: {:?}", hex::encode(&transfer_id[..8]));
            Ok(())
        } else {
            Err(NodeError::TransferNotFound(*transfer_id))
        }
    }

    /// Generate random transfer ID
    ///
    /// # Panics
    ///
    /// Panics if the CSPRNG fails to generate random bytes (extremely unlikely).
    pub(crate) fn generate_transfer_id() -> TransferId {
        let mut id = [0u8; 32];
        getrandom(&mut id).expect("Failed to generate transfer ID");
        id
    }

    /// Send raw data to a peer
    ///
    /// This method establishes a session if needed, then sends the data as a DATA frame
    /// on stream 0. This is useful for sending small messages like chat messages.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer ID (32-byte Ed25519 public key)
    /// * `data` - The data to send (will be encrypted)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the data was sent successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The session cannot be established
    /// - Frame building fails
    /// - Encryption fails
    /// - The transport layer fails to send
    pub async fn send_data(&self, peer_id: &PeerId, data: &[u8]) -> Result<()> {
        use crate::FRAME_HEADER_SIZE;
        use crate::frame::{FrameBuilder, FrameType};

        // Get or establish session
        let connection = self.get_or_establish_session(peer_id).await?;

        // Build data frame
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(0) // Use stream 0 for generic data/chat messages
            .sequence(0)
            .payload(data)
            .build(FRAME_HEADER_SIZE + data.len())
            .map_err(|e| NodeError::Other(format!("Failed to build frame: {e:?}").into()))?;

        // Encrypt and send
        let encrypted = connection.encrypt_frame(&frame).await?;
        let transport = self.get_transport().await?;
        transport
            .send_to(&encrypted, connection.peer_addr())
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to send data: {e}").into()))?;

        tracing::debug!(
            "Sent {} bytes data to peer {:?}",
            data.len(),
            hex::encode(&peer_id[..8])
        );

        Ok(())
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
            .ok_or(NodeError::InvalidState(std::borrow::Cow::Borrowed(
                "Transport not initialized",
            )))
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
