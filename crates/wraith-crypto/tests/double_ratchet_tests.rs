use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
use wraith_crypto::x25519::PrivateKey;
use wraith_crypto::random::SecureRng;

#[test]
fn test_double_ratchet_rekeying_integration() {
    // 1. Setup
    let alice_static = NoiseKeypair::generate().unwrap();
    let bob_static = NoiseKeypair::generate().unwrap();

    let mut alice_hs = NoiseHandshake::new_initiator(&alice_static).unwrap();
    let mut bob_hs = NoiseHandshake::new_responder(&bob_static).unwrap();

    // Handshake
    let msg1 = alice_hs.write_message(&[]).unwrap();
    bob_hs.read_message(&msg1).unwrap();

    let msg2 = bob_hs.write_message(&[]).unwrap();
    alice_hs.read_message(&msg2).unwrap();

    let msg3 = alice_hs.write_message(&[]).unwrap();
    bob_hs.read_message(&msg3).unwrap();

    // Generate initial ratchet keys (simulating what would happen in a real session)
    let mut rng = SecureRng::new();
    let bob_ratchet_priv = PrivateKey::generate(&mut rng);
    let bob_ratchet_pub = bob_ratchet_priv.public_key();

    // Alice (Initiator) needs Bob's public ratchet key
    let mut alice = alice_hs.into_transport(None, Some(bob_ratchet_pub)).unwrap();
    
    // Bob (Responder) needs his own private ratchet key
    let mut bob = bob_hs.into_transport(Some(bob_ratchet_priv), None).unwrap();

    // 2. Normal Message
    let plaintext1 = b"msg1";
    let ciphertext1 = alice.write_message(plaintext1).unwrap();
    let decrypted1 = bob.read_message(&ciphertext1).unwrap();
    assert_eq!(decrypted1, plaintext1);

    // Bob replies to establish his new key on Alice's side
    let plaintext_reply = b"reply1";
    let ciphertext_reply = bob.write_message(plaintext_reply).unwrap();
    let decrypted_reply = alice.read_message(&ciphertext_reply).unwrap();
    assert_eq!(decrypted_reply, plaintext_reply);

    // 3. Alice Rekey (Force DH Step)
    alice.rekey_dh().unwrap();

    // 4. Alice sends message with NEW key
    let plaintext2 = b"msg2 - rekeyed";
    let ciphertext2 = alice.write_message(plaintext2).unwrap();
    
    // Verify ciphertext is different/valid structure
    // Header is 40 bytes.
    let header_bytes = &ciphertext2[0..40];
    
    // 5. Bob receives. Should trigger DH ratchet.
    let decrypted2 = bob.read_message(&ciphertext2).unwrap();
    assert_eq!(decrypted2, plaintext2);

    // 6. Verify forward secrecy (can't decrypt old msg with new state? DoubleRatchet zeroizes keys)
    // But verify Bob accepted the new key.
    
    // If Bob replies, he should use the new ratchet.
    let plaintext3 = b"msg3 - reply";
    let ciphertext3 = bob.write_message(plaintext3).unwrap();
    let decrypted3 = alice.read_message(&ciphertext3).unwrap();
    assert_eq!(decrypted3, plaintext3);
}