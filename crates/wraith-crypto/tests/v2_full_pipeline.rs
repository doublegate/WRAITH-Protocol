//! Full v2 crypto pipeline end-to-end integration tests.

use rand_core::OsRng;
use wraith_crypto::aead::{AeadKey, NONCE_SIZE, Nonce};
use wraith_crypto::context::{CryptoContextV2, EncapsulationResult};
use wraith_crypto::hybrid::HybridKeyPair;
use wraith_crypto::kdf::derive_session_keys_v2;
use wraith_crypto::packet_ratchet::PacketRatchet;
use wraith_crypto::suite::CryptoSuite;

/// Build a nonce from a packet number (simple scheme for testing).
fn nonce_from_pn(pn: u64) -> Nonce {
    let mut bytes = [0u8; NONCE_SIZE];
    bytes[..8].copy_from_slice(&pn.to_le_bytes());
    Nonce::from_bytes(bytes)
}

#[test]
fn full_pipeline_alice_bob_key_exchange_and_messaging() {
    // 1. Alice and Bob each generate hybrid keypairs
    let _alice_kp = HybridKeyPair::generate(&mut OsRng);
    let bob_kp = HybridKeyPair::generate(&mut OsRng);

    // 2. Alice encapsulates to Bob's public key
    let (alice_ss, ct) = bob_kp.public.encapsulate(&mut OsRng).unwrap();

    // 3. Bob decapsulates
    let bob_ss = bob_kp.secret.decapsulate(&ct).unwrap();
    assert_eq!(alice_ss.as_bytes(), bob_ss.as_bytes());

    // 4. Both derive v2 session keys from shared secret
    let transcript = blake3::hash(b"alice-bob handshake transcript");
    let alice_keys = derive_session_keys_v2(alice_ss.as_bytes(), transcript.as_bytes());
    let bob_keys = derive_session_keys_v2(bob_ss.as_bytes(), transcript.as_bytes());

    assert_eq!(
        alice_keys.initiator_to_responder,
        bob_keys.initiator_to_responder
    );
    assert_eq!(
        alice_keys.responder_to_initiator,
        bob_keys.responder_to_initiator
    );
    assert_eq!(alice_keys.initial_chain_key, bob_keys.initial_chain_key);

    // 5. Both create packet ratchets from chain key
    // Alice is initiator: sends with i2r key ratchet, receives with r2i key ratchet
    let mut alice_send_ratchet = PacketRatchet::new(alice_keys.initial_chain_key);
    let mut bob_recv_ratchet = PacketRatchet::new(bob_keys.initial_chain_key);

    // 6. Alice encrypts message with ratchet key + AEAD
    let (pn, send_key) = alice_send_ratchet.next_send_key();
    let alice_aead = AeadKey::new(send_key);
    let nonce = nonce_from_pn(pn);
    let plaintext = b"Hello Bob, this is Alice!";
    let ciphertext = alice_aead.encrypt(&nonce, plaintext, &[]).unwrap();

    // Bob decrypts
    let recv_key = bob_recv_ratchet.key_for_packet(pn).unwrap();
    let bob_aead = AeadKey::new(recv_key);
    let decrypted = bob_aead.decrypt(&nonce, &ciphertext, &[]).unwrap();
    assert_eq!(decrypted, plaintext);

    // 7. Bob encrypts response (using a separate ratchet for reverse direction)
    // For simplicity, we use a second pair of ratchets seeded from r2i key
    let mut bob_send_ratchet = PacketRatchet::new(bob_keys.responder_to_initiator);
    let mut alice_recv_ratchet = PacketRatchet::new(alice_keys.responder_to_initiator);

    let (pn2, bob_send_key) = bob_send_ratchet.next_send_key();
    let bob_aead2 = AeadKey::new(bob_send_key);
    let nonce2 = nonce_from_pn(pn2);
    let response = b"Hi Alice, Bob here!";
    let ct2 = bob_aead2.encrypt(&nonce2, response, &[]).unwrap();

    let alice_recv_key = alice_recv_ratchet.key_for_packet(pn2).unwrap();
    let alice_aead2 = AeadKey::new(alice_recv_key);
    let decrypted2 = alice_aead2.decrypt(&nonce2, &ct2, &[]).unwrap();
    assert_eq!(decrypted2, response);
}

#[test]
fn ten_message_bidirectional_exchange() {
    let _alice_kp = HybridKeyPair::generate(&mut OsRng);
    let bob_kp = HybridKeyPair::generate(&mut OsRng);

    let (ss, ct) = bob_kp.public.encapsulate(&mut OsRng).unwrap();
    let ss_bob = bob_kp.secret.decapsulate(&ct).unwrap();

    let transcript = blake3::hash(b"bidirectional test");
    let alice_keys = derive_session_keys_v2(ss.as_bytes(), transcript.as_bytes());
    let bob_keys = derive_session_keys_v2(ss_bob.as_bytes(), transcript.as_bytes());

    // i2r direction: Alice -> Bob
    let mut alice_i2r_send = PacketRatchet::new(alice_keys.initial_chain_key);
    let mut bob_i2r_recv = PacketRatchet::new(bob_keys.initial_chain_key);

    // r2i direction: Bob -> Alice
    let mut bob_r2i_send = PacketRatchet::new(bob_keys.responder_to_initiator);
    let mut alice_r2i_recv = PacketRatchet::new(alice_keys.responder_to_initiator);

    for i in 0u64..10 {
        // Alice -> Bob
        let msg = format!("Alice message {i}");
        let (pn, key) = alice_i2r_send.next_send_key();
        let aead = AeadKey::new(key);
        let nonce = nonce_from_pn(pn);
        let ct = aead.encrypt(&nonce, msg.as_bytes(), &[]).unwrap();

        let rk = bob_i2r_recv.key_for_packet(pn).unwrap();
        let raead = AeadKey::new(rk);
        let pt = raead.decrypt(&nonce, &ct, &[]).unwrap();
        assert_eq!(pt, msg.as_bytes());

        // Bob -> Alice
        let reply = format!("Bob reply {i}");
        let (pn2, key2) = bob_r2i_send.next_send_key();
        let aead2 = AeadKey::new(key2);
        let nonce2 = nonce_from_pn(pn2);
        let ct2 = aead2.encrypt(&nonce2, reply.as_bytes(), &[]).unwrap();

        let rk2 = alice_r2i_recv.key_for_packet(pn2).unwrap();
        let raead2 = AeadKey::new(rk2);
        let pt2 = raead2.decrypt(&nonce2, &ct2, &[]).unwrap();
        assert_eq!(pt2, reply.as_bytes());
    }
}

#[test]
fn simulate_packet_loss_and_reorder() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss, ct) = kp.public.encapsulate(&mut OsRng).unwrap();
    let _ss_dec = kp.secret.decapsulate(&ct).unwrap();

    let transcript = blake3::hash(b"reorder test");
    let keys = derive_session_keys_v2(ss.as_bytes(), transcript.as_bytes());

    let mut sender = PacketRatchet::new(keys.initial_chain_key);
    let mut receiver = PacketRatchet::new(keys.initial_chain_key);

    // Sender sends 10 packets
    let mut packets: Vec<(u64, Vec<u8>)> = Vec::new();
    for i in 0u64..10 {
        let msg = format!("packet {i}");
        let (pn, key) = sender.next_send_key();
        let aead = AeadKey::new(key);
        let nonce = nonce_from_pn(pn);
        let ct = aead.encrypt(&nonce, msg.as_bytes(), &[]).unwrap();
        packets.push((pn, ct));
    }

    // Simulate reorder: receive in order 5, 3, 7, 0, 9, 1, 8, 2, 6, 4
    let order = [5, 3, 7, 0, 9, 1, 8, 2, 6, 4];
    for &idx in &order {
        let (pn, ref ct) = packets[idx];
        let key = receiver.key_for_packet(pn).unwrap();
        let aead = AeadKey::new(key);
        let nonce = nonce_from_pn(pn);
        let pt = aead.decrypt(&nonce, ct, &[]).unwrap();
        let expected = format!("packet {idx}");
        assert_eq!(pt, expected.as_bytes());
    }
}

#[test]
fn full_pipeline_via_crypto_context() {
    let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);

    let _alice_kp = ctx.generate_keypair(&mut OsRng);
    let bob_kp = ctx.generate_keypair(&mut OsRng);

    // Alice encapsulates to Bob via context
    let (ss_alice, result) = ctx.encapsulate(&mut OsRng, &bob_kp.public).unwrap();

    let ss_bob = match result {
        EncapsulationResult::Hybrid(ct) => bob_kp.secret.decapsulate(&ct).unwrap(),
        EncapsulationResult::ClassicalOnly(epk) => {
            bob_kp.secret.decapsulate_classical_only(&epk).unwrap()
        }
    };
    assert_eq!(ss_alice.as_bytes(), ss_bob.as_bytes());

    let transcript = blake3::hash(b"context pipeline test");
    let alice_keys = ctx.derive_session_keys(ss_alice.as_bytes(), transcript.as_bytes());
    let bob_keys = ctx.derive_session_keys(ss_bob.as_bytes(), transcript.as_bytes());

    let mut alice_ratchet = ctx.create_packet_ratchet(alice_keys.initial_chain_key);
    let mut bob_ratchet = ctx.create_packet_ratchet(bob_keys.initial_chain_key);

    // Send 5 messages
    for i in 0..5 {
        let msg = format!("context message {i}");
        let (pn, key) = alice_ratchet.next_send_key();
        let aead = AeadKey::new(key);
        let nonce = nonce_from_pn(pn);
        let ct = aead.encrypt(&nonce, msg.as_bytes(), &[]).unwrap();

        let rk = bob_ratchet.key_for_packet(pn).unwrap();
        let raead = AeadKey::new(rk);
        let pt = raead.decrypt(&nonce, &ct, &[]).unwrap();
        assert_eq!(pt, msg.as_bytes());
    }
}
