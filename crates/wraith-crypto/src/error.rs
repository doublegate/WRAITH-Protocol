//! Cryptographic error types.

use alloc::string::String;
use thiserror::Error;

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    /// AEAD encryption failed
    #[error("encryption failed")]
    EncryptionFailed,

    /// AEAD decryption failed (authentication failure)
    #[error("decryption failed: authentication failure")]
    DecryptionFailed,

    /// Invalid key length
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// Invalid nonce length
    #[error("invalid nonce length")]
    InvalidNonceLength,

    /// Noise handshake error
    #[error("handshake error: {0}")]
    Handshake(String),

    /// Noise handshake failed
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    /// Invalid state for operation
    #[error("invalid state for operation")]
    InvalidState,

    /// Invalid message format
    #[error("invalid message format: {0}")]
    InvalidMessage(String),

    /// Key derivation failed
    #[error("key derivation failed")]
    KeyDerivationFailed,

    /// Key not encodable with Elligator2
    #[error("key not encodable with Elligator2")]
    NotEncodable,

    /// Random number generation failed
    #[error("random number generation failed")]
    RandomFailed,

    /// Nonce overflow (counter exhausted)
    #[error("nonce counter exhausted, rekey required")]
    NonceOverflow,

    /// Replay attack detected (duplicate sequence number)
    #[error("replay attack detected")]
    ReplayDetected,

    /// Invalid signature
    #[error("invalid signature")]
    InvalidSignature,

    /// Invalid public key
    #[error("invalid public key")]
    InvalidPublicKey,

    /// Invalid parameter
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Random number generation failed with details
    #[error("random generation failed: {0}")]
    RandomGenerationFailed(String),

    /// Invalid key material (corrupted or wrong format)
    #[error("invalid key material")]
    InvalidKeyMaterial,
}
