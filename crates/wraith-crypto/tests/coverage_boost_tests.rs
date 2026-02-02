//! Tests targeting low-coverage modules: pq, random, lib.rs, aead/cipher.

// ---------------------------------------------------------------------------
// pq.rs -- post-quantum KEM
// ---------------------------------------------------------------------------
mod pq_tests {
    use wraith_crypto::pq;

    #[test]
    fn keypair_encapsulate_decapsulate_roundtrip() {
        let mut rng = rand_core::OsRng;
        let (pk, sk) = pq::generate_keypair(&mut rng);
        let (ct, ss_enc) = pq::encapsulate(&mut rng, &pk);
        let ss_dec = pq::decapsulate(&sk, &ct);
        assert_eq!(ss_enc, ss_dec);
    }

    #[test]
    fn different_keypairs_produce_different_secrets() {
        let mut rng = rand_core::OsRng;
        let (pk1, _sk1) = pq::generate_keypair(&mut rng);
        let (pk2, _sk2) = pq::generate_keypair(&mut rng);

        let (_ct1, ss1) = pq::encapsulate(&mut rng, &pk1);
        let (_ct2, ss2) = pq::encapsulate(&mut rng, &pk2);
        // Overwhelmingly likely to differ
        assert_ne!(ss1, ss2);
    }

    #[test]
    fn public_key_serialization_roundtrip() {
        let mut rng = rand_core::OsRng;
        let (pk, _sk) = pq::generate_keypair(&mut rng);
        let bytes = pq::public_key_to_vec(&pk);
        let pk2 = pq::public_key_from_bytes(&bytes).expect("should parse");
        assert_eq!(pq::public_key_to_vec(&pk2), bytes);
    }

    #[test]
    fn public_key_from_invalid_bytes() {
        let err = pq::public_key_from_bytes(&[0u8; 10]);
        assert!(err.is_err());
        let e = err.unwrap_err();
        assert_eq!(e, pq::PqParseError);
        // Display impl
        assert_eq!(std::format!("{e}"), "invalid post-quantum key material");
    }

    #[test]
    fn ciphertext_serialization_roundtrip() {
        let mut rng = rand_core::OsRng;
        let (pk, _sk) = pq::generate_keypair(&mut rng);
        let (ct, _ss) = pq::encapsulate(&mut rng, &pk);
        let bytes = pq::ciphertext_to_vec(&ct);
        let ct2 = pq::ciphertext_from_bytes(&bytes).expect("should parse");
        assert_eq!(pq::ciphertext_to_vec(&ct2), bytes);
    }

    #[test]
    fn ciphertext_from_invalid_bytes() {
        assert!(pq::ciphertext_from_bytes(&[0u8; 5]).is_err());
    }

    #[test]
    fn pq_parse_error_debug_clone_copy_eq() {
        let e1 = pq::PqParseError;
        let e2 = e1; // Copy
        let e3 = e1; // Clone (Copy implies Clone)
        assert_eq!(e1, e2);
        assert_eq!(e2, e3);
        let _ = std::format!("{e1:?}"); // Debug
    }
}

// ---------------------------------------------------------------------------
// random.rs -- SecureRng, fill_random, random_32, random_8
// ---------------------------------------------------------------------------
mod random_tests {
    use rand_core::RngCore;
    use wraith_crypto::random::{self, SecureRng};

    #[test]
    fn secure_rng_new_and_default() {
        let _r1 = SecureRng::new();
        let _r2 = SecureRng;
    }

    #[test]
    fn secure_rng_next_u32() {
        let mut rng = SecureRng::new();
        // Just ensure it doesn't panic and produces a value
        let _ = rng.next_u32();
    }

    #[test]
    fn secure_rng_next_u64() {
        let mut rng = SecureRng::new();
        let _ = rng.next_u64();
    }

    #[test]
    fn secure_rng_fill_bytes() {
        let mut rng = SecureRng::new();
        let mut buf = [0u8; 64];
        rng.fill_bytes(&mut buf);
        // Extremely unlikely to remain all zeros
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn secure_rng_try_fill_bytes() {
        let mut rng = SecureRng::new();
        let mut buf = [0u8; 32];
        rng.try_fill_bytes(&mut buf).expect("should succeed");
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn fill_random_works() {
        let mut buf = [0u8; 64];
        random::fill_random(&mut buf).unwrap();
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn fill_random_empty_buffer() {
        let mut buf = [0u8; 0];
        random::fill_random(&mut buf).unwrap();
    }

    #[test]
    fn random_32_returns_32_bytes() {
        let r = random::random_32().unwrap();
        assert_eq!(r.len(), 32);
        assert!(r.iter().any(|&b| b != 0));
    }

    #[test]
    fn random_8_returns_8_bytes() {
        let r = random::random_8().unwrap();
        assert_eq!(r.len(), 8);
    }

    #[test]
    fn random_32_produces_unique_values() {
        let a = random::random_32().unwrap();
        let b = random::random_32().unwrap();
        assert_ne!(a, b);
    }
}

// ---------------------------------------------------------------------------
// lib.rs -- SessionKeys, constants
// ---------------------------------------------------------------------------
mod lib_tests {
    use wraith_crypto::SessionKeys;

    #[test]
    fn session_keys_derive_connection_id() {
        let sk = SessionKeys {
            send_key: [0xAA; 32],
            recv_key: [0xBB; 32],
            chain_key: [0xCC; 32],
        };
        let cid = sk.derive_connection_id();
        assert_eq!(cid.len(), 8);
    }

    #[test]
    fn session_keys_different_chain_keys_produce_different_ids() {
        let sk1 = SessionKeys {
            send_key: [0; 32],
            recv_key: [0; 32],
            chain_key: [1; 32],
        };
        let sk2 = SessionKeys {
            send_key: [0; 32],
            recv_key: [0; 32],
            chain_key: [2; 32],
        };
        assert_ne!(sk1.derive_connection_id(), sk2.derive_connection_id());
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(wraith_crypto::X25519_PUBLIC_KEY_SIZE, 32);
        assert_eq!(wraith_crypto::X25519_SECRET_KEY_SIZE, 32);
        assert_eq!(wraith_crypto::ELLIGATOR_REPR_SIZE, 32);
        assert_eq!(wraith_crypto::XCHACHA_KEY_SIZE, 32);
        assert_eq!(wraith_crypto::XCHACHA_NONCE_SIZE, 24);
        assert_eq!(wraith_crypto::BLAKE3_OUTPUT_SIZE, 32);
        assert_eq!(wraith_crypto::ED25519_PUBLIC_KEY_SIZE, 32);
        assert_eq!(wraith_crypto::ED25519_SECRET_KEY_SIZE, 32);
        assert_eq!(wraith_crypto::ED25519_SIGNATURE_SIZE, 64);
    }
}

// ---------------------------------------------------------------------------
// aead/cipher.rs -- additional coverage for uncovered paths
// ---------------------------------------------------------------------------
mod aead_cipher_tests {
    use wraith_crypto::aead::cipher::CachedAeadCipher;
    use wraith_crypto::aead::{AeadKey, KEY_SIZE, NONCE_SIZE, Nonce, TAG_SIZE, Tag};

    #[test]
    fn aead_key_from_slice_valid() {
        let bytes = [0x42u8; KEY_SIZE];
        let key = AeadKey::from_slice(&bytes).unwrap();
        assert_eq!(key.as_bytes(), &bytes);
    }

    #[test]
    fn aead_key_from_slice_too_short() {
        let err = AeadKey::from_slice(&[0u8; 16]).err().unwrap();
        let msg = std::format!("{err}");
        assert!(msg.contains("invalid key length"));
    }

    #[test]
    fn aead_key_from_slice_too_long() {
        let err = AeadKey::from_slice(&[0u8; 64]).err().unwrap();
        let msg = std::format!("{err}");
        assert!(msg.contains("64"));
    }

    #[test]
    fn aead_key_from_slice_empty() {
        assert!(AeadKey::from_slice(&[]).is_err());
    }

    #[test]
    fn nonce_from_slice_valid() {
        let bytes = [0x11u8; NONCE_SIZE];
        let n = Nonce::from_slice(&bytes).unwrap();
        assert_eq!(*n.as_bytes(), bytes);
    }

    #[test]
    fn nonce_from_slice_wrong_size() {
        assert!(Nonce::from_slice(&[0u8; 10]).is_none());
        assert!(Nonce::from_slice(&[0u8; 25]).is_none());
        assert!(Nonce::from_slice(&[]).is_none());
    }

    #[test]
    fn nonce_default_is_zero() {
        let n = Nonce::default();
        assert_eq!(*n.as_bytes(), [0u8; NONCE_SIZE]);
    }

    #[test]
    fn nonce_from_bytes() {
        let bytes = [0xFFu8; NONCE_SIZE];
        let n = Nonce::from_bytes(bytes);
        assert_eq!(*n.as_bytes(), bytes);
    }

    #[test]
    fn tag_from_slice_valid() {
        let bytes = [0xAA; TAG_SIZE];
        let t = Tag::from_slice(&bytes).unwrap();
        assert_eq!(*t.as_bytes(), bytes);
    }

    #[test]
    fn tag_from_slice_wrong_size() {
        assert!(Tag::from_slice(&[0u8; 10]).is_none());
        assert!(Tag::from_slice(&[]).is_none());
        assert!(Tag::from_slice(&[0u8; 17]).is_none());
    }

    #[test]
    fn tag_from_bytes() {
        let bytes = [0xBB; TAG_SIZE];
        let t = Tag::from_bytes(bytes);
        assert_eq!(*t.as_bytes(), bytes);
    }

    #[test]
    fn encrypt_empty_plaintext() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let ct = key.encrypt(&nonce, &[], b"aad").unwrap();
        assert_eq!(ct.len(), TAG_SIZE); // only tag, no data
        let pt = key.decrypt(&nonce, &ct, b"aad").unwrap();
        assert!(pt.is_empty());
    }

    #[test]
    fn decrypt_too_short_ciphertext() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        // Less than TAG_SIZE bytes should fail fast
        let err = key.decrypt(&nonce, &[0u8; 15], b"");
        assert!(err.is_err());
    }

    #[test]
    fn decrypt_exactly_tag_size_invalid() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        // 16 random bytes won't be a valid tag for empty plaintext
        let err = key.decrypt(&nonce, &[0xFFu8; TAG_SIZE], b"");
        assert!(err.is_err());
    }

    #[test]
    fn encrypt_in_place_empty_buffer() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let mut buf = vec![];
        let tag = key.encrypt_in_place(&nonce, &mut buf, b"aad").unwrap();
        key.decrypt_in_place(&nonce, &mut buf, &tag, b"aad")
            .unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn decrypt_in_place_wrong_tag_fails() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let mut buf = b"hello world".to_vec();
        let _tag = key.encrypt_in_place(&nonce, &mut buf, b"").unwrap();
        let bad_tag = Tag::from_bytes([0xFF; TAG_SIZE]);
        assert!(
            key.decrypt_in_place(&nonce, &mut buf, &bad_tag, b"")
                .is_err()
        );
    }

    #[test]
    fn decrypt_in_place_wrong_aad_fails() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let mut buf = b"hello world".to_vec();
        let tag = key.encrypt_in_place(&nonce, &mut buf, b"aad1").unwrap();
        assert!(
            key.decrypt_in_place(&nonce, &mut buf, &tag, b"aad2")
                .is_err()
        );
    }

    #[test]
    fn key_commitment_deterministic() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let c1 = key.commitment();
        let c2 = key.commitment();
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_keys_different_commitments() {
        let k1 = AeadKey::new([0x01; KEY_SIZE]);
        let k2 = AeadKey::new([0x02; KEY_SIZE]);
        assert_ne!(k1.commitment(), k2.commitment());
    }

    // CachedAeadCipher tests
    #[test]
    fn cached_cipher_roundtrip() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let cipher = CachedAeadCipher::new(&key);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let ct = cipher.encrypt(&nonce, b"hello", b"aad").unwrap();
        let pt = cipher.decrypt(&nonce, &ct, b"aad").unwrap();
        assert_eq!(pt, b"hello");
    }

    #[test]
    fn cached_cipher_from_bytes() {
        let cipher = CachedAeadCipher::from_bytes(&[0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let ct = cipher.encrypt(&nonce, b"data", b"").unwrap();
        let pt = cipher.decrypt(&nonce, &ct, b"").unwrap();
        assert_eq!(pt, b"data");
    }

    #[test]
    fn cached_cipher_decrypt_too_short() {
        let cipher = CachedAeadCipher::from_bytes(&[0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        assert!(cipher.decrypt(&nonce, &[0u8; 10], b"").is_err());
    }

    #[test]
    fn cached_cipher_decrypt_tampered() {
        let cipher = CachedAeadCipher::from_bytes(&[0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let mut ct = cipher.encrypt(&nonce, b"secret", b"").unwrap();
        ct[0] ^= 0xFF;
        assert!(cipher.decrypt(&nonce, &ct, b"").is_err());
    }

    #[test]
    fn cached_cipher_empty_plaintext() {
        let cipher = CachedAeadCipher::from_bytes(&[0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let ct = cipher.encrypt(&nonce, &[], b"").unwrap();
        assert_eq!(ct.len(), TAG_SIZE);
        let pt = cipher.decrypt(&nonce, &ct, b"").unwrap();
        assert!(pt.is_empty());
    }

    #[test]
    fn large_plaintext_encrypt_decrypt() {
        let key = AeadKey::new([0x42; KEY_SIZE]);
        let nonce = Nonce::from_bytes([0; NONCE_SIZE]);
        let big = vec![0xABu8; 65536];
        let ct = key.encrypt(&nonce, &big, b"").unwrap();
        let pt = key.decrypt(&nonce, &ct, b"").unwrap();
        assert_eq!(pt, big);
    }

    // Nonce equality / clone / debug
    #[test]
    fn nonce_eq_and_debug() {
        let n1 = Nonce::from_bytes([1; NONCE_SIZE]);
        let n2 = Nonce::from_bytes([1; NONCE_SIZE]);
        let n3 = Nonce::from_bytes([2; NONCE_SIZE]);
        assert_eq!(n1, n2);
        assert_ne!(n1, n3);
        let _ = std::format!("{n1:?}");
    }

    // Tag equality / clone / debug
    #[test]
    fn tag_eq_and_debug() {
        let t1 = Tag::from_bytes([1; TAG_SIZE]);
        let t2 = Tag::from_bytes([1; TAG_SIZE]);
        let t3 = Tag::from_bytes([2; TAG_SIZE]);
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
        let _ = std::format!("{t1:?}");
    }
}
