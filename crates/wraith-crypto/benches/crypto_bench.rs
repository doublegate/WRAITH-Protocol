//! Performance benchmarks for wraith-crypto.
//!
//! Run with: `cargo bench -p wraith-crypto`
//!
//! Target performance metrics:
//! - AEAD encryption: >3 GB/s (single core)
//! - Noise handshake: <50ms (full XX)
//! - Key ratcheting: >10M ops/sec
//!
//! SECURITY NOTE: All hard-coded cryptographic values in this file are intentional
//! test data for benchmarking, NOT production keys.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::RngCore;
use rand_core::OsRng;
use std::hint::black_box;
use wraith_crypto::aead::{AeadKey, Nonce};
use wraith_crypto::hash::{Kdf, hash, hkdf_expand, hkdf_extract};
use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
use wraith_crypto::ratchet::{DoubleRatchet, MessageHeader, SymmetricRatchet};
use wraith_crypto::x25519::PrivateKey;

// ============================================================================
// AEAD Benchmarks
// ============================================================================

fn bench_aead_encrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("aead_encrypt");

    // Test various message sizes
    let sizes = [64, 256, 1024, 4096, 16384, 65536];

    for size in sizes {
        let key_bytes = [0x42u8; 32];
        let key = AeadKey::new(key_bytes);
        let nonce = Nonce::from_bytes([0u8; 24]);
        let aad = b"additional data";
        let plaintext = vec![0xAA; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| key.encrypt(black_box(&nonce), black_box(&plaintext), black_box(aad)))
        });
    }

    group.finish();
}

fn bench_aead_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("aead_decrypt");

    let sizes = [64, 256, 1024, 4096, 16384, 65536];

    for size in sizes {
        let key_bytes = [0x42u8; 32];
        let key = AeadKey::new(key_bytes);
        let nonce = Nonce::from_bytes([0u8; 24]);
        let aad = b"additional data";
        let plaintext = vec![0xAA; size];

        // Pre-encrypt for decryption benchmark
        let ciphertext = key.encrypt(&nonce, &plaintext, aad).unwrap();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| key.decrypt(black_box(&nonce), black_box(&ciphertext), black_box(aad)))
        });
    }

    group.finish();
}

fn bench_aead_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("aead_roundtrip");

    // Focus on typical MTU sizes
    let sizes = [1200, 1400, 4096];

    for size in sizes {
        let key_bytes = [0x42u8; 32];
        let key = AeadKey::new(key_bytes);
        let nonce = Nonce::from_bytes([0u8; 24]);
        let aad = b"wraith-frame-aad";
        let plaintext = vec![0xBB; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let ct = key
                    .encrypt(black_box(&nonce), black_box(&plaintext), black_box(aad))
                    .unwrap();
                key.decrypt(black_box(&nonce), black_box(&ct), black_box(aad))
            })
        });
    }

    group.finish();
}

fn bench_aead_encrypt_in_place(c: &mut Criterion) {
    let mut group = c.benchmark_group("aead_encrypt_in_place");

    let sizes = [64, 256, 1024, 4096, 16384];

    for size in sizes {
        let key_bytes = [0x42u8; 32];
        let key = AeadKey::new(key_bytes);
        let nonce = Nonce::from_bytes([0u8; 24]);
        let aad = b"additional data";

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &sz| {
            b.iter_batched(
                || vec![0xAA; sz],
                |mut buffer| {
                    let _tag = key
                        .encrypt_in_place(black_box(&nonce), black_box(&mut buffer), black_box(aad))
                        .unwrap();
                    black_box(buffer)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_aead_decrypt_in_place(c: &mut Criterion) {
    use wraith_crypto::aead::Tag;

    let mut group = c.benchmark_group("aead_decrypt_in_place");

    let sizes = [64, 256, 1024, 4096, 16384];

    for size in sizes {
        let key_bytes = [0x42u8; 32];
        let key = AeadKey::new(key_bytes);
        let nonce = Nonce::from_bytes([0u8; 24]);
        let aad = b"additional data";

        // Pre-encrypt to get ciphertext and tag
        let mut plaintext = vec![0xAA; size];
        let tag = key.encrypt_in_place(&nonce, &mut plaintext, aad).unwrap();
        let ciphertext = plaintext; // now encrypted
        let tag_bytes = *tag.as_bytes();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter_batched(
                || (ciphertext.clone(), Tag::from_bytes(tag_bytes)),
                |(mut buffer, tag)| {
                    key.decrypt_in_place(
                        black_box(&nonce),
                        black_box(&mut buffer),
                        black_box(&tag),
                        black_box(aad),
                    )
                    .unwrap();
                    black_box(buffer)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// ============================================================================
// X25519 Benchmarks
// ============================================================================

fn bench_x25519_keygen(c: &mut Criterion) {
    c.bench_function("x25519_keygen", |b| {
        b.iter(|| PrivateKey::generate(&mut OsRng))
    });
}

fn bench_x25519_exchange(c: &mut Criterion) {
    let alice_private = PrivateKey::generate(&mut OsRng);
    let bob_private = PrivateKey::generate(&mut OsRng);
    let bob_public = bob_private.public_key();

    c.bench_function("x25519_exchange", |b| {
        b.iter(|| alice_private.exchange(black_box(&bob_public)))
    });
}

// ============================================================================
// BLAKE3 Benchmarks
// ============================================================================

fn bench_blake3_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3_hash");

    let sizes = [32, 256, 1024, 4096, 65536];

    for size in sizes {
        let data = vec![0xCC; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| hash(black_box(&data)))
        });
    }

    group.finish();
}

fn bench_hkdf(c: &mut Criterion) {
    let ikm = [0x42u8; 32];
    let salt = [0xABu8; 32];
    let info = b"wraith-key-derivation";

    c.bench_function("hkdf_extract", |b| {
        b.iter(|| hkdf_extract(black_box(&salt), black_box(&ikm)))
    });

    let prk = hkdf_extract(&salt, &ikm);
    let mut output = [0u8; 32];
    c.bench_function("hkdf_expand", |b| {
        b.iter(|| hkdf_expand(black_box(&prk), black_box(info), &mut output))
    });

    c.bench_function("hkdf_full", |b| {
        b.iter(|| {
            let prk = hkdf_extract(black_box(&salt), black_box(&ikm));
            let mut out = [0u8; 32];
            hkdf_expand(black_box(&prk), black_box(info), &mut out);
            out
        })
    });
}

fn bench_kdf(c: &mut Criterion) {
    let ikm = [0x42u8; 32];
    let kdf = Kdf::new("wraith-benchmark-context");

    c.bench_function("kdf_derive_key", |b| {
        b.iter(|| kdf.derive_key(black_box(&ikm)))
    });
}

// ============================================================================
// Noise Handshake Benchmarks
// ============================================================================

fn bench_noise_keypair_generation(c: &mut Criterion) {
    c.bench_function("noise_keypair_generate", |b| b.iter(NoiseKeypair::generate));
}

fn bench_noise_full_handshake(c: &mut Criterion) {
    c.bench_function("noise_xx_handshake", |b| {
        b.iter(|| {
            let alice_static = NoiseKeypair::generate().unwrap();
            let bob_static = NoiseKeypair::generate().unwrap();

            let mut alice = NoiseHandshake::new_initiator(&alice_static).unwrap();
            let mut bob = NoiseHandshake::new_responder(&bob_static).unwrap();

            // Message 1: -> e
            let msg1 = alice.write_message(&[]).unwrap();
            bob.read_message(&msg1).unwrap();

            // Message 2: <- e, ee, s, es
            let msg2 = bob.write_message(&[]).unwrap();
            alice.read_message(&msg2).unwrap();

            // Message 3: -> s, se
            let msg3 = alice.write_message(&[]).unwrap();
            bob.read_message(&msg3).unwrap();

            // Get session keys
            black_box(alice.into_session_keys().unwrap());
            black_box(bob.into_session_keys().unwrap());
        })
    });
}

fn bench_noise_message_write(c: &mut Criterion) {
    let alice_static = NoiseKeypair::generate().unwrap();

    // Benchmark just the first message write
    c.bench_function("noise_write_message_1", |b| {
        b.iter(|| {
            let mut alice = NoiseHandshake::new_initiator(&alice_static).unwrap();
            let m1 = alice.write_message(&[]).unwrap();
            black_box(m1)
        })
    });
}

// ============================================================================
// Key Ratcheting Benchmarks
// ============================================================================

fn bench_symmetric_ratchet(c: &mut Criterion) {
    let initial_key = [0x42u8; 32];

    c.bench_function("symmetric_ratchet_step", |b| {
        b.iter_batched(
            || SymmetricRatchet::new(&initial_key),
            |mut ratchet| {
                let key = ratchet.next_key();
                black_box(key)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_double_ratchet_init(c: &mut Criterion) {
    let shared_secret = [0x42u8; 32];
    let bob_dh = PrivateKey::generate(&mut OsRng);
    let bob_dh_public = bob_dh.public_key();

    c.bench_function("double_ratchet_init_initiator", |b| {
        b.iter(|| {
            DoubleRatchet::new_initiator(
                &mut OsRng,
                black_box(&shared_secret),
                black_box(bob_dh_public),
            )
        })
    });

    c.bench_function("double_ratchet_init_responder", |b| {
        b.iter(|| {
            let bob_key = PrivateKey::generate(&mut OsRng);
            DoubleRatchet::new_responder(black_box(&shared_secret), bob_key)
        })
    });
}

fn bench_double_ratchet_encrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("double_ratchet_encrypt");

    let sizes = [64, 256, 1024, 4096];
    let shared_secret = [0x42u8; 32];

    for size in sizes {
        let plaintext = vec![0xAA; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter_batched(
                || {
                    let bob_dh = PrivateKey::generate(&mut OsRng);
                    let bob_dh_public = bob_dh.public_key();
                    DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public)
                },
                |mut alice| alice.encrypt(&mut OsRng, black_box(&plaintext)),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_double_ratchet_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("double_ratchet_decrypt");

    let sizes = [64, 256, 1024, 4096];
    let shared_secret = [0x42u8; 32];

    for size in sizes {
        let plaintext = vec![0xAA; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter_batched(
                || {
                    let bob_dh = PrivateKey::generate(&mut OsRng);
                    let bob_dh_public = bob_dh.public_key();
                    let mut alice =
                        DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
                    let bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);
                    let (header, ct) = alice.encrypt(&mut OsRng, &plaintext).unwrap();
                    (bob, header, ct)
                },
                |(mut bob, header, ct)| black_box(bob.decrypt(&mut OsRng, &header, &ct)),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_double_ratchet_roundtrip(c: &mut Criterion) {
    let shared_secret = [0x42u8; 32];
    let plaintext = vec![0xAA; 1024];

    c.bench_function("double_ratchet_roundtrip_1k", |b| {
        b.iter_batched(
            || {
                let bob_key = PrivateKey::generate(&mut OsRng);
                let bob_pub = bob_key.public_key();
                let alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_pub);
                let bob = DoubleRatchet::new_responder(&shared_secret, bob_key);
                (alice, bob)
            },
            |(mut alice, mut bob)| {
                let (header, ct) = alice.encrypt(&mut OsRng, black_box(&plaintext)).unwrap();
                bob.decrypt(&mut OsRng, black_box(&header), black_box(&ct))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_message_header_serialize(c: &mut Criterion) {
    let dh_public = PrivateKey::generate(&mut OsRng).public_key();
    let header = MessageHeader {
        dh_public,
        prev_chain_length: 100,
        message_number: 42,
    };

    c.bench_function("message_header_serialize", |b| {
        b.iter(|| black_box(&header).to_bytes())
    });

    let bytes = header.to_bytes();
    c.bench_function("message_header_deserialize", |b| {
        b.iter(|| MessageHeader::from_bytes(black_box(&bytes)))
    });
}

// ============================================================================
// Replay Protection Benchmarks
// ============================================================================

fn bench_replay_protection(c: &mut Criterion) {
    use wraith_crypto::aead::ReplayProtection;

    let mut group = c.benchmark_group("replay_protection");

    group.bench_function("sequential_accept", |b| {
        b.iter_batched(
            || (ReplayProtection::new(), 0u64),
            |(mut rp, mut seq)| {
                seq += 1;
                let accepted = rp.check_and_update(black_box(seq));
                black_box(accepted)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("replay_reject", |b| {
        b.iter_batched(
            || {
                let mut rp = ReplayProtection::new();
                rp.check_and_update(100); // Insert seq 100
                rp
            },
            |mut rp| {
                let rejected = rp.check_and_update(black_box(100)); // Replay
                black_box(rejected)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// Elligator2 Benchmarks
// ============================================================================

fn bench_elligator_keygen(c: &mut Criterion) {
    use wraith_crypto::elligator::{ElligatorKeypair, generate_encodable_keypair};

    c.bench_function("elligator_generate_keypair", |b| {
        b.iter(|| generate_encodable_keypair(&mut OsRng))
    });

    c.bench_function("elligator_keypair_struct", |b| {
        b.iter(|| ElligatorKeypair::generate(&mut OsRng))
    });
}

fn bench_elligator_decode(c: &mut Criterion) {
    use wraith_crypto::elligator::{
        Representative, decode_representative, generate_encodable_keypair,
    };

    let (_, repr) = generate_encodable_keypair(&mut OsRng);

    c.bench_function("elligator_decode_representative", |b| {
        b.iter(|| decode_representative(black_box(&repr)))
    });

    // Also test decoding arbitrary bytes
    let mut random_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut random_bytes);
    let random_repr = Representative::from_bytes(random_bytes);

    c.bench_function("elligator_decode_random_bytes", |b| {
        b.iter(|| decode_representative(black_box(&random_repr)))
    });
}

fn bench_elligator_exchange(c: &mut Criterion) {
    use wraith_crypto::elligator::ElligatorKeypair;

    let alice = ElligatorKeypair::generate(&mut OsRng);
    let bob = ElligatorKeypair::generate(&mut OsRng);

    c.bench_function("elligator_exchange_representative", |b| {
        b.iter(|| alice.exchange_representative(black_box(bob.representative())))
    });
}

// ============================================================================
// Constant-Time Operations Benchmarks
// ============================================================================

fn bench_constant_time_ops(c: &mut Criterion) {
    use wraith_crypto::constant_time::{ct_eq, ct_select};

    let a = [0x42u8; 32];
    let b = [0x42u8; 32];
    let c_arr = [0xABu8; 32];

    c.bench_function("ct_eq_32_bytes_equal", |b_iter| {
        b_iter.iter(|| ct_eq(black_box(&a), black_box(&b)))
    });

    c.bench_function("ct_eq_32_bytes_unequal", |b_iter| {
        b_iter.iter(|| ct_eq(black_box(&a), black_box(&c_arr)))
    });

    let x = [0x11u8; 8];
    let y = [0x22u8; 8];

    c.bench_function("ct_select_8_bytes", |b_iter| {
        b_iter.iter(|| {
            let mut result = [0u8; 8];
            ct_select(black_box(true), black_box(&x), black_box(&y), &mut result);
            result
        })
    });
}

// ============================================================================
// Hybrid KEM Benchmarks
// ============================================================================

fn bench_hybrid_keygen(c: &mut Criterion) {
    c.bench_function("hybrid_keypair_generate", |b| {
        b.iter(|| wraith_crypto::hybrid::HybridKeyPair::generate(&mut OsRng))
    });
}

fn bench_hybrid_encapsulate(c: &mut Criterion) {
    let kp = wraith_crypto::hybrid::HybridKeyPair::generate(&mut OsRng);

    c.bench_function("hybrid_encapsulate", |b| {
        b.iter(|| kp.public.encapsulate(black_box(&mut OsRng)))
    });
}

fn bench_hybrid_decapsulate(c: &mut Criterion) {
    let kp = wraith_crypto::hybrid::HybridKeyPair::generate(&mut OsRng);
    let (_ss, ct) = kp.public.encapsulate(&mut OsRng).unwrap();

    c.bench_function("hybrid_decapsulate", |b| {
        b.iter(|| kp.secret.decapsulate(black_box(&ct)))
    });
}

fn bench_hybrid_classical_only(c: &mut Criterion) {
    let kp = wraith_crypto::hybrid::HybridKeyPair::generate(&mut OsRng);

    c.bench_function("hybrid_encapsulate_classical_only", |b| {
        b.iter(|| kp.public.encapsulate_classical_only(black_box(&mut OsRng)))
    });

    let (_ss, epk) = kp.public.encapsulate_classical_only(&mut OsRng).unwrap();

    c.bench_function("hybrid_decapsulate_classical_only", |b| {
        b.iter(|| kp.secret.decapsulate_classical_only(black_box(&epk)))
    });
}

fn bench_hybrid_serialization(c: &mut Criterion) {
    let kp = wraith_crypto::hybrid::HybridKeyPair::generate(&mut OsRng);
    let (_ss, ct) = kp.public.encapsulate(&mut OsRng).unwrap();

    let pk_bytes = kp.public.to_bytes();
    let ct_bytes = ct.to_bytes();

    c.bench_function("hybrid_public_key_serialize", |b| {
        b.iter(|| black_box(&kp.public).to_bytes())
    });

    c.bench_function("hybrid_public_key_deserialize", |b| {
        b.iter(|| wraith_crypto::hybrid::HybridPublicKey::from_bytes(black_box(&pk_bytes)))
    });

    c.bench_function("hybrid_ciphertext_serialize", |b| {
        b.iter(|| black_box(&ct).to_bytes())
    });

    c.bench_function("hybrid_ciphertext_deserialize", |b| {
        b.iter(|| wraith_crypto::hybrid::HybridCiphertext::from_bytes(black_box(&ct_bytes)))
    });
}

// ============================================================================
// Per-Packet Ratchet Benchmarks
// ============================================================================

fn bench_packet_ratchet_new(c: &mut Criterion) {
    let chain_key = [0x42u8; 32];

    c.bench_function("packet_ratchet_new", |b| {
        b.iter(|| wraith_crypto::packet_ratchet::PacketRatchet::new(black_box(chain_key)))
    });
}

fn bench_packet_ratchet_next_send_key(c: &mut Criterion) {
    let chain_key = [0x42u8; 32];

    c.bench_function("packet_ratchet_next_send_key", |b| {
        b.iter_batched(
            || wraith_crypto::packet_ratchet::PacketRatchet::new(chain_key),
            |mut ratchet| {
                let result = ratchet.next_send_key();
                black_box(result)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_packet_ratchet_1000_sequential(c: &mut Criterion) {
    let chain_key = [0x42u8; 32];

    c.bench_function("packet_ratchet_1000_sequential_send_keys", |b| {
        b.iter_batched(
            || wraith_crypto::packet_ratchet::PacketRatchet::new(chain_key),
            |mut ratchet| {
                for _ in 0..1000 {
                    black_box(ratchet.next_send_key());
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_packet_ratchet_key_for_packet(c: &mut Criterion) {
    let chain_key = [0x42u8; 32];

    // In-order access
    c.bench_function("packet_ratchet_key_for_packet_in_order", |b| {
        b.iter_batched(
            || wraith_crypto::packet_ratchet::PacketRatchet::new(chain_key),
            |mut ratchet| {
                let key = ratchet.key_for_packet(0).unwrap();
                black_box(key)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Out-of-order: request packet 10 then packet 5 (from cache)
    c.bench_function("packet_ratchet_key_for_packet_out_of_order", |b| {
        b.iter_batched(
            || {
                let mut ratchet = wraith_crypto::packet_ratchet::PacketRatchet::new(chain_key);
                // Advance to packet 10, caching 0..9
                let _ = ratchet.key_for_packet(10).unwrap();
                ratchet
            },
            |mut ratchet| {
                let key = ratchet.key_for_packet(5).unwrap();
                black_box(key)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ============================================================================
// KDF v2 Benchmarks
// ============================================================================

fn bench_kdf_v2(c: &mut Criterion) {
    let secret = [0x42u8; 32];
    let transcript = [0x43u8; 32];

    c.bench_function("kdf_v2_derive_session_keys", |b| {
        b.iter(|| {
            wraith_crypto::kdf::derive_session_keys_v2(black_box(&secret), black_box(&transcript))
        })
    });

    let traffic_key = [0x42u8; 32];

    c.bench_function("kdf_v2_derive_stream_key", |b| {
        b.iter(|| wraith_crypto::kdf::derive_stream_key(black_box(&traffic_key), black_box(42)))
    });
}

// ============================================================================
// Crypto Suite Benchmarks
// ============================================================================

fn bench_crypto_suite_negotiate(c: &mut Criterion) {
    use wraith_crypto::suite::CryptoSuite;

    let local = [
        CryptoSuite::SuiteA,
        CryptoSuite::SuiteB,
        CryptoSuite::SuiteD,
    ];
    let remote = [
        CryptoSuite::SuiteC,
        CryptoSuite::SuiteA,
        CryptoSuite::SuiteD,
    ];

    c.bench_function("crypto_suite_negotiate", |b| {
        b.iter(|| CryptoSuite::negotiate(black_box(&local), black_box(&remote)))
    });
}

// ============================================================================
// CryptoContext v2 Benchmarks
// ============================================================================

fn bench_crypto_context_v2(c: &mut Criterion) {
    use wraith_crypto::context::CryptoContextV2;
    use wraith_crypto::suite::CryptoSuite;

    let ctx = CryptoContextV2::new(CryptoSuite::SuiteA);

    c.bench_function("crypto_context_v2_generate_keypair", |b| {
        b.iter(|| ctx.generate_keypair(&mut OsRng))
    });

    let kp = ctx.generate_keypair(&mut OsRng);

    c.bench_function("crypto_context_v2_encapsulate_cycle", |b| {
        b.iter(|| {
            let (ss, result) = ctx.encapsulate(&mut OsRng, black_box(&kp.public)).unwrap();
            black_box((ss, result))
        })
    });
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    aead_benches,
    bench_aead_encrypt,
    bench_aead_decrypt,
    bench_aead_roundtrip,
    bench_aead_encrypt_in_place,
    bench_aead_decrypt_in_place,
);

criterion_group!(x25519_benches, bench_x25519_keygen, bench_x25519_exchange,);

criterion_group!(blake3_benches, bench_blake3_hash, bench_hkdf, bench_kdf,);

criterion_group!(
    noise_benches,
    bench_noise_keypair_generation,
    bench_noise_full_handshake,
    bench_noise_message_write,
);

criterion_group!(
    ratchet_benches,
    bench_symmetric_ratchet,
    bench_double_ratchet_init,
    bench_double_ratchet_encrypt,
    bench_double_ratchet_decrypt,
    bench_double_ratchet_roundtrip,
    bench_message_header_serialize,
);

criterion_group!(
    elligator_benches,
    bench_elligator_keygen,
    bench_elligator_decode,
    bench_elligator_exchange,
);

criterion_group!(constant_time_benches, bench_constant_time_ops,);

criterion_group!(replay_benches, bench_replay_protection,);

criterion_group!(
    hybrid_kem_benches,
    bench_hybrid_keygen,
    bench_hybrid_encapsulate,
    bench_hybrid_decapsulate,
    bench_hybrid_classical_only,
    bench_hybrid_serialization,
);

criterion_group!(
    packet_ratchet_benches,
    bench_packet_ratchet_new,
    bench_packet_ratchet_next_send_key,
    bench_packet_ratchet_1000_sequential,
    bench_packet_ratchet_key_for_packet,
);

criterion_group!(kdf_v2_benches, bench_kdf_v2,);

criterion_group!(suite_benches, bench_crypto_suite_negotiate,);

criterion_group!(context_v2_benches, bench_crypto_context_v2,);

criterion_main!(
    aead_benches,
    x25519_benches,
    blake3_benches,
    noise_benches,
    ratchet_benches,
    elligator_benches,
    constant_time_benches,
    replay_benches,
    hybrid_kem_benches,
    packet_ratchet_benches,
    kdf_v2_benches,
    suite_benches,
    context_v2_benches,
);
