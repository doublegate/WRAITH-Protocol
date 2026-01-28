//! Performance benchmarks for wraith-crypto.
//!
//! Run with: `cargo bench -p wraith-crypto`
//!
//! Target performance metrics:
//! - AEAD encryption: >3 GB/s (single core)
//! - Noise handshake: <50ms (full XX)
//! - Key ratcheting: >10M ops/sec

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
// Criterion Configuration
// ============================================================================

criterion_group!(
    aead_benches,
    bench_aead_encrypt,
    bench_aead_decrypt,
    bench_aead_roundtrip,
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

criterion_main!(
    aead_benches,
    x25519_benches,
    blake3_benches,
    noise_benches,
    ratchet_benches,
    elligator_benches,
    constant_time_benches,
);
