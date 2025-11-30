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

use crate::CryptoError;
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, AeadInPlace, KeyInit},
};
use rand_core::{CryptoRng, RngCore};
use zeroize::ZeroizeOnDrop;

/// Authentication tag size (16 bytes / 128 bits).
pub const TAG_SIZE: usize = 16;

/// XChaCha20-Poly1305 nonce size (24 bytes / 192 bits).
pub const NONCE_SIZE: usize = 24;

/// AEAD key size (32 bytes / 256 bits).
pub const KEY_SIZE: usize = 32;

/// XChaCha20-Poly1305 nonce (24 bytes).
///
/// The extended 192-bit nonce allows safe random nonce generation
/// without risk of collision (birthday bound is 2^96 messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nonce([u8; NONCE_SIZE]);

impl Nonce {
    /// Create a nonce from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; NONCE_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create a nonce from a slice.
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != NONCE_SIZE {
            return None;
        }
        let mut bytes = [0u8; NONCE_SIZE];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Generate a random nonce.
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Create a nonce from a counter value.
    ///
    /// The counter is placed in the first 8 bytes (little-endian),
    /// with the remaining 16 bytes available for session ID or salt.
    #[must_use]
    pub fn from_counter(counter: u64, salt: &[u8; 16]) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..8].copy_from_slice(&counter.to_le_bytes());
        bytes[8..].copy_from_slice(salt);
        Self(bytes)
    }

    /// Get raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; NONCE_SIZE] {
        &self.0
    }

    /// Get as a reference for chacha20poly1305.
    fn as_generic(&self) -> &chacha20poly1305::XNonce {
        chacha20poly1305::XNonce::from_slice(&self.0)
    }
}

impl Default for Nonce {
    fn default() -> Self {
        Self([0u8; NONCE_SIZE])
    }
}

/// Authentication tag (16 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag([u8; TAG_SIZE]);

impl Tag {
    /// Create a tag from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; TAG_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create from slice.
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != TAG_SIZE {
            return None;
        }
        let mut bytes = [0u8; TAG_SIZE];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Get raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; TAG_SIZE] {
        &self.0
    }
}

/// AEAD encryption key (32 bytes).
///
/// Wraps the raw key material and provides encryption/decryption methods.
/// Key is zeroized on drop.
#[derive(Clone, ZeroizeOnDrop)]
pub struct AeadKey([u8; KEY_SIZE]);

impl AeadKey {
    /// Create a key from raw bytes.
    #[must_use]
    pub fn new(bytes: [u8; KEY_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create from slice.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::InvalidKeyLength` if slice length is not 32 bytes.
    pub fn from_slice(slice: &[u8]) -> Result<Self, CryptoError> {
        if slice.len() != KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_SIZE,
                actual: slice.len(),
            });
        }
        let mut bytes = [0u8; KEY_SIZE];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Generate a random key.
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; KEY_SIZE];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Get raw key bytes.
    ///
    /// # Security
    ///
    /// Handle with extreme care - this exposes the raw key material.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.0
    }

    /// Compute key commitment for key-committing AEAD.
    ///
    /// Returns a 16-byte commitment that binds the ciphertext to this specific key.
    /// This prevents key-commitment attacks where an attacker crafts ciphertexts
    /// that decrypt validly under multiple keys.
    ///
    /// The commitment is computed as: `BLAKE3(key || "wraith-key-commitment")[0..16]`
    #[must_use]
    pub fn commitment(&self) -> [u8; 16] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.0);
        hasher.update(b"wraith-key-commitment");
        let hash = hasher.finalize();
        let mut commitment = [0u8; 16];
        commitment.copy_from_slice(&hash.as_bytes()[..16]);
        commitment
    }

    /// Encrypt plaintext with associated data.
    ///
    /// Returns ciphertext with appended authentication tag (`plaintext.len()` + 16 bytes).
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt(
        &self,
        nonce: &Nonce,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .encrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt ciphertext with associated data.
    ///
    /// Input must include the authentication tag at the end.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt(
        &self,
        nonce: &Nonce,
        ciphertext_and_tag: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if ciphertext_and_tag.len() < TAG_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .decrypt(
                nonce.as_generic(),
                chacha20poly1305::aead::Payload {
                    msg: ciphertext_and_tag,
                    aad,
                },
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Encrypt in-place, returning the authentication tag.
    ///
    /// The buffer is modified in-place to contain the ciphertext.
    /// Returns the authentication tag separately.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut [u8],
        aad: &[u8],
    ) -> Result<Tag, CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        let tag = cipher
            .encrypt_in_place_detached(nonce.as_generic(), aad, buffer)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut tag_bytes = [0u8; TAG_SIZE];
        tag_bytes.copy_from_slice(&tag);
        Ok(Tag(tag_bytes))
    }

    /// Decrypt in-place, verifying the authentication tag.
    ///
    /// The buffer is modified in-place to contain the plaintext.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut [u8],
        tag: &Tag,
        aad: &[u8],
    ) -> Result<(), CryptoError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher
            .decrypt_in_place_detached(
                nonce.as_generic(),
                aad,
                buffer,
                chacha20poly1305::Tag::from_slice(&tag.0),
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

/// AEAD cipher for packet encryption (legacy API).
///
/// Use `AeadKey` directly for new code.
pub struct AeadCipher {
    cipher: XChaCha20Poly1305,
}

impl AeadCipher {
    /// Create a new AEAD cipher with the given key.
    #[must_use]
    pub fn new(key: &[u8; KEY_SIZE]) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(key.into()),
        }
    }

    /// Encrypt plaintext with the given nonce and associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
    pub fn encrypt(
        &self,
        nonce: &[u8; NONCE_SIZE],
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::aead::Payload;

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        self.cipher
            .encrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::EncryptionFailed)
    }

    /// Decrypt ciphertext with the given nonce and associated data.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure.
    pub fn decrypt(
        &self,
        nonce: &[u8; NONCE_SIZE],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::aead::Payload;

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        self.cipher
            .decrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

/// Replay protection using sliding window.
///
/// Tracks seen packet sequence numbers to prevent replay attacks.
/// Uses a 64-bit window for efficient out-of-order packet handling.
#[derive(Clone)]
pub struct ReplayProtection {
    /// Maximum sequence number seen
    max_seq: u64,
    /// Sliding window bitmap (64 bits = 64 packets)
    window: u64,
}

impl ReplayProtection {
    /// Size of the replay protection window
    pub const WINDOW_SIZE: u64 = 64;

    /// Create a new replay protection window
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_seq: 0,
            window: 0,
        }
    }

    /// Check if a sequence number is acceptable and update the window.
    ///
    /// Returns `true` if the packet should be accepted (not a replay).
    /// Returns `false` if the packet is a replay or too old.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut rp = ReplayProtection::new();
    ///
    /// assert!(rp.check_and_update(1)); // First packet
    /// assert!(!rp.check_and_update(1)); // Replay - rejected
    /// assert!(rp.check_and_update(2)); // Next packet
    /// assert!(rp.check_and_update(65)); // Jump ahead
    /// assert!(!rp.check_and_update(1)); // Too old - rejected
    /// ```
    pub fn check_and_update(&mut self, seq: u64) -> bool {
        // Packet is too old (beyond window)
        // Use <= to prevent bit_position from being exactly WINDOW_SIZE (64), which would overflow
        if seq + Self::WINDOW_SIZE <= self.max_seq {
            return false;
        }

        // Packet is newer than max_seq (advance window)
        if seq > self.max_seq {
            let shift = seq - self.max_seq;

            if shift >= Self::WINDOW_SIZE {
                // Shift is >= window size, reset window completely
                self.window = 1;
            } else {
                // Shift window (safe because shift < 64)
                self.window <<= shift;
                self.window |= 1; // Mark current max_seq as seen
            }

            self.max_seq = seq;
            return true;
        }

        // Packet is within window (seq <= max_seq)
        let bit_position = self.max_seq - seq;

        // Check if already seen
        if self.window & (1 << bit_position) != 0 {
            return false; // Replay detected
        }

        // Mark as seen
        self.window |= 1 << bit_position;
        true
    }

    /// Get the maximum sequence number seen
    #[must_use]
    pub fn max_seq(&self) -> u64 {
        self.max_seq
    }

    /// Reset the replay protection window
    pub fn reset(&mut self) {
        self.max_seq = 0;
        self.window = 0;
    }
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new()
    }
}

/// Reusable buffer pool to avoid allocation in hot path.
///
/// Maintains a pool of pre-allocated buffers that can be reused
/// for encryption/decryption operations to reduce memory allocations.
pub struct BufferPool {
    buffers: Vec<Vec<u8>>,
    default_capacity: usize,
    max_buffers: usize,
}

impl BufferPool {
    /// Create a new buffer pool.
    ///
    /// # Arguments
    ///
    /// * `default_capacity` - Default capacity for new buffers
    /// * `max_buffers` - Maximum number of buffers to keep in the pool
    #[must_use]
    pub fn new(default_capacity: usize, max_buffers: usize) -> Self {
        Self {
            buffers: Vec::with_capacity(max_buffers),
            default_capacity,
            max_buffers,
        }
    }

    /// Get a buffer from pool (or allocate if empty).
    ///
    /// Returns a buffer with at least `default_capacity` capacity.
    /// The buffer is cleared before being returned.
    pub fn get(&mut self) -> Vec<u8> {
        self.buffers
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.default_capacity))
    }

    /// Return buffer to pool for reuse.
    ///
    /// The buffer is cleared and returned to the pool if the pool
    /// is not full. Otherwise, the buffer is dropped.
    pub fn put(&mut self, mut buffer: Vec<u8>) {
        if self.buffers.len() < self.max_buffers {
            buffer.clear();
            self.buffers.push(buffer);
        }
        // If pool is full, buffer is dropped
    }
}

/// Session encryption state for post-handshake communication.
///
/// Manages nonce counters and keys for bidirectional encrypted communication.
#[derive(ZeroizeOnDrop)]
pub struct SessionCrypto {
    /// Key for sending messages
    send_key: AeadKey,
    /// Key for receiving messages
    recv_key: AeadKey,
    /// Nonce salt (derived from session)
    #[zeroize(skip)]
    nonce_salt: [u8; 16],
    /// Send nonce counter
    #[zeroize(skip)]
    send_counter: u64,
    /// Receive nonce counter
    #[zeroize(skip)]
    recv_counter: u64,
    /// Maximum allowed counter before rekey
    #[zeroize(skip)]
    max_counter: u64,
    /// Replay protection for received packets
    #[zeroize(skip)]
    replay_protection: ReplayProtection,
}

impl SessionCrypto {
    /// Create a new session crypto state from session keys.
    #[must_use]
    pub fn new(send_key: [u8; 32], recv_key: [u8; 32], chain_key: &[u8; 32]) -> Self {
        // Derive nonce salt from chain key
        let mut nonce_salt = [0u8; 16];
        nonce_salt.copy_from_slice(&chain_key[..16]);

        Self {
            send_key: AeadKey::new(send_key),
            recv_key: AeadKey::new(recv_key),
            nonce_salt,
            send_counter: 0,
            recv_counter: 0,
            max_counter: 1_000_000, // Rekey after 1M messages
            replay_protection: ReplayProtection::new(),
        }
    }

    /// Encrypt a message with key commitment.
    ///
    /// Returns the ciphertext with authentication tag.
    /// Automatically increments the send counter.
    /// Includes key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::NonceOverflow` if send counter is exhausted.
    /// Returns `CryptoError::EncryptionFailed` on AEAD encryption failure.
    pub fn encrypt(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if self.send_counter >= self.max_counter {
            return Err(CryptoError::NonceOverflow);
        }

        let nonce = Nonce::from_counter(self.send_counter, &self.nonce_salt);
        self.send_counter += 1;

        // Prepend key commitment to AAD for key-committing AEAD
        let commitment = self.send_key.commitment();
        let mut committed_aad = Vec::with_capacity(commitment.len() + aad.len());
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        self.send_key.encrypt(&nonce, plaintext, &committed_aad)
    }

    /// Encrypt a message with explicit counter and key commitment.
    ///
    /// Returns the ciphertext and the counter used.
    /// Does NOT automatically increment the counter (caller's responsibility).
    /// Includes key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::EncryptionFailed` on AEAD encryption failure.
    pub fn encrypt_with_counter(
        &self,
        counter: u64,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let nonce = Nonce::from_counter(counter, &self.nonce_salt);

        // Prepend key commitment to AAD for key-committing AEAD
        let commitment = self.send_key.commitment();
        let mut committed_aad = Vec::with_capacity(commitment.len() + aad.len());
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        self.send_key.encrypt(&nonce, plaintext, &committed_aad)
    }

    /// Decrypt a message with key commitment verification.
    ///
    /// Uses the receive counter for nonce generation.
    /// Automatically increments the receive counter.
    /// Verifies key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::NonceOverflow` if receive counter is exhausted.
    /// Returns `CryptoError::DecryptionFailed` on authentication failure (including wrong key commitment).
    pub fn decrypt(&mut self, ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if self.recv_counter >= self.max_counter {
            return Err(CryptoError::NonceOverflow);
        }

        let nonce = Nonce::from_counter(self.recv_counter, &self.nonce_salt);
        self.recv_counter += 1;

        // Prepend key commitment to AAD for key-committing AEAD
        let commitment = self.recv_key.commitment();
        let mut committed_aad = Vec::with_capacity(commitment.len() + aad.len());
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        self.recv_key.decrypt(&nonce, ciphertext, &committed_aad)
    }

    /// Decrypt a message with explicit counter and key commitment verification.
    ///
    /// Does NOT automatically increment the counter.
    /// Checks replay protection - packets with duplicate or old sequence numbers are rejected.
    /// Verifies key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::DecryptionFailed` on authentication failure (including wrong key commitment).
    /// Returns `CryptoError::ReplayDetected` if the sequence number has already been seen.
    pub fn decrypt_with_counter(
        &mut self,
        counter: u64,
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // Check replay protection first (before decryption to prevent DoS)
        if !self.replay_protection.check_and_update(counter) {
            return Err(CryptoError::ReplayDetected);
        }

        let nonce = Nonce::from_counter(counter, &self.nonce_salt);

        // Prepend key commitment to AAD for key-committing AEAD
        let commitment = self.recv_key.commitment();
        let mut committed_aad = Vec::with_capacity(commitment.len() + aad.len());
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        self.recv_key.decrypt(&nonce, ciphertext, &committed_aad)
    }

    /// Get the current send counter.
    #[must_use]
    pub fn send_counter(&self) -> u64 {
        self.send_counter
    }

    /// Get the current receive counter.
    #[must_use]
    pub fn recv_counter(&self) -> u64 {
        self.recv_counter
    }

    /// Check if rekey is needed.
    #[must_use]
    pub fn needs_rekey(&self) -> bool {
        self.send_counter >= self.max_counter || self.recv_counter >= self.max_counter
    }

    /// Update keys for a new session (ratchet).
    pub fn update_keys(&mut self, send_key: [u8; 32], recv_key: [u8; 32], chain_key: &[u8; 32]) {
        self.send_key = AeadKey::new(send_key);
        self.recv_key = AeadKey::new(recv_key);
        self.nonce_salt.copy_from_slice(&chain_key[..16]);
        self.send_counter = 0;
        self.recv_counter = 0;
        self.replay_protection.reset();
    }

    /// Encrypt a message using a buffer from the pool.
    ///
    /// Returns the ciphertext with authentication tag.
    /// Automatically increments the send counter.
    /// Includes key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::NonceOverflow` if send counter is exhausted.
    /// Returns `CryptoError::EncryptionFailed` on AEAD encryption failure.
    pub fn encrypt_with_pool(
        &mut self,
        pool: &mut BufferPool,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if self.send_counter >= self.max_counter {
            return Err(CryptoError::NonceOverflow);
        }

        let nonce = Nonce::from_counter(self.send_counter, &self.nonce_salt);
        self.send_counter += 1;

        // Get buffer from pool for AAD
        let commitment = self.send_key.commitment();
        let mut committed_aad = pool.get();
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        // Encrypt
        let result = self.send_key.encrypt(&nonce, plaintext, &committed_aad);

        // Return buffer to pool
        pool.put(committed_aad);

        result
    }

    /// Decrypt a message using a buffer from the pool.
    ///
    /// Uses the receive counter for nonce generation.
    /// Automatically increments the receive counter.
    /// Verifies key commitment in AAD to prevent key-commitment attacks.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::NonceOverflow` if receive counter is exhausted.
    /// Returns `CryptoError::DecryptionFailed` on authentication failure (including wrong key commitment).
    pub fn decrypt_with_pool(
        &mut self,
        pool: &mut BufferPool,
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if self.recv_counter >= self.max_counter {
            return Err(CryptoError::NonceOverflow);
        }

        let nonce = Nonce::from_counter(self.recv_counter, &self.nonce_salt);
        self.recv_counter += 1;

        // Get buffer from pool for AAD
        let commitment = self.recv_key.commitment();
        let mut committed_aad = pool.get();
        committed_aad.extend_from_slice(&commitment);
        committed_aad.extend_from_slice(aad);

        // Decrypt
        let result = self.recv_key.decrypt(&nonce, ciphertext, &committed_aad);

        // Return buffer to pool
        pool.put(committed_aad);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_aead_roundtrip() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"Hello, WRAITH!";
        let aad = b"additional data";

        let cipher = AeadCipher::new(&key);

        let ciphertext = cipher.encrypt(&nonce, plaintext, aad).unwrap();
        let decrypted = cipher.decrypt(&nonce, &ciphertext, aad).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_tamper_detection() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"Hello, WRAITH!";
        let aad = b"additional data";

        let cipher = AeadCipher::new(&key);

        let mut ciphertext = cipher.encrypt(&nonce, plaintext, aad).unwrap();
        ciphertext[0] ^= 0xFF; // Tamper with ciphertext

        assert!(cipher.decrypt(&nonce, &ciphertext, aad).is_err());
    }

    #[test]
    fn test_aead_key_encrypt_decrypt() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);
        let plaintext = b"secret message";
        let aad = b"header";

        let ciphertext = key.encrypt(&nonce, plaintext, aad).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_SIZE);

        let decrypted = key.decrypt(&nonce, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_wrong_key_fails() {
        let key1 = AeadKey::generate(&mut OsRng);
        let key2 = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);

        let ciphertext = key1.encrypt(&nonce, b"secret", b"").unwrap();
        assert!(key2.decrypt(&nonce, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_nonce_fails() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce1 = Nonce::generate(&mut OsRng);
        let nonce2 = Nonce::generate(&mut OsRng);

        let ciphertext = key.encrypt(&nonce1, b"secret", b"").unwrap();
        assert!(key.decrypt(&nonce2, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_aad_fails() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);

        let ciphertext = key.encrypt(&nonce, b"secret", b"aad1").unwrap();
        assert!(key.decrypt(&nonce, &ciphertext, b"aad2").is_err());
    }

    #[test]
    fn test_aead_in_place() {
        let key = AeadKey::generate(&mut OsRng);
        let nonce = Nonce::generate(&mut OsRng);
        let plaintext = b"hello world";
        let mut buffer = plaintext.to_vec();

        let tag = key.encrypt_in_place(&nonce, &mut buffer, b"").unwrap();
        assert_ne!(&buffer, plaintext);

        key.decrypt_in_place(&nonce, &mut buffer, &tag, b"")
            .unwrap();
        assert_eq!(&buffer, plaintext);
    }

    #[test]
    fn test_nonce_from_counter() {
        let salt = [0x42u8; 16];
        let nonce1 = Nonce::from_counter(0, &salt);
        let nonce2 = Nonce::from_counter(1, &salt);
        let nonce3 = Nonce::from_counter(0, &salt);

        assert_ne!(nonce1.as_bytes(), nonce2.as_bytes());
        assert_eq!(nonce1.as_bytes(), nonce3.as_bytes());
    }

    #[test]
    fn test_session_crypto_encrypt_decrypt() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        // Create two sessions with swapped keys
        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice sends to Bob
        let plaintext1 = b"hello from alice";
        let ct1 = alice.encrypt(plaintext1, b"").unwrap();
        let pt1 = bob.decrypt(&ct1, b"").unwrap();
        assert_eq!(pt1, plaintext1);

        // Bob sends to Alice
        let plaintext2 = b"hello from bob";
        let ct2 = bob.encrypt(plaintext2, b"").unwrap();
        let pt2 = alice.decrypt(&ct2, b"").unwrap();
        assert_eq!(pt2, plaintext2);
    }

    #[test]
    fn test_session_crypto_counter_increment() {
        let mut session = SessionCrypto::new([1u8; 32], [2u8; 32], &[3u8; 32]);

        assert_eq!(session.send_counter(), 0);
        let _ = session.encrypt(b"test", b"").unwrap();
        assert_eq!(session.send_counter(), 1);
        let _ = session.encrypt(b"test", b"").unwrap();
        assert_eq!(session.send_counter(), 2);
    }

    #[test]
    fn test_session_crypto_with_explicit_counter() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice encrypts with counter 5
        let ct = alice.encrypt_with_counter(5, b"message", b"").unwrap();

        // Bob decrypts with counter 5
        let pt = bob.decrypt_with_counter(5, &ct, b"").unwrap();
        assert_eq!(pt, b"message");
    }

    #[test]
    fn test_tag_from_slice() {
        let bytes = [0x42u8; TAG_SIZE];
        let tag = Tag::from_slice(&bytes).unwrap();
        assert_eq!(tag.as_bytes(), &bytes);

        // Wrong size should fail
        assert!(Tag::from_slice(&[0u8; 15]).is_none());
    }

    #[test]
    fn test_nonce_from_slice() {
        let bytes = [0x42u8; NONCE_SIZE];
        let nonce = Nonce::from_slice(&bytes).unwrap();
        assert_eq!(nonce.as_bytes(), &bytes);

        // Wrong size should fail
        assert!(Nonce::from_slice(&[0u8; 23]).is_none());
    }

    // ==================== Replay Protection Tests ====================

    #[test]
    fn test_replay_protection_normal_sequential() {
        let mut rp = ReplayProtection::new();

        // Accept sequential packets
        assert!(rp.check_and_update(1));
        assert_eq!(rp.max_seq(), 1);

        assert!(rp.check_and_update(2));
        assert_eq!(rp.max_seq(), 2);

        assert!(rp.check_and_update(3));
        assert_eq!(rp.max_seq(), 3);

        assert!(rp.check_and_update(4));
        assert_eq!(rp.max_seq(), 4);
    }

    #[test]
    fn test_replay_protection_out_of_order_within_window() {
        let mut rp = ReplayProtection::new();

        // Accept packet 10
        assert!(rp.check_and_update(10));
        assert_eq!(rp.max_seq(), 10);

        // Accept packet 5 (within window, out-of-order)
        assert!(rp.check_and_update(5));
        assert_eq!(rp.max_seq(), 10); // max_seq unchanged

        // Accept packet 8 (within window)
        assert!(rp.check_and_update(8));
        assert_eq!(rp.max_seq(), 10);

        // Accept packet 9 (within window)
        assert!(rp.check_and_update(9));
        assert_eq!(rp.max_seq(), 10);

        // Accept packet 15 (new max)
        assert!(rp.check_and_update(15));
        assert_eq!(rp.max_seq(), 15);
    }

    #[test]
    fn test_replay_protection_replay_rejection() {
        let mut rp = ReplayProtection::new();

        // Accept packet 5
        assert!(rp.check_and_update(5));

        // Reject duplicate packet 5 (replay)
        assert!(!rp.check_and_update(5));

        // Accept packet 10
        assert!(rp.check_and_update(10));

        // Reject duplicate packet 10 (replay)
        assert!(!rp.check_and_update(10));

        // Accept packet 7 (out-of-order, within window)
        assert!(rp.check_and_update(7));

        // Reject duplicate packet 7 (replay)
        assert!(!rp.check_and_update(7));
    }

    #[test]
    fn test_replay_protection_old_packet_rejection() {
        let mut rp = ReplayProtection::new();

        // Accept packet 100
        assert!(rp.check_and_update(100));
        assert_eq!(rp.max_seq(), 100);

        // Packet 35 is beyond the 64-packet window (100 - 64 = 36)
        // 35 + 64 = 99, which is < 100, so it should be rejected
        assert!(!rp.check_and_update(35));

        // Packet 36 is exactly at the window boundary (edge case)
        // 36 + 64 = 100, which is <= 100, so it should be rejected
        assert!(!rp.check_and_update(36));

        // Packet 37 is within the window
        // 37 + 64 = 101, which is > 100, so it should be accepted
        assert!(rp.check_and_update(37));

        // Packet 1 is way too old
        assert!(!rp.check_and_update(1));
    }

    #[test]
    fn test_replay_protection_window_shift() {
        let mut rp = ReplayProtection::new();

        // Accept packets 1-5
        for i in 1..=5 {
            assert!(rp.check_and_update(i));
        }

        // Jump to packet 70 (shift window by 65, more than window size)
        assert!(rp.check_and_update(70));
        assert_eq!(rp.max_seq(), 70);

        // Packet 6 is exactly at the window boundary (edge case)
        // 6 + 64 = 70, which is <= 70, so it should be rejected
        assert!(!rp.check_and_update(6));

        // Packet 7 is within the window
        // 7 + 64 = 71, which is > 70, so it should be accepted
        assert!(rp.check_and_update(7));

        // Packet 5 is too old
        assert!(!rp.check_and_update(5));
    }

    #[test]
    fn test_replay_protection_large_jump() {
        let mut rp = ReplayProtection::new();

        // Start at 10
        assert!(rp.check_and_update(10));

        // Jump to 1000 (shift > window size)
        assert!(rp.check_and_update(1000));
        assert_eq!(rp.max_seq(), 1000);

        // Packets beyond the window should be rejected
        // 935 + 64 = 999, which is < 1000, so rejected
        assert!(!rp.check_and_update(935));

        // Packet 936 is exactly at the window boundary (edge case)
        // 936 + 64 = 1000, which is <= 1000, so rejected
        assert!(!rp.check_and_update(936));

        // Packet 937 is within the window
        // 937 + 64 = 1001, which is > 1000, so accepted
        assert!(rp.check_and_update(937));

        // Packet 10 is way too old
        assert!(!rp.check_and_update(10));
    }

    #[test]
    fn test_replay_protection_reset() {
        let mut rp = ReplayProtection::new();

        // Accept some packets
        assert!(rp.check_and_update(1));
        assert!(rp.check_and_update(2));
        assert!(rp.check_and_update(3));
        assert_eq!(rp.max_seq(), 3);

        // Reject replay
        assert!(!rp.check_and_update(2));

        // Reset
        rp.reset();
        assert_eq!(rp.max_seq(), 0);

        // After reset, packet 2 should be accepted again
        assert!(rp.check_and_update(2));
        assert_eq!(rp.max_seq(), 2);
    }

    #[test]
    fn test_replay_protection_zero_sequence() {
        let mut rp = ReplayProtection::new();

        // Sequence 0 should work (edge case)
        assert!(rp.check_and_update(0));
        assert_eq!(rp.max_seq(), 0);

        // Duplicate should be rejected
        assert!(!rp.check_and_update(0));
    }

    #[test]
    fn test_session_crypto_replay_detection() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice encrypts message with counter 5
        let ct = alice.encrypt_with_counter(5, b"message", b"").unwrap();

        // Bob decrypts successfully the first time
        assert!(bob.decrypt_with_counter(5, &ct, b"").is_ok());

        // Bob attempts to decrypt the same message again (replay attack)
        let result = bob.decrypt_with_counter(5, &ct, b"");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CryptoError::ReplayDetected));
    }

    #[test]
    fn test_session_crypto_out_of_order_with_replay_protection() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice sends messages with counters 10, 5, 8
        let ct10 = alice.encrypt_with_counter(10, b"msg10", b"").unwrap();
        let ct5 = alice.encrypt_with_counter(5, b"msg5", b"").unwrap();
        let ct8 = alice.encrypt_with_counter(8, b"msg8", b"").unwrap();

        // Bob receives out of order: 10 first
        assert!(bob.decrypt_with_counter(10, &ct10, b"").is_ok());

        // Then 5 (out-of-order, but within window)
        assert!(bob.decrypt_with_counter(5, &ct5, b"").is_ok());

        // Then 8 (out-of-order, within window)
        assert!(bob.decrypt_with_counter(8, &ct8, b"").is_ok());

        // Replay of 5 should be rejected
        assert!(matches!(
            bob.decrypt_with_counter(5, &ct5, b""),
            Err(CryptoError::ReplayDetected)
        ));
    }

    #[test]
    fn test_session_crypto_old_packet_rejection() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice sends packet with counter 100
        let ct100 = alice.encrypt_with_counter(100, b"msg100", b"").unwrap();
        assert!(bob.decrypt_with_counter(100, &ct100, b"").is_ok());

        // Alice sends old packet with counter 35 (beyond 64-packet window)
        let ct35 = alice.encrypt_with_counter(35, b"msg35", b"").unwrap();
        let result = bob.decrypt_with_counter(35, &ct35, b"");
        assert!(matches!(result, Err(CryptoError::ReplayDetected)));
    }

    #[test]
    fn test_session_crypto_rekey_resets_replay_protection() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice sends message with counter 5
        let ct = alice.encrypt_with_counter(5, b"message", b"").unwrap();
        assert!(bob.decrypt_with_counter(5, &ct, b"").is_ok());

        // Replay should be rejected
        assert!(matches!(
            bob.decrypt_with_counter(5, &ct, b""),
            Err(CryptoError::ReplayDetected)
        ));

        // Bob ratchets to new keys
        let new_send_key = [10u8; 32];
        let new_recv_key = [20u8; 32];
        let new_chain_key = [30u8; 32];
        bob.update_keys(new_send_key, new_recv_key, &new_chain_key);

        // After rekey, counter 5 can be used again (new session)
        let alice_new = SessionCrypto::new(new_recv_key, new_send_key, &new_chain_key);
        let ct_new = alice_new
            .encrypt_with_counter(5, b"new_message", b"")
            .unwrap();
        assert!(bob.decrypt_with_counter(5, &ct_new, b"").is_ok());
    }

    // ==================== Key Commitment Tests ====================

    #[test]
    fn test_key_commitment_deterministic() {
        let key_bytes = [0x42u8; 32];
        let key1 = AeadKey::new(key_bytes);
        let key2 = AeadKey::new(key_bytes);

        // Same key should produce same commitment
        assert_eq!(key1.commitment(), key2.commitment());
    }

    #[test]
    fn test_key_commitment_different_keys() {
        let key1 = AeadKey::new([1u8; 32]);
        let key2 = AeadKey::new([2u8; 32]);

        // Different keys should produce different commitments
        assert_ne!(key1.commitment(), key2.commitment());
    }

    #[test]
    fn test_key_commitment_prevents_key_substitution() {
        let send_key1 = [1u8; 32];
        let recv_key1 = [2u8; 32];
        let chain_key = [3u8; 32];

        // Alice encrypts with key1
        let alice = SessionCrypto::new(send_key1, recv_key1, &chain_key);
        let ct = alice.encrypt_with_counter(1, b"secret", b"").unwrap();

        // Bob tries to decrypt with key2 (wrong key)
        let send_key2 = [99u8; 32];
        let recv_key2 = [2u8; 32]; // Different send key
        let mut bob = SessionCrypto::new(recv_key2, send_key2, &chain_key);

        // Should fail due to key commitment mismatch
        assert!(bob.decrypt_with_counter(1, &ct, b"").is_err());
    }

    #[test]
    fn test_key_commitment_in_session_crypto() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Alice encrypts
        let plaintext = b"test message with key commitment";
        let ct = alice.encrypt(plaintext, b"aad").unwrap();

        // Bob decrypts successfully (same keys)
        let pt = bob.decrypt(&ct, b"aad").unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_key_commitment_with_wrong_aad() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Encrypt with one AAD
        let ct = alice.encrypt(b"test", b"aad1").unwrap();

        // Decrypt with different AAD should fail (commitment + AAD mismatch)
        assert!(bob.decrypt(&ct, b"aad2").is_err());
    }

    #[test]
    fn test_key_commitment_empty_aad() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        // Encrypt with empty AAD (only key commitment in AAD)
        let ct = alice.encrypt(b"test", b"").unwrap();

        // Should decrypt successfully
        let pt = bob.decrypt(&ct, b"").unwrap();
        assert_eq!(pt, b"test");
    }

    #[test]
    fn test_key_commitment_size() {
        let key = AeadKey::generate(&mut OsRng);
        let commitment = key.commitment();

        // Commitment should be 16 bytes
        assert_eq!(commitment.len(), 16);
    }

    #[test]
    fn test_key_commitment_cross_key_attack_prevention() {
        // This test simulates an attack where an adversary tries to craft
        // a ciphertext that decrypts validly under two different keys

        let key1 = [0x01u8; 32];
        let key2 = [0x02u8; 32];
        let chain_key = [0x03u8; 32];

        let alice = SessionCrypto::new(key1, [0u8; 32], &chain_key);
        let mut bob1 = SessionCrypto::new([0u8; 32], key1, &chain_key);
        let mut bob2 = SessionCrypto::new([0u8; 32], key2, &chain_key);

        // Alice encrypts with key1
        let ct = alice.encrypt_with_counter(1, b"message", b"").unwrap();

        // Bob1 can decrypt (correct key)
        assert!(bob1.decrypt_with_counter(1, &ct, b"").is_ok());

        // Bob2 cannot decrypt (wrong key, key commitment mismatch)
        assert!(bob2.decrypt_with_counter(1, &ct, b"").is_err());
    }

    // ==================== Buffer Pool Tests ====================

    #[test]
    fn test_buffer_pool_reuse() {
        let mut pool = BufferPool::new(1024, 8);

        // Get a buffer
        let mut buf1 = pool.get();
        assert!(buf1.capacity() >= 1024);
        assert_eq!(buf1.len(), 0);

        // Use it
        buf1.extend_from_slice(b"test data");

        // Return it
        pool.put(buf1);

        // Get it back (should be reused and cleared)
        let buf2 = pool.get();
        assert_eq!(buf2.len(), 0);
        assert!(buf2.capacity() >= 1024);
    }

    #[test]
    fn test_buffer_pool_capacity_respected() {
        let mut pool = BufferPool::new(512, 2);

        // Get and return 3 buffers
        let buf1 = pool.get();
        let buf2 = pool.get();
        let buf3 = pool.get();

        pool.put(buf1);
        pool.put(buf2);
        pool.put(buf3); // This should be dropped (pool full)

        // Pool should only have 2 buffers
        assert_eq!(pool.buffers.len(), 2);
    }

    #[test]
    fn test_encrypt_decrypt_with_pool() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        let mut pool = BufferPool::new(256, 4);

        // Alice encrypts using pool
        let plaintext = b"hello from alice with buffer pool";
        let ct = alice.encrypt_with_pool(&mut pool, plaintext, b"").unwrap();

        // Bob decrypts using pool
        let pt = bob.decrypt_with_pool(&mut pool, &ct, b"").unwrap();
        assert_eq!(pt, plaintext);

        // Pool should have buffers returned
        assert!(!pool.buffers.is_empty());
    }

    #[test]
    fn test_buffer_pool_multiple_operations() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);

        let mut pool = BufferPool::new(256, 4);

        // Multiple encrypt/decrypt cycles
        for i in 0..10 {
            let plaintext = format!("message {}", i);
            let ct = alice
                .encrypt_with_pool(&mut pool, plaintext.as_bytes(), b"")
                .unwrap();
            let pt = bob.decrypt_with_pool(&mut pool, &ct, b"").unwrap();
            assert_eq!(pt, plaintext.as_bytes());
        }

        // Pool should be reusing buffers (not growing unbounded)
        assert!(pool.buffers.len() <= 4);
    }
}
