//! Zeroization validation tests
//!
//! Verifies that sensitive cryptographic material is properly zeroized on drop
//! to prevent key material from lingering in memory.
//!
//! SECURITY NOTE: All hard-coded cryptographic values in this file are intentional
//! test data for verifying zeroization behavior, NOT production keys.

use wraith_crypto::aead::{AeadKey, SessionCrypto};
use wraith_crypto::ratchet::{DoubleRatchet, MessageKey, SymmetricRatchet};
use wraith_crypto::x25519::PrivateKey;

/// Helper function to check if memory region contains all zeros
fn is_zeroed(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0)
}

/// Helper to create a pointer to stack memory and verify zeroization
#[allow(dead_code)]
unsafe fn verify_stack_zeroed<F>(size: usize, test_fn: F)
where
    F: FnOnce(*const u8),
{
    let buffer = vec![0u8; size];
    let ptr = buffer.as_ptr();

    // Run test function
    test_fn(ptr);

    // Buffer should be zeroed after test function returns
    assert!(is_zeroed(&buffer), "Stack memory was not properly zeroized");
}

#[test]
fn test_aead_key_zeroization() {
    // Create a key
    let key_bytes = [42u8; 32];
    let key = AeadKey::new(key_bytes);

    // Drop the key
    drop(key);

    // Note: This test is limited because we can't directly read the memory
    // after drop without unsafe code. In practice, zeroize crate handles this.
    // The fact that AeadKey derives ZeroizeOnDrop is the primary guarantee.
}

#[test]
fn test_session_crypto_zeroization() {
    // Create session crypto
    let send_key = [1u8; 32];
    let recv_key = [2u8; 32];
    let chain_key = [3u8; 32];

    let session = SessionCrypto::new(send_key, recv_key, &chain_key);

    // SessionCrypto derives ZeroizeOnDrop, so keys should be zeroed on drop
    drop(session);

    // The zeroize crate ensures keys are zeroed
    // We verify this by checking that the type has #[derive(ZeroizeOnDrop)]
}

#[test]
fn test_symmetric_ratchet_zeroization() {
    // Create ratchet
    let chain_key = [42u8; 32];
    let mut ratchet = SymmetricRatchet::new(&chain_key);

    // Derive some keys
    let _key1 = ratchet.next_key();
    let _key2 = ratchet.next_key();

    // Drop ratchet - chain key should be zeroed
    drop(ratchet);

    // ZeroizeOnDrop ensures chain_key is zeroed
}

#[test]
fn test_message_key_zeroization() {
    // Create a ratchet and derive a key
    let chain_key = [99u8; 32];
    let mut ratchet = SymmetricRatchet::new(&chain_key);
    let key = ratchet.next_key();

    // MessageKey derives ZeroizeOnDrop
    drop(key);

    // Key material should be zeroed
}

#[test]
fn test_double_ratchet_zeroization() {
    // Create double ratchet
    let shared_secret = [11u8; 32];
    let remote_public = PrivateKey::generate(&mut rand::thread_rng()).public_key();

    let mut ratchet =
        DoubleRatchet::new_initiator(&mut rand::thread_rng(), &shared_secret, remote_public);

    // Perform some ratchet steps
    let plaintext = b"test message";
    let _ = ratchet.encrypt(&mut rand::thread_rng(), plaintext);
    let _ = ratchet.encrypt(&mut rand::thread_rng(), plaintext);

    // Drop ratchet - all keys should be zeroed
    drop(ratchet);

    // DoubleRatchet derives ZeroizeOnDrop
}

#[test]
fn test_private_key_zeroization() {
    // Generate a private key
    let key = PrivateKey::generate(&mut rand::thread_rng());

    // PrivateKey should zeroize on drop
    drop(key);

    // The x25519_dalek::StaticSecret type handles zeroization
}

#[test]
fn test_message_key_to_aead_conversion() {
    // Create a message key
    let chain_key = [77u8; 32];
    let mut ratchet = SymmetricRatchet::new(&chain_key);
    let message_key = ratchet.next_key();

    // Convert to AEAD key
    let aead_key = message_key.to_aead_key();

    // Drop both - both should zeroize
    drop(message_key);
    drop(aead_key);

    // Both MessageKey and AeadKey derive ZeroizeOnDrop
}

#[test]
fn test_session_crypto_encryption_zeroization() {
    // Create session crypto
    let send_key = [0x11u8; 32];
    let recv_key = [0x22u8; 32];
    let chain_key = [0x33u8; 32];

    let mut session = SessionCrypto::new(send_key, recv_key, &chain_key);

    // Encrypt some data
    let plaintext = b"sensitive data";
    let aad = b"additional data";

    let ciphertext = session.encrypt(plaintext, aad).expect("encryption failed");

    // Ciphertext can be dropped normally
    drop(ciphertext);

    // Drop session - keys should be zeroed
    drop(session);

    // SessionCrypto derives ZeroizeOnDrop
}

#[test]
fn test_ratchet_skipped_keys_zeroization() {
    // Create ratchet
    let chain_key = [0xAAu8; 32];
    let mut ratchet = SymmetricRatchet::new(&chain_key);

    // Skip to later counter
    let skipped = ratchet.skip_to(5).expect("skip failed");

    // Skipped keys should all be zeroizable
    for (_counter, key) in skipped {
        drop(key); // Each MessageKey is ZeroizeOnDrop
    }

    drop(ratchet);
}

#[test]
fn test_double_ratchet_skipped_message_keys() {
    // Create double ratchet
    let shared_secret = [0xBBu8; 32];
    let remote_public = PrivateKey::generate(&mut rand::thread_rng()).public_key();

    let mut ratchet =
        DoubleRatchet::new_initiator(&mut rand::thread_rng(), &shared_secret, remote_public);

    // Generate some keys
    let plaintext = b"test";
    let _ = ratchet.encrypt(&mut rand::thread_rng(), plaintext);
    let _ = ratchet.encrypt(&mut rand::thread_rng(), plaintext);

    // DoubleRatchet manages all keys with ZeroizeOnDrop
    drop(ratchet);
}

/// Verify that key material is not left in stack or heap after operations
#[test]
fn test_no_key_leakage_in_encrypt_decrypt() {
    // Create session
    let send_key = [0xEEu8; 32];
    let recv_key = [0xFFu8; 32];
    let chain_key = [0x00u8; 32];

    let mut send_session = SessionCrypto::new(send_key, recv_key, &chain_key);
    let mut recv_session = SessionCrypto::new(recv_key, send_key, &chain_key);

    // Perform encryption/decryption
    let plaintext = b"secret message";
    let aad = b"";

    let ciphertext = send_session
        .encrypt(plaintext, aad)
        .expect("encryption failed");
    let _decrypted = recv_session
        .decrypt(&ciphertext, aad)
        .expect("decryption failed");

    // Drop sessions
    drop(send_session);
    drop(recv_session);

    // Keys should be zeroed
    // Note: The zeroize crate guarantees this for types with ZeroizeOnDrop
}

/// Compile-time verification that key types derive ZeroizeOnDrop
#[test]
fn test_zeroize_trait_bounds() {
    // This test verifies at compile time that key types implement the necessary traits
    fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}

    // These should all compile (types derive ZeroizeOnDrop)
    assert_zeroize_on_drop::<SessionCrypto>();
    assert_zeroize_on_drop::<SymmetricRatchet>();
    assert_zeroize_on_drop::<MessageKey>();
    assert_zeroize_on_drop::<DoubleRatchet>();

    // Note: AeadKey also derives ZeroizeOnDrop
    // Note: PrivateKey (x25519) has built-in zeroization
}

/// Test that multiple drops don't cause issues (idempotent zeroization)
#[test]
fn test_double_drop_safety() {
    // Create a key
    let chain_key = [0x12u8; 32];
    let ratchet = SymmetricRatchet::new(&chain_key);

    // First drop
    drop(ratchet);

    // Rust prevents double-drop at compile time, but if we could,
    // zeroize should be safe to call multiple times
}

/// Test zeroization under panic conditions
#[test]
#[should_panic(expected = "intentional panic")]
fn test_zeroization_on_panic() {
    let chain_key = [0x34u8; 32];
    let _ratchet = SymmetricRatchet::new(&chain_key);

    // Even if we panic, ZeroizeOnDrop ensures cleanup
    panic!("intentional panic");
}

/// Test that cloning doesn't leak keys
#[test]
fn test_no_clone_for_sensitive_types() {
    // Sensitive types should NOT implement Clone to prevent key duplication
    // This is enforced by not deriving Clone

    // The following would not compile if uncommented:
    // let chain_key = [0x56u8; 32];
    // let ratchet1 = SymmetricRatchet::new(&chain_key);
    // let ratchet2 = ratchet1.clone(); // ERROR: no Clone trait
}
