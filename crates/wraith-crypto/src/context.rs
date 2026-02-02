//! CryptoContext v2 -- unified facade for WRAITH v2 cryptographic operations.
//!
//! Provides a single entry point that ties together the crypto suite, hybrid KEM,
//! v2 KDF, and per-packet ratchet. This allows callers to work with a consistent
//! API regardless of the negotiated cipher suite.

use crate::CryptoError;
use crate::hybrid::{HybridCiphertext, HybridKeyPair, HybridSharedSecret};
use crate::kdf::{self, SessionKeysV2};
use crate::packet_ratchet::PacketRatchet;
use crate::suite::CryptoSuite;
use rand_core::{CryptoRng, RngCore};

/// Unified cryptographic context for the WRAITH v2 protocol.
///
/// Encapsulates the negotiated cipher suite and provides high-level operations
/// for key generation, session key derivation, and ratchet creation.
pub struct CryptoContextV2 {
    /// The negotiated (or configured) cipher suite.
    suite: CryptoSuite,
}

impl CryptoContextV2 {
    /// Create a new context with the given cipher suite.
    #[must_use]
    pub fn new(suite: CryptoSuite) -> Self {
        Self { suite }
    }

    /// Get the active cipher suite.
    #[must_use]
    pub fn suite(&self) -> CryptoSuite {
        self.suite
    }

    /// Generate a hybrid keypair appropriate for the active suite.
    ///
    /// For suites A, B, and C this generates X25519 + ML-KEM-768 keys.
    /// For suite D (classical only) this still generates the hybrid keypair
    /// but callers should use the classical-only encapsulation path.
    pub fn generate_keypair<R: RngCore + CryptoRng>(&self, rng: &mut R) -> HybridKeyPair {
        HybridKeyPair::generate(rng)
    }

    /// Encapsulate a shared secret to a peer's public key.
    ///
    /// Uses full hybrid encapsulation for PQ suites, or classical-only for Suite D.
    ///
    /// # Errors
    ///
    /// Returns an error if the public key is invalid.
    pub fn encapsulate<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        peer_public: &crate::hybrid::HybridPublicKey,
    ) -> Result<(HybridSharedSecret, EncapsulationResult), CryptoError> {
        if self.suite.supports_post_quantum() {
            let (ss, ct) = peer_public.encapsulate(rng)?;
            Ok((ss, EncapsulationResult::Hybrid(alloc::boxed::Box::new(ct))))
        } else {
            let (ss, epk) = peer_public.encapsulate_classical_only(rng)?;
            Ok((ss, EncapsulationResult::ClassicalOnly(epk)))
        }
    }

    /// Derive v2 session keys from a combined shared secret and transcript hash.
    #[must_use]
    pub fn derive_session_keys(
        &self,
        shared_secret: &[u8; 32],
        transcript_hash: &[u8; 32],
    ) -> SessionKeysV2 {
        kdf::derive_session_keys_v2(shared_secret, transcript_hash)
    }

    /// Create a per-packet ratchet from a chain key.
    #[must_use]
    pub fn create_packet_ratchet(&self, chain_key: [u8; 32]) -> PacketRatchet {
        PacketRatchet::new(chain_key)
    }
}

/// Result of encapsulation, which varies by cipher suite.
pub enum EncapsulationResult {
    /// Full hybrid ciphertext (X25519 + ML-KEM-768).
    Hybrid(alloc::boxed::Box<HybridCiphertext>),
    /// Classical-only ephemeral public key (32 bytes X25519).
    ClassicalOnly([u8; 32]),
}

impl EncapsulationResult {
    /// Serialize to bytes regardless of variant.
    #[must_use]
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        match self {
            EncapsulationResult::Hybrid(ct) => ct.to_bytes(),
            EncapsulationResult::ClassicalOnly(pk) => pk.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_context_suite_a_roundtrip() {
        let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
        assert_eq!(ctx.suite(), CryptoSuite::SuiteA);

        let kp = ctx.generate_keypair(&mut OsRng);
        let (ss_enc, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();

        // Verify it is a hybrid result
        assert!(matches!(result, EncapsulationResult::Hybrid(_)));

        // Decapsulate
        if let EncapsulationResult::Hybrid(ct) = result {
            let ss_dec = kp.secret.decapsulate(&ct).unwrap();
            assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
        }
    }

    #[test]
    fn test_context_suite_d_classical_only() {
        let ctx = CryptoContextV2::new(CryptoSuite::SuiteD);
        assert!(!ctx.suite().supports_post_quantum());

        let kp = ctx.generate_keypair(&mut OsRng);
        let (ss_enc, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();

        assert!(matches!(result, EncapsulationResult::ClassicalOnly(_)));

        if let EncapsulationResult::ClassicalOnly(epk) = result {
            let ss_dec = kp.secret.decapsulate_classical_only(&epk).unwrap();
            assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
        }
    }

    #[test]
    fn test_context_derive_session_keys() {
        let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
        let secret = [0x42u8; 32];
        let transcript = [0x43u8; 32];
        let keys = ctx.derive_session_keys(&secret, &transcript);

        assert_ne!(keys.initiator_to_responder, [0u8; 32]);
        assert_ne!(keys.responder_to_initiator, [0u8; 32]);
    }

    #[test]
    fn test_context_create_packet_ratchet() {
        let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
        let chain = [0x42u8; 32];
        let mut ratchet = ctx.create_packet_ratchet(chain);

        assert_eq!(ratchet.packet_number(), 0);
        let (pn, key) = ratchet.next_send_key();
        assert_eq!(pn, 0);
        assert_ne!(key, [0u8; 32]);
    }

    #[test]
    fn test_encapsulation_result_to_bytes() {
        let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
        let kp = ctx.generate_keypair(&mut OsRng);
        let (_ss, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();
        let bytes = result.to_bytes();
        assert!(!bytes.is_empty());
    }
}
