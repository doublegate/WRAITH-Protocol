//! Multi-peer download optimization
//!
//! Implements intelligent chunk assignment strategies for downloading from
//! multiple peers simultaneously to maximize throughput.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Chunk assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChunkAssignmentStrategy {
    /// Distribute chunks evenly across all peers (round-robin)
    RoundRobin,

    /// Prioritize fastest peers based on recent performance
    FastestFirst,

    /// Prefer geographically closer peers (based on RTT)
    Geographic,

    /// Adaptive strategy that dynamically adjusts based on performance
    #[default]
    Adaptive,
}

/// Peer performance metrics
#[derive(Debug, Clone)]
pub struct PeerPerformance {
    /// Peer ID
    pub peer_id: [u8; 32],

    /// Peer address
    pub address: SocketAddr,

    /// Round-trip time in microseconds
    pub rtt_us: u64,

    /// Average throughput in bytes per second
    pub throughput_bps: u64,

    /// Number of chunks successfully received
    pub chunks_succeeded: usize,

    /// Number of chunks that failed
    pub chunks_failed: usize,

    /// Last activity timestamp
    pub last_active: Instant,

    /// Current number of in-flight chunks
    pub in_flight: usize,

    /// Maximum concurrent chunks for this peer
    pub max_concurrent: usize,

    /// Cached performance score (to avoid recomputation)
    cached_score: f64,

    /// When the cached score was last updated
    score_updated_at: Instant,
}

impl PeerPerformance {
    /// Create new peer performance tracker
    pub fn new(peer_id: [u8; 32], address: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            address,
            rtt_us: 100_000,           // Initial estimate: 100ms
            throughput_bps: 1_000_000, // Initial estimate: 1 MB/s
            chunks_succeeded: 0,
            chunks_failed: 0,
            last_active: now,
            in_flight: 0,
            max_concurrent: 4,
            cached_score: 0.5, // Initial moderate score
            score_updated_at: now,
        }
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        let total = self.chunks_succeeded + self.chunks_failed;
        if total == 0 {
            0.0
        } else {
            self.chunks_failed as f64 / total as f64
        }
    }

    /// Calculate reliability score (0.0 to 1.0)
    pub fn reliability_score(&self) -> f64 {
        1.0 - self.failure_rate()
    }

    /// Calculate speed score (normalized)
    pub fn speed_score(&self) -> f64 {
        // Higher throughput = higher score
        // Normalize to 0-1 range (assume max 100 MB/s)
        let max_bps = 100 * 1024 * 1024;
        (self.throughput_bps as f64 / max_bps as f64).min(1.0)
    }

    /// Calculate latency score (normalized, lower RTT = higher score)
    pub fn latency_score(&self) -> f64 {
        // Lower RTT = higher score
        // Normalize to 0-1 range (assume max 1000ms)
        let max_rtt = 1_000_000; // 1000ms in microseconds
        1.0 - (self.rtt_us as f64 / max_rtt as f64).min(1.0)
    }

    /// Calculate overall performance score
    ///
    /// Returns cached score that is updated when metrics change
    pub fn performance_score(&self) -> f64 {
        self.cached_score
    }

    /// Update cached performance score
    ///
    /// Called internally whenever metrics change
    fn update_cached_score(&mut self) {
        // Weighted combination of reliability, speed, and latency
        let reliability_weight = 0.4;
        let speed_weight = 0.4;
        let latency_weight = 0.2;

        self.cached_score = reliability_weight * self.reliability_score()
            + speed_weight * self.speed_score()
            + latency_weight * self.latency_score();

        self.score_updated_at = Instant::now();
    }

    /// Check if peer has capacity for more chunks
    pub fn has_capacity(&self) -> bool {
        self.in_flight < self.max_concurrent
    }

    /// Update RTT measurement
    pub fn update_rtt(&mut self, rtt_us: u64) {
        // Exponential moving average
        let alpha = 0.125; // Standard TCP alpha
        self.rtt_us = ((1.0 - alpha) * self.rtt_us as f64 + alpha * rtt_us as f64) as u64;

        // Update cached performance score
        self.update_cached_score();
    }

    /// Update throughput measurement
    pub fn update_throughput(&mut self, bytes: u64, duration: Duration) {
        let bps = (bytes as f64 / duration.as_secs_f64()) as u64;

        // Exponential moving average
        let alpha = 0.25;
        self.throughput_bps =
            ((1.0 - alpha) * self.throughput_bps as f64 + alpha * bps as f64) as u64;

        // Update cached performance score
        self.update_cached_score();
    }

    /// Record successful chunk
    pub fn record_success(&mut self) {
        self.chunks_succeeded += 1;
        self.last_active = Instant::now();
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }

        // Update cached performance score
        self.update_cached_score();
    }

    /// Record failed chunk
    pub fn record_failure(&mut self) {
        self.chunks_failed += 1;
        self.last_active = Instant::now();
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }

        // Reduce max concurrent on failures
        if self.failure_rate() > 0.2 && self.max_concurrent > 1 {
            self.max_concurrent -= 1;
        }

        // Update cached performance score
        self.update_cached_score();
    }

    /// Record chunk assignment
    pub fn record_assignment(&mut self) {
        self.in_flight += 1;
    }
}

/// Multi-peer chunk coordinator
pub struct MultiPeerCoordinator {
    /// Strategy for chunk assignment
    strategy: ChunkAssignmentStrategy,

    /// Peer performance tracking
    peers: Arc<RwLock<HashMap<[u8; 32], PeerPerformance>>>,

    /// Chunk assignments (chunk_index -> peer_id)
    assignments: Arc<RwLock<HashMap<usize, [u8; 32]>>>,

    /// Round-robin counter for RoundRobin strategy
    round_robin_counter: Arc<RwLock<usize>>,
}

impl MultiPeerCoordinator {
    /// Create a new multi-peer coordinator
    pub fn new(strategy: ChunkAssignmentStrategy) -> Self {
        Self {
            strategy,
            peers: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a peer to the coordinator
    pub async fn add_peer(&self, peer_id: [u8; 32], address: SocketAddr) {
        let mut peers = self.peers.write().await;
        peers.insert(peer_id, PeerPerformance::new(peer_id, address));
    }

    /// Remove a peer from the coordinator
    pub async fn remove_peer(&self, peer_id: &[u8; 32]) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);

        // Reassign chunks from removed peer
        let mut assignments = self.assignments.write().await;
        assignments.retain(|_, assigned_peer| assigned_peer != peer_id);
    }

    /// Assign a chunk to a peer using the configured strategy
    pub async fn assign_chunk(&self, chunk_index: usize) -> Option<[u8; 32]> {
        let peers = self.peers.read().await;
        if peers.is_empty() {
            return None;
        }

        let peer_id = match self.strategy {
            ChunkAssignmentStrategy::RoundRobin => self.assign_round_robin(&peers).await,
            ChunkAssignmentStrategy::FastestFirst => self.assign_fastest_first(&peers),
            ChunkAssignmentStrategy::Geographic => self.assign_geographic(&peers),
            ChunkAssignmentStrategy::Adaptive => self.assign_adaptive(&peers),
        }?;

        // Record assignment
        drop(peers);
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&peer_id) {
            peer.record_assignment();
        }

        let mut assignments = self.assignments.write().await;
        assignments.insert(chunk_index, peer_id);

        Some(peer_id)
    }

    /// Round-robin assignment
    async fn assign_round_robin(
        &self,
        peers: &HashMap<[u8; 32], PeerPerformance>,
    ) -> Option<[u8; 32]> {
        // Count available peers without allocating a Vec
        let available_count = peers.values().filter(|p| p.has_capacity()).count();
        if available_count == 0 {
            return None;
        }

        let mut counter = self.round_robin_counter.write().await;
        let index = *counter % available_count;
        *counter = counter.wrapping_add(1);

        // Use nth() to select the peer at the calculated index
        peers
            .iter()
            .filter(|(_, p)| p.has_capacity())
            .nth(index)
            .map(|(id, _)| *id)
    }

    /// Fastest-first assignment (highest throughput)
    fn assign_fastest_first(&self, peers: &HashMap<[u8; 32], PeerPerformance>) -> Option<[u8; 32]> {
        peers
            .iter()
            .filter(|(_, p)| p.has_capacity())
            .max_by_key(|(_, p)| p.throughput_bps)
            .map(|(id, _)| *id)
    }

    /// Geographic assignment (lowest RTT)
    fn assign_geographic(&self, peers: &HashMap<[u8; 32], PeerPerformance>) -> Option<[u8; 32]> {
        peers
            .iter()
            .filter(|(_, p)| p.has_capacity())
            .min_by_key(|(_, p)| p.rtt_us)
            .map(|(id, _)| *id)
    }

    /// Adaptive assignment (highest performance score)
    fn assign_adaptive(&self, peers: &HashMap<[u8; 32], PeerPerformance>) -> Option<[u8; 32]> {
        peers
            .iter()
            .filter(|(_, p)| p.has_capacity())
            .max_by(|(_, a), (_, b)| {
                a.performance_score()
                    .partial_cmp(&b.performance_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    /// Reassign a chunk on failure
    pub async fn reassign_chunk(&self, chunk_index: usize) -> Option<[u8; 32]> {
        // Remove old assignment
        {
            let mut assignments = self.assignments.write().await;
            if let Some(old_peer) = assignments.remove(&chunk_index) {
                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(&old_peer) {
                    peer.record_failure();
                }
            }
        }

        // Assign to new peer
        self.assign_chunk(chunk_index).await
    }

    /// Record successful chunk download
    pub async fn record_success(&self, chunk_index: usize, bytes: u64, duration: Duration) {
        let assignments = self.assignments.read().await;
        let peer_id = if let Some(peer_id) = assignments.get(&chunk_index) {
            *peer_id
        } else {
            return;
        };
        drop(assignments);

        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&peer_id) {
            peer.record_success();
            peer.update_throughput(bytes, duration);
        }
    }

    /// Update peer RTT
    pub async fn update_peer_rtt(&self, peer_id: &[u8; 32], rtt_us: u64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.update_rtt(rtt_us);
        }
    }

    /// Get peer performance metrics
    pub async fn peer_performance(&self, peer_id: &[u8; 32]) -> Option<PeerPerformance> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    /// Get all peer performances
    pub async fn all_peer_performances(&self) -> Vec<PeerPerformance> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Get current strategy
    pub fn strategy(&self) -> ChunkAssignmentStrategy {
        self.strategy
    }

    /// Change strategy
    #[allow(dead_code)] // Public API for future use
    pub fn set_strategy(&mut self, strategy: ChunkAssignmentStrategy) {
        self.strategy = strategy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_performance_creation() {
        let peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());
        assert_eq!(peer.chunks_succeeded, 0);
        assert_eq!(peer.chunks_failed, 0);
        assert_eq!(peer.failure_rate(), 0.0);
        assert_eq!(peer.reliability_score(), 1.0);
    }

    #[test]
    fn test_peer_performance_failure_rate() {
        let mut peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());

        peer.record_success();
        peer.record_success();
        peer.record_failure();

        // Use approximate comparison for floating point
        assert!((peer.failure_rate() - 1.0 / 3.0).abs() < 1e-10);
        assert!((peer.reliability_score() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_peer_performance_scores() {
        let mut peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());

        // High throughput, low RTT
        peer.throughput_bps = 50 * 1024 * 1024; // 50 MB/s
        peer.rtt_us = 10_000; // 10ms

        // Update cached score after directly modifying metrics (for testing)
        peer.update_cached_score();

        assert!(peer.speed_score() > 0.4);
        assert!(peer.latency_score() > 0.9);
        assert!(peer.performance_score() > 0.7);
    }

    #[test]
    fn test_peer_performance_capacity() {
        let mut peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());
        peer.max_concurrent = 2;

        assert!(peer.has_capacity());

        peer.record_assignment();
        assert!(peer.has_capacity());

        peer.record_assignment();
        assert!(!peer.has_capacity());
    }

    #[test]
    fn test_peer_performance_rtt_update() {
        let mut peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());
        let initial_rtt = peer.rtt_us;

        peer.update_rtt(50_000); // 50ms
        assert!(peer.rtt_us != initial_rtt);
    }

    #[test]
    fn test_peer_performance_throughput_update() {
        let mut peer = PeerPerformance::new([1u8; 32], "127.0.0.1:8420".parse().unwrap());
        let initial_throughput = peer.throughput_bps;

        // Update with significantly different value to ensure EMA changes it
        peer.update_throughput(10_000_000, Duration::from_secs(1)); // 10 MB/s

        // Verify throughput changed (should be between initial and new value due to EMA)
        assert!(peer.throughput_bps > initial_throughput);
        assert!(peer.throughput_bps != initial_throughput);
    }

    #[tokio::test]
    async fn test_multi_peer_coordinator_creation() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);
        assert_eq!(coordinator.strategy(), ChunkAssignmentStrategy::RoundRobin);
    }

    #[tokio::test]
    async fn test_multi_peer_add_remove() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);
        let peer_id = [1u8; 32];
        let address = "127.0.0.1:8420".parse().unwrap();

        coordinator.add_peer(peer_id, address).await;
        assert!(coordinator.peer_performance(&peer_id).await.is_some());

        coordinator.remove_peer(&peer_id).await;
        assert!(coordinator.peer_performance(&peer_id).await.is_none());
    }

    #[tokio::test]
    async fn test_multi_peer_round_robin() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);
        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        coordinator
            .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
            .await;
        coordinator
            .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
            .await;

        let assigned1 = coordinator.assign_chunk(0).await.unwrap();
        let assigned2 = coordinator.assign_chunk(1).await.unwrap();

        // Should alternate between peers
        assert_ne!(assigned1, assigned2);
    }

    #[tokio::test]
    async fn test_multi_peer_fastest_first() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::FastestFirst);
        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        coordinator
            .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
            .await;
        coordinator
            .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
            .await;

        // Make peer2 faster
        {
            let mut peers = coordinator.peers.write().await;
            if let Some(peer) = peers.get_mut(&peer2) {
                peer.throughput_bps = 100 * 1024 * 1024; // 100 MB/s
            }
        }

        let assigned = coordinator.assign_chunk(0).await.unwrap();
        assert_eq!(assigned, peer2); // Should pick fastest peer
    }

    #[tokio::test]
    async fn test_multi_peer_geographic() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::Geographic);
        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        coordinator
            .add_peer(peer1, "127.0.0.1:8420".parse().unwrap())
            .await;
        coordinator
            .add_peer(peer2, "127.0.0.1:8421".parse().unwrap())
            .await;

        // Make peer1 closer (lower RTT)
        {
            let mut peers = coordinator.peers.write().await;
            if let Some(peer) = peers.get_mut(&peer1) {
                peer.rtt_us = 5_000; // 5ms
            }
        }

        let assigned = coordinator.assign_chunk(0).await.unwrap();
        assert_eq!(assigned, peer1); // Should pick closest peer
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

        let assigned1 = coordinator.assign_chunk(0).await.unwrap();
        let reassigned = coordinator.reassign_chunk(0).await.unwrap();

        // Should be reassigned to different peer
        assert_ne!(assigned1, reassigned);
    }

    #[tokio::test]
    async fn test_multi_peer_record_success() {
        let coordinator = MultiPeerCoordinator::new(ChunkAssignmentStrategy::RoundRobin);
        let peer_id = [1u8; 32];

        coordinator
            .add_peer(peer_id, "127.0.0.1:8420".parse().unwrap())
            .await;
        coordinator.assign_chunk(0).await.unwrap();

        coordinator
            .record_success(0, 1_000_000, Duration::from_secs(1))
            .await;

        let perf = coordinator.peer_performance(&peer_id).await.unwrap();
        assert_eq!(perf.chunks_succeeded, 1);
    }
}
