//! Integration tests for KDF v2 with real crypto.

use rand_core::OsRng;
use wraith_crypto::hybrid::HybridKeyPair;
use wraith_crypto::kdf::{derive_session_keys_v2, derive_stream_key};
use wraith_crypto::packet_ratchet::PacketRatchet;

#[test]
fn derive_session_keys_from_hybrid_shared_secret() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss, ct) = kp.public.encapsulate(&mut OsRng).unwrap();
    let ss_dec = kp.secret.decapsulate(&ct).unwrap();
    assert_eq!(ss.as_bytes(), ss_dec.as_bytes());

    let transcript = blake3::hash(b"handshake transcript data");
    let keys = derive_session_keys_v2(ss.as_bytes(), transcript.as_bytes());

    assert_ne!(keys.initiator_to_responder, [0u8; 32]);
    assert_ne!(keys.responder_to_initiator, [0u8; 32]);
    assert_ne!(keys.format_key, [0u8; 32]);
    assert_ne!(keys.initial_chain_key, [0u8; 32]);
}

#[test]
fn directional_keys_are_different() {
    let secret = [0x42u8; 32];
    let transcript = [0x43u8; 32];
    let keys = derive_session_keys_v2(&secret, &transcript);

    assert_ne!(keys.initiator_to_responder, keys.responder_to_initiator);
}

#[test]
fn same_inputs_produce_same_outputs() {
    let secret = [0x42u8; 32];
    let transcript = [0x43u8; 32];

    let k1 = derive_session_keys_v2(&secret, &transcript);
    let k2 = derive_session_keys_v2(&secret, &transcript);

    assert_eq!(k1.initiator_to_responder, k2.initiator_to_responder);
    assert_eq!(k1.responder_to_initiator, k2.responder_to_initiator);
    assert_eq!(k1.format_key, k2.format_key);
    assert_eq!(k1.initial_chain_key, k2.initial_chain_key);
}

#[test]
fn different_transcript_hashes_produce_different_keys() {
    let secret = [0x42u8; 32];
    let k1 = derive_session_keys_v2(&secret, &[0x01u8; 32]);
    let k2 = derive_session_keys_v2(&secret, &[0x02u8; 32]);

    assert_ne!(k1.initiator_to_responder, k2.initiator_to_responder);
    assert_ne!(k1.responder_to_initiator, k2.responder_to_initiator);
    assert_ne!(k1.format_key, k2.format_key);
    assert_ne!(k1.initial_chain_key, k2.initial_chain_key);
}

#[test]
fn stream_key_derivation_different_ids_produce_different_keys() {
    let traffic_key = [0x42u8; 32];
    let sk0 = derive_stream_key(&traffic_key, 0);
    let sk1 = derive_stream_key(&traffic_key, 1);
    let sk2 = derive_stream_key(&traffic_key, 2);
    let sk_max = derive_stream_key(&traffic_key, u32::MAX);

    assert_ne!(sk0, sk1);
    assert_ne!(sk1, sk2);
    assert_ne!(sk0, sk2);
    assert_ne!(sk0, sk_max);
}

#[test]
fn session_keys_fed_into_packet_ratchet_produce_working_ratchet() {
    let kp = HybridKeyPair::generate(&mut OsRng);
    let (ss, _ct) = kp.public.encapsulate(&mut OsRng).unwrap();
    let transcript = blake3::hash(b"transcript");
    let keys = derive_session_keys_v2(ss.as_bytes(), transcript.as_bytes());

    // Create send and receive ratchets from the same chain key
    let mut send_ratchet = PacketRatchet::new(keys.initial_chain_key);
    let mut recv_ratchet = PacketRatchet::new(keys.initial_chain_key);

    // Verify 100 packets stay in sync
    for i in 0u64..100 {
        let (pn, sk) = send_ratchet.next_send_key();
        assert_eq!(pn, i);
        let rk = recv_ratchet.key_for_packet(pn).unwrap();
        assert_eq!(sk, rk);
    }
}
