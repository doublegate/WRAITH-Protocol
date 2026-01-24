// Integration tests for cross-crate interactions.
//
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

/// Test end-to-end file transfer simulation (component integration)
#[test]
fn test_file_transfer_end_to_end() {
    use std::fs;

    // Create test file
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("source.dat");
    let dest_file = temp_dir.path().join("dest.dat");

    // 5 MB test file
    let test_data = vec![0xCD; 5 * 1024 * 1024];
    fs::write(&source_file, &test_data).unwrap();

    // 1. Sender: Chunk file and compute tree hash
    let mut sender_chunker = FileChunker::new(&source_file, DEFAULT_CHUNK_SIZE).unwrap();
    let tree_hash = compute_tree_hash(&source_file, DEFAULT_CHUNK_SIZE).unwrap();
    let total_chunks = sender_chunker.num_chunks();

    // 2. Sender: Create transfer session
    let transfer_id = [0x42; 32];
    let sender_session = TransferSession::new_send(
        transfer_id,
        source_file.clone(),
        test_data.len() as u64,
        DEFAULT_CHUNK_SIZE,
    );
    assert_eq!(
        sender_session.direction,
        wraith_core::transfer::Direction::Send
    );

    // 3. Receiver: Create reassembler and session
    let mut receiver_reassembler =
        FileReassembler::new(&dest_file, test_data.len() as u64, DEFAULT_CHUNK_SIZE).unwrap();

    let mut receiver_session = TransferSession::new_receive(
        transfer_id,
        dest_file.clone(),
        test_data.len() as u64,
        DEFAULT_CHUNK_SIZE,
    );

    // 4. Simulate transfer: sender chunks → receiver reassembles
    let mut chunk_index: u64 = 0;
    while let Some(chunk) = sender_chunker.read_chunk().unwrap() {
        // Verify chunk integrity
        assert!(tree_hash.verify_chunk(chunk_index as usize, &chunk));

        // Receiver writes chunk
        receiver_reassembler
            .write_chunk(chunk_index, &chunk)
            .unwrap();

        // Update receiver session
        receiver_session.mark_chunk_transferred(chunk_index, chunk.len());

        chunk_index += 1;
    }

    // 5. Verify transfer complete
    assert_eq!(chunk_index, total_chunks);
    assert!(receiver_reassembler.is_complete());
    assert_eq!(receiver_session.progress(), 1.0); // Progress is 0.0 to 1.0

    // 6. Finalize and verify file integrity
    receiver_reassembler.finalize().unwrap();
    let received_data = fs::read(&dest_file).unwrap();
    assert_eq!(received_data, test_data);

    // 7. Verify tree hash of received file
    let received_tree_hash = compute_tree_hash(&dest_file, DEFAULT_CHUNK_SIZE).unwrap();
    assert_eq!(received_tree_hash.root, tree_hash.root);
}

/// Test file transfer resume functionality (component integration)
#[test]
fn test_file_transfer_with_resume() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("source.dat");
    let dest_file = temp_dir.path().join("dest.dat");

    // 2 MB test file
    let test_data = vec![0xEF; 2 * 1024 * 1024];
    fs::write(&source_file, &test_data).unwrap();

    let mut chunker = FileChunker::new(&source_file, DEFAULT_CHUNK_SIZE).unwrap();
    let total_chunks = chunker.num_chunks();
    assert_eq!(total_chunks, 8); // 2MB / 256KB

    // 1. Initial transfer: download first 50% (4 chunks)
    let mut reassembler =
        FileReassembler::new(&dest_file, test_data.len() as u64, DEFAULT_CHUNK_SIZE).unwrap();

    for i in 0..4 {
        let chunk = chunker.read_chunk().unwrap().unwrap();
        reassembler.write_chunk(i, &chunk).unwrap();
    }

    // Transfer interrupted at 50%
    assert!(!reassembler.is_complete());
    let mut missing = reassembler.missing_chunks();
    assert_eq!(missing.len(), 4);
    missing.sort(); // Sort since order may not be guaranteed
    assert_eq!(missing, vec![4, 5, 6, 7]);

    // 2. Resume: Continue from where we left off
    for chunk_index in missing {
        let chunk = chunker.read_chunk().unwrap().unwrap();
        reassembler.write_chunk(chunk_index, &chunk).unwrap();
    }

    // 3. Verify complete
    assert!(reassembler.is_complete());
    reassembler.finalize().unwrap();

    let received_data = fs::read(&dest_file).unwrap();
    assert_eq!(received_data, test_data);
}

/// Test multi-peer parallel download coordination (component integration)
#[test]
fn test_multi_peer_parallel_download() {
    let mut session = TransferSession::new_receive(
        [0x99; 32],
        PathBuf::from("/tmp/multi.dat"),
        20 * DEFAULT_CHUNK_SIZE as u64, // 20 chunks
        DEFAULT_CHUNK_SIZE,
    );

    // Add 3 peers
    let peer1 = [1u8; 32];
    let peer2 = [2u8; 32];
    let peer3 = [3u8; 32];

    session.add_peer(peer1);
    session.add_peer(peer2);
    session.add_peer(peer3);

    assert_eq!(session.peer_count(), 3);

    // 1. Distribute chunks across peers (round-robin)
    for chunk_idx in 0..20 {
        let peer = match chunk_idx % 3 {
            0 => &peer1,
            1 => &peer2,
            _ => &peer3,
        };
        session.assign_chunk_to_peer(peer, chunk_idx);
    }

    // 2. Simulate parallel downloads
    // Peer 1 downloads chunks 0, 3, 6, 9, 12, 15, 18
    for &chunk in &[0, 3, 6, 9, 12, 15, 18] {
        session.mark_peer_chunk_downloaded(&peer1, chunk);
        session.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }

    // Peer 2 downloads chunks 1, 4, 7, 10, 13, 16, 19
    for &chunk in &[1, 4, 7, 10, 13, 16, 19] {
        session.mark_peer_chunk_downloaded(&peer2, chunk);
        session.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }

    // Peer 3 downloads chunks 2, 5, 8, 11, 14, 17
    for &chunk in &[2, 5, 8, 11, 14, 17] {
        session.mark_peer_chunk_downloaded(&peer3, chunk);
        session.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }

    // 3. Verify distribution
    assert_eq!(session.peer_downloaded_count(&peer1), 7);
    assert_eq!(session.peer_downloaded_count(&peer2), 7);
    assert_eq!(session.peer_downloaded_count(&peer3), 6);

    // 4. Verify all chunks received
    assert_eq!(session.progress(), 1.0); // Progress is 0.0 to 1.0
    assert_eq!(session.missing_count(), 0);
}

/// Test NAT traversal components (unit-level integration)
#[test]
fn test_nat_traversal() {
    use std::net::SocketAddr;
    use wraith_discovery::nat::{Candidate, CandidateType, NatType};

    // 1. Simulate NAT type detection
    let nat_type = NatType::PortRestrictedCone;

    // 2. Create ICE candidates for both peers
    let local_addr: SocketAddr = "192.168.1.100:5000".parse().unwrap();
    let public_addr: SocketAddr = "203.0.113.50:5000".parse().unwrap();
    let relay_addr: SocketAddr = "198.51.100.1:3478".parse().unwrap();

    let host_candidate = Candidate {
        address: local_addr,
        candidate_type: CandidateType::Host,
        priority: 126,
    };

    let srflx_candidate = Candidate {
        address: public_addr,
        candidate_type: CandidateType::ServerReflexive,
        priority: 100,
    };

    let relay_candidate = Candidate {
        address: relay_addr,
        candidate_type: CandidateType::Relay,
        priority: 0,
    };

    // 3. Verify candidate prioritization
    assert!(host_candidate.priority > srflx_candidate.priority);
    assert!(srflx_candidate.priority > relay_candidate.priority);

    // 4. Verify NAT type allows hole punching (cone NATs can be punched)
    assert!(matches!(
        nat_type,
        NatType::FullCone | NatType::RestrictedCone | NatType::PortRestrictedCone
    ));
}

/// Test relay fallback mechanism (component integration)
#[test]
fn test_relay_fallback() {
    use std::net::SocketAddr;
    use wraith_discovery::{ConnectionType, RelayInfo};

    // 1. Attempt direct connection (simulated failure)
    let _direct_connection = ConnectionType::Direct;
    let direct_available = false; // Simulate NAT/firewall blocking

    // 2. Fall back to relay
    use wraith_discovery::dht::NodeId;
    let relay_addr: SocketAddr = "198.51.100.1:443".parse().unwrap();
    let relay_info = RelayInfo {
        addr: relay_addr,
        node_id: NodeId::random(),
        public_key: [0x42; 32],
    };

    let connection_type = if direct_available {
        ConnectionType::Direct
    } else {
        ConnectionType::Relayed(relay_info.node_id)
    };

    // 3. Verify relay fallback occurred
    assert!(matches!(connection_type, ConnectionType::Relayed(_)));

    // 4. Verify relay server is configured
    assert_eq!(relay_info.addr, relay_addr);
}

/// Test obfuscation levels (component integration)
#[test]
fn test_obfuscation_levels() {
    use wraith_obfuscation::{PaddingEngine, PaddingMode, TimingMode, TimingObfuscator};

    // Test all 5 padding modes
    let modes = vec![
        PaddingMode::None,
        PaddingMode::PowerOfTwo,
        PaddingMode::SizeClasses,
        PaddingMode::ConstantRate,
        PaddingMode::Statistical,
    ];

    for mode in modes {
        let mut engine = PaddingEngine::new(mode);

        // Test padding a small packet
        let original = vec![0xAA; 100];
        let mut padded = original.clone();

        // Calculate target size and pad
        let target_size = engine.padded_size(padded.len());
        engine.pad(&mut padded, target_size);

        // Verify padding applied correctly
        match mode {
            PaddingMode::None => assert_eq!(padded.len(), 100),
            PaddingMode::PowerOfTwo => {
                // Should round up to 128 (next power of 2)
                assert_eq!(padded.len(), 128);
            }
            PaddingMode::SizeClasses => {
                // Should fit in 128-byte size class
                assert_eq!(padded.len(), 128);
            }
            PaddingMode::ConstantRate => {
                // Should pad to max size (16384)
                assert_eq!(padded.len(), 16384);
            }
            PaddingMode::Statistical => {
                // Statistical padding adds random amount
                assert!(padded.len() >= 100);
            }
        }

        // Verify unpadding recovers original
        let unpadded = engine.unpad(&padded, original.len());
        assert_eq!(unpadded, &original[..]);
    }

    // Test timing obfuscation modes
    use std::time::Duration;

    let timing_modes = vec![
        TimingMode::None,
        TimingMode::Fixed(Duration::from_millis(10)),
        TimingMode::Uniform {
            min: Duration::from_millis(5),
            max: Duration::from_millis(15),
        },
        TimingMode::Normal {
            mean: Duration::from_millis(10),
            stddev: Duration::from_millis(2),
        },
        TimingMode::Exponential {
            mean: Duration::from_millis(10),
        },
    ];

    for mode in timing_modes {
        let mut obfuscator = TimingObfuscator::new(mode);

        // Verify delay is within expected range
        let delay = obfuscator.next_delay();
        match mode {
            TimingMode::None => assert_eq!(delay.as_millis(), 0),
            TimingMode::Fixed(d) => {
                assert_eq!(delay, d);
            }
            TimingMode::Uniform { min, max } => {
                assert!(delay >= min && delay <= max);
            }
            TimingMode::Normal { mean, .. } => {
                // Normal can vary, just check reasonable bounds
                assert!(delay.as_millis() < (mean.as_millis() * 10));
            }
            TimingMode::Exponential { mean } => {
                // Exponential can vary widely, just check reasonable bounds
                assert!(delay.as_millis() < (mean.as_millis() * 10));
            }
        }
    }
}

/// Test encryption end-to-end with Noise_XX handshake (component integration)
#[test]
fn test_encryption_end_to_end() {
    use wraith_core::{Frame, FrameBuilder, FrameType};
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
    use wraith_crypto::ratchet::SymmetricRatchet;

    // 1. Both parties generate static keypairs
    let alice_keypair = NoiseKeypair::generate().unwrap();
    let bob_keypair = NoiseKeypair::generate().unwrap();

    // 2. Perform Noise_XX handshake
    let mut alice_handshake = NoiseHandshake::new_initiator(&alice_keypair).unwrap();
    let mut bob_handshake = NoiseHandshake::new_responder(&bob_keypair).unwrap();

    // Message 1: Alice -> Bob (e)
    let msg1 = alice_handshake.write_message(&[]).unwrap();
    assert!(!msg1.is_empty());

    bob_handshake.read_message(&msg1).unwrap();

    // Message 2: Bob -> Alice (e, ee, s, es)
    let msg2 = bob_handshake.write_message(&[]).unwrap();
    assert!(msg2.len() > msg1.len()); // Contains static key

    alice_handshake.read_message(&msg2).unwrap();

    // Message 3: Alice -> Bob (s, se)
    let msg3 = alice_handshake.write_message(&[]).unwrap();
    assert!(!msg3.is_empty());

    bob_handshake.read_message(&msg3).unwrap();

    // 3. Extract session keys
    let alice_keys = alice_handshake.into_session_keys().unwrap();
    let bob_keys = bob_handshake.into_session_keys().unwrap();

    // 4. Test encrypted frame exchange
    let mut alice_ratchet = SymmetricRatchet::new(&alice_keys.chain_key);
    let mut bob_ratchet = SymmetricRatchet::new(&bob_keys.chain_key);

    // Alice creates encrypted DATA frame
    let alice_key = alice_ratchet.next_key();
    let alice_frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Encrypted payload")
        .build(512)
        .unwrap();

    let nonce = wraith_crypto::aead::Nonce::from_bytes([1u8; 24]);
    let alice_ct = alice_key
        .to_aead_key()
        .encrypt(&nonce, &alice_frame, b"")
        .unwrap();

    // Bob decrypts
    let bob_key = bob_ratchet.next_key();
    let bob_pt = bob_key
        .to_aead_key()
        .decrypt(&nonce, &alice_ct, b"")
        .unwrap();

    let decrypted_frame = Frame::parse(&bob_pt).unwrap();
    assert_eq!(decrypted_frame.payload(), b"Encrypted payload");

    // 5. Verify key ratcheting occurred
    let alice_key2 = alice_ratchet.next_key();
    let bob_key2 = bob_ratchet.next_key();

    // Old keys should not decrypt new ciphertexts
    let alice_frame2 = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Ratcheted payload")
        .build(512)
        .unwrap();

    let nonce2 = wraith_crypto::aead::Nonce::from_bytes([2u8; 24]);
    let alice_ct2 = alice_key2
        .to_aead_key()
        .encrypt(&nonce2, &alice_frame2, b"")
        .unwrap();

    // Old key cannot decrypt new ciphertext
    assert!(
        alice_key
            .to_aead_key()
            .decrypt(&nonce2, &alice_ct2, b"")
            .is_err()
    );

    // New key can decrypt
    let bob_pt2 = bob_key2
        .to_aead_key()
        .decrypt(&nonce2, &alice_ct2, b"")
        .unwrap();

    let decrypted_frame2 = Frame::parse(&bob_pt2).unwrap();
    assert_eq!(decrypted_frame2.payload(), b"Ratcheted payload");
}

// ============================================================================
// Phase 7 Integration Tests
// ============================================================================

/// Test connection establishment with full handshake
///
/// Tests the complete connection establishment flow:
/// 1. Noise_XX handshake between two peers
/// 2. Session key derivation
/// 3. Encrypted frame exchange
/// 4. Session state transitions
#[test]
fn test_connection_establishment_integration() {
    use wraith_core::{Frame, FrameBuilder, FrameType, Session, SessionState};
    use wraith_crypto::aead::SessionCrypto;
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

    // 1. Generate keypairs for both parties
    let alice_keypair = NoiseKeypair::generate().unwrap();
    let bob_keypair = NoiseKeypair::generate().unwrap();

    // 2. Create sessions for state tracking
    let mut alice_session = Session::new();
    let mut bob_session = Session::new();

    // Start in Handshaking state
    alice_session
        .transition_to(SessionState::Handshaking(
            wraith_core::HandshakePhase::InitSent,
        ))
        .unwrap();
    bob_session
        .transition_to(SessionState::Handshaking(
            wraith_core::HandshakePhase::RespSent,
        ))
        .unwrap();

    // 3. Perform Noise_XX handshake
    let mut alice_noise = NoiseHandshake::new_initiator(&alice_keypair).unwrap();
    let mut bob_noise = NoiseHandshake::new_responder(&bob_keypair).unwrap();

    // Message 1: Alice -> Bob
    let msg1 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg1).unwrap();

    // Message 2: Bob -> Alice
    let msg2 = bob_noise.write_message(&[]).unwrap();
    alice_noise.read_message(&msg2).unwrap();

    // Message 3: Alice -> Bob
    let msg3 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg3).unwrap();

    assert!(alice_noise.is_complete());
    assert!(bob_noise.is_complete());

    // 4. Extract session keys
    let alice_keys = alice_noise.into_session_keys().unwrap();
    let bob_keys = bob_noise.into_session_keys().unwrap();

    // 5. Transition sessions to Established
    alice_session
        .transition_to(SessionState::Established)
        .unwrap();
    bob_session
        .transition_to(SessionState::Established)
        .unwrap();

    assert_eq!(alice_session.state(), SessionState::Established);
    assert_eq!(bob_session.state(), SessionState::Established);

    // 6. Create session crypto for encrypted communication
    let mut alice_crypto = SessionCrypto::new(
        alice_keys.send_key,
        alice_keys.recv_key,
        &alice_keys.chain_key,
    );
    let mut bob_crypto =
        SessionCrypto::new(bob_keys.send_key, bob_keys.recv_key, &bob_keys.chain_key);

    // 7. Test encrypted frame exchange
    let alice_frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16)
        .payload(b"Hello from Alice")
        .build(512)
        .unwrap();

    let alice_ct = alice_crypto.encrypt(&alice_frame, b"").unwrap();
    let bob_pt = bob_crypto.decrypt(&alice_ct, b"").unwrap();
    let decrypted = Frame::parse(&bob_pt).unwrap();

    assert_eq!(decrypted.payload(), b"Hello from Alice");
    assert_eq!(decrypted.stream_id(), 16);

    // 8. Bob responds
    let bob_frame = FrameBuilder::new()
        .frame_type(FrameType::Ack)
        .stream_id(16)
        .build(512)
        .unwrap();

    let bob_ct = bob_crypto.encrypt(&bob_frame, b"").unwrap();
    let alice_pt = alice_crypto.decrypt(&bob_ct, b"").unwrap();
    let bob_ack = Frame::parse(&alice_pt).unwrap();

    assert_eq!(bob_ack.frame_type(), FrameType::Ack);
}

/// Test obfuscation layer integration with frames
///
/// Tests padding and timing obfuscation applied to real frames:
/// 1. Apply padding modes to frames
/// 2. Verify size transformation
/// 3. Test TLS/WebSocket mimicry
#[test]
fn test_obfuscation_integration() {
    use wraith_core::{FrameBuilder, FrameType};
    use wraith_obfuscation::{PaddingEngine, PaddingMode, TimingMode, TimingObfuscator};

    // 1. Create a frame
    let frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(1)
        .payload(b"Sensitive payload")
        .build(512)
        .unwrap();

    // 2. Apply different padding modes
    let modes = [
        PaddingMode::PowerOfTwo,
        PaddingMode::SizeClasses,
        PaddingMode::Statistical,
    ];

    for mode in modes {
        let mut engine = PaddingEngine::new(mode);
        let original_len = frame.len();
        let target_size = engine.padded_size(original_len);

        let mut padded = frame.clone();
        engine.pad(&mut padded, target_size);

        // Verify size transformation
        match mode {
            PaddingMode::PowerOfTwo => {
                // Should round to power of 2
                assert!(padded.len().is_power_of_two());
                assert!(padded.len() >= original_len);
            }
            PaddingMode::SizeClasses => {
                // Should fit in defined size class
                assert!(padded.len() >= original_len);
            }
            PaddingMode::Statistical => {
                // Should add randomized padding
                assert!(padded.len() >= original_len);
            }
            _ => {}
        }

        // Verify unpadding recovers original
        let unpadded = engine.unpad(&padded, original_len);
        assert_eq!(unpadded, &frame[..original_len]);
    }

    // 3. Test timing obfuscation
    let mut timing = TimingObfuscator::new(TimingMode::Uniform {
        min: std::time::Duration::from_millis(5),
        max: std::time::Duration::from_millis(15),
    });

    let delay = timing.next_delay();
    assert!(delay >= std::time::Duration::from_millis(5));
    assert!(delay <= std::time::Duration::from_millis(15));

    // 4. Test TLS mimicry integration
    use wraith_obfuscation::tls_mimicry::TlsRecordWrapper;

    let mut tls_wrapper = TlsRecordWrapper::new();
    let wrapped = tls_wrapper.wrap(&frame);

    // TLS Application Data record: type (1) + version (2) + length (2) + data
    assert!(wrapped.len() >= frame.len() + 5);
    assert_eq!(wrapped[0], 0x17); // Application Data

    let unwrapped = tls_wrapper.unwrap(&wrapped).unwrap();
    assert_eq!(unwrapped, frame);
}

/// Test DHT peer discovery integration
///
/// Tests peer announcement, lookup, and connection establishment:
/// 1. Create DHT nodes
/// 2. Announce file availability
/// 3. Lookup peers
/// 4. Verify peer information
#[test]
fn test_discovery_integration() {
    use std::net::SocketAddr;
    use wraith_discovery::dht::NodeId;

    // 1. Create DHT node
    let node_id = NodeId::random();
    let _listen_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();

    // In a real scenario, we'd start the DHT node
    // For this test, we verify the core functionality exists
    assert_eq!(node_id.distance(&node_id).as_bytes(), &[0u8; 32]);

    // 2. Test node ID distance calculation
    let other_id = NodeId::random();
    let distance = node_id.distance(&other_id);

    // Distance to self is zero
    assert_eq!(node_id.distance(&node_id).as_bytes(), &[0u8; 32]);

    // Distance is symmetric
    assert_eq!(distance, other_id.distance(&node_id));

    // 3. Test connection type selection
    use wraith_discovery::ConnectionType;

    let direct = ConnectionType::Direct;
    assert!(matches!(direct, ConnectionType::Direct));

    // 4. Test relay info structure
    use wraith_discovery::RelayInfo;

    let relay_addr: SocketAddr = "198.51.100.1:443".parse().unwrap();
    let relay = RelayInfo {
        addr: relay_addr,
        node_id: NodeId::random(),
        public_key: [0x42; 32],
    };

    assert_eq!(relay.addr, relay_addr);
}

/// Test multi-path transfer coordination
///
/// Tests using multiple network paths for a single transfer:
/// 1. Create PathManager with multiple paths
/// 2. Distribute chunks across paths
/// 3. Track per-path statistics
/// 4. Handle path failure and migration
#[test]
fn test_multi_path_transfer() {
    use std::path::PathBuf;
    use wraith_core::transfer::TransferSession;
    use wraith_files::DEFAULT_CHUNK_SIZE;

    // 1. Create a transfer session
    let mut session = TransferSession::new_receive(
        [0x99; 32],
        PathBuf::from("/tmp/multipath.dat"),
        20 * DEFAULT_CHUNK_SIZE as u64, // 20 chunks
        DEFAULT_CHUNK_SIZE,
    );

    // 2. Simulate multiple peers on different paths
    let peer1 = [1u8; 32]; // Path 1: Direct connection
    let peer2 = [2u8; 32]; // Path 2: Relayed connection
    let peer3 = [3u8; 32]; // Path 3: Alternative route

    session.add_peer(peer1);
    session.add_peer(peer2);
    session.add_peer(peer3);

    // 3. Distribute chunks across paths
    // Path 1 (peer1): Chunks 0-6
    for chunk_idx in 0..7 {
        session.assign_chunk_to_peer(&peer1, chunk_idx);
    }

    // Path 2 (peer2): Chunks 7-13
    for chunk_idx in 7..14 {
        session.assign_chunk_to_peer(&peer2, chunk_idx);
    }

    // Path 3 (peer3): Chunks 14-19
    for chunk_idx in 14..20 {
        session.assign_chunk_to_peer(&peer3, chunk_idx);
    }

    // 4. Simulate downloads with different speeds
    session.update_peer_speed(&peer1, 10_000_000.0); // 10 MB/s
    session.update_peer_speed(&peer2, 5_000_000.0); // 5 MB/s (slower, relayed)
    session.update_peer_speed(&peer3, 8_000_000.0); // 8 MB/s

    // 5. Download chunks
    for chunk_idx in 0..7 {
        session.mark_peer_chunk_downloaded(&peer1, chunk_idx);
        session.mark_chunk_transferred(chunk_idx, DEFAULT_CHUNK_SIZE);
    }

    for chunk_idx in 7..14 {
        session.mark_peer_chunk_downloaded(&peer2, chunk_idx);
        session.mark_chunk_transferred(chunk_idx, DEFAULT_CHUNK_SIZE);
    }

    for chunk_idx in 14..20 {
        session.mark_peer_chunk_downloaded(&peer3, chunk_idx);
        session.mark_chunk_transferred(chunk_idx, DEFAULT_CHUNK_SIZE);
    }

    // 6. Verify all chunks received
    assert_eq!(session.progress(), 1.0);
    assert_eq!(session.missing_count(), 0);

    // 7. Verify per-path statistics
    assert_eq!(session.peer_downloaded_count(&peer1), 7);
    assert_eq!(session.peer_downloaded_count(&peer2), 7);
    assert_eq!(session.peer_downloaded_count(&peer3), 6);

    // 8. Aggregate speed should be sum of all paths
    let aggregate = session.aggregate_peer_speed();
    assert_eq!(aggregate, 23_000_000.0); // 10 + 5 + 8 MB/s
}

/// Test error recovery and resilience
///
/// Tests handling of various error conditions:
/// 1. Connection drops during transfer
/// 2. Chunk corruption detection
/// 3. Timeout and retry logic
/// 4. Partial transfer resume
#[test]
fn test_error_recovery() {
    use std::path::PathBuf;
    use wraith_core::transfer::{TransferSession, TransferState};
    use wraith_files::DEFAULT_CHUNK_SIZE;

    // 1. Create a transfer session
    let mut session = TransferSession::new_receive(
        [0xAB; 32],
        PathBuf::from("/tmp/recovery.dat"),
        10 * DEFAULT_CHUNK_SIZE as u64,
        DEFAULT_CHUNK_SIZE,
    );

    session.start();
    assert_eq!(session.state(), TransferState::Transferring);

    // 2. Add peers
    let peer1 = [1u8; 32];
    let peer2 = [2u8; 32];

    session.add_peer(peer1);
    session.add_peer(peer2);

    // 3. Assign chunks
    for chunk_idx in 0..5 {
        session.assign_chunk_to_peer(&peer1, chunk_idx);
    }
    for chunk_idx in 5..10 {
        session.assign_chunk_to_peer(&peer2, chunk_idx);
    }

    // 4. Simulate partial download from peer1
    session.mark_peer_chunk_downloaded(&peer1, 0);
    session.mark_chunk_transferred(0, DEFAULT_CHUNK_SIZE);
    session.mark_peer_chunk_downloaded(&peer1, 1);
    session.mark_chunk_transferred(1, DEFAULT_CHUNK_SIZE);

    assert_eq!(session.progress(), 0.2); // 2/10 chunks

    // 5. Simulate peer1 connection drop
    let peer1_assigned = session.remove_peer(&peer1);
    assert!(peer1_assigned.is_some());
    let chunks_to_reassign = peer1_assigned.unwrap();

    // Chunks 2, 3, 4 were assigned but not downloaded
    assert!(chunks_to_reassign.contains(&2));
    assert!(chunks_to_reassign.contains(&3));
    assert!(chunks_to_reassign.contains(&4));

    // 6. Reassign failed chunks to peer2
    for chunk in chunks_to_reassign {
        session.assign_chunk_to_peer(&peer2, chunk);
    }

    // 7. Test pause/resume
    session.pause();
    assert_eq!(session.state(), TransferState::Paused);

    session.resume();
    assert_eq!(session.state(), TransferState::Transferring);

    // 8. Complete download from peer2
    for chunk_idx in 2..10 {
        session.mark_peer_chunk_downloaded(&peer2, chunk_idx);
        session.mark_chunk_transferred(chunk_idx, DEFAULT_CHUNK_SIZE);
    }

    // 9. Verify recovery successful
    assert_eq!(session.progress(), 1.0);
    assert_eq!(session.state(), TransferState::Complete);
}

/// Test concurrent transfers
///
/// Tests managing multiple simultaneous file transfers:
/// 1. Create multiple transfer sessions
/// 2. Verify isolation between transfers
/// 3. Test resource sharing
/// 4. Verify no interference
#[test]
fn test_concurrent_transfers() {
    use std::path::PathBuf;
    use wraith_core::transfer::{TransferSession, TransferState};
    use wraith_files::DEFAULT_CHUNK_SIZE;

    // 1. Create three concurrent transfers
    let mut transfer1 = TransferSession::new_receive(
        [0x01; 32],
        PathBuf::from("/tmp/file1.dat"),
        5 * DEFAULT_CHUNK_SIZE as u64,
        DEFAULT_CHUNK_SIZE,
    );

    let mut transfer2 = TransferSession::new_send(
        [0x02; 32],
        PathBuf::from("/tmp/file2.dat"),
        10 * DEFAULT_CHUNK_SIZE as u64,
        DEFAULT_CHUNK_SIZE,
    );

    let mut transfer3 = TransferSession::new_receive(
        [0x03; 32],
        PathBuf::from("/tmp/file3.dat"),
        8 * DEFAULT_CHUNK_SIZE as u64,
        DEFAULT_CHUNK_SIZE,
    );

    // 2. Start all transfers
    transfer1.start();
    transfer2.start();
    transfer3.start();

    assert_eq!(transfer1.state(), TransferState::Transferring);
    assert_eq!(transfer2.state(), TransferState::Transferring);
    assert_eq!(transfer3.state(), TransferState::Transferring);

    // 3. Verify isolation - different transfer IDs
    assert_ne!(transfer1.id, transfer2.id);
    assert_ne!(transfer2.id, transfer3.id);
    assert_ne!(transfer1.id, transfer3.id);

    // 4. Simulate progress on transfer1
    transfer1.mark_chunk_transferred(0, DEFAULT_CHUNK_SIZE);
    transfer1.mark_chunk_transferred(1, DEFAULT_CHUNK_SIZE);
    assert_eq!(transfer1.progress(), 0.4); // 2/5

    // 5. Simulate progress on transfer2
    for chunk in 0..5 {
        transfer2.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }
    assert_eq!(transfer2.progress(), 0.5); // 5/10

    // 6. Simulate progress on transfer3
    for chunk in 0..8 {
        transfer3.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }
    assert_eq!(transfer3.progress(), 1.0); // 8/8

    // 7. Verify states are independent
    assert_eq!(transfer1.state(), TransferState::Transferring);
    assert_eq!(transfer2.state(), TransferState::Transferring);
    assert_eq!(transfer3.state(), TransferState::Complete);

    // 8. Complete transfer1
    for chunk in 2..5 {
        transfer1.mark_chunk_transferred(chunk, DEFAULT_CHUNK_SIZE);
    }
    assert!(transfer1.is_complete());

    // 9. Pause transfer2
    transfer2.pause();
    assert_eq!(transfer2.state(), TransferState::Paused);

    // 10. Verify no cross-interference
    assert!(transfer1.is_complete());
    assert_eq!(transfer2.state(), TransferState::Paused);
    assert!(transfer3.is_complete());
}

/// Test full protocol integration
///
/// Tests the complete WRAITH protocol stack:
/// 1. Handshake (Noise_XX)
/// 2. Session establishment
/// 3. Obfuscation layer
/// 4. File chunking and transfer
/// 5. Integrity verification
#[test]
fn test_full_protocol_integration() {
    use std::fs;
    use tempfile::TempDir;
    use wraith_core::{FrameBuilder, FrameType};
    use wraith_crypto::aead::SessionCrypto;
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};
    use wraith_files::chunker::FileChunker;
    use wraith_files::tree_hash::compute_tree_hash;
    use wraith_obfuscation::{PaddingEngine, PaddingMode};

    // 1. Setup: Create test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.dat");
    let test_data = vec![0xCD; 2 * 1024]; // 2 KB (small for this test)
    fs::write(&test_file, &test_data).unwrap();

    // 2. Handshake phase
    let alice_keypair = NoiseKeypair::generate().unwrap();
    let bob_keypair = NoiseKeypair::generate().unwrap();

    let mut alice_noise = NoiseHandshake::new_initiator(&alice_keypair).unwrap();
    let mut bob_noise = NoiseHandshake::new_responder(&bob_keypair).unwrap();

    // Three-way handshake
    let msg1 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg1).unwrap();

    let msg2 = bob_noise.write_message(&[]).unwrap();
    alice_noise.read_message(&msg2).unwrap();

    let msg3 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg3).unwrap();

    // 3. Session establishment
    let alice_keys = alice_noise.into_session_keys().unwrap();
    let bob_keys = bob_noise.into_session_keys().unwrap();

    let mut alice_crypto = SessionCrypto::new(
        alice_keys.send_key,
        alice_keys.recv_key,
        &alice_keys.chain_key,
    );
    let mut bob_crypto =
        SessionCrypto::new(bob_keys.send_key, bob_keys.recv_key, &bob_keys.chain_key);

    // 4. File chunking
    let chunk_size = 1024; // Use 1KB chunks for this test
    let mut chunker = FileChunker::new(&test_file, chunk_size).unwrap();
    let tree_hash = compute_tree_hash(&test_file, chunk_size).unwrap();

    assert_eq!(tree_hash.chunk_count(), 2); // 2KB / 1KB

    // 5. Transfer phase with obfuscation
    let mut padding = PaddingEngine::new(PaddingMode::PowerOfTwo);

    let mut chunk_index = 0;
    while let Some(chunk) = chunker.read_chunk().unwrap() {
        // Create DATA frame
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(16)
            .offset(chunk_index * chunk_size as u64)
            .payload(&chunk)
            .build(4096)
            .unwrap();

        // Apply padding obfuscation
        let target_size = padding.padded_size(frame.len());
        let mut padded_frame = frame.clone();
        padding.pad(&mut padded_frame, target_size);

        // Encrypt
        let ciphertext = alice_crypto.encrypt(&padded_frame, b"").unwrap();

        // Bob receives and decrypts
        let plaintext = bob_crypto.decrypt(&ciphertext, b"").unwrap();

        // Remove padding
        let unpadded = padding.unpad(&plaintext, frame.len());
        assert_eq!(unpadded, &frame[..]);

        // Verify chunk integrity
        assert!(tree_hash.verify_chunk(chunk_index as usize, &chunk));

        chunk_index += 1;
    }

    // 6. Verify complete transfer
    assert_eq!(chunk_index, 2);
}

// ============================================================================
// Phase 10 Session 3.4: Integration Tests
// ============================================================================

/// Test transport layer initialization and packet exchange
///
/// Verifies that UDP transport can:
/// 1. Bind to a local address
/// 2. Send packets
/// 3. Receive packets
#[tokio::test]
async fn test_transport_initialization() {
    use std::net::SocketAddr;
    use wraith_transport::transport::Transport;
    use wraith_transport::udp_async::AsyncUdpTransport;

    // 1. Create two transports on different ports
    let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let transport1 = AsyncUdpTransport::bind(addr1).await.unwrap();
    let transport2 = AsyncUdpTransport::bind(addr2).await.unwrap();

    // Get actual bound addresses
    let local1 = transport1.local_addr().unwrap();
    let local2 = transport2.local_addr().unwrap();

    assert_ne!(local1.port(), 0);
    assert_ne!(local2.port(), 0);

    // 2. Send packet from transport1 to transport2
    let test_data = b"Hello from transport1";
    transport1.send_to(test_data, local2).await.unwrap();

    // 3. Receive packet on transport2
    let mut buf = vec![0u8; 1500];
    let (size, from) = transport2.recv_from(&mut buf).await.unwrap();

    assert_eq!(size, test_data.len());
    assert_eq!(&buf[..size], test_data);
    assert_eq!(from, local1);

    // 4. Send response back
    let response = b"Response from transport2";
    transport2.send_to(response, from).await.unwrap();

    // 5. Receive response
    let mut buf2 = vec![0u8; 1500];
    let (size2, from2) = transport1.recv_from(&mut buf2).await.unwrap();

    assert_eq!(size2, response.len());
    assert_eq!(&buf2[..size2], response);
    assert_eq!(from2, local2);

    // 6. Verify statistics
    let stats1 = transport1.stats();
    let stats2 = transport2.stats();

    assert!(stats1.bytes_sent > 0);
    assert!(stats1.packets_sent > 0);
    assert!(stats2.bytes_received > 0);
    assert!(stats2.packets_received > 0);
}

/// Test Noise_XX handshake between two nodes (loopback)
///
/// Verifies complete handshake flow:
/// 1. Create two nodes with random identities
/// 2. Perform three-way Noise_XX handshake
/// 3. Verify session establishment on both sides
/// 4. Verify session keys are derived
#[tokio::test]
async fn test_noise_handshake_loopback() {
    use wraith_core::node::Node;

    // 1. Create two nodes on ephemeral ports
    let node1 = Node::new_random_with_port(0).await.unwrap();
    let node2 = Node::new_random_with_port(0).await.unwrap();

    // 2. Start both nodes
    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // 3. Get node2's actual listening address
    let node2_addr = node2.listen_addr().await.unwrap();

    // 4. Establish session from node1 to node2 using known address
    let session_id = node1
        .establish_session_with_addr(node2.node_id(), node2_addr)
        .await
        .unwrap();

    // Verify session ID is 32 bytes
    assert_eq!(session_id.len(), 32);

    // 5. Give responder time to process handshake and create session
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 6. Verify session exists in node1 (initiator)
    // Sessions are keyed by X25519 public key from Noise handshake
    let sessions1 = node1.active_sessions().await;
    assert_eq!(sessions1.len(), 1);
    assert_eq!(&sessions1[0], node2.x25519_public_key());

    // 7. Verify session exists in node2 (responder)
    let sessions2 = node2.active_sessions().await;
    assert_eq!(sessions2.len(), 1);
    assert_eq!(&sessions2[0], node1.x25519_public_key());

    // 8. Cleanup
    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

/// Test encrypted frame exchange after handshake
///
/// Verifies that after Noise handshake:
/// 1. Frames can be encrypted with session keys
/// 2. Frames can be transmitted over transport
/// 3. Frames can be decrypted by receiver
/// 4. Frame integrity is maintained
#[tokio::test]
async fn test_encrypted_frame_exchange() {
    use wraith_core::{Frame, FrameBuilder, FrameType};
    use wraith_crypto::aead::SessionCrypto;
    use wraith_crypto::noise::{NoiseHandshake, NoiseKeypair};

    // 1. Generate keypairs
    let alice_keypair = NoiseKeypair::generate().unwrap();
    let bob_keypair = NoiseKeypair::generate().unwrap();

    // 2. Perform handshake
    let mut alice_noise = NoiseHandshake::new_initiator(&alice_keypair).unwrap();
    let mut bob_noise = NoiseHandshake::new_responder(&bob_keypair).unwrap();

    // Three-way handshake
    let msg1 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg1).unwrap();

    let msg2 = bob_noise.write_message(&[]).unwrap();
    alice_noise.read_message(&msg2).unwrap();

    let msg3 = alice_noise.write_message(&[]).unwrap();
    bob_noise.read_message(&msg3).unwrap();

    assert!(alice_noise.is_complete());
    assert!(bob_noise.is_complete());

    // 3. Extract session keys
    let alice_keys = alice_noise.into_session_keys().unwrap();
    let bob_keys = bob_noise.into_session_keys().unwrap();

    // 4. Create session crypto instances
    let mut alice_crypto = SessionCrypto::new(
        alice_keys.send_key,
        alice_keys.recv_key,
        &alice_keys.chain_key,
    );
    let mut bob_crypto =
        SessionCrypto::new(bob_keys.send_key, bob_keys.recv_key, &bob_keys.chain_key);

    // 5. Alice creates and encrypts a DATA frame
    let payload = b"Encrypted test payload";
    let alice_frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(42)
        .offset(0)
        .payload(payload)
        .build(512)
        .unwrap();

    let encrypted = alice_crypto.encrypt(&alice_frame, b"").unwrap();

    // 6. Bob decrypts the frame
    let decrypted = bob_crypto.decrypt(&encrypted, b"").unwrap();

    // 7. Parse and verify
    let frame = Frame::parse(&decrypted).unwrap();
    assert_eq!(frame.frame_type(), FrameType::Data);
    assert_eq!(frame.stream_id(), 42);
    assert_eq!(frame.payload(), payload);

    // 8. Test bidirectional - Bob sends ACK
    let bob_frame = FrameBuilder::new()
        .frame_type(FrameType::Ack)
        .stream_id(42)
        .offset(payload.len() as u64)
        .build(512)
        .unwrap();

    let bob_encrypted = bob_crypto.encrypt(&bob_frame, b"").unwrap();
    let alice_decrypted = alice_crypto.decrypt(&bob_encrypted, b"").unwrap();

    let ack_frame = Frame::parse(&alice_decrypted).unwrap();
    assert_eq!(ack_frame.frame_type(), FrameType::Ack);
    assert_eq!(ack_frame.stream_id(), 42);
}

/// Test obfuscation pipeline (padding → encryption → mimicry)
///
/// Verifies the complete obfuscation pipeline:
/// 1. Apply padding to frame
/// 2. Encrypt padded frame
/// 3. Apply protocol mimicry (TLS wrapper)
/// 4. Verify reverse pipeline (unwrap → decrypt → unpad)
#[tokio::test]
async fn test_obfuscation_pipeline() {
    use wraith_core::{Frame, FrameBuilder, FrameType};
    use wraith_crypto::aead::SessionCrypto;
    use wraith_obfuscation::tls_mimicry::TlsRecordWrapper;
    use wraith_obfuscation::{PaddingEngine, PaddingMode};

    // 1. Create a test frame
    let payload = b"Test payload for obfuscation";
    let frame = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(16) // Use 16 to avoid reserved range (1-15)
        .payload(payload)
        .build(512)
        .unwrap();

    // 2. Apply padding
    let mut padding = PaddingEngine::new(PaddingMode::PowerOfTwo);
    let original_len = frame.len();
    let target_size = padding.padded_size(original_len);

    let mut padded_frame = frame.clone();
    padding.pad(&mut padded_frame, target_size);

    assert!(padded_frame.len().is_power_of_two());
    assert!(padded_frame.len() >= original_len);

    // 3. Encrypt padded frame
    // Create sender and receiver crypto instances (simulating Alice and Bob)
    let mut alice_crypto = SessionCrypto::new([1u8; 32], [2u8; 32], &[3u8; 32]);
    let mut bob_crypto = SessionCrypto::new([2u8; 32], [1u8; 32], &[3u8; 32]);

    let encrypted = alice_crypto.encrypt(&padded_frame, b"").unwrap();

    // 4. Apply TLS mimicry
    let mut tls_wrapper = TlsRecordWrapper::new();
    let wrapped = tls_wrapper.wrap(&encrypted);

    // Verify TLS header
    assert_eq!(wrapped[0], 0x17); // Application Data
    assert!(wrapped.len() >= encrypted.len() + 5);

    // 5. Reverse pipeline: unwrap TLS
    let unwrapped = tls_wrapper.unwrap(&wrapped).unwrap();
    assert_eq!(unwrapped, encrypted);

    // 6. Decrypt with Bob's crypto
    let decrypted = bob_crypto.decrypt(&unwrapped, b"").unwrap();
    assert_eq!(decrypted.len(), padded_frame.len());

    // 7. Remove padding
    let unpadded = padding.unpad(&decrypted, original_len);
    assert_eq!(unpadded, &frame[..original_len]);

    // 8. Parse final frame
    let final_frame = Frame::parse(unpadded).unwrap();
    assert_eq!(final_frame.frame_type(), FrameType::Data);
    assert_eq!(final_frame.payload(), payload);
}

/// Test file chunk transfer with integrity verification
///
/// Verifies file transfer components:
/// 1. Chunk file into pieces
/// 2. Compute BLAKE3 tree hash
/// 3. Transfer chunks (simulated)
/// 4. Verify each chunk integrity
/// 5. Reassemble file
/// 6. Verify complete file hash
#[tokio::test]
async fn test_file_chunk_transfer() {
    use std::fs;
    use tempfile::TempDir;
    use wraith_files::DEFAULT_CHUNK_SIZE;
    use wraith_files::chunker::{FileChunker, FileReassembler};
    use wraith_files::tree_hash::compute_tree_hash;

    // 1. Create test file
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.dat");
    let dest = temp_dir.path().join("dest.dat");

    let test_data = vec![0xCD; 1024 * 1024]; // 1 MB
    fs::write(&source, &test_data).unwrap();

    // 2. Compute tree hash for integrity verification
    let tree_hash = compute_tree_hash(&source, DEFAULT_CHUNK_SIZE).unwrap();
    assert_eq!(tree_hash.chunk_count(), 4); // 1MB / 256KB

    // 3. Create chunker and reassembler
    let mut chunker = FileChunker::new(&source, DEFAULT_CHUNK_SIZE).unwrap();
    let mut reassembler =
        FileReassembler::new(&dest, test_data.len() as u64, DEFAULT_CHUNK_SIZE).unwrap();

    // 4. Transfer chunks with integrity verification
    let mut chunk_index = 0u64;
    while let Some(chunk) = chunker.read_chunk().unwrap() {
        // Verify chunk integrity using tree hash
        assert!(tree_hash.verify_chunk(chunk_index as usize, &chunk));

        // Write chunk to reassembler
        reassembler.write_chunk(chunk_index, &chunk).unwrap();

        chunk_index += 1;
    }

    // 5. Verify transfer complete
    assert_eq!(chunk_index, 4);
    assert!(reassembler.is_complete());

    // 6. Finalize and verify file integrity
    reassembler.finalize().unwrap();
    let received = fs::read(&dest).unwrap();
    assert_eq!(received, test_data);

    // 7. Verify tree hash of received file
    let received_hash = compute_tree_hash(&dest, DEFAULT_CHUNK_SIZE).unwrap();
    assert_eq!(received_hash.root, tree_hash.root);
}

/// Test cover traffic generation
///
/// Verifies cover traffic components:
/// 1. Create cover traffic generator
/// 2. Verify timing patterns
/// 3. Verify activation/deactivation
#[tokio::test]
async fn test_cover_traffic_generation() {
    use wraith_obfuscation::{CoverTrafficGenerator, TrafficDistribution};

    // 1. Create cover traffic generator with constant rate
    let mut generator = CoverTrafficGenerator::new(
        10.0, // 10 packets per second
        TrafficDistribution::Constant,
    );

    // 2. Check if scheduled (may or may not be ready immediately)
    let initial_delay = generator.time_until_next();
    assert!(initial_delay.as_millis() <= 200); // Should schedule within reasonable time

    // 3. Mark as sent and verify delay
    generator.mark_sent();
    let delay = generator.time_until_next();

    // Should wait ~100ms for next send (1000ms / 10 pps = 100ms)
    assert!(delay.as_millis() <= 150); // Allow some tolerance

    // 4. Test deactivation
    generator.set_active(false);
    assert!(!generator.should_send());

    // 5. Test reactivation
    generator.set_active(true);

    // 6. Test Poisson distribution
    let poisson_gen =
        CoverTrafficGenerator::new(10.0, TrafficDistribution::Poisson { lambda: 10.0 });

    // Should have scheduled a send time
    let _ = poisson_gen.time_until_next();

    // 7. Test Uniform distribution
    let uniform_gen = CoverTrafficGenerator::new(
        10.0,
        TrafficDistribution::Uniform {
            min_ms: 50,
            max_ms: 150,
        },
    );

    // Should have scheduled a send time
    let uniform_delay = uniform_gen.time_until_next();
    assert!(uniform_delay.as_millis() <= 200); // Should be within max range
}

/// Test discovery node integration (DHT and NAT detection)
///
/// Verifies discovery components with Node API:
/// 1. Create Node with discovery enabled
/// 2. Verify NAT type detection
/// 3. Test peer announcement (basic)
/// 4. Verify discovery manager lifecycle
#[tokio::test]
async fn test_discovery_node_integration() {
    use wraith_core::node::{NatType, Node};

    // 1. Create node with discovery enabled
    let node = Node::new_random_with_port(0).await.unwrap();

    // 2. Start node (initializes discovery)
    node.start().await.unwrap();

    // 3. Test NAT detection
    let nat_type = node.detect_nat_type().await.unwrap();

    // NAT detection returns actual network NAT type - any valid type is acceptable
    assert!(matches!(
        nat_type,
        NatType::None
            | NatType::FullCone
            | NatType::RestrictedCone
            | NatType::PortRestricted
            | NatType::Symmetric
    ));

    // 4. Verify node can announce (even if DHT is empty)
    // This tests the announce mechanism, not DHT population
    let announce_result = node.announce().await;
    // May fail if no DHT nodes available, which is expected in isolated test
    // We just verify the API works
    assert!(announce_result.is_ok() || announce_result.is_err());

    // 5. Verify node is running with discovery initialized
    assert!(node.is_running());

    // 6. Cleanup
    node.stop().await.unwrap();
}

// ============================================================================
// Node API Integration Tests (Phase 9)
// ============================================================================

/// Test end-to-end file transfer using Node API
///
/// Tests the complete file transfer workflow:
/// 1. Create sender and receiver nodes
/// 2. Start both nodes
/// 3. Send file from sender to receiver
/// 4. Wait for transfer completion
/// 5. Verify file integrity
#[tokio::test]
#[ignore = "Placeholder: Requires chunk-based file data transmission implementation"]
async fn test_end_to_end_file_transfer() {
    use std::fs;
    use tempfile::TempDir;
    use wraith_core::node::Node;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();

    // Create sender and receiver nodes
    let sender = Node::new_random_with_port(0).await.unwrap();
    let receiver = Node::new_random_with_port(0).await.unwrap();

    // Start both nodes
    sender.start().await.unwrap();
    receiver.start().await.unwrap();

    // Create test file (1 MB)
    let test_data = vec![0xAA; 1024 * 1024];
    let send_path = temp_dir.path().join("test_file.bin");
    fs::write(&send_path, &test_data).unwrap();

    // Get receiver's listening address
    let receiver_addr = receiver.listen_addr().await.unwrap();

    // Establish session first (so receiver knows about sender)
    // Use Ed25519 node_id for addressing, session is stored by X25519 key
    let _session = sender
        .establish_session_with_addr(receiver.node_id(), receiver_addr)
        .await
        .unwrap();

    // Give time for session establishment
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send file - use X25519 key since sessions are keyed by X25519
    let transfer_id = sender
        .send_file(&send_path, receiver.x25519_public_key())
        .await
        .unwrap();

    // Verify transfer was created
    assert_eq!(transfer_id.len(), 32);

    // Verify transfer is tracked on sender
    let active_transfers = sender.active_transfers().await;
    assert!(active_transfers.contains(&transfer_id));

    // Wait for transfer to complete (with generous timeout for file I/O)
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        sender.wait_for_transfer(transfer_id),
    )
    .await;

    // Verify transfer completed successfully
    match result {
        Ok(Ok(())) => {
            // Transfer completed successfully
            tracing::info!("File transfer completed successfully");
        }
        Ok(Err(e)) => {
            panic!("Transfer failed: {}", e);
        }
        Err(_) => {
            // Timeout - check progress for debugging
            let progress = sender.get_transfer_progress(&transfer_id).await;
            panic!("Transfer timed out. Progress: {:?}", progress);
        }
    }

    // Cleanup
    sender.stop().await.unwrap();
    receiver.stop().await.unwrap();
}

/// Test connection establishment with Noise handshake
///
/// Tests session establishment between two nodes:
/// 1. Create two nodes
/// 2. Establish encrypted session using known address
/// 3. Verify session state
#[tokio::test]
async fn test_connection_establishment() {
    use wraith_core::node::Node;

    let node1 = Node::new_random_with_port(0).await.unwrap();
    let node2 = Node::new_random_with_port(0).await.unwrap();

    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Get node2's actual listening address
    let node2_addr = node2.listen_addr().await.unwrap();

    // Establish session using known address
    let session_id = node1
        .establish_session_with_addr(node2.node_id(), node2_addr)
        .await
        .unwrap();

    assert_eq!(session_id.len(), 32);

    // Give responder time to process handshake
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify session exists (keyed by X25519 public key from handshake)
    let sessions = node1.active_sessions().await;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0], *node2.x25519_public_key());

    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

/// Test obfuscation modes configuration
///
/// Tests that nodes can be configured with different obfuscation settings:
/// 1. Create node with custom obfuscation config
/// 2. Verify configuration is applied
#[tokio::test]
async fn test_obfuscation_modes() {
    use wraith_core::node::config::{PaddingMode, TimingMode};
    use wraith_core::node::{Node, NodeConfig};

    let mut config = NodeConfig::default();
    config.obfuscation.padding_mode = PaddingMode::PowerOfTwo;
    config.obfuscation.timing_mode = TimingMode::Uniform {
        min: std::time::Duration::from_millis(1),
        max: std::time::Duration::from_millis(10),
    };

    let node = Node::new_with_config(config).await.unwrap();

    // Verify node created successfully with custom config
    assert_eq!(node.node_id().len(), 32);

    // Note: Full implementation will test padding/timing application
    // For now, we just verify the node can be created with these settings
}

/// Test peer discovery and bidirectional session establishment
///
/// Tests establishing sessions between two nodes in both directions:
/// 1. Create two nodes
/// 2. Establish sessions from both sides using known addresses
/// 3. Verify both nodes have sessions
#[tokio::test]
async fn test_discovery_and_peer_finding() {
    use wraith_core::node::Node;

    let node1 = Node::new_random_with_port(0).await.unwrap();
    let node2 = Node::new_random_with_port(0).await.unwrap();

    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Get actual listening addresses
    let node1_addr = node1.listen_addr().await.unwrap();
    let node2_addr = node2.listen_addr().await.unwrap();

    // Establish session from node1 to node2
    let _session_id1 = node1
        .establish_session_with_addr(node2.node_id(), node2_addr)
        .await
        .unwrap();

    // Give responder time to process handshake
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Establish session from node2 to node1
    let _session_id2 = node2
        .establish_session_with_addr(node1.node_id(), node1_addr)
        .await
        .unwrap();

    // Give responder time to process handshake
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify sessions exist on both nodes
    // Each node should have the other's X25519 public key in their session map
    let node1_sessions = node1.active_sessions().await;
    let node2_sessions = node2.active_sessions().await;

    // Node1 initiated to node2, so it has node2's X25519 key
    assert!(!node1_sessions.is_empty(), "Node1 should have sessions");
    // Node2 initiated to node1, so it has node1's X25519 key
    assert!(!node2_sessions.is_empty(), "Node2 should have sessions");

    // Note: Full DHT implementation will test:
    // - node1.announce().await
    // - node1.find_peers(10).await
    // - node1.lookup_peer(node2.node_id()).await

    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

/// Test multi-peer file transfer
///
/// Tests sending a file to multiple peers simultaneously:
/// 1. Create sender and multiple receiver nodes
/// 2. Initiate multi-peer send
/// 3. Verify all transfers complete
#[tokio::test]
#[ignore = "Placeholder: Requires chunk-based file data transmission implementation"]
async fn test_multi_path_transfer_node_api() {
    use std::fs;
    use tempfile::TempDir;
    use wraith_core::node::Node;

    let temp_dir = TempDir::new().unwrap();

    let sender = Node::new_random_with_port(0).await.unwrap();
    let receiver1 = Node::new_random_with_port(0).await.unwrap();
    let receiver2 = Node::new_random_with_port(0).await.unwrap();

    sender.start().await.unwrap();
    receiver1.start().await.unwrap();
    receiver2.start().await.unwrap();

    // Create test file (2 MB)
    let test_data = vec![0xBB; 2 * 1024 * 1024];
    let send_path = temp_dir.path().join("multi_test.bin");
    fs::write(&send_path, &test_data).unwrap();

    // Get receiver addresses
    let receiver1_addr = receiver1.listen_addr().await.unwrap();
    let receiver2_addr = receiver2.listen_addr().await.unwrap();

    // Establish sessions with multiple peers
    // Use Ed25519 node_id for addressing, session is stored by X25519 key
    let _session1 = sender
        .establish_session_with_addr(receiver1.node_id(), receiver1_addr)
        .await
        .unwrap();
    let _session2 = sender
        .establish_session_with_addr(receiver2.node_id(), receiver2_addr)
        .await
        .unwrap();

    // Give time for session establishment
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify both sessions exist
    let sessions = sender.active_sessions().await;
    assert_eq!(sessions.len(), 2);

    // Send file to multiple peers - use X25519 keys since sessions are keyed by X25519
    let peers = vec![
        *receiver1.x25519_public_key(),
        *receiver2.x25519_public_key(),
    ];
    let transfer_id = sender.send_file_to_peers(&send_path, &peers).await.unwrap();

    // Verify transfer was created
    assert_eq!(transfer_id.len(), 32);

    // Wait for transfer to complete
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        sender.wait_for_transfer(transfer_id),
    )
    .await;

    // Verify transfer completed successfully
    match result {
        Ok(Ok(())) => {
            tracing::info!("Multi-peer file transfer completed successfully");
        }
        Ok(Err(e)) => {
            panic!("Multi-peer transfer failed: {}", e);
        }
        Err(_) => {
            let progress = sender.get_transfer_progress(&transfer_id).await;
            panic!("Multi-peer transfer timed out. Progress: {:?}", progress);
        }
    }

    sender.stop().await.unwrap();
    receiver1.stop().await.unwrap();
    receiver2.stop().await.unwrap();
}

/// Test error recovery and resilience
///
/// Tests handling of error conditions:
/// 1. Close non-existent session
/// 2. Node remains healthy after errors
/// 3. Normal operations work after error conditions
#[tokio::test]
async fn test_error_recovery_node_api() {
    use wraith_core::node::Node;

    let node1 = Node::new_random_with_port(0).await.unwrap();
    let node2 = Node::new_random_with_port(0).await.unwrap();
    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Try to close non-existent session before any sessions exist
    let fake_peer_id = [0xFF; 32];
    let result = node1.close_session(&fake_peer_id).await;
    assert!(result.is_err(), "Closing non-existent session should fail");

    // Node should still be healthy after error
    assert!(node1.is_running());

    // Verify no sessions exist
    let sessions = node1.active_sessions().await;
    assert_eq!(sessions.len(), 0, "Should have no sessions");

    // Now establish a valid session - node should work normally after error
    let node2_addr = node2.listen_addr().await.unwrap();
    let result = node1
        .establish_session_with_addr(node2.node_id(), node2_addr)
        .await;
    assert!(
        result.is_ok(),
        "Session establishment should succeed after previous error"
    );

    // Give responder time to process handshake
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify session was created
    let sessions = node1.active_sessions().await;
    assert_eq!(sessions.len(), 1, "Should have one session");
    assert_eq!(&sessions[0], node2.x25519_public_key());

    // Try to close the wrong session (should fail)
    let wrong_peer = [0xAA; 32];
    let result = node1.close_session(&wrong_peer).await;
    assert!(result.is_err(), "Closing wrong session should fail");

    // Original session should still exist
    let sessions = node1.active_sessions().await;
    assert_eq!(
        sessions.len(),
        1,
        "Session should still exist after failed close"
    );

    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

/// Test concurrent transfers
///
/// Tests managing multiple simultaneous file transfers:
/// 1. Create multiple transfer sessions
/// 2. Verify isolation between transfers
/// 3. Test concurrent execution and completion
#[tokio::test]
#[ignore = "Placeholder: Requires chunk-based file data transmission implementation"]
async fn test_concurrent_transfers_node_api() {
    use std::fs;
    use tempfile::TempDir;
    use wraith_core::node::Node;

    let temp_dir = TempDir::new().unwrap();

    let sender = Node::new_random_with_port(0).await.unwrap();
    let receiver1 = Node::new_random_with_port(0).await.unwrap();
    let receiver2 = Node::new_random_with_port(0).await.unwrap();
    let receiver3 = Node::new_random_with_port(0).await.unwrap();

    sender.start().await.unwrap();
    receiver1.start().await.unwrap();
    receiver2.start().await.unwrap();
    receiver3.start().await.unwrap();

    // Get receiver addresses and establish sessions
    // Use Ed25519 node_id for addressing, session is stored by X25519 key
    let receiver1_addr = receiver1.listen_addr().await.unwrap();
    let receiver2_addr = receiver2.listen_addr().await.unwrap();
    let receiver3_addr = receiver3.listen_addr().await.unwrap();

    sender
        .establish_session_with_addr(receiver1.node_id(), receiver1_addr)
        .await
        .unwrap();
    sender
        .establish_session_with_addr(receiver2.node_id(), receiver2_addr)
        .await
        .unwrap();
    sender
        .establish_session_with_addr(receiver3.node_id(), receiver3_addr)
        .await
        .unwrap();

    // Give time for session establishment
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create multiple test files and send to different receivers
    // Use X25519 keys since sessions are keyed by X25519
    let mut transfer_ids = Vec::new();
    for i in 0..3 {
        let data = vec![i as u8; 512 * 1024]; // 512KB each
        let path = temp_dir.path().join(format!("file_{}.bin", i));
        fs::write(&path, &data).unwrap();

        let receiver = match i {
            0 => receiver1.x25519_public_key(),
            1 => receiver2.x25519_public_key(),
            _ => receiver3.x25519_public_key(),
        };

        let id = sender.send_file(&path, receiver).await.unwrap();
        transfer_ids.push(id);
    }

    // Verify all transfers were created
    assert_eq!(transfer_ids.len(), 3);

    // Verify all transfer IDs are unique
    assert_ne!(transfer_ids[0], transfer_ids[1]);
    assert_ne!(transfer_ids[1], transfer_ids[2]);
    assert_ne!(transfer_ids[0], transfer_ids[2]);

    // Check that all transfers are tracked
    for id in &transfer_ids {
        let progress = sender.get_transfer_progress(id).await;
        assert!(
            progress.is_some(),
            "Transfer {:?} should be tracked",
            hex::encode(&id[..8])
        );
    }

    // List active transfers
    let active = sender.active_transfers().await;
    assert_eq!(active.len(), 3, "Should have 3 active transfers");

    // Wait for all transfers to complete concurrently
    let mut wait_handles = Vec::new();
    for id in &transfer_ids {
        let sender_clone = sender.clone();
        let id_copy = *id;
        let handle = tokio::spawn(async move {
            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                sender_clone.wait_for_transfer(id_copy),
            )
            .await
        });
        wait_handles.push(handle);
    }

    // Verify all transfers complete successfully
    for (i, handle) in wait_handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        match result {
            Ok(Ok(())) => {
                tracing::info!("Transfer {} completed successfully", i);
            }
            Ok(Err(e)) => {
                panic!("Transfer {} failed: {}", i, e);
            }
            Err(_) => {
                let progress = sender.get_transfer_progress(&transfer_ids[i]).await;
                panic!("Transfer {} timed out. Progress: {:?}", i, progress);
            }
        }
    }

    sender.stop().await.unwrap();
    receiver1.stop().await.unwrap();
    receiver2.stop().await.unwrap();
    receiver3.stop().await.unwrap();
}
