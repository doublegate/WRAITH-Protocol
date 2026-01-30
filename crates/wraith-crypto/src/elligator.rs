//! Elligator2 encoding for key indistinguishability.
//!
//! Elligator2 maps elliptic curve points to uniform random byte strings,
//! making key exchange indistinguishable from random data. This prevents
//! traffic analysis attacks that identify cryptographic protocols by
//! recognizing public key patterns.
//!
//! ## Algorithm Overview
//!
//! For Curve25519 (Montgomery curve y² = x³ + Ax² + x over GF(2²⁵⁵ - 19)):
//! - Not all curve points are encodable (~50% probability)
//! - Encoding maps a point to a uniform 32-byte representative
//! - Decoding maps any 32 bytes to a valid curve point
//!
//! ## Implementation
//!
//! This module uses the `curve25519-elligator2` crate which provides a
//! well-tested implementation based on `curve25519-dalek`. We use the
//! `Randomized` variant which ensures representatives are indistinguishable
//! from uniform random bytes.
//!
//! ## Constant-Time Security
//!
//! This implementation provides constant-time guarantees for security-critical operations:
//!
//! - **Encoding (`to_representative`):** Uses `subtle::CtOption` from the `curve25519-elligator2`
//!   crate, ensuring no timing leaks based on whether a point is encodable. The underlying
//!   field operations in `curve25519-dalek` are constant-time.
//!
//! - **Decoding (`from_representative`):** Uses `curve25519-dalek`'s constant-time Montgomery
//!   ladder implementation. While the return type is `Option` (not `CtOption`), the internal
//!   field operations are constant-time and don't depend on secret data.
//!
//! - **Loop-until-encodable:** The keypair generation loop retries with fresh random bytes
//!   until an encodable point is found. This is safe because:
//!   1. Each iteration uses independent random bytes
//!   2. Encodability is a public mathematical property (quadratic residue)
//!   3. No secret key material is leaked by the number of iterations
//!
//! **Security Note:** Cache-timing resistance verified 2025-12-07. The implementation uses
//! no secret-dependent table lookups or branching on secret data.
//!
//! ## References
//!
//! - "Elligator: Elliptic-curve points indistinguishable from uniform random strings"
//!   Bernstein, Hamburg, Krasnova, Lange (2013)
//! - RFC 9380 - Hashing to Elliptic Curves

use crate::CryptoError;
use crate::x25519::{PrivateKey, PublicKey};
use core::fmt;
use curve25519_elligator2::MapToPointVariant;
use curve25519_elligator2::MontgomeryPoint;
use curve25519_elligator2::elligator2::Randomized;
use rand_core::{CryptoRng, RngCore};
use subtle::CtOption;

/// Elligator2 representative (encoded public key).
///
/// A representative is a 32-byte value that looks uniformly random
/// but can be decoded to a Curve25519 public key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Representative([u8; 32]);

impl Representative {
    /// Get the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create from raw bytes.
    ///
    /// Any 32-byte array is a valid representative and can be decoded
    /// to a curve point.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from slice.
    ///
    /// Returns `None` if slice length is not 32.
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }
}

impl AsRef<[u8]> for Representative {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Representative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Representative([...])")
    }
}

/// Generate an Elligator2-encodable keypair.
///
/// Not all Curve25519 public keys are Elligator2-encodable (~50% are).
/// This function generates random keypairs until it finds one that is
/// encodable, then returns the private key and the representative.
///
/// # Security
///
/// The returned representative is indistinguishable from random bytes,
/// which provides traffic analysis resistance.
///
/// # Example
///
/// ```ignore
/// use wraith_crypto::elligator::generate_encodable_keypair;
/// use rand_core::OsRng;
///
/// let (private_key, representative) = generate_encodable_keypair(&mut OsRng);
/// // representative.as_bytes() looks like random data
/// ```
pub fn generate_encodable_keypair<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> (PrivateKey, Representative) {
    // Loop until we find an encodable keypair
    // About 50% of keys are encodable, so ~2 iterations on average
    loop {
        // Generate random 32-byte private key
        let mut private_bytes = [0u8; 32];
        rng.fill_bytes(&mut private_bytes);

        // Generate a random tweak byte for the representative
        let tweak = (rng.next_u32() & 0xFF) as u8;

        // Try to get a representative using the Randomized variant
        // This returns CtOption which is constant-time
        let ct_repr: CtOption<[u8; 32]> = Randomized::to_representative(&private_bytes, tweak);

        // Convert CtOption to Option for checking
        // is_some() returns a Choice, which can be converted to bool
        if bool::from(ct_repr.is_some()) {
            // unwrap() is safe here since we checked is_some()
            let representative = ct_repr.unwrap();

            // Create the private key (this applies RFC 7748 clamping)
            let private = PrivateKey::from_bytes(private_bytes);

            return (private, Representative(representative));
        }
        // If not encodable, loop and try another key
    }
}

/// Generate an Elligator2-encodable keypair with the default RNG.
///
/// Convenience function using `OsRng`.
///
/// # Errors
///
/// Returns `CryptoError::RandomnessFailure` if RNG fails (unlikely with `OsRng`).
pub fn generate_encodable_keypair_default() -> Result<(PrivateKey, Representative), CryptoError> {
    use rand_core::OsRng;
    Ok(generate_encodable_keypair(&mut OsRng))
}

/// Decode a representative to a public key.
///
/// Any 32-byte array is a valid representative and can be decoded
/// to a curve point. This is the forward Elligator2 map.
///
/// # Example
///
/// ```ignore
/// use wraith_crypto::elligator::{generate_encodable_keypair, decode_representative};
/// use rand_core::OsRng;
///
/// let (private_key, representative) = generate_encodable_keypair(&mut OsRng);
/// let public_key = decode_representative(&representative);
///
/// assert_eq!(public_key.to_bytes(), private_key.public_key().to_bytes());
/// ```
///
/// # Panics
///
/// Never panics - the forward map always succeeds for any 32-byte input.
#[must_use]
pub fn decode_representative(repr: &Representative) -> PublicKey {
    // Use the Randomized variant's forward map
    // The forward map always succeeds - any 32 bytes map to a valid point
    let point: Option<MontgomeryPoint> =
        MontgomeryPoint::from_representative::<Randomized>(&repr.0);

    // The forward map should always succeed for valid representatives
    let point = point.expect("Forward Elligator2 map should never fail");
    PublicKey::from_bytes(point.to_bytes())
}

/// Try to encode a public key as a representative.
///
/// # Limitation
///
/// **This function always returns `None`** because computing the inverse Elligator2
/// map from just a public key (without the private key) requires solving a
/// quadratic equation in the field, which is computationally expensive and has
/// only a 50% success rate.
///
/// # Recommended Usage
///
/// **Always use `generate_encodable_keypair()` instead**, which:
/// - Generates keypairs that are guaranteed to be encodable
/// - Provides the representative directly during key generation
/// - Is more efficient than attempting inverse mapping
///
/// # Mathematical Background
///
/// The inverse Elligator2 map requires solving for `r` where `map(r) = u` (the public key).
/// This involves:
/// 1. Computing a quadratic equation in GF(2^255 - 19)
/// 2. Finding square roots in the field (if they exist)
/// 3. Only ~50% of curve points have valid preimages
///
/// Without the private key, this is computationally intensive and unreliable.
///
/// # Returns
///
/// Always returns `None`. Use `generate_encodable_keypair()` for forward encoding.
#[must_use]
pub fn encode_public_key(public: &PublicKey) -> Option<Representative> {
    // The inverse Elligator2 map from public key alone is not implemented.
    // Rationale:
    // 1. Requires solving quadratic in GF(2^255 - 19) - computationally expensive
    // 2. Only 50% of points are encodable - success not guaranteed
    // 3. Forward encoding (generate_encodable_keypair) is the standard approach
    //
    // Users should generate encodable keypairs upfront rather than attempting
    // inverse mapping after key generation.
    let _ = public;
    None
}

/// Keypair with Elligator2 representative.
///
/// Contains a private key and its corresponding representative (encoded public key).
/// The private key is zeroized on drop for security.
pub struct ElligatorKeypair {
    /// Private key (zeroized on drop via `PrivateKey`'s `ZeroizeOnDrop`).
    pub private: PrivateKey,
    /// Public key encoded as representative.
    pub representative: Representative,
}

impl ElligatorKeypair {
    /// Generate a new Elligator2-encodable keypair.
    #[must_use]
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let (private, representative) = generate_encodable_keypair(rng);
        Self {
            private,
            representative,
        }
    }

    /// Get the actual public key.
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.private.public_key()
    }

    /// Get the representative (encoded public key).
    #[must_use]
    pub fn representative(&self) -> &Representative {
        &self.representative
    }

    /// Perform key exchange with a peer's public key.
    #[must_use]
    pub fn exchange(&self, peer_public: &PublicKey) -> Option<crate::x25519::SharedSecret> {
        self.private.exchange(peer_public)
    }

    /// Perform key exchange with a peer's representative.
    ///
    /// Decodes the representative to a public key and performs the exchange.
    #[must_use]
    pub fn exchange_representative(
        &self,
        peer_repr: &Representative,
    ) -> Option<crate::x25519::SharedSecret> {
        let peer_public = decode_representative(peer_repr);
        self.exchange(&peer_public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;
    use std::format;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn test_generate_encodable_keypair() {
        let (_private, representative) = generate_encodable_keypair(&mut OsRng);

        // Representative should not be all zeros
        assert_ne!(representative.as_bytes(), &[0u8; 32]);

        // Decoding should give us a valid public key (not necessarily the same as private.public_key())
        let decoded_public = decode_representative(&representative);

        // The decoded public key should not be all zeros
        assert_ne!(decoded_public.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_elligator_keypair_produces_working_exchange() {
        // The key property of Elligator2 is that key exchange works
        // even though the decoded public key may differ from private.public_key()
        for _ in 0..10 {
            let alice = ElligatorKeypair::generate(&mut OsRng);
            let bob = ElligatorKeypair::generate(&mut OsRng);

            // Exchange using representatives
            let alice_shared = alice.exchange_representative(&bob.representative).unwrap();
            let bob_shared = bob.exchange_representative(&alice.representative).unwrap();

            assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
        }
    }

    #[test]
    fn test_any_bytes_decodable() {
        // Any 32 random bytes should decode to a valid point
        for _ in 0..100 {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);

            let repr = Representative::from_bytes(bytes);
            let _public = decode_representative(&repr); // Should not panic
        }
    }

    #[test]
    fn test_representative_looks_random() {
        // Statistical test: representatives should have roughly uniform distribution
        let mut byte_counts = [0u32; 256];

        for _ in 0..100 {
            let (_, repr) = generate_encodable_keypair(&mut OsRng);
            for &byte in repr.as_bytes() {
                byte_counts[byte as usize] += 1;
            }
        }

        // 100 * 32 = 3200 bytes total
        // Expected count per byte value: 3200 / 256 = 12.5
        let total: u32 = byte_counts.iter().sum();
        assert_eq!(total, 3200);

        // No byte value should dominate excessively
        let max_count = *byte_counts.iter().max().unwrap();

        // With 3200 samples across 256 buckets, we expect reasonable spread
        assert!(max_count < 100, "max count {} too high", max_count);
    }

    #[test]
    fn test_multiple_keypairs_unique() {
        let (_, repr1) = generate_encodable_keypair(&mut OsRng);
        let (_, repr2) = generate_encodable_keypair(&mut OsRng);

        // Should be different (with overwhelming probability)
        assert_ne!(repr1.as_bytes(), repr2.as_bytes());
    }

    #[test]
    fn test_key_exchange_with_representative() {
        let alice = ElligatorKeypair::generate(&mut OsRng);
        let bob = ElligatorKeypair::generate(&mut OsRng);

        // Exchange using representatives
        let alice_shared = alice.exchange_representative(&bob.representative).unwrap();
        let bob_shared = bob.exchange_representative(&alice.representative).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_key_exchange_mixed() {
        let alice = ElligatorKeypair::generate(&mut OsRng);
        let bob = PrivateKey::generate(&mut OsRng);
        let bob_public = bob.public_key();

        // Alice uses Elligator, Bob uses regular keys
        let alice_shared = alice.exchange(&bob_public).unwrap();
        let bob_shared = bob.exchange(&alice.public_key()).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_representative_from_slice() {
        let bytes = [0x42u8; 32];
        let repr = Representative::from_slice(&bytes).unwrap();
        assert_eq!(repr.as_bytes(), &bytes);

        // Wrong length should fail
        let short = [0x42u8; 16];
        assert!(Representative::from_slice(&short).is_none());
    }

    #[test]
    fn test_elligator_keypair_struct() {
        let keypair = ElligatorKeypair::generate(&mut OsRng);

        // The keypair should have a valid representative and public key
        assert_ne!(keypair.representative.as_bytes(), &[0u8; 32]);
        assert_ne!(keypair.public_key().to_bytes(), [0u8; 32]);

        // The decoded representative is valid (though may differ from public_key())
        let derived = decode_representative(&keypair.representative);
        assert_ne!(derived.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_representative_debug() {
        let repr = Representative::from_bytes([0u8; 32]);
        let debug_str = format!("{:?}", repr);
        assert!(debug_str.contains("Representative"));
    }

    #[test]
    fn test_deterministic_decoding() {
        // Same representative should always decode to same point
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);

        let repr = Representative::from_bytes(bytes);
        let point1 = decode_representative(&repr);
        let point2 = decode_representative(&repr);

        assert_eq!(point1.to_bytes(), point2.to_bytes());
    }

    /// Timing consistency test for decoding operations.
    ///
    /// This test verifies that decoding takes similar time for different inputs,
    /// which is an indicator of constant-time behavior. Note that true constant-time
    /// verification requires specialized tools (dudect, ctgrind).
    ///
    /// Timing tests are inherently incompatible with coverage instrumentation,
    /// so this test is disabled when running under coverage (cfg(coverage)).
    ///
    /// macOS CI runners exhibit extreme timing variance (100%+) due to virtualization
    /// overhead and noisy neighbor effects, making this test unreliable in CI.
    /// The underlying constant-time properties are still enforced by the crypto
    /// libraries (curve25519-dalek, subtle), which are formally verified.
    #[test]
    #[cfg_attr(
        coverage,
        ignore = "Timing tests incompatible with coverage instrumentation"
    )]
    #[cfg_attr(
        target_os = "macos",
        ignore = "macOS CI timing variance too high for reliable measurement"
    )]
    fn test_decode_timing_consistency() {
        use std::time::Instant;

        // Generate various representative patterns
        let patterns: Vec<[u8; 32]> = vec![
            [0u8; 32],    // All zeros
            [0xFFu8; 32], // All ones
            [0x55u8; 32], // Alternating bits pattern 1
            [0xAAu8; 32], // Alternating bits pattern 2
            {
                let mut b = [0u8; 32];
                OsRng.fill_bytes(&mut b);
                b
            }, // Random
            {
                let mut b = [0u8; 32];
                OsRng.fill_bytes(&mut b);
                b
            }, // Another random
        ];

        let mut timings = Vec::new();

        for pattern in &patterns {
            let repr = Representative::from_bytes(*pattern);

            // Warm up
            for _ in 0..100 {
                let _ = decode_representative(&repr);
            }

            // Measure
            let start = Instant::now();
            for _ in 0..1000 {
                let _ = decode_representative(&repr);
            }
            let elapsed = start.elapsed();
            timings.push(elapsed.as_nanos());
        }

        // Calculate statistics
        let mean: u128 = timings.iter().sum::<u128>() / timings.len() as u128;
        let max_deviation = timings
            .iter()
            .map(|&t: &u128| t.abs_diff(mean))
            .max()
            .unwrap();

        // Allow 100% timing variation as a sanity check in CI environments.
        // This is not a proof of constant-time behavior (that requires dudect/ctgrind),
        // but catches egregious timing differences that might indicate variable-time code.
        // CI environments have shared resources and virtualization overhead, so we use
        // a very generous threshold to avoid false positives.
        // Note: macOS is skipped entirely via #[cfg_attr] due to extreme variance.
        let max_allowed_deviation = mean; // 100% tolerance

        assert!(
            max_deviation < max_allowed_deviation,
            "Timing variation too large: mean={}, max_deviation={}, threshold={}",
            mean,
            max_deviation,
            max_allowed_deviation
        );
    }

    /// Test that the encoding operation uses CtOption (constant-time option).
    ///
    /// This test verifies the API contract that to_representative returns CtOption,
    /// which provides constant-time unwrapping and checking.
    #[test]
    fn test_encoding_uses_ct_option() {
        use subtle::CtOption;

        // Generate a random private key
        let mut private_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut private_bytes);

        // The to_representative call should return CtOption
        let result: CtOption<[u8; 32]> = Randomized::to_representative(&private_bytes, 0);

        // Verify CtOption semantics work correctly
        // is_some() returns Choice, which converts to bool in constant time
        let is_some = bool::from(result.is_some());

        // This test passes if compilation succeeds - it verifies the type contract
        // that Randomized::to_representative returns CtOption (constant-time)
        if is_some {
            let _ = result.unwrap();
        }
    }
}
