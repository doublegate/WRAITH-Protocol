//! Session management with Noise_XX handshake integration

use crate::node::error::{NodeError, Result};
use crate::{ConnectionId, Session, SessionState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use wraith_crypto::aead::SessionCrypto;
use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

/// Peer identifier (Ed25519 public key)
pub type PeerId = [u8; 32];

/// Unique session identifier
pub type SessionId = [u8; 32];

/// Peer connection handle
///
/// Combines session state, crypto state, and transport connection.
pub struct PeerConnection {
    /// Session ID
    pub session_id: SessionId,

    /// Peer ID (public key)
    pub peer_id: PeerId,

    /// Peer address
    pub peer_addr: SocketAddr,

    /// Connection ID for this session
    pub connection_id: ConnectionId,

    /// Session state machine
    pub session: Arc<RwLock<Session>>,

    /// Session crypto (AEAD + ratchet)
    pub crypto: Arc<RwLock<SessionCrypto>>,

    /// Connection statistics
    pub stats: ConnectionStats,

    /// Last activity timestamp
    pub last_activity: Instant,
}

impl PeerConnection {
    /// Create new peer connection
    pub fn new(
        session_id: SessionId,
        peer_id: PeerId,
        peer_addr: SocketAddr,
        connection_id: ConnectionId,
        crypto: SessionCrypto,
    ) -> Self {
        Self {
            session_id,
            peer_id,
            peer_addr,
            connection_id,
            session: Arc::new(RwLock::new(Session::new())),
            crypto: Arc::new(RwLock::new(crypto)),
            stats: ConnectionStats::default(),
            last_activity: Instant::now(),
        }
    }

    /// Check if connection is stale
    pub fn is_stale(&self, idle_timeout: Duration) -> bool {
        self.last_activity.elapsed() > idle_timeout
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get session state
    pub async fn state(&self) -> SessionState {
        self.session.read().await.state()
    }

    /// Transition session state
    pub async fn transition_to(&self, new_state: SessionState) -> Result<()> {
        self.session
            .write()
            .await
            .transition_to(new_state)
            .map_err(|e| NodeError::InvalidState(e.to_string()))
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Packets sent
    pub packets_sent: u64,

    /// Packets received
    pub packets_received: u64,

    /// Round-trip time (microseconds)
    pub rtt_us: Option<u64>,

    /// Packet loss rate (0.0 to 1.0)
    pub loss_rate: f64,
}

/// Perform Noise_XX handshake as initiator
///
/// Returns session keys on success.
pub async fn perform_handshake_initiator(
    local_keypair: &NoiseKeypair,
    _peer_id: &PeerId,
) -> Result<(SessionCrypto, SessionId)> {
    // Create Noise handshake state
    let mut noise = NoiseHandshake::new_initiator(local_keypair)
        .map_err(|e| NodeError::Handshake(e.to_string()))?;

    // In a real implementation, we would:
    // 1. Send message 1 (-> e)
    // 2. Receive message 2 (<- e, ee, s, es)
    // 3. Send message 3 (-> s, se)
    //
    // For now, we'll create a simplified version that assumes
    // the handshake messages are exchanged via the transport layer

    // TODO: Integrate with actual transport send/receive
    // This is a placeholder that shows the structure

    let _msg1 = noise
        .write_message(&[])
        .map_err(|e| NodeError::Handshake(e.to_string()))?;

    // Simulate receiving msg2 (in real impl, this comes from peer)
    // For now, we'll just complete the handshake to get the structure right

    // Extract session keys after handshake completes
    if noise.is_complete() {
        let keys = noise
            .into_session_keys()
            .map_err(|e| NodeError::Handshake(e.to_string()))?;

        // Create session crypto
        let crypto = SessionCrypto::new(keys.send_key, keys.recv_key, &keys.chain_key);

        // Derive session ID from keys (extend 8-byte CID to 32-byte session ID)
        let cid = keys.derive_connection_id();
        let mut session_id = [0u8; 32];
        session_id[..8].copy_from_slice(&cid);
        // Fill rest with hash of chain key for uniqueness
        session_id[8..].copy_from_slice(&keys.chain_key[..24]);

        Ok((crypto, session_id))
    } else {
        Err(NodeError::Handshake("Handshake not complete".to_string()))
    }
}

/// Perform Noise_XX handshake as responder
///
/// Returns session keys on success.
pub async fn perform_handshake_responder(
    local_keypair: &NoiseKeypair,
    _peer_id: &PeerId,
) -> Result<(SessionCrypto, SessionId)> {
    // Create Noise handshake state
    let noise = NoiseHandshake::new_responder(local_keypair)
        .map_err(|e| NodeError::Handshake(e.to_string()))?;

    // In a real implementation, we would:
    // 1. Receive message 1 (<- e)
    // 2. Send message 2 (-> e, ee, s, es)
    // 3. Receive message 3 (<- s, se)
    //
    // For now, this is a placeholder structure

    // TODO: Integrate with actual transport send/receive

    // Extract session keys after handshake completes
    if noise.is_complete() {
        let keys = noise
            .into_session_keys()
            .map_err(|e| NodeError::Handshake(e.to_string()))?;

        // Create session crypto
        let crypto = SessionCrypto::new(keys.recv_key, keys.send_key, &keys.chain_key);

        // Derive session ID from keys (extend 8-byte CID to 32-byte session ID)
        let cid = keys.derive_connection_id();
        let mut session_id = [0u8; 32];
        session_id[..8].copy_from_slice(&cid);
        // Fill rest with hash of chain key for uniqueness
        session_id[8..].copy_from_slice(&keys.chain_key[..24]);

        Ok((crypto, session_id))
    } else {
        Err(NodeError::Handshake("Handshake not complete".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wraith_crypto::noise::NoiseKeypair;

    #[test]
    fn test_peer_connection_creation() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);
        let crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);

        let conn = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        assert_eq!(conn.session_id, session_id);
        assert_eq!(conn.peer_id, peer_id);
        assert_eq!(conn.peer_addr, peer_addr);
        assert!(!conn.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn test_stale_detection() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);
        let crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);

        let mut conn = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Set last activity to 5 minutes ago
        conn.last_activity = Instant::now() - Duration::from_secs(300);

        assert!(conn.is_stale(Duration::from_secs(180))); // 3 min timeout
        assert!(!conn.is_stale(Duration::from_secs(360))); // 6 min timeout
    }

    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.rtt_us, None);
        assert_eq!(stats.loss_rate, 0.0);
    }

    #[tokio::test]
    async fn test_handshake_keypair_generation() {
        // Test that we can generate keypairs for handshakes
        let keypair = NoiseKeypair::generate().unwrap();
        assert!(!keypair.private_key().is_empty());
        assert!(!keypair.public_key().is_empty());
    }
}
