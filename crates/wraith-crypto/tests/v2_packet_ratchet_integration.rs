//! Integration tests for per-packet ratchet with AEAD.

use wraith_crypto::aead::{AeadKey, NONCE_SIZE, Nonce};
use wraith_crypto::packet_ratchet::PacketRatchet;

#[test]
fn derive_n_send_keys_all_unique() {
    let mut ratchet = PacketRatchet::new([0x42u8; 32]);
    let mut keys = Vec::new();

    for _ in 0..50 {
        let (_pn, key) = ratchet.next_send_key();
        // Ensure no duplicates
        assert!(
            !keys.contains(&key),
            "duplicate key found in ratchet sequence"
        );
        keys.push(key);
    }
}

#[test]
fn two_ratchets_same_seed_produce_identical_sequences() {
    let seed = [0xABu8; 32];
    let mut r1 = PacketRatchet::new(seed);
    let mut r2 = PacketRatchet::new(seed);

    for _ in 0..100 {
        let (pn1, k1) = r1.next_send_key();
        let (pn2, k2) = r2.next_send_key();
        assert_eq!(pn1, pn2);
        assert_eq!(k1, k2);
    }
}

#[test]
fn forward_secrecy_after_advancing() {
    let seed = [0x42u8; 32];
    let mut recv = PacketRatchet::new(seed);

    // Consume packet 0
    let _k0 = recv.key_for_packet(0).unwrap();
    // Consume packet 1
    let _k1 = recv.key_for_packet(1).unwrap();

    // Packet 0 key is gone -- forward secrecy
    assert!(recv.key_for_packet(0).is_err());
    // Packet 1 key is also gone
    assert!(recv.key_for_packet(1).is_err());
}

#[test]
fn out_of_order_delivery_all_decrypt() {
    let seed = [0x42u8; 32];
    let mut sender = PacketRatchet::new(seed);

    // Generate keys 0,1,2,3
    let (_, k0) = sender.next_send_key();
    let (_, k1) = sender.next_send_key();
    let (_, k2) = sender.next_send_key();
    let (_, k3) = sender.next_send_key();

    // Receive in order 3,0,2,1
    let mut recv = PacketRatchet::new(seed);
    assert_eq!(recv.key_for_packet(3).unwrap(), k3);
    assert_eq!(recv.key_for_packet(0).unwrap(), k0);
    assert_eq!(recv.key_for_packet(2).unwrap(), k2);
    assert_eq!(recv.key_for_packet(1).unwrap(), k1);
}

#[test]
fn window_overflow_returns_error() {
    let mut r = PacketRatchet::with_window_size([0x42u8; 32], 10);
    // Request packet far beyond window
    assert!(r.key_for_packet(100).is_err());
}

#[test]
fn ratchet_plus_aead_integration() {
    let seed = [0x42u8; 32];
    let mut sender_ratchet = PacketRatchet::new(seed);
    let mut recv_ratchet = PacketRatchet::new(seed);

    let plaintext = b"secret message via ratchet + AEAD";

    // Sender derives key and encrypts
    let (pn, send_key) = sender_ratchet.next_send_key();
    let aead_key = AeadKey::new(send_key);
    let nonce = Nonce::from_bytes([0u8; NONCE_SIZE]);
    let ciphertext = aead_key.encrypt(&nonce, plaintext, &[]).unwrap();

    // Receiver derives key for same packet number and decrypts
    let recv_key = recv_ratchet.key_for_packet(pn).unwrap();
    let recv_aead_key = AeadKey::new(recv_key);
    let decrypted = recv_aead_key.decrypt(&nonce, &ciphertext, &[]).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn thousand_packet_simulation_sender_receiver_sync() {
    let seed = [0xFFu8; 32];
    let mut sender = PacketRatchet::new(seed);
    let mut recv = PacketRatchet::new(seed);

    for i in 0u64..1000 {
        let (pn, sk) = sender.next_send_key();
        assert_eq!(pn, i);
        let rk = recv.key_for_packet(pn).unwrap();
        assert_eq!(sk, rk);
    }
}
