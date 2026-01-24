//! Error types for Node API
//!
//! This module provides a comprehensive error hierarchy for the WRAITH Node API.
//! Errors are categorized to support retry logic and proper error handling.
//!
//! # Error Categories
//!
//! - **Transient**: Temporary failures that may succeed on retry (network timeouts, congestion)
//! - **Permanent**: Failures that will not succeed without intervention (invalid config, not found)
//! - **Retriable**: A subset of transient errors with specific retry semantics
//!
//! # Example
//!
//! ```no_run
//! use wraith_core::node::{NodeError, Result};
//!
//! fn handle_error(err: NodeError) {
//!     if err.is_transient() {
//!         // Consider retrying with backoff
//!         println!("Transient error, may retry: {}", err);
//!     } else {
//!         // Permanent failure, needs user intervention
//!         println!("Permanent error: {}", err);
//!     }
//! }
//! ```

use std::borrow::Cow;
use thiserror::Error;

/// Errors that can occur in Node operations
#[derive(Debug, Error, Clone)]
pub enum NodeError {
    // ============ Transport Errors ============
    /// Failed to initialize transport layer
    #[error("Transport initialization failed: {0}")]
    TransportInit(Cow<'static, str>),

    /// Transport operation failed
    #[error("Transport error: {0}")]
    Transport(Cow<'static, str>),

    // ============ Cryptographic Errors ============
    /// Cryptographic operation failed
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Handshake failed
    #[error("Handshake failed: {0}")]
    Handshake(Cow<'static, str>),

    // ============ Session Errors ============
    /// Session establishment failed
    #[error("Session establishment failed: {0}")]
    SessionEstablishment(Cow<'static, str>),

    /// Session not found
    #[error("Session not found for peer {}", hex::encode(&.0[..8]))]
    SessionNotFound([u8; 32]),

    /// Session migration failed
    #[error("Session migration failed: {0}")]
    SessionMigration(Cow<'static, str>),

    // ============ Transfer Errors ============
    /// Transfer operation failed
    #[error("Transfer error: {0}")]
    Transfer(Cow<'static, str>),

    /// Transfer not found
    #[error("Transfer not found: {}", hex::encode(&.0[..8]))]
    TransferNotFound([u8; 32]),

    /// Hash mismatch during verification
    #[error("Hash mismatch: integrity verification failed")]
    HashMismatch,

    // ============ I/O Errors ============
    /// File I/O error
    #[error("File I/O error: {0}")]
    Io(String),

    // ============ Discovery Errors ============
    /// Discovery operation failed
    #[error("Discovery error: {0}")]
    Discovery(Cow<'static, str>),

    /// NAT traversal failed
    #[error("NAT traversal failed: {0}")]
    NatTraversal(Cow<'static, str>),

    /// Peer not found in DHT or local cache
    #[error("Peer not found: {}", hex::encode(&.0[..8]))]
    PeerNotFound([u8; 32]),

    // ============ Connection Errors ============
    /// Connection migration failed
    #[error("Connection migration failed: {0}")]
    Migration(Cow<'static, str>),

    /// Obfuscation operation failed
    #[error("Obfuscation error: {0}")]
    Obfuscation(Cow<'static, str>),

    // ============ Configuration & State Errors ============
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(Cow<'static, str>),

    /// Invalid state transition
    #[error("Invalid state: {0}")]
    InvalidState(Cow<'static, str>),

    // ============ Operational Errors ============
    /// Operation timed out
    #[error("Operation timed out: {0}")]
    Timeout(Cow<'static, str>),

    /// Task join error
    #[error("Task join error: {0}")]
    TaskJoin(Cow<'static, str>),

    /// Channel send/receive error
    #[error("Channel error: {0}")]
    Channel(Cow<'static, str>),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(Cow<'static, str>),

    /// Generic error for edge cases
    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl NodeError {
    /// Returns `true` if this error is transient and may succeed on retry
    ///
    /// Transient errors include:
    /// - Network timeouts
    /// - Transport failures (may be congestion)
    /// - NAT traversal failures (may succeed with different candidates)
    /// - Channel errors (may be temporary backpressure)
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            NodeError::Timeout(_)
                | NodeError::Transport(_)
                | NodeError::NatTraversal(_)
                | NodeError::Channel(_)
                | NodeError::Migration(_)
        )
    }

    /// Returns `true` if this error is permanent and will not succeed on retry
    ///
    /// Permanent errors include:
    /// - Invalid configuration
    /// - Session/transfer not found
    /// - Hash mismatches (data corruption)
    /// - Cryptographic failures
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            NodeError::InvalidConfig(_)
                | NodeError::SessionNotFound(_)
                | NodeError::TransferNotFound(_)
                | NodeError::PeerNotFound(_)
                | NodeError::HashMismatch
                | NodeError::InvalidState(_)
        )
    }

    /// Returns `true` if this error should trigger a retry with exponential backoff
    #[must_use]
    pub fn should_retry(&self) -> bool {
        self.is_transient() && !matches!(self, NodeError::Timeout(_))
    }

    /// Create a transport error with static context (zero allocation)
    #[must_use]
    pub const fn transport(context: &'static str) -> Self {
        NodeError::Transport(Cow::Borrowed(context))
    }

    /// Create a timeout error with static context (zero allocation)
    #[must_use]
    pub const fn timeout(context: &'static str) -> Self {
        NodeError::Timeout(Cow::Borrowed(context))
    }

    /// Create a handshake error with static context (zero allocation)
    #[must_use]
    pub const fn handshake(context: &'static str) -> Self {
        NodeError::Handshake(Cow::Borrowed(context))
    }

    /// Create an invalid state error with static context (zero allocation)
    #[must_use]
    pub const fn invalid_state(context: &'static str) -> Self {
        NodeError::InvalidState(Cow::Borrowed(context))
    }

    /// Create a discovery error with static context (zero allocation)
    #[must_use]
    pub const fn discovery(context: &'static str) -> Self {
        NodeError::Discovery(Cow::Borrowed(context))
    }

    /// Create a serialization error with static context (zero allocation)
    #[must_use]
    pub const fn serialization(context: &'static str) -> Self {
        NodeError::Serialization(Cow::Borrowed(context))
    }
}

impl From<wraith_crypto::CryptoError> for NodeError {
    fn from(err: wraith_crypto::CryptoError) -> Self {
        NodeError::Crypto(err.to_string())
    }
}

impl From<std::io::Error> for NodeError {
    fn from(err: std::io::Error) -> Self {
        NodeError::Io(err.to_string())
    }
}

/// Result type for Node operations
pub type Result<T> = std::result::Result<T, NodeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_errors() {
        assert!(NodeError::Timeout(Cow::Borrowed("test")).is_transient());
        assert!(NodeError::Transport(Cow::Borrowed("test")).is_transient());
        assert!(NodeError::NatTraversal(Cow::Borrowed("test")).is_transient());
        assert!(NodeError::Channel(Cow::Borrowed("test")).is_transient());
        assert!(NodeError::Migration(Cow::Borrowed("test")).is_transient());
    }

    #[test]
    fn test_permanent_errors() {
        assert!(NodeError::InvalidConfig(Cow::Borrowed("test")).is_permanent());
        assert!(NodeError::SessionNotFound([0u8; 32]).is_permanent());
        assert!(NodeError::TransferNotFound([0u8; 32]).is_permanent());
        assert!(NodeError::PeerNotFound([0u8; 32]).is_permanent());
        assert!(NodeError::HashMismatch.is_permanent());
        assert!(NodeError::InvalidState(Cow::Borrowed("test")).is_permanent());
    }

    #[test]
    fn test_should_retry() {
        // Transient non-timeout errors should retry
        assert!(NodeError::Transport(Cow::Borrowed("test")).should_retry());
        assert!(NodeError::NatTraversal(Cow::Borrowed("test")).should_retry());
        assert!(NodeError::Channel(Cow::Borrowed("test")).should_retry());

        // Timeouts should not auto-retry (caller decides)
        assert!(!NodeError::Timeout(Cow::Borrowed("test")).should_retry());

        // Permanent errors should not retry
        assert!(!NodeError::InvalidConfig(Cow::Borrowed("test")).should_retry());
        assert!(!NodeError::SessionNotFound([0u8; 32]).should_retry());
    }

    #[test]
    fn test_error_display() {
        let mut peer_id = [0u8; 32];
        peer_id[0..8].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        let err = NodeError::SessionNotFound(peer_id);
        assert!(err.to_string().contains("123456789abcdef0"));

        let err = NodeError::HashMismatch;
        assert!(err.to_string().contains("integrity verification"));
    }

    #[test]
    fn test_convenience_constructors() {
        let err = NodeError::transport("connection refused");
        assert!(matches!(err, NodeError::Transport(_)));

        let err = NodeError::timeout("handshake");
        assert!(matches!(err, NodeError::Timeout(_)));

        let err = NodeError::handshake("invalid message");
        assert!(matches!(err, NodeError::Handshake(_)));

        let err = NodeError::invalid_state("not running");
        assert!(matches!(err, NodeError::InvalidState(_)));

        let err = NodeError::discovery("DHT unreachable");
        assert!(matches!(err, NodeError::Discovery(_)));

        let err = NodeError::serialization("invalid JSON");
        assert!(matches!(err, NodeError::Serialization(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let node_err: NodeError = io_err.into();
        assert!(matches!(node_err, NodeError::Io(_)));
    }

    #[test]
    fn test_mutual_exclusivity() {
        // Errors should be either transient or permanent, not both
        let transient_errors = [
            NodeError::Timeout(Cow::Borrowed("test")),
            NodeError::Transport(Cow::Borrowed("test")),
            NodeError::NatTraversal(Cow::Borrowed("test")),
        ];

        for err in &transient_errors {
            assert!(err.is_transient());
            assert!(!err.is_permanent());
        }

        let permanent_errors = [
            NodeError::InvalidConfig(Cow::Borrowed("test")),
            NodeError::SessionNotFound([0u8; 32]),
            NodeError::HashMismatch,
        ];

        for err in &permanent_errors {
            assert!(err.is_permanent());
            assert!(!err.is_transient());
        }
    }
}
