//! # WRAITH Crypto
//!
//! Cryptographic primitives for the WRAITH protocol.
//!
//! This crate provides:
//! - `Noise_XX` handshake for mutual authentication
//! - `XChaCha20-Poly1305` AEAD encryption
//! - Elligator2 encoding for key indistinguishability
//! - Forward secrecy key ratcheting
//! - Secure random number generation
//!
//! ## Cryptographic Suite
//!
//! | Function | Algorithm | Security Level |
//! |----------|-----------|----------------|
//! | Key Exchange | X25519 | 128-bit |
//! | Key Encoding | Elligator2 | N/A |
//! | AEAD | XChaCha20-Poly1305 | 256-bit key |
//! | Hash | BLAKE3 | 128-bit collision |
//! | KDF | HKDF-BLAKE3 | 128-bit |
//! | Signatures | Ed25519 | 128-bit |

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod aead;
pub mod constant_time;
pub mod elligator;
pub mod error;
pub mod hash;
pub mod noise;
pub mod random;
pub mod ratchet;
pub mod x25519;

pub use error::CryptoError;

/// X25519 public key size
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;

/// X25519 secret key size
pub const X25519_SECRET_KEY_SIZE: usize = 32;

/// Elligator2 representative size
pub const ELLIGATOR_REPR_SIZE: usize = 32;

/// XChaCha20-Poly1305 key size
pub const XCHACHA_KEY_SIZE: usize = 32;

/// XChaCha20-Poly1305 nonce size
pub const XCHACHA_NONCE_SIZE: usize = 24;

/// BLAKE3 output size
pub const BLAKE3_OUTPUT_SIZE: usize = 32;

/// Session keys derived from handshake
#[derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
pub struct SessionKeys {
    /// Key for sending data
    pub send_key: [u8; 32],
    /// Key for receiving data
    pub recv_key: [u8; 32],
    /// Chain key for ratcheting
    pub chain_key: [u8; 32],
}

impl SessionKeys {
    /// Derive connection ID from session keys
    #[must_use]
    pub fn derive_connection_id(&self) -> [u8; 8] {
        let hash = blake3::hash(&self.chain_key);
        let mut cid = [0u8; 8];
        cid.copy_from_slice(&hash.as_bytes()[..8]);
        cid
    }
}
