//! Session encryption state for post-handshake communication.
//!
//! Provides bidirectional encrypted communication with automatic nonce
//! management, replay protection, and key-committing AEAD.
//!
//! # Lint Note
//!
//! This module allows `unused_assignments` because Rust 1.92+ incorrectly flags
//! struct fields with `#[zeroize(skip)]` as "assigned but never read" due to
//! the `ZeroizeOnDrop` derive macro not reading those fields in Drop. The fields
//! ARE actively used in the struct's methods.
#![allow(unused_assignments)]

use super::cipher::{AeadKey, Nonce};
use super::replay::ReplayProtection;
use crate::CryptoError;
use alloc::vec::Vec;
use zeroize::ZeroizeOnDrop;

/// Reusable buffer pool to avoid allocation in hot path.
///
/// Maintains a pool of pre-allocated buffers that can be reused
/// for encryption/decryption operations to reduce memory allocations.
///
/// # Security Note
///
/// The pool is bounded to prevent unbounded memory growth. Buffers
/// exceeding the `max_buffers` limit are dropped instead of being
/// retained in the pool.
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
        // SECURITY: Buffer pre-allocation, immediately overwritten with first 16 bytes of chain_key
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

        // Alice encrypts with explicit counter
        let plaintext = b"test message";
        let ct = alice.encrypt_with_counter(42, plaintext, b"aad").unwrap();

        // Bob decrypts with same counter
        let pt = bob.decrypt_with_counter(42, &ct, b"aad").unwrap();
        assert_eq!(pt, plaintext);

        // Replay should be rejected
        assert!(bob.decrypt_with_counter(42, &ct, b"aad").is_err());
    }

    #[test]
    fn test_session_crypto_rekey() {
        let mut session = SessionCrypto::new([1u8; 32], [2u8; 32], &[3u8; 32]);

        // Encrypt some messages
        let ct1 = session.encrypt(b"msg1", b"").unwrap();
        let ct2 = session.encrypt(b"msg2", b"").unwrap();

        // Update keys
        session.update_keys([4u8; 32], [5u8; 32], &[6u8; 32]);

        // Counter should be reset
        assert_eq!(session.send_counter(), 0);
        assert_eq!(session.recv_counter(), 0);

        // Old ciphertext should fail with new keys (different key commitment)
        let mut bob = SessionCrypto::new([5u8; 32], [4u8; 32], &[6u8; 32]);
        assert!(bob.decrypt(&ct1, b"").is_err());
        assert!(bob.decrypt(&ct2, b"").is_err());
    }

    #[test]
    fn test_buffer_pool() {
        let mut pool = BufferPool::new(1024, 4);

        // Get buffers
        let buf1 = pool.get();
        let buf2 = pool.get();
        assert!(buf1.capacity() >= 1024);
        assert!(buf2.capacity() >= 1024);

        // Return to pool
        pool.put(buf1);
        pool.put(buf2);

        // Get again (should reuse)
        let buf3 = pool.get();
        assert!(buf3.capacity() >= 1024);
    }

    #[test]
    fn test_session_crypto_with_pool() {
        let send_key = [1u8; 32];
        let recv_key = [2u8; 32];
        let chain_key = [3u8; 32];

        let mut alice = SessionCrypto::new(send_key, recv_key, &chain_key);
        let mut bob = SessionCrypto::new(recv_key, send_key, &chain_key);
        let mut pool = BufferPool::new(256, 4);

        // Alice encrypts with pool
        let plaintext = b"pooled message";
        let ct = alice.encrypt_with_pool(&mut pool, plaintext, b"").unwrap();

        // Bob decrypts with pool
        let pt = bob.decrypt_with_pool(&mut pool, &ct, b"").unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_needs_rekey() {
        let mut session = SessionCrypto::new([1u8; 32], [2u8; 32], &[3u8; 32]);

        // Initially doesn't need rekey
        assert!(!session.needs_rekey());

        // Simulate reaching the limit (would need to encrypt 1M messages in real test)
        session.send_counter = 1_000_000;
        assert!(session.needs_rekey());
    }
}
