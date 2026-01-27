//! Forward secrecy key ratcheting.
//!
//! Implements the Double Ratchet algorithm for continuous forward secrecy
//! and post-compromise security. The Double Ratchet combines:
//!
//! - **Symmetric Ratchet**: Derives new encryption keys for each message
//! - **DH Ratchet**: Performs new Diffie-Hellman exchanges to limit key compromise
//!
//! ## Algorithm Overview
//!
//! Each party maintains:
//! - A root key that evolves with each DH ratchet step
//! - Separate sending and receiving chain keys
//! - Current DH keypair for the DH ratchet
//!
//! # Lint Note
//!
//! This module allows `unused_assignments` because Rust 1.92+ incorrectly flags
//! struct fields with `#[zeroize(skip)]` as "assigned but never read" due to
//! the `ZeroizeOnDrop` derive macro not reading those fields in Drop. The fields
//! ARE actively used in the struct's methods.
#![allow(unused_assignments)]
//!
//! ## Security Properties
//!
//! - **Forward Secrecy**: Compromising current keys doesn't reveal past messages
//! - **Post-Compromise Security**: Session recovers security after temporary compromise
//! - **Out-of-Order Messages**: Skipped message keys can be stored for later delivery
//!
//! ## Reference
//!
//! Based on "The Double Ratchet Algorithm" by Perrin & Marlinspike (2016)

use crate::CryptoError;
use crate::aead::{AeadKey, Nonce};
use crate::constant_time::ct_eq;
use crate::hash::{hkdf_expand, hkdf_extract};
use crate::x25519::{PrivateKey, PublicKey};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;
use rand_core::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Maximum number of skipped message keys to store.
/// Prevents memory exhaustion from malicious message counters.
const MAX_SKIP: u64 = 1000;

/// Maximum gap allowed when skipping messages.
/// Prevents `DoS` by limiting how far ahead we'll pre-compute keys.
const MAX_SKIP_GAP: u64 = 100;

/// Symmetric key ratchet for deriving per-message keys.
///
/// The symmetric ratchet derives a sequence of message keys from a chain key.
/// Each call to `next_key()` advances the ratchet and returns a new key.
///
/// # Example
///
/// ```ignore
/// use wraith_crypto::ratchet::SymmetricRatchet;
///
/// let mut ratchet = SymmetricRatchet::new(&root_key);
/// let key1 = ratchet.next_key();  // First message key
/// let key2 = ratchet.next_key();  // Second message key (different from key1)
/// ```
#[derive(ZeroizeOnDrop)]
pub struct SymmetricRatchet {
    /// Current chain key
    chain_key: [u8; 32],
    /// Message counter
    counter: u64,
}

impl SymmetricRatchet {
    /// Create a new symmetric ratchet from a root/chain key.
    #[must_use]
    pub fn new(chain_key: &[u8; 32]) -> Self {
        Self {
            chain_key: *chain_key,
            counter: 0,
        }
    }

    /// Derive the next message key and advance the ratchet.
    ///
    /// Returns a 32-byte key suitable for use with AEAD encryption.
    /// The internal state is updated so subsequent calls produce different keys.
    pub fn next_key(&mut self) -> MessageKey {
        // Derive message key: HKDF-Expand(chain_key, "message")
        let mut message_key = [0u8; 32];
        hkdf_expand(&self.chain_key, b"wraith_message_key", &mut message_key);

        // Derive next chain key: HKDF-Expand(chain_key, "chain")
        let mut next_chain = [0u8; 32];
        hkdf_expand(&self.chain_key, b"wraith_chain_key", &mut next_chain);

        // Update state (old chain key is zeroized)
        self.chain_key.zeroize();
        self.chain_key = next_chain;
        self.counter += 1;

        MessageKey(message_key)
    }

    /// Get the current message counter.
    #[must_use]
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Skip to a specific counter, returning all skipped keys.
    ///
    /// Used when receiving out-of-order messages. The skipped keys
    /// should be stored to decrypt messages that arrive later.
    ///
    /// # Errors
    ///
    /// Returns error if target is too far ahead (`DoS` protection).
    pub fn skip_to(&mut self, target: u64) -> Result<Vec<(u64, MessageKey)>, RatchetError> {
        if target < self.counter {
            return Err(RatchetError::InvalidCounter);
        }

        let gap = target - self.counter;
        if gap > MAX_SKIP_GAP {
            return Err(RatchetError::TooManySkipped);
        }

        #[allow(clippy::cast_possible_truncation)]
        let mut skipped = Vec::with_capacity(gap as usize);

        while self.counter < target {
            let counter = self.counter;
            let key = self.next_key();
            skipped.push((counter, key));
        }

        Ok(skipped)
    }
}

/// Message key derived from the symmetric ratchet.
///
/// This key is used for a single message encryption/decryption.
/// It is zeroized on drop to prevent key material from lingering in memory.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MessageKey([u8; 32]);

impl MessageKey {
    /// Get the raw key bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to an AEAD key for encryption/decryption.
    #[must_use]
    pub fn to_aead_key(&self) -> AeadKey {
        AeadKey::new(self.0)
    }
}

/// Header sent with each encrypted message.
///
/// Contains the sender's current DH public key and message number
/// for the receiver to synchronize their ratchet state.
#[derive(Clone, Debug)]
pub struct MessageHeader {
    /// Sender's current DH public key
    pub dh_public: PublicKey,
    /// Previous chain length (number of messages sent on previous sending chain)
    pub prev_chain_length: u32,
    /// Message number in the current chain
    pub message_number: u32,
}

impl MessageHeader {
    /// Serialize to bytes (32 + 4 + 4 = 40 bytes).
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 40] {
        let mut bytes = [0u8; 40];
        bytes[..32].copy_from_slice(&self.dh_public.to_bytes());
        bytes[32..36].copy_from_slice(&self.prev_chain_length.to_le_bytes());
        bytes[36..40].copy_from_slice(&self.message_number.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RatchetError::InvalidHeader`] if the input is less than 40 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RatchetError> {
        if bytes.len() < 40 {
            return Err(RatchetError::InvalidHeader);
        }

        let mut dh_bytes = [0u8; 32];
        dh_bytes.copy_from_slice(&bytes[..32]);
        let dh_public = PublicKey::from_bytes(dh_bytes);

        let prev_chain_length = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        let message_number = u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);

        Ok(Self {
            dh_public,
            prev_chain_length,
            message_number,
        })
    }
}

/// Double Ratchet state for a session.
///
/// Combines DH ratcheting with symmetric ratcheting to provide
/// forward secrecy and post-compromise security.
///
/// # Example
///
/// ```ignore
/// use wraith_crypto::ratchet::DoubleRatchet;
/// use rand_core::OsRng;
///
/// // After handshake, both parties have shared_secret and know peer's DH key
/// let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
/// let mut bob = DoubleRatchet::new_responder(&mut OsRng, &shared_secret, bob_dh_private);
///
/// // Alice sends to Bob
/// let (header, ciphertext) = alice.encrypt(&mut OsRng, b"hello")?;
/// let plaintext = bob.decrypt(&header, &ciphertext)?;
/// ```
#[derive(ZeroizeOnDrop)]
pub struct DoubleRatchet {
    /// Our current DH key pair
    #[zeroize(skip)]
    dh_self: PrivateKey,
    /// Peer's current DH public key
    #[zeroize(skip)]
    dh_peer: Option<PublicKey>,
    /// Root key (32 bytes)
    root_key: [u8; 32],
    /// Sending chain key
    send_chain_key: Option<[u8; 32]>,
    /// Receiving chain key
    recv_chain_key: Option<[u8; 32]>,
    /// Number of messages sent in current sending chain
    send_count: u32,
    /// Number of messages received in current receiving chain
    recv_count: u32,
    /// Previous sending chain length (for header)
    prev_send_count: u32,
    /// Skipped message keys (`dh_public`, `n`) -> key
    #[zeroize(skip)]
    skipped_keys: BTreeMap<([u8; 32], u32), MessageKey>,
}

impl DoubleRatchet {
    /// Initialize as the initiator (Alice role).
    ///
    /// The initiator performs the first DH ratchet step using
    /// the responder's public key from the handshake.
    ///
    /// # Panics
    ///
    /// Panics if the peer's public key is an invalid low-order point.
    #[must_use]
    pub fn new_initiator<R: RngCore + CryptoRng>(
        rng: &mut R,
        shared_secret: &[u8; 32],
        peer_public: PublicKey,
    ) -> Self {
        // Generate our ephemeral DH key
        let dh_self = PrivateKey::generate(rng);

        // Perform initial DH
        let dh_out = dh_self
            .exchange(&peer_public)
            .expect("Invalid peer public key");

        // Derive root key and sending chain key
        let (root_key, send_chain_key) = kdf_rk(shared_secret, dh_out.as_bytes());

        Self {
            dh_self,
            dh_peer: Some(peer_public),
            root_key,
            send_chain_key: Some(send_chain_key),
            recv_chain_key: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_keys: BTreeMap::new(),
        }
    }

    /// Initialize as the responder (Bob role).
    ///
    /// The responder uses the DH key from the handshake and waits
    /// for the initiator's first message to complete the DH ratchet.
    #[must_use]
    pub fn new_responder(shared_secret: &[u8; 32], dh_keypair: PrivateKey) -> Self {
        Self {
            dh_self: dh_keypair,
            dh_peer: None,
            root_key: *shared_secret,
            send_chain_key: None,
            recv_chain_key: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_keys: BTreeMap::new(),
        }
    }

    /// Get our current DH public key (for including in message headers).
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.dh_self.public_key()
    }

    /// Encrypt a plaintext message.
    ///
    /// Returns a header (containing our DH public key and message number)
    /// and the ciphertext.
    ///
    /// # Errors
    ///
    /// Returns [`RatchetError::NoSendingChain`] if no sending chain is established.
    /// Returns [`RatchetError::EncryptionFailed`] if AEAD encryption fails.
    pub fn encrypt<R: RngCore + CryptoRng>(
        &mut self,
        _rng: &mut R,
        plaintext: &[u8],
    ) -> Result<(MessageHeader, Vec<u8>), RatchetError> {
        // Ensure we have a sending chain
        let send_chain = self
            .send_chain_key
            .as_mut()
            .ok_or(RatchetError::NoSendingChain)?;

        // Derive message key using symmetric ratchet step
        let (message_key, new_chain_key) = kdf_ck(send_chain);
        *send_chain = new_chain_key;

        let message_number = self.send_count;
        self.send_count += 1;

        // Create header
        let header = MessageHeader {
            dh_public: self.dh_self.public_key(),
            prev_chain_length: self.prev_send_count,
            message_number,
        };

        // Encrypt with AEAD using header as associated data
        let aead_key = AeadKey::new(message_key);
        let nonce = derive_nonce(message_number);
        let ciphertext = aead_key
            .encrypt(&nonce, plaintext, &header.to_bytes())
            .map_err(|_| RatchetError::EncryptionFailed)?;

        Ok((header, ciphertext))
    }

    /// Force a DH ratchet step (rotate sending key).
    pub fn force_dh_step<R: RngCore + CryptoRng>(&mut self, rng: &mut R) {
        self.dh_self = PrivateKey::generate(rng);
        self.prev_send_count = self.send_count;
        self.send_count = 0;
    }

    /// Try to retrieve a skipped message key using constant-time lookup.
    ///
    /// This prevents timing side-channel attacks by iterating through all
    /// skipped keys with constant-time comparison.
    fn try_skipped_keys(
        &mut self,
        dh_public: &[u8; 32],
        message_number: u32,
    ) -> Option<MessageKey> {
        // Prepare the key_id we're looking for
        let target_dh = dh_public;
        let target_num = message_number.to_le_bytes();

        let mut found_key: Option<MessageKey> = None;
        let mut remove_key_id: Option<([u8; 32], u32)> = None;

        // Iterate through all skipped keys with constant-time comparison
        for (stored_dh, stored_num) in self.skipped_keys.keys() {
            let stored_num_bytes = stored_num.to_le_bytes();

            // Constant-time comparison of both DH public key and message number
            let dh_match = ct_eq(target_dh, stored_dh);
            let num_match = ct_eq(&target_num, &stored_num_bytes);

            if dh_match && num_match {
                remove_key_id = Some((*stored_dh, *stored_num));
                // Don't break - continue iterating to maintain constant-time
            }
        }

        // Remove the key if found (this happens outside the loop)
        if let Some(key_id) = remove_key_id {
            found_key = self.skipped_keys.remove(&key_id);
        }

        found_key
    }

    /// Decrypt a ciphertext message.
    ///
    /// Performs DH ratchet if the sender's DH public key has changed,
    /// then decrypts using the appropriate message key.
    ///
    /// # Errors
    ///
    /// Returns [`RatchetError::NoReceivingChain`] if no receiving chain is established.
    /// Returns [`RatchetError::DecryptionFailed`] if AEAD decryption fails.
    /// Returns [`RatchetError::TooManySkipped`] if too many message keys need to be skipped.
    pub fn decrypt<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        header: &MessageHeader,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, RatchetError> {
        // Check if we have a skipped key for this message (constant-time lookup)
        if let Some(message_key) =
            self.try_skipped_keys(&header.dh_public.to_bytes(), header.message_number)
        {
            let aead_key = message_key.to_aead_key();
            let nonce = derive_nonce(header.message_number);
            return aead_key
                .decrypt(&nonce, ciphertext, &header.to_bytes())
                .map_err(|_| RatchetError::DecryptionFailed);
        }

        // Check if we need to perform a DH ratchet step
        let need_dh_ratchet = self.dh_peer.as_ref() != Some(&header.dh_public);

        if need_dh_ratchet {
            // Skip any remaining messages in current receiving chain
            if self.recv_chain_key.is_some() {
                self.skip_message_keys(header.prev_chain_length)?;
            }

            // Perform DH ratchet
            self.dh_ratchet(rng, &header.dh_public)?;
        }

        // Skip ahead if needed (out-of-order message)
        self.skip_message_keys(header.message_number)?;

        // Derive message key
        let recv_chain = self
            .recv_chain_key
            .as_mut()
            .ok_or(RatchetError::NoReceivingChain)?;

        let (message_key, new_chain_key) = kdf_ck(recv_chain);
        *recv_chain = new_chain_key;
        self.recv_count += 1;

        // Decrypt
        let aead_key = AeadKey::new(message_key);
        let nonce = derive_nonce(header.message_number);
        aead_key
            .decrypt(&nonce, ciphertext, &header.to_bytes())
            .map_err(|_| RatchetError::DecryptionFailed)
    }

    /// Perform a DH ratchet step.
    ///
    /// Called when we receive a message with a new DH public key.
    fn dh_ratchet<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        peer_public: &PublicKey,
    ) -> Result<(), RatchetError> {
        // Store new peer public key
        self.dh_peer = Some(*peer_public);
        self.prev_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;

        // DH with our current key and their new public key
        let dh_recv = self
            .dh_self
            .exchange(peer_public)
            .ok_or(RatchetError::InvalidPublicKey)?;

        // Derive new root key and receiving chain key
        let (new_root, recv_chain_key) = kdf_rk(&self.root_key, dh_recv.as_bytes());
        self.root_key = new_root;
        self.recv_chain_key = Some(recv_chain_key);

        // Generate new DH keypair
        self.dh_self = PrivateKey::generate(rng);

        // DH with our new key and their public key
        let dh_send = self
            .dh_self
            .exchange(peer_public)
            .ok_or(RatchetError::InvalidPublicKey)?;

        // Derive new root key and sending chain key
        let (new_root, send_chain_key) = kdf_rk(&self.root_key, dh_send.as_bytes());
        self.root_key = new_root;
        self.send_chain_key = Some(send_chain_key);

        Ok(())
    }

    /// Skip message keys up to `until` and store them.
    fn skip_message_keys(&mut self, until: u32) -> Result<(), RatchetError> {
        if self.recv_chain_key.is_none() {
            return Ok(());
        }

        let skip_count = until.saturating_sub(self.recv_count);
        if u64::from(skip_count) > MAX_SKIP_GAP {
            return Err(RatchetError::TooManySkipped);
        }

        if self.skipped_keys.len() as u64 + u64::from(skip_count) > MAX_SKIP {
            return Err(RatchetError::TooManySkipped);
        }

        let dh_peer_bytes = self
            .dh_peer
            .as_ref()
            .ok_or(RatchetError::NoPeerKey)?
            .to_bytes();

        let recv_chain = self.recv_chain_key.as_mut().unwrap();

        while self.recv_count < until {
            let (message_key, new_chain_key) = kdf_ck(recv_chain);
            *recv_chain = new_chain_key;

            let key_id = (dh_peer_bytes, self.recv_count);
            self.skipped_keys.insert(key_id, MessageKey(message_key));

            self.recv_count += 1;
        }

        Ok(())
    }

    /// Number of skipped keys currently stored.
    #[must_use]
    pub fn skipped_key_count(&self) -> usize {
        self.skipped_keys.len()
    }
}

/// Root key KDF: Derive new root key and chain key from DH output.
fn kdf_rk(root_key: &[u8; 32], dh_out: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let temp = hkdf_extract(root_key, dh_out);

    let mut new_root = [0u8; 32];
    let mut chain_key = [0u8; 32];

    hkdf_expand(&temp, b"wraith_root", &mut new_root);
    hkdf_expand(&temp, b"wraith_chain", &mut chain_key);

    (new_root, chain_key)
}

/// Chain key KDF: Derive message key and next chain key.
fn kdf_ck(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut message_key = [0u8; 32];
    let mut next_chain = [0u8; 32];

    hkdf_expand(chain_key, b"wraith_message_key", &mut message_key);
    hkdf_expand(chain_key, b"wraith_chain_key", &mut next_chain);

    (message_key, next_chain)
}

/// Derive a nonce from message number.
///
/// Uses a deterministic nonce since each message key is only used once.
fn derive_nonce(message_number: u32) -> Nonce {
    let mut nonce_bytes = [0u8; 24];
    nonce_bytes[..4].copy_from_slice(&message_number.to_le_bytes());
    // Remaining bytes are zero (deterministic nonce is safe with unique keys)
    Nonce::from_bytes(nonce_bytes)
}

/// Errors that can occur during ratcheting operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatchetError {
    /// Message counter is invalid (e.g., trying to go backwards)
    InvalidCounter,
    /// Too many messages skipped (`DoS` protection)
    TooManySkipped,
    /// Invalid message header format
    InvalidHeader,
    /// No sending chain established
    NoSendingChain,
    /// No receiving chain established
    NoReceivingChain,
    /// No peer DH public key available
    NoPeerKey,
    /// Invalid peer public key (low-order point)
    InvalidPublicKey,
    /// AEAD encryption failed
    EncryptionFailed,
    /// AEAD decryption failed (authentication failure)
    DecryptionFailed,
}

impl fmt::Display for RatchetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RatchetError::InvalidCounter => write!(f, "invalid message counter"),
            RatchetError::TooManySkipped => write!(f, "too many skipped messages"),
            RatchetError::InvalidHeader => write!(f, "invalid message header"),
            RatchetError::NoSendingChain => write!(f, "no sending chain established"),
            RatchetError::NoReceivingChain => write!(f, "no receiving chain established"),
            RatchetError::NoPeerKey => write!(f, "no peer DH public key"),
            RatchetError::InvalidPublicKey => write!(f, "invalid public key"),
            RatchetError::EncryptionFailed => write!(f, "encryption failed"),
            RatchetError::DecryptionFailed => write!(f, "decryption failed"),
        }
    }
}

impl core::error::Error for RatchetError {}

impl From<RatchetError> for CryptoError {
    fn from(err: RatchetError) -> Self {
        match err {
            RatchetError::DecryptionFailed => CryptoError::DecryptionFailed,
            RatchetError::EncryptionFailed => CryptoError::EncryptionFailed,
            _ => CryptoError::InvalidState,
        }
    }
}

// ============================================================================
// Legacy ChainKey API (for backwards compatibility)
// ============================================================================

/// Chain key for symmetric ratcheting (legacy API).
///
/// Prefer using `SymmetricRatchet` for new code.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct ChainKey([u8; 32]);

impl ChainKey {
    /// Create from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Ratchet forward and derive message key.
    pub fn ratchet(&mut self) -> LegacyMessageKey {
        let old_key = self.0;

        // chain_key[n+1] = BLAKE3(chain_key[n] || 0x01)
        let mut hasher = blake3::Hasher::new();
        hasher.update(&old_key);
        hasher.update(&[0x01]);
        self.0.copy_from_slice(&hasher.finalize().as_bytes()[..32]);

        // message_key[n] = BLAKE3(chain_key[n] || 0x02)
        let mut hasher = blake3::Hasher::new();
        hasher.update(&old_key);
        hasher.update(&[0x02]);
        let mut msg_key = [0u8; 32];
        msg_key.copy_from_slice(&hasher.finalize().as_bytes()[..32]);

        LegacyMessageKey(msg_key)
    }
}

/// Message key derived from chain key (legacy API).
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct LegacyMessageKey([u8; 32]);

impl LegacyMessageKey {
    /// Get the raw key bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    // ========================================================================
    // SymmetricRatchet Tests
    // ========================================================================

    #[test]
    fn test_symmetric_ratchet_deterministic() {
        let root = [0x42u8; 32];
        let mut ratchet1 = SymmetricRatchet::new(&root);
        let mut ratchet2 = SymmetricRatchet::new(&root);

        let key1a = ratchet1.next_key();
        let key1b = ratchet2.next_key();
        assert_eq!(key1a.as_bytes(), key1b.as_bytes());

        let key2a = ratchet1.next_key();
        let key2b = ratchet2.next_key();
        assert_eq!(key2a.as_bytes(), key2b.as_bytes());
    }

    #[test]
    fn test_symmetric_ratchet_unique_keys() {
        let root = [0x42u8; 32];
        let mut ratchet = SymmetricRatchet::new(&root);

        let key1 = ratchet.next_key();
        let key2 = ratchet.next_key();
        let key3 = ratchet.next_key();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
        assert_ne!(key2.as_bytes(), key3.as_bytes());
        assert_ne!(key1.as_bytes(), key3.as_bytes());
    }

    #[test]
    fn test_symmetric_ratchet_counter() {
        let root = [0x42u8; 32];
        let mut ratchet = SymmetricRatchet::new(&root);

        assert_eq!(ratchet.counter(), 0);
        let _ = ratchet.next_key();
        assert_eq!(ratchet.counter(), 1);
        let _ = ratchet.next_key();
        assert_eq!(ratchet.counter(), 2);
    }

    #[test]
    fn test_symmetric_ratchet_skip() {
        let root = [0x42u8; 32];
        let mut ratchet1 = SymmetricRatchet::new(&root);
        let mut ratchet2 = SymmetricRatchet::new(&root);

        // Ratchet1 advances normally
        let _key0 = ratchet1.next_key();
        let _key1 = ratchet1.next_key();
        let _key2 = ratchet1.next_key();
        let key3 = ratchet1.next_key();

        // Ratchet2 skips to position 3
        let skipped = ratchet2.skip_to(3).unwrap();
        assert_eq!(skipped.len(), 3);

        // Verify skipped keys match
        assert_eq!(skipped[0].0, 0);
        assert_eq!(skipped[1].0, 1);
        assert_eq!(skipped[2].0, 2);

        // Both should produce same key at position 3
        let key3b = ratchet2.next_key();
        assert_eq!(key3.as_bytes(), key3b.as_bytes());
    }

    #[test]
    fn test_symmetric_ratchet_skip_too_far() {
        let root = [0x42u8; 32];
        let mut ratchet = SymmetricRatchet::new(&root);

        // Trying to skip too far should fail
        let result = ratchet.skip_to(MAX_SKIP_GAP + 10);
        assert!(matches!(result, Err(RatchetError::TooManySkipped)));
    }

    // ========================================================================
    // MessageHeader Tests
    // ========================================================================

    #[test]
    fn test_message_header_serialization() {
        let dh_public = PublicKey::from_bytes([0x42u8; 32]);
        let header = MessageHeader {
            dh_public,
            prev_chain_length: 5,
            message_number: 10,
        };

        let bytes = header.to_bytes();
        let recovered = MessageHeader::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.dh_public.to_bytes(), header.dh_public.to_bytes());
        assert_eq!(recovered.prev_chain_length, header.prev_chain_length);
        assert_eq!(recovered.message_number, header.message_number);
    }

    #[test]
    fn test_message_header_invalid() {
        let result = MessageHeader::from_bytes(&[0u8; 10]);
        assert!(matches!(result, Err(RatchetError::InvalidHeader)));
    }

    // ========================================================================
    // DoubleRatchet Tests
    // ========================================================================

    #[test]
    fn test_double_ratchet_basic() {
        let shared_secret = [0x42u8; 32];

        // Bob generates a DH keypair
        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        // Alice initializes as initiator with Bob's public key
        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);

        // Bob initializes as responder with his private key
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

        // Alice sends message to Bob
        let (header, ciphertext) = alice.encrypt(&mut OsRng, b"hello bob").unwrap();

        // Bob decrypts
        let plaintext = bob.decrypt(&mut OsRng, &header, &ciphertext).unwrap();
        assert_eq!(plaintext, b"hello bob");
    }

    #[test]
    fn test_double_ratchet_bidirectional() {
        let shared_secret = [0x42u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

        // Alice -> Bob
        let (h1, c1) = alice.encrypt(&mut OsRng, b"alice to bob 1").unwrap();
        assert_eq!(
            bob.decrypt(&mut OsRng, &h1, &c1).unwrap(),
            b"alice to bob 1"
        );

        // Bob -> Alice
        let (h2, c2) = bob.encrypt(&mut OsRng, b"bob to alice 1").unwrap();
        assert_eq!(
            alice.decrypt(&mut OsRng, &h2, &c2).unwrap(),
            b"bob to alice 1"
        );

        // Alice -> Bob again
        let (h3, c3) = alice.encrypt(&mut OsRng, b"alice to bob 2").unwrap();
        assert_eq!(
            bob.decrypt(&mut OsRng, &h3, &c3).unwrap(),
            b"alice to bob 2"
        );

        // Bob -> Alice again
        let (h4, c4) = bob.encrypt(&mut OsRng, b"bob to alice 2").unwrap();
        assert_eq!(
            alice.decrypt(&mut OsRng, &h4, &c4).unwrap(),
            b"bob to alice 2"
        );
    }

    #[test]
    fn test_double_ratchet_multiple_messages_same_chain() {
        let shared_secret = [0x42u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

        // Alice sends multiple messages without Bob responding
        let (h1, c1) = alice.encrypt(&mut OsRng, b"message 1").unwrap();
        let (h2, c2) = alice.encrypt(&mut OsRng, b"message 2").unwrap();
        let (h3, c3) = alice.encrypt(&mut OsRng, b"message 3").unwrap();

        // Bob decrypts all
        assert_eq!(bob.decrypt(&mut OsRng, &h1, &c1).unwrap(), b"message 1");
        assert_eq!(bob.decrypt(&mut OsRng, &h2, &c2).unwrap(), b"message 2");
        assert_eq!(bob.decrypt(&mut OsRng, &h3, &c3).unwrap(), b"message 3");
    }

    #[test]
    fn test_double_ratchet_out_of_order() {
        let shared_secret = [0x42u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

        // Alice sends multiple messages
        let (h1, c1) = alice.encrypt(&mut OsRng, b"message 1").unwrap();
        let (h2, c2) = alice.encrypt(&mut OsRng, b"message 2").unwrap();
        let (h3, c3) = alice.encrypt(&mut OsRng, b"message 3").unwrap();

        // Bob receives out of order
        assert_eq!(bob.decrypt(&mut OsRng, &h3, &c3).unwrap(), b"message 3");
        assert_eq!(bob.decrypt(&mut OsRng, &h1, &c1).unwrap(), b"message 1");
        assert_eq!(bob.decrypt(&mut OsRng, &h2, &c2).unwrap(), b"message 2");
    }

    #[test]
    fn test_double_ratchet_wrong_key() {
        let shared_secret1 = [0x42u8; 32];
        let shared_secret2 = [0x43u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret1, bob_dh_public);

        // Eve has wrong shared secret
        let eve_dh = PrivateKey::generate(&mut OsRng);
        let mut eve = DoubleRatchet::new_responder(&shared_secret2, eve_dh);

        let (header, ciphertext) = alice.encrypt(&mut OsRng, b"secret").unwrap();

        // Eve cannot decrypt
        let result = eve.decrypt(&mut OsRng, &header, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_double_ratchet_tampering_detected() {
        let shared_secret = [0x42u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

        let (header, mut ciphertext) = alice.encrypt(&mut OsRng, b"secret message").unwrap();

        // Tamper with ciphertext
        if !ciphertext.is_empty() {
            ciphertext[0] ^= 0xFF;
        }

        // Bob detects tampering
        let result = bob.decrypt(&mut OsRng, &header, &ciphertext);
        assert!(matches!(result, Err(RatchetError::DecryptionFailed)));
    }

    #[test]
    fn test_double_ratchet_unique_ciphertexts() {
        let shared_secret = [0x42u8; 32];

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_dh_public = bob_dh.public_key();

        let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);

        // Same plaintext produces different ciphertexts
        let (_, c1) = alice.encrypt(&mut OsRng, b"same message").unwrap();
        let (_, c2) = alice.encrypt(&mut OsRng, b"same message").unwrap();

        assert_ne!(c1, c2);
    }

    // ========================================================================
    // Legacy API Tests
    // ========================================================================

    #[test]
    fn test_legacy_chain_key_ratchet() {
        let mut chain = ChainKey::from_bytes([0x42u8; 32]);

        let key1 = chain.ratchet();
        let key2 = chain.ratchet();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_legacy_chain_key_deterministic() {
        let mut chain1 = ChainKey::from_bytes([0x42u8; 32]);
        let mut chain2 = ChainKey::from_bytes([0x42u8; 32]);

        let key1 = chain1.ratchet();
        let key2 = chain2.ratchet();

        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }
}
