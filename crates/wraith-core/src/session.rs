//! Session state machine and connection management.
//!
//! A Session represents an authenticated, encrypted connection between
//! two peers. Sessions multiplex multiple streams (file transfers) over
//! a single UDP "connection".

use std::time::Duration;

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
    state: SessionState,
    config: SessionConfig,
    // TODO: Add session implementation fields
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
        }
    }

    /// Get current session state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
