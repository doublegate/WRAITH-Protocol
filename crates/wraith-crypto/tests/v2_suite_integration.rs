//! Integration tests for suite negotiation and CryptoContextV2 behavior.

use rand_core::OsRng;
use wraith_crypto::context::{CryptoContextV2, EncapsulationResult};
use wraith_crypto::suite::CryptoSuite;

#[test]
fn negotiate_both_support_all_suites_selects_strongest() {
    let all = CryptoSuite::all();
    let result = CryptoSuite::negotiate(all, all);
    // SuiteC is strongest in priority order
    assert_eq!(result, Some(CryptoSuite::SuiteC));
}

#[test]
fn negotiate_one_side_classical_only_selects_suite_d() {
    let local = CryptoSuite::all().to_vec();
    let remote = [CryptoSuite::SuiteD];
    let result = CryptoSuite::negotiate(&local, &remote);
    assert_eq!(result, Some(CryptoSuite::SuiteD));
}

#[test]
fn negotiate_no_overlap_returns_none() {
    let local = [CryptoSuite::SuiteA];
    let remote = [CryptoSuite::SuiteD];
    let result = CryptoSuite::negotiate(&local, &remote);
    assert_eq!(result, None);
}

#[test]
fn suite_a_context_uses_hybrid_encapsulation() {
    let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
    let kp = ctx.generate_keypair(&mut OsRng);
    let (ss_enc, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();

    assert!(matches!(result, EncapsulationResult::Hybrid(_)));

    if let EncapsulationResult::Hybrid(ct) = result {
        let ss_dec = kp.secret.decapsulate(&ct).unwrap();
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }
}

#[test]
fn suite_d_context_uses_classical_only_encapsulation() {
    let ctx = CryptoContextV2::new(CryptoSuite::SuiteD);
    let kp = ctx.generate_keypair(&mut OsRng);
    let (ss_enc, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();

    assert!(matches!(result, EncapsulationResult::ClassicalOnly(_)));

    if let EncapsulationResult::ClassicalOnly(epk) = result {
        let ss_dec = kp.secret.decapsulate_classical_only(&epk).unwrap();
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }
}

#[test]
fn suite_b_context_uses_hybrid_encapsulation() {
    let ctx = CryptoContextV2::new(CryptoSuite::SuiteB);
    assert!(ctx.suite().supports_post_quantum());
    let kp = ctx.generate_keypair(&mut OsRng);
    let (_ss, result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();
    assert!(matches!(result, EncapsulationResult::Hybrid(_)));
}

#[test]
fn suite_affects_encapsulation_result_size() {
    let ctx_a = CryptoContextV2::new(CryptoSuite::SuiteA);
    let ctx_d = CryptoContextV2::new(CryptoSuite::SuiteD);

    let kp_a = ctx_a.generate_keypair(&mut OsRng);
    let kp_d = ctx_d.generate_keypair(&mut OsRng);

    let (_ss_a, result_a) = ctx_a.encapsulate(&mut OsRng, &kp_a.public).unwrap();
    let (_ss_d, result_d) = ctx_d.encapsulate(&mut OsRng, &kp_d.public).unwrap();

    let bytes_a = result_a.to_bytes();
    let bytes_d = result_d.to_bytes();

    // Hybrid ciphertext is much larger than classical (32 + 1088 vs 32)
    assert!(bytes_a.len() > bytes_d.len());
    assert_eq!(bytes_d.len(), 32);
}

#[test]
fn context_derive_session_keys_and_create_ratchet() {
    let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);
    let kp = ctx.generate_keypair(&mut OsRng);
    let (ss, _result) = ctx.encapsulate(&mut OsRng, &kp.public).unwrap();

    let transcript = blake3::hash(b"suite integration test transcript");
    let keys = ctx.derive_session_keys(ss.as_bytes(), transcript.as_bytes());

    let mut ratchet = ctx.create_packet_ratchet(keys.initial_chain_key);
    assert_eq!(ratchet.packet_number(), 0);

    let (pn, key) = ratchet.next_send_key();
    assert_eq!(pn, 0);
    assert_ne!(key, [0u8; 32]);
}
