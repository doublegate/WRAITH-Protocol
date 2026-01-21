// WRAITH-Chat Integration Tests
//
// Comprehensive integration tests for the WRAITH-Chat desktop application.
// Tests voice calling, video calling, and group messaging integration.

#![cfg(test)]

use std::time::Duration;

use crate::group::{
    GroupEncryptedMessage, GroupError, GroupInfo, GroupMember, GroupRole, GroupSession,
    GroupSessionManager, KEY_ROTATION_DAYS, MAX_GROUP_MEMBERS, SenderKeyDistribution,
    SenderKeyState,
};
use crate::video_call::{
    VideoCallError, VideoCallInfo, VideoCallManager, VideoCallSignal, VideoCallStats,
    VideoCodecConfig, VideoPacket, VideoSource,
};
use crate::voice_call::{
    CallDirection, CallInfo, CallSignal, CallState, CallStats, CodecConfig, VoiceCallError,
    VoiceCallManager, VoicePacket,
};

// ============================================================================
// End-to-End Testing (Task 17.8.1)
// ============================================================================

/// Test that voice, video, and group modules can coexist
#[tokio::test]
async fn test_full_module_initialization() {
    // Initialize all three managers
    let voice_manager = VoiceCallManager::new();
    let video_manager = VideoCallManager::new();
    let group_manager = GroupSessionManager::new();

    // Verify all managers start clean
    assert!(voice_manager.get_active_calls().await.is_empty());
    assert!(video_manager.get_active_calls().await.is_empty());
    assert!(group_manager.list_groups().is_empty());
}

/// Test voice call lifecycle
#[tokio::test]
async fn test_voice_call_lifecycle() {
    let manager = VoiceCallManager::new();

    // Start a call
    let peer_id = "test_peer_12345";
    let call_result = manager.start_call(peer_id).await;

    // Should succeed
    assert!(call_result.is_ok());
    let call_info = call_result.unwrap();

    // Verify initial state
    assert_eq!(call_info.peer_id, peer_id);
    assert_eq!(call_info.direction, CallDirection::Outgoing);
    assert_eq!(call_info.state, CallState::Ringing);
    assert!(!call_info.muted);
    assert!(!call_info.speaker_on);

    // Get call info
    let retrieved = manager.get_call_info(&call_info.call_id).await;
    assert!(retrieved.is_ok());
    assert!(retrieved.unwrap().is_some());

    // End the call
    let end_result = manager.end_call(&call_info.call_id, "test complete").await;
    assert!(end_result.is_ok());
}

/// Test incoming call handling
#[tokio::test]
async fn test_incoming_call_handling() {
    let manager = VoiceCallManager::new();

    let peer_id = "remote_peer";
    let call_id = "incoming_call_123";
    let codec_config = CodecConfig::default();

    // Handle incoming call
    let result = manager
        .handle_incoming_call(peer_id, call_id, codec_config)
        .await;
    assert!(result.is_ok());

    let call_info = result.unwrap();
    assert_eq!(call_info.state, CallState::Incoming);
    assert_eq!(call_info.direction, CallDirection::Incoming);

    // Reject the call
    let reject_result = manager.reject_call(call_id, "busy").await;
    assert!(reject_result.is_ok());
}

/// Test video call lifecycle
#[tokio::test]
async fn test_video_call_lifecycle() {
    let manager = VideoCallManager::new();

    // Start a video call
    let peer_id = "video_peer";
    let call_result = manager.start_video_call(peer_id, false).await;

    // Should succeed
    assert!(call_result.is_ok());
    let call_info = call_result.unwrap();

    // Verify state
    assert_eq!(call_info.peer_id, peer_id);
    assert_eq!(call_info.state, CallState::Ringing);
    assert!(!call_info.video_enabled); // Started without video

    // End the call
    let end_result = manager
        .end_video_call(&call_info.call_id, "test complete")
        .await;
    assert!(end_result.is_ok());
}

/// Test group session lifecycle
#[test]
fn test_group_session_lifecycle() {
    let mut manager = GroupSessionManager::new();

    // Create a group
    let group_info = manager.create_group(
        "Test Group".to_string(),
        "our_peer_id".to_string(),
        Some("Alice".to_string()),
    );

    assert_eq!(group_info.name, "Test Group");
    assert!(group_info.am_i_admin);
    assert_eq!(group_info.member_count, 1);

    // Get the session
    let session = manager.get_session(&group_info.group_id);
    assert!(session.is_some());

    // List groups
    let groups = manager.list_groups();
    assert_eq!(groups.len(), 1);

    // Remove the group
    let removed = manager.remove_session(&group_info.group_id);
    assert!(removed.is_some());
    assert!(manager.list_groups().is_empty());
}

// ============================================================================
// Cross-Platform Interop Testing (Task 17.8.2)
// ============================================================================

/// Test voice packet serialization/deserialization
#[test]
fn test_voice_packet_interop() {
    let packet = VoicePacket {
        call_id: "call_123".to_string(),
        sequence: 42,
        timestamp: 960 * 42,
        audio_data: vec![0u8; 100],
        is_silence: false,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&packet).expect("Should serialize");

    // Deserialize
    let decoded: VoicePacket = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(decoded.call_id, packet.call_id);
    assert_eq!(decoded.sequence, packet.sequence);
    assert_eq!(decoded.timestamp, packet.timestamp);
    assert_eq!(decoded.audio_data.len(), packet.audio_data.len());
    assert_eq!(decoded.is_silence, packet.is_silence);
}

/// Test video packet serialization/deserialization
#[test]
fn test_video_packet_interop() {
    use crate::video::VideoCodec;

    let packet = VideoPacket {
        call_id: "video_call_456".to_string(),
        sequence: 100,
        timestamp_us: 33333,
        video_data: vec![0u8; 1000],
        width: 1280,
        height: 720,
        is_keyframe: true,
        codec: VideoCodec::Vp9,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&packet).expect("Should serialize");

    // Deserialize
    let decoded: VideoPacket = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(decoded.call_id, packet.call_id);
    assert_eq!(decoded.sequence, packet.sequence);
    assert_eq!(decoded.width, packet.width);
    assert_eq!(decoded.height, packet.height);
    assert_eq!(decoded.is_keyframe, packet.is_keyframe);
}

/// Test call signal serialization
#[test]
fn test_call_signal_interop() {
    let signals = vec![
        CallSignal::Offer {
            call_id: "call_1".to_string(),
            codec_config: CodecConfig::default(),
        },
        CallSignal::Answer {
            call_id: "call_1".to_string(),
        },
        CallSignal::Reject {
            call_id: "call_1".to_string(),
            reason: "busy".to_string(),
        },
        CallSignal::Hangup {
            call_id: "call_1".to_string(),
            reason: "user ended".to_string(),
        },
        CallSignal::Ringing {
            call_id: "call_1".to_string(),
        },
        CallSignal::Hold {
            call_id: "call_1".to_string(),
        },
        CallSignal::Resume {
            call_id: "call_1".to_string(),
        },
        CallSignal::Ping {
            call_id: "call_1".to_string(),
            timestamp: 1700000000,
        },
        CallSignal::Pong {
            call_id: "call_1".to_string(),
            timestamp: 1700000000,
        },
    ];

    for signal in signals {
        let json = serde_json::to_string(&signal).expect("Should serialize");
        let _decoded: CallSignal = serde_json::from_str(&json).expect("Should deserialize");
    }
}

/// Test video call signal serialization
#[test]
fn test_video_call_signal_interop() {
    let signals = vec![
        VideoCallSignal::VideoOffer {
            call_id: "call_1".to_string(),
            video_config: VideoCodecConfig::default(),
            source: VideoSource::Camera,
        },
        VideoCallSignal::VideoAccept {
            call_id: "call_1".to_string(),
            video_config: VideoCodecConfig::default(),
        },
        VideoCallSignal::VideoReject {
            call_id: "call_1".to_string(),
            reason: "no video".to_string(),
        },
        VideoCallSignal::VideoEnable {
            call_id: "call_1".to_string(),
            source: VideoSource::Screen,
        },
        VideoCallSignal::VideoDisable {
            call_id: "call_1".to_string(),
        },
        VideoCallSignal::KeyframeRequest {
            call_id: "call_1".to_string(),
        },
        VideoCallSignal::BandwidthUpdate {
            call_id: "call_1".to_string(),
            estimated_bps: 1_500_000,
        },
    ];

    for signal in signals {
        let json = serde_json::to_string(&signal).expect("Should serialize");
        let _decoded: VideoCallSignal = serde_json::from_str(&json).expect("Should deserialize");
    }
}

/// Test group encrypted message serialization
#[test]
fn test_group_message_interop() {
    let message = GroupEncryptedMessage {
        group_id: "group_123".to_string(),
        sender_peer_id: "sender_456".to_string(),
        key_generation: 1,
        iteration: 42,
        nonce: vec![0u8; 12],
        ciphertext: vec![0u8; 100],
    };

    // Serialize to JSON
    let json = serde_json::to_string(&message).expect("Should serialize");

    // Deserialize
    let decoded: GroupEncryptedMessage = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(decoded.group_id, message.group_id);
    assert_eq!(decoded.sender_peer_id, message.sender_peer_id);
    assert_eq!(decoded.key_generation, message.key_generation);
    assert_eq!(decoded.iteration, message.iteration);
}

/// Test sender key distribution interop
#[test]
fn test_sender_key_distribution_interop() {
    let key_state = SenderKeyState::new();
    let distribution = key_state.to_distribution();

    // Serialize
    let json = serde_json::to_string(&distribution).expect("Should serialize");

    // Deserialize
    let decoded: SenderKeyDistribution = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(decoded.generation, distribution.generation);
    assert_eq!(decoded.iteration, distribution.iteration);
    assert_eq!(decoded.chain_key.len(), distribution.chain_key.len());
}

// ============================================================================
// Performance Benchmark Testing (Task 17.8.3)
// ============================================================================

/// Benchmark group encryption
#[test]
fn test_group_encryption_performance() {
    let mut session = GroupSession::new(
        "perf_test_group".to_string(),
        "Performance Test".to_string(),
        "our_peer".to_string(),
        None,
    );

    let message = b"This is a test message for performance benchmarking.";
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _encrypted = session.encrypt(message).unwrap();
    }
    let duration = start.elapsed();

    // Should be able to encrypt at least 1000 messages per second
    assert!(
        duration.as_millis() < 1000,
        "Group encryption too slow: {:?} for {} iterations",
        duration,
        iterations
    );

    println!(
        "Group encryption: {} ops in {:?} ({:.0} ops/sec)",
        iterations,
        duration,
        iterations as f64 / duration.as_secs_f64()
    );
}

/// Benchmark group decryption
#[test]
fn test_group_decryption_performance() {
    // Setup: Alice creates group, Bob joins
    let mut alice_session = GroupSession::new(
        "perf_group".to_string(),
        "Perf Test".to_string(),
        "alice".to_string(),
        None,
    );

    let alice_dist = alice_session.get_my_distribution();
    let mut bob_session = GroupSession::join(
        "perf_group".to_string(),
        "Perf Test".to_string(),
        "bob".to_string(),
        "alice".to_string(),
        alice_dist,
    );

    let bob_dist = bob_session.get_my_distribution();
    alice_session
        .add_member_key("bob", bob_dist, None, GroupRole::Member)
        .unwrap();

    // Pre-generate encrypted messages
    let message = b"Test message for decryption benchmark";
    let encrypted_messages: Vec<_> = (0..100)
        .map(|_| bob_session.encrypt(message).unwrap())
        .collect();

    let iterations = encrypted_messages.len();
    let start = std::time::Instant::now();

    for encrypted in encrypted_messages {
        let _decrypted = alice_session.decrypt(&encrypted).unwrap();
    }

    let duration = start.elapsed();

    println!(
        "Group decryption: {} ops in {:?} ({:.0} ops/sec)",
        iterations,
        duration,
        iterations as f64 / duration.as_secs_f64()
    );
}

/// Benchmark sender key derivation
#[test]
fn test_sender_key_derivation_performance() {
    let mut key_state = SenderKeyState::new();
    let iterations = 10000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _msg_key = key_state.derive_message_key().unwrap();
    }
    let duration = start.elapsed();

    // Should be very fast
    assert!(
        duration.as_millis() < 500,
        "Key derivation too slow: {:?}",
        duration
    );

    println!(
        "Key derivation: {} ops in {:?} ({:.0} ops/sec)",
        iterations,
        duration,
        iterations as f64 / duration.as_secs_f64()
    );
}

/// Benchmark packet serialization
#[test]
fn test_packet_serialization_performance() {
    let voice_packet = VoicePacket {
        call_id: "call_123".to_string(),
        sequence: 42,
        timestamp: 960 * 42,
        audio_data: vec![0u8; 100],
        is_silence: false,
    };

    let iterations = 10000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let json = serde_json::to_string(&voice_packet).unwrap();
        let _decoded: VoicePacket = serde_json::from_str(&json).unwrap();
    }
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 500,
        "Packet serialization too slow: {:?}",
        duration
    );

    println!(
        "Packet serialization: {} round-trips in {:?} ({:.0} ops/sec)",
        iterations,
        duration,
        (iterations * 2) as f64 / duration.as_secs_f64()
    );
}

/// Benchmark call manager operations
#[tokio::test]
async fn test_call_manager_performance() {
    let manager = VoiceCallManager::new();
    let iterations = 10; // Reduced for faster test execution

    let start = std::time::Instant::now();

    for i in 0..iterations {
        let peer_id = format!("peer_{}", i);
        let call_result = manager.start_call(&peer_id).await;

        if let Ok(call_info) = call_result {
            // Toggle mute
            let _ = manager.toggle_mute(&call_info.call_id).await;
            // Toggle speaker
            let _ = manager.toggle_speaker(&call_info.call_id).await;
            // End call
            let _ = manager.end_call(&call_info.call_id, "benchmark").await;
        }
    }

    let duration = start.elapsed();

    println!(
        "Call manager ops: {} cycles in {:?} ({:.1} cycles/sec)",
        iterations,
        duration,
        iterations as f64 / duration.as_secs_f64()
    );
}

// ============================================================================
// Security Validation Testing (Task 17.8.4)
// ============================================================================

/// Test sender key forward secrecy
#[test]
fn test_sender_key_forward_secrecy() {
    let mut key_state = SenderKeyState::new();

    // Generate some message keys
    let msg_key1 = key_state.derive_message_key().unwrap();
    let msg_key2 = key_state.derive_message_key().unwrap();
    let msg_key3 = key_state.derive_message_key().unwrap();

    // All keys should be different (chain advances)
    assert_ne!(msg_key1, msg_key2);
    assert_ne!(msg_key2, msg_key3);
    assert_ne!(msg_key1, msg_key3);

    // After rotation, new keys should be completely different
    let old_generation = key_state.generation;
    key_state.rotate();

    assert_eq!(key_state.generation, old_generation + 1);
    assert_eq!(key_state.iteration, 0); // Reset

    let new_msg_key = key_state.derive_message_key().unwrap();
    assert_ne!(new_msg_key, msg_key1);
    assert_ne!(new_msg_key, msg_key2);
    assert_ne!(new_msg_key, msg_key3);
}

/// Test group member removal triggers key rotation
#[test]
fn test_member_removal_key_rotation() {
    let mut session = GroupSession::new(
        "secure_group".to_string(),
        "Secure Group".to_string(),
        "admin".to_string(),
        None,
    );

    // Add a member
    let member_key = SenderKeyState::new();
    session
        .add_member_key(
            "member1",
            member_key.to_distribution(),
            None,
            GroupRole::Member,
        )
        .unwrap();

    // Get distribution before removal
    let old_dist = session.get_my_distribution();
    let old_generation = old_dist.generation;

    // Remove member
    session.remove_member("member1").unwrap();

    // Key should be rotated
    let new_dist = session.get_my_distribution();
    assert_eq!(new_dist.generation, old_generation + 1);
    assert_ne!(new_dist.chain_key, old_dist.chain_key);
}

/// Test stale key generation rejection
#[test]
fn test_stale_key_rejection() {
    let mut alice_session = GroupSession::new(
        "stale_test".to_string(),
        "Stale Test".to_string(),
        "alice".to_string(),
        None,
    );

    let alice_dist = alice_session.get_my_distribution();
    let mut bob_session = GroupSession::join(
        "stale_test".to_string(),
        "Stale Test".to_string(),
        "bob".to_string(),
        "alice".to_string(),
        alice_dist,
    );

    let bob_dist = bob_session.get_my_distribution();
    alice_session
        .add_member_key("bob", bob_dist, None, GroupRole::Member)
        .unwrap();

    // Bob encrypts a message
    let encrypted = bob_session.encrypt(b"Hello").unwrap();

    // Alice can decrypt
    let decrypted = alice_session.decrypt(&encrypted);
    assert!(decrypted.is_ok());

    // Now Bob rotates his key but doesn't share it
    bob_session.rotate_sender_key();

    // Bob encrypts with new key
    let new_encrypted = bob_session.encrypt(b"Hello again").unwrap();

    // Alice can't decrypt (key generation mismatch)
    let result = alice_session.decrypt(&new_encrypted);
    assert!(result.is_err());

    match result {
        Err(GroupError::InvalidSenderKey) => {} // Expected
        other => panic!("Expected InvalidSenderKey, got: {:?}", other),
    }
}

/// Test group authorization
#[test]
fn test_group_authorization() {
    let mut session = GroupSession::new(
        "auth_test".to_string(),
        "Auth Test".to_string(),
        "admin".to_string(),
        None,
    );

    // Add a regular member
    let member_key = SenderKeyState::new();
    session
        .add_member_key(
            "member",
            member_key.to_distribution(),
            None,
            GroupRole::Member,
        )
        .unwrap();

    // Admin check
    assert!(session.is_admin("admin"));
    assert!(!session.is_admin("member"));
    assert!(session.am_i_admin());

    // Promote member to admin
    session.promote_to_admin("member").unwrap();
    assert!(session.is_admin("member"));

    // Demote back
    session.demote_from_admin("member").unwrap();
    assert!(!session.is_admin("member"));

    // Can't demote last admin
    let result = session.demote_from_admin("admin");
    assert!(result.is_err());
}

/// Test encryption with empty and large messages
#[test]
fn test_encryption_edge_cases() {
    let mut session = GroupSession::new(
        "edge_test".to_string(),
        "Edge Test".to_string(),
        "peer".to_string(),
        None,
    );

    // Empty message
    let encrypted_empty = session.encrypt(b"");
    assert!(encrypted_empty.is_ok());

    // Large message (1MB)
    let large_message = vec![0xab; 1024 * 1024];
    let encrypted_large = session.encrypt(&large_message);
    assert!(encrypted_large.is_ok());

    // Verify ciphertext is larger than plaintext (due to auth tag)
    let ciphertext = encrypted_large.unwrap().ciphertext;
    assert!(ciphertext.len() > large_message.len());
}

/// Test invalid input handling
#[test]
fn test_invalid_input_handling() {
    let mut session = GroupSession::new(
        "invalid_test".to_string(),
        "Invalid Test".to_string(),
        "peer".to_string(),
        None,
    );

    // Try to remove non-existent member
    let result = session.remove_member("nonexistent");
    assert!(matches!(result, Err(GroupError::MemberNotFound(_))));

    // Try to decrypt from unknown sender
    let fake_message = GroupEncryptedMessage {
        group_id: "invalid_test".to_string(),
        sender_peer_id: "unknown_sender".to_string(),
        key_generation: 0,
        iteration: 0,
        nonce: vec![0u8; 12],
        ciphertext: vec![0u8; 32],
    };

    let result = session.decrypt(&fake_message);
    assert!(matches!(result, Err(GroupError::MemberNotFound(_))));
}

/// Test nonce uniqueness
#[test]
fn test_nonce_uniqueness() {
    let mut session = GroupSession::new(
        "nonce_test".to_string(),
        "Nonce Test".to_string(),
        "peer".to_string(),
        None,
    );

    // Encrypt many messages and collect nonces
    let mut nonces: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    let message = b"Test message";

    for _ in 0..1000 {
        let encrypted = session.encrypt(message).unwrap();
        // Nonces should all be unique
        assert!(nonces.insert(encrypted.nonce), "Nonce collision detected!");
    }
}

// ============================================================================
// Voice/Video/Group Integration Tests
// ============================================================================

/// Test voice and video manager integration
#[tokio::test]
async fn test_voice_video_integration() {
    let video_manager = VideoCallManager::new();

    // Access the underlying voice manager
    let voice_manager = video_manager.voice_manager();

    // Both managers should share state properly
    let voice_calls = voice_manager.get_active_calls().await;
    let video_calls = video_manager.get_active_calls().await;

    assert!(voice_calls.is_empty());
    assert!(video_calls.is_empty());

    // Start a video call (which internally starts voice)
    let call_result = video_manager.start_video_call("peer_123", true).await;
    assert!(call_result.is_ok());
}

/// Test concurrent group operations
#[test]
fn test_concurrent_group_operations() {
    use std::sync::Arc;
    use std::thread;

    let manager = Arc::new(std::sync::Mutex::new(GroupSessionManager::new()));
    let mut handles = vec![];

    // Create groups concurrently
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mut mgr = manager_clone.lock().unwrap();
            mgr.create_group(
                format!("Group {}", i),
                format!("peer_{}", i),
                Some(format!("User {}", i)),
            )
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all groups created
    let mgr = manager.lock().unwrap();
    assert_eq!(mgr.list_groups().len(), 10);
}

/// Test call state transitions
#[tokio::test]
async fn test_call_state_transitions() {
    let manager = VoiceCallManager::new();

    // Verify all state transitions
    assert_eq!(CallState::Initiating.to_string(), "initiating");
    assert_eq!(CallState::Ringing.to_string(), "ringing");
    assert_eq!(CallState::Incoming.to_string(), "incoming");
    assert_eq!(CallState::Connected.to_string(), "connected");
    assert_eq!(CallState::OnHold.to_string(), "on_hold");
    assert_eq!(CallState::Reconnecting.to_string(), "reconnecting");
    assert_eq!(CallState::Ended.to_string(), "ended");

    // Test state serialization
    let states = vec![
        CallState::Initiating,
        CallState::Ringing,
        CallState::Incoming,
        CallState::Connected,
        CallState::OnHold,
        CallState::Reconnecting,
        CallState::Ended,
    ];

    for state in states {
        let json = serde_json::to_string(&state).unwrap();
        let decoded: CallState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, state);
    }
}

/// Test video source types
#[test]
fn test_video_source_types() {
    assert_eq!(VideoSource::default(), VideoSource::Camera);

    let sources = vec![VideoSource::Camera, VideoSource::Screen, VideoSource::None];

    for source in sources {
        let json = serde_json::to_string(&source).unwrap();
        let decoded: VideoSource = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, source);
    }
}

/// Test group role types
#[test]
fn test_group_role_types() {
    let roles = vec![GroupRole::Admin, GroupRole::Member];

    for role in roles {
        let json = serde_json::to_string(&role).unwrap();
        let decoded: GroupRole = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, role);
    }
}

/// Test codec config defaults
#[test]
fn test_codec_config_defaults() {
    // Voice codec
    let voice_config = CodecConfig::default();
    assert_eq!(voice_config.codec, "opus");
    assert_eq!(voice_config.sample_rate, 48000);
    assert_eq!(voice_config.bitrate, 64000);
    assert_eq!(voice_config.frame_size, 960);

    // Video codec
    let video_config = VideoCodecConfig::default();
    assert_eq!(video_config.codec, crate::video::VideoCodec::Vp9);
    assert_eq!(video_config.framerate, 30);
    assert!(video_config.adaptive);
}

/// Test stats structures
#[test]
fn test_stats_structures() {
    // Call stats
    let call_stats = CallStats::default();
    assert_eq!(call_stats.duration_secs, 0);
    assert_eq!(call_stats.packets_sent, 0);
    assert_eq!(call_stats.packets_received, 0);
    assert_eq!(call_stats.packets_lost, 0);

    // Video call stats
    let video_stats = VideoCallStats::default();
    assert_eq!(video_stats.video_frames_sent, 0);
    assert_eq!(video_stats.video_frames_received, 0);
    assert_eq!(video_stats.video_bitrate, 0);
}

/// Test group constants
#[test]
fn test_group_constants() {
    // Max members should be reasonable
    assert!(MAX_GROUP_MEMBERS >= 100);
    assert!(MAX_GROUP_MEMBERS <= 10000);

    // Key rotation interval should be at least 1 day
    assert!(KEY_ROTATION_DAYS >= 1);
    assert!(KEY_ROTATION_DAYS <= 30);
}

/// Test key rotation check
#[test]
fn test_key_rotation_check() {
    let session = GroupSession::new(
        "rotation_test".to_string(),
        "Rotation Test".to_string(),
        "peer".to_string(),
        None,
    );

    // Freshly created session should not need rotation
    assert!(!session.needs_key_rotation());
}

/// Test group serialization
#[test]
fn test_group_session_serialization() {
    let session = GroupSession::new(
        "serialize_test".to_string(),
        "Serialize Test".to_string(),
        "peer".to_string(),
        Some("Test User".to_string()),
    );

    // Serialize to JSON
    let json = session.to_json().unwrap();

    // Deserialize
    let decoded = GroupSession::from_json(&json).unwrap();

    assert_eq!(decoded.group_id, session.group_id);
    assert_eq!(decoded.name, session.name);
}
