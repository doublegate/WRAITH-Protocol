//! ML-DSA-65 post-quantum digital signatures (FIPS 204).
//!
//! Provides ML-DSA-65 (formerly CRYSTALS-Dilithium) signatures for post-quantum
//! authentication, and a hybrid signature scheme combining Ed25519 with ML-DSA-65
//! for defense-in-depth.
//!
//! This module is gated behind the `pq-signatures` feature flag because the
//! underlying `ml-dsa` crate is still pre-1.0 and its API may change.
//!
//! When the `pq-signatures` feature is not enabled, the module provides type
//! definitions and a trait interface but no implementation, allowing downstream
//! code to be written against the interface without requiring the dependency.

use crate::error::CryptoError;
use alloc::vec::Vec;

/// ML-DSA-65 signature bytes. Variable length (up to ~3309 bytes for ML-DSA-65).
#[derive(Clone, Debug)]
pub struct MlDsa65Signature {
    /// Raw signature bytes.
    bytes: Vec<u8>,
}

impl MlDsa65Signature {
    /// Create a signature from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Get the raw signature bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// ML-DSA-65 public key for signature verification.
#[derive(Clone, Debug)]
pub struct MlDsa65VerifyingKey {
    /// Raw public key bytes.
    bytes: Vec<u8>,
}

impl MlDsa65VerifyingKey {
    /// Create a verifying key from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Get the raw public key bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Verify a signature on a message.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] if the signature is invalid.
    #[cfg(feature = "pq-signatures")]
    pub fn verify(&self, message: &[u8], signature: &MlDsa65Signature) -> Result<(), CryptoError> {
        use ml_dsa::signature::Verifier;
        let enc = ml_dsa::EncodedVerifyingKey::<ml_dsa::MlDsa65>::try_from(self.bytes.as_slice())
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        let vk = ml_dsa::VerifyingKey::<ml_dsa::MlDsa65>::decode(&enc);
        let sig = ml_dsa::Signature::<ml_dsa::MlDsa65>::try_from(signature.as_bytes())
            .map_err(|_| CryptoError::InvalidSignature)?;
        vk.verify(message, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Verify a signature (stub when `pq-signatures` feature is not enabled).
    ///
    /// # Errors
    ///
    /// Always returns [`CryptoError::InvalidState`] when the feature is disabled.
    #[cfg(not(feature = "pq-signatures"))]
    pub fn verify(
        &self,
        _message: &[u8],
        _signature: &MlDsa65Signature,
    ) -> Result<(), CryptoError> {
        Err(CryptoError::InvalidState)
    }
}

/// ML-DSA-65 signing key.
#[allow(dead_code)]
pub struct MlDsa65SigningKey {
    /// 32-byte seed for deterministic key regeneration.
    seed: Vec<u8>,
    /// Corresponding verifying (public) key.
    verifying_key: MlDsa65VerifyingKey,
}

impl MlDsa65SigningKey {
    /// Generate a new ML-DSA-65 keypair.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidState`] when the `pq-signatures` feature is not enabled.
    #[cfg(feature = "pq-signatures")]
    pub fn generate<R: rand_core::RngCore + rand_core::CryptoRng>(rng: &mut R) -> Self {
        // Generate a 32-byte seed using the caller's RNG (rand_core 0.6).
        // Then use from_seed which avoids the rand_core version mismatch.
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        let seed_array = ml_dsa::Seed::try_from(seed.as_slice()).expect("seed is exactly 32 bytes");
        let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed_array);
        let vk = sk.verifying_key();
        let vk_bytes: Vec<u8> = vk.encode().to_vec();
        Self {
            seed: seed.to_vec(),
            verifying_key: MlDsa65VerifyingKey::from_bytes(vk_bytes),
        }
    }

    /// Generate a new ML-DSA-65 keypair (stub).
    ///
    /// # Panics
    ///
    /// Panics when the `pq-signatures` feature is not enabled.
    #[cfg(not(feature = "pq-signatures"))]
    pub fn generate<R: rand_core::RngCore + rand_core::CryptoRng>(_rng: &mut R) -> Self {
        panic!("ML-DSA-65 requires the `pq-signatures` feature")
    }

    /// Sign a message.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidState`] when the `pq-signatures` feature is not enabled.
    #[cfg(feature = "pq-signatures")]
    pub fn sign(&self, message: &[u8]) -> Result<MlDsa65Signature, CryptoError> {
        use ml_dsa::signature::Signer;
        let seed = ml_dsa::Seed::try_from(self.seed.as_slice())
            .map_err(|_| CryptoError::InvalidKeyMaterial)?;
        let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
        let sig: ml_dsa::Signature<ml_dsa::MlDsa65> = sk.sign(message);
        use ml_dsa::signature::SignatureEncoding;
        Ok(MlDsa65Signature::from_bytes(sig.to_bytes().to_vec()))
    }

    /// Sign a message (stub).
    ///
    /// # Errors
    ///
    /// Always returns [`CryptoError::InvalidState`] when the feature is disabled.
    #[cfg(not(feature = "pq-signatures"))]
    pub fn sign(&self, _message: &[u8]) -> Result<MlDsa65Signature, CryptoError> {
        Err(CryptoError::InvalidState)
    }

    /// Get the corresponding verifying (public) key.
    #[must_use]
    pub fn verifying_key(&self) -> &MlDsa65VerifyingKey {
        &self.verifying_key
    }
}

/// Hybrid signature combining Ed25519 and ML-DSA-65.
///
/// Both signatures must independently verify for the hybrid to be valid.
/// A binding proof ties the two signatures together to prevent stripping attacks.
#[derive(Clone, Debug)]
pub struct HybridSignature {
    /// Classical Ed25519 signature (64 bytes).
    pub classical: crate::signatures::Signature,
    /// Post-quantum ML-DSA-65 signature.
    pub post_quantum: MlDsa65Signature,
    /// Binding proof: BLAKE3 hash of (ed25519_sig || mldsa_sig || message).
    pub binding: [u8; 32],
}

/// Hybrid signing key pair combining Ed25519 and ML-DSA-65.
pub struct HybridSigningKey {
    /// Classical Ed25519 signing key.
    pub classical: crate::signatures::SigningKey,
    /// Post-quantum ML-DSA-65 signing key.
    pub post_quantum: MlDsa65SigningKey,
}

/// Hybrid verifying key combining Ed25519 and ML-DSA-65 public keys.
#[derive(Clone, Debug)]
pub struct HybridVerifyingKey {
    /// Classical Ed25519 verifying key.
    pub classical: crate::signatures::VerifyingKey,
    /// Post-quantum ML-DSA-65 verifying key.
    pub post_quantum: MlDsa65VerifyingKey,
}

/// Compute binding proof for a hybrid signature.
fn compute_binding(ed25519_sig: &[u8; 64], mldsa_sig: &[u8], message: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"wraith-v2-hybrid-sig-binding");
    hasher.update(ed25519_sig);
    hasher.update(mldsa_sig);
    hasher.update(message);
    *hasher.finalize().as_bytes()
}

impl HybridSigningKey {
    /// Generate a new hybrid signing keypair.
    pub fn generate<R: rand_core::RngCore + rand_core::CryptoRng>(rng: &mut R) -> Self {
        let classical = crate::signatures::SigningKey::generate(rng);
        let post_quantum = MlDsa65SigningKey::generate(rng);
        Self {
            classical,
            post_quantum,
        }
    }

    /// Sign a message with both Ed25519 and ML-DSA-65, producing a hybrid signature
    /// with a binding proof.
    ///
    /// # Errors
    ///
    /// Returns an error if the ML-DSA-65 signing fails.
    pub fn sign(&self, message: &[u8]) -> Result<HybridSignature, CryptoError> {
        let ed_sig = self.classical.sign(message);
        let pq_sig = self.post_quantum.sign(message)?;
        let binding = compute_binding(ed_sig.as_bytes(), pq_sig.as_bytes(), message);

        Ok(HybridSignature {
            classical: ed_sig,
            post_quantum: pq_sig,
            binding,
        })
    }

    /// Get the corresponding hybrid verifying key.
    #[must_use]
    pub fn verifying_key(&self) -> HybridVerifyingKey {
        HybridVerifyingKey {
            classical: self.classical.verifying_key(),
            post_quantum: self.post_quantum.verifying_key().clone(),
        }
    }
}

impl HybridVerifyingKey {
    /// Verify a hybrid signature on a message.
    ///
    /// Both the Ed25519 and ML-DSA-65 signatures must verify, and the binding
    /// proof must match.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] if either signature is invalid
    /// or the binding proof does not match.
    pub fn verify(&self, message: &[u8], signature: &HybridSignature) -> Result<(), CryptoError> {
        // Verify Ed25519
        self.classical.verify(message, &signature.classical)?;

        // Verify ML-DSA-65
        self.post_quantum.verify(message, &signature.post_quantum)?;

        // Verify binding proof
        let expected_binding = compute_binding(
            signature.classical.as_bytes(),
            signature.post_quantum.as_bytes(),
            message,
        );
        if expected_binding != signature.binding {
            return Err(CryptoError::InvalidSignature);
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "pq-signatures")]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_mldsa65_sign_verify() {
        let sk = MlDsa65SigningKey::generate(&mut OsRng);
        let message = b"test message for ML-DSA-65";
        let sig = sk.sign(message).unwrap();
        let result = sk.verifying_key().verify(message, &sig);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mldsa65_wrong_message() {
        let sk = MlDsa65SigningKey::generate(&mut OsRng);
        let sig = sk.sign(b"correct message").unwrap();
        let result = sk.verifying_key().verify(b"wrong message", &sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_mldsa65_wrong_key() {
        let sk1 = MlDsa65SigningKey::generate(&mut OsRng);
        let sk2 = MlDsa65SigningKey::generate(&mut OsRng);
        let message = b"test";
        let sig = sk1.sign(message).unwrap();
        let result = sk2.verifying_key().verify(message, &sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_hybrid_sign_verify() {
        let hsk = HybridSigningKey::generate(&mut OsRng);
        let hvk = hsk.verifying_key();
        let message = b"hybrid signature test";
        let sig = hsk.sign(message).unwrap();
        assert!(hvk.verify(message, &sig).is_ok());
    }

    #[test]
    fn test_hybrid_wrong_message() {
        let hsk = HybridSigningKey::generate(&mut OsRng);
        let hvk = hsk.verifying_key();
        let sig = hsk.sign(b"correct").unwrap();
        assert!(hvk.verify(b"wrong", &sig).is_err());
    }

    #[test]
    fn test_hybrid_tampered_binding() {
        let hsk = HybridSigningKey::generate(&mut OsRng);
        let hvk = hsk.verifying_key();
        let message = b"test";
        let mut sig = hsk.sign(message).unwrap();
        sig.binding[0] ^= 0xFF;
        assert!(hvk.verify(message, &sig).is_err());
    }
}
