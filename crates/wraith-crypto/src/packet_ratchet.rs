//! Per-packet forward secrecy ratchet.
//!
//! Provides a lightweight BLAKE3-based symmetric ratchet for per-packet key
//! derivation. This is separate from the Signal Double Ratchet in [`crate::ratchet`]
//! and is designed for the v2 protocol's packet-level forward secrecy.
//!
//! Each packet gets a unique encryption key derived from a chain key. After
//! deriving a message key, the chain key is ratcheted forward and the old value
//! is zeroized, providing forward secrecy at the packet level.
//!
//! An out-of-order window with a BTreeMap cache allows decryption of packets
//! that arrive out of sequence, up to a configurable window size.

use alloc::collections::BTreeMap;
use zeroize::Zeroize;

use crate::error::CryptoError;

/// Default out-of-order window size (number of cached keys).
const DEFAULT_WINDOW_SIZE: usize = 1024;

/// A per-packet ratchet providing forward secrecy for each packet.
///
/// Maintains a BLAKE3-based chain key that is ratcheted after each key derivation.
/// Supports out-of-order packet delivery via a bounded key cache.
pub struct PacketRatchet {
    /// Current chain key (ratcheted forward on each derivation).
    chain_key: [u8; 32],
    /// Next packet number to be sent or next expected for sequential receive.
    packet_number: u64,
    /// Cached keys for out-of-order packet delivery.
    /// Maps packet_number -> message_key.
    key_cache: BTreeMap<u64, [u8; 32]>,
    /// Maximum number of keys to cache for out-of-order delivery.
    max_cache_size: usize,
}

/// Derive a message key from the current chain key.
fn derive_message_key(chain_key: &[u8; 32]) -> [u8; 32] {
    *blake3::keyed_hash(chain_key, b"wraith-v2-ratchet-message").as_bytes()
}

/// Derive the next chain key from the current chain key.
fn derive_next_chain_key(chain_key: &[u8; 32]) -> [u8; 32] {
    *blake3::keyed_hash(chain_key, b"wraith-v2-ratchet-chain").as_bytes()
}

impl PacketRatchet {
    /// Create a new packet ratchet from an initial chain key.
    #[must_use]
    pub fn new(chain_key: [u8; 32]) -> Self {
        Self {
            chain_key,
            packet_number: 0,
            key_cache: BTreeMap::new(),
            max_cache_size: DEFAULT_WINDOW_SIZE,
        }
    }

    /// Create a new packet ratchet with a custom window size.
    #[must_use]
    pub fn with_window_size(chain_key: [u8; 32], window_size: usize) -> Self {
        Self {
            chain_key,
            packet_number: 0,
            key_cache: BTreeMap::new(),
            max_cache_size: window_size,
        }
    }

    /// Get the next send key and advance the ratchet.
    ///
    /// Returns `(packet_number, message_key)` for encrypting the next packet.
    pub fn next_send_key(&mut self) -> (u64, [u8; 32]) {
        let pn = self.packet_number;
        let message_key = derive_message_key(&self.chain_key);
        let next_chain = derive_next_chain_key(&self.chain_key);
        self.chain_key.zeroize();
        self.chain_key = next_chain;
        self.packet_number += 1;
        (pn, message_key)
    }

    /// Get the decryption key for a specific packet number.
    ///
    /// If the packet number is ahead of the current position, the ratchet
    /// advances and intermediate keys are cached for out-of-order delivery.
    /// If the packet number matches a cached key, it is removed and returned.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidState`] if:
    /// - The packet number is behind the current position and not in the cache
    ///   (the key was already used or evicted -- forward secrecy).
    /// - The packet is too far ahead and would exceed the cache size.
    pub fn key_for_packet(&mut self, pn: u64) -> Result<[u8; 32], CryptoError> {
        // Check cache first (for out-of-order packets)
        if let Some(key) = self.key_cache.remove(&pn) {
            return Ok(key);
        }

        // Packet is behind current position -- key was already consumed
        if pn < self.packet_number {
            return Err(CryptoError::InvalidState);
        }

        // Packet is ahead -- advance and cache intermediate keys
        let gap = pn - self.packet_number;
        if gap as usize > self.max_cache_size {
            return Err(CryptoError::InvalidState);
        }

        // Advance ratchet, caching intermediate keys
        while self.packet_number < pn {
            let intermediate_pn = self.packet_number;
            let message_key = derive_message_key(&self.chain_key);
            let next_chain = derive_next_chain_key(&self.chain_key);
            self.chain_key.zeroize();
            self.chain_key = next_chain;
            self.packet_number += 1;

            self.key_cache.insert(intermediate_pn, message_key);

            // Evict oldest if over capacity
            while self.key_cache.len() > self.max_cache_size {
                if let Some((&oldest_pn, _)) = self.key_cache.iter().next() {
                    let mut removed = self.key_cache.remove(&oldest_pn).unwrap_or([0u8; 32]);
                    removed.zeroize();
                }
            }
        }

        // Now packet_number == pn, derive the requested key
        let message_key = derive_message_key(&self.chain_key);
        let next_chain = derive_next_chain_key(&self.chain_key);
        self.chain_key.zeroize();
        self.chain_key = next_chain;
        self.packet_number += 1;

        Ok(message_key)
    }

    /// Get the current packet number (next to be sent/expected).
    #[must_use]
    pub fn packet_number(&self) -> u64 {
        self.packet_number
    }

    /// Get the number of cached out-of-order keys.
    #[must_use]
    pub fn cached_key_count(&self) -> usize {
        self.key_cache.len()
    }
}

impl Drop for PacketRatchet {
    fn drop(&mut self) {
        // Zeroize all cached keys
        for (_, key) in self.key_cache.iter_mut() {
            key.zeroize();
        }
        self.key_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_ratchet_deterministic() {
        let chain = [0x42u8; 32];
        let mut r1 = PacketRatchet::new(chain);
        let mut r2 = PacketRatchet::new(chain);

        let (pn1, k1) = r1.next_send_key();
        let (pn2, k2) = r2.next_send_key();

        assert_eq!(pn1, 0);
        assert_eq!(pn2, 0);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_packet_ratchet_unique_keys() {
        let mut r = PacketRatchet::new([0x42u8; 32]);

        let (_, k1) = r.next_send_key();
        let (_, k2) = r.next_send_key();
        let (_, k3) = r.next_send_key();

        assert_ne!(k1, k2);
        assert_ne!(k2, k3);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_packet_ratchet_forward_secrecy() {
        let chain = [0x42u8; 32];
        let mut r = PacketRatchet::new(chain);

        // Advance past packet 0
        let (_, _k0) = r.next_send_key();
        let (_, _k1) = r.next_send_key();

        // Cannot retrieve key for packet 0 anymore (forward secrecy)
        let mut recv = PacketRatchet::new(chain);
        let _ = recv.key_for_packet(0).unwrap();
        let _ = recv.key_for_packet(1).unwrap();

        // Key for packet 0 is gone from recv after consumption
        let result = recv.key_for_packet(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_packet_ratchet_out_of_order() {
        let chain = [0x42u8; 32];

        // Sender generates keys sequentially
        let mut sender = PacketRatchet::new(chain);
        let (_, k0) = sender.next_send_key();
        let (_, k1) = sender.next_send_key();
        let (_, k2) = sender.next_send_key();

        // Receiver gets packet 2 first, then 0, then 1
        let mut recv = PacketRatchet::new(chain);
        let rk2 = recv.key_for_packet(2).unwrap();
        assert_eq!(k2, rk2);

        let rk0 = recv.key_for_packet(0).unwrap();
        assert_eq!(k0, rk0);

        let rk1 = recv.key_for_packet(1).unwrap();
        assert_eq!(k1, rk1);
    }

    #[test]
    fn test_packet_ratchet_window_overflow() {
        let mut r = PacketRatchet::with_window_size([0x42u8; 32], 10);

        // Jump too far ahead
        let result = r.key_for_packet(100);
        assert!(result.is_err());
    }

    #[test]
    fn test_packet_ratchet_sequential_send_recv() {
        let chain = [0x42u8; 32];
        let mut sender = PacketRatchet::new(chain);
        let mut recv = PacketRatchet::new(chain);

        for i in 0u64..100 {
            let (pn, sk) = sender.next_send_key();
            assert_eq!(pn, i);
            let rk = recv.key_for_packet(pn).unwrap();
            assert_eq!(sk, rk);
        }
    }

    #[test]
    fn test_packet_ratchet_packet_number_advances() {
        let mut r = PacketRatchet::new([0x42u8; 32]);
        assert_eq!(r.packet_number(), 0);
        let _ = r.next_send_key();
        assert_eq!(r.packet_number(), 1);
        let _ = r.next_send_key();
        assert_eq!(r.packet_number(), 2);
    }

    #[test]
    fn test_packet_ratchet_cached_key_count() {
        let chain = [0x42u8; 32];
        let mut r = PacketRatchet::new(chain);

        // Skip ahead to packet 5 -- packets 0..4 get cached
        let _ = r.key_for_packet(5).unwrap();
        assert_eq!(r.cached_key_count(), 5);

        // Consume a cached key
        let _ = r.key_for_packet(2).unwrap();
        assert_eq!(r.cached_key_count(), 4);
    }

    #[test]
    fn test_derive_functions_distinct() {
        let chain = [0x42u8; 32];
        let msg_key = derive_message_key(&chain);
        let next_chain = derive_next_chain_key(&chain);

        // Message key and next chain key must differ
        assert_ne!(msg_key, next_chain);
        // Neither should be the input
        assert_ne!(msg_key, chain);
        assert_ne!(next_chain, chain);
    }
}
