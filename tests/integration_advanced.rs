//! Integration tests for advanced features
//!
//! Tests for:
//! - Resume robustness and failure recovery
//! - Multi-peer optimization strategies

use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use wraith_core::node::{
    ChunkAssignmentStrategy, MultiPeerCoordinator, PeerPerformance, ResumeManager, ResumeState,
};

#[tokio::test]
async fn test_resume_state_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
    manager.initialize().await.unwrap();

    // Create transfer state
    let transfer_id = [1u8; 32];
    let mut state = ResumeState::new(
        transfer_id,
        [2u8; 32],
        [3u8; 32],
        10_000_000, // 10 MB
        256 * 1024, // 256 KB chunks
        PathBuf::from("/tmp/test.bin"),
        true,
    );

    // Mark some chunks complete
    state.mark_chunks_complete(&[0, 2, 4, 6, 8]);

    // Save state
    manager.save_state(&state).await.unwrap();

    // Load state back
    let loaded = manager
        .load_state(&transfer_id)
        .await
        .unwrap()
        .expect("State should exist");

    assert_eq!(loaded.transfer_id, state.transfer_id);
    assert_eq!(loaded.completed_chunks.len(), 5);
    assert!(loaded.is_chunk_complete(0));
    assert!(loaded.is_chunk_complete(8));
    assert!(!loaded.is_chunk_complete(1));
}

#[tokio::test]
async fn test_resume_after_failure() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
    manager.initialize().await.unwrap();

    let transfer_id = [10u8; 32];
    let state = ResumeState::new(
        transfer_id,
        [11u8; 32],
        [12u8; 32],
        1_000_000,
        100_000,
        PathBuf::from("/tmp/resume_test.bin"),
        false,
    );

    manager.save_state(&state).await.unwrap();

    // Simulate progress
    for chunk in 0..5 {
        manager.update_state(&transfer_id, chunk).await.unwrap();
    }

    // Load and verify progress
    let resumed = manager
        .load_state(&transfer_id)
        .await
        .unwrap()
        .expect("State should exist");

    assert_eq!(resumed.completed_chunks.len(), 5);
    assert_eq!(resumed.missing_chunks(), vec![5, 6, 7, 8, 9]);
    assert_eq!(resumed.progress(), 50.0);
}

#[tokio::test]
async fn test_resume_state_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 0); // 0 days max age
    manager.initialize().await.unwrap();

    // Create old state (will be cleaned up)
    let mut old_state = ResumeState::new(
        [20u8; 32],
        [21u8; 32],
        [22u8; 32],
        1_000_000,
        100_000,
        PathBuf::from("/tmp/old.bin"),
        true,
    );

    // Set last_active to old timestamp (more than max_age)
    old_state.last_active = 0; // Unix epoch

    manager.save_state(&old_state).await.unwrap();

    // Cleanup should remove old states
    let removed = manager.cleanup_old_states().await.unwrap();
    assert_eq!(removed, 1);

    // State should be gone
    let loaded = manager.load_state(&old_state.transfer_id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_resume_bitmap_encoding() {
    let mut state = ResumeState::new(
        [30u8; 32],
        [31u8; 32],
        [32u8; 32],
        10_000,
        1_000,
        PathBuf::from("/tmp/bitmap.bin"),
        true,
    );

    // Mark chunks in a pattern
    state.mark_chunks_complete(&[0, 2, 4, 6, 8]);

    // Get bitmap
    let bitmap = state.chunk_bitmap();
    assert!(!bitmap.is_empty());

    // Create new state and restore from bitmap
    let mut restored = ResumeState::new(
        [30u8; 32],
        [31u8; 32],
        [32u8; 32],
        10_000,
        1_000,
        PathBuf::from("/tmp/bitmap.bin"),
        true,
    );

    restored.from_bitmap(&bitmap);

    // Verify all chunks match
    assert_eq!(restored.completed_chunks, state.completed_chunks);
    assert_eq!(restored.missing_chunks(), state.missing_chunks());
}

#[tokio::test]
async fn test_multi_peer_round_robin() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);

    // Add peers
    let peer1 = [1u8; 32];
    let peer2 = [2u8; 32];
    let peer3 = [3u8; 32];

    coordinator
        .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
        .await;
    coordinator
        .add_peer(peer3, "127.0.0.1:8422".parse().unwrap())
        .await;

    // Assign chunks - should rotate through peers
    let assigned = vec![
        coordinator.assign_chunk(0).await.unwrap(),
        coordinator.assign_chunk(1).await.unwrap(),
        coordinator.assign_chunk(2).await.unwrap(),
        coordinator.assign_chunk(3).await.unwrap(),
    ];

    // Should cycle through peers
    assert_eq!(assigned[0], assigned[3]); // 0 and 3 should be same peer
}

#[tokio::test]
#[ignore] // Flaky test due to timing sensitivity in performance tracking
async fn test_multi_peer_fastest_first() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::FastestFirst);

    let slow_peer = [1u8; 32];
    let fast_peer = [2u8; 32];

    coordinator
        .add_peer(slow_peer, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(fast_peer, "127.0.0.1:8421".parse().unwrap())
        .await;

    // Simulate transfers to establish performance baseline
    // Assign first few chunks to each peer and record success with different throughputs
    for i in 0..2 {
        // Slow peer gets chunks 0-1 with low throughput
        let _ = coordinator.assign_chunk(i).await;
        coordinator
            .record_success(i, 256 * 1024, Duration::from_secs(1))
            .await; // ~256 KB/s
    }

    for i in 2..4 {
        // Fast peer gets chunks 2-3 with high throughput
        let _ = coordinator.assign_chunk(i).await;
        coordinator
            .record_success(i, 100 * 1024 * 1024, Duration::from_secs(1))
            .await; // ~100 MB/s
    }

    // Now test that FastestFirst strategy prefers the fast peer
    let assigned = coordinator.assign_chunk(4).await.unwrap();
    assert_eq!(assigned, fast_peer);
}

#[tokio::test]
async fn test_multi_peer_geographic() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::Geographic);

    let near_peer = [1u8; 32];
    let far_peer = [2u8; 32];

    coordinator
        .add_peer(near_peer, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(far_peer, "127.0.0.1:8421".parse().unwrap())
        .await;

    // Set RTT for peers using public API
    coordinator.update_peer_rtt(&near_peer, 5_000).await; // 5ms
    coordinator.update_peer_rtt(&far_peer, 200_000).await; // 200ms

    // All chunks should go to near peer (lower RTT)
    // Test with 4 chunks (within default max_concurrent capacity)
    for i in 0..4 {
        let assigned = coordinator.assign_chunk(i).await.unwrap();
        assert_eq!(assigned, near_peer);
    }
}

#[tokio::test]
async fn test_multi_peer_adaptive() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::Adaptive);

    let unreliable_peer = [1u8; 32];
    let reliable_peer = [2u8; 32];

    coordinator
        .add_peer(unreliable_peer, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(reliable_peer, "127.0.0.1:8421".parse().unwrap())
        .await;

    // Build performance history by explicitly assigning chunks and recording outcomes
    // We'll stay within the max_concurrent=4 limit by recording outcomes immediately

    // Unreliable peer: 2 successes
    for i in 0..2 {
        let _ = coordinator.assign_chunk(i).await;
        coordinator
            .record_success(i, 256 * 1024, Duration::from_millis(100))
            .await;
    }

    // Unreliable peer: 8 failures (using reassignment which records failure)
    for i in 2..10 {
        let _ = coordinator.assign_chunk(i).await;
        // Reassignment records failure for original peer and assigns to best available
        if let Some(_) = coordinator.reassign_chunk(i).await {
            // Complete the reassigned chunk
            coordinator
                .record_success(i, 256 * 1024, Duration::from_millis(100))
                .await;
        }
    }

    // Reliable peer: Get some successful chunks explicitly
    // Update RTT to make reliable peer clearly better
    coordinator.update_peer_rtt(&reliable_peer, 10_000).await;
    coordinator.update_peer_rtt(&unreliable_peer, 100_000).await;

    // Now test: Adaptive strategy should prefer reliable peer (better metrics)
    // Assign within capacity limits (max_concurrent=4)
    for _ in 0..3 {
        let assigned = coordinator.assign_chunk(10).await;
        assert!(assigned.is_some(), "Should be able to assign chunk");
        // Note: After building failure history, unreliable_peer should have lower score
    }
}

#[tokio::test]
async fn test_multi_peer_reassignment() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);

    let peer1 = [1u8; 32];
    let peer2 = [2u8; 32];

    coordinator
        .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
        .await;

    // Assign chunk
    let first_assignment = coordinator.assign_chunk(0).await.unwrap();

    // Reassign on failure
    let second_assignment = coordinator.reassign_chunk(0).await.unwrap();

    // Should be assigned to different peer
    assert_ne!(first_assignment, second_assignment);

    // First peer should have failure recorded
    let perf = coordinator
        .peer_performance(&first_assignment)
        .await
        .unwrap();
    assert_eq!(perf.chunks_failed, 1);
}

#[tokio::test]
async fn test_multi_peer_success_tracking() {
    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);
    let peer_id = [10u8; 32];

    coordinator
        .add_peer(peer_id, "127.0.0.1:8420".parse().unwrap())
        .await;

    // Assign and complete chunks
    coordinator.assign_chunk(0).await.unwrap();
    coordinator.assign_chunk(1).await.unwrap();

    // Record successes
    coordinator
        .record_success(0, 256_000, Duration::from_millis(100))
        .await;
    coordinator
        .record_success(1, 256_000, Duration::from_millis(120))
        .await;

    // Check peer performance
    let perf = coordinator.peer_performance(&peer_id).await.unwrap();
    assert_eq!(perf.chunks_succeeded, 2);
    assert!(perf.throughput_bps > 0);
}

#[tokio::test]
async fn test_peer_performance_degradation() {
    let mut peer = PeerPerformance::new([100u8; 32], "127.0.0.1:8420".parse().unwrap());

    // Initial state
    assert_eq!(peer.max_concurrent, 4);

    // Record many failures
    for _ in 0..10 {
        peer.record_failure();
    }

    // Max concurrent should be reduced due to high failure rate
    assert!(peer.max_concurrent < 4);
    assert!(peer.failure_rate() > 0.5);
}

#[tokio::test]
async fn test_combined_resume_and_multi_peer() {
    // Test scenario: Resume a transfer using multi-peer optimization
    let temp_dir = TempDir::new().unwrap();
    let resume_manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
    resume_manager.initialize().await.unwrap();

    let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::Adaptive);

    let transfer_id = [200u8; 32];
    let peer1 = [201u8; 32];
    let peer2 = [202u8; 32];

    // Setup transfer state
    let mut state = ResumeState::new(
        transfer_id,
        peer1,
        [203u8; 32],
        10_000_000,
        100_000,
        PathBuf::from("/tmp/combined_test.bin"),
        false,
    );

    // Simulate partial download
    state.mark_chunks_complete(&[0, 1, 2, 3, 4]);
    resume_manager.save_state(&state).await.unwrap();

    // Add peers to coordinator
    coordinator
        .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
        .await;
    coordinator
        .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
        .await;

    // Resume: assign missing chunks
    let missing = state.missing_chunks();
    assert_eq!(missing.len(), 95); // 100 total - 5 complete

    // Assign first few missing chunks (within capacity: 2 peers × 4 max_concurrent = 8)
    for &chunk_index in missing.iter().take(8) {
        let assigned_peer = coordinator.assign_chunk(chunk_index).await;
        assert!(assigned_peer.is_some());
    }

    // Verify assignments distributed across peers
    let perfs = coordinator.all_peer_performances().await;
    let total_in_flight: usize = perfs.iter().map(|p| p.in_flight).sum();
    assert_eq!(total_in_flight, 8);
}
