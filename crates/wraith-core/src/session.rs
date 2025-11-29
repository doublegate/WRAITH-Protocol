//! Session state machine and connection management.
//!
//! A Session represents an authenticated, encrypted connection between
//! two peers. Sessions multiplex multiple streams (file transfers) over
//! a single UDP "connection".

use crate::error::SessionError;
use crate::stream::Stream;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Connection ID (CID) for session demultiplexing.
///
/// The Connection ID is a 64-bit value derived during the handshake:
/// - High 32 bits: Random session identifier
/// - Low 32 bits: Rotate based on packet sequence to prevent tracking
///
/// Derivation:
/// ```text
/// initial_cid = BLAKE3(shared_secret || "connection-id")[0..8]
/// rotating_cid = initial_cid[0..4] || (initial_cid[4..8] XOR seq_num)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Reserved (invalid) connection ID
    pub const INVALID: Self = Self(0x0000000000000000);

    /// Handshake initiation packet
    pub const HANDSHAKE: Self = Self(0xFFFFFFFFFFFFFFFF);

    /// Version negotiation
    pub const VERSION_NEGOTIATION: Self = Self(0xFFFFFFFFFFFFFFFE);

    /// Stateless reset
    pub const STATELESS_RESET: Self = Self(0xFFFFFFFFFFFFFFFD);

    /// Create a new connection ID from raw value
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_be_bytes(bytes))
    }

    /// Convert to raw bytes
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    /// Get the raw u64 value
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Create a rotating connection ID from initial CID and sequence number
    pub fn rotate(self, seq_num: u32) -> Self {
        let base = (self.0 >> 32) as u32;
        let rotated = (self.0 as u32) ^ seq_num;
        Self((u64::from(base) << 32) | u64::from(rotated))
    }

    /// Check if this is a special connection ID
    pub fn is_special(self) -> bool {
        matches!(
            self,
            Self::HANDSHAKE | Self::VERSION_NEGOTIATION | Self::STATELESS_RESET
        )
    }

    /// Check if this is a valid connection ID
    pub fn is_valid(self) -> bool {
        self != Self::INVALID && !self.is_special()
    }
}

/// Session configuration parameters
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum concurrent streams per session
    pub max_streams: u16,
    /// Initial flow control window (bytes)
    pub initial_window: u64,
    /// Maximum flow control window (bytes)
    pub max_window: u64,
    /// Idle timeout before session close
    pub idle_timeout: Duration,
    /// Rekey interval for forward secrecy
    pub rekey_interval: Duration,
    /// Maximum packets before mandatory rekey
    pub rekey_packet_limit: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_streams: 16384,
            initial_window: 1024 * 1024,  // 1 MiB
            max_window: 16 * 1024 * 1024, // 16 MiB
            idle_timeout: Duration::from_secs(30),
            rekey_interval: Duration::from_secs(120),
            rekey_packet_limit: 1_000_000,
        }
    }
}

/// Session state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial state, no connection
    Closed,
    /// Handshake in progress
    Handshaking(HandshakePhase),
    /// Connection established, normal operation
    Established,
    /// Rekeying in progress (forward secrecy)
    Rekeying,
    /// Graceful shutdown, draining pending data
    Draining,
    /// Connection migration, validating new path
    Migrating,
}

/// Handshake sub-states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakePhase {
    /// Initiator: sent Phase 1, awaiting Phase 2
    InitSent,
    /// Responder: received Phase 1, sent Phase 2, awaiting Phase 3
    RespSent,
    /// Initiator: received Phase 2, sent Phase 3
    InitComplete,
}

/// A single session with a remote peer
pub struct Session {
    /// Current session state
    state: SessionState,
    /// Session configuration
    config: SessionConfig,
    /// Connection ID for this session
    connection_id: ConnectionId,
    /// Active streams (stream_id -> Stream)
    streams: HashMap<u16, Stream>,
    /// Next stream ID to allocate (client: odd, server: even)
    next_stream_id: u16,
    /// Session establishment timestamp
    established_at: Option<Instant>,
    /// Last activity timestamp
    last_activity: Instant,
    /// Last rekey timestamp
    last_rekey: Option<Instant>,
    /// Packet counter for nonce generation
    packet_counter: u64,
    /// Total bytes sent
    bytes_sent: u64,
    /// Total bytes received
    bytes_received: u64,
    /// Packets sent
    packets_sent: u64,
    /// Packets received
    packets_received: u64,
}

impl Session {
    /// Create a new session with default configuration
    pub fn new() -> Self {
        Self::with_config(SessionConfig::default())
    }

    /// Create a new session with custom configuration
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            state: SessionState::Closed,
            config,
            connection_id: ConnectionId::INVALID,
            streams: HashMap::new(),
            next_stream_id: 1, // Client starts with odd IDs
            established_at: None,
            last_activity: Instant::now(),
            last_rekey: None,
            packet_counter: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
        }
    }

    /// Create a new session as initiator (client)
    pub fn new_initiator(config: SessionConfig) -> Self {
        let mut session = Self::with_config(config);
        session.next_stream_id = 1; // Odd stream IDs
        session
    }

    /// Create a new session as responder (server)
    pub fn new_responder(config: SessionConfig) -> Self {
        let mut session = Self::with_config(config);
        session.next_stream_id = 2; // Even stream IDs
        session
    }

    /// Get current session state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Get connection ID
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    /// Set connection ID (called after handshake)
    pub fn set_connection_id(&mut self, cid: ConnectionId) {
        self.connection_id = cid;
    }

    /// Check if a state transition is valid
    pub fn can_transition(&self, to: SessionState) -> bool {
        use SessionState::*;

        match (self.state, to) {
            // From Closed
            (Closed, Handshaking(_)) => true,
            (Closed, Closed) => true, // Allow re-close

            // From Handshaking
            (Handshaking(HandshakePhase::InitSent), Handshaking(HandshakePhase::InitComplete)) => {
                true
            }
            (Handshaking(HandshakePhase::RespSent), Established) => true,
            (Handshaking(HandshakePhase::InitComplete), Established) => true,
            (Handshaking(_), Established) => true, // Allow any handshake phase to Established
            (Handshaking(_), Closed) => true,      // Allow abort during handshake

            // From Established
            (Established, Rekeying) => true,
            (Established, Draining) => true,
            (Established, Migrating) => true,
            (Established, Closed) => true,
            (Established, Established) => true, // Allow re-establish

            // From Rekeying
            (Rekeying, Established) => true,
            (Rekeying, Closed) => true,

            // From Draining
            (Draining, Closed) => true,

            // From Migrating
            (Migrating, Established) => true,
            (Migrating, Closed) => true,

            // All other transitions invalid
            _ => false,
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: SessionState) -> Result<(), SessionError> {
        if !self.can_transition(new_state) {
            return Err(SessionError::InvalidState);
        }

        let old_state = self.state;
        self.state = new_state;

        // Handle state entry logic
        match new_state {
            SessionState::Established => {
                if self.established_at.is_none() {
                    self.established_at = Some(Instant::now());
                }
            }
            SessionState::Rekeying => {
                self.last_rekey = Some(Instant::now());
            }
            SessionState::Closed => {
                // Clean up resources
                self.streams.clear();
            }
            _ => {}
        }

        tracing::debug!(
            "Session state transition: {:?} -> {:?}",
            old_state,
            new_state
        );

        Ok(())
    }

    /// Allocate a new stream ID
    pub fn allocate_stream_id(&mut self) -> Result<u16, SessionError> {
        if self.streams.len() >= self.config.max_streams as usize {
            return Err(SessionError::TooManyStreams);
        }

        let stream_id = self.next_stream_id;
        self.next_stream_id = self.next_stream_id.wrapping_add(2); // Skip to next odd/even ID

        Ok(stream_id)
    }

    /// Create a new stream
    pub fn create_stream(&mut self) -> Result<u16, SessionError> {
        let stream_id = self.allocate_stream_id()?;
        let stream = Stream::new(stream_id, self.config.initial_window);
        self.streams.insert(stream_id, stream);
        Ok(stream_id)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: u16) -> Option<&Stream> {
        self.streams.get(&stream_id)
    }

    /// Get a mutable stream by ID
    pub fn get_stream_mut(&mut self, stream_id: u16) -> Option<&mut Stream> {
        self.streams.get_mut(&stream_id)
    }

    /// Remove a stream
    pub fn remove_stream(&mut self, stream_id: u16) -> Option<Stream> {
        self.streams.remove(&stream_id)
    }

    /// Get number of active streams
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Check if session is idle (no activity for idle_timeout duration)
    pub fn is_idle(&self) -> bool {
        self.last_activity.elapsed() >= self.config.idle_timeout
    }

    /// Check if rekey is needed
    pub fn needs_rekey(&self) -> bool {
        // Rekey if time interval exceeded
        if let Some(last_rekey) = self.last_rekey {
            if last_rekey.elapsed() >= self.config.rekey_interval {
                return true;
            }
        } else if let Some(established) = self.established_at {
            // First rekey after establishment
            if established.elapsed() >= self.config.rekey_interval {
                return true;
            }
        }

        // Rekey if packet limit exceeded
        if self.packet_counter >= self.config.rekey_packet_limit {
            return true;
        }

        false
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Increment packet counter and return current value
    pub fn next_packet_counter(&mut self) -> u64 {
        let counter = self.packet_counter;
        self.packet_counter += 1;
        counter
    }

    /// Record bytes sent
    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.packets_sent += 1;
        self.update_activity();
    }

    /// Record bytes received
    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.packets_received += 1;
        self.update_activity();
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            state: self.state,
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            packets_sent: self.packets_sent,
            packets_received: self.packets_received,
            stream_count: self.streams.len(),
            established_at: self.established_at,
            last_activity: self.last_activity,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Session statistics snapshot
#[derive(Debug, Clone)]
pub struct SessionStats {
    /// Current session state
    pub state: SessionState,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Number of active streams
    pub stream_count: usize,
    /// When session was established
    pub established_at: Option<Instant>,
    /// Last activity timestamp
    pub last_activity: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_special_values() {
        assert!(ConnectionId::INVALID == ConnectionId::from_bytes([0; 8]));
        assert!(ConnectionId::HANDSHAKE.is_special());
        assert!(ConnectionId::VERSION_NEGOTIATION.is_special());
        assert!(ConnectionId::STATELESS_RESET.is_special());
        assert!(!ConnectionId::INVALID.is_valid());
    }

    #[test]
    fn test_connection_id_rotation() {
        let cid = ConnectionId::from_bytes([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
        let rotated = cid.rotate(0x11111111);

        // High 32 bits should remain the same
        let cid_bytes = cid.to_bytes();
        let rotated_bytes = rotated.to_bytes();
        assert_eq!(cid_bytes[0..4], rotated_bytes[0..4]);

        // Low 32 bits should be XORed
        assert_ne!(cid_bytes[4..8], rotated_bytes[4..8]);
    }

    #[test]
    fn test_connection_id_roundtrip() {
        let original = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let cid = ConnectionId::from_bytes(original);
        assert_eq!(cid.to_bytes(), original);
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert_eq!(session.state(), SessionState::Closed);
        assert_eq!(session.stream_count(), 0);
    }

    #[test]
    fn test_session_initiator_responder() {
        let initiator = Session::new_initiator(SessionConfig::default());
        let responder = Session::new_responder(SessionConfig::default());

        // Initiator uses odd stream IDs
        assert_eq!(initiator.next_stream_id % 2, 1);

        // Responder uses even stream IDs
        assert_eq!(responder.next_stream_id % 2, 0);
    }

    #[test]
    fn test_state_transitions_valid() {
        let mut session = Session::new();

        // Closed -> Handshaking
        assert!(session.can_transition(SessionState::Handshaking(HandshakePhase::InitSent)));
        assert!(
            session
                .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
                .is_ok()
        );

        // Handshaking -> Established
        assert!(session.can_transition(SessionState::Established));
        assert!(session.transition_to(SessionState::Established).is_ok());
        assert_eq!(session.state(), SessionState::Established);
        assert!(session.established_at.is_some());
    }

    #[test]
    fn test_state_transitions_invalid() {
        let mut session = Session::new();

        // Can't go from Closed -> Established directly
        assert!(!session.can_transition(SessionState::Established));
        assert!(session.transition_to(SessionState::Established).is_err());

        // Can't go from Closed -> Rekeying
        assert!(!session.can_transition(SessionState::Rekeying));
    }

    #[test]
    fn test_stream_creation() {
        let mut session = Session::new_initiator(SessionConfig::default());
        session
            .transition_to(SessionState::Established)
            .unwrap_or(());

        // Create first stream
        let stream_id = session.create_stream().unwrap();
        assert_eq!(stream_id, 1);
        assert_eq!(session.stream_count(), 1);

        // Create second stream
        let stream_id2 = session.create_stream().unwrap();
        assert_eq!(stream_id2, 3); // Next odd ID
        assert_eq!(session.stream_count(), 2);
    }

    #[test]
    fn test_stream_management() {
        let mut session = Session::new();
        let stream_id = session.create_stream().unwrap();

        // Get stream
        assert!(session.get_stream(stream_id).is_some());
        assert!(session.get_stream_mut(stream_id).is_some());
        assert!(session.get_stream(999).is_none());

        // Remove stream
        let removed = session.remove_stream(stream_id);
        assert!(removed.is_some());
        assert_eq!(session.stream_count(), 0);
    }

    #[test]
    fn test_max_streams_limit() {
        let mut config = SessionConfig::default();
        config.max_streams = 2;
        let mut session = Session::with_config(config);

        // Create up to max
        assert!(session.create_stream().is_ok());
        assert!(session.create_stream().is_ok());

        // Exceed limit
        assert!(matches!(
            session.create_stream(),
            Err(SessionError::TooManyStreams)
        ));
    }

    #[test]
    fn test_connection_id_assignment() {
        let mut session = Session::new();
        let cid = ConnectionId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8]);

        session.set_connection_id(cid);
        assert_eq!(session.connection_id(), cid);
    }

    #[test]
    fn test_packet_counter() {
        let mut session = Session::new();

        assert_eq!(session.next_packet_counter(), 0);
        assert_eq!(session.next_packet_counter(), 1);
        assert_eq!(session.next_packet_counter(), 2);
    }

    #[test]
    fn test_rekey_needed_time() {
        let mut config = SessionConfig::default();
        config.rekey_interval = Duration::from_millis(10);
        let mut session = Session::new_initiator(config);

        // Establish session properly
        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .unwrap();
        session.transition_to(SessionState::Established).unwrap();

        // Initially no rekey needed
        assert!(!session.needs_rekey());

        // Wait for rekey interval
        std::thread::sleep(Duration::from_millis(15));
        assert!(session.needs_rekey());
    }

    #[test]
    fn test_rekey_needed_packets() {
        let mut config = SessionConfig::default();
        config.rekey_packet_limit = 5;
        let mut session = Session::with_config(config);

        // Send packets
        for _ in 0..5 {
            session.next_packet_counter();
        }

        assert!(session.needs_rekey());
    }

    #[test]
    fn test_activity_tracking() {
        let mut session = Session::new();
        let initial_activity = session.last_activity;

        std::thread::sleep(Duration::from_millis(10));
        session.update_activity();

        assert!(session.last_activity > initial_activity);
    }

    #[test]
    fn test_idle_detection() {
        let mut config = SessionConfig::default();
        config.idle_timeout = Duration::from_millis(10);
        let session = Session::with_config(config);

        // Not idle initially
        assert!(!session.is_idle());

        // Wait for idle timeout
        std::thread::sleep(Duration::from_millis(15));
        assert!(session.is_idle());
    }

    #[test]
    fn test_record_sent_received() {
        let mut session = Session::new();

        session.record_sent(100);
        session.record_sent(200);
        session.record_received(150);

        let stats = session.stats();
        assert_eq!(stats.bytes_sent, 300);
        assert_eq!(stats.bytes_received, 150);
        assert_eq!(stats.packets_sent, 2);
        assert_eq!(stats.packets_received, 1);
    }

    #[test]
    fn test_session_stats() {
        let mut session = Session::new_initiator(SessionConfig::default());
        session.create_stream().unwrap();
        session.create_stream().unwrap();

        let stats = session.stats();
        assert_eq!(stats.state, SessionState::Closed);
        assert_eq!(stats.stream_count, 2);
    }

    #[test]
    fn test_state_cleanup_on_close() {
        let mut session = Session::new();
        session
            .transition_to(SessionState::Established)
            .unwrap_or(());
        session.create_stream().unwrap();
        session.create_stream().unwrap();
        assert_eq!(session.stream_count(), 2);

        session.transition_to(SessionState::Closed).unwrap();
        assert_eq!(session.stream_count(), 0);
    }

    #[test]
    fn test_handshake_phases() {
        let mut session = Session::new();

        // Phase 1: InitSent
        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .unwrap();

        // Can transition within handshake
        assert!(session.can_transition(SessionState::Handshaking(HandshakePhase::InitComplete)));

        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
            .unwrap();

        // Complete handshake
        assert!(session.can_transition(SessionState::Established));
        session.transition_to(SessionState::Established).unwrap();
    }

    #[test]
    fn test_rekey_transition() {
        let mut session = Session::new();
        // Properly establish session
        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .unwrap();
        session.transition_to(SessionState::Established).unwrap();

        // Enter rekeying
        assert!(session.can_transition(SessionState::Rekeying));
        session.transition_to(SessionState::Rekeying).unwrap();
        assert!(session.last_rekey.is_some());

        // Return to established
        assert!(session.can_transition(SessionState::Established));
        session.transition_to(SessionState::Established).unwrap();
    }

    #[test]
    fn test_draining_state() {
        let mut session = Session::new();
        // Properly establish session
        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .unwrap();
        session.transition_to(SessionState::Established).unwrap();

        // Enter draining
        assert!(session.can_transition(SessionState::Draining));
        session.transition_to(SessionState::Draining).unwrap();

        // Can only close from draining
        assert!(session.can_transition(SessionState::Closed));
        assert!(!session.can_transition(SessionState::Established));
    }

    #[test]
    fn test_migration_state() {
        let mut session = Session::new();
        // Properly establish session
        session
            .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
            .unwrap();
        session.transition_to(SessionState::Established).unwrap();

        // Enter migration
        assert!(session.can_transition(SessionState::Migrating));
        session.transition_to(SessionState::Migrating).unwrap();

        // Can return to established or close
        assert!(session.can_transition(SessionState::Established));
        assert!(session.can_transition(SessionState::Closed));
    }
}
