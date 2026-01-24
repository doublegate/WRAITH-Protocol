# WRAITH Protocol v2 Testing Strategy

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Testing Levels](#testing-levels)
3. [Test Categories](#test-categories)
4. [Security Testing](#security-testing)
5. [Performance Testing](#performance-testing)
6. [Interoperability Testing](#interoperability-testing)
7. [CI/CD Integration](#cicd-integration)
8. [Test Infrastructure](#test-infrastructure)
9. [Coverage Requirements](#coverage-requirements)

---

## Overview

This document defines the comprehensive testing strategy for WRAITH Protocol v2, ensuring security, correctness, performance, and interoperability across all components.

### Testing Philosophy

1. **Security-First:** Crypto and security tests are mandatory
2. **Automated:** All tests run in CI/CD pipeline
3. **Reproducible:** Tests produce consistent results
4. **Comprehensive:** Cover all code paths and edge cases
5. **Fast Feedback:** Unit tests complete in seconds

### Test Pyramid

```
                    ┌─────────┐
                   /  E2E     \          Few, slow, high-value
                  /   Tests    \
                 /──────────────\
                /  Integration   \       Some, moderate speed
               /     Tests        \
              /────────────────────\
             /       Unit Tests     \    Many, fast, foundational
            /──────────────────────────\
           ──────────────────────────────

Distribution:
- Unit Tests:        70% (~1,200 tests)
- Integration Tests: 20% (~350 tests)
- E2E Tests:         10% (~150 tests)
- Total Target:      ~1,700 tests (matching v1)
```

---

## Testing Levels

### Unit Tests

Test individual functions and types in isolation.

```rust
/// Unit test examples
#[cfg(test)]
mod unit_tests {
    use super::*;

    /// Test hybrid key generation
    #[test]
    fn test_hybrid_key_generation() {
        let (secret, public) = HybridSecretKey::generate();

        // Verify key sizes
        assert_eq!(public.classical.as_bytes().len(), 32);
        assert!(public.post_quantum.as_bytes().len() > 1000);

        // Verify keys are non-zero
        assert_ne!(public.classical.as_bytes(), &[0u8; 32]);
    }

    /// Test connection ID generation
    #[test]
    fn test_connection_id_uniqueness() {
        let mut cids = std::collections::HashSet::new();

        for _ in 0..10000 {
            let cid = ConnectionId::generate();
            assert!(cids.insert(cid), "Duplicate CID generated");
        }
    }

    /// Test polymorphic format derivation
    #[test]
    fn test_format_derivation_deterministic() {
        let secret = [0x42u8; 32];

        let format1 = PolymorphicFormat::derive(&secret);
        let format2 = PolymorphicFormat::derive(&secret);

        // Same secret should produce same format
        assert_eq!(format1.format_key, format2.format_key);
    }

    /// Test frame header round-trip
    #[test]
    fn test_frame_header_roundtrip() {
        let header = FrameHeader {
            version: 0x20,
            frame_type: FrameType::Data,
            flags: Flags::ENCRYPTED,
            sequence: 12345678901234,
            length: 1024,
            connection_id: ConnectionId::generate(),
        };

        let format = PolymorphicFormat::derive(&[0x42u8; 32]);

        let encoded = format.encode_header(&header);
        let decoded = format.decode_header(&encoded).unwrap();

        assert_eq!(header.version, decoded.version);
        assert_eq!(header.frame_type, decoded.frame_type);
        assert_eq!(header.sequence, decoded.sequence);
        assert_eq!(header.connection_id, decoded.connection_id);
    }
}
```

### Integration Tests

Test interactions between components.

```rust
/// Integration test examples
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::net::UdpSocket;

    /// Test complete handshake flow
    #[tokio::test]
    async fn test_hybrid_handshake_complete() {
        // Setup server
        let server_key = HybridSecretKey::generate();
        let server = TestServer::new(server_key.clone()).await;

        // Setup client
        let client_key = HybridSecretKey::generate();
        let client = TestClient::new(client_key);

        // Perform handshake
        let result = client.connect(&server.addr(), &server_key.1).await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert!(session.is_hybrid_pq());
    }

    /// Test session data transfer
    #[tokio::test]
    async fn test_session_data_transfer() {
        let (client, server) = create_connected_pair().await;

        let test_data = b"Hello, WRAITH Protocol v2!";

        // Send from client
        client.send(test_data).await.unwrap();

        // Receive on server
        let received = server.recv().await.unwrap();

        assert_eq!(received.as_slice(), test_data);
    }

    /// Test connection migration
    #[tokio::test]
    async fn test_connection_migration() {
        let (client, server) = create_connected_pair().await;

        // Record original path
        let original_addr = client.local_addr();

        // Simulate network change (rebind to new port)
        client.migrate_to_new_path().await.unwrap();

        assert_ne!(client.local_addr(), original_addr);

        // Verify session still works
        client.send(b"post-migration").await.unwrap();
        let received = server.recv().await.unwrap();
        assert_eq!(received.as_slice(), b"post-migration");
    }

    /// Test multi-stream multiplexing
    #[tokio::test]
    async fn test_stream_multiplexing() {
        let (client, server) = create_connected_pair().await;

        // Open multiple streams
        let streams: Vec<_> = (0..10)
            .map(|i| client.open_stream(StreamPriority::Normal))
            .collect::<Result<_, _>>()
            .unwrap();

        // Send on all streams concurrently
        let sends: Vec<_> = streams.iter()
            .enumerate()
            .map(|(i, s)| s.send(format!("stream {}", i).as_bytes()))
            .collect();

        futures::future::try_join_all(sends).await.unwrap();

        // Verify all received (may be in different order)
        let mut received = Vec::new();
        for _ in 0..10 {
            received.push(server.recv().await.unwrap());
        }

        assert_eq!(received.len(), 10);
    }
}
```

### End-to-End Tests

Test complete user scenarios.

```rust
/// End-to-end test examples
#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// Test file transfer workflow
    #[tokio::test]
    async fn test_file_transfer_e2e() {
        // Create test file
        let test_file = TempFile::new_with_size(10 * 1024 * 1024); // 10 MB

        // Start server
        let server = WraithServer::start().await;

        // Connect client
        let client = WraithClient::connect(&server.addr()).await.unwrap();

        // Transfer file
        let transfer = client.send_file(&test_file.path()).await.unwrap();

        // Wait for completion
        transfer.await.unwrap();

        // Verify received file
        let received_path = server.received_files().first().unwrap();
        assert_eq!(
            std::fs::read(&test_file.path()).unwrap(),
            std::fs::read(received_path).unwrap(),
        );
    }

    /// Test network disruption recovery
    #[tokio::test]
    async fn test_network_disruption_recovery() {
        let (client, server) = create_connected_pair().await;

        // Start transfer
        let transfer = client.send_file(&large_file()).await.unwrap();

        // Simulate 5s network disruption
        client.simulate_network_down(Duration::from_secs(5)).await;

        // Transfer should eventually complete
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            transfer,
        ).await;

        assert!(result.is_ok());
    }

    /// Test multi-peer transfer
    #[tokio::test]
    async fn test_multi_peer_transfer() {
        let server = WraithServer::start().await;

        // Connect multiple clients
        let clients: Vec<_> = (0..10)
            .map(|_| WraithClient::connect(&server.addr()))
            .collect::<Result<_, _>>()
            .await
            .unwrap();

        // All clients send simultaneously
        let transfers: Vec<_> = clients.iter()
            .map(|c| c.send_file(&test_file()))
            .collect::<Result<_, _>>()
            .await
            .unwrap();

        // Wait for all to complete
        futures::future::try_join_all(transfers).await.unwrap();

        // Verify server received all
        assert_eq!(server.received_files().len(), 10);
    }
}
```

---

## Test Categories

### Cryptographic Tests

```rust
/// Cryptographic test suite
#[cfg(test)]
mod crypto_tests {
    use super::*;

    /// Test known answer vectors (NIST)
    #[test]
    fn test_ml_kem_768_known_answers() {
        // Load NIST test vectors
        let vectors = load_nist_vectors("ML-KEM-768");

        for vector in vectors {
            let (dk, ek) = MlKem768::from_seed(&vector.seed);

            // Verify encapsulation
            let (ct, ss) = ek.encapsulate_deterministic(&vector.encap_seed);
            assert_eq!(ct.as_bytes(), vector.expected_ciphertext);
            assert_eq!(ss.as_ref(), vector.expected_shared_secret);

            // Verify decapsulation
            let ss_dec = dk.decapsulate(&ct);
            assert_eq!(ss_dec.as_ref(), vector.expected_shared_secret);
        }
    }

    /// Test hybrid key combination
    #[test]
    fn test_hybrid_key_combination() {
        let classical = [0x11u8; 32];
        let post_quantum = [0x22u8; 32];

        let combined = combine_shared_secrets(&classical, &post_quantum);

        // Different inputs should produce different outputs
        let combined2 = combine_shared_secrets(&post_quantum, &classical);
        assert_ne!(combined.as_bytes(), combined2.as_bytes());

        // Same inputs should produce same output
        let combined3 = combine_shared_secrets(&classical, &post_quantum);
        assert_eq!(combined.as_bytes(), combined3.as_bytes());
    }

    /// Test ratchet forward secrecy
    #[test]
    fn test_ratchet_forward_secrecy() {
        let mut ratchet = PacketRatchet::new(&[0x42u8; 32]);

        // Collect first 100 keys
        let keys: Vec<_> = (0..100)
            .map(|_| ratchet.next_send_key().1)
            .collect();

        // Verify all keys are unique
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len());

        // Create new ratchet from same seed
        let mut ratchet2 = PacketRatchet::new(&[0x42u8; 32]);

        // Should produce same key sequence
        for (i, expected) in keys.iter().enumerate() {
            let (pn, key) = ratchet2.next_send_key();
            assert_eq!(pn, i as u64);
            assert_eq!(&key, expected);
        }
    }

    /// Test AEAD authentication
    #[test]
    fn test_aead_authentication() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"test data";
        let aad = b"additional data";

        // Encrypt
        let (ciphertext, tag) = encrypt_aead(&key, &nonce, plaintext, aad);

        // Verify decryption works
        let decrypted = decrypt_aead(&key, &nonce, &ciphertext, &tag, aad);
        assert_eq!(decrypted.as_ref().unwrap(), plaintext);

        // Modify ciphertext - should fail
        let mut modified = ciphertext.clone();
        modified[0] ^= 0x01;
        assert!(decrypt_aead(&key, &nonce, &modified, &tag, aad).is_err());

        // Modify AAD - should fail
        assert!(decrypt_aead(&key, &nonce, &ciphertext, &tag, b"wrong aad").is_err());

        // Modify tag - should fail
        let mut bad_tag = tag;
        bad_tag[0] ^= 0x01;
        assert!(decrypt_aead(&key, &nonce, &ciphertext, &bad_tag, aad).is_err());
    }
}
```

### Protocol State Machine Tests

```rust
/// State machine tests
#[cfg(test)]
mod state_machine_tests {
    use super::*;

    /// Test session state transitions
    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new_pending();

        // Initial state
        assert_eq!(session.state(), SessionState::Pending);

        // Start handshake
        session.start_handshake().unwrap();
        assert_eq!(session.state(), SessionState::Handshaking);

        // Cannot start handshake again
        assert!(session.start_handshake().is_err());

        // Complete handshake
        session.complete_handshake(SessionSecrets::test()).unwrap();
        assert_eq!(session.state(), SessionState::Established);

        // Can send/receive in established state
        assert!(session.can_send());
        assert!(session.can_receive());

        // Close session
        session.close().unwrap();
        assert_eq!(session.state(), SessionState::Closed);

        // Cannot send/receive when closed
        assert!(!session.can_send());
        assert!(!session.can_receive());
    }

    /// Test stream state machine
    #[test]
    fn test_stream_states() {
        let mut stream = Stream::new(StreamId(1), StreamRole::Initiator);

        // Initial state: Open
        assert!(stream.can_send());
        assert!(stream.can_receive());

        // Send FIN
        stream.shutdown_write().unwrap();
        assert!(!stream.can_send());
        assert!(stream.can_receive());

        // Receive FIN
        stream.shutdown_read().unwrap();
        assert!(!stream.can_send());
        assert!(!stream.can_receive());
        assert_eq!(stream.state(), StreamState::Closed);
    }

    /// Test handshake state machine
    #[test]
    fn test_handshake_state_machine() {
        let states = vec![
            HandshakeState::Initial,
            HandshakeState::WaitingForResponse,
            HandshakeState::WaitingForFinal,
            HandshakeState::Complete,
        ];

        for i in 0..states.len() - 1 {
            let mut hs = Handshake::new_at_state(states[i].clone());

            // Valid transition
            assert!(hs.transition_to(states[i + 1].clone()).is_ok());

            // Cannot skip states
            if i + 2 < states.len() {
                let mut hs2 = Handshake::new_at_state(states[i].clone());
                assert!(hs2.transition_to(states[i + 2].clone()).is_err());
            }
        }
    }
}
```

### Edge Case Tests

```rust
/// Edge case and boundary tests
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    /// Test maximum packet size
    #[test]
    fn test_max_packet_size() {
        let session = create_test_session();

        // Maximum payload
        let max_payload = vec![0u8; MAX_PAYLOAD_SIZE];
        let frame = session.create_data_frame(&max_payload).unwrap();
        assert!(frame.total_size() <= MAX_PACKET_SIZE);

        // One byte over should fail
        let too_large = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        assert!(session.create_data_frame(&too_large).is_err());
    }

    /// Test empty payload
    #[test]
    fn test_empty_payload() {
        let session = create_test_session();

        let frame = session.create_data_frame(&[]).unwrap();
        assert_eq!(frame.payload.len(), 0);

        // Should still encrypt/decrypt correctly
        let encrypted = session.encrypt_frame(&frame).unwrap();
        let decrypted = session.decrypt_frame(&encrypted).unwrap();
        assert_eq!(decrypted.payload.len(), 0);
    }

    /// Test sequence number wraparound
    #[test]
    fn test_sequence_wraparound() {
        let mut session = create_test_session();

        // Set sequence near max
        session.set_sequence(u64::MAX - 10);

        // Should handle wraparound
        for _ in 0..20 {
            let frame = session.create_data_frame(b"test").unwrap();
            assert!(session.process_frame(&frame).is_ok());
        }
    }

    /// Test connection ID collision handling
    #[test]
    fn test_cid_collision() {
        let mut sessions = SessionManager::new();

        // Create first session
        let cid = ConnectionId::generate();
        sessions.insert(cid, Session::new()).unwrap();

        // Attempt duplicate - should fail
        assert!(sessions.insert(cid, Session::new()).is_err());

        // Different CID should work
        let cid2 = ConnectionId::generate();
        assert!(sessions.insert(cid2, Session::new()).is_ok());
    }

    /// Test all frame types
    #[test]
    fn test_all_frame_types() {
        for frame_type in FrameType::all() {
            let header = FrameHeader {
                version: 0x20,
                frame_type,
                flags: Flags::empty(),
                sequence: 1,
                length: 0,
                connection_id: ConnectionId::generate(),
            };

            // Should serialize/deserialize
            let format = PolymorphicFormat::derive(&[0x42u8; 32]);
            let encoded = format.encode_header(&header);
            let decoded = format.decode_header(&encoded).unwrap();

            assert_eq!(header.frame_type, decoded.frame_type);
        }
    }
}
```

---

## Security Testing

### Fuzzing

```rust
/// Fuzzing harnesses
#[cfg(fuzzing)]
pub mod fuzz {
    use libfuzzer_sys::fuzz_target;

    fuzz_target!(|data: &[u8]| {
        // Fuzz frame parsing
        let _ = Frame::parse(data);
    });

    fuzz_target!(|data: &[u8]| {
        // Fuzz handshake message parsing
        let _ = HandshakeMessage::parse(data);
    });

    fuzz_target!(|data: FuzzInput| {
        // Fuzz AEAD decryption
        let key = data.key;
        let nonce = data.nonce;
        let _ = decrypt_aead(&key, &nonce, &data.ciphertext, &data.tag, &data.aad);
    });

    fuzz_target!(|data: &[u8]| {
        // Fuzz polymorphic format decoding
        if data.len() >= 32 {
            let format = PolymorphicFormat::derive(data[..32].try_into().unwrap());
            if data.len() >= 56 {
                let _ = format.decode_header(data[32..56].try_into().unwrap());
            }
        }
    });
}
```

### Property-Based Testing

```rust
/// Property-based tests using proptest
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        /// Encryption is reversible
        #[test]
        fn encrypt_decrypt_roundtrip(
            key in prop::array::uniform32(any::<u8>()),
            plaintext in prop::collection::vec(any::<u8>(), 0..10000),
        ) {
            let nonce = [0u8; 24];
            let aad = b"test";

            let (ct, tag) = encrypt_aead(&key, &nonce, &plaintext, aad);
            let decrypted = decrypt_aead(&key, &nonce, &ct, &tag, aad).unwrap();

            prop_assert_eq!(plaintext, decrypted);
        }

        /// Frame header round-trip
        #[test]
        fn frame_header_roundtrip(
            version in 0x20u8..0x30u8,
            frame_type in 0u8..0x70u8,
            flags in any::<u8>(),
            sequence in any::<u64>(),
            length in any::<u32>(),
            cid in prop::array::uniform16(any::<u8>()),
        ) {
            let header = FrameHeader {
                version,
                frame_type: FrameType::try_from(frame_type).unwrap_or(FrameType::Data),
                flags: Flags::from_bits_truncate(flags),
                sequence,
                length,
                connection_id: ConnectionId::from_bytes(cid),
            };

            let format = PolymorphicFormat::derive(&[0x42u8; 32]);
            let encoded = format.encode_header(&header);
            let decoded = format.decode_header(&encoded).unwrap();

            prop_assert_eq!(header.version, decoded.version);
            prop_assert_eq!(header.sequence, decoded.sequence);
            prop_assert_eq!(header.length, decoded.length);
        }

        /// Ratchet produces unique keys
        #[test]
        fn ratchet_unique_keys(
            seed in prop::array::uniform32(any::<u8>()),
            count in 1usize..1000,
        ) {
            let mut ratchet = PacketRatchet::new(&seed);
            let mut keys = std::collections::HashSet::new();

            for _ in 0..count {
                let (_, key) = ratchet.next_send_key();
                prop_assert!(keys.insert(key), "Duplicate key produced");
            }
        }
    }
}
```

### Timing Attack Tests

```rust
/// Timing attack resistance tests
#[cfg(test)]
mod timing_tests {
    use super::*;
    use std::time::Instant;

    /// Verify constant-time comparison
    #[test]
    fn test_constant_time_compare() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let c = [1u8; 32];

        // Many iterations to measure timing
        let iterations = 100000;

        // Time equal comparison
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = constant_time_eq(&a, &b);
        }
        let equal_time = start.elapsed();

        // Time unequal comparison
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = constant_time_eq(&a, &c);
        }
        let unequal_time = start.elapsed();

        // Times should be within 10% of each other
        let ratio = equal_time.as_nanos() as f64 / unequal_time.as_nanos() as f64;
        assert!(
            (0.9..=1.1).contains(&ratio),
            "Timing difference detected: ratio = {}",
            ratio
        );
    }

    /// Verify decryption timing doesn't leak plaintext
    #[test]
    fn test_decryption_timing() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 24];
        let plaintext = b"secret data here";
        let aad = b"aad";

        let (ciphertext, tag) = encrypt_aead(&key, &nonce, plaintext, aad);

        // Create invalid ciphertext (bit flip at different positions)
        let positions: Vec<usize> = (0..ciphertext.len()).collect();
        let mut timings = Vec::new();

        for pos in positions {
            let mut modified = ciphertext.clone();
            modified[pos] ^= 0x01;

            let start = Instant::now();
            for _ in 0..1000 {
                let _ = decrypt_aead(&key, &nonce, &modified, &tag, aad);
            }
            timings.push(start.elapsed());
        }

        // All timings should be similar (within 20%)
        let avg = timings.iter().map(|t| t.as_nanos()).sum::<u128>() / timings.len() as u128;
        for timing in &timings {
            let ratio = timing.as_nanos() as f64 / avg as f64;
            assert!(
                (0.8..=1.2).contains(&ratio),
                "Position-dependent timing detected"
            );
        }
    }
}
```

---

## Performance Testing

### Benchmark Suite

```rust
/// Performance benchmarks
pub mod benchmarks {
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

    pub fn crypto_benchmarks(c: &mut Criterion) {
        let mut group = c.benchmark_group("crypto");

        // X25519 key exchange
        group.bench_function("x25519_keygen", |b| {
            b.iter(|| x25519_dalek::StaticSecret::random_from_rng(&mut rand::thread_rng()))
        });

        // ML-KEM-768 key exchange
        group.bench_function("ml_kem_768_keygen", |b| {
            b.iter(|| MlKem768::generate(&mut rand::thread_rng()))
        });

        // Hybrid encapsulation
        group.bench_function("hybrid_encapsulate", |b| {
            let (_, pk) = HybridSecretKey::generate();
            b.iter(|| pk.encapsulate())
        });

        // AEAD encryption
        for size in [64, 1024, 8192, 65536].iter() {
            group.throughput(Throughput::Bytes(*size as u64));
            group.bench_with_input(BenchmarkId::new("aead_encrypt", size), size, |b, &size| {
                let key = [0x42u8; 32];
                let nonce = [0u8; 24];
                let data = vec![0u8; size];
                b.iter(|| encrypt_aead(&key, &nonce, &data, b"aad"))
            });
        }

        group.finish();
    }

    pub fn throughput_benchmarks(c: &mut Criterion) {
        let mut group = c.benchmark_group("throughput");

        for size in [1024, 8192, 65536, 262144, 1048576].iter() {
            group.throughput(Throughput::Bytes(*size as u64));

            group.bench_with_input(BenchmarkId::new("session_send", size), size, |b, &size| {
                let session = create_test_session();
                let data = vec![0u8; size];
                b.iter(|| session.send_sync(&data))
            });
        }

        group.finish();
    }

    pub fn latency_benchmarks(c: &mut Criterion) {
        // Packet processing latency
        c.bench_function("packet_process", |b| {
            let session = create_test_session();
            let packet = create_test_packet(1024);
            b.iter(|| session.process_packet(&packet))
        });

        // Ratchet advance
        c.bench_function("ratchet_advance", |b| {
            let mut ratchet = PacketRatchet::new(&[0x42u8; 32]);
            b.iter(|| ratchet.next_send_key())
        });

        // Frame encoding
        c.bench_function("frame_encode", |b| {
            let format = PolymorphicFormat::derive(&[0x42u8; 32]);
            let header = create_test_header();
            b.iter(|| format.encode_header(&header))
        });
    }

    criterion_group!(
        benches,
        crypto_benchmarks,
        throughput_benchmarks,
        latency_benchmarks,
    );
    criterion_main!(benches);
}
```

### Load Testing

```rust
/// Load test framework
pub mod load_tests {
    use super::*;

    /// Load test configuration
    pub struct LoadTestConfig {
        pub concurrent_sessions: usize,
        pub messages_per_session: usize,
        pub message_size: usize,
        pub duration: Duration,
    }

    /// Run load test
    pub async fn run_load_test(config: LoadTestConfig) -> LoadTestResult {
        let server = TestServer::start().await;
        let mut handles = Vec::new();

        let start = Instant::now();

        // Spawn concurrent sessions
        for _ in 0..config.concurrent_sessions {
            let server_addr = server.addr();
            let messages = config.messages_per_session;
            let size = config.message_size;

            handles.push(tokio::spawn(async move {
                let client = TestClient::connect(&server_addr).await?;
                let data = vec![0u8; size];

                for _ in 0..messages {
                    client.send(&data).await?;
                }

                Ok::<_, Error>(())
            }));
        }

        // Wait for completion or timeout
        let results = tokio::time::timeout(
            config.duration,
            futures::future::join_all(handles),
        ).await;

        let elapsed = start.elapsed();

        // Calculate metrics
        let total_messages = config.concurrent_sessions * config.messages_per_session;
        let total_bytes = total_messages * config.message_size;

        LoadTestResult {
            total_messages,
            total_bytes,
            duration: elapsed,
            throughput_mbps: (total_bytes as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0,
            messages_per_second: total_messages as f64 / elapsed.as_secs_f64(),
            success: results.is_ok(),
        }
    }
}
```

---

## Interoperability Testing

### v1/v2 Compatibility

```rust
/// v1/v2 interoperability tests
#[cfg(test)]
mod interop_tests {
    use super::*;

    /// Test v2 server accepts v1 clients
    #[tokio::test]
    async fn test_v2_server_v1_client() {
        let server = V2Server::start_with_compat().await;
        let client = V1Client::new();

        let session = client.connect(&server.addr()).await.unwrap();

        // Should negotiate v1 protocol
        assert!(session.is_v1_compat());

        // Data transfer should work
        client.send(b"hello from v1").await.unwrap();
        let received = server.recv().await.unwrap();
        assert_eq!(received.as_slice(), b"hello from v1");
    }

    /// Test v2 client connects to v1 server
    #[tokio::test]
    async fn test_v2_client_v1_server() {
        let server = V1Server::start().await;
        let client = V2Client::new();

        let session = client.connect(&server.addr()).await.unwrap();

        // Should fall back to v1
        assert!(session.is_v1_compat());
        assert!(!session.is_hybrid_pq());
    }

    /// Test version upgrade during session
    #[tokio::test]
    async fn test_session_upgrade() {
        let server = V2Server::start_with_compat().await;

        // Connect as v1
        let client = V2Client::with_initial_version(Version::V1);
        let session = client.connect(&server.addr()).await.unwrap();

        assert!(session.is_v1_compat());

        // Request upgrade
        session.request_upgrade(Version::V2).await.unwrap();

        // Should now be v2
        assert!(!session.is_v1_compat());
    }
}
```

### Cross-Platform Testing

```rust
/// Cross-platform test definitions
pub mod cross_platform {
    /// Platforms to test
    pub const PLATFORMS: &[Platform] = &[
        Platform::LinuxX86_64,
        Platform::LinuxAarch64,
        Platform::MacOsX86_64,
        Platform::MacOsAarch64,
        Platform::WindowsX86_64,
        Platform::Wasm,
    ];

    /// Cross-platform test matrix
    #[test]
    fn test_encoding_consistency() {
        let test_data = include_bytes!("test_vectors/encoding.bin");

        for platform in PLATFORMS {
            let result = encode_on_platform(test_data, *platform);
            assert_eq!(result, EXPECTED_ENCODING);
        }
    }
}
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Test Suite

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run unit tests
        run: cargo test --lib --all-features

      - name: Run doc tests
        run: cargo test --doc --all-features

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run integration tests
        run: cargo test --test '*' --all-features

  security-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Run fuzz tests (quick)
        run: |
          for target in $(cargo fuzz list); do
            cargo fuzz run $target -- -max_total_time=60
          done

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info

  benchmarks:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run benchmarks
        run: cargo bench --all-features -- --noplot

      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/report/index.html
```

---

## Test Infrastructure

### Test Utilities

```rust
/// Test utilities module
pub mod test_utils {
    use super::*;

    /// Create connected test pair
    pub async fn create_connected_pair() -> (TestClient, TestServer) {
        let server = TestServer::start().await;
        let client = TestClient::connect(&server.addr()).await.unwrap();
        (client, server)
    }

    /// Create test session with default config
    pub fn create_test_session() -> Session {
        let (secret, public) = HybridSecretKey::generate();
        Session::new_test(secret, public)
    }

    /// Create test packet of given size
    pub fn create_test_packet(size: usize) -> Packet {
        let session = create_test_session();
        let data = vec![0x42u8; size];
        session.create_packet(&data).unwrap()
    }

    /// Temporary file helper
    pub struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        pub fn new_with_size(size: usize) -> Self {
            let path = std::env::temp_dir().join(format!("wraith_test_{}", uuid::Uuid::new_v4()));
            let mut file = std::fs::File::create(&path).unwrap();

            let mut remaining = size;
            let chunk = vec![0x42u8; 65536];
            while remaining > 0 {
                let to_write = remaining.min(chunk.len());
                file.write_all(&chunk[..to_write]).unwrap();
                remaining -= to_write;
            }

            Self { path }
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
```

---

## Coverage Requirements

### Minimum Coverage Targets

| Category | Target | Notes |
|----------|--------|-------|
| Overall | 80% | Weighted by risk |
| Crypto | 95% | Critical paths |
| Protocol | 90% | State machines |
| Transport | 85% | I/O paths |
| Utility | 70% | Helpers |

### Coverage Exceptions

```rust
// Coverage exceptions (documented reasons)

#[cfg(not(tarpaulin_include))]
mod platform_specific {
    // Platform-specific code tested on CI matrix
}

#[cfg(not(tarpaulin_include))]
fn panic_handler() {
    // Panic handlers are emergency paths
}

#[cfg(not(tarpaulin_include))]
mod debug_only {
    // Debug-only code not in release builds
}
```

---

## Related Documents

- [Implementation Phases](10-WRAITH-Protocol-v2-Implementation-Phases.md) - Development timeline
- [Performance Targets](13-WRAITH-Protocol-v2-Performance-Targets.md) - Performance requirements
- [Security Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md) - Security testing details
- [CI/CD Integration](07-WRAITH-Protocol-v2-Implementation-Guide.md) - Build system

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial testing strategy document |
