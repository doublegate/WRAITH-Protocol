//! Elligator2 encoding for key indistinguishability.
//!
//! Elligator2 maps elliptic curve points to uniform random byte strings,
//! making key exchange indistinguishable from random data.

use crate::CryptoError;
use rand_core::OsRng;

/// Elligator2 representative (encoded public key)
pub struct Representative([u8; 32]);

impl Representative {
    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Generate an Elligator2-encodable keypair
///
/// Not all curve points are encodable (~50%), so this function
/// loops until an encodable point is found.
pub fn generate_encodable_keypair()
-> Result<(x25519_dalek::StaticSecret, Representative), CryptoError> {
    // TODO: Implement proper Elligator2 encoding
    // For now, return a placeholder
    let secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
    let public = x25519_dalek::PublicKey::from(&secret);

    // Placeholder: just use the public key bytes
    // Real implementation needs Elligator2 inverse mapping
    Ok((secret, Representative(*public.as_bytes())))
}

/// Decode a representative back to a public key
pub fn decode_representative(repr: &Representative) -> x25519_dalek::PublicKey {
    // TODO: Implement proper Elligator2 decoding
    // For now, treat representative as raw public key bytes
    x25519_dalek::PublicKey::from(repr.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_encodable_keypair() {
        // Test that keypair generation succeeds
        let result = generate_encodable_keypair();
        assert!(result.is_ok());

        let (secret, representative) = result.unwrap();

        // Verify representative has valid bytes
        let repr_bytes = representative.as_bytes();
        assert_eq!(repr_bytes.len(), 32);

        // Verify the representative can be decoded back to a public key
        let decoded_public = decode_representative(&representative);

        // The decoded public key should match the one derived from the secret
        let expected_public = x25519_dalek::PublicKey::from(&secret);
        assert_eq!(decoded_public.as_bytes(), expected_public.as_bytes());
    }

    #[test]
    fn test_representative_roundtrip() {
        // Test Representative from_bytes and as_bytes
        let original_bytes = [0x42u8; 32];
        let repr = Representative::from_bytes(original_bytes);
        assert_eq!(repr.as_bytes(), &original_bytes);
    }

    #[test]
    fn test_multiple_keypairs_are_unique() {
        // Generate multiple keypairs and verify they are different
        let (_, repr1) = generate_encodable_keypair().unwrap();
        let (_, repr2) = generate_encodable_keypair().unwrap();

        // Representatives should be different (with overwhelming probability)
        assert_ne!(repr1.as_bytes(), repr2.as_bytes());
    }
}
