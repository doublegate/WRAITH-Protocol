//! Node implementation - high-level protocol orchestrator

use crate::node::config::NodeConfig;
use crate::node::error::{NodeError, Result};
use crate::node::file_transfer::FileTransferContext;
use crate::node::session::{PeerConnection, PeerId, SessionId};
use crate::transfer::TransferSession;
use crate::{ConnectionId, HandshakePhase, SessionState};
use dashmap::DashMap;
use getrandom::getrandom;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use wraith_crypto::noise::NoiseKeypair;
use wraith_crypto::signatures::SigningKey as Ed25519SigningKey;
use wraith_discovery::{DiscoveryConfig as DiscoveryConfigInternal, DiscoveryManager};
use wraith_files::chunker::FileChunker;
use wraith_files::tree_hash::compute_tree_hash;
use wraith_transport::transport::Transport;
use wraith_transport::udp_async::AsyncUdpTransport;

/// Transfer ID (32-byte unique identifier)
pub type TransferId = [u8; 32];

/// Identity keypair
#[derive(Clone)]
pub struct Identity {
    /// Node ID (derived from Ed25519 public key)
    node_id: [u8; 32],

    /// X25519 keypair for Noise handshakes
    x25519: NoiseKeypair,
}

impl Identity {
    /// Generate random identity
    pub fn generate() -> Result<Self> {
        use rand_core::OsRng;

        // Generate Ed25519 keypair and extract public key as node ID
        let ed25519 = Ed25519SigningKey::generate(&mut OsRng);
        let node_id = ed25519.verifying_key().to_bytes();
        // Note: We don't store the signing key, only use the public key as node ID

        // Generate X25519 keypair for Noise handshakes
        let x25519 = NoiseKeypair::generate()
            .map_err(|e| NodeError::Crypto(wraith_crypto::CryptoError::Handshake(e.to_string())))?;

        Ok(Self { node_id, x25519 })
    }

    /// Get Ed25519 public key (node ID)
    pub fn public_key(&self) -> &[u8; 32] {
        &self.node_id
    }

    /// Get X25519 keypair for Noise
    pub fn x25519_keypair(&self) -> &NoiseKeypair {
        &self.x25519
    }
}

/// Node inner state
pub(crate) struct NodeInner {
    /// Node identity
    pub(crate) identity: Arc<Identity>,

    /// Node configuration
    pub(crate) config: NodeConfig,

    /// Active sessions (peer_id -> connection)
    /// Uses DashMap for lock-free concurrent access
    pub(crate) sessions: Arc<DashMap<PeerId, Arc<PeerConnection>>>,

    /// Active file transfers (transfer_id -> transfer context)
    /// Uses DashMap for lock-free concurrent access
    /// Consolidates transfer session, reassembler, and tree hash
    pub(crate) transfers: Arc<DashMap<TransferId, Arc<FileTransferContext>>>,

    /// Node running state
    pub(crate) running: Arc<AtomicBool>,

    /// Transport layer (initialized on start)
    pub(crate) transport: Arc<Mutex<Option<Arc<AsyncUdpTransport>>>>,

    /// Discovery manager (initialized on start)
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
///
/// # Examples
///
/// ```no_run
/// use wraith_core::node::Node;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create node with random identity
///     let node = Node::new_random().await?;
///
///     println!("Node ID: {:?}", node.node_id());
///
///     // Send file to peer
///     let peer_id = [0u8; 32]; // Peer's public key
///     let transfer_id = node.send_file("document.pdf", &peer_id).await?;
///
///     // Wait for transfer to complete
///     node.wait_for_transfer(transfer_id).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Node {
    pub(crate) inner: Arc<NodeInner>,
}

impl Node {
    /// Create node with random identity
    ///
    /// Uses default configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::node::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let node = Node::new_random().await?;
    /// println!("Node ID: {:?}", node.node_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new_random() -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, NodeConfig::default()).await
    }

    /// Create node with custom configuration
    pub async fn new_with_config(config: NodeConfig) -> Result<Self> {
        let identity = Identity::generate()?;
        Self::new_from_identity(identity, config).await
    }

    /// Create node with random identity and specific port
    ///
    /// Useful for testing to avoid port conflicts. Use port 0 for automatic port selection.
    ///
    /// # Arguments
    ///
    /// * `port` - Port number to bind to (use 0 for OS auto-selection)
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
            transfers: Arc::new(DashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            transport: Arc::new(Mutex::new(None)),
            discovery: Arc::new(Mutex::new(None)),
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Get node's public key (node ID)
    pub fn node_id(&self) -> &[u8; 32] {
        self.inner.identity.public_key()
    }

    /// Get node's identity
    pub fn identity(&self) -> &Arc<Identity> {
        &self.inner.identity
    }

    /// Start the node
    ///
    /// Initializes transport, starts workers, and begins accepting connections.
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

        // 1. Initialize UDP transport
        let transport = AsyncUdpTransport::bind(self.inner.config.listen_addr)
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to bind transport: {}", e)))?;

        let transport = Arc::new(transport);
        *self.inner.transport.lock().await = Some(Arc::clone(&transport));

        tracing::debug!(
            "Transport initialized on {}",
            transport
                .local_addr()
                .map_err(|e| NodeError::Transport(e.to_string()))?
        );

        // 2. Initialize Discovery Manager
        let node_id_bytes = wraith_discovery::dht::NodeId::from_bytes(*self.node_id());
        let discovery_config =
            DiscoveryConfigInternal::new(node_id_bytes, self.inner.config.listen_addr);
        // TODO: Add bootstrap nodes from config

        let discovery = DiscoveryManager::new(discovery_config).await.map_err(|e| {
            NodeError::Discovery(format!("Failed to create discovery manager: {}", e))
        })?;

        let discovery = Arc::new(discovery);
        *self.inner.discovery.lock().await = Some(Arc::clone(&discovery));

        // Start discovery (DHT, NAT detection, relay connections)
        discovery
            .start()
            .await
            .map_err(|e| NodeError::Discovery(format!("Failed to start discovery: {}", e)))?;

        tracing::debug!("Discovery manager started");

        // 3. Start packet receive loop
        let node = self.clone();
        tokio::spawn(async move {
            node.packet_receive_loop().await;
        });

        // 4. Start cover traffic generator if enabled
        if self.inner.config.obfuscation.cover_traffic.enabled {
            let node = self.clone();
            tokio::spawn(async move {
                node.cover_traffic_loop().await;
            });
            tracing::debug!("Cover traffic generator started");
        }

        // TODO: Start worker pool for packet processing
        // TODO: Start connection monitor

        tracing::info!("Node started: {:?}", hex::encode(self.node_id()));

        Ok(())
    }

    /// Stop the node
    ///
    /// Gracefully closes all sessions and stops workers.
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

        // Close transport
        if let Some(transport) = self.inner.transport.lock().await.take() {
            if let Err(e) = transport.close().await {
                tracing::warn!("Error closing transport: {}", e);
            }
        }

        tracing::info!("Node stopped");

        Ok(())
    }

    /// Packet receive loop
    ///
    /// Continuously receives packets from the transport and processes them.
    async fn packet_receive_loop(&self) {
        tracing::debug!("Starting packet receive loop");

        let mut buf = vec![0u8; 65536]; // 64KB buffer for jumbo frames

        loop {
            // Check if node is still running
            if !self.inner.running.load(Ordering::SeqCst) {
                tracing::debug!("Node stopped, exiting receive loop");
                break;
            }

            // Get transport
            let transport = {
                let guard = self.inner.transport.lock().await;
                match guard.as_ref() {
                    Some(t) => Arc::clone(t),
                    None => {
                        tracing::warn!("Transport not initialized, exiting receive loop");
                        break;
                    }
                }
            };

            // Receive packet with timeout
            match tokio::time::timeout(Duration::from_millis(100), transport.recv_from(&mut buf))
                .await
            {
                Ok(Ok((size, from))) => {
                    // Process packet
                    let packet_data = buf[..size].to_vec();
                    let node = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = node.handle_incoming_packet(packet_data, from).await {
                            tracing::debug!("Error handling packet from {}: {}", from, e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error receiving packet: {}", e);
                }
                Err(_) => {
                    // Timeout - continue loop
                    continue;
                }
            }
        }

        tracing::debug!("Packet receive loop terminated");
    }

    /// Cover traffic generation loop
    ///
    /// Sends PAD frames to active sessions at configured rate to provide
    /// traffic analysis resistance.
    async fn cover_traffic_loop(&self) {
        use crate::node::config::CoverTrafficDistribution;

        tracing::debug!("Starting cover traffic generator");

        let config = &self.inner.config.obfuscation.cover_traffic;
        let rate = config.rate;

        loop {
            // Check if node is still running
            if !self.inner.running.load(Ordering::SeqCst) {
                tracing::debug!("Node stopped, exiting cover traffic loop");
                break;
            }

            // Calculate delay based on distribution
            let delay = match config.distribution {
                CoverTrafficDistribution::Constant => {
                    // Fixed interval based on rate
                    if rate > 0.0 {
                        Duration::from_secs_f64(1.0 / rate)
                    } else {
                        Duration::from_secs(1)
                    }
                }
                CoverTrafficDistribution::Poisson => {
                    // Exponential inter-arrival time (Poisson process)
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let u: f64 = rng.r#gen();
                    let delay_secs = -u.ln() / rate;
                    Duration::from_secs_f64(delay_secs.min(10.0)) // Cap at 10 seconds
                }
                CoverTrafficDistribution::Uniform { min_ms, max_ms } => {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let delay_ms = rng.gen_range(min_ms..=max_ms);
                    Duration::from_millis(delay_ms)
                }
            };

            // Wait for next send time
            tokio::time::sleep(delay).await;

            // Send cover traffic to all active sessions
            for entry in self.inner.sessions.iter() {
                let connection = entry.value();

                // Generate random padding data
                let mut pad_data = vec![0u8; 64];
                if getrandom(&mut pad_data).is_err() {
                    continue;
                }

                // Create PAD frame using FrameBuilder
                let frame_bytes = match crate::frame::FrameBuilder::new()
                    .frame_type(crate::frame::FrameType::Pad)
                    .stream_id(0) // No stream ID for PAD frames
                    .payload(&pad_data)
                    .build(128) // 128-byte PAD frame
                {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };

                // Send encrypted PAD frame (non-blocking, ignore errors)
                let connection = Arc::clone(connection);
                let node = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = node.send_encrypted_frame(&connection, &frame_bytes).await {
                        tracing::trace!("Cover traffic send error: {}", e);
                    }
                });
            }

            tracing::trace!("Sent cover traffic to active sessions");
        }

        tracing::debug!("Cover traffic loop terminated");
    }

    /// Dispatch a frame to the appropriate handler
    ///
    /// Parses the frame bytes and routes to the correct handler based on frame type.
    async fn dispatch_frame(&self, frame_bytes: Vec<u8>) -> Result<()> {
        // Parse the frame
        let frame = crate::frame::Frame::parse(&frame_bytes)
            .map_err(|e| NodeError::Other(format!("Failed to parse frame: {}", e)))?;

        tracing::debug!(
            "Received {} frame with {} byte payload",
            format!("{:?}", frame.frame_type()),
            frame.payload().len()
        );

        // Route frame to appropriate handler based on frame type
        use crate::frame::FrameType;
        match frame.frame_type() {
            FrameType::StreamOpen => self.handle_stream_open_frame(frame).await,
            FrameType::Data => self.handle_data_frame(frame).await,
            FrameType::StreamClose => {
                tracing::debug!("Received StreamClose frame");
                Ok(())
            }
            _ => {
                tracing::debug!("Unhandled frame type: {:?}", frame.frame_type());
                Ok(())
            }
        }
    }

    /// Handle incoming packet with obfuscation unwrapping
    ///
    /// Unwraps protocol mimicry, decrypts, and routes packets to appropriate handlers.
    async fn handle_incoming_packet(&self, data: Vec<u8>, from: SocketAddr) -> Result<()> {
        // 1. Unwrap protocol mimicry (TLS/WebSocket/DoH)
        let unwrapped = self.unwrap_protocol(&data)?;

        // Find session for this peer address
        let connection = self
            .inner
            .sessions
            .iter()
            .find(|entry| entry.value().peer_addr == from)
            .map(|entry| Arc::clone(entry.value()));

        if let Some(conn) = connection {
            // 2. Decrypt the packet (padding is stripped during decryption or frame parsing)
            match conn.decrypt_frame(&unwrapped).await {
                Ok(frame_bytes) => {
                    let node = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = node.dispatch_frame(frame_bytes).await {
                            tracing::warn!("Error handling frame: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to decrypt packet from {}: {}", from, e);
                }
            }
        } else {
            // No established session - could be handshake initiation
            tracing::trace!("Received {} bytes from unknown peer {}", data.len(), from);
            // TODO: Handle handshake initiation
        }

        Ok(())
    }

    /// Send encrypted frame to peer with obfuscation
    ///
    /// Encrypts the frame data, applies padding, timing delay, and protocol
    /// mimicry, then sends it over the transport.
    ///
    /// # Arguments
    ///
    /// * `connection` - The peer connection to send to
    /// * `frame_bytes` - Serialized frame data
    ///
    /// # Errors
    ///
    /// Returns error if encryption, obfuscation, or transmission fails.
    #[allow(dead_code)] // Infrastructure for Session 3.2+
    async fn send_encrypted_frame(
        &self,
        connection: &crate::node::session::PeerConnection,
        frame_bytes: &[u8],
    ) -> Result<()> {
        // 1. Encrypt the frame
        let encrypted = connection.encrypt_frame(frame_bytes).await?;
        let encrypted_len = encrypted.len(); // Save length before moving

        // 2. Apply obfuscation (padding)
        let mut obfuscated = encrypted;
        self.apply_obfuscation(&mut obfuscated)?;

        // 3. Wrap in protocol mimicry (TLS/WebSocket/DoH)
        let wrapped = self.wrap_protocol(&obfuscated)?;

        // 4. Apply timing delay
        let delay = self.get_timing_delay();
        if !delay.is_zero() {
            tracing::trace!("Applying timing delay: {:?}", delay);
            tokio::time::sleep(delay).await;
        }

        // 5. Get transport and send
        let transport = {
            let guard = self.inner.transport.lock().await;
            guard
                .as_ref()
                .ok_or_else(|| NodeError::InvalidState("Transport not initialized".to_string()))?
                .clone()
        };

        // Send obfuscated packet
        transport
            .send_to(&wrapped, connection.peer_addr)
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to send packet: {}", e)))?;

        tracing::trace!(
            "Sent {} obfuscated bytes to {} (original: {} encrypted)",
            wrapped.len(),
            connection.peer_addr,
            encrypted_len
        );

        Ok(())
    }

    /// Check if node is running
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    /// Establish session with peer
    ///
    /// Performs Noise_XX handshake and creates encrypted session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::node::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peer_id = [0u8; 32];
    /// let session_id = node.establish_session(&peer_id).await?;
    /// println!("Session established: {:?}", session_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn establish_session(&self, peer_id: &PeerId) -> Result<SessionId> {
        // Check if session already exists
        if let Some(connection) = self.inner.sessions.get(peer_id) {
            return Ok(connection.session_id);
        }

        // Get transport
        let transport = {
            let guard = self.inner.transport.lock().await;
            guard
                .as_ref()
                .ok_or_else(|| NodeError::InvalidState("Transport not initialized".to_string()))?
                .clone()
        };

        // TODO: Lookup peer address via DHT
        // For now, use a placeholder address
        let peer_addr: SocketAddr = "127.0.0.1:8421".parse().unwrap();

        tracing::info!(
            "Establishing session with peer {} at {}",
            hex::encode(peer_id),
            peer_addr
        );

        // Perform Noise_XX handshake as initiator
        let (crypto, session_id) = crate::node::session::perform_handshake_initiator(
            self.inner.identity.x25519_keypair(),
            peer_addr,
            transport.as_ref(),
        )
        .await?;

        // Derive connection ID from session ID
        let mut connection_id_bytes = [0u8; 8];
        connection_id_bytes.copy_from_slice(&session_id[..8]);
        let connection_id = ConnectionId::from_bytes(connection_id_bytes);

        // Create connection
        let connection =
            PeerConnection::new(session_id, *peer_id, peer_addr, connection_id, crypto);

        // Transition through handshake states to established
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .await?;
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        // Store session
        let connection_arc = Arc::new(connection);
        self.inner.sessions.insert(*peer_id, connection_arc);

        tracing::info!(
            "Session established with peer {}, session: {}",
            hex::encode(peer_id),
            hex::encode(&session_id[..8])
        );

        Ok(session_id)
    }

    /// Get or establish session with peer
    pub async fn get_or_establish_session(&self, peer_id: &PeerId) -> Result<Arc<PeerConnection>> {
        // Try to get existing session
        if let Some(connection) = self.inner.sessions.get(peer_id) {
            return Ok(Arc::clone(connection.value()));
        }

        // Establish new session
        let _session_id = self.establish_session(peer_id).await?;

        // Retrieve the newly created session
        self.inner
            .sessions
            .get(peer_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(NodeError::SessionNotFound(*peer_id))
    }

    /// Close session with peer
    pub async fn close_session(&self, peer_id: &PeerId) -> Result<()> {
        if let Some((_, connection)) = self.inner.sessions.remove(peer_id) {
            connection.transition_to(SessionState::Closed).await?;
            tracing::info!("Session closed with peer {:?}", peer_id);
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

    /// Send file to peer
    ///
    /// Chunks file, computes tree hash, and transfers to peer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::node::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peer_id = [0u8; 32];
    /// let transfer_id = node.send_file("document.pdf", &peer_id).await?;
    /// node.wait_for_transfer(transfer_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_file(
        &self,
        file_path: impl AsRef<Path>,
        peer_id: &PeerId,
    ) -> Result<TransferId> {
        let file_path = file_path.as_ref();

        // 1. Get file metadata
        let file_size = std::fs::metadata(file_path).map_err(NodeError::Io)?.len();

        if file_size == 0 {
            return Err(NodeError::InvalidState(
                "Cannot send empty file".to_string(),
            ));
        }

        let chunk_size = self.inner.config.transfer.chunk_size;

        // 2. Compute tree hash for integrity verification
        tracing::debug!(
            "Computing BLAKE3 tree hash for {} ({} bytes, chunk_size={})",
            file_path.display(),
            file_size,
            chunk_size
        );

        let tree_hash = compute_tree_hash(file_path, chunk_size).map_err(NodeError::Io)?;

        // 3. Generate transfer ID
        let transfer_id = Self::generate_transfer_id();

        // 4. Create transfer session
        let mut transfer =
            TransferSession::new_send(transfer_id, file_path.to_path_buf(), file_size, chunk_size);

        transfer.start(); // Start immediately

        // Store transfer context (consolidates session and tree hash)
        let transfer_arc = Arc::new(RwLock::new(transfer));
        let context = Arc::new(FileTransferContext::new_send(
            transfer_id,
            Arc::clone(&transfer_arc),
            tree_hash.clone(),
        ));
        self.inner
            .transfers
            .insert(transfer_id, Arc::clone(&context));

        // 5. Establish session with peer
        let connection = self.get_or_establish_session(peer_id).await?;

        tracing::info!(
            "Starting file transfer {:?} to peer {:?} ({} bytes, {} chunks)",
            hex::encode(&transfer_id[..8]),
            hex::encode(peer_id),
            file_size,
            file_size.div_ceil(chunk_size as u64)
        );

        // 6. Send file metadata (StreamOpen frame)
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

        tracing::debug!(
            "Sent file metadata for transfer {:?} (stream_id={})",
            hex::encode(&transfer_id[..8]),
            stream_id
        );

        // 7. Spawn task to send chunks
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

    /// Send file chunks (called from spawned task)
    async fn send_file_chunks(
        &self,
        transfer_id: TransferId,
        file_path: PathBuf,
        stream_id: u16,
        connection: Arc<crate::node::session::PeerConnection>,
    ) -> Result<()> {
        // Get transfer context
        let context = self
            .inner
            .transfers
            .get(&transfer_id)
            .ok_or(NodeError::TransferNotFound(transfer_id))?
            .clone();

        // Create chunker
        let mut chunker = FileChunker::new(&file_path, self.inner.config.transfer.chunk_size)
            .map_err(NodeError::Io)?;

        let total_chunks = chunker.num_chunks();

        tracing::debug!(
            "Sending {} chunks for transfer {:?}",
            total_chunks,
            hex::encode(&transfer_id[..8])
        );

        // Send each chunk
        for chunk_index in 0..total_chunks {
            // Read chunk
            let chunk_data = chunker.read_chunk_at(chunk_index).map_err(NodeError::Io)?;

            let chunk_len = chunk_data.len();

            // Verify chunk hash against tree hash
            if chunk_index < context.tree_hash.chunks.len() as u64 {
                let computed_hash = blake3::hash(&chunk_data);
                if computed_hash.as_bytes() != &context.tree_hash.chunks[chunk_index as usize] {
                    tracing::error!("Chunk {} hash mismatch during send", chunk_index);
                    return Err(NodeError::InvalidState(
                        "Chunk hash verification failed".to_string(),
                    ));
                }
            }

            // Build chunk frame
            let chunk_frame =
                crate::node::file_transfer::build_chunk_frame(stream_id, chunk_index, &chunk_data)?;

            // Send encrypted frame
            self.send_encrypted_frame(&connection, &chunk_frame).await?;

            // Update transfer progress
            {
                let mut transfer = context.transfer_session.write().await;
                transfer.mark_chunk_transferred(chunk_index, chunk_len);
            }

            tracing::trace!(
                "Sent chunk {}/{} for transfer {:?} ({} bytes)",
                chunk_index + 1,
                total_chunks,
                hex::encode(&transfer_id[..8]),
                chunk_len
            );
        }

        tracing::info!(
            "File transfer {:?} completed ({} chunks sent)",
            hex::encode(&transfer_id[..8]),
            total_chunks
        );

        Ok(())
    }

    /// Handle StreamOpen frame (file transfer metadata)
    async fn handle_stream_open_frame(&self, frame: crate::frame::Frame<'_>) -> Result<()> {
        // Parse metadata from payload
        let metadata = crate::node::file_transfer::FileMetadata::deserialize(frame.payload())?;

        tracing::info!(
            "Received file transfer request: {} ({} bytes, {} chunks, transfer_id={:?})",
            metadata.file_name,
            metadata.file_size,
            metadata.total_chunks,
            hex::encode(&metadata.transfer_id[..8])
        );

        // Create receive transfer session
        let mut transfer = TransferSession::new_receive(
            metadata.transfer_id,
            std::path::PathBuf::from(&metadata.file_name),
            metadata.file_size,
            metadata.chunk_size as usize,
        );

        transfer.start();

        // Create file reassembler
        let reassembler = wraith_files::chunker::FileReassembler::new(
            &metadata.file_name,
            metadata.file_size,
            metadata.chunk_size as usize,
        )
        .map_err(NodeError::Io)?;

        // Create tree hash (just root for now - we'll build full tree from chunks)
        let tree_hash = wraith_files::tree_hash::FileTreeHash {
            root: metadata.root_hash,
            chunks: Vec::new(), // Will be populated as chunks arrive
        };

        // Store consolidated transfer context
        let context = Arc::new(FileTransferContext::new_receive(
            metadata.transfer_id,
            Arc::new(RwLock::new(transfer)),
            Arc::new(Mutex::new(reassembler)),
            tree_hash,
        ));
        self.inner.transfers.insert(metadata.transfer_id, context);

        tracing::debug!(
            "Initialized receive session for transfer {:?}",
            hex::encode(&metadata.transfer_id[..8])
        );

        Ok(())
    }

    /// Handle Data frame (file chunk)
    async fn handle_data_frame(&self, frame: crate::frame::Frame<'_>) -> Result<()> {
        // Extract chunk index from sequence number
        let chunk_index = frame.sequence() as u64;
        let chunk_data = frame.payload();

        tracing::trace!(
            "Received chunk {} ({} bytes, stream_id={})",
            chunk_index,
            chunk_data.len(),
            frame.stream_id()
        );

        // Find transfer by stream_id (derived from transfer_id)
        let mut matched_context = None;
        for entry in self.inner.transfers.iter() {
            // Derive stream_id from transfer_id (same as in send_file)
            let transfer_id = entry.key();
            let stream_id = ((transfer_id[0] as u16) << 8) | (transfer_id[1] as u16);

            if stream_id == frame.stream_id() {
                matched_context = Some(entry.value().clone());
                break;
            }
        }

        let context = matched_context.ok_or_else(|| {
            NodeError::InvalidState(format!(
                "No transfer found for stream_id {}",
                frame.stream_id()
            ))
        })?;

        let transfer_id = context.transfer_id;

        // Write chunk to reassembler
        if let Some(reassembler_arc) = &context.reassembler {
            let mut reassembler = reassembler_arc.lock().await;
            reassembler
                .write_chunk(chunk_index, chunk_data)
                .map_err(NodeError::Io)?;

            tracing::trace!(
                "Wrote chunk {} to reassembler for transfer {:?}",
                chunk_index,
                hex::encode(&transfer_id[..8])
            );
        } else {
            return Err(NodeError::InvalidState(format!(
                "No reassembler found for transfer {:?}",
                hex::encode(&transfer_id[..8])
            )));
        }

        // Verify chunk hash
        let tree_hash = &context.tree_hash;
        if chunk_index < tree_hash.chunks.len() as u64 {
            let computed_hash = blake3::hash(chunk_data);
            if computed_hash.as_bytes() != &tree_hash.chunks[chunk_index as usize] {
                tracing::error!(
                    "Chunk {} hash mismatch for transfer {:?}",
                    chunk_index,
                    hex::encode(&transfer_id[..8])
                );
                return Err(NodeError::InvalidState(
                    "Chunk hash verification failed".to_string(),
                ));
            }
        }

        // Update transfer progress
        let mut transfer = context.transfer_session.write().await;
        transfer.mark_chunk_transferred(chunk_index, chunk_data.len());

        // Check if transfer is complete
        if transfer.is_complete() {
            tracing::info!(
                "File transfer {:?} completed successfully ({} bytes received)",
                hex::encode(&transfer_id[..8]),
                transfer.file_size
            );
        }

        Ok(())
    }

    /// Wait for transfer to complete
    pub async fn wait_for_transfer(&self, transfer_id: TransferId) -> Result<()> {
        loop {
            if let Some(context) = self.inner.transfers.get(&transfer_id) {
                let transfer_guard = context.transfer_session.read().await;
                if transfer_guard.is_complete() {
                    return Ok(());
                }
            } else {
                return Err(NodeError::TransferNotFound(transfer_id));
            }

            // Wait before checking again
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

        // Start node
        node.start().await.unwrap();
        assert!(node.is_running());

        // Cannot start twice
        assert!(node.start().await.is_err());

        // Stop node
        node.stop().await.unwrap();
        assert!(!node.is_running());

        // Cannot stop twice
        assert!(node.stop().await.is_err());
    }

    #[tokio::test]
    #[ignore = "TODO(Session 3.4): Requires two-node end-to-end setup"]
    async fn test_session_establishment() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer_id = [42u8; 32];
        let session_id = node.establish_session(&peer_id).await.unwrap();

        assert_eq!(session_id.len(), 32);

        // Verify session exists
        let sessions = node.active_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], peer_id);

        node.stop().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "TODO(Session 3.4): Requires two-node end-to-end setup"]
    async fn test_session_close() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer_id = [42u8; 32];
        node.establish_session(&peer_id).await.unwrap();

        // Close session
        node.close_session(&peer_id).await.unwrap();

        // Verify session removed
        let sessions = node.active_sessions().await;
        assert_eq!(sessions.len(), 0);

        node.stop().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "TODO(Session 3.4): Requires two-node end-to-end setup"]
    async fn test_get_or_establish_session() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer_id = [42u8; 32];

        // First call establishes new session
        let conn1 = node.get_or_establish_session(&peer_id).await.unwrap();

        // Second call returns existing session
        let conn2 = node.get_or_establish_session(&peer_id).await.unwrap();

        assert_eq!(conn1.session_id, conn2.session_id);

        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_active_sessions_empty() {
        let node = Node::new_random().await.unwrap();
        let sessions = node.active_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_transfer_id_generation() {
        let id1 = Node::generate_transfer_id();
        let id2 = Node::generate_transfer_id();

        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        assert_ne!(id1, id2); // Should be unique
    }

    #[tokio::test]
    async fn test_frame_encryption_roundtrip() {
        use crate::FRAME_HEADER_SIZE;
        use crate::frame::{FrameBuilder, FrameType};
        use crate::node::session::PeerConnection;
        use wraith_crypto::aead::SessionCrypto;

        // Create two peer connections with swapped keys
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = crate::ConnectionId::from_bytes([3u8; 8]);

        let send_key = [4u8; 32];
        let recv_key = [5u8; 32];
        let chain_key = [6u8; 32];

        let alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
        let bob_crypto = SessionCrypto::new(recv_key, send_key, &chain_key);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, bob_crypto);

        // Build a frame
        let payload = b"Hello, encrypted WRAITH!";
        let frame_bytes = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .offset(0)
            .payload(payload)
            .build(FRAME_HEADER_SIZE + payload.len())
            .unwrap();

        // Encrypt the frame
        let encrypted = alice.encrypt_frame(&frame_bytes).await.unwrap();

        // Verify encrypted size is larger (includes auth tag)
        assert!(encrypted.len() > frame_bytes.len());

        // Decrypt the frame
        let decrypted = bob.decrypt_frame(&encrypted).await.unwrap();

        // Parse the decrypted frame
        let parsed_frame = crate::frame::Frame::parse(&decrypted).unwrap();

        // Verify frame contents
        assert_eq!(parsed_frame.frame_type(), FrameType::Data);
        assert_eq!(parsed_frame.stream_id(), 42);
        assert_eq!(parsed_frame.sequence(), 1000);
        assert_eq!(parsed_frame.offset(), 0);
        assert_eq!(parsed_frame.payload(), payload);
    }

    #[tokio::test]
    async fn test_encrypted_frame_tampering_detection() {
        use crate::FRAME_HEADER_SIZE;
        use crate::frame::{FrameBuilder, FrameType};
        use crate::node::session::PeerConnection;
        use wraith_crypto::aead::SessionCrypto;

        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = crate::ConnectionId::from_bytes([3u8; 8]);

        let send_key = [4u8; 32];
        let recv_key = [5u8; 32];
        let chain_key = [6u8; 32];

        let alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
        let bob_crypto = SessionCrypto::new(recv_key, send_key, &chain_key);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, bob_crypto);

        // Build and encrypt a frame
        let payload = b"Secret data";
        let frame_bytes = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(100)
            .sequence(1)
            .payload(payload)
            .build(FRAME_HEADER_SIZE + payload.len())
            .unwrap();

        let mut encrypted = alice.encrypt_frame(&frame_bytes).await.unwrap();

        // Tamper with the encrypted data
        if let Some(byte) = encrypted.get_mut(10) {
            *byte ^= 0xFF; // Flip all bits
        }

        // Decryption should fail due to authentication tag mismatch
        let result = bob.decrypt_frame(&encrypted).await;
        assert!(
            result.is_err(),
            "Tampered ciphertext should fail decryption"
        );
    }
}
