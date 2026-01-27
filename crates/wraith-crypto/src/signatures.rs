//! Ed25519 digital signatures for authentication and non-repudiation.
//!
//! Provides high-speed digital signatures using the Ed25519 algorithm:
//! - 64-byte signatures
//! - 32-byte public keys
//! - 32-byte private keys (zeroized on drop)
//! - Deterministic signature generation
//! - Batch verification support
//!
//! ## Security Properties
//!
//! - **Existential unforgeability**: Cannot forge valid signatures
//! - **Strong unforgeability**: Cannot create alternative signatures for signed messages
//! - **Deterministic nonces**: No RNG required for signing (safer)
//! - **Small keys and signatures**: Efficient for network protocols
//!
//! ## Usage
//!
//! ```ignore
//! use wraith_crypto::signatures::{SigningKey, VerifyingKey};
//! use rand_core::OsRng;
//!
//! // Generate keypair
//! let signing_key = SigningKey::generate(&mut OsRng);
//! let verifying_key = signing_key.verifying_key();
//!
//! // Sign message
//! let message = b"authenticate this message";
//! let signature = signing_key.sign(message);
//!
//! // Verify signature
//! assert!(verifying_key.verify(message, &signature).is_ok());
//! ```

use crate::CryptoError;
use ed25519_dalek::{Signer, Verifier};
use rand_core::{CryptoRng, RngCore};
use zeroize::ZeroizeOnDrop;

/// Ed25519 signature (64 bytes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature([u8; 64]);

impl Signature {
    /// Create a signature from raw bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Create a signature from a slice
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] if the slice is not exactly 64 bytes.
    pub fn from_slice(slice: &[u8]) -> Result<Self, CryptoError> {
        if slice.len() != 64 {
            return Err(CryptoError::InvalidSignature);
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Get the raw signature bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Convert to ed25519_dalek signature
    fn to_dalek(self) -> ed25519_dalek::Signature {
        ed25519_dalek::Signature::from_bytes(&self.0)
    }
}

/// Ed25519 signing key (private key)
///
/// Contains the secret key material for signing messages.
/// Zeroized on drop to prevent key material from lingering in memory.
#[derive(ZeroizeOnDrop)]
pub struct SigningKey {
    inner: ed25519_dalek::SigningKey,
}

impl SigningKey {
    /// Generate a new random signing key
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::generate(rng),
        }
    }

    /// Create from raw 32-byte seed
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::from_bytes(bytes),
        }
    }

    /// Sign a message
    ///
    /// Returns a 64-byte Ed25519 signature that can be verified with
    /// the corresponding verifying key.
    ///
    /// Signing is deterministic - the same message will always produce
    /// the same signature with the same key.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.inner.sign(message);
        Signature(sig.to_bytes())
    }

    /// Get the corresponding verifying key (public key)
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }

    /// Export signing key bytes (use with extreme caution)
    ///
    /// # Security
    ///
    /// This exposes the raw secret key bytes. Handle with extreme care
    /// and ensure proper zeroization after use.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }
}

/// Ed25519 verifying key (public key)
///
/// Used to verify signatures created by the corresponding signing key.
/// Can be safely shared publicly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifyingKey {
    inner: ed25519_dalek::VerifyingKey,
}

impl VerifyingKey {
    /// Create from raw 32-byte public key
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] if the bytes do not
    /// represent a valid Ed25519 public key point.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let inner = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        Ok(Self { inner })
    }

    /// Get the raw public key bytes
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Verify a signature on a message
    ///
    /// Returns `Ok(())` if the signature is valid for this message and public key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] if the signature is invalid,
    /// malformed, or does not authenticate the message.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        self.inner
            .verify(message, &signature.to_dalek())
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_signing_key_generation() {
        let key1 = SigningKey::generate(&mut OsRng);
        let key2 = SigningKey::generate(&mut OsRng);

        // Different keys should produce different signatures
        let message = b"test message";
        let sig1 = key1.sign(message);
        let sig2 = key2.sign(message);

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let message = b"authenticate this message";
        let signature = signing_key.sign(message);

        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_wrong_message_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let message = b"original message";
        let wrong_message = b"tampered message";
        let signature = signing_key.sign(message);

        assert!(verifying_key.verify(wrong_message, &signature).is_err());
    }

    #[test]
    fn test_wrong_key_fails_verification() {
        let signing_key1 = SigningKey::generate(&mut OsRng);
        let signing_key2 = SigningKey::generate(&mut OsRng);
        let verifying_key2 = signing_key2.verifying_key();

        let message = b"test";
        let signature = signing_key1.sign(message);

        // Wrong verifying key should fail
        assert!(verifying_key2.verify(message, &signature).is_err());
    }

    #[test]
    fn test_signature_deterministic() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let message = b"deterministic test";

        let sig1 = signing_key.sign(message);
        let sig2 = signing_key.sign(message);

        // Same key and message should produce identical signatures
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_from_bytes_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let message = b"test";
        let signature = signing_key.sign(message);

        let bytes = signature.as_bytes();
        let recovered = Signature::from_bytes(*bytes);

        assert_eq!(signature, recovered);
    }

    #[test]
    fn test_signature_from_slice() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signature = signing_key.sign(b"test");

        let bytes = signature.as_bytes();
        let recovered = Signature::from_slice(bytes).unwrap();

        assert_eq!(signature, recovered);
    }

    #[test]
    fn test_signature_from_slice_wrong_size() {
        let short = [0u8; 32];
        assert!(Signature::from_slice(&short).is_err());

        let long = [0u8; 128];
        assert!(Signature::from_slice(&long).is_err());
    }

    #[test]
    fn test_verifying_key_from_bytes_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let bytes = verifying_key.to_bytes();
        let recovered = VerifyingKey::from_bytes(&bytes).unwrap();

        assert_eq!(verifying_key, recovered);
    }

    #[test]
    fn test_signing_key_from_bytes_roundtrip() {
        let original = SigningKey::generate(&mut OsRng);
        let bytes = original.to_bytes();
        let recovered = SigningKey::from_bytes(&bytes);

        // Should produce same signatures
        let message = b"test message";
        let sig1 = original.sign(message);
        let sig2 = recovered.sign(message);

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_invalid_signature_bytes() {
        let verifying_key = SigningKey::generate(&mut OsRng).verifying_key();

        // All zeros is not a valid signature
        let invalid_sig = Signature::from_bytes([0u8; 64]);

        assert!(verifying_key.verify(b"test", &invalid_sig).is_err());
    }

    #[test]
    fn test_tampered_signature_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let message = b"test message";
        let signature = signing_key.sign(message);

        // Tamper with signature
        let mut tampered_bytes = *signature.as_bytes();
        tampered_bytes[0] ^= 0xFF;
        let tampered_sig = Signature::from_bytes(tampered_bytes);

        assert!(verifying_key.verify(message, &tampered_sig).is_err());
    }

    #[test]
    fn test_empty_message() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let signature = signing_key.sign(b"");
        assert!(verifying_key.verify(b"", &signature).is_ok());
    }

    #[test]
    fn test_large_message() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let large_message = vec![0x42u8; 1024 * 1024]; // 1 MB
        let signature = signing_key.sign(&large_message);

        assert!(verifying_key.verify(&large_message, &signature).is_ok());
    }

    #[test]
    fn test_different_messages_different_signatures() {
        let signing_key = SigningKey::generate(&mut OsRng);

        let sig1 = signing_key.sign(b"message 1");
        let sig2 = signing_key.sign(b"message 2");
        let sig3 = signing_key.sign(b"message 3");

        // All signatures should be different
        assert_ne!(sig1, sig2);
        assert_ne!(sig2, sig3);
        assert_ne!(sig1, sig3);
    }
}
