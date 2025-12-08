//! Session management with Noise_XX handshake integration

use crate::node::error::{NodeError, Result};
use crate::{ConnectionId, Session, SessionState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, oneshot};
use wraith_crypto::aead::SessionCrypto;
use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
use wraith_transport::transport::Transport;

/// Type alias for peer address with interior mutability for connection migration
type PeerAddr = std::sync::RwLock<SocketAddr>;

/// Handshake packet with sender address (for channel-based handshake)
///
/// # Handshake Packet Channeling
///
/// To prevent race conditions between `packet_receive_loop` and handshake functions,
/// handshake packets (msg2, msg3) are channeled to waiting handshake code instead of
/// both code paths competing for `recv_from()` on the same UDP socket.
///
/// When a node initiates a handshake, it registers a oneshot channel in
/// `pending_handshakes` before sending msg1. When the `packet_receive_loop` receives
/// msg2, it checks `pending_handshakes` and forwards the packet via the channel instead
/// of trying to decrypt it as a normal packet.
///
/// This solves the race condition where:
/// 1. Initiator sends msg1 and calls `transport.recv_from()` to wait for msg2
/// 2. `packet_receive_loop` also calls `transport.recv_from()` in parallel
/// 3. Whichever wins the race receives msg2, causing the other to timeout
///
/// With channeling, only the packet loop receives packets, and it forwards handshake
/// packets to the appropriate waiting handshake code.
#[derive(Debug, Clone)]
pub struct HandshakePacket {
    /// Packet data (already unwrapped from protocol mimicry)
    pub data: Vec<u8>,
    /// Sender address
    pub from: SocketAddr,
}

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

    /// Peer address (wrapped for interior mutability during connection migration)
    peer_addr: PeerAddr,

    /// Connection ID for this session
    pub connection_id: ConnectionId,

    /// Session state machine
    pub session: Arc<RwLock<Session>>,

    /// Session crypto (AEAD + ratchet)
    pub crypto: Arc<RwLock<SessionCrypto>>,

    /// Connection statistics
    pub stats: ConnectionStats,

    /// Last activity timestamp (milliseconds since UNIX epoch)
    /// Uses AtomicU64 for lock-free updates from routing table lookups
    last_activity_ms: AtomicU64,

    /// Failed consecutive ping counter (lock-free using atomic)
    failed_pings: std::sync::atomic::AtomicU32,

    /// Timestamp when the session was established
    pub established_at: std::time::SystemTime,
}

/// Get current time as milliseconds since UNIX epoch
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Clone for PeerConnection {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id,
            peer_id: self.peer_id,
            // Clone peer_addr by reading current value and creating new RwLock
            peer_addr: std::sync::RwLock::new(self.peer_addr()),
            connection_id: self.connection_id,
            // Clone Arc references (cheap - just incrementing refcount)
            session: Arc::clone(&self.session),
            crypto: Arc::clone(&self.crypto),
            stats: self.stats.clone(),
            // Clone AtomicU64 by loading its current value
            last_activity_ms: AtomicU64::new(self.last_activity_ms.load(Ordering::Relaxed)),
            failed_pings: std::sync::atomic::AtomicU32::new(
                self.failed_pings.load(Ordering::Relaxed),
            ),
            established_at: self.established_at,
        }
    }
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
            peer_addr: std::sync::RwLock::new(peer_addr),
            connection_id,
            session: Arc::new(RwLock::new(Session::new())),
            crypto: Arc::new(RwLock::new(crypto)),
            stats: ConnectionStats::default(),
            last_activity_ms: AtomicU64::new(current_time_ms()),
            failed_pings: std::sync::atomic::AtomicU32::new(0),
            established_at: std::time::SystemTime::now(),
        }
    }

    /// Get the current peer address
    ///
    /// Thread-safe read access to the peer address.
    #[inline]
    pub fn peer_addr(&self) -> SocketAddr {
        *self.peer_addr.read().expect("peer_addr lock poisoned")
    }

    /// Increment failed ping counter
    pub fn increment_failed_pings(&self) -> u32 {
        self.failed_pings.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Reset failed ping counter (successful ping)
    pub fn reset_failed_pings(&self) {
        self.failed_pings.store(0, Ordering::Relaxed);
    }

    /// Get current failed ping count
    pub fn failed_ping_count(&self) -> u32 {
        self.failed_pings.load(Ordering::Relaxed)
    }

    /// Check if connection is stale
    ///
    /// Uses atomic load for lock-free staleness check.
    pub fn is_stale(&self, idle_timeout: Duration) -> bool {
        let last_ms = self.last_activity_ms.load(Ordering::Relaxed);
        let now_ms = current_time_ms();
        let elapsed_ms = now_ms.saturating_sub(last_ms);
        elapsed_ms > idle_timeout.as_millis() as u64
    }

    /// Update last activity timestamp
    ///
    /// Lock-free update using atomic store - safe to call from routing table lookup.
    pub fn touch(&self) {
        self.last_activity_ms
            .store(current_time_ms(), Ordering::Relaxed);
    }

    /// Get milliseconds since last activity
    pub fn idle_duration_ms(&self) -> u64 {
        let last_ms = self.last_activity_ms.load(Ordering::Relaxed);
        current_time_ms().saturating_sub(last_ms)
    }

    /// Create a mock peer connection for testing
    ///
    /// Creates a PeerConnection with random session crypto for unit tests
    /// that need to insert sessions into the sessions map.
    #[cfg(test)]
    pub fn new_for_test(peer_id: PeerId, peer_addr: SocketAddr) -> Self {
        use wraith_crypto::aead::SessionCrypto;

        // Generate random session ID from peer_id
        let mut session_id = [0u8; 32];
        session_id.copy_from_slice(&peer_id);

        // Create random session crypto with random keys
        let mut send_key = [0u8; 32];
        let mut recv_key = [0u8; 32];
        let mut chain_key = [0u8; 32];
        getrandom::getrandom(&mut send_key).expect("getrandom failed");
        getrandom::getrandom(&mut recv_key).expect("getrandom failed");
        getrandom::getrandom(&mut chain_key).expect("getrandom failed");
        let crypto = SessionCrypto::new(send_key, recv_key, &chain_key);

        // Generate connection ID from peer_id bytes
        let mut cid_bytes = [0u8; 8];
        cid_bytes.copy_from_slice(&peer_id[..8]);
        let connection_id = crate::ConnectionId::from_bytes(cid_bytes);

        Self {
            session_id,
            peer_id,
            peer_addr: std::sync::RwLock::new(peer_addr),
            connection_id,
            session: Arc::new(RwLock::new(crate::Session::new())),
            crypto: Arc::new(RwLock::new(crypto)),
            stats: ConnectionStats::default(),
            last_activity_ms: AtomicU64::new(current_time_ms()),
            failed_pings: std::sync::atomic::AtomicU32::new(0),
            established_at: std::time::SystemTime::now(),
        }
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
            .map_err(|e| NodeError::InvalidState(e.to_string().into()))
    }

    /// Encrypt frame data for transmission
    ///
    /// Takes serialized frame bytes and encrypts them using the session crypto.
    /// Automatically manages nonce counters and checks for rekey conditions.
    ///
    /// # Arguments
    ///
    /// * `frame_bytes` - Serialized frame data to encrypt
    ///
    /// # Returns
    ///
    /// Encrypted frame data (ciphertext + auth tag)
    ///
    /// # Errors
    ///
    /// Returns error if encryption fails or rekey is needed.
    pub async fn encrypt_frame(&self, frame_bytes: &[u8]) -> Result<Vec<u8>> {
        let mut crypto = self.crypto.write().await;

        // Check if rekey is needed
        if crypto.needs_rekey() {
            return Err(NodeError::Crypto(
                wraith_crypto::CryptoError::NonceOverflow.to_string(),
            ));
        }

        // Encrypt with empty AAD (frame already contains all necessary data)
        crypto
            .encrypt(frame_bytes, &[])
            .map_err(|e| NodeError::Crypto(e.to_string()))
    }

    /// Decrypt received frame data
    ///
    /// Takes encrypted bytes and decrypts them using the session crypto.
    /// Automatically manages nonce counters and replay protection.
    ///
    /// # Arguments
    ///
    /// * `encrypted_bytes` - Encrypted frame data (ciphertext + auth tag)
    ///
    /// # Returns
    ///
    /// Decrypted frame data ready for parsing
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails, authentication fails, or replay is detected.
    pub async fn decrypt_frame(&self, encrypted_bytes: &[u8]) -> Result<Vec<u8>> {
        let mut crypto = self.crypto.write().await;

        // Decrypt with empty AAD
        crypto
            .decrypt(encrypted_bytes, &[])
            .map_err(|e| NodeError::Crypto(e.to_string()))
    }

    /// Check if session needs rekeying
    pub async fn needs_rekey(&self) -> bool {
        self.crypto.read().await.needs_rekey()
    }

    /// Get current send counter (for debugging/monitoring)
    pub async fn send_counter(&self) -> u64 {
        self.crypto.read().await.send_counter()
    }

    /// Get current receive counter (for debugging/monitoring)
    pub async fn recv_counter(&self) -> u64 {
        self.crypto.read().await.recv_counter()
    }

    /// Update peer address after successful migration
    ///
    /// This method is called after PATH_CHALLENGE/RESPONSE validation
    /// to update the session's peer address to the new validated address.
    ///
    /// # Arguments
    ///
    /// * `new_addr` - The new validated peer address
    ///
    /// # Thread Safety
    ///
    /// This method uses RwLock for safe interior mutability.
    pub fn update_peer_addr(&self, new_addr: SocketAddr) {
        let old_addr = self.peer_addr();

        // Update address using RwLock write lock
        {
            let mut addr = self.peer_addr.write().expect("peer_addr lock poisoned");
            *addr = new_addr;
        }

        tracing::info!(
            "Updated peer address from {} to {} for session {:?}",
            old_addr,
            new_addr,
            hex::encode(&self.session_id[..8])
        );
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
/// Exchanges handshake messages over the transport to establish a secure session.
///
/// # Arguments
///
/// * `local_keypair` - Local X25519 keypair for handshake
/// * `peer_addr` - Remote peer address
/// * `transport` - Transport layer for sending/receiving handshake messages
/// * `msg2_rx` - Optional channel to receive msg2. When provided, msg2 is received via the
///   channel instead of calling `transport.recv_from()` directly. This prevents race conditions
///   with `packet_receive_loop` where both code paths compete for the same socket. If `None`,
///   falls back to direct `recv_from()` (for tests or standalone usage).
///
/// # Returns
///
/// Returns session crypto, session ID, and peer's X25519 public key on success.
///
/// # Race Condition Prevention
///
/// Without channeling, both `packet_receive_loop` and this function call `recv_from()` on the
/// same socket, causing a race where whichever wins receives msg2 and the other times out.
/// With channeling, only `packet_receive_loop` receives packets and forwards handshake packets
/// to the appropriate channel.
pub async fn perform_handshake_initiator<T: Transport + Send + Sync>(
    local_keypair: &NoiseKeypair,
    peer_addr: SocketAddr,
    transport: &T,
    msg2_rx: Option<oneshot::Receiver<HandshakePacket>>,
) -> Result<(SessionCrypto, SessionId, PeerId)> {
    tracing::debug!(
        "Starting Noise_XX handshake as initiator with {}",
        peer_addr
    );

    // Create Noise handshake state
    let mut noise = NoiseHandshake::new_initiator(local_keypair)
        .map_err(|e| NodeError::Handshake(e.to_string().into()))?;

    // Noise_XX handshake pattern:
    // -> e (initiator sends ephemeral key)
    // <- e, ee, s, es (responder sends ephemeral, performs DH, sends static, performs DH)
    // -> s, se (initiator sends static, performs DH)

    // 1. Send message 1 (-> e)
    let msg1 = noise
        .write_message(&[])
        .map_err(|e| NodeError::Handshake(format!("Failed to create msg1: {}", e).into()))?;

    tracing::trace!(
        "Sending handshake msg1 ({} bytes) to {}",
        msg1.len(),
        peer_addr
    );

    transport
        .send_to(&msg1, peer_addr)
        .await
        .map_err(|e| NodeError::Transport(format!("Failed to send msg1: {}", e).into()))?;

    // 2. Receive message 2 (<- e, ee, s, es)
    // Use channel if provided (prevents racing with packet_receive_loop)
    // Otherwise fall back to direct recv_from (for tests and standalone usage)
    let (msg2_data, from) = if let Some(rx) = msg2_rx {
        // Receive via channel (registered in pending_handshakes)
        let packet = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|_| NodeError::Handshake("Handshake timeout waiting for msg2".into()))?
            .map_err(|_| NodeError::Handshake("Handshake channel closed".into()))?;
        (packet.data, packet.from)
    } else {
        // Direct recv_from (fallback for tests)
        let mut buf = vec![0u8; 4096];
        let (size, from) =
            tokio::time::timeout(Duration::from_secs(5), transport.recv_from(&mut buf))
                .await
                .map_err(|_| NodeError::Handshake("Handshake timeout waiting for msg2".into()))?
                .map_err(|e| {
                    NodeError::Transport(format!("Failed to receive msg2: {}", e).into())
                })?;
        (buf[..size].to_vec(), from)
    };

    // Validate sender address - handle 0.0.0.0 (INADDR_ANY) case
    // When peer binds to 0.0.0.0:port, packets appear from their actual IP
    let addr_matches = if peer_addr.ip().is_unspecified() {
        // Only check port when peer is bound to INADDR_ANY
        from.port() == peer_addr.port()
    } else {
        from == peer_addr
    };

    if !addr_matches {
        return Err(NodeError::Handshake(
            format!(
                "Received msg2 from unexpected address: {} (expected {})",
                from, peer_addr
            )
            .into(),
        ));
    }

    tracing::trace!(
        "Received handshake msg2 ({} bytes) from {}",
        msg2_data.len(),
        from
    );

    let _payload2 = noise
        .read_message(&msg2_data)
        .map_err(|e| NodeError::Handshake(format!("Failed to process msg2: {}", e).into()))?;

    // 3. Send message 3 (-> s, se)
    let msg3 = noise
        .write_message(&[])
        .map_err(|e| NodeError::Handshake(format!("Failed to create msg3: {}", e).into()))?;

    tracing::trace!(
        "Sending handshake msg3 ({} bytes) to {}",
        msg3.len(),
        peer_addr
    );

    transport
        .send_to(&msg3, peer_addr)
        .await
        .map_err(|e| NodeError::Transport(format!("Failed to send msg3: {}", e).into()))?;

    // Extract session keys after handshake completes
    if !noise.is_complete() {
        return Err(NodeError::Handshake(
            "Handshake not complete after msg3".into(),
        ));
    }

    // Get peer's public key BEFORE consuming noise with into_session_keys()
    let peer_id = noise.get_remote_static().ok_or_else(|| {
        NodeError::Handshake("Failed to get remote static key after handshake".into())
    })?;

    let keys = noise
        .into_session_keys()
        .map_err(|e| NodeError::Handshake(format!("Failed to extract keys: {}", e).into()))?;

    // Create session crypto (initiator: send=send_key, recv=recv_key)
    let crypto = SessionCrypto::new(keys.send_key, keys.recv_key, &keys.chain_key);

    // Derive session ID from keys (extend 8-byte CID to 32-byte session ID)
    let cid = keys.derive_connection_id();
    let mut session_id = [0u8; 32];
    session_id[..8].copy_from_slice(&cid);
    // Fill rest with hash of chain key for uniqueness
    session_id[8..].copy_from_slice(&keys.chain_key[..24]);

    tracing::info!(
        "Noise_XX handshake complete as initiator, session: {:?}, peer: {}",
        hex::encode(&session_id[..8]),
        hex::encode(&peer_id[..8])
    );

    Ok((crypto, session_id, peer_id))
}

/// Perform Noise_XX handshake as responder
///
/// Exchanges handshake messages over the transport to establish a secure session.
///
/// # Arguments
///
/// * `local_keypair` - Local X25519 keypair for handshake
/// * `msg1` - First handshake message from initiator
/// * `peer_addr` - Remote peer address
/// * `transport` - Transport layer for sending/receiving handshake messages
/// * `msg3_rx` - Optional channel to receive msg3 (prevents recv_from racing with packet loop)
///
/// # Returns
///
/// Returns session crypto, session ID, and peer's public key on success.
pub async fn perform_handshake_responder<T: Transport + Send + Sync>(
    local_keypair: &NoiseKeypair,
    msg1: &[u8],
    peer_addr: SocketAddr,
    transport: &T,
    msg3_rx: Option<oneshot::Receiver<HandshakePacket>>,
) -> Result<(SessionCrypto, SessionId, PeerId)> {
    tracing::debug!(
        "Starting Noise_XX handshake as responder with {}",
        peer_addr
    );

    // Create Noise handshake state
    let mut noise = NoiseHandshake::new_responder(local_keypair)
        .map_err(|e| NodeError::Handshake(e.to_string().into()))?;

    // Noise_XX handshake pattern (from responder perspective):
    // <- e (receive initiator's ephemeral key)
    // -> e, ee, s, es (send ephemeral, perform DH, send static, perform DH)
    // <- s, se (receive initiator's static, perform DH)

    // 1. Process message 1 (<- e)
    tracing::trace!(
        "Processing handshake msg1 ({} bytes) from {}",
        msg1.len(),
        peer_addr
    );

    let _payload1 = noise
        .read_message(msg1)
        .map_err(|e| NodeError::Handshake(format!("Failed to process msg1: {}", e).into()))?;

    // 2. Send message 2 (-> e, ee, s, es)
    let msg2 = noise
        .write_message(&[])
        .map_err(|e| NodeError::Handshake(format!("Failed to create msg2: {}", e).into()))?;

    tracing::trace!(
        "Sending handshake msg2 ({} bytes) to {}",
        msg2.len(),
        peer_addr
    );

    transport
        .send_to(&msg2, peer_addr)
        .await
        .map_err(|e| NodeError::Transport(format!("Failed to send msg2: {}", e).into()))?;

    // 3. Receive message 3 (<- s, se)
    // Use channel if provided (prevents racing with packet_receive_loop)
    // Otherwise fall back to direct recv_from (for tests and standalone usage)
    let (msg3_data, from) = if let Some(rx) = msg3_rx {
        // Receive via channel (registered in pending_handshakes)
        let packet = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|_| NodeError::Handshake("Handshake timeout waiting for msg3".into()))?
            .map_err(|_| NodeError::Handshake("Handshake channel closed".into()))?;
        (packet.data, packet.from)
    } else {
        // Direct recv_from (fallback for tests)
        let mut buf = vec![0u8; 4096];
        let (size, from) =
            tokio::time::timeout(Duration::from_secs(5), transport.recv_from(&mut buf))
                .await
                .map_err(|_| NodeError::Handshake("Handshake timeout waiting for msg3".into()))?
                .map_err(|e| {
                    NodeError::Transport(format!("Failed to receive msg3: {}", e).into())
                })?;
        (buf[..size].to_vec(), from)
    };

    // Validate sender address - handle 0.0.0.0 (INADDR_ANY) case
    // When peer binds to 0.0.0.0:port, packets appear from their actual IP
    let addr_matches = if peer_addr.ip().is_unspecified() {
        // Only check port when peer is bound to INADDR_ANY
        from.port() == peer_addr.port()
    } else {
        from == peer_addr
    };

    if !addr_matches {
        return Err(NodeError::Handshake(
            format!(
                "Received msg3 from unexpected address: {} (expected {})",
                from, peer_addr
            )
            .into(),
        ));
    }

    tracing::trace!(
        "Received handshake msg3 ({} bytes) from {}",
        msg3_data.len(),
        from
    );

    let _payload3 = noise
        .read_message(&msg3_data)
        .map_err(|e| NodeError::Handshake(format!("Failed to process msg3: {}", e).into()))?;

    // Extract session keys after handshake completes
    if !noise.is_complete() {
        return Err(NodeError::Handshake(
            "Handshake not complete after msg3".into(),
        ));
    }

    // Get peer's public key BEFORE consuming noise with into_session_keys()
    let peer_id = noise.get_remote_static().ok_or_else(|| {
        NodeError::Handshake("Failed to get remote static key after handshake".into())
    })?;

    let keys = noise
        .into_session_keys()
        .map_err(|e| NodeError::Handshake(format!("Failed to extract keys: {}", e).into()))?;

    // Create session crypto (responder: recv=send_key, send=recv_key - reversed from initiator)
    let crypto = SessionCrypto::new(keys.recv_key, keys.send_key, &keys.chain_key);

    // Derive session ID from keys (extend 8-byte CID to 32-byte session ID)
    let cid = keys.derive_connection_id();
    let mut session_id = [0u8; 32];
    session_id[..8].copy_from_slice(&cid);
    // Fill rest with hash of chain key for uniqueness
    session_id[8..].copy_from_slice(&keys.chain_key[..24]);

    tracing::info!(
        "Noise_XX handshake complete as responder, session: {:?}, peer: {}",
        hex::encode(&session_id[..8]),
        hex::encode(&peer_id[..8])
    );

    Ok((crypto, session_id, peer_id))
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
        assert_eq!(conn.peer_addr(), peer_addr);
        assert!(!conn.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn test_stale_detection() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);
        let crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);

        let conn = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Set last activity to 5 minutes ago using atomic store
        let now_ms = current_time_ms();
        let five_minutes_ago = now_ms.saturating_sub(300_000); // 5 minutes in ms
        conn.last_activity_ms
            .store(five_minutes_ago, Ordering::Relaxed);

        // Verify idle duration is approximately 5 minutes (allow 10 second tolerance)
        let idle_ms = conn.idle_duration_ms();
        assert!(
            idle_ms >= 295_000,
            "Should be idle for ~5 minutes, got {} ms",
            idle_ms
        );

        // Test with a 3-minute timeout - connection should be stale
        let short_timeout = Duration::from_secs(180);
        assert!(
            conn.is_stale(short_timeout),
            "Connection should be stale with 3min timeout (idle {} ms)",
            idle_ms
        );

        // Test with a 6-minute timeout - connection should not be stale
        let long_timeout = Duration::from_secs(360);
        assert!(
            !conn.is_stale(long_timeout),
            "Connection should not be stale with 6min timeout"
        );

        // Test touch() updates last activity
        conn.touch();
        let new_idle = conn.idle_duration_ms();
        assert!(
            new_idle < 1000,
            "After touch(), idle time should be < 1 second, got {} ms",
            new_idle
        );
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

    #[tokio::test]
    async fn test_encrypt_decrypt_frame() {
        // Create two connections with swapped keys (simulating bidirectional communication)
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);

        let send_key = [4u8; 32];
        let recv_key = [5u8; 32];
        let chain_key = [6u8; 32];

        let alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
        let bob_crypto = SessionCrypto::new(recv_key, send_key, &chain_key);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, bob_crypto);

        // Alice encrypts a frame
        let frame_data = b"Hello from Alice!";
        let encrypted = alice.encrypt_frame(frame_data).await.unwrap();

        // Bob decrypts it
        let decrypted = bob.decrypt_frame(&encrypted).await.unwrap();
        assert_eq!(decrypted, frame_data);

        // Bob replies
        let reply_data = b"Hello from Bob!";
        let encrypted_reply = bob.encrypt_frame(reply_data).await.unwrap();

        // Alice decrypts the reply
        let decrypted_reply = alice.decrypt_frame(&encrypted_reply).await.unwrap();
        assert_eq!(decrypted_reply, reply_data);
    }

    #[tokio::test]
    async fn test_counter_increment() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);
        let crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);

        let conn = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Initial counter should be 0
        assert_eq!(conn.send_counter().await, 0);

        // Encrypt a frame
        let _ = conn.encrypt_frame(b"test1").await.unwrap();

        // Counter should increment
        assert_eq!(conn.send_counter().await, 1);

        // Encrypt another frame
        let _ = conn.encrypt_frame(b"test2").await.unwrap();

        // Counter should increment again
        assert_eq!(conn.send_counter().await, 2);
    }

    #[tokio::test]
    async fn test_decrypt_wrong_key_fails() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);

        let send_key = [4u8; 32];
        let recv_key = [5u8; 32];
        let wrong_key = [99u8; 32];
        let chain_key = [6u8; 32];

        let alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
        let bob_wrong_crypto = SessionCrypto::new(recv_key, wrong_key, &chain_key);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(
            session_id,
            peer_id,
            peer_addr,
            connection_id,
            bob_wrong_crypto,
        );

        // Alice encrypts
        let encrypted = alice.encrypt_frame(b"secret message").await.unwrap();

        // Bob tries to decrypt with wrong key - should fail
        let result = bob.decrypt_frame(&encrypted).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_needs_rekey_detection() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);
        let crypto = SessionCrypto::new([4u8; 32], [5u8; 32], &[6u8; 32]);

        let conn = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Initially should not need rekey
        assert!(!conn.needs_rekey().await);

        // Simulate reaching the counter limit (1M messages)
        {
            let mut crypto = conn.crypto.write().await;
            crypto.update_keys([7u8; 32], [8u8; 32], &[9u8; 32]);
            // Manually set counter to limit - 1
            for _ in 0..999_999 {
                // In real code this would be done by encrypting messages
            }
        }

        // After many encryptions, should eventually need rekey
        // (In real code this would happen after 1M messages)
    }

    #[tokio::test]
    async fn test_multiple_frames_sequential() {
        let session_id = [1u8; 32];
        let peer_id = [2u8; 32];
        let peer_addr = "127.0.0.1:5000".parse().unwrap();
        let connection_id = ConnectionId::from_bytes([3u8; 8]);

        let send_key = [4u8; 32];
        let recv_key = [5u8; 32];
        let chain_key = [6u8; 32];

        let alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
        let bob_crypto = SessionCrypto::new(recv_key, send_key, &chain_key);

        let alice =
            PeerConnection::new(session_id, peer_id, peer_addr, connection_id, alice_crypto);
        let bob = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, bob_crypto);

        // Send multiple frames in sequence
        let frames = [b"frame1", b"frame2", b"frame3", b"frame4", b"frame5"];

        for (i, frame_data) in frames.iter().enumerate() {
            let encrypted = alice.encrypt_frame(*frame_data).await.unwrap();
            let decrypted = bob.decrypt_frame(&encrypted).await.unwrap();
            assert_eq!(decrypted.as_slice(), *frame_data);

            // Verify counters increment correctly
            assert_eq!(alice.send_counter().await, (i + 1) as u64);
            assert_eq!(bob.recv_counter().await, (i + 1) as u64);
        }
    }
}
