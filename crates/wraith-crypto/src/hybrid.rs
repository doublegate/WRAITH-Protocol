//! Hybrid X25519 + ML-KEM-768 Key Encapsulation Mechanism.
//!
//! Provides a hybrid KEM combining classical X25519 Diffie-Hellman with
//! post-quantum ML-KEM-768 (FIPS 203) for defense against quantum computers.
//!
//! The shared secrets from both schemes are combined using BLAKE3 keyed hashing
//! with domain separation, ensuring that the hybrid scheme is at least as strong
//! as the stronger of the two components.
//!
//! A classical-only fallback mode is provided for environments where post-quantum
//! key sizes are prohibitive or interoperability with legacy systems is required.

use crate::error::CryptoError;
use crate::pq;
use crate::x25519;
use alloc::vec::Vec;
use rand_core::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Combined shared secret (32 bytes) derived from both classical and post-quantum components.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct HybridSharedSecret([u8; 32]);

impl HybridSharedSecret {
    /// Access the raw shared secret bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Hybrid public key containing both X25519 and ML-KEM-768 components.
pub struct HybridPublicKey {
    /// Classical X25519 public key (32 bytes).
    pub classical: x25519::PublicKey,
    /// Post-quantum ML-KEM-768 encapsulation key.
    pub post_quantum: pq::PqPublicKey,
}

/// Hybrid secret key containing both X25519 and ML-KEM-768 private keys.
pub struct HybridSecretKey {
    /// Classical X25519 private key.
    classical: x25519::PrivateKey,
    /// Post-quantum ML-KEM-768 decapsulation key.
    post_quantum: pq::PqPrivateKey,
}

/// Hybrid ciphertext containing both classical and post-quantum components.
pub struct HybridCiphertext {
    /// X25519 ephemeral public key (32 bytes).
    pub classical_public: [u8; 32],
    /// ML-KEM-768 ciphertext.
    pub post_quantum: pq::PqCiphertext,
}

/// Hybrid keypair combining X25519 and ML-KEM-768.
pub struct HybridKeyPair {
    /// The secret (private) key.
    pub secret: HybridSecretKey,
    /// The public key.
    pub public: HybridPublicKey,
}

/// ML-KEM-768 ciphertext size in bytes.
const PQ_CIPHERTEXT_SIZE: usize = 1088;

/// ML-KEM-768 public key size in bytes.
const PQ_PUBLIC_KEY_SIZE: usize = 1184;

/// Combine classical and post-quantum shared secrets using BLAKE3 keyed hashing
/// with domain separation and length encoding.
fn combine_shared_secrets(classical: &[u8; 32], post_quantum: &[u8; 32]) -> HybridSharedSecret {
    let mut hasher = blake3::Hasher::new_keyed(b"wraith-v2-hybrid-kem-combine-ss\0");
    hasher.update(classical);
    hasher.update(post_quantum);
    hasher.update(&(classical.len() as u32).to_le_bytes());
    hasher.update(&(post_quantum.len() as u32).to_le_bytes());
    HybridSharedSecret(*hasher.finalize().as_bytes())
}

impl HybridKeyPair {
    /// Generate a new hybrid keypair using both X25519 and ML-KEM-768.
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let classical_sk = x25519::PrivateKey::generate(rng);
        let classical_pk = classical_sk.public_key();
        let (pq_pk, pq_sk) = pq::generate_keypair(rng);

        HybridKeyPair {
            secret: HybridSecretKey {
                classical: classical_sk,
                post_quantum: pq_sk,
            },
            public: HybridPublicKey {
                classical: classical_pk,
                post_quantum: pq_pk,
            },
        }
    }
}

impl HybridPublicKey {
    /// Encapsulate a shared secret to this public key.
    ///
    /// Generates an ephemeral X25519 keypair, performs DH with the recipient's
    /// classical public key, encapsulates to the ML-KEM-768 public key, and
    /// combines both shared secrets.
    ///
    /// Returns the combined shared secret and hybrid ciphertext.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] if the classical DH produces
    /// an all-zero shared secret (low-order point).
    pub fn encapsulate<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<(HybridSharedSecret, HybridCiphertext), CryptoError> {
        // Classical: ephemeral X25519 DH
        let ephemeral_sk = x25519::PrivateKey::generate(rng);
        let ephemeral_pk = ephemeral_sk.public_key();
        let classical_ss = ephemeral_sk
            .exchange(&self.classical)
            .ok_or(CryptoError::InvalidPublicKey)?;

        // Post-quantum: ML-KEM-768 encapsulation
        let (pq_ct, pq_ss) = pq::encapsulate(rng, &self.post_quantum);

        // Combine shared secrets
        let combined = combine_shared_secrets(classical_ss.as_bytes(), &pq_ss);

        let ciphertext = HybridCiphertext {
            classical_public: ephemeral_pk.to_bytes(),
            post_quantum: pq_ct,
        };

        Ok((combined, ciphertext))
    }

    /// Encapsulate using only the classical X25519 component (no post-quantum).
    ///
    /// This fallback mode produces smaller ciphertexts at the cost of losing
    /// post-quantum security.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] if the classical DH produces
    /// an all-zero shared secret (low-order point).
    pub fn encapsulate_classical_only<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<(HybridSharedSecret, [u8; 32]), CryptoError> {
        let ephemeral_sk = x25519::PrivateKey::generate(rng);
        let ephemeral_pk = ephemeral_sk.public_key();
        let classical_ss = ephemeral_sk
            .exchange(&self.classical)
            .ok_or(CryptoError::InvalidPublicKey)?;

        // Use zero post-quantum contribution for classical-only mode
        let zero_pq = [0u8; 32];
        let combined = combine_shared_secrets(classical_ss.as_bytes(), &zero_pq);

        Ok((combined, ephemeral_pk.to_bytes()))
    }

    /// Serialize the hybrid public key to bytes.
    ///
    /// Format: 32 bytes X25519 || ML-KEM-768 public key bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let pq_bytes = pq::public_key_to_vec(&self.post_quantum);
        let mut out = Vec::with_capacity(32 + pq_bytes.len());
        out.extend_from_slice(&self.classical.to_bytes());
        out.extend_from_slice(&pq_bytes);
        out
    }

    /// Deserialize a hybrid public key from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKeyLength`] if the input is not the expected size.
    /// Returns [`CryptoError::InvalidKeyMaterial`] if the ML-KEM-768 key bytes are invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let expected = 32 + PQ_PUBLIC_KEY_SIZE;
        if bytes.len() != expected {
            return Err(CryptoError::InvalidKeyLength {
                expected,
                actual: bytes.len(),
            });
        }
        let mut classical_bytes = [0u8; 32];
        classical_bytes.copy_from_slice(&bytes[..32]);
        let classical = x25519::PublicKey::from_bytes(classical_bytes);
        let post_quantum =
            pq::public_key_from_bytes(&bytes[32..]).map_err(|_| CryptoError::InvalidKeyMaterial)?;
        Ok(Self {
            classical,
            post_quantum,
        })
    }
}

impl HybridSecretKey {
    /// Decapsulate a shared secret from a hybrid ciphertext.
    ///
    /// Performs X25519 DH with the ephemeral public key from the ciphertext,
    /// decapsulates the ML-KEM-768 component, and combines both shared secrets.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] if the classical DH produces
    /// an all-zero shared secret (low-order point).
    pub fn decapsulate(&self, ct: &HybridCiphertext) -> Result<HybridSharedSecret, CryptoError> {
        // Classical: X25519 DH with ephemeral public key
        let ephemeral_pk = x25519::PublicKey::from_bytes(ct.classical_public);
        let classical_ss = self
            .classical
            .exchange(&ephemeral_pk)
            .ok_or(CryptoError::InvalidPublicKey)?;

        // Post-quantum: ML-KEM-768 decapsulation
        let pq_ss = pq::decapsulate(&self.post_quantum, &ct.post_quantum);

        // Combine shared secrets
        let combined = combine_shared_secrets(classical_ss.as_bytes(), &pq_ss);

        Ok(combined)
    }

    /// Decapsulate using only the classical X25519 component.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] if the classical DH produces
    /// an all-zero shared secret.
    pub fn decapsulate_classical_only(
        &self,
        ephemeral_public: &[u8; 32],
    ) -> Result<HybridSharedSecret, CryptoError> {
        let ephemeral_pk = x25519::PublicKey::from_bytes(*ephemeral_public);
        let classical_ss = self
            .classical
            .exchange(&ephemeral_pk)
            .ok_or(CryptoError::InvalidPublicKey)?;

        let zero_pq = [0u8; 32];
        let combined = combine_shared_secrets(classical_ss.as_bytes(), &zero_pq);

        Ok(combined)
    }
}

impl HybridCiphertext {
    /// Serialize the hybrid ciphertext to bytes.
    ///
    /// Format: 32 bytes X25519 ephemeral public key || ML-KEM-768 ciphertext.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let pq_bytes = pq::ciphertext_to_vec(&self.post_quantum);
        let mut out = Vec::with_capacity(32 + pq_bytes.len());
        out.extend_from_slice(&self.classical_public);
        out.extend_from_slice(&pq_bytes);
        out
    }

    /// Deserialize a hybrid ciphertext from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKeyLength`] if the input is not the expected size.
    /// Returns [`CryptoError::InvalidKeyMaterial`] if the ML-KEM-768 ciphertext is invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let expected = 32 + PQ_CIPHERTEXT_SIZE;
        if bytes.len() != expected {
            return Err(CryptoError::InvalidKeyLength {
                expected,
                actual: bytes.len(),
            });
        }
        let mut classical_public = [0u8; 32];
        classical_public.copy_from_slice(&bytes[..32]);
        let post_quantum =
            pq::ciphertext_from_bytes(&bytes[32..]).map_err(|_| CryptoError::InvalidKeyMaterial)?;
        Ok(Self {
            classical_public,
            post_quantum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_hybrid_roundtrip() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let (ss_enc, ct) = kp.public.encapsulate(&mut OsRng).unwrap();
        let ss_dec = kp.secret.decapsulate(&ct).unwrap();
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_hybrid_different_keypairs_different_secrets() {
        let kp1 = HybridKeyPair::generate(&mut OsRng);
        let kp2 = HybridKeyPair::generate(&mut OsRng);

        let (ss1, _ct1) = kp1.public.encapsulate(&mut OsRng).unwrap();
        let (ss2, _ct2) = kp2.public.encapsulate(&mut OsRng).unwrap();

        // Different keypairs should produce different shared secrets
        assert_ne!(ss1.as_bytes(), ss2.as_bytes());
    }

    #[test]
    fn test_hybrid_wrong_key_fails() {
        let kp1 = HybridKeyPair::generate(&mut OsRng);
        let kp2 = HybridKeyPair::generate(&mut OsRng);

        let (ss_enc, ct) = kp1.public.encapsulate(&mut OsRng).unwrap();

        // Decapsulate with wrong key -- classical DH will differ
        let ss_dec = kp2.secret.decapsulate(&ct).unwrap();
        assert_ne!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_hybrid_classical_only_roundtrip() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let (ss_enc, ephemeral_pk) = kp.public.encapsulate_classical_only(&mut OsRng).unwrap();
        let ss_dec = kp.secret.decapsulate_classical_only(&ephemeral_pk).unwrap();
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_hybrid_classical_differs_from_full() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let (ss_full, _ct) = kp.public.encapsulate(&mut OsRng).unwrap();
        let (ss_classical, _pk) = kp.public.encapsulate_classical_only(&mut OsRng).unwrap();

        // Classical-only and full hybrid produce different secrets
        assert_ne!(ss_full.as_bytes(), ss_classical.as_bytes());
    }

    #[test]
    fn test_hybrid_shared_secret_nonzero() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let (ss, _ct) = kp.public.encapsulate(&mut OsRng).unwrap();
        assert_ne!(ss.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_hybrid_public_key_serialization() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let bytes = kp.public.to_bytes();
        let recovered = HybridPublicKey::from_bytes(&bytes).unwrap();

        assert_eq!(
            kp.public.classical.to_bytes(),
            recovered.classical.to_bytes()
        );
    }

    #[test]
    fn test_hybrid_public_key_invalid_length() {
        let result = HybridPublicKey::from_bytes(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_hybrid_ciphertext_serialization() {
        let kp = HybridKeyPair::generate(&mut OsRng);
        let (_ss, ct) = kp.public.encapsulate(&mut OsRng).unwrap();
        let bytes = ct.to_bytes();
        let recovered = HybridCiphertext::from_bytes(&bytes).unwrap();

        assert_eq!(ct.classical_public, recovered.classical_public);
    }

    #[test]
    fn test_hybrid_ciphertext_invalid_length() {
        let result = HybridCiphertext::from_bytes(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_combine_shared_secrets_deterministic() {
        let a = [0x42u8; 32];
        let b = [0x43u8; 32];
        let s1 = combine_shared_secrets(&a, &b);
        let s2 = combine_shared_secrets(&a, &b);
        assert_eq!(s1.as_bytes(), s2.as_bytes());
    }

    #[test]
    fn test_combine_shared_secrets_order_matters() {
        let a = [0x42u8; 32];
        let b = [0x43u8; 32];
        let s1 = combine_shared_secrets(&a, &b);
        let s2 = combine_shared_secrets(&b, &a);
        assert_ne!(s1.as_bytes(), s2.as_bytes());
    }

    #[test]
    fn test_hybrid_encapsulate_decapsulate_multiple() {
        let kp = HybridKeyPair::generate(&mut OsRng);

        // Multiple encapsulations produce different shared secrets
        let (ss1, ct1) = kp.public.encapsulate(&mut OsRng).unwrap();
        let (ss2, ct2) = kp.public.encapsulate(&mut OsRng).unwrap();

        assert_ne!(ss1.as_bytes(), ss2.as_bytes());

        // But both decapsulate correctly
        let dec1 = kp.secret.decapsulate(&ct1).unwrap();
        let dec2 = kp.secret.decapsulate(&ct2).unwrap();
        assert_eq!(ss1.as_bytes(), dec1.as_bytes());
        assert_eq!(ss2.as_bytes(), dec2.as_bytes());
    }
}
