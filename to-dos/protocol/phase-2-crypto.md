# Phase 2: Cryptographic Layer Sprint Planning

**Duration:** Weeks 7-12 (4-6 weeks)
**Total Story Points:** 102
**Risk Level:** Medium (cryptographic correctness critical)

---

## Phase Overview

**Goal:** Implement the complete cryptographic layer including Noise_XX handshake protocol, AEAD encryption/decryption, key ratcheting, and all required primitives with constant-time operations and memory security.

### Success Criteria

- [ ] Handshake completes in <50ms (LAN environment)
- [ ] Encryption throughput >3 GB/s (single core, x86_64 AVX2)
- [ ] All operations are constant-time (verified with tools)
- [ ] Forward secrecy validated through ratcheting
- [ ] Test coverage >90% for all cryptographic code
- [ ] Zero critical vulnerabilities in security audit

### Dependencies

- Phase 1 complete (frame encoding, session states)
- `wraith-core` types defined
- Cryptographic libraries selected and audited

### Deliverables

1. X25519 Diffie-Hellman key exchange
2. Elligator2 encoding/decoding for key obfuscation
3. Noise_XX handshake protocol (3-message pattern)
4. XChaCha20-Poly1305 AEAD encryption
5. Symmetric ratchet (key derivation)
6. DH ratchet (forward secrecy)
7. BLAKE3 cryptographic hashing
8. Constant-time operation verification
9. Secure memory zeroization
10. Comprehensive cryptographic test vectors

---

## Sprint Breakdown

### Sprint 2.1: Cryptographic Primitives Setup (Weeks 7-8)

**Duration:** 2 weeks
**Story Points:** 18

#### Tasks

**2.1.1: X25519 Key Exchange Implementation** (8 SP)

Implement the X25519 elliptic curve Diffie-Hellman key exchange using `curve25519-dalek`.

```rust
// wraith-crypto/src/x25519.rs

use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::montgomery::MontgomeryPoint;
use zeroize::{Zeroize, ZeroizeOnDrop};
use rand_core::{RngCore, CryptoRng};

/// X25519 private key (32 bytes)
#[derive(Clone, ZeroizeOnDrop)]
pub struct PrivateKey([u8; 32]);

/// X25519 public key (32 bytes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey([u8; 32]);

/// X25519 shared secret (32 bytes)
#[derive(ZeroizeOnDrop)]
pub struct SharedSecret([u8; 32]);

impl PrivateKey {
    /// Generate a new random private key
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);

        // Clamp the scalar as per RFC 7748
        bytes[0] &= 248;
        bytes[31] &= 127;
        bytes[31] |= 64;

        Self(bytes)
    }

    /// Derive the public key from this private key
    pub fn public_key(&self) -> PublicKey {
        let scalar = Scalar::from_bytes_mod_order(self.0);
        let point = &scalar * &MontgomeryPoint::generator();
        PublicKey(point.to_bytes())
    }

    /// Perform Diffie-Hellman key exchange
    pub fn exchange(&self, peer_public: &PublicKey) -> Option<SharedSecret> {
        let scalar = Scalar::from_bytes_mod_order(self.0);
        let peer_point = MontgomeryPoint(peer_public.0);
        let shared_point = &scalar * &peer_point;

        // Check for low-order points (security critical)
        if shared_point.is_identity() {
            return None;
        }

        Some(SharedSecret(shared_point.to_bytes()))
    }

    /// Export as bytes (for serialization)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Import from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl PublicKey {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl SharedSecret {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_x25519_key_generation() {
        let private = PrivateKey::generate(&mut OsRng);
        let public = private.public_key();

        assert_ne!(public.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_x25519_key_exchange() {
        let alice_private = PrivateKey::generate(&mut OsRng);
        let alice_public = alice_private.public_key();

        let bob_private = PrivateKey::generate(&mut OsRng);
        let bob_public = bob_private.public_key();

        let alice_shared = alice_private.exchange(&bob_public).unwrap();
        let bob_shared = bob_private.exchange(&alice_public).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_reject_low_order_points() {
        let private = PrivateKey::generate(&mut OsRng);

        // Test with all-zero public key (low order)
        let zero_public = PublicKey([0u8; 32]);
        assert!(private.exchange(&zero_public).is_none());
    }

    // RFC 7748 test vectors
    #[test]
    fn test_rfc7748_vectors() {
        let scalar = [
            0xa5, 0x46, 0xe3, 0x6b, 0xf0, 0x52, 0x7c, 0x9d,
            0x3b, 0x16, 0x15, 0x4b, 0x82, 0x46, 0x5e, 0xdd,
            0x62, 0x14, 0x4c, 0x0a, 0xc1, 0xfc, 0x5a, 0x18,
            0x50, 0x6a, 0x22, 0x44, 0xba, 0x44, 0x9a, 0xc4,
        ];

        let basepoint = [
            0xe6, 0xdb, 0x68, 0x67, 0x58, 0x30, 0x30, 0xdb,
            0x35, 0x94, 0xc1, 0xa4, 0x24, 0xb1, 0x5f, 0x7c,
            0x72, 0x66, 0x24, 0xec, 0x26, 0xb3, 0x35, 0x3b,
            0x10, 0xa9, 0x03, 0xa6, 0xd0, 0xab, 0x1c, 0x4c,
        ];

        let expected = [
            0xc3, 0xda, 0x55, 0x37, 0x9d, 0xe9, 0xc6, 0x90,
            0x8e, 0x94, 0xea, 0x4d, 0xf2, 0x8d, 0x08, 0x4f,
            0x32, 0xec, 0xcf, 0x03, 0x49, 0x1c, 0x71, 0xf7,
            0x54, 0xb4, 0x07, 0x55, 0x77, 0xa2, 0x85, 0x52,
        ];

        let private = PrivateKey(scalar);
        let public = PublicKey(basepoint);
        let shared = private.exchange(&public).unwrap();

        assert_eq!(shared.as_bytes(), &expected);
    }
}
```

**Acceptance Criteria:**
- [ ] X25519 keypair generation works
- [ ] Diffie-Hellman exchange produces matching shared secrets
- [ ] Low-order points are rejected
- [ ] RFC 7748 test vectors pass
- [ ] Keys are zeroized on drop
- [ ] Constant-time operations verified

---

**2.1.2: BLAKE3 Hashing Integration** (5 SP)

Integrate BLAKE3 for cryptographic hashing and key derivation.

```rust
// wraith-crypto/src/hash.rs

use blake3::{Hash, Hasher, OutputReader};

/// BLAKE3 hash output (32 bytes)
pub type HashOutput = [u8; 32];

/// Compute BLAKE3 hash of input data
pub fn hash(data: &[u8]) -> HashOutput {
    blake3::hash(data).into()
}

/// BLAKE3 tree hasher for large files
pub struct TreeHasher {
    hasher: Hasher,
}

impl TreeHasher {
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(&self) -> HashOutput {
        self.hasher.finalize().into()
    }
}

/// BLAKE3 Key Derivation Function
pub struct Kdf {
    context: &'static str,
}

impl Kdf {
    pub fn new(context: &'static str) -> Self {
        Self { context }
    }

    /// Derive a key from input key material
    pub fn derive(&self, ikm: &[u8], output: &mut [u8]) {
        let mut hasher = Hasher::new_keyed(&hash(ikm));
        hasher.update(self.context.as_bytes());

        let mut reader = hasher.finalize_xof();
        reader.fill(output);
    }

    /// Derive a 32-byte key
    pub fn derive_key(&self, ikm: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        self.derive(ikm, &mut output);
        output
    }
}

/// HKDF-like extract and expand operations
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let mut hasher = if salt.is_empty() {
        Hasher::new()
    } else {
        Hasher::new_keyed(&hash(salt))
    };
    hasher.update(ikm);
    hasher.finalize().into()
}

pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], output: &mut [u8]) {
    let mut hasher = Hasher::new_keyed(prk);
    hasher.update(info);

    let mut reader = hasher.finalize_xof();
    reader.fill(output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_basic() {
        let data = b"hello world";
        let hash1 = hash(data);
        let hash2 = hash(data);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, [0u8; 32]);
    }

    #[test]
    fn test_kdf_deterministic() {
        let kdf = Kdf::new("wraith-test");
        let ikm = b"test input";

        let key1 = kdf.derive_key(ikm);
        let key2 = kdf.derive_key(ikm);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_kdf_different_contexts() {
        let kdf1 = Kdf::new("context-1");
        let kdf2 = Kdf::new("context-2");
        let ikm = b"same input";

        let key1 = kdf1.derive_key(ikm);
        let key2 = kdf2.derive_key(ikm);

        assert_ne!(key1, key2);
    }
}
```

**Acceptance Criteria:**
- [ ] BLAKE3 hashing works correctly
- [ ] KDF produces deterministic outputs
- [ ] Different contexts produce different keys
- [ ] Tree hashing supports incremental updates
- [ ] HKDF extract/expand operations work

---

**2.1.3: Constant-Time Operations Framework** (5 SP)

Set up framework for verifying constant-time operations in cryptographic code.

```rust
// wraith-crypto/src/constant_time.rs

use subtle::{Choice, ConstantTimeEq, ConditionallySelectable};

/// Constant-time comparison of byte slices
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.ct_eq(b).into()
}

/// Constant-time conditional copy
pub fn ct_select(condition: bool, a: &[u8], b: &[u8], out: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), out.len());

    let choice = Choice::from(condition as u8);

    for i in 0..out.len() {
        out[i] = u8::conditional_select(&b[i], &a[i], choice);
    }
}

/// Timing-safe array comparison
#[inline(never)]
pub fn verify_16(a: &[u8; 16], b: &[u8; 16]) -> bool {
    ct_eq(a, b)
}

#[inline(never)]
pub fn verify_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    ct_eq(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq_same() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        assert!(ct_eq(&a, &b));
    }

    #[test]
    fn test_ct_eq_different() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn test_ct_select() {
        let a = [1u8; 8];
        let b = [2u8; 8];
        let mut out = [0u8; 8];

        ct_select(true, &a, &b, &mut out);
        assert_eq!(out, a);

        ct_select(false, &a, &b, &mut out);
        assert_eq!(out, b);
    }
}
```

**Acceptance Criteria:**
- [ ] Constant-time equality works for all sizes
- [ ] Conditional selection is constant-time
- [ ] Test vectors pass
- [ ] No branching on secret data (code review)

---

### Sprint 2.2: Elligator2 Encoding (Weeks 8-9)

**Duration:** 1 week
**Story Points:** 13

**2.2.1: Elligator2 Implementation** (13 SP)

Implement Elligator2 encoding/decoding to make X25519 public keys indistinguishable from random.

```rust
// wraith-crypto/src/elligator2.rs

use curve25519_dalek::edwards::CompressedEdwardsY;
use curve25519_dalek::montgomery::MontgomeryPoint;
use rand_core::{RngCore, CryptoRng};

/// Elligator2 representative (32 bytes, looks random)
#[derive(Clone, Copy)]
pub struct Representative([u8; 32]);

/// Encode a Curve25519 point to an Elligator2 representative
/// Returns None if the point cannot be encoded (50% probability)
pub fn encode(point: &MontgomeryPoint) -> Option<Representative> {
    // Implementation based on "Elligator: Elliptic-curve points indistinguishable
    // from uniform random strings" by Bernstein, Hamburg, Krasnova, Lange

    // Convert Montgomery to Edwards
    let edwards = point_to_edwards(point)?;

    // Try to find a representative
    encode_edwards(&edwards)
}

/// Decode an Elligator2 representative to a Curve25519 point
pub fn decode(repr: &Representative) -> MontgomeryPoint {
    // This always succeeds - any 32-byte string is a valid representative
    let edwards = decode_to_edwards(repr);
    edwards_to_point(&edwards)
}

/// Generate a random representative and corresponding public key
pub fn generate_keypair<R: RngCore + CryptoRng>(
    rng: &mut R
) -> (crate::x25519::PrivateKey, Representative) {
    use crate::x25519::PrivateKey;

    loop {
        let private = PrivateKey::generate(rng);
        let public = private.public_key();
        let point = MontgomeryPoint(public.to_bytes());

        // Keep trying until we get an encodable point (50% chance each time)
        if let Some(repr) = encode(&point) {
            return (private, repr);
        }
    }
}

// Internal helpers
fn point_to_edwards(point: &MontgomeryPoint) -> Option<CompressedEdwardsY> {
    // Montgomery u -> Edwards (x, y) conversion
    // See RFC 7748 Section 4.1
    todo!("Implement Montgomery to Edwards conversion")
}

fn edwards_to_point(edwards: &CompressedEdwardsY) -> MontgomeryPoint {
    todo!("Implement Edwards to Montgomery conversion")
}

fn encode_edwards(edwards: &CompressedEdwardsY) -> Option<Representative> {
    todo!("Implement Elligator2 encoding")
}

fn decode_to_edwards(repr: &Representative) -> CompressedEdwardsY {
    todo!("Implement Elligator2 decoding")
}

impl Representative {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Check if bytes look uniformly random (statistical test)
    pub fn is_uniform_random(bytes: &[u8; 32]) -> bool {
        // Simple chi-squared test for uniformity
        // In production, use more sophisticated tests
        let mut buckets = [0u32; 256];
        for &byte in bytes.iter() {
            buckets[byte as usize] += 1;
        }

        let expected = bytes.len() as f64 / 256.0;
        let chi_squared: f64 = buckets.iter()
            .map(|&count| {
                let diff = count as f64 - expected;
                (diff * diff) / expected
            })
            .sum();

        // Very loose threshold for 32 bytes (should be ~255 for uniform)
        chi_squared < 400.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_elligator2_encode_decode() {
        let (private, repr) = generate_keypair(&mut OsRng);
        let expected_public = private.public_key();

        let decoded_point = decode(&repr);
        let decoded_public = crate::x25519::PublicKey::from_bytes(decoded_point.to_bytes());

        assert_eq!(decoded_public, expected_public);
    }

    #[test]
    fn test_representative_looks_random() {
        let (_private, repr) = generate_keypair(&mut OsRng);
        let bytes = repr.to_bytes();

        // Should not be all zeros
        assert_ne!(bytes, [0u8; 32]);

        // Should pass basic randomness test
        assert!(Representative::is_uniform_random(&bytes));
    }

    #[test]
    fn test_any_bytes_decodable() {
        // Any 32 random bytes should decode to a valid point
        let mut rng = OsRng;
        for _ in 0..100 {
            let mut bytes = [0u8; 32];
            rng.fill_bytes(&mut bytes);

            let repr = Representative::from_bytes(bytes);
            let _point = decode(&repr); // Should not panic
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Elligator2 encode/decode round-trips correctly
- [ ] Representatives look statistically random
- [ ] Any 32 bytes can be decoded
- [ ] Keypair generation finds encodable points
- [ ] Implementation matches Elligator2 spec

---

### Sprint 2.3: Noise_XX Handshake (Weeks 9-10)

**Duration:** 2 weeks
**Story Points:** 26

**2.3.1: Noise Protocol Framework** (8 SP)

Implement the Noise protocol framework (state machine, message patterns).

```rust
// wraith-crypto/src/noise/mod.rs

pub mod handshake;
pub mod state;
pub mod patterns;

use crate::x25519::{PrivateKey, PublicKey, SharedSecret};
use crate::hash::{hash, hkdf_extract, hkdf_expand};
use zeroize::Zeroize;

/// Noise protocol state
pub struct NoiseState {
    /// Symmetric state for encryption
    symmetric: SymmetricState,
    /// Local static keypair
    s: Option<PrivateKey>,
    /// Local ephemeral keypair
    e: Option<PrivateKey>,
    /// Remote static public key
    rs: Option<PublicKey>,
    /// Remote ephemeral public key
    re: Option<PublicKey>,
    /// Handshake hash
    h: [u8; 32],
    /// Chaining key
    ck: [u8; 32],
    /// Role (initiator or responder)
    role: Role,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Initiator,
    Responder,
}

struct SymmetricState {
    /// Encryption key (None during preamble)
    k: Option<[u8; 32]>,
}

impl NoiseState {
    /// Initialize Noise_XX protocol
    pub fn new_xx(role: Role, static_key: PrivateKey, prologue: &[u8]) -> Self {
        const PROTOCOL_NAME: &[u8] = b"Noise_XX_25519_ChaChaPoly_BLAKE3";

        let mut h = [0u8; 32];
        if PROTOCOL_NAME.len() <= 32 {
            h[..PROTOCOL_NAME.len()].copy_from_slice(PROTOCOL_NAME);
        } else {
            h = hash(PROTOCOL_NAME);
        }

        let ck = h;

        // Mix prologue into handshake hash
        let h = if prologue.is_empty() {
            h
        } else {
            let mut data = h.to_vec();
            data.extend_from_slice(prologue);
            hash(&data)
        };

        Self {
            symmetric: SymmetricState { k: None },
            s: Some(static_key),
            e: None,
            rs: None,
            re: None,
            h,
            ck,
            role,
        }
    }

    /// Mix key material into chaining key
    fn mix_key(&mut self, input_key_material: &[u8]) {
        let temp_k = hkdf_extract(&self.ck, input_key_material);
        self.ck = temp_k;

        let mut k = [0u8; 32];
        hkdf_expand(&temp_k, b"", &mut k);
        self.symmetric.k = Some(k);
    }

    /// Mix data into handshake hash
    fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.h.to_vec();
        input.extend_from_slice(data);
        self.h = hash(&input);
    }

    /// Encrypt and authenticate plaintext
    fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Vec<u8> {
        if let Some(k) = &self.symmetric.k {
            // Use XChaCha20-Poly1305 AEAD (implemented in next sprint)
            let ciphertext = self.encrypt_with_ad(&self.h, plaintext, k);
            self.mix_hash(&ciphertext);
            ciphertext
        } else {
            // No key yet, just hash
            self.mix_hash(plaintext);
            plaintext.to_vec()
        }
    }

    /// Decrypt and verify ciphertext
    fn decrypt_and_hash(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if let Some(k) = &self.symmetric.k {
            let plaintext = self.decrypt_with_ad(&self.h, ciphertext, k)?;
            self.mix_hash(ciphertext);
            Ok(plaintext)
        } else {
            self.mix_hash(ciphertext);
            Ok(ciphertext.to_vec())
        }
    }

    // Placeholder for AEAD (implemented in Sprint 2.4)
    fn encrypt_with_ad(&self, ad: &[u8], plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
        todo!("Implement XChaCha20-Poly1305 encryption")
    }

    fn decrypt_with_ad(&self, ad: &[u8], ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, NoiseError> {
        todo!("Implement XChaCha20-Poly1305 decryption")
    }
}

#[derive(Debug)]
pub enum NoiseError {
    InvalidMessage,
    DecryptionFailed,
    InvalidState,
}

impl Drop for NoiseState {
    fn drop(&mut self) {
        self.h.zeroize();
        self.ck.zeroize();
    }
}
```

**Acceptance Criteria:**
- [ ] Noise state machine implemented
- [ ] mix_key and mix_hash operations work
- [ ] Protocol name hashing correct
- [ ] Prologue mixing works
- [ ] State is zeroized on drop

---

**2.3.2: Noise_XX Message 1 (Initiator → Responder)** (6 SP)

```rust
// wraith-crypto/src/noise/handshake.rs

use super::{NoiseState, NoiseError, Role};
use crate::x25519::PrivateKey;
use rand_core::{RngCore, CryptoRng};

impl NoiseState {
    /// Generate Message 1: → e
    /// Initiator sends ephemeral public key
    pub fn write_message_1<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        payload: &[u8],
    ) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Initiator {
            return Err(NoiseError::InvalidState);
        }

        // Generate ephemeral keypair
        let e = PrivateKey::generate(rng);
        let e_pub = e.public_key();
        self.e = Some(e);

        // Mix ephemeral public key into hash
        self.mix_hash(&e_pub.to_bytes());

        // Build message: e || encrypted(payload)
        let mut message = e_pub.to_bytes().to_vec();
        message.extend_from_slice(&self.encrypt_and_hash(payload));

        Ok(message)
    }

    /// Read Message 1: → e
    /// Responder receives ephemeral public key
    pub fn read_message_1(&mut self, message: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Responder {
            return Err(NoiseError::InvalidState);
        }

        if message.len() < 32 {
            return Err(NoiseError::InvalidMessage);
        }

        // Extract ephemeral public key
        let mut re_bytes = [0u8; 32];
        re_bytes.copy_from_slice(&message[..32]);
        let re = crate::x25519::PublicKey::from_bytes(re_bytes);
        self.re = Some(re);

        // Mix into hash
        self.mix_hash(&re_bytes);

        // Decrypt payload
        let payload = self.decrypt_and_hash(&message[32..])?;

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_message_1_round_trip() {
        let initiator_static = PrivateKey::generate(&mut OsRng);
        let responder_static = PrivateKey::generate(&mut OsRng);

        let mut initiator = NoiseState::new_xx(Role::Initiator, initiator_static, b"");
        let mut responder = NoiseState::new_xx(Role::Responder, responder_static, b"");

        let payload = b"hello";
        let message = initiator.write_message_1(&mut OsRng, payload).unwrap();
        let received = responder.read_message_1(&message).unwrap();

        assert_eq!(received, payload);
    }
}
```

**Acceptance Criteria:**
- [ ] Message 1 generation works
- [ ] Message 1 parsing works
- [ ] Ephemeral keys are stored correctly
- [ ] Handshake hash updated properly
- [ ] Round-trip test passes

---

**2.3.3: Noise_XX Messages 2 and 3** (12 SP)

```rust
// Continued in wraith-crypto/src/noise/handshake.rs

impl NoiseState {
    /// Generate Message 2: ← e, ee, s, es
    /// Responder sends ephemeral, performs DH, sends static
    pub fn write_message_2<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        payload: &[u8],
    ) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Responder {
            return Err(NoiseError::InvalidState);
        }

        let re = self.re.as_ref().ok_or(NoiseError::InvalidState)?;
        let s = self.s.as_ref().ok_or(NoiseError::InvalidState)?;

        // Generate ephemeral keypair
        let e = PrivateKey::generate(rng);
        let e_pub = e.public_key();

        // Mix e into hash
        self.mix_hash(&e_pub.to_bytes());

        // Perform DH(e, re) -> ee
        let ee = e.exchange(re).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(ee.as_bytes());

        // Get static public key
        let s_pub = s.public_key();

        // Encrypt and send static key
        let encrypted_s = self.encrypt_and_hash(&s_pub.to_bytes());

        // Perform DH(s, re) -> es
        let es = s.exchange(re).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(es.as_bytes());

        self.e = Some(e);

        // Build message: e || encrypted(s) || encrypted(payload)
        let mut message = e_pub.to_bytes().to_vec();
        message.extend_from_slice(&encrypted_s);
        message.extend_from_slice(&self.encrypt_and_hash(payload));

        Ok(message)
    }

    /// Read Message 2: ← e, ee, s, es
    pub fn read_message_2(&mut self, message: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Initiator {
            return Err(NoiseError::InvalidState);
        }

        if message.len() < 32 {
            return Err(NoiseError::InvalidMessage);
        }

        let e = self.e.as_ref().ok_or(NoiseError::InvalidState)?;

        // Extract ephemeral public key
        let mut re_bytes = [0u8; 32];
        re_bytes.copy_from_slice(&message[..32]);
        let re = crate::x25519::PublicKey::from_bytes(re_bytes);

        self.mix_hash(&re_bytes);

        // Perform DH(e, re) -> ee
        let ee = e.exchange(&re).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(ee.as_bytes());

        // Decrypt static key (encrypted, so 32 + 16 = 48 bytes)
        let encrypted_rs = &message[32..80]; // 32 bytes key + 16 bytes tag
        let rs_bytes = self.decrypt_and_hash(encrypted_rs)?;

        let mut rs_array = [0u8; 32];
        rs_array.copy_from_slice(&rs_bytes);
        let rs = crate::x25519::PublicKey::from_bytes(rs_array);

        // Perform DH(e, rs) -> es
        let es = e.exchange(&rs).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(es.as_bytes());

        self.re = Some(re);
        self.rs = Some(rs);

        // Decrypt payload
        let payload = self.decrypt_and_hash(&message[80..])?;

        Ok(payload)
    }

    /// Generate Message 3: → s, se
    /// Initiator sends static key, performs final DH
    pub fn write_message_3(&mut self, payload: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Initiator {
            return Err(NoiseError::InvalidState);
        }

        let s = self.s.as_ref().ok_or(NoiseError::InvalidState)?;
        let re = self.re.as_ref().ok_or(NoiseError::InvalidState)?;

        // Encrypt and send static key
        let s_pub = s.public_key();
        let encrypted_s = self.encrypt_and_hash(&s_pub.to_bytes());

        // Perform DH(s, re) -> se
        let se = s.exchange(re).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(se.as_bytes());

        // Build message: encrypted(s) || encrypted(payload)
        let mut message = encrypted_s;
        message.extend_from_slice(&self.encrypt_and_hash(payload));

        Ok(message)
    }

    /// Read Message 3: → s, se
    pub fn read_message_3(&mut self, message: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if self.role != Role::Responder {
            return Err(NoiseError::InvalidState);
        }

        let s = self.s.as_ref().ok_or(NoiseError::InvalidState)?;
        let e = self.e.as_ref().ok_or(NoiseError::InvalidState)?;

        if message.len() < 48 {
            return Err(NoiseError::InvalidMessage);
        }

        // Decrypt static key
        let encrypted_rs = &message[..48];
        let rs_bytes = self.decrypt_and_hash(encrypted_rs)?;

        let mut rs_array = [0u8; 32];
        rs_array.copy_from_slice(&rs_bytes);
        let rs = crate::x25519::PublicKey::from_bytes(rs_array);

        // Perform DH(e, rs) -> se
        let se = e.exchange(&rs).ok_or(NoiseError::InvalidMessage)?;
        self.mix_key(se.as_bytes());

        self.rs = Some(rs);

        // Decrypt payload
        let payload = self.decrypt_and_hash(&message[48..])?;

        Ok(payload)
    }

    /// Split into transport keys after handshake complete
    pub fn split(mut self) -> (TransportKeys, TransportKeys) {
        let mut temp_k1 = [0u8; 32];
        let mut temp_k2 = [0u8; 32];

        hkdf_expand(&self.ck, b"", &mut temp_k1);
        hkdf_expand(&temp_k1, b"", &mut temp_k2);

        let (send_key, recv_key) = match self.role {
            Role::Initiator => (temp_k1, temp_k2),
            Role::Responder => (temp_k2, temp_k1),
        };

        (
            TransportKeys { key: send_key, nonce: 0 },
            TransportKeys { key: recv_key, nonce: 0 },
        )
    }
}

/// Transport encryption keys after handshake
pub struct TransportKeys {
    pub key: [u8; 32],
    pub nonce: u64,
}

impl Drop for TransportKeys {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}
```

**Acceptance Criteria:**
- [ ] All three messages can be sent/received
- [ ] DH operations performed correctly (ee, es, se)
- [ ] Static keys encrypted properly
- [ ] Handshake completes successfully
- [ ] Transport keys derived correctly
- [ ] Full handshake integration test passes

---

### Sprint 2.4: AEAD Encryption (Weeks 10-11)

**Duration:** 1.5 weeks
**Story Points:** 21

**2.4.1: XChaCha20-Poly1305 Implementation** (13 SP)

```rust
// wraith-crypto/src/aead.rs

use chacha20poly1305::{
    XChaCha20Poly1305, KeyInit, AeadInPlace,
    aead::{Payload, Error as AeadError},
};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// AEAD encryption key (32 bytes)
#[derive(Clone, ZeroizeOnDrop)]
pub struct AeadKey([u8; 32]);

/// AEAD nonce for XChaCha20-Poly1305 (24 bytes)
pub type Nonce = [u8; 24];

/// Authentication tag (16 bytes)
pub type Tag = [u8; 16];

impl AeadKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Encrypt plaintext with associated data
    /// Returns ciphertext || tag (plaintext.len() + 16 bytes)
    pub fn encrypt(
        &self,
        nonce: &Nonce,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, AeadError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        let mut buffer = plaintext.to_vec();

        let payload = Payload {
            msg: &buffer,
            aad: associated_data,
        };

        let tag = cipher.encrypt_in_place_detached(nonce.into(), associated_data, &mut buffer)?;

        buffer.extend_from_slice(&tag);
        Ok(buffer)
    }

    /// Decrypt ciphertext with associated data
    /// Input must be ciphertext || tag
    pub fn decrypt(
        &self,
        nonce: &Nonce,
        ciphertext_and_tag: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, AeadError> {
        if ciphertext_and_tag.len() < 16 {
            return Err(AeadError);
        }

        let cipher = XChaCha20Poly1305::new((&self.0).into());

        let (ciphertext, tag) = ciphertext_and_tag.split_at(ciphertext_and_tag.len() - 16);
        let mut buffer = ciphertext.to_vec();

        cipher.decrypt_in_place_detached(
            nonce.into(),
            associated_data,
            &mut buffer,
            tag.into(),
        )?;

        Ok(buffer)
    }

    /// In-place encryption (modifies buffer)
    pub fn encrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut Vec<u8>,
        associated_data: &[u8],
    ) -> Result<Tag, AeadError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        let tag = cipher.encrypt_in_place_detached(nonce.into(), associated_data, buffer)?;

        Ok(tag.into())
    }

    /// In-place decryption (modifies buffer)
    pub fn decrypt_in_place(
        &self,
        nonce: &Nonce,
        buffer: &mut Vec<u8>,
        tag: &Tag,
        associated_data: &[u8],
    ) -> Result<(), AeadError> {
        let cipher = XChaCha20Poly1305::new((&self.0).into());

        cipher.decrypt_in_place_detached(
            nonce.into(),
            associated_data,
            buffer,
            tag.into(),
        )?;

        Ok(())
    }
}

/// Generate a random nonce
pub fn generate_nonce<R: rand_core::RngCore>(rng: &mut R) -> Nonce {
    let mut nonce = [0u8; 24];
    rng.fill_bytes(&mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_aead_encrypt_decrypt() {
        let key = AeadKey::new([1u8; 32]);
        let nonce = generate_nonce(&mut OsRng);
        let plaintext = b"hello world";
        let aad = b"additional data";

        let ciphertext = key.encrypt(&nonce, plaintext, aad).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + 16);

        let decrypted = key.decrypt(&nonce, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_wrong_key_fails() {
        let key1 = AeadKey::new([1u8; 32]);
        let key2 = AeadKey::new([2u8; 32]);
        let nonce = generate_nonce(&mut OsRng);

        let ciphertext = key1.encrypt(&nonce, b"secret", b"").unwrap();
        assert!(key2.decrypt(&nonce, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_nonce_fails() {
        let key = AeadKey::new([1u8; 32]);
        let nonce1 = generate_nonce(&mut OsRng);
        let nonce2 = generate_nonce(&mut OsRng);

        let ciphertext = key.encrypt(&nonce1, b"secret", b"").unwrap();
        assert!(key.decrypt(&nonce2, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_wrong_aad_fails() {
        let key = AeadKey::new([1u8; 32]);
        let nonce = generate_nonce(&mut OsRng);

        let ciphertext = key.encrypt(&nonce, b"secret", b"aad1").unwrap();
        assert!(key.decrypt(&nonce, &ciphertext, b"aad2").is_err());
    }

    #[test]
    fn test_aead_tampering_detected() {
        let key = AeadKey::new([1u8; 32]);
        let nonce = generate_nonce(&mut OsRng);

        let mut ciphertext = key.encrypt(&nonce, b"secret", b"").unwrap();

        // Tamper with ciphertext
        ciphertext[5] ^= 0xFF;

        assert!(key.decrypt(&nonce, &ciphertext, b"").is_err());
    }

    #[test]
    fn test_aead_in_place() {
        let key = AeadKey::new([1u8; 32]);
        let nonce = generate_nonce(&mut OsRng);
        let mut buffer = b"hello world".to_vec();
        let original = buffer.clone();

        let tag = key.encrypt_in_place(&nonce, &mut buffer, b"").unwrap();
        assert_ne!(buffer, original);

        key.decrypt_in_place(&nonce, &mut buffer, &tag, b"").unwrap();
        assert_eq!(buffer, original);
    }

    // RFC 8439 test vector
    #[test]
    fn test_rfc8439_vector() {
        // Test with known vector from RFC
        // (Simplified - full implementation would include actual RFC vectors)
        let key = AeadKey::new([0x80; 32]);
        let nonce = [0u8; 24];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let ciphertext = key.encrypt(&nonce, plaintext, b"").unwrap();
        let decrypted = key.decrypt(&nonce, &ciphertext, b"").unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
```

**Acceptance Criteria:**
- [ ] XChaCha20-Poly1305 encryption works
- [ ] Decryption with correct key/nonce succeeds
- [ ] Wrong key/nonce/AAD causes decryption failure
- [ ] Tampering is detected
- [ ] In-place operations work
- [ ] RFC 8439 test vectors pass
- [ ] Performance: >3 GB/s on AVX2 hardware

---

**2.4.2: Integrate AEAD into Noise Protocol** (8 SP)

Update Noise implementation to use real XChaCha20-Poly1305.

```rust
// wraith-crypto/src/noise/mod.rs (updates)

use crate::aead::{AeadKey, generate_nonce};

impl NoiseState {
    fn encrypt_with_ad(&self, ad: &[u8], plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
        let aead_key = AeadKey::new(*key);

        // For Noise, nonce is always 0 during handshake
        let nonce = [0u8; 24];

        aead_key.encrypt(&nonce, plaintext, ad)
            .expect("AEAD encryption failed")
    }

    fn decrypt_with_ad(
        &self,
        ad: &[u8],
        ciphertext: &[u8],
        key: &[u8; 32]
    ) -> Result<Vec<u8>, NoiseError> {
        let aead_key = AeadKey::new(*key);
        let nonce = [0u8; 24];

        aead_key.decrypt(&nonce, ciphertext, ad)
            .map_err(|_| NoiseError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_full_noise_xx_handshake() {
        let initiator_static = PrivateKey::generate(&mut OsRng);
        let responder_static = PrivateKey::generate(&mut OsRng);

        let mut initiator = NoiseState::new_xx(
            Role::Initiator,
            initiator_static.clone(),
            b"wraith-v1"
        );
        let mut responder = NoiseState::new_xx(
            Role::Responder,
            responder_static.clone(),
            b"wraith-v1"
        );

        // Message 1: Initiator → Responder
        let msg1 = initiator.write_message_1(&mut OsRng, b"init").unwrap();
        let payload1 = responder.read_message_1(&msg1).unwrap();
        assert_eq!(payload1, b"init");

        // Message 2: Responder → Initiator
        let msg2 = responder.write_message_2(&mut OsRng, b"resp").unwrap();
        let payload2 = initiator.read_message_2(&msg2).unwrap();
        assert_eq!(payload2, b"resp");

        // Message 3: Initiator → Responder
        let msg3 = initiator.write_message_3(b"final").unwrap();
        let payload3 = responder.read_message_3(&msg3).unwrap();
        assert_eq!(payload3, b"final");

        // Split into transport keys
        let (init_send, init_recv) = initiator.split();
        let (resp_send, resp_recv) = responder.split();

        // Verify keys match
        assert_eq!(init_send.key, resp_recv.key);
        assert_eq!(init_recv.key, resp_send.key);
    }

    #[test]
    fn test_noise_handshake_mutual_authentication() {
        let initiator_static = PrivateKey::generate(&mut OsRng);
        let initiator_public = initiator_static.public_key();

        let responder_static = PrivateKey::generate(&mut OsRng);
        let responder_public = responder_static.public_key();

        let mut initiator = NoiseState::new_xx(Role::Initiator, initiator_static, b"");
        let mut responder = NoiseState::new_xx(Role::Responder, responder_static, b"");

        // Full handshake
        let msg1 = initiator.write_message_1(&mut OsRng, b"").unwrap();
        responder.read_message_1(&msg1).unwrap();

        let msg2 = responder.write_message_2(&mut OsRng, b"").unwrap();
        initiator.read_message_2(&msg2).unwrap();

        let msg3 = initiator.write_message_3(b"").unwrap();
        responder.read_message_3(&msg3).unwrap();

        // Verify both sides know each other's static keys
        assert_eq!(initiator.rs.unwrap(), responder_public);
        assert_eq!(responder.rs.unwrap(), initiator_public);
    }
}
```

**Acceptance Criteria:**
- [ ] Noise handshake uses real AEAD encryption
- [ ] Full 3-message handshake completes
- [ ] Transport keys derived correctly
- [ ] Mutual authentication verified
- [ ] Handshake completes in <50ms (LAN)

---

### Sprint 2.5: Key Ratcheting (Week 11)

**Duration:** 1 week
**Story Points:** 16

**2.5.1: Symmetric Ratchet** (8 SP)

```rust
// wraith-crypto/src/ratchet.rs

use crate::hash::{hkdf_extract, hkdf_expand};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Symmetric key ratchet for forward secrecy
#[derive(ZeroizeOnDrop)]
pub struct SymmetricRatchet {
    /// Current chain key
    chain_key: [u8; 32],
    /// Message counter
    counter: u64,
}

impl SymmetricRatchet {
    /// Initialize from root key
    pub fn new(root_key: &[u8; 32]) -> Self {
        Self {
            chain_key: *root_key,
            counter: 0,
        }
    }

    /// Derive next message key and advance ratchet
    pub fn next_key(&mut self) -> [u8; 32] {
        // Derive message key and next chain key
        let mut message_key = [0u8; 32];
        let mut next_chain_key = [0u8; 32];

        hkdf_expand(&self.chain_key, b"message", &mut message_key);
        hkdf_expand(&self.chain_key, b"chain", &mut next_chain_key);

        // Update state
        self.chain_key.zeroize();
        self.chain_key = next_chain_key;
        self.counter += 1;

        message_key
    }

    /// Get current counter value
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Jump to specific counter (for out-of-order messages)
    pub fn skip_to(&mut self, target: u64) -> Vec<[u8; 32]> {
        let mut skipped_keys = Vec::new();

        while self.counter < target {
            skipped_keys.push(self.next_key());
        }

        skipped_keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetric_ratchet_deterministic() {
        let root = [1u8; 32];
        let mut ratchet1 = SymmetricRatchet::new(&root);
        let mut ratchet2 = SymmetricRatchet::new(&root);

        let key1a = ratchet1.next_key();
        let key1b = ratchet2.next_key();
        assert_eq!(key1a, key1b);

        let key2a = ratchet1.next_key();
        let key2b = ratchet2.next_key();
        assert_eq!(key2a, key2b);
    }

    #[test]
    fn test_symmetric_ratchet_unique_keys() {
        let root = [1u8; 32];
        let mut ratchet = SymmetricRatchet::new(&root);

        let key1 = ratchet.next_key();
        let key2 = ratchet.next_key();
        let key3 = ratchet.next_key();

        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_symmetric_ratchet_skip() {
        let root = [1u8; 32];
        let mut ratchet1 = SymmetricRatchet::new(&root);
        let mut ratchet2 = SymmetricRatchet::new(&root);

        // Ratchet1 advances normally
        let _key1 = ratchet1.next_key();
        let _key2 = ratchet1.next_key();
        let key3 = ratchet1.next_key();

        // Ratchet2 skips ahead
        let skipped = ratchet2.skip_to(3);
        assert_eq!(skipped.len(), 3);

        // Both should produce same next key
        let key4a = ratchet1.next_key();
        let key4b = ratchet2.next_key();
        assert_eq!(key4a, key4b);
    }
}
```

**Acceptance Criteria:**
- [ ] Symmetric ratchet produces deterministic keys
- [ ] Each key is unique
- [ ] Counter increments correctly
- [ ] Skip functionality works for out-of-order messages
- [ ] Keys are zeroized on drop

---

**2.5.2: DH Ratchet** (8 SP)

```rust
// wraith-crypto/src/ratchet.rs (continued)

use crate::x25519::{PrivateKey, PublicKey};
use rand_core::{RngCore, CryptoRng};

/// Double Ratchet combining DH and symmetric ratchets
#[derive(ZeroizeOnDrop)]
pub struct DHRatchet {
    /// Our current DH private key
    dh_self: PrivateKey,
    /// Peer's current DH public key
    dh_peer: Option<PublicKey>,
    /// Root chain key
    root_key: [u8; 32],
    /// Sending chain ratchet
    send_ratchet: SymmetricRatchet,
    /// Receiving chain ratchet
    recv_ratchet: Option<SymmetricRatchet>,
    /// Number of messages in current sending chain
    send_chain_length: u32,
    /// Number of messages in current receiving chain
    recv_chain_length: u32,
}

impl DHRatchet {
    /// Initialize as initiator (knows peer's initial key)
    pub fn new_initiator<R: RngCore + CryptoRng>(
        rng: &mut R,
        shared_secret: &[u8; 32],
        peer_public: PublicKey,
    ) -> Self {
        let dh_self = PrivateKey::generate(rng);
        let dh_pair = dh_self.public_key();

        // Initial DH ratchet step
        let dh_out = dh_self.exchange(&peer_public).unwrap();

        let mut root_key = [0u8; 32];
        let mut chain_key = [0u8; 32];

        kdf_rk(shared_secret, dh_out.as_bytes(), &mut root_key, &mut chain_key);

        Self {
            dh_self,
            dh_peer: Some(peer_public),
            root_key,
            send_ratchet: SymmetricRatchet::new(&chain_key),
            recv_ratchet: None,
            send_chain_length: 0,
            recv_chain_length: 0,
        }
    }

    /// Initialize as responder (will receive peer's key first)
    pub fn new_responder<R: RngCore + CryptoRng>(
        rng: &mut R,
        shared_secret: &[u8; 32],
    ) -> Self {
        let dh_self = PrivateKey::generate(rng);

        Self {
            dh_self,
            dh_peer: None,
            root_key: *shared_secret,
            send_ratchet: SymmetricRatchet::new(shared_secret),
            recv_ratchet: None,
            send_chain_length: 0,
            recv_chain_length: 0,
        }
    }

    /// Encrypt message and get current DH public key
    pub fn encrypt(&mut self, plaintext: &[u8]) -> (PublicKey, Vec<u8>) {
        let message_key = self.send_ratchet.next_key();
        self.send_chain_length += 1;

        // TODO: Actual encryption with message_key
        let ciphertext = plaintext.to_vec();

        (self.dh_self.public_key(), ciphertext)
    }

    /// Decrypt message and update ratchet
    pub fn decrypt<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        peer_dh_public: &PublicKey,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, RatchetError> {
        // Check if we need to perform DH ratchet step
        if self.dh_peer.as_ref() != Some(peer_dh_public) {
            self.dh_ratchet_step(rng, peer_dh_public)?;
        }

        // Decrypt with receiving chain
        let message_key = self.recv_ratchet
            .as_mut()
            .ok_or(RatchetError::NoReceivingChain)?
            .next_key();

        self.recv_chain_length += 1;

        // TODO: Actual decryption with message_key
        Ok(ciphertext.to_vec())
    }

    /// Perform DH ratchet step (new DH exchange)
    fn dh_ratchet_step<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        peer_public: &PublicKey,
    ) -> Result<(), RatchetError> {
        // Store peer's new public key
        self.dh_peer = Some(*peer_public);
        self.recv_chain_length = 0;

        // Create receiving chain
        let dh_recv = self.dh_self.exchange(peer_public)
            .ok_or(RatchetError::InvalidPublicKey)?;

        let mut new_root_key = [0u8; 32];
        let mut recv_chain_key = [0u8; 32];

        kdf_rk(&self.root_key, dh_recv.as_bytes(), &mut new_root_key, &mut recv_chain_key);

        self.root_key = new_root_key;
        self.recv_ratchet = Some(SymmetricRatchet::new(&recv_chain_key));

        // Generate new DH keypair
        let new_dh_self = PrivateKey::generate(rng);

        // Create sending chain
        let dh_send = new_dh_self.exchange(peer_public)
            .ok_or(RatchetError::InvalidPublicKey)?;

        let mut send_chain_key = [0u8; 32];

        kdf_rk(&self.root_key, dh_send.as_bytes(), &mut new_root_key, &mut send_chain_key);

        self.root_key = new_root_key;
        self.dh_self = new_dh_self;
        self.send_ratchet = SymmetricRatchet::new(&send_chain_key);
        self.send_chain_length = 0;

        Ok(())
    }
}

/// KDF for root key derivation
fn kdf_rk(root_key: &[u8; 32], dh_out: &[u8; 32], new_root: &mut [u8; 32], chain_key: &mut [u8; 32]) {
    let temp = hkdf_extract(root_key, dh_out);
    hkdf_expand(&temp, b"root", new_root);
    hkdf_expand(&temp, b"chain", chain_key);
}

#[derive(Debug)]
pub enum RatchetError {
    InvalidPublicKey,
    NoReceivingChain,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn test_dh_ratchet_basic() {
        let shared_secret = [1u8; 32];

        let alice_dh = PrivateKey::generate(&mut OsRng);
        let alice_pub = alice_dh.public_key();

        let bob_dh = PrivateKey::generate(&mut OsRng);
        let bob_pub = bob_dh.public_key();

        let mut alice = DHRatchet::new_initiator(&mut OsRng, &shared_secret, bob_pub);
        let mut bob = DHRatchet::new_responder(&mut OsRng, &shared_secret);

        // Alice sends to Bob
        let (alice_dh_pub, ciphertext) = alice.encrypt(b"hello");
        let plaintext = bob.decrypt(&mut OsRng, &alice_dh_pub, &ciphertext).unwrap();
        assert_eq!(plaintext, b"hello");
    }
}
```

**Acceptance Criteria:**
- [ ] DH ratchet performs key exchanges
- [ ] Symmetric chains created correctly
- [ ] Forward secrecy maintained (old keys deleted)
- [ ] Round-trip encryption/decryption works
- [ ] Integration test with full handshake

---

### Sprint 2.6: Testing & Documentation (Week 12)

**Duration:** 1 week
**Story Points:** 8

**2.6.1: Cryptographic Test Vectors** (3 SP)

Create comprehensive test vectors for all cryptographic operations.

```rust
// wraith-crypto/tests/vectors.rs

/// Test vectors from RFC 7748 (X25519)
#[test]
fn test_x25519_rfc7748_vectors() {
    // Vector 1
    let scalar1 = hex::decode("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4").unwrap();
    let point1 = hex::decode("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c").unwrap();
    let expected1 = hex::decode("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552").unwrap();

    // ... implement vector tests
}

/// Test vectors for BLAKE3
#[test]
fn test_blake3_vectors() {
    // Empty string
    assert_eq!(
        hex::encode(blake3::hash(b"")),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );

    // ... more vectors
}

/// Noise protocol test vectors
#[test]
fn test_noise_xx_vectors() {
    // Implement Noise test vectors from noise-protocol repository
}
```

**Acceptance Criteria:**
- [ ] All RFC test vectors pass
- [ ] Noise protocol test vectors implemented
- [ ] Custom WRAITH-specific test vectors
- [ ] Edge case coverage (all zeros, all ones, etc.)

---

**2.6.2: Performance Benchmarks** (3 SP)

```rust
// wraith-crypto/benches/crypto.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use wraith_crypto::*;

fn bench_x25519(c: &mut Criterion) {
    let mut rng = rand_core::OsRng;
    let private = x25519::PrivateKey::generate(&mut rng);
    let public = x25519::PublicKey::from_bytes([1u8; 32]);

    c.bench_function("x25519_exchange", |b| {
        b.iter(|| private.exchange(black_box(&public)))
    });
}

fn bench_blake3(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3");

    for size in [1024, 65536, 1_048_576] {
        let data = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("{}", size), &data, |b, data| {
            b.iter(|| hash::hash(black_box(data)))
        });
    }

    group.finish();
}

fn bench_aead(c: &mut Criterion) {
    let mut group = c.benchmark_group("xchacha20poly1305");

    for size in [1024, 65536, 1_048_576] {
        let key = aead::AeadKey::new([1u8; 32]);
        let nonce = [0u8; 24];
        let data = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("encrypt_{}", size), &data, |b, data| {
            b.iter(|| key.encrypt(black_box(&nonce), black_box(data), b""))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_x25519, bench_blake3, bench_aead);
criterion_main!(benches);
```

**Acceptance Criteria:**
- [ ] Benchmarks for all crypto primitives
- [ ] Performance meets targets (>3 GB/s AEAD)
- [ ] Benchmark suite runs in CI
- [ ] Results documented

---

**2.6.3: Security Documentation** (2 SP)

Document cryptographic implementation details and security properties.

```markdown
# wraith-crypto/SECURITY.md

## Cryptographic Primitives

### X25519 (RFC 7748)
- Implementation: `curve25519-dalek`
- Key size: 256 bits
- Security level: ~128 bits
- Resistant to: Timing attacks, invalid curve attacks

### XChaCha20-Poly1305
- Stream cipher: XChaCha20 (extended nonce)
- MAC: Poly1305
- Nonce size: 192 bits (never reused)
- Tag size: 128 bits

### BLAKE3
- Hash output: 256 bits
- Security level: 128 bits pre-image, 256 bits collision
- Features: Tree hashing, parallelizable

### Noise_XX Pattern
- Mutual authentication
- Forward secrecy: Yes (ephemeral keys + ratcheting)
- Identity hiding: Encrypted static keys
- 0-RTT: No (3-message handshake)

## Security Properties

1. **Confidentiality**: All data encrypted with XChaCha20-Poly1305
2. **Integrity**: Poly1305 authentication tag
3. **Forward Secrecy**: DH ratchet, ephemeral keys deleted
4. **Post-Compromise Security**: Ratcheting provides healing
5. **Replay Protection**: Message counters, session IDs

## Implementation Notes

- All operations are constant-time (verified with `subtle` crate)
- Keys zeroized on drop (`zeroize` crate)
- No secret-dependent branching
- No secret-dependent memory access patterns

## Audits

- Internal code review: [DATE]
- External audit: [Planned]
- Fuzzing: Continuous (oss-fuzz)
```

**Acceptance Criteria:**
- [ ] Security properties documented
- [ ] Implementation details explained
- [ ] Threat model defined
- [ ] Audit plan created

---

## Definition of Done (Phase 2)

### Code Quality
- [ ] All code passes `cargo clippy` with zero warnings
- [ ] All code formatted with `rustfmt`
- [ ] No unsafe code (or justified and documented)
- [ ] All public APIs documented with rustdoc
- [ ] Test coverage >90%

### Functionality
- [ ] All cryptographic primitives implemented
- [ ] Noise_XX handshake completes successfully
- [ ] AEAD encryption/decryption works
- [ ] Key ratcheting provides forward secrecy
- [ ] All test vectors pass

### Performance
- [ ] Handshake completes in <50ms (LAN)
- [ ] AEAD throughput >3 GB/s (single core, AVX2)
- [ ] All operations constant-time (verified)

### Security
- [ ] Constant-time operations verified with tools
- [ ] Keys properly zeroized on drop
- [ ] No timing side channels (code review)
- [ ] Security documentation complete

### Testing
- [ ] Unit tests for all modules
- [ ] Integration tests for full handshake
- [ ] RFC test vectors implemented
- [ ] Property-based tests for crypto primitives
- [ ] Fuzzing harness created

### Documentation
- [ ] Module-level documentation
- [ ] API documentation (rustdoc)
- [ ] Security properties documented
- [ ] Performance characteristics documented
- [ ] Examples provided

---

## Risk Mitigation

### Cryptographic Correctness
**Risk**: Subtle bugs in crypto implementation
**Mitigation**:
- Use well-audited libraries (`curve25519-dalek`, `chacha20poly1305`)
- Implement all RFC test vectors
- Constant-time verification with `subtle` crate
- Security-focused code review

### Performance
**Risk**: Cannot achieve 3 GB/s AEAD throughput
**Mitigation**:
- Use CPU-specific optimizations (AVX2, NEON)
- Profile early and often
- Benchmark against targets
- Document actual performance if targets not met

### Timing Attacks
**Risk**: Secret-dependent timing leaks information
**Mitigation**:
- Use constant-time operations for all crypto
- Code review for timing side channels
- Testing with timing analysis tools
- No branching on secret data

---

## Dependencies

**Previous Phase:**
- Phase 1: Frame encoding, session states

**Required Libraries:**
- `curve25519-dalek` (X25519)
- `chacha20poly1305` (AEAD)
- `blake3` (Hashing)
- `zeroize` (Memory zeroization)
- `subtle` (Constant-time operations)
- `rand_core` (Randomness)

**Next Phase:**
- Phase 3: Transport layer will use crypto for encryption
- Phase 4: Obfuscation will use Elligator2

---

## Phase 2 Completion Checklist

- [ ] Sprint 2.1: Cryptographic primitives (X25519, BLAKE3, constant-time)
- [ ] Sprint 2.2: Elligator2 encoding
- [ ] Sprint 2.3: Noise_XX handshake (3 messages)
- [ ] Sprint 2.4: XChaCha20-Poly1305 AEAD
- [ ] Sprint 2.5: Key ratcheting (symmetric + DH)
- [ ] Sprint 2.6: Testing, benchmarks, documentation
- [ ] All acceptance criteria met
- [ ] All performance targets achieved
- [ ] Security review complete
- [ ] Documentation published

**Estimated Completion:** Week 12 (end of Phase 2)
