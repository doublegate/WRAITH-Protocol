//! Kademlia Routing Table Implementation
//!
//! This module implements the k-bucket routing table used in Kademlia DHT.
//! The routing table organizes peers by their XOR distance from the local node,
//! enabling efficient O(log n) lookups.

use super::node_id::NodeId;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Kademlia k-bucket size (number of peers per bucket)
///
/// Standard Kademlia uses k=20. This provides good redundancy while
/// keeping routing table size manageable.
pub const K: usize = 20;

/// Number of buckets in the routing table (one per bit of NodeId)
pub const NUM_BUCKETS: usize = 256;

/// Peer liveness timeout (15 minutes)
///
/// Peers that haven't responded in this duration are considered dead
/// and may be replaced in k-buckets.
const PEER_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// DHT peer information
///
/// Stores metadata about a peer in the DHT, including their NodeId,
/// network address, last seen timestamp, and RTT measurement.
#[derive(Clone, Debug)]
pub struct DhtPeer {
    /// Node identifier (256-bit)
    pub id: NodeId,
    /// Network address (IP:port)
    pub addr: SocketAddr,
    /// Last time we received a response from this peer
    pub last_seen: Instant,
    /// Round-trip time measurement (if available)
    pub rtt: Option<Duration>,
}

impl DhtPeer {
    /// Create a new DhtPeer
    ///
    /// # Arguments
    ///
    /// * `id` - The peer's NodeId
    /// * `addr` - The peer's network address
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{NodeId, DhtPeer};
    /// use std::net::SocketAddr;
    ///
    /// let id = NodeId::random();
    /// let addr = "127.0.0.1:8000".parse().unwrap();
    /// let peer = DhtPeer::new(id, addr);
    /// ```
    #[must_use]
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            last_seen: Instant::now(),
            rtt: None,
        }
    }

    /// Check if the peer is considered alive
    ///
    /// A peer is alive if they responded within the last 15 minutes.
    ///
    /// # Returns
    ///
    /// `true` if peer responded recently, `false` otherwise
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.last_seen.elapsed() < PEER_TIMEOUT
    }

    /// Update the last seen timestamp
    ///
    /// Should be called whenever we receive a response from this peer.
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Update the RTT measurement
    ///
    /// # Arguments
    ///
    /// * `rtt` - Measured round-trip time
    pub fn update_rtt(&mut self, rtt: Duration) {
        self.rtt = Some(rtt);
    }
}

/// K-bucket for storing peers at a specific distance range
///
/// K-buckets use LRU eviction policy: when full, the least recently seen
/// alive peer is replaced. Dead peers are always replaced immediately.
#[derive(Clone, Debug)]
pub struct KBucket {
    /// Peers in this bucket, ordered by last-seen (LRU)
    peers: VecDeque<DhtPeer>,
    /// Maximum number of peers this bucket can hold
    capacity: usize,
}

impl KBucket {
    /// Create a new k-bucket with the given capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of peers (typically K=20)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::KBucket;
    ///
    /// let bucket = KBucket::new(20);
    /// assert_eq!(bucket.len(), 0);
    /// ```
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            peers: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Insert a peer into the bucket
    ///
    /// Insertion follows these rules:
    /// 1. If peer already exists, move to front (most recently seen)
    /// 2. If bucket not full, append peer
    /// 3. If bucket full, try to replace dead peer
    /// 4. If all peers alive, reject insertion (bucket full)
    ///
    /// # Arguments
    ///
    /// * `peer` - The peer to insert
    ///
    /// # Errors
    ///
    /// Returns `DhtError::BucketFull` if bucket is full with all alive peers
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{KBucket, DhtPeer, NodeId};
    ///
    /// let mut bucket = KBucket::new(3);
    /// let peer = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// assert!(bucket.insert(peer).is_ok());
    /// ```
    pub fn insert(&mut self, peer: DhtPeer) -> Result<(), DhtError> {
        // Check if peer already exists
        if let Some(pos) = self.peers.iter().position(|p| p.id == peer.id) {
            // Move to front (most recently seen)
            let mut existing = self.peers.remove(pos).unwrap();
            existing.touch();
            if let Some(rtt) = peer.rtt {
                existing.update_rtt(rtt);
            }
            self.peers.push_front(existing);
            return Ok(());
        }

        // If bucket not full, add peer
        if self.peers.len() < self.capacity {
            self.peers.push_front(peer);
            return Ok(());
        }

        // Bucket full - try to replace dead peer
        if let Some(pos) = self.peers.iter().position(|p| !p.is_alive()) {
            self.peers.remove(pos);
            self.peers.push_front(peer);
            return Ok(());
        }

        // Bucket full with all alive peers
        Err(DhtError::BucketFull)
    }

    /// Get a peer by NodeId
    ///
    /// # Arguments
    ///
    /// * `id` - NodeId to search for
    ///
    /// # Returns
    ///
    /// Reference to the peer if found, None otherwise
    #[must_use]
    pub fn get(&self, id: &NodeId) -> Option<&DhtPeer> {
        self.peers.iter().find(|p| p.id == *id)
    }

    /// Get all peers in this bucket
    ///
    /// # Returns
    ///
    /// Slice of all peers, ordered by last-seen (most recent first)
    #[must_use]
    pub fn peers(&self) -> &VecDeque<DhtPeer> {
        &self.peers
    }

    /// Get the K closest peers to a target NodeId
    ///
    /// # Arguments
    ///
    /// * `target` - The NodeId to find closest peers to
    /// * `count` - Maximum number of peers to return
    ///
    /// # Returns
    ///
    /// Vector of up to `count` closest peers, sorted by distance
    #[must_use]
    pub fn closest_to(&self, target: &NodeId, count: usize) -> Vec<DhtPeer> {
        let mut peers: Vec<_> = self.peers.iter().cloned().collect();
        peers.sort_by_key(|p| p.id.distance(target));
        peers.into_iter().take(count).collect()
    }

    /// Remove dead peers from the bucket
    ///
    /// Peers that haven't responded within the timeout are removed.
    pub fn prune(&mut self) {
        self.peers.retain(|p| p.is_alive());
    }

    /// Get the number of peers in this bucket
    ///
    /// # Returns
    ///
    /// Number of peers currently stored
    #[must_use]
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Check if the bucket is empty
    ///
    /// # Returns
    ///
    /// `true` if bucket contains no peers
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

/// Kademlia routing table
///
/// The routing table contains 256 k-buckets, one for each bit position
/// in the 256-bit NodeId space. Peers are placed in buckets based on
/// their XOR distance from the local node.
#[derive(Clone, Debug)]
pub struct RoutingTable {
    /// Local node's identifier
    local_id: NodeId,
    /// 256 k-buckets, one per bit of the NodeId
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    /// Create a new routing table for the given local NodeId
    ///
    /// # Arguments
    ///
    /// * `local_id` - The local node's identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{RoutingTable, NodeId};
    ///
    /// let local_id = NodeId::random();
    /// let table = RoutingTable::new(local_id);
    /// assert_eq!(table.peer_count(), 0);
    /// ```
    #[must_use]
    pub fn new(local_id: NodeId) -> Self {
        let buckets = (0..NUM_BUCKETS).map(|_| KBucket::new(K)).collect();
        Self { local_id, buckets }
    }

    /// Get the local node's identifier
    ///
    /// # Returns
    ///
    /// Reference to the local NodeId
    #[must_use]
    pub const fn local_id(&self) -> &NodeId {
        &self.local_id
    }

    /// Calculate which bucket a peer belongs to
    ///
    /// The bucket index is determined by the position of the first
    /// differing bit in the XOR distance between the local ID and peer ID.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer's NodeId
    ///
    /// # Returns
    ///
    /// Bucket index (0-255), or None if peer_id equals local_id
    #[must_use]
    fn bucket_index(&self, peer_id: &NodeId) -> Option<usize> {
        peer_id.bucket_index(&self.local_id)
    }

    /// Insert a peer into the routing table
    ///
    /// The peer is automatically placed in the correct k-bucket based
    /// on XOR distance from the local node.
    ///
    /// # Arguments
    ///
    /// * `peer` - The peer to insert
    ///
    /// # Errors
    ///
    /// Returns `DhtError::SelfInsert` if peer_id equals local_id.
    /// Returns `DhtError::BucketFull` if the appropriate bucket is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{RoutingTable, DhtPeer, NodeId};
    ///
    /// let local_id = NodeId::random();
    /// let mut table = RoutingTable::new(local_id);
    ///
    /// let peer = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// table.insert(peer).unwrap();
    /// assert_eq!(table.peer_count(), 1);
    /// ```
    pub fn insert(&mut self, peer: DhtPeer) -> Result<(), DhtError> {
        let bucket_idx = self.bucket_index(&peer.id).ok_or(DhtError::SelfInsert)?;
        self.buckets[bucket_idx].insert(peer)
    }

    /// Find the K closest peers to a target NodeId
    ///
    /// Searches all buckets and returns the K closest peers to the target,
    /// sorted by XOR distance.
    ///
    /// # Arguments
    ///
    /// * `target` - The NodeId to find closest peers to
    /// * `count` - Maximum number of peers to return (typically K=20)
    ///
    /// # Returns
    ///
    /// Vector of up to `count` closest peers, sorted by distance
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{RoutingTable, DhtPeer, NodeId};
    ///
    /// let local_id = NodeId::random();
    /// let mut table = RoutingTable::new(local_id);
    ///
    /// for i in 0..50 {
    ///     let peer = DhtPeer::new(NodeId::random(), format!("127.0.0.1:{}", 8000 + i).parse().unwrap());
    ///     let _ = table.insert(peer);
    /// }
    ///
    /// let target = NodeId::random();
    /// let closest = table.closest_peers(&target, 20);
    /// assert!(closest.len() <= 20);
    /// ```
    #[must_use]
    pub fn closest_peers(&self, target: &NodeId, count: usize) -> Vec<DhtPeer> {
        let mut all_peers = Vec::new();

        for bucket in &self.buckets {
            all_peers.extend(bucket.peers().iter().cloned());
        }

        // Sort by distance to target
        all_peers.sort_by_key(|p| p.id.distance(target));

        // Return up to `count` closest peers
        all_peers.into_iter().take(count).collect()
    }

    /// Get a peer by NodeId
    ///
    /// # Arguments
    ///
    /// * `id` - NodeId to search for
    ///
    /// # Returns
    ///
    /// Cloned peer if found, None otherwise
    #[must_use]
    pub fn get_peer(&self, id: &NodeId) -> Option<DhtPeer> {
        let bucket_idx = self.bucket_index(id)?;
        self.buckets[bucket_idx].get(id).cloned()
    }

    /// Get all peers in the routing table
    ///
    /// # Returns
    ///
    /// Vector of all peers from all buckets
    #[must_use]
    pub fn all_peers(&self) -> Vec<DhtPeer> {
        let mut peers = Vec::new();
        for bucket in &self.buckets {
            peers.extend(bucket.peers().iter().cloned());
        }
        peers
    }

    /// Get the total number of peers across all buckets
    ///
    /// # Returns
    ///
    /// Total peer count
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{RoutingTable, DhtPeer, NodeId};
    ///
    /// let local_id = NodeId::random();
    /// let mut table = RoutingTable::new(local_id);
    /// assert_eq!(table.peer_count(), 0);
    ///
    /// let peer = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// table.insert(peer).unwrap();
    /// assert_eq!(table.peer_count(), 1);
    /// ```
    #[must_use]
    pub fn peer_count(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// Remove dead peers from all buckets
    ///
    /// Should be called periodically to maintain routing table health.
    pub fn prune(&mut self) {
        for bucket in &mut self.buckets {
            bucket.prune();
        }
    }

    /// Identify buckets that need refreshing
    ///
    /// Returns indices of buckets that have fewer than K/2 peers,
    /// indicating they should be refreshed with new lookups.
    ///
    /// # Returns
    ///
    /// Vector of bucket indices that need refresh
    #[must_use]
    pub fn buckets_needing_refresh(&self) -> Vec<usize> {
        self.buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.len() < K / 2)
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// DHT errors
#[derive(Debug, Error)]
pub enum DhtError {
    /// K-bucket is full and all peers are alive
    #[error("K-bucket is full")]
    BucketFull,

    /// Attempted to insert local node into routing table
    #[error("Cannot insert local node into routing table")]
    SelfInsert,

    /// Peer not found in routing table
    #[error("Peer not found")]
    PeerNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_peer_creation() {
        let id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let peer = DhtPeer::new(id, addr);

        assert_eq!(peer.id, id);
        assert_eq!(peer.addr, addr);
        assert!(peer.is_alive());
        assert_eq!(peer.rtt, None);
    }

    #[test]
    fn test_dht_peer_touch() {
        let mut peer = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
        let first_seen = peer.last_seen;

        std::thread::sleep(std::time::Duration::from_millis(10));
        peer.touch();

        assert!(peer.last_seen > first_seen);
    }

    #[test]
    fn test_kbucket_insert() {
        let mut bucket = KBucket::new(3);

        let peer1 = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
        let peer2 = DhtPeer::new(NodeId::random(), "127.0.0.1:8001".parse().unwrap());
        let peer3 = DhtPeer::new(NodeId::random(), "127.0.0.1:8002".parse().unwrap());

        assert!(bucket.insert(peer1).is_ok());
        assert!(bucket.insert(peer2).is_ok());
        assert!(bucket.insert(peer3).is_ok());
        assert_eq!(bucket.len(), 3);

        // Fourth insert should fail (bucket full)
        let peer4 = DhtPeer::new(NodeId::random(), "127.0.0.1:8003".parse().unwrap());
        assert!(matches!(bucket.insert(peer4), Err(DhtError::BucketFull)));
    }

    #[test]
    fn test_kbucket_lru() {
        let mut bucket = KBucket::new(3);

        let id1 = NodeId::random();
        let peer1 = DhtPeer::new(id1, "127.0.0.1:8000".parse().unwrap());
        bucket.insert(peer1.clone()).unwrap();

        let peer2 = DhtPeer::new(NodeId::random(), "127.0.0.1:8001".parse().unwrap());
        bucket.insert(peer2).unwrap();

        // Re-insert peer1 (should move to front)
        let peer1_updated = DhtPeer::new(id1, "127.0.0.1:8000".parse().unwrap());
        bucket.insert(peer1_updated).unwrap();

        assert_eq!(bucket.len(), 2);
        assert_eq!(bucket.peers().front().unwrap().id, id1);
    }

    #[test]
    fn test_kbucket_prune() {
        let mut bucket = KBucket::new(3);

        let mut peer = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
        peer.last_seen = Instant::now() - Duration::from_secs(20 * 60); // 20 minutes ago
        bucket.insert(peer).unwrap();

        assert_eq!(bucket.len(), 1);
        bucket.prune();
        assert_eq!(bucket.len(), 0);
    }

    #[test]
    fn test_routing_table_insert() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        for i in 0..50 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8000 + i).parse().unwrap(),
            );
            let _ = table.insert(peer);
        }

        assert!(table.peer_count() > 0);
        assert!(table.peer_count() <= 50);
    }

    #[test]
    fn test_routing_table_self_insert() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        let peer = DhtPeer::new(local_id, "127.0.0.1:8000".parse().unwrap());
        assert!(matches!(table.insert(peer), Err(DhtError::SelfInsert)));
    }

    #[test]
    fn test_routing_table_closest_peers() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        // Insert some peers
        for i in 0..20 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8000 + i).parse().unwrap(),
            );
            let _ = table.insert(peer);
        }

        let target = NodeId::random();
        let closest = table.closest_peers(&target, 5);

        assert!(closest.len() <= 5);

        // Verify they're sorted by distance
        for i in 0..closest.len().saturating_sub(1) {
            let dist1 = closest[i].id.distance(&target);
            let dist2 = closest[i + 1].id.distance(&target);
            assert!(dist1 <= dist2);
        }
    }

    #[test]
    fn test_routing_table_get_peer() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        let peer_id = NodeId::random();
        let peer = DhtPeer::new(peer_id, "127.0.0.1:8000".parse().unwrap());
        table.insert(peer).unwrap();

        let retrieved = table.get_peer(&peer_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, peer_id);
    }

    #[test]
    fn test_routing_table_buckets_needing_refresh() {
        let local_id = NodeId::random();
        let table = RoutingTable::new(local_id);

        let needing_refresh = table.buckets_needing_refresh();
        // All buckets start empty, so all need refresh
        assert_eq!(needing_refresh.len(), 256);
    }
}
