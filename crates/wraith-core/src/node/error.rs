//! Error types for Node API

use thiserror::Error;

/// Errors that can occur in Node operations
#[derive(Debug, Error)]
pub enum NodeError {
    /// Failed to initialize transport layer
    #[error("Transport initialization failed: {0}")]
    TransportInit(String),

    /// Transport operation failed
    #[error("Transport error: {0}")]
    Transport(String),

    /// Cryptographic operation failed
    #[error("Crypto error: {0}")]
    Crypto(#[from] wraith_crypto::CryptoError),

    /// Session establishment failed
    #[error("Session establishment failed: {0}")]
    SessionEstablishment(String),

    /// Session not found
    #[error("Session not found for peer {0:?}")]
    SessionNotFound([u8; 32]),

    /// Transfer operation failed
    #[error("Transfer error: {0}")]
    Transfer(String),

    /// Transfer not found
    #[error("Transfer not found: {0:?}")]
    TransferNotFound([u8; 32]),

    /// File I/O error
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Discovery operation failed
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// NAT traversal failed
    #[error("NAT traversal failed: {0}")]
    NatTraversal(String),

    /// Connection migration failed
    #[error("Connection migration failed: {0}")]
    Migration(String),

    /// Session migration failed
    #[error("Session migration failed: {0}")]
    SessionMigration(String),

    /// Obfuscation operation failed
    #[error("Obfuscation error: {0}")]
    Obfuscation(String),

    /// Hash mismatch
    #[error("Hash mismatch")]
    HashMismatch,

    /// File I/O error (wraith-files)
    #[error("File I/O error: {0}")]
    FileIO(std::io::Error),

    /// Task join error
    #[error("Task join error: {0}")]
    TaskJoin(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Timeout occurred
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Peer not found
    #[error("Peer not found: {0:?}")]
    PeerNotFound([u8; 32]),

    /// Handshake failed
    #[error("Handshake failed: {0}")]
    Handshake(String),

    /// Invalid state transition
    #[error("Invalid state transition: {0}")]
    InvalidState(String),

    /// Channel send/receive error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type for Node operations
pub type Result<T> = std::result::Result<T, NodeError>;
