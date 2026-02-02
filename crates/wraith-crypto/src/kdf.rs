//! v2 Key Derivation Functions with domain-separated labels.
//!
//! Provides labeled KDF operations for the WRAITH v2 protocol, building on
//! the HKDF-BLAKE3 primitives in [`crate::hash`]. Each derivation uses a
//! unique label for cryptographic domain separation, preventing cross-protocol
//! key confusion attacks.

use crate::hash::{hkdf_expand, hkdf_extract};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// v2 KDF labels for domain separation.
///
/// Each label uniquely identifies the purpose of a derived key, ensuring that
/// keys derived for different purposes are cryptographically independent.
pub mod labels {
    /// Label for deriving the handshake secret from hybrid KEM output.
    pub const HANDSHAKE_SECRET: &[u8] = b"wraith-v2-handshake-secret";
    /// Label for deriving the master traffic secret.
    pub const TRAFFIC_SECRET: &[u8] = b"wraith-v2-traffic-secret";
    /// Label for the initiator-to-responder traffic key.
    pub const TRAFFIC_KEY_I2R: &[u8] = b"wraith-v2-traffic-key-i2r";
    /// Label for the responder-to-initiator traffic key.
    pub const TRAFFIC_KEY_R2I: &[u8] = b"wraith-v2-traffic-key-r2i";
    /// Label for the per-packet ratchet chain key derivation.
    pub const RATCHET_CHAIN: &[u8] = b"wraith-v2-ratchet-chain";
    /// Label for the per-packet ratchet message key derivation.
    pub const RATCHET_MESSAGE: &[u8] = b"wraith-v2-ratchet-message";
    /// Label for the frame format obfuscation key.
    pub const FORMAT_KEY: &[u8] = b"wraith-v2-format-key";
    /// Label for per-stream subkey derivation.
    pub const STREAM_KEY: &[u8] = b"wraith-v2-stream-key";
    /// Label for hybrid KEM secret combination.
    pub const HYBRID_COMBINE: &[u8] = b"wraith-v2-hybrid-combine";
    /// Label for group secret derivation (Phase 8).
    pub const GROUP_SECRET: &[u8] = b"wraith-v2-group-secret";
    /// Label for group application key derivation (Phase 8).
    pub const GROUP_APPLICATION: &[u8] = b"wraith-v2-group-application";
}

/// v2 session keys with directional traffic keys.
///
/// Derived from the combined hybrid KEM shared secret and a transcript hash
/// of the handshake. Provides separate keys for each traffic direction and
/// additional keys for format obfuscation and ratchet initialization.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SessionKeysV2 {
    /// Key for initiator-to-responder traffic encryption.
    pub initiator_to_responder: [u8; 32],
    /// Key for responder-to-initiator traffic encryption.
    pub responder_to_initiator: [u8; 32],
    /// Key for frame format obfuscation.
    pub format_key: [u8; 32],
    /// Initial chain key for per-packet ratchet.
    pub initial_chain_key: [u8; 32],
}

/// Derive v2 session keys from a combined shared secret and transcript hash.
///
/// Uses a two-phase HKDF construction:
/// 1. Extract: Combines the shared secret and transcript hash into a PRK.
/// 2. Expand: Derives four independent 32-byte keys from the PRK using
///    distinct labels for domain separation.
///
/// # Arguments
///
/// * `combined_secret` - The output of hybrid KEM (X25519 + ML-KEM-768 combined).
/// * `transcript_hash` - BLAKE3 hash of the handshake transcript for binding.
#[must_use]
pub fn derive_session_keys_v2(
    combined_secret: &[u8; 32],
    transcript_hash: &[u8; 32],
) -> SessionKeysV2 {
    // Extract phase: combine shared secret with transcript hash
    let mut prk = hkdf_extract(transcript_hash, combined_secret);

    // Expand phase: derive directional keys with domain separation
    let mut i2r = [0u8; 32];
    let mut r2i = [0u8; 32];
    let mut format_key = [0u8; 32];
    let mut chain_key = [0u8; 32];

    hkdf_expand(&prk, labels::TRAFFIC_KEY_I2R, &mut i2r);
    hkdf_expand(&prk, labels::TRAFFIC_KEY_R2I, &mut r2i);
    hkdf_expand(&prk, labels::FORMAT_KEY, &mut format_key);
    hkdf_expand(&prk, labels::RATCHET_CHAIN, &mut chain_key);

    prk.zeroize();

    SessionKeysV2 {
        initiator_to_responder: i2r,
        responder_to_initiator: r2i,
        format_key,
        initial_chain_key: chain_key,
    }
}

/// Derive a per-stream subkey from a base traffic key and stream identifier.
///
/// Used to derive independent encryption keys for multiplexed streams within
/// a single session.
#[must_use]
pub fn derive_stream_key(traffic_key: &[u8; 32], stream_id: u32) -> [u8; 32] {
    let label = labels::STREAM_KEY;
    let id_bytes = stream_id.to_le_bytes();
    let mut info = alloc::vec::Vec::with_capacity(label.len() + id_bytes.len());
    info.extend_from_slice(label);
    info.extend_from_slice(&id_bytes);
    let mut out = [0u8; 32];
    hkdf_expand(traffic_key, &info, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_session_keys_deterministic() {
        let secret = [0x42u8; 32];
        let transcript = [0x43u8; 32];

        let keys1 = derive_session_keys_v2(&secret, &transcript);
        let keys2 = derive_session_keys_v2(&secret, &transcript);

        assert_eq!(keys1.initiator_to_responder, keys2.initiator_to_responder);
        assert_eq!(keys1.responder_to_initiator, keys2.responder_to_initiator);
        assert_eq!(keys1.format_key, keys2.format_key);
        assert_eq!(keys1.initial_chain_key, keys2.initial_chain_key);
    }

    #[test]
    fn test_derive_session_keys_all_different() {
        let secret = [0x42u8; 32];
        let transcript = [0x43u8; 32];

        let keys = derive_session_keys_v2(&secret, &transcript);

        // All four derived keys should be distinct
        assert_ne!(keys.initiator_to_responder, keys.responder_to_initiator);
        assert_ne!(keys.initiator_to_responder, keys.format_key);
        assert_ne!(keys.initiator_to_responder, keys.initial_chain_key);
        assert_ne!(keys.responder_to_initiator, keys.format_key);
        assert_ne!(keys.responder_to_initiator, keys.initial_chain_key);
        assert_ne!(keys.format_key, keys.initial_chain_key);
    }

    #[test]
    fn test_derive_session_keys_different_secret() {
        let transcript = [0x43u8; 32];

        let keys1 = derive_session_keys_v2(&[0x01u8; 32], &transcript);
        let keys2 = derive_session_keys_v2(&[0x02u8; 32], &transcript);

        assert_ne!(keys1.initiator_to_responder, keys2.initiator_to_responder);
    }

    #[test]
    fn test_derive_session_keys_different_transcript() {
        let secret = [0x42u8; 32];

        let keys1 = derive_session_keys_v2(&secret, &[0x01u8; 32]);
        let keys2 = derive_session_keys_v2(&secret, &[0x02u8; 32]);

        assert_ne!(keys1.initiator_to_responder, keys2.initiator_to_responder);
    }

    #[test]
    fn test_derive_session_keys_nonzero() {
        let secret = [0x42u8; 32];
        let transcript = [0x43u8; 32];

        let keys = derive_session_keys_v2(&secret, &transcript);

        assert_ne!(keys.initiator_to_responder, [0u8; 32]);
        assert_ne!(keys.responder_to_initiator, [0u8; 32]);
        assert_ne!(keys.format_key, [0u8; 32]);
        assert_ne!(keys.initial_chain_key, [0u8; 32]);
    }

    #[test]
    fn test_derive_stream_key_deterministic() {
        let traffic_key = [0x42u8; 32];
        let k1 = derive_stream_key(&traffic_key, 0);
        let k2 = derive_stream_key(&traffic_key, 0);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_derive_stream_key_different_ids() {
        let traffic_key = [0x42u8; 32];
        let k1 = derive_stream_key(&traffic_key, 0);
        let k2 = derive_stream_key(&traffic_key, 1);
        let k3 = derive_stream_key(&traffic_key, 2);
        assert_ne!(k1, k2);
        assert_ne!(k2, k3);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_derive_stream_key_different_traffic_keys() {
        let k1 = derive_stream_key(&[0x01u8; 32], 0);
        let k2 = derive_stream_key(&[0x02u8; 32], 0);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_labels_unique() {
        // Verify all labels are distinct
        let all_labels: &[&[u8]] = &[
            labels::HANDSHAKE_SECRET,
            labels::TRAFFIC_SECRET,
            labels::TRAFFIC_KEY_I2R,
            labels::TRAFFIC_KEY_R2I,
            labels::RATCHET_CHAIN,
            labels::RATCHET_MESSAGE,
            labels::FORMAT_KEY,
            labels::STREAM_KEY,
            labels::HYBRID_COMBINE,
            labels::GROUP_SECRET,
            labels::GROUP_APPLICATION,
        ];
        for i in 0..all_labels.len() {
            for j in (i + 1)..all_labels.len() {
                assert_ne!(
                    all_labels[i], all_labels[j],
                    "Labels at {i} and {j} collide"
                );
            }
        }
    }
}
