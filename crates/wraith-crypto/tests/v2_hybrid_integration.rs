//! Integration tests for hybrid KEM end-to-end.

use rand_core::OsRng;
use wraith_crypto::hybrid::{HybridCiphertext, HybridKeyPair, HybridPublicKey};
use wraith_crypto::kdf::derive_session_keys_v2;

#[test]
fn full_keygen_encapsulate_decapsulate_cycle() {
    let alice = HybridKeyPair::generate(&mut OsRng);
    let bob = HybridKeyPair::generate(&mut OsRng);

    // Alice encapsulates to Bob
    let (ss_alice, ct) = bob.public.encapsulate(&mut OsRng).unwrap();
    // Bob decapsulates
    let ss_bob = bob.secret.decapsulate(&ct).unwrap();
    assert_eq!(ss_alice.as_bytes(), ss_bob.as_bytes());

    // Bob encapsulates to Alice
    let (ss_bob2, ct2) = alice.public.encapsulate(&mut OsRng).unwrap();
    let ss_alice2 = alice.secret.decapsulate(&ct2).unwrap();
    assert_eq!(ss_bob2.as_bytes(), ss_alice2.as_bytes());
}

#[test]
fn classical_only_fallback_produces_valid_shared_secret() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss_enc, epk) = kp.public.encapsulate_classical_only(&mut OsRng).unwrap();
    let ss_dec = kp.secret.decapsulate_classical_only(&epk).unwrap();
    assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    // Shared secret is non-zero
    assert_ne!(ss_enc.as_bytes(), &[0u8; 32]);
}

#[test]
fn hybrid_vs_classical_produces_different_shared_secrets() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss_hybrid, _ct) = kp.public.encapsulate(&mut OsRng).unwrap();
    let (ss_classical, _epk) = kp.public.encapsulate_classical_only(&mut OsRng).unwrap();

    // Domain separation ensures these differ even though the classical component
    // uses different ephemeral keys anyway. The key point is that the hybrid scheme
    // mixes in a non-zero PQ contribution while classical uses zero PQ.
    assert_ne!(ss_hybrid.as_bytes(), ss_classical.as_bytes());
}

#[test]
fn multiple_encapsulations_to_same_key_produce_different_ciphertexts() {
    let kp = HybridKeyPair::generate(&mut OsRng);

    let (_ss1, ct1) = kp.public.encapsulate(&mut OsRng).unwrap();
    let (_ss2, ct2) = kp.public.encapsulate(&mut OsRng).unwrap();

    // The ephemeral keys are randomized, so ciphertexts differ
    let ct1_bytes = ct1.to_bytes();
    let ct2_bytes = ct2.to_bytes();
    assert_ne!(ct1_bytes, ct2_bytes);
}

#[test]
fn public_key_serialization_roundtrip() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let bytes = kp.public.to_bytes();
    let recovered = HybridPublicKey::from_bytes(&bytes).unwrap();
    let bytes2 = recovered.to_bytes();
    assert_eq!(bytes, bytes2);
}

#[test]
fn ciphertext_serialization_roundtrip() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss_enc, ct) = kp.public.encapsulate(&mut OsRng).unwrap();
    let bytes = ct.to_bytes();
    let recovered = HybridCiphertext::from_bytes(&bytes).unwrap();
    let bytes2 = recovered.to_bytes();
    assert_eq!(bytes, bytes2);

    // Recovered ciphertext still decapsulates correctly
    let ss_dec = kp.secret.decapsulate(&recovered).unwrap();
    assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
}

#[test]
fn invalid_ciphertext_bytes_rejected() {
    // Too short
    assert!(HybridCiphertext::from_bytes(&[0u8; 10]).is_err());
    // Too long
    assert!(HybridCiphertext::from_bytes(&[0u8; 2000]).is_err());
    // Empty
    assert!(HybridCiphertext::from_bytes(&[]).is_err());
}

#[test]
fn invalid_public_key_bytes_rejected() {
    // Too short
    assert!(HybridPublicKey::from_bytes(&[0u8; 10]).is_err());
    // Too long
    assert!(HybridPublicKey::from_bytes(&[0u8; 2000]).is_err());
    // Empty
    assert!(HybridPublicKey::from_bytes(&[]).is_err());
}

#[test]
fn hybrid_shared_secret_fed_into_v2_kdf_produces_valid_session_keys() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss, _ct) = kp.public.encapsulate(&mut OsRng).unwrap();

    let transcript_hash = blake3::hash(b"test handshake transcript");
    let keys = derive_session_keys_v2(ss.as_bytes(), transcript_hash.as_bytes());

    // All keys should be non-zero and distinct
    assert_ne!(keys.initiator_to_responder, [0u8; 32]);
    assert_ne!(keys.responder_to_initiator, [0u8; 32]);
    assert_ne!(keys.format_key, [0u8; 32]);
    assert_ne!(keys.initial_chain_key, [0u8; 32]);
    assert_ne!(keys.initiator_to_responder, keys.responder_to_initiator);
    assert_ne!(keys.initiator_to_responder, keys.initial_chain_key);
}
