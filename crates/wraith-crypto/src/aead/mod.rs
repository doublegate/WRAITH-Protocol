//! `XChaCha20-Poly1305` AEAD encryption.
//!
//! Provides authenticated encryption with associated data (AEAD) using
//! `XChaCha20-Poly1305`. Features include:
//! - 256-bit keys
//! - 192-bit nonces (extended nonce for safe random generation)
//! - 128-bit authentication tags
//! - Associated data authentication
//! - In-place encryption/decryption for zero-copy operations
//!
//! ## Security Properties
//!
//! - Confidentiality: `XChaCha20` stream cipher
//! - Integrity: Poly1305 MAC with 128-bit security
//! - Nonce misuse: 192-bit nonce makes random collisions negligible
//!
//! ## Module Organization
//!
//! - [`cipher`] - Core AEAD types (Nonce, Tag, AeadKey, AeadCipher)
//! - [`replay`] - Replay protection with sliding window
//! - [`session`] - Session encryption state (SessionCrypto, BufferPool)
//!
//! ## Usage
//!
//! ```ignore
//! use wraith_crypto::aead::{AeadKey, AeadCipher, Nonce};
//!
//! let key = AeadKey::generate(&mut OsRng);
//! let nonce = Nonce::generate(&mut OsRng);
//!
//! let ciphertext = key.encrypt(&nonce, b"secret", b"aad")?;
//! let plaintext = key.decrypt(&nonce, &ciphertext, b"aad")?;
//! ```

pub mod cipher;
pub mod replay;
pub mod session;

// Re-export all public types for backward compatibility
pub use cipher::{AeadCipher, AeadKey, Nonce, Tag, KEY_SIZE, NONCE_SIZE, TAG_SIZE};
pub use replay::ReplayProtection;
pub use session::{BufferPool, SessionCrypto};
