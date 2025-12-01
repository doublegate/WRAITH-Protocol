// Integration tests for cross-crate interactions.
//!
// Tests the integration between wraith-crypto and wraith-core crates,
// verifying that cryptographic operations work correctly with frame
// encoding/decoding and session management.

use rand_core::{OsRng, RngCore};
use wraith_core::{
    ConnectionId, FRAME_HEADER_SIZE, Frame, FrameBuilder, FrameFlags, FrameType, HandshakePhase,
    Session, SessionState, Stream, StreamState,
};
use wraith_crypto::{
    SessionKeys,
    aead::{AeadKey, Nonce, SessionCrypto, TAG_SIZE},
    ratchet::{DoubleRatchet, SymmetricRatchet},
    x25519::PrivateKey,
};

/// Minimum frame size for tests (header + small payload + some padding).
const TEST_FRAME_SIZE: usize = FRAME_HEADER_SIZE + 64;

/// Generate a random connection ID.
fn generate_connection_id() -> ConnectionId {
    let mut bytes = [0u8; 8];
    OsRng.fill_bytes(&mut bytes);
    ConnectionId::from_bytes(bytes)
}

// ============================================================================
// Frame Encryption Integration Tests
// ============================================================================

/// Test encrypting and decrypting frame payloads using AEAD.
#[test]
fn test_frame_payload_encryption_roundtrip() {
    // Create a frame with payload
    let stream_id = 16u16;
    let payload = b"Hello, WRAITH Protocol!";
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .flags(FrameFlags::new())
        .stream_id(stream_id)
        .offset(0)
        .payload(payload)
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    // Create AEAD key for encryption
    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);

    // Use connection ID as associated data (simulating real protocol usage)
    let connection_id = generate_connection_id();
    let aad = &connection_id.to_bytes();

    // Encrypt the frame data
    let ciphertext = key
        .encrypt(&nonce, &frame_data, aad)
        .expect("Encryption failed");

    // Verify ciphertext is different from plaintext
    assert_ne!(&ciphertext[..frame_data.len()], &frame_data[..]);

    // Decrypt the frame data
    let decrypted = key
        .decrypt(&nonce, &ciphertext, aad)
        .expect("Decryption failed");

    // Verify roundtrip
    assert_eq!(decrypted, frame_data);

    // Parse the decrypted frame
    let frame = Frame::parse(&decrypted).expect("Failed to parse decrypted frame");
    assert_eq!(frame.frame_type(), FrameType::Data);
    assert_eq!(frame.stream_id(), stream_id);
    assert_eq!(frame.payload(), payload);
}

/// Test that tampered ciphertext is detected.
#[test]
fn test_frame_tampering_detection() {
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"sensitive data")
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);
    let aad = b"session-id";

    let mut ciphertext = key
        .encrypt(&nonce, &frame_data, aad)
        .expect("Encryption failed");

    // Tamper with the ciphertext
    if !ciphertext.is_empty() {
        ciphertext[0] ^= 0xFF;
    }

    // Decryption should fail
    assert!(key.decrypt(&nonce, &ciphertext, aad).is_err());
}

/// Test wrong AAD detection (connection ID mismatch).
#[test]
fn test_wrong_connection_id_detection() {
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Ack)
        .stream_id(16)
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);

    let cid1 = generate_connection_id();
    let cid2 = generate_connection_id();

    let ciphertext = key
        .encrypt(&nonce, &frame_data, &cid1.to_bytes())
        .expect("Encryption failed");

    // Decryption with wrong connection ID should fail
    assert!(key.decrypt(&nonce, &ciphertext, &cid2.to_bytes()).is_err());
}

// ============================================================================
// Session Keys Integration Tests
// ============================================================================

/// Test deriving connection ID from session keys.
#[test]
fn test_session_keys_connection_id_derivation() {
    let keys = SessionKeys {
        send_key: [0x42u8; 32],
        recv_key: [0x43u8; 32],
        chain_key: [0x44u8; 32],
    };

    let cid = keys.derive_connection_id();

    // Connection ID should be 8 bytes
    assert_eq!(cid.len(), 8);

    // Same keys should produce same connection ID (deterministic)
    let keys2 = SessionKeys {
        send_key: [0x42u8; 32],
        recv_key: [0x43u8; 32],
        chain_key: [0x44u8; 32],
    };
    assert_eq!(keys2.derive_connection_id(), cid);

    // Different chain key should produce different connection ID
    let keys3 = SessionKeys {
        send_key: [0x42u8; 32],
        recv_key: [0x43u8; 32],
        chain_key: [0x45u8; 32],
    };
    assert_ne!(keys3.derive_connection_id(), cid);
}

/// Test session crypto with frame encryption.
#[test]
fn test_session_crypto_frame_exchange() {
    let chain_key = [0x42u8; 32];

    // Alice's perspective: send with key A, receive with key B
    let mut alice = SessionCrypto::new([1u8; 32], [2u8; 32], &chain_key);
    // Bob's perspective: send with key B, receive with key A
    let mut bob = SessionCrypto::new([2u8; 32], [1u8; 32], &chain_key);

    // Alice creates and encrypts a DATA frame
    let alice_frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Hello Bob!")
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    let alice_ct = alice.encrypt(&alice_frame, b"").expect("Encryption failed");

    // Bob decrypts and parses
    let bob_pt = bob.decrypt(&alice_ct, b"").expect("Decryption failed");
    let bob_frame = Frame::parse(&bob_pt).expect("Failed to parse frame");
    assert_eq!(bob_frame.payload(), b"Hello Bob!");

    // Bob creates and encrypts an ACK frame
    let bob_ack = FrameBuilder::new()
        .frame_type(FrameType::Ack)
        .stream_id(16)
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    let bob_ct = bob.encrypt(&bob_ack, b"").expect("Encryption failed");

    // Alice decrypts and parses
    let alice_pt = alice.decrypt(&bob_ct, b"").expect("Decryption failed");
    let alice_ack = Frame::parse(&alice_pt).expect("Failed to parse frame");
    assert_eq!(alice_ack.frame_type(), FrameType::Ack);
}

// ============================================================================
// Double Ratchet Integration Tests
// ============================================================================

/// Test encrypting frames with double ratchet keys.
#[test]
fn test_double_ratchet_frame_encryption() {
    let shared_secret = [0x42u8; 32];

    // Setup double ratchet
    let bob_dh = PrivateKey::generate(&mut OsRng);
    let bob_dh_public = bob_dh.public_key();

    let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
    let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

    // Alice encrypts a frame payload
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Ratcheted payload")
        .build(TEST_FRAME_SIZE)
        .expect("Failed to build frame");

    let (header, ciphertext) = alice
        .encrypt(&mut OsRng, &frame_data)
        .expect("Ratchet encryption failed");

    // Bob decrypts
    let plaintext = bob
        .decrypt(&mut OsRng, &header, &ciphertext)
        .expect("Ratchet decryption failed");

    // Parse the decrypted frame
    let frame = Frame::parse(&plaintext).expect("Failed to parse frame");
    assert_eq!(frame.payload(), b"Ratcheted payload");
}

/// Test bidirectional frame exchange with double ratchet.
#[test]
fn test_double_ratchet_bidirectional_frames() {
    let shared_secret = [0x42u8; 32];

    let bob_dh = PrivateKey::generate(&mut OsRng);
    let bob_dh_public = bob_dh.public_key();

    let mut alice = DoubleRatchet::new_initiator(&mut OsRng, &shared_secret, bob_dh_public);
    let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_dh);

    // Alice -> Bob: Data frame
    let alice_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Request data")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let (h1, c1) = alice.encrypt(&mut OsRng, &alice_data).unwrap();
    let p1 = bob.decrypt(&mut OsRng, &h1, &c1).unwrap();
    assert_eq!(Frame::parse(&p1).unwrap().payload(), b"Request data");

    // Bob -> Alice: Ack frame
    let bob_ack = FrameBuilder::new()
        .frame_type(FrameType::Ack)
        .stream_id(16)
        .offset(100)
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let (h2, c2) = bob.encrypt(&mut OsRng, &bob_ack).unwrap();
    let p2 = alice.decrypt(&mut OsRng, &h2, &c2).unwrap();
    assert_eq!(Frame::parse(&p2).unwrap().frame_type(), FrameType::Ack);

    // Alice -> Bob: More data
    let alice_data2 = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .offset(100)
        .payload(b"More data")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let (h3, c3) = alice.encrypt(&mut OsRng, &alice_data2).unwrap();
    let p3 = bob.decrypt(&mut OsRng, &h3, &c3).unwrap();
    assert_eq!(Frame::parse(&p3).unwrap().payload(), b"More data");
}

/// Test forward secrecy: old keys cannot decrypt new messages.
#[test]
fn test_forward_secrecy_with_frames() {
    let chain_key = [0x42u8; 32];
    let mut ratchet = SymmetricRatchet::new(&chain_key);

    // Get first key
    let key1 = ratchet.next_key();

    // Create and encrypt a frame with key1
    let frame1 = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Message 1")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let aead1 = key1.to_aead_key();
    let nonce1 = Nonce::from_bytes([0u8; 24]);
    let ct1 = aead1.encrypt(&nonce1, &frame1, b"").unwrap();

    // Advance ratchet and get new key
    let key2 = ratchet.next_key();

    // Create and encrypt a frame with key2
    let frame2 = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Message 2")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let aead2 = key2.to_aead_key();
    let nonce2 = Nonce::from_bytes([1u8; 24]);
    let ct2 = aead2.encrypt(&nonce2, &frame2, b"").unwrap();

    // Key1 should NOT be able to decrypt message encrypted with key2
    assert!(aead1.decrypt(&nonce2, &ct2, b"").is_err());

    // Key2 should NOT be able to decrypt message encrypted with key1
    assert!(aead2.decrypt(&nonce1, &ct1, b"").is_err());

    // Original keys should still decrypt their own messages
    let pt1 = aead1.decrypt(&nonce1, &ct1, b"").unwrap();
    assert_eq!(Frame::parse(&pt1).unwrap().payload(), b"Message 1");

    let pt2 = aead2.decrypt(&nonce2, &ct2, b"").unwrap();
    assert_eq!(Frame::parse(&pt2).unwrap().payload(), b"Message 2");
}

// ============================================================================
// Session State Integration Tests
// ============================================================================

/// Test session state machine with crypto operations.
#[test]
fn test_session_state_with_crypto() {
    let mut session = Session::new();

    // Initial state should be Closed
    assert_eq!(session.state(), SessionState::Closed);

    // Transition to handshaking
    session
        .transition_to(SessionState::Handshaking(HandshakePhase::InitSent))
        .expect("Failed to transition to handshaking");

    // Simulate crypto handshake completion
    session
        .transition_to(SessionState::Handshaking(HandshakePhase::InitComplete))
        .expect("Failed to transition to init complete");

    session
        .transition_to(SessionState::Established)
        .expect("Failed to transition to established");

    // Now we can use encrypted communication
    assert_eq!(session.state(), SessionState::Established);

    // Test rekeying transition (would happen after crypto ratchet)
    session
        .transition_to(SessionState::Rekeying)
        .expect("Failed to transition to rekeying");

    session
        .transition_to(SessionState::Established)
        .expect("Failed to return to established after rekey");
}

/// Test stream state transitions with crypto context.
#[test]
fn test_stream_state_transitions() {
    // Create a stream with reasonable initial window
    let mut stream = Stream::new(1, 65536);

    // Stream starts in Idle state
    assert!(stream.can_transition(StreamState::Open));

    // Transition to Open (would happen after encrypted handshake)
    stream.transition_to(StreamState::Open).unwrap();
    assert_eq!(stream.state(), StreamState::Open);

    // Test half-close transition (encrypted FIN sent)
    stream.transition_to(StreamState::HalfClosedLocal).unwrap();
    assert_eq!(stream.state(), StreamState::HalfClosedLocal);

    // Test final close
    stream.transition_to(StreamState::Closed).unwrap();
    assert_eq!(stream.state(), StreamState::Closed);
}

/// Test stream encryption with associated stream ID.
#[test]
fn test_stream_encryption_with_stream_id() {
    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);

    // Create stream and use stream ID in AAD for binding
    let stream = Stream::new(42, 65536);
    let stream_id_aad = stream.id().to_be_bytes();

    let payload = b"Stream-bound encrypted data";
    let ciphertext = key.encrypt(&nonce, payload, &stream_id_aad).unwrap();

    // Decrypting with correct stream ID works
    let decrypted = key.decrypt(&nonce, &ciphertext, &stream_id_aad).unwrap();
    assert_eq!(decrypted, payload);

    // Decrypting with wrong stream ID fails
    let wrong_stream_id = 99u16.to_be_bytes();
    assert!(key.decrypt(&nonce, &ciphertext, &wrong_stream_id).is_err());
}

// ============================================================================
// Control Frame Tests
// ============================================================================

/// Test control frame encryption for session management.
#[test]
fn test_control_frame_encryption() {
    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);

    // Test various control frame types
    let control_frames = [
        FrameBuilder::new()
            .frame_type(FrameType::Ping)
            .build(TEST_FRAME_SIZE)
            .unwrap(),
        FrameBuilder::new()
            .frame_type(FrameType::Pong)
            .build(TEST_FRAME_SIZE)
            .unwrap(),
        FrameBuilder::new()
            .frame_type(FrameType::Close)
            .payload(&[0x00, 0x00]) // Error code
            .build(TEST_FRAME_SIZE)
            .unwrap(),
        FrameBuilder::new()
            .frame_type(FrameType::Rekey)
            .build(TEST_FRAME_SIZE)
            .unwrap(),
    ];

    for frame in &control_frames {
        // Encrypt
        let ct = key.encrypt(&nonce, frame, b"control").unwrap();

        // Decrypt
        let pt = key.decrypt(&nonce, &ct, b"control").unwrap();
        assert_eq!(&pt, frame);

        // Parse
        let _parsed = Frame::parse(&pt).unwrap();
    }
}

/// Test rekey control frame with key material.
#[test]
fn test_rekey_frame_with_new_keys() {
    // Old session keys
    let old_chain_key = [0x42u8; 32];
    let mut alice = SessionCrypto::new([1u8; 32], [2u8; 32], &old_chain_key);
    let mut bob = SessionCrypto::new([2u8; 32], [1u8; 32], &old_chain_key);

    // Alice sends a rekey frame
    let rekey_frame = FrameBuilder::new()
        .frame_type(FrameType::Rekey)
        .build(TEST_FRAME_SIZE)
        .unwrap();
    let ct = alice.encrypt(&rekey_frame, b"").unwrap();
    let pt = bob.decrypt(&ct, b"").unwrap();
    assert_eq!(Frame::parse(&pt).unwrap().frame_type(), FrameType::Rekey);

    // Both sides update to new keys (simulating DH ratchet result)
    let new_chain_key = [0x99u8; 32];
    alice.update_keys([3u8; 32], [4u8; 32], &new_chain_key);
    bob.update_keys([4u8; 32], [3u8; 32], &new_chain_key);

    // Communication continues with new keys
    let data_frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Post-rekey data")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let ct2 = alice.encrypt(&data_frame, b"").unwrap();
    let pt2 = bob.decrypt(&ct2, b"").unwrap();
    assert_eq!(Frame::parse(&pt2).unwrap().payload(), b"Post-rekey data");
}

// ============================================================================
// Padding Frame Tests
// ============================================================================

/// Test padding frame with cryptographic randomness.
#[test]
fn test_padding_frame_encryption() {
    let key = AeadKey::generate(&mut OsRng);
    let nonce = Nonce::generate(&mut OsRng);

    // Generate random padding
    let mut padding = vec![0u8; 32];
    OsRng.fill_bytes(&mut padding);

    let pad_frame = FrameBuilder::new()
        .frame_type(FrameType::Pad)
        .payload(&padding)
        .build(TEST_FRAME_SIZE)
        .unwrap();

    // Encrypt padding frame
    let ct = key.encrypt(&nonce, &pad_frame, b"").unwrap();

    // Ciphertext should be indistinguishable from random
    // (Statistical test would be needed for real verification)
    assert_eq!(ct.len(), pad_frame.len() + TAG_SIZE);

    // Decrypt
    let pt = key.decrypt(&nonce, &ct, b"").unwrap();
    let parsed = Frame::parse(&pt).unwrap();
    assert_eq!(parsed.frame_type(), FrameType::Pad);
    assert_eq!(parsed.payload(), &padding);
}

// ============================================================================
// X25519 Key Exchange Integration Tests
// ============================================================================

/// Test key exchange leading to session establishment.
#[test]
fn test_x25519_to_session_keys() {
    use wraith_crypto::hash::hkdf_expand;

    // Alice and Bob each generate keypairs
    let alice_private = PrivateKey::generate(&mut OsRng);
    let alice_public = alice_private.public_key();

    let bob_private = PrivateKey::generate(&mut OsRng);
    let bob_public = bob_private.public_key();

    // Each computes the shared secret
    let alice_shared = alice_private.exchange(&bob_public).unwrap();
    let bob_shared = bob_private.exchange(&alice_public).unwrap();

    // Shared secrets should match
    assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());

    // Derive session keys from shared secret (simulating HKDF derivation)
    let mut send_key = [0u8; 32];
    let mut recv_key = [0u8; 32];
    let mut chain_key = [0u8; 32];

    hkdf_expand(alice_shared.as_bytes(), b"wraith_send_key", &mut send_key);
    hkdf_expand(alice_shared.as_bytes(), b"wraith_recv_key", &mut recv_key);
    hkdf_expand(alice_shared.as_bytes(), b"wraith_chain_key", &mut chain_key);

    let alice_keys = SessionKeys {
        send_key,
        recv_key,
        chain_key,
    };

    // Derive connection ID
    let cid = alice_keys.derive_connection_id();
    assert_eq!(cid.len(), 8);

    // Create session crypto
    let mut alice_crypto = SessionCrypto::new(send_key, recv_key, &chain_key);
    let mut bob_crypto = SessionCrypto::new(recv_key, send_key, &chain_key);

    // Test encrypted frame exchange
    let frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Post-handshake message")
        .build(TEST_FRAME_SIZE)
        .unwrap();

    let ct = alice_crypto.encrypt(&frame, &cid).unwrap();
    let pt = bob_crypto.decrypt(&ct, &cid).unwrap();
    assert_eq!(
        Frame::parse(&pt).unwrap().payload(),
        b"Post-handshake message"
    );
}
// Integration tests for WRAITH Protocol
//
// Tests the integration of all protocol components:
// - File transfer end-to-end
// - Multi-peer coordination
// - Resume functionality
// - NAT traversal
// - Relay fallback

use std::path::PathBuf;
use tempfile::TempDir;
use wraith_core::transfer::{TransferSession, TransferState};
use wraith_files::DEFAULT_CHUNK_SIZE;
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::tree_hash::compute_tree_hash;

/// Test basic file chunking and reassembly (unit-level integration)
#[test]
fn test_file_chunking_integration() {
    // Create test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.dat");
    let test_data = vec![0xAA; 1024 * 1024]; // 1 MB
    std::fs::write(&test_file, &test_data).unwrap();

    // Chunk file
    let mut chunker = FileChunker::new(&test_file, DEFAULT_CHUNK_SIZE).unwrap();
    let total_chunks = chunker.num_chunks();
    assert_eq!(total_chunks, 4); // 1MB / 256KB = 4 chunks

    // Reassemble
    let output_file = temp_dir.path().join("output.dat");
    let mut reassembler =
        FileReassembler::new(&output_file, test_data.len() as u64, DEFAULT_CHUNK_SIZE).unwrap();

    let mut chunk_index = 0;
    while let Some(chunk) = chunker.read_chunk().unwrap() {
        reassembler.write_chunk(chunk_index, &chunk).unwrap();
        chunk_index += 1;
    }

    assert!(reassembler.is_complete());
    reassembler.finalize().unwrap();

    // Verify
    let reconstructed = std::fs::read(&output_file).unwrap();
    assert_eq!(reconstructed, test_data);
}

/// Test tree hash verification integration
#[test]
fn test_tree_hash_verification_integration() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.dat");
    let test_data = vec![0xBB; 512 * 1024]; // 512 KB
    std::fs::write(&test_file, &test_data).unwrap();

    // Compute tree hash
    let tree = compute_tree_hash(&test_file, DEFAULT_CHUNK_SIZE).unwrap();
    assert_eq!(tree.chunks.len(), 2); // 512KB / 256KB = 2 chunks

    // Verify first chunk
    let mut chunker = FileChunker::new(&test_file, DEFAULT_CHUNK_SIZE).unwrap();
    let chunk = chunker.read_chunk().unwrap().unwrap();
    assert!(tree.verify_chunk(0, &chunk));

    // Verify second chunk
    let chunk2 = chunker.read_chunk().unwrap().unwrap();
    assert!(tree.verify_chunk(1, &chunk2));
}

/// Test transfer session progress tracking
#[test]
fn test_transfer_session_progress() {
    let session = TransferSession::new_receive(
        [1u8; 32],
        PathBuf::from("/tmp/test.dat"),
        1024 * 1024, // 1 MB
        DEFAULT_CHUNK_SIZE,
    );

    assert_eq!(session.state(), TransferState::Initializing);
    assert_eq!(session.progress(), 0.0);
    assert_eq!(session.missing_count(), 4); // 4 chunks missing
}

/// Test multi-peer coordination
#[test]
fn test_multi_peer_coordination() {
    let mut session = TransferSession::new_receive(
        [2u8; 32],
        PathBuf::from("/tmp/multi.dat"),
        10 * DEFAULT_CHUNK_SIZE as u64,
        DEFAULT_CHUNK_SIZE,
    );

    let peer1 = [1u8; 32];
    let peer2 = [2u8; 32];

    session.add_peer(peer1);
    session.add_peer(peer2);

    assert_eq!(session.peer_count(), 2);

    // Assign chunks to different peers
    session.assign_chunk_to_peer(&peer1, 0);
    session.assign_chunk_to_peer(&peer1, 1);
    session.assign_chunk_to_peer(&peer2, 2);
    session.assign_chunk_to_peer(&peer2, 3);

    // Next unassigned chunk should be 4
    assert_eq!(session.next_chunk_to_request(), Some(4));

    // Mark chunks as downloaded
    session.mark_peer_chunk_downloaded(&peer1, 0);
    session.mark_peer_chunk_downloaded(&peer2, 2);

    assert_eq!(session.peer_downloaded_count(&peer1), 1);
    assert_eq!(session.peer_downloaded_count(&peer2), 1);
}

/// Placeholder: Full file transfer test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_file_transfer_end_to_end() {
    // Placeholder for full end-to-end test
    // Will be implemented in Phase 7 when protocol components are integrated

    // Structure:
    // 1. Start sender and receiver nodes
    // 2. Transfer file
    // 3. Verify received file matches original
    // 4. Check transfer session state
}

/// Placeholder: Resume functionality test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_file_transfer_with_resume() {
    // Placeholder for resume test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Start transfer
    // 2. Interrupt after 50% complete
    // 3. Resume transfer
    // 4. Verify complete file
}

/// Placeholder: Multi-peer download test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_multi_peer_parallel_download() {
    // Placeholder for multi-peer test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Setup multiple sender nodes
    // 2. Download from all simultaneously
    // 3. Verify speedup vs single peer
    // 4. Verify chunk deduplication
}

/// Placeholder: NAT traversal test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_nat_traversal() {
    // Placeholder for NAT traversal test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Simulate NAT scenario
    // 2. Perform STUN/ICE hole punching
    // 3. Establish direct connection
    // 4. Transfer file
}

/// Placeholder: Relay fallback test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_relay_fallback() {
    // Placeholder for relay fallback test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Start relay server
    // 2. Simulate failed direct connection
    // 3. Fall back to relay
    // 4. Transfer file via relay
    // 5. Verify throughput acceptable
}

/// Placeholder: Obfuscation levels test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_obfuscation_levels() {
    // Placeholder for obfuscation test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Transfer with each obfuscation level (none, low, medium, high, paranoid)
    // 2. Verify packets look as expected
    // 3. Measure overhead
}

/// Placeholder: Encryption end-to-end test (requires protocol integration)
#[test]
#[ignore = "Requires full protocol integration (Phase 7)"]
fn test_encryption_end_to_end() {
    // Placeholder for encryption test
    // Will be implemented in Phase 7

    // Structure:
    // 1. Perform Noise_XX handshake
    // 2. Transfer encrypted file
    // 3. Verify decryption at receiver
    // 4. Verify key ratcheting occurred
}
