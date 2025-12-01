# Phase 5: Discovery & NAT Traversal Sprint Planning

**Duration:** Weeks 25-31 (5-7 weeks)
**Total Story Points:** 123
**Risk Level:** High (network complexity, NAT diversity)

---

## Phase Overview

**Goal:** Implement peer discovery using privacy-enhanced Kademlia DHT, DERP-style relay protocol for NAT traversal, and various hole-punching techniques to establish direct connections through NATs and firewalls.

### Success Criteria

- [ ] DHT lookup: <500ms (typical)
- [ ] Relay connection established: <200ms
- [ ] NAT traversal success rate: >90%
- [ ] Hole punching timeout: <5 seconds
- [ ] Graceful relay fallback when direct connection fails
- [ ] Privacy: No cleartext peer identifiers in DHT
- [ ] Scalability: DHT supports 100K+ nodes

### Dependencies

- Phase 2 complete (crypto for encrypted announcements)
- Phase 3 complete (transport layer)
- Phase 4 complete (obfuscation optional)

### Deliverables

1. Privacy-enhanced Kademlia DHT implementation
2. Encrypted peer announcements
3. DHT query/store operations
4. DERP-style relay server and client
5. NAT type detection (STUN-like)
6. Endpoint discovery
7. Simultaneous open hole punching
8. Birthday attack for symmetric NAT
9. Connection migration
10. Path validation

---

## Sprint Breakdown

### Sprint 5.1: Kademlia DHT Foundation (Weeks 25-26)

**Duration:** 2 weeks
**Story Points:** 26

**5.1.1: DHT Node Implementation** (13 SP)

```rust
// wraith-discovery/src/dht/node.rs

use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Kademlia K-bucket size
const K: usize = 20;

/// Number of bits in node ID
const ID_BITS: usize = 256;

/// DHT node identifier (32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Generate random node ID
    pub fn random() -> Self {
        let mut id = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut id[..]);
        Self(id)
    }

    /// Generate from public key
    pub fn from_public_key(pubkey: &[u8; 32]) -> Self {
        use blake3::hash;
        let hash = hash(pubkey);
        Self(hash.into())
    }

    /// XOR distance between two node IDs
    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut result = [0u8; 32];
        for i in 0..32 {
            result[i] = self.0[i] ^ other.0[i];
        }
        NodeId(result)
    }

    /// Common prefix length (for routing table)
    pub fn common_prefix_len(&self, other: &NodeId) -> usize {
        let mut len = 0;
        for i in 0..32 {
            let xor = self.0[i] ^ other.0[i];
            if xor == 0 {
                len += 8;
            } else {
                len += xor.leading_zeros() as usize;
                break;
            }
        }
        len
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// DHT peer information
#[derive(Clone, Debug)]
pub struct DhtPeer {
    pub id: NodeId,
    pub addr: SocketAddr,
    pub last_seen: Instant,
    pub rtt: Option<Duration>,
}

impl DhtPeer {
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            last_seen: Instant::now(),
            rtt: None,
        }
    }

    /// Check if peer is responsive (seen in last 15 minutes)
    pub fn is_alive(&self) -> bool {
        self.last_seen.elapsed() < Duration::from_secs(15 * 60)
    }

    /// Update last seen time
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Update RTT
    pub fn update_rtt(&mut self, rtt: Duration) {
        self.rtt = Some(rtt);
    }
}

/// K-bucket for storing peers at specific distance
pub struct KBucket {
    peers: Vec<DhtPeer>,
    max_size: usize,
}

impl KBucket {
    pub fn new(max_size: usize) -> Self {
        Self {
            peers: Vec::new(),
            max_size,
        }
    }

    /// Add peer to bucket
    pub fn insert(&mut self, peer: DhtPeer) -> Result<(), DhtError> {
        // Check if peer already exists
        if let Some(existing) = self.peers.iter_mut().find(|p| p.id == peer.id) {
            existing.touch();
            return Ok(());
        }

        // If bucket not full, add peer
        if self.peers.len() < self.max_size {
            self.peers.push(peer);
            return Ok(());
        }

        // Bucket full - try to replace dead peer
        if let Some(pos) = self.peers.iter().position(|p| !p.is_alive()) {
            self.peers[pos] = peer;
            return Ok(());
        }

        // Bucket full with all alive peers
        Err(DhtError::BucketFull)
    }

    /// Get peer by ID
    pub fn get(&self, id: &NodeId) -> Option<&DhtPeer> {
        self.peers.iter().find(|p| p.id == *id)
    }

    /// Get all peers
    pub fn peers(&self) -> &[DhtPeer] {
        &self.peers
    }

    /// Get closest peers to target
    pub fn closest_to(&self, target: &NodeId, count: usize) -> Vec<DhtPeer> {
        let mut peers = self.peers.clone();
        peers.sort_by_key(|p| p.id.distance(target).0);
        peers.into_iter().take(count).collect()
    }

    /// Remove dead peers
    pub fn prune(&mut self) {
        self.peers.retain(|p| p.is_alive());
    }
}

#[derive(Debug)]
pub enum DhtError {
    BucketFull,
    PeerNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_distance() {
        let id1 = NodeId([1u8; 32]);
        let id2 = NodeId([2u8; 32]);

        let dist = id1.distance(&id2);

        // XOR of 1 and 2 is 3
        assert_eq!(dist.0[0], 3);
        assert_eq!(dist.0[1], 3);
    }

    #[test]
    fn test_node_id_common_prefix() {
        let id1 = NodeId([0b11110000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let id2 = NodeId([0b11111111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let prefix = id1.common_prefix_len(&id2);
        assert_eq!(prefix, 4); // First 4 bits match
    }

    #[test]
    fn test_kbucket_insert() {
        let mut bucket = KBucket::new(3);

        let peer1 = DhtPeer::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
        let peer2 = DhtPeer::new(NodeId::random(), "127.0.0.1:8001".parse().unwrap());
        let peer3 = DhtPeer::new(NodeId::random(), "127.0.0.1:8002".parse().unwrap());

        bucket.insert(peer1).unwrap();
        bucket.insert(peer2).unwrap();
        bucket.insert(peer3).unwrap();

        assert_eq!(bucket.peers().len(), 3);

        // Fourth insert should fail (bucket full)
        let peer4 = DhtPeer::new(NodeId::random(), "127.0.0.1:8003".parse().unwrap());
        assert!(bucket.insert(peer4).is_err());
    }
}
```

**Acceptance Criteria:**
- [ ] Node ID generation from public key
- [ ] XOR distance calculation correct
- [ ] K-bucket insertion/eviction works
- [ ] Peer liveness tracking
- [ ] Bucket pruning removes dead peers

---

**5.1.2: Routing Table** (13 SP)

```rust
// wraith-discovery/src/dht/routing.rs

use super::node::{NodeId, DhtPeer, KBucket, K};
use std::net::SocketAddr;

/// Kademlia routing table
pub struct RoutingTable {
    local_id: NodeId,
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        // Create 256 buckets (one for each bit of the ID)
        let buckets = (0..256).map(|_| KBucket::new(K)).collect();

        Self {
            local_id,
            buckets,
        }
    }

    /// Calculate which bucket a peer belongs to
    fn bucket_index(&self, peer_id: &NodeId) -> usize {
        let prefix_len = self.local_id.common_prefix_len(peer_id);

        // Bucket index is the first differing bit position
        // Clamped to [0, 255]
        prefix_len.min(255)
    }

    /// Add peer to routing table
    pub fn insert(&mut self, peer: DhtPeer) -> Result<(), super::node::DhtError> {
        let bucket_idx = self.bucket_index(&peer.id);
        self.buckets[bucket_idx].insert(peer)
    }

    /// Find K closest peers to target
    pub fn closest_peers(&self, target: &NodeId, count: usize) -> Vec<DhtPeer> {
        let mut all_peers = Vec::new();

        for bucket in &self.buckets {
            all_peers.extend(bucket.peers().iter().cloned());
        }

        // Sort by distance to target
        all_peers.sort_by_key(|p| p.id.distance(target).0);

        // Return up to `count` closest peers
        all_peers.into_iter().take(count).collect()
    }

    /// Get peer by ID
    pub fn get_peer(&self, id: &NodeId) -> Option<DhtPeer> {
        let bucket_idx = self.bucket_index(id);
        self.buckets[bucket_idx].get(id).cloned()
    }

    /// Get all peers in routing table
    pub fn all_peers(&self) -> Vec<DhtPeer> {
        let mut peers = Vec::new();
        for bucket in &self.buckets {
            peers.extend(bucket.peers().iter().cloned());
        }
        peers
    }

    /// Count total peers
    pub fn peer_count(&self) -> usize {
        self.buckets.iter().map(|b| b.peers().len()).sum()
    }

    /// Prune dead peers from all buckets
    pub fn prune(&mut self) {
        for bucket in &mut self.buckets {
            bucket.prune();
        }
    }

    /// Get peers for refresh (buckets with few peers)
    pub fn buckets_needing_refresh(&self) -> Vec<usize> {
        self.buckets.iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.peers().len() < K / 2)
            .map(|(idx, _)| idx)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_table_insert() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        for i in 0..50 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8000 + i).parse().unwrap()
            );
            let _ = table.insert(peer);
        }

        assert!(table.peer_count() > 0);
        assert!(table.peer_count() <= 50);
    }

    #[test]
    fn test_closest_peers() {
        let local_id = NodeId::random();
        let mut table = RoutingTable::new(local_id);

        // Insert some peers
        for i in 0..20 {
            let peer = DhtPeer::new(
                NodeId::random(),
                format!("127.0.0.1:{}", 8000 + i).parse().unwrap()
            );
            let _ = table.insert(peer);
        }

        let target = NodeId::random();
        let closest = table.closest_peers(&target, 5);

        assert!(closest.len() <= 5);

        // Verify they're actually closest (sorted by distance)
        for i in 0..closest.len().saturating_sub(1) {
            let dist1 = closest[i].id.distance(&target);
            let dist2 = closest[i + 1].id.distance(&target);
            assert!(dist1.0 <= dist2.0);
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Routing table stores peers in correct buckets
- [ ] Closest peers query returns K nearest
- [ ] Bucket refresh identification works
- [ ] Peer count accurate
- [ ] Pruning removes dead peers from all buckets

---

### Sprint 5.2: DHT Protocol (Weeks 26-27)

**Duration:** 1.5 weeks
**Story Points:** 26

**5.2.1: DHT RPC Messages** (13 SP)

```rust
// wraith-discovery/src/dht/protocol.rs

use super::node::{NodeId, DhtPeer};
use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

/// DHT RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtMessage {
    Ping(PingRequest),
    Pong(PongResponse),
    FindNode(FindNodeRequest),
    FoundNodes(FoundNodesResponse),
    Store(StoreRequest),
    StoreAck(StoreAckResponse),
    FindValue(FindValueRequest),
    FoundValue(FoundValueResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    pub sender_id: NodeId,
    pub sender_addr: SocketAddr,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongResponse {
    pub sender_id: NodeId,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNodeRequest {
    pub sender_id: NodeId,
    pub sender_addr: SocketAddr,
    pub target_id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundNodesResponse {
    pub sender_id: NodeId,
    pub peers: Vec<CompactPeer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactPeer {
    pub id: NodeId,
    pub addr: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    pub sender_id: NodeId,
    pub sender_addr: SocketAddr,
    pub key: [u8; 32],
    pub value: Vec<u8>,
    pub ttl: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAckResponse {
    pub sender_id: NodeId,
    pub stored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindValueRequest {
    pub sender_id: NodeId,
    pub sender_addr: SocketAddr,
    pub key: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FoundValueResponse {
    Value {
        sender_id: NodeId,
        value: Vec<u8>,
    },
    Peers {
        sender_id: NodeId,
        peers: Vec<CompactPeer>,
    },
}

impl DhtMessage {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Encrypt message for privacy
    pub fn encrypt(&self, key: &[u8; 32]) -> Result<Vec<u8>, EncryptError> {
        // Use AEAD from Phase 2
        let plaintext = self.to_bytes()?;

        // Encrypt with XChaCha20-Poly1305
        use wraith_crypto::aead::{AeadKey, generate_nonce};

        let aead_key = AeadKey::new(*key);
        let nonce = generate_nonce(&mut rand::thread_rng());

        let mut ciphertext = aead_key.encrypt(&nonce, &plaintext, b"")?;

        // Prepend nonce
        let mut encrypted = nonce.to_vec();
        encrypted.extend_from_slice(&ciphertext);

        Ok(encrypted)
    }

    /// Decrypt message
    pub fn decrypt(encrypted: &[u8], key: &[u8; 32]) -> Result<Self, EncryptError> {
        if encrypted.len() < 24 {
            return Err(EncryptError::TooShort);
        }

        // Extract nonce
        let mut nonce = [0u8; 24];
        nonce.copy_from_slice(&encrypted[..24]);

        let ciphertext = &encrypted[24..];

        // Decrypt
        use wraith_crypto::aead::AeadKey;

        let aead_key = AeadKey::new(*key);
        let plaintext = aead_key.decrypt(&nonce, ciphertext, b"")?;

        let message = Self::from_bytes(&plaintext)?;
        Ok(message)
    }
}

#[derive(Debug)]
pub enum EncryptError {
    Serialization(bincode::Error),
    Aead(chacha20poly1305::aead::Error),
    TooShort,
}

impl From<bincode::Error> for EncryptError {
    fn from(err: bincode::Error) -> Self {
        EncryptError::Serialization(err)
    }
}

impl From<chacha20poly1305::aead::Error> for EncryptError {
    fn from(err: chacha20poly1305::aead::Error) -> Self {
        EncryptError::Aead(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let bytes = msg.to_bytes().unwrap();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();

        match decoded {
            DhtMessage::Ping(ping) => assert_eq!(ping.nonce, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_encryption() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let key = [1u8; 32];

        let encrypted = msg.encrypt(&key).unwrap();
        let decrypted = DhtMessage::decrypt(&encrypted, &key).unwrap();

        match decrypted {
            DhtMessage::Ping(ping) => assert_eq!(ping.nonce, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_encryption_wrong_key_fails() {
        let msg = DhtMessage::Ping(PingRequest {
            sender_id: NodeId::random(),
            sender_addr: "127.0.0.1:8000".parse().unwrap(),
            nonce: 12345,
        });

        let key1 = [1u8; 32];
        let key2 = [2u8; 32];

        let encrypted = msg.encrypt(&key1).unwrap();
        assert!(DhtMessage::decrypt(&encrypted, &key2).is_err());
    }
}
```

**Acceptance Criteria:**
- [ ] All DHT message types serializable
- [ ] Encryption/decryption works
- [ ] Wrong key fails decryption
- [ ] Compact peer encoding efficient
- [ ] Message format documented

---

**5.2.2: DHT Operations (FIND_NODE, STORE)** (13 SP)

```rust
// wraith-discovery/src/dht/operations.rs

use super::node::{NodeId, DhtPeer, K};
use super::routing::RoutingTable;
use super::protocol::*;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::collections::HashMap;

pub struct DhtNode {
    id: NodeId,
    addr: SocketAddr,
    routing_table: RoutingTable,
    storage: HashMap<[u8; 32], StoredValue>,
}

struct StoredValue {
    data: Vec<u8>,
    stored_at: Instant,
    ttl: Duration,
}

impl DhtNode {
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            routing_table: RoutingTable::new(id),
            storage: HashMap::new(),
        }
    }

    /// Perform iterative FIND_NODE lookup
    pub async fn find_node(&mut self, target: &NodeId) -> Vec<DhtPeer> {
        let alpha = 3; // Parallelism factor
        let mut queried = std::collections::HashSet::new();
        let mut closest = self.routing_table.closest_peers(target, K);

        loop {
            // Find up to alpha unqueried peers
            let to_query: Vec<_> = closest.iter()
                .filter(|p| !queried.contains(&p.id))
                .take(alpha)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break; // No more peers to query
            }

            // Query peers in parallel
            let mut responses = Vec::new();

            for peer in &to_query {
                queried.insert(peer.id);

                // Send FIND_NODE request
                let request = DhtMessage::FindNode(FindNodeRequest {
                    sender_id: self.id,
                    sender_addr: self.addr,
                    target_id: *target,
                });

                if let Ok(response) = self.send_rpc(peer.addr, request).await {
                    if let DhtMessage::FoundNodes(found) = response {
                        responses.push(found);
                    }
                }
            }

            // Merge responses into closest list
            let mut updated = false;

            for found in responses {
                for compact_peer in found.peers {
                    let peer = DhtPeer::new(compact_peer.id, compact_peer.addr);
                    let dist = peer.id.distance(target);

                    // Check if this peer is closer than our furthest close peer
                    if closest.len() < K {
                        closest.push(peer);
                        updated = true;
                    } else if let Some(furthest) = closest.last() {
                        let furthest_dist = furthest.id.distance(target);
                        if dist.0 < furthest_dist.0 {
                            closest.pop();
                            closest.push(peer);
                            updated = true;
                        }
                    }
                }

                // Re-sort by distance
                closest.sort_by_key(|p| p.id.distance(target).0);
            }

            if !updated {
                break; // No improvement, we're done
            }
        }

        closest
    }

    /// Store value in DHT
    pub async fn store(&mut self, key: [u8; 32], value: Vec<u8>, ttl: Duration) -> Result<(), DhtError> {
        // Find K closest nodes to key
        let key_id = NodeId(key);
        let closest = self.find_node(&key_id).await;

        // Send STORE requests to all of them
        let mut stored_count = 0;

        for peer in closest {
            let request = DhtMessage::Store(StoreRequest {
                sender_id: self.id,
                sender_addr: self.addr,
                key,
                value: value.clone(),
                ttl: ttl.as_secs(),
            });

            if let Ok(DhtMessage::StoreAck(ack)) = self.send_rpc(peer.addr, request).await {
                if ack.stored {
                    stored_count += 1;
                }
            }
        }

        if stored_count > 0 {
            Ok(())
        } else {
            Err(DhtError::StoreFailed)
        }
    }

    /// Retrieve value from DHT
    pub async fn find_value(&mut self, key: [u8; 32]) -> Result<Vec<u8>, DhtError> {
        let key_id = NodeId(key);

        // Start with closest known peers
        let mut queried = std::collections::HashSet::new();
        let mut closest = self.routing_table.closest_peers(&key_id, K);

        let alpha = 3;

        loop {
            let to_query: Vec<_> = closest.iter()
                .filter(|p| !queried.contains(&p.id))
                .take(alpha)
                .cloned()
                .collect();

            if to_query.is_empty() {
                return Err(DhtError::ValueNotFound);
            }

            for peer in to_query {
                queried.insert(peer.id);

                let request = DhtMessage::FindValue(FindValueRequest {
                    sender_id: self.id,
                    sender_addr: self.addr,
                    key,
                });

                if let Ok(response) = self.send_rpc(peer.addr, request).await {
                    match response {
                        DhtMessage::FoundValue(FoundValueResponse::Value { value, .. }) => {
                            return Ok(value);
                        }
                        DhtMessage::FoundValue(FoundValueResponse::Peers { peers, .. }) => {
                            // Add these peers to our search
                            for compact_peer in peers {
                                let new_peer = DhtPeer::new(compact_peer.id, compact_peer.addr);
                                if !queried.contains(&new_peer.id) {
                                    closest.push(new_peer);
                                }
                            }
                            closest.sort_by_key(|p| p.id.distance(&key_id).0);
                            closest.truncate(K);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Handle incoming DHT RPC
    pub fn handle_rpc(&mut self, message: DhtMessage, from: SocketAddr) -> Option<DhtMessage> {
        match message {
            DhtMessage::Ping(ping) => {
                // Update routing table
                let peer = DhtPeer::new(ping.sender_id, ping.sender_addr);
                let _ = self.routing_table.insert(peer);

                Some(DhtMessage::Pong(PongResponse {
                    sender_id: self.id,
                    nonce: ping.nonce,
                }))
            }

            DhtMessage::FindNode(find) => {
                // Update routing table
                let peer = DhtPeer::new(find.sender_id, find.sender_addr);
                let _ = self.routing_table.insert(peer);

                // Return K closest peers to target
                let closest = self.routing_table.closest_peers(&find.target_id, K);

                let compact_peers = closest.into_iter()
                    .map(|p| CompactPeer { id: p.id, addr: p.addr })
                    .collect();

                Some(DhtMessage::FoundNodes(FoundNodesResponse {
                    sender_id: self.id,
                    peers: compact_peers,
                }))
            }

            DhtMessage::Store(store) => {
                // Store value locally
                let stored_value = StoredValue {
                    data: store.value,
                    stored_at: Instant::now(),
                    ttl: Duration::from_secs(store.ttl),
                };

                self.storage.insert(store.key, stored_value);

                Some(DhtMessage::StoreAck(StoreAckResponse {
                    sender_id: self.id,
                    stored: true,
                }))
            }

            DhtMessage::FindValue(find) => {
                // Check if we have the value
                if let Some(stored) = self.storage.get(&find.key) {
                    // Check if not expired
                    if stored.stored_at.elapsed() < stored.ttl {
                        return Some(DhtMessage::FoundValue(FoundValueResponse::Value {
                            sender_id: self.id,
                            value: stored.data.clone(),
                        }));
                    }
                }

                // Don't have value - return closest peers
                let key_id = NodeId(find.key);
                let closest = self.routing_table.closest_peers(&key_id, K);

                let compact_peers = closest.into_iter()
                    .map(|p| CompactPeer { id: p.id, addr: p.addr })
                    .collect();

                Some(DhtMessage::FoundValue(FoundValueResponse::Peers {
                    sender_id: self.id,
                    peers: compact_peers,
                }))
            }

            _ => None,
        }
    }

    /// Send RPC and wait for response (placeholder)
    async fn send_rpc(&self, addr: SocketAddr, message: DhtMessage) -> Result<DhtMessage, DhtError> {
        // TODO: Implement actual network send/receive with timeout
        todo!("Implement network RPC")
    }
}

#[derive(Debug)]
pub enum DhtError {
    StoreFailed,
    ValueNotFound,
    Timeout,
}
```

**Acceptance Criteria:**
- [ ] Iterative FIND_NODE works
- [ ] STORE replicates to K nodes
- [ ] FIND_VALUE retrieves stored data
- [ ] Expired values not returned
- [ ] Alpha parallelism implemented

---

### Sprint 5.3: DERP Relay (Weeks 27-29)

**Duration:** 2 weeks
**Story Points:** 26

**5.3.1: Relay Server** (13 SP)

```rust
// wraith-discovery/src/relay/server.rs

use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::{UdpSocket, TcpListener, TcpStream};

/// DERP-style relay server
pub struct RelayServer {
    /// Registered clients (key -> client info)
    clients: Arc<RwLock<HashMap<PublicKey, ClientInfo>>>,
    /// UDP socket for datagram relay
    udp_socket: Arc<UdpSocket>,
}

type PublicKey = [u8; 32];

struct ClientInfo {
    addr: SocketAddr,
    last_seen: std::time::Instant,
}

impl RelayServer {
    pub async fn new(bind_addr: SocketAddr) -> std::io::Result<Self> {
        let udp_socket = UdpSocket::bind(bind_addr).await?;

        Ok(Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            udp_socket: Arc::new(udp_socket),
        })
    }

    /// Run relay server
    pub async fn run(self: Arc<Self>) -> std::io::Result<()> {
        let udp_task = {
            let server = self.clone();
            tokio::spawn(async move {
                server.run_udp().await
            })
        };

        udp_task.await.unwrap()
    }

    async fn run_udp(&self) -> std::io::Result<()> {
        let mut buf = vec![0u8; 65536];

        loop {
            let (len, from) = self.udp_socket.recv_from(&mut buf).await?;
            let packet = &buf[..len];

            // Parse relay packet
            if let Ok(relay_packet) = RelayPacket::parse(packet) {
                self.handle_relay_packet(relay_packet, from).await;
            }
        }
    }

    async fn handle_relay_packet(&self, packet: RelayPacket, from: SocketAddr) {
        match packet {
            RelayPacket::Register { public_key } => {
                // Register client
                let mut clients = self.clients.write().await;
                clients.insert(public_key, ClientInfo {
                    addr: from,
                    last_seen: std::time::Instant::now(),
                });

                println!("Client registered: {:?} from {}", public_key, from);

                // Send acknowledgment
                let ack = RelayPacket::RegisterAck;
                if let Ok(bytes) = ack.to_bytes() {
                    let _ = self.udp_socket.send_to(&bytes, from).await;
                }
            }

            RelayPacket::Relay { to, data } => {
                // Forward packet to destination
                let clients = self.clients.read().await;

                if let Some(dest_client) = clients.get(&to) {
                    let relay_forward = RelayPacket::RelayForward {
                        from: packet.sender_key().unwrap(),
                        data,
                    };

                    if let Ok(bytes) = relay_forward.to_bytes() {
                        let _ = self.udp_socket.send_to(&bytes, dest_client.addr).await;
                    }
                }
            }

            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
enum RelayPacket {
    Register {
        public_key: PublicKey,
    },
    RegisterAck,
    Relay {
        to: PublicKey,
        data: Vec<u8>,
    },
    RelayForward {
        from: PublicKey,
        data: Vec<u8>,
    },
}

impl RelayPacket {
    fn parse(bytes: &[u8]) -> Result<Self, RelayError> {
        // Simplified parsing (use bincode or similar in practice)
        if bytes.is_empty() {
            return Err(RelayError::EmptyPacket);
        }

        match bytes[0] {
            0 => {
                // Register
                if bytes.len() < 33 {
                    return Err(RelayError::InvalidPacket);
                }
                let mut public_key = [0u8; 32];
                public_key.copy_from_slice(&bytes[1..33]);
                Ok(RelayPacket::Register { public_key })
            }
            1 => Ok(RelayPacket::RegisterAck),
            2 => {
                // Relay
                if bytes.len() < 33 {
                    return Err(RelayError::InvalidPacket);
                }
                let mut to = [0u8; 32];
                to.copy_from_slice(&bytes[1..33]);
                let data = bytes[33..].to_vec();
                Ok(RelayPacket::Relay { to, data })
            }
            _ => Err(RelayError::UnknownType),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>, RelayError> {
        let mut bytes = Vec::new();

        match self {
            RelayPacket::Register { public_key } => {
                bytes.push(0);
                bytes.extend_from_slice(public_key);
            }
            RelayPacket::RegisterAck => {
                bytes.push(1);
            }
            RelayPacket::Relay { to, data } => {
                bytes.push(2);
                bytes.extend_from_slice(to);
                bytes.extend_from_slice(data);
            }
            RelayPacket::RelayForward { from, data } => {
                bytes.push(3);
                bytes.extend_from_slice(from);
                bytes.extend_from_slice(data);
            }
        }

        Ok(bytes)
    }

    fn sender_key(&self) -> Option<PublicKey> {
        match self {
            RelayPacket::Register { public_key } => Some(*public_key),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum RelayError {
    EmptyPacket,
    InvalidPacket,
    UnknownType,
}
```

**Acceptance Criteria:**
- [ ] Relay server accepts registrations
- [ ] Packets forwarded to correct destination
- [ ] Client liveness tracking
- [ ] Handles thousands of concurrent clients
- [ ] Low latency (<10ms relay overhead)

---

**5.3.2: Relay Client** (13 SP)

```rust
// wraith-discovery/src/relay/client.rs

use std::net::SocketAddr;
use tokio::net::UdpSocket;

pub struct RelayClient {
    relay_addr: SocketAddr,
    socket: UdpSocket,
    public_key: [u8; 32],
}

impl RelayClient {
    pub async fn connect(
        relay_addr: SocketAddr,
        public_key: [u8; 32],
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(relay_addr).await?;

        let mut client = Self {
            relay_addr,
            socket,
            public_key,
        };

        client.register().await?;

        Ok(client)
    }

    async fn register(&mut self) -> std::io::Result<()> {
        use super::server::RelayPacket;

        let register = RelayPacket::Register {
            public_key: self.public_key,
        };

        let bytes = register.to_bytes().unwrap();
        self.socket.send(&bytes).await?;

        // Wait for acknowledgment
        let mut buf = [0u8; 1024];
        let len = self.socket.recv(&mut buf).await?;

        let ack = RelayPacket::parse(&buf[..len]).unwrap();
        if !matches!(ack, RelayPacket::RegisterAck) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Registration failed"
            ));
        }

        Ok(())
    }

    /// Send packet through relay to destination
    pub async fn send_to(&self, data: &[u8], dest_key: &[u8; 32]) -> std::io::Result<usize> {
        use super::server::RelayPacket;

        let relay_packet = RelayPacket::Relay {
            to: *dest_key,
            data: data.to_vec(),
        };

        let bytes = relay_packet.to_bytes().unwrap();
        self.socket.send(&bytes).await
    }

    /// Receive packet from relay
    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, [u8; 32])> {
        use super::server::RelayPacket;

        let len = self.socket.recv(buf).await?;
        let packet = RelayPacket::parse(&buf[..len]).unwrap();

        match packet {
            RelayPacket::RelayForward { from, data } => {
                buf[..data.len()].copy_from_slice(&data);
                Ok((data.len(), from))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unexpected packet type"
            )),
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Client connects to relay
- [ ] Registration succeeds
- [ ] send_to/recv_from work
- [ ] Connection maintained (heartbeat)
- [ ] Graceful disconnection

---

### Sprint 5.4: NAT Traversal (Weeks 29-31)

**Duration:** 2 weeks
**Story Points:** 34

**5.4.1: NAT Type Detection** (8 SP)

```rust
// wraith-discovery/src/nat/detection.rs

use std::net::{SocketAddr, Ipv4Addr};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// No NAT, public IP
    None,
    /// Full cone NAT (easiest to traverse)
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port-restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT (hardest to traverse)
    Symmetric,
}

pub struct NatDetector {
    stun_servers: Vec<SocketAddr>,
}

impl NatDetector {
    pub fn new() -> Self {
        Self {
            stun_servers: vec![
                "stun.l.google.com:19302".parse().unwrap(),
                "stun1.l.google.com:19302".parse().unwrap(),
            ],
        }
    }

    /// Detect NAT type using STUN-like probing
    pub async fn detect(&self) -> Result<NatType, NatError> {
        // Test 1: Bind socket and get external address from STUN server 1
        let socket1 = UdpSocket::bind("0.0.0.0:0").await?;
        let local_addr1 = socket1.local_addr()?;

        let external1 = self.get_external_addr(&socket1, self.stun_servers[0]).await?;

        // If external == local, we're not behind NAT
        if Self::is_public_ip(&external1.ip()) {
            if external1.ip() == local_addr1.ip() {
                return Ok(NatType::None);
            }
        }

        // Test 2: Query from different server, same local socket
        let external2 = self.get_external_addr(&socket1, self.stun_servers[1]).await?;

        // If external addresses different, it's symmetric NAT
        if external1 != external2 {
            return Ok(NatType::Symmetric);
        }

        // Test 3: Use different local socket, same server
        let socket2 = UdpSocket::bind("0.0.0.0:0").await?;
        let external3 = self.get_external_addr(&socket2, self.stun_servers[0]).await?;

        // Check if external port changes with local port
        if external1.port() != external3.port() {
            return Ok(NatType::PortRestrictedCone);
        }

        // Test 4: Check if we can receive from different IP
        // (This would require a more complex STUN implementation)

        // Default to restricted cone
        Ok(NatType::RestrictedCone)
    }

    async fn get_external_addr(
        &self,
        socket: &UdpSocket,
        stun_server: SocketAddr,
    ) -> Result<SocketAddr, NatError> {
        // Simplified STUN request/response
        // In production, use full STUN RFC 5389 implementation

        let request = self.create_stun_request();
        socket.send_to(&request, stun_server).await?;

        let mut buf = [0u8; 1024];
        let (len, _) = socket.recv_from(&mut buf).await?;

        self.parse_stun_response(&buf[..len])
    }

    fn create_stun_request(&self) -> Vec<u8> {
        // Simplified STUN Binding Request
        vec![
            0x00, 0x01, // Message Type: Binding Request
            0x00, 0x00, // Message Length: 0
            0x21, 0x12, 0xA4, 0x42, // Magic Cookie
            // Transaction ID (12 random bytes)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    }

    fn parse_stun_response(&self, data: &[u8]) -> Result<SocketAddr, NatError> {
        // Simplified STUN response parsing
        // Look for XOR-MAPPED-ADDRESS attribute

        // For now, return a placeholder
        Ok("0.0.0.0:0".parse().unwrap())
    }

    fn is_public_ip(ip: &std::net::IpAddr) -> bool {
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                !ipv4.is_private() &&
                !ipv4.is_loopback() &&
                !ipv4.is_link_local()
            }
            std::net::IpAddr::V6(ipv6) => {
                !ipv6.is_loopback() &&
                !ipv6.is_multicast()
            }
        }
    }
}

#[derive(Debug)]
pub enum NatError {
    Io(std::io::Error),
    Timeout,
    InvalidResponse,
}

impl From<std::io::Error> for NatError {
    fn from(err: std::io::Error) -> Self {
        NatError::Io(err)
    }
}
```

**Acceptance Criteria:**
- [ ] Detects all NAT types accurately
- [ ] STUN-like probing works
- [ ] Public IP detection correct
- [ ] Symmetric NAT identified
- [ ] Timeout handling

---

**5.4.2: Hole Punching (Simultaneous Open)** (13 SP)

```rust
// wraith-discovery/src/nat/hole_punch.rs

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use std::time::Duration;

pub struct HolePuncher {
    socket: UdpSocket,
}

impl HolePuncher {
    pub async fn new(bind_addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self { socket })
    }

    /// Perform simultaneous open hole punching
    pub async fn punch(
        &self,
        peer_external: SocketAddr,
        peer_internal: Option<SocketAddr>,
    ) -> Result<SocketAddr, PunchError> {
        // Try multiple strategies in parallel

        let strategies = vec![
            self.try_direct(peer_external),
            self.try_internal(peer_internal),
            self.try_sequential_ports(peer_external),
        ];

        // Race all strategies
        tokio::select! {
            result = strategies[0] => result,
            result = strategies[1] => result,
            result = strategies[2] => result,
        }
    }

    async fn try_direct(&self, peer: SocketAddr) -> Result<SocketAddr, PunchError> {
        // Send probes to peer's external address
        let probe = b"WRAITH_PROBE";

        for _ in 0..10 {
            self.socket.send_to(probe, peer).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Try to receive
            match tokio::time::timeout(
                Duration::from_millis(50),
                self.recv_probe()
            ).await {
                Ok(Ok(from)) if from.ip() == peer.ip() => {
                    return Ok(from);
                }
                _ => continue,
            }
        }

        Err(PunchError::Timeout)
    }

    async fn try_internal(&self, peer: Option<SocketAddr>) -> Result<SocketAddr, PunchError> {
        let peer = peer.ok_or(PunchError::NoInternalAddress)?;

        // Try peer's internal address (for LAN peers)
        let probe = b"WRAITH_PROBE";

        for _ in 0..5 {
            self.socket.send_to(probe, peer).await?;

            match tokio::time::timeout(
                Duration::from_millis(50),
                self.recv_probe()
            ).await {
                Ok(Ok(from)) if from == peer => {
                    return Ok(from);
                }
                _ => continue,
            }
        }

        Err(PunchError::Timeout)
    }

    async fn try_sequential_ports(&self, peer: SocketAddr) -> Result<SocketAddr, PunchError> {
        // Try sequential ports (for predictable NAT port allocation)
        let base_port = peer.port();

        for offset in 0..10 {
            let try_port = base_port.wrapping_add(offset);
            let try_addr = SocketAddr::new(peer.ip(), try_port);

            let probe = b"WRAITH_PROBE";
            self.socket.send_to(probe, try_addr).await?;

            match tokio::time::timeout(
                Duration::from_millis(50),
                self.recv_probe()
            ).await {
                Ok(Ok(from)) if from.ip() == peer.ip() => {
                    return Ok(from);
                }
                _ => continue,
            }
        }

        Err(PunchError::Timeout)
    }

    async fn recv_probe(&self) -> std::io::Result<SocketAddr> {
        let mut buf = [0u8; 1024];
        let (len, from) = self.socket.recv_from(&mut buf).await?;

        if &buf[..len] == b"WRAITH_PROBE" || &buf[..len] == b"WRAITH_RESPONSE" {
            // Send response
            self.socket.send_to(b"WRAITH_RESPONSE", from).await?;
            Ok(from)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a probe packet"
            ))
        }
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

#[derive(Debug)]
pub enum PunchError {
    Io(std::io::Error),
    Timeout,
    NoInternalAddress,
}

impl From<std::io::Error> for PunchError {
    fn from(err: std::io::Error) -> Self {
        PunchError::Io(err)
    }
}
```

**Acceptance Criteria:**
- [ ] Simultaneous open works
- [ ] Direct connection attempts first
- [ ] LAN addresses tried
- [ ] Sequential port prediction works
- [ ] Timeout after 5 seconds
- [ ] Success rate >90% on cone NATs

---

**5.4.3: Birthday Attack (Symmetric NAT)** (13 SP)

```rust
// wraith-discovery/src/nat/birthday_attack.rs

use std::net::{SocketAddr, Ipv4Addr};
use tokio::net::UdpSocket;
use std::time::Duration;

/// Birthday attack for symmetric NAT traversal
/// Tries to predict allocated port by opening many sockets
pub struct BirthdayAttacker {
    sockets: Vec<UdpSocket>,
}

impl BirthdayAttacker {
    /// Create many sockets to "warm up" the NAT's port allocator
    pub async fn new(count: usize) -> std::io::Result<Self> {
        let mut sockets = Vec::new();

        for _ in 0..count {
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            sockets.push(socket);
        }

        Ok(Self { sockets })
    }

    /// Perform birthday attack to traverse symmetric NAT
    pub async fn attack(
        &self,
        peer_addr: SocketAddr,
        estimated_port_range: (u16, u16),
    ) -> Result<SocketAddr, AttackError> {
        let (start_port, end_port) = estimated_port_range;

        // Send probes to all possible ports in parallel
        let mut tasks = Vec::new();

        for port in start_port..=end_port {
            let try_addr = SocketAddr::new(peer_addr.ip(), port);

            for socket in &self.sockets {
                let socket_ref = socket;
                let try_addr_clone = try_addr;

                let task = async move {
                    let probe = b"WRAITH_BIRTHDAY";
                    let _ = socket_ref.send_to(probe, try_addr_clone).await;

                    // Try to receive response
                    let mut buf = [0u8; 1024];
                    match tokio::time::timeout(
                        Duration::from_millis(100),
                        socket_ref.recv_from(&mut buf)
                    ).await {
                        Ok(Ok((len, from))) if from.ip() == peer_addr.ip() => {
                            Some(from)
                        }
                        _ => None,
                    }
                };

                tasks.push(task);
            }
        }

        // Wait for first success
        let results = futures::future::join_all(tasks).await;

        for result in results {
            if let Some(addr) = result {
                return Ok(addr);
            }
        }

        Err(AttackError::NoConnection)
    }

    /// Estimate port range based on observed allocations
    pub fn estimate_port_range(observations: &[u16]) -> (u16, u16) {
        if observations.is_empty() {
            return (49152, 65535); // Ephemeral port range
        }

        let min = *observations.iter().min().unwrap();
        let max = *observations.iter().max().unwrap();

        // Expand range by 20%
        let range = max - min;
        let expansion = (range as f32 * 0.2) as u16;

        (
            min.saturating_sub(expansion),
            max.saturating_add(expansion),
        )
    }
}

#[derive(Debug)]
pub enum AttackError {
    Io(std::io::Error),
    NoConnection,
}

impl From<std::io::Error> for AttackError {
    fn from(err: std::io::Error) -> Self {
        AttackError::Io(err)
    }
}
```

**Acceptance Criteria:**
- [ ] Birthday attack attempts many ports
- [ ] Port range estimation works
- [ ] Success on symmetric NAT >50%
- [ ] Resource usage reasonable (<1000 sockets)
- [ ] Timeout after reasonable duration

---

## Definition of Done (Phase 5)

### Code Quality
- [ ] All code passes `cargo clippy`
- [ ] Code formatted with `rustfmt`
- [ ] Public APIs documented
- [ ] Test coverage >75%

### Functionality
- [ ] DHT node discovery works
- [ ] Peer announcements encrypted
- [ ] Relay connectivity functional
- [ ] NAT type detection accurate
- [ ] Hole punching succeeds >90% (cone NATs)

### Performance
- [ ] DHT lookup <500ms
- [ ] Relay connection <200ms
- [ ] NAT traversal <5 seconds
- [ ] DHT scales to 100K+ nodes

### Privacy
- [ ] Peer IDs not exposed in cleartext
- [ ] Announcements encrypted
- [ ] No metadata leakage in DHT

### Testing
- [ ] Unit tests for DHT operations
- [ ] Integration tests for relay
- [ ] NAT traversal success rate measured
- [ ] Stress testing (1000+ peers)

### Documentation
- [ ] DHT protocol documented
- [ ] Relay setup guide
- [ ] NAT traversal strategies explained
- [ ] Performance characteristics documented

---

## Risk Mitigation

### NAT Traversal Success Rate
**Risk**: Cannot achieve >90% success rate
**Mitigation**: Relay fallback mandatory, document known limitations

### DHT Scalability
**Risk**: DHT doesn't scale to 100K nodes
**Mitigation**: Profile early, optimize routing table, limit bucket size

### Privacy Leakage
**Risk**: Peer identities exposed in DHT
**Mitigation**: Encrypt all announcements, use ephemeral IDs

---

## Sprint 5.5: Integration & Testing (13 SP) ✅ COMPLETE

### 5.5.1: Discovery Manager Implementation (5 SP) ✅

**Deliverables:**
- Unified `DiscoveryManager` that orchestrates DHT, NAT, and Relay
- Configuration system (`DiscoveryConfig`, `RelayInfo`)
- State management (`DiscoveryState`)
- Connection type tracking (`ConnectionType`, `PeerConnection`)
- DHT bootstrap integration
- Relay server connection and registration
- NAT type detection integration

**Implementation:** `wraith-discovery/src/manager.rs`

### 5.5.2: End-to-End Connection Flow (4 SP) ✅

**Deliverables:**
- `connect_to_peer()` method implementing full connection flow:
  1. DHT lookup for peer discovery
  2. ICE candidate gathering
  3. Direct connection attempt
  4. Hole punching with timeout
  5. Relay fallback
- Helper methods for each connection strategy
- Timeout and error handling
- Connection type differentiation

### 5.5.3: Integration Tests (4 SP) ✅

**Test Coverage:** 15 integration tests
- Discovery manager lifecycle (creation, shutdown)
- Configuration with bootstrap nodes, STUN servers, relay servers
- NAT detection enable/disable
- Relay enable/disable
- Connection type variants
- Peer discovery flow
- Error handling
- Concurrent peer discovery
- State transitions

**Test File:** `wraith-discovery/tests/discovery_integration.rs`

**Sprint 5.5 Status:** ✅ **COMPLETE** (13/13 SP, 100%)

---

## Phase 5 Completion Checklist

- [x] Sprint 5.1: Kademlia DHT foundation (26 SP)
- [x] Sprint 5.2: DHT protocol (FIND_NODE, STORE, FIND_VALUE) (26 SP)
- [x] Sprint 5.3: DERP relay (server + client) (26 SP)
- [x] Sprint 5.4: NAT traversal (detection, hole punching, birthday attack) (34 SP)
- [x] Sprint 5.5: Integration & Testing (13 SP)
- [x] All quality gates passing (clippy, fmt, tests)
- [x] Integration tests complete (15 tests)
- [x] Documentation updated

**Phase 5 Status:** ✅ **COMPLETE** (123/123 SP, 100%)
**Completion Date:** 2025-11-30
