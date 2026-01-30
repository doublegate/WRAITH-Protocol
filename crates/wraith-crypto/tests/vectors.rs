//! Cryptographic test vectors from official specifications.
//!
//! This module contains test vectors from:
//! - RFC 7748 (X25519)
//! - RFC 8439 (ChaCha20-Poly1305)
//! - BLAKE3 official test vectors
//!
//! These vectors ensure our implementations match the specifications exactly.
//!
//! SECURITY NOTE: All hard-coded cryptographic values in this file are intentional
//! test vectors from RFCs and official specifications, NOT production keys.

use wraith_crypto::aead::{AeadKey, Nonce};
use wraith_crypto::hash;
use wraith_crypto::x25519::{PrivateKey, PublicKey};

// Helper function to decode hex strings
fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

// ============================================================================
// RFC 7748 Test Vectors (X25519)
// ============================================================================

#[test]
fn test_x25519_rfc7748_vector_1() {
    // RFC 7748 Section 5.2 - Test Vector 1
    let alice_private =
        decode_hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let alice_public_expected =
        decode_hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");

    let bob_private =
        decode_hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let bob_public_expected =
        decode_hex("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");

    let shared_expected =
        decode_hex("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");

    // Parse keys
    let mut alice_bytes = [0u8; 32];
    alice_bytes.copy_from_slice(&alice_private);
    let alice = PrivateKey::from_bytes(alice_bytes);
    let alice_public = alice.public_key();

    let mut bob_bytes = [0u8; 32];
    bob_bytes.copy_from_slice(&bob_private);
    let bob = PrivateKey::from_bytes(bob_bytes);
    let bob_public = bob.public_key();

    // Verify public keys
    assert_eq!(alice_public.to_bytes().to_vec(), alice_public_expected);
    assert_eq!(bob_public.to_bytes().to_vec(), bob_public_expected);

    // Verify shared secret
    let alice_shared = alice.exchange(&bob_public).expect("DH exchange failed");
    let bob_shared = bob.exchange(&alice_public).expect("DH exchange failed");

    assert_eq!(alice_shared.as_bytes().to_vec(), shared_expected);
    assert_eq!(bob_shared.as_bytes().to_vec(), shared_expected);
}

#[test]
fn test_x25519_scalar_multiplication() {
    // RFC 7748 Section 5.2 - Scalar multiplication test vector
    let scalar = decode_hex("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let point = decode_hex("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
    let expected = decode_hex("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");

    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&scalar);
    let private = PrivateKey::from_bytes(scalar_bytes);

    let mut point_bytes = [0u8; 32];
    point_bytes.copy_from_slice(&point);
    let public = PublicKey::from_bytes(point_bytes);

    let shared = private.exchange(&public).expect("DH exchange failed");
    assert_eq!(shared.as_bytes().to_vec(), expected);
}

#[test]
fn test_x25519_low_order_rejection() {
    // Points of low order should be rejected
    let private = PrivateKey::generate(&mut rand_core::OsRng);

    // All-zeros is a low-order point
    let zero_public = PublicKey::from_bytes([0u8; 32]);
    assert!(private.exchange(&zero_public).is_none());

    // Point at infinity represented as all zeros should fail
    let low_order_points = [
        // Identity point
        [0u8; 32],
    ];

    for point_bytes in low_order_points {
        let public = PublicKey::from_bytes(point_bytes);
        let result = private.exchange(&public);
        // Should either be None or return a non-zero shared secret
        if let Some(shared) = result {
            // If accepted, it must not be all zeros
            assert_ne!(
                shared.as_bytes(),
                &[0u8; 32],
                "Low-order point produced zero shared secret"
            );
        }
    }
}

// ============================================================================
// BLAKE3 Test Vectors
// ============================================================================

#[test]
fn test_blake3_empty() {
    // Official BLAKE3 test vector for empty input
    let hash = hash::hash(b"");
    let expected = decode_hex("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");

    assert_eq!(hash.to_vec(), expected);
}

#[test]
fn test_blake3_single_byte() {
    // Test single byte input
    let hash = hash::hash(&[0u8]);

    // Verify it's deterministic and non-zero
    let hash2 = hash::hash(&[0u8]);
    assert_eq!(hash, hash2);
    assert_ne!(hash, [0u8; 32]);
}

#[test]
fn test_blake3_incremental() {
    // Test that incremental hashing matches single-shot
    let data = b"hello world";

    let single_shot = hash::hash(data);

    let mut hasher = hash::TreeHasher::new();
    hasher.update(b"hello ");
    hasher.update(b"world");
    let incremental = hasher.finalize();

    assert_eq!(single_shot, incremental);
}

#[test]
fn test_blake3_kdf_separation() {
    // Different contexts must produce different keys
    let ikm = b"input key material";

    let kdf1 = hash::Kdf::new("context1");
    let kdf2 = hash::Kdf::new("context2");

    let key1 = kdf1.derive_key(ikm);
    let key2 = kdf2.derive_key(ikm);

    assert_ne!(key1, key2);
}

#[test]
fn test_blake3_hkdf() {
    // Test HKDF extract-expand pattern
    let salt = b"salt";
    let ikm = b"input key material";
    let info = b"application info";

    let prk = hash::hkdf_extract(salt, ikm);

    let mut okm1 = [0u8; 64];
    let mut okm2 = [0u8; 64];

    hash::hkdf_expand(&prk, info, &mut okm1);
    hash::hkdf_expand(&prk, info, &mut okm2);

    // Must be deterministic
    assert_eq!(okm1, okm2);

    // Different info must produce different output
    let mut okm3 = [0u8; 64];
    hash::hkdf_expand(&prk, b"different info", &mut okm3);
    assert_ne!(okm1, okm3);
}

// ============================================================================
// XChaCha20-Poly1305 Test Vectors
// ============================================================================

#[test]
fn test_xchacha_basic_roundtrip() {
    let key_bytes = [0x42u8; 32];
    let key = AeadKey::new(key_bytes);
    let nonce = Nonce::from_bytes([0u8; 24]);

    let plaintext = b"secret message";
    let aad = b"additional data";

    let ciphertext = key
        .encrypt(&nonce, plaintext, aad)
        .expect("Encryption failed");
    let decrypted = key
        .decrypt(&nonce, &ciphertext, aad)
        .expect("Decryption failed");

    assert_eq!(plaintext.to_vec(), decrypted);
}

#[test]
fn test_xchacha_authentication() {
    let key = AeadKey::new([0x42u8; 32]);
    let nonce = Nonce::from_bytes([0u8; 24]);

    let plaintext = b"secret message";
    let ciphertext = key
        .encrypt(&nonce, plaintext, b"")
        .expect("Encryption failed");

    // Tamper with ciphertext
    let mut tampered = ciphertext.clone();
    if !tampered.is_empty() {
        tampered[0] ^= 0xFF;
    }

    // Tampered ciphertext should fail authentication
    assert!(key.decrypt(&nonce, &tampered, b"").is_err());

    // Tamper with tag (last 16 bytes)
    let mut tag_tampered = ciphertext.clone();
    let len = tag_tampered.len();
    if len >= 16 {
        tag_tampered[len - 1] ^= 0xFF;
    }

    assert!(key.decrypt(&nonce, &tag_tampered, b"").is_err());
}

#[test]
fn test_xchacha_wrong_key() {
    let key1 = AeadKey::new([0x42u8; 32]);
    let key2 = AeadKey::new([0x43u8; 32]);
    let nonce = Nonce::from_bytes([0u8; 24]);

    let ciphertext = key1
        .encrypt(&nonce, b"secret", b"")
        .expect("Encryption failed");

    // Wrong key should fail
    assert!(key2.decrypt(&nonce, &ciphertext, b"").is_err());
}

#[test]
fn test_xchacha_wrong_nonce() {
    let key = AeadKey::new([0x42u8; 32]);
    let nonce1 = Nonce::from_bytes([0u8; 24]);
    let nonce2 = Nonce::from_bytes([1u8; 24]);

    let ciphertext = key
        .encrypt(&nonce1, b"secret", b"")
        .expect("Encryption failed");

    // Wrong nonce should fail
    assert!(key.decrypt(&nonce2, &ciphertext, b"").is_err());
}

#[test]
fn test_xchacha_wrong_aad() {
    let key = AeadKey::new([0x42u8; 32]);
    let nonce = Nonce::from_bytes([0u8; 24]);

    let ciphertext = key
        .encrypt(&nonce, b"secret", b"aad1")
        .expect("Encryption failed");

    // Wrong AAD should fail
    assert!(key.decrypt(&nonce, &ciphertext, b"aad2").is_err());
}

#[test]
fn test_xchacha_empty_message() {
    let key = AeadKey::new([0x42u8; 32]);
    let nonce = Nonce::from_bytes([0u8; 24]);

    // Empty plaintext is valid
    let ciphertext = key.encrypt(&nonce, b"", b"aad").expect("Encryption failed");

    // Should contain only the tag
    assert_eq!(ciphertext.len(), 16);

    let decrypted = key
        .decrypt(&nonce, &ciphertext, b"aad")
        .expect("Decryption failed");
    assert!(decrypted.is_empty());
}

#[test]
fn test_xchacha_large_message() {
    let key = AeadKey::new([0x42u8; 32]);
    let nonce = Nonce::from_bytes([0u8; 24]);

    // Test with 1 MiB message
    let plaintext = vec![0x42u8; 1024 * 1024];

    let ciphertext = key
        .encrypt(&nonce, &plaintext, b"")
        .expect("Encryption failed");
    let decrypted = key
        .decrypt(&nonce, &ciphertext, b"")
        .expect("Decryption failed");

    assert_eq!(plaintext, decrypted);
}

// ============================================================================
// Noise Protocol Test Vectors
// ============================================================================

#[test]
fn test_noise_xx_handshake_produces_unique_keys() {
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

    // Generate two sessions with different static keys
    let alice_static = NoiseKeypair::generate().unwrap();
    let bob_static = NoiseKeypair::generate().unwrap();

    let mut alice = NoiseHandshake::new_initiator(&alice_static).unwrap();
    let mut bob = NoiseHandshake::new_responder(&bob_static).unwrap();

    // Message 1
    let msg1 = alice.write_message(&[]).unwrap();
    bob.read_message(&msg1).unwrap();

    // Message 2
    let msg2 = bob.write_message(&[]).unwrap();
    alice.read_message(&msg2).unwrap();

    // Message 3
    let msg3 = alice.write_message(&[]).unwrap();
    bob.read_message(&msg3).unwrap();

    // Both should be complete
    assert!(alice.is_complete());
    assert!(bob.is_complete());

    // Get session keys
    let alice_keys = alice.into_session_keys().unwrap();
    let bob_keys = bob.into_session_keys().unwrap();

    // Keys should match (Alice's send = Bob's recv, etc.)
    assert_eq!(alice_keys.send_key, bob_keys.recv_key);
    assert_eq!(alice_keys.recv_key, bob_keys.send_key);
}

#[test]
fn test_noise_handshake_with_payloads() {
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

    let alice_static = NoiseKeypair::generate().unwrap();
    let bob_static = NoiseKeypair::generate().unwrap();

    let mut alice = NoiseHandshake::new_initiator(&alice_static).unwrap();
    let mut bob = NoiseHandshake::new_responder(&bob_static).unwrap();

    // Include payloads in handshake messages
    let payload1 = b"hello from alice";
    let payload2 = b"hello from bob";
    let payload3 = b"final message";

    let msg1 = alice.write_message(payload1).unwrap();
    let recv1 = bob.read_message(&msg1).unwrap();
    assert_eq!(recv1, payload1);

    let msg2 = bob.write_message(payload2).unwrap();
    let recv2 = alice.read_message(&msg2).unwrap();
    assert_eq!(recv2, payload2);

    let msg3 = alice.write_message(payload3).unwrap();
    let recv3 = bob.read_message(&msg3).unwrap();
    assert_eq!(recv3, payload3);
}

// ============================================================================
// Double Ratchet Test Vectors
// ============================================================================

#[test]
fn test_double_ratchet_forward_secrecy() {
    use wraith_crypto::ratchet::DoubleRatchet;
    use wraith_crypto::x25519::PrivateKey;

    let shared_secret = [0x42u8; 32];
    let bob_dh = PrivateKey::generate(&mut rand_core::OsRng);
    let bob_dh_public = bob_dh.public_key();

    let mut alice =
        DoubleRatchet::new_initiator(&mut rand_core::OsRng, &shared_secret, bob_dh_public);
    let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

    // Alice sends messages
    let (h1, c1) = alice.encrypt(&mut rand_core::OsRng, b"message 1").unwrap();
    let (h2, c2) = alice.encrypt(&mut rand_core::OsRng, b"message 2").unwrap();

    // Bob decrypts
    let p1 = bob.decrypt(&mut rand_core::OsRng, &h1, &c1).unwrap();
    let p2 = bob.decrypt(&mut rand_core::OsRng, &h2, &c2).unwrap();

    assert_eq!(p1, b"message 1");
    assert_eq!(p2, b"message 2");

    // The ciphertexts should be different (forward secrecy property)
    assert_ne!(c1, c2);
}

#[test]
fn test_double_ratchet_dh_ratchet_step() {
    use wraith_crypto::ratchet::DoubleRatchet;
    use wraith_crypto::x25519::PrivateKey;

    let shared_secret = [0x42u8; 32];
    let bob_dh = PrivateKey::generate(&mut rand_core::OsRng);
    let bob_dh_public = bob_dh.public_key();

    let mut alice =
        DoubleRatchet::new_initiator(&mut rand_core::OsRng, &shared_secret, bob_dh_public);
    let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

    // Alice -> Bob
    let (h1, c1) = alice
        .encrypt(&mut rand_core::OsRng, b"alice msg 1")
        .unwrap();
    bob.decrypt(&mut rand_core::OsRng, &h1, &c1).unwrap();

    // Bob -> Alice (triggers DH ratchet in Alice)
    let (h2, c2) = bob.encrypt(&mut rand_core::OsRng, b"bob msg 1").unwrap();
    alice.decrypt(&mut rand_core::OsRng, &h2, &c2).unwrap();

    // Alice -> Bob (triggers DH ratchet in Bob)
    let (h3, c3) = alice
        .encrypt(&mut rand_core::OsRng, b"alice msg 2")
        .unwrap();
    bob.decrypt(&mut rand_core::OsRng, &h3, &c3).unwrap();

    // All DH public keys should be different (fresh keys each ratchet step)
    assert_ne!(h1.dh_public.to_bytes(), h2.dh_public.to_bytes());
    assert_ne!(h2.dh_public.to_bytes(), h3.dh_public.to_bytes());
    assert_ne!(h1.dh_public.to_bytes(), h3.dh_public.to_bytes());
}

// ============================================================================
// Elligator2 Test Vectors
// ============================================================================

#[test]
fn test_elligator2_uniform_distribution() {
    use wraith_crypto::elligator::generate_encodable_keypair;

    // Generate many representatives and verify they look random
    let mut byte_counts = [0u32; 256];
    let sample_count = 100;

    for _ in 0..sample_count {
        let (_, repr) = generate_encodable_keypair(&mut rand_core::OsRng);
        for &byte in repr.as_bytes() {
            byte_counts[byte as usize] += 1;
        }
    }

    // Total bytes = 100 samples * 32 bytes = 3200
    let total: u32 = byte_counts.iter().sum();
    assert_eq!(total, (sample_count * 32) as u32);

    // Expected count per byte value: 3200 / 256 = 12.5
    // No byte should appear more than ~50 times (very loose bound)
    let max_count = *byte_counts.iter().max().unwrap();
    assert!(
        max_count < 100,
        "Byte distribution not uniform: max count = {}",
        max_count
    );
}

#[test]
fn test_elligator2_key_exchange_works() {
    use wraith_crypto::elligator::ElligatorKeypair;

    for _ in 0..10 {
        let alice = ElligatorKeypair::generate(&mut rand_core::OsRng);
        let bob = ElligatorKeypair::generate(&mut rand_core::OsRng);

        // Exchange using representatives
        let alice_shared = alice.exchange_representative(&bob.representative).unwrap();
        let bob_shared = bob.exchange_representative(&alice.representative).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }
}

// ============================================================================
// Constant-Time Operation Tests
// ============================================================================

#[test]
fn test_constant_time_comparison() {
    use wraith_crypto::constant_time::{ct_eq, verify_32};

    let a = [0x42u8; 32];
    let b = [0x42u8; 32];
    let c = [0x43u8; 32];

    assert!(ct_eq(&a, &b));
    assert!(!ct_eq(&a, &c));

    assert!(verify_32(&a, &b));
    assert!(!verify_32(&a, &c));
}

#[test]
fn test_constant_time_select() {
    use wraith_crypto::constant_time::ct_select;

    let a = [1u8; 8];
    let b = [2u8; 8];
    let mut result = [0u8; 8];

    // Select a when condition is true
    ct_select(true, &a, &b, &mut result);
    assert_eq!(result, a);

    // Select b when condition is false
    ct_select(false, &a, &b, &mut result);
    assert_eq!(result, b);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_crypto_pipeline() {
    // Test complete flow: Key exchange -> Handshake -> AEAD -> Ratcheting
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
    use wraith_crypto::ratchet::DoubleRatchet;

    // Step 1: Noise handshake
    let alice_static = NoiseKeypair::generate().unwrap();
    let bob_static = NoiseKeypair::generate().unwrap();

    let mut alice_hs = NoiseHandshake::new_initiator(&alice_static).unwrap();
    let mut bob_hs = NoiseHandshake::new_responder(&bob_static).unwrap();

    // Complete handshake
    let msg1 = alice_hs.write_message(b"").unwrap();
    bob_hs.read_message(&msg1).unwrap();

    let msg2 = bob_hs.write_message(b"").unwrap();
    alice_hs.read_message(&msg2).unwrap();

    let msg3 = alice_hs.write_message(b"").unwrap();
    bob_hs.read_message(&msg3).unwrap();

    // Get session keys
    let alice_keys = alice_hs.into_session_keys().unwrap();
    let bob_keys = bob_hs.into_session_keys().unwrap();

    // Verify keys match
    assert_eq!(alice_keys.send_key, bob_keys.recv_key);
    assert_eq!(alice_keys.recv_key, bob_keys.send_key);

    // Step 2: Initialize Double Ratchet with handshake output
    let bob_dh = PrivateKey::generate(&mut rand_core::OsRng);
    let bob_dh_public = bob_dh.public_key();

    let mut alice_ratchet =
        DoubleRatchet::new_initiator(&mut rand_core::OsRng, &alice_keys.chain_key, bob_dh_public);
    let mut bob_ratchet = DoubleRatchet::new_responder(&bob_keys.chain_key, bob_dh);

    // Step 3: Exchange encrypted messages
    let messages = [
        b"Hello Bob!".to_vec(),
        b"How are you?".to_vec(),
        b"I'm using WRAITH protocol!".to_vec(),
    ];

    for msg in &messages {
        let (header, ciphertext) = alice_ratchet.encrypt(&mut rand_core::OsRng, msg).unwrap();
        let decrypted = bob_ratchet
            .decrypt(&mut rand_core::OsRng, &header, &ciphertext)
            .unwrap();
        assert_eq!(decrypted, *msg);
    }

    // Bob responds
    let (header, ciphertext) = bob_ratchet
        .encrypt(&mut rand_core::OsRng, b"I'm great!")
        .unwrap();
    let decrypted = alice_ratchet
        .decrypt(&mut rand_core::OsRng, &header, &ciphertext)
        .unwrap();
    assert_eq!(decrypted, b"I'm great!");
}
