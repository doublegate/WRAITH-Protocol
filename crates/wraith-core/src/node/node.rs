//! Node implementation - high-level protocol orchestrator

use crate::node::config::NodeConfig;
use crate::node::error::{NodeError, Result};
use crate::node::session::{PeerConnection, PeerId, SessionId};
use crate::transfer::TransferSession;
use crate::{ConnectionId, HandshakePhase, SessionState};
use getrandom::getrandom;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use wraith_crypto::aead::SessionCrypto;
use wraith_crypto::noise::NoiseKeypair;
use wraith_crypto::signatures::SigningKey as Ed25519SigningKey;
use wraith_files::chunker::FileChunker;
use wraith_files::tree_hash::compute_tree_hash;

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
    pub(crate) sessions: Arc<RwLock<HashMap<PeerId, Arc<PeerConnection>>>>,

    /// Active transfers (transfer_id -> transfer session)
    pub(crate) transfers: Arc<RwLock<HashMap<TransferId, Arc<RwLock<TransferSession>>>>>,

    /// Node running state
    pub(crate) running: Arc<AtomicBool>,
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

    /// Create node from existing identity
    pub async fn new_from_identity(identity: Identity, config: NodeConfig) -> Result<Self> {
        let inner = NodeInner {
            identity: Arc::new(identity),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            transfers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
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

        tracing::info!("Node started: {:?}", self.node_id());

        // TODO: Initialize transport layer
        // TODO: Start worker threads
        // TODO: Start discovery
        // TODO: Start connection monitor

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
        let sessions = self.inner.sessions.write().await;
        for (peer_id, connection) in sessions.iter() {
            tracing::debug!("Closing session with peer {:?}", peer_id);
            if let Err(e) = connection.transition_to(SessionState::Closed).await {
                tracing::warn!("Error closing session: {}", e);
            }
        }

        tracing::info!("Node stopped");

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
        {
            let sessions = self.inner.sessions.read().await;
            if let Some(connection) = sessions.get(peer_id) {
                return Ok(connection.session_id);
            }
        }

        // TODO: Lookup peer address (DHT or config)
        let peer_addr: SocketAddr = "127.0.0.1:8421".parse().unwrap(); // Placeholder

        // TODO: Perform Noise_XX handshake
        // For now, create a mock session
        let session_id = Self::generate_session_id();
        let connection_id = ConnectionId::from_bytes([0u8; 8]); // Placeholder

        // Create mock crypto (in real impl, this comes from handshake)
        let crypto = SessionCrypto::new([1u8; 32], [2u8; 32], &[3u8; 32]);

        // Create connection
        let connection =
            PeerConnection::new(session_id, *peer_id, peer_addr, connection_id, crypto);

        // Transition to established
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .await?;
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        // Store session
        let connection_arc = Arc::new(connection);
        self.inner
            .sessions
            .write()
            .await
            .insert(*peer_id, connection_arc);

        tracing::info!("Session established with peer {:?}", peer_id);

        Ok(session_id)
    }

    /// Get or establish session with peer
    pub async fn get_or_establish_session(&self, peer_id: &PeerId) -> Result<Arc<PeerConnection>> {
        // Try to get existing session
        {
            let sessions = self.inner.sessions.read().await;
            if let Some(connection) = sessions.get(peer_id) {
                return Ok(Arc::clone(connection));
            }
        }

        // Establish new session
        let _session_id = self.establish_session(peer_id).await?;

        // Retrieve the newly created session
        let sessions = self.inner.sessions.read().await;
        sessions
            .get(peer_id)
            .map(Arc::clone)
            .ok_or(NodeError::SessionNotFound(*peer_id))
    }

    /// Close session with peer
    pub async fn close_session(&self, peer_id: &PeerId) -> Result<()> {
        let mut sessions = self.inner.sessions.write().await;

        if let Some(connection) = sessions.remove(peer_id) {
            connection.transition_to(SessionState::Closed).await?;
            tracing::info!("Session closed with peer {:?}", peer_id);
            Ok(())
        } else {
            Err(NodeError::SessionNotFound(*peer_id))
        }
    }

    /// List active sessions
    pub async fn active_sessions(&self) -> Vec<PeerId> {
        self.inner.sessions.read().await.keys().copied().collect()
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

        // Get file metadata
        let file_size = std::fs::metadata(file_path).map_err(NodeError::Io)?.len();

        // Create chunker (unused for now - placeholder for actual implementation)
        let _chunker = FileChunker::new(file_path, self.inner.config.transfer.chunk_size)
            .map_err(NodeError::Io)?;

        // Compute tree hash for integrity verification (unused for now - placeholder)
        let _tree_hash = compute_tree_hash(file_path, self.inner.config.transfer.chunk_size)
            .map_err(NodeError::Io)?;

        // Generate transfer ID
        let transfer_id = Self::generate_transfer_id();

        // Create transfer session
        let transfer = TransferSession::new_send(
            transfer_id,
            file_path.to_path_buf(),
            file_size,
            self.inner.config.transfer.chunk_size,
        );

        // Establish session with peer
        let _connection = self.get_or_establish_session(peer_id).await?;

        // TODO: Send file metadata to peer
        // TODO: Send chunks with encryption and obfuscation
        // For now, just create the transfer session

        // Store transfer
        self.inner
            .transfers
            .write()
            .await
            .insert(transfer_id, Arc::new(RwLock::new(transfer)));

        tracing::info!(
            "Started file transfer {:?} to peer {:?}",
            transfer_id,
            peer_id
        );

        Ok(transfer_id)
    }

    /// Wait for transfer to complete
    pub async fn wait_for_transfer(&self, transfer_id: TransferId) -> Result<()> {
        loop {
            let transfers = self.inner.transfers.read().await;
            if let Some(transfer) = transfers.get(&transfer_id) {
                let transfer_guard = transfer.read().await;
                if transfer_guard.is_complete() {
                    return Ok(());
                }
                drop(transfer_guard);
                drop(transfers);
            } else {
                return Err(NodeError::TransferNotFound(transfer_id));
            }

            // Wait before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get transfer progress
    pub async fn get_transfer_progress(&self, transfer_id: &TransferId) -> Option<f64> {
        let transfers = self.inner.transfers.read().await;
        if let Some(transfer) = transfers.get(transfer_id) {
            Some(transfer.read().await.progress())
        } else {
            None
        }
    }

    /// List active transfers
    pub async fn active_transfers(&self) -> Vec<TransferId> {
        self.inner.transfers.read().await.keys().copied().collect()
    }

    /// Generate random session ID
    fn generate_session_id() -> SessionId {
        let mut id = [0u8; 32];
        getrandom(&mut id).expect("Failed to generate session ID");
        id
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
    async fn test_session_id_generation() {
        let id1 = Node::generate_session_id();
        let id2 = Node::generate_session_id();

        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        assert_ne!(id1, id2); // Should be unique
    }
}
