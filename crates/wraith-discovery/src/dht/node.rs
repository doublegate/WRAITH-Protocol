//! DHT Node Structure and State Management
//!
//! This module defines the main DhtNode structure which maintains:
//! - Node identity and network address
//! - Routing table for peer discovery
//! - Local key-value storage
//! - Node state tracking

use super::node_id::NodeId;
use super::routing::RoutingTable;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// DHT node state
///
/// Tracks the health state of a DHT node based on recent activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Node is responsive and in good standing
    ///
    /// A node is Good if it has responded to queries within the last 15 minutes.
    Good,

    /// Node is questionable - hasn't responded recently
    ///
    /// A node becomes Questionable if it hasn't responded within 15 minutes
    /// but is still within the 30-minute timeout window.
    Questionable,

    /// Node is considered dead
    ///
    /// A node is Bad if it hasn't responded within 30 minutes.
    /// Bad nodes should be removed from routing tables.
    Bad,
}

/// Stored value in the DHT
///
/// Stores both the data and metadata for values stored in the DHT.
#[derive(Clone, Debug)]
pub struct StoredValue {
    /// The stored data
    pub data: Vec<u8>,
    /// When this value was stored
    pub stored_at: Instant,
    /// Time-to-live for this value
    pub ttl: Duration,
}

impl StoredValue {
    /// Create a new stored value
    ///
    /// # Arguments
    ///
    /// * `data` - The data to store
    /// * `ttl` - Time-to-live for this value
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::StoredValue;
    /// use std::time::Duration;
    ///
    /// let value = StoredValue::new(vec![1, 2, 3], Duration::from_secs(3600));
    /// ```
    #[must_use]
    pub fn new(data: Vec<u8>, ttl: Duration) -> Self {
        Self {
            data,
            stored_at: Instant::now(),
            ttl,
        }
    }

    /// Check if this value has expired
    ///
    /// # Returns
    ///
    /// `true` if the value's TTL has elapsed
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::StoredValue;
    /// use std::time::Duration;
    ///
    /// let value = StoredValue::new(vec![1, 2, 3], Duration::from_secs(0));
    /// std::thread::sleep(Duration::from_millis(10));
    /// assert!(value.is_expired());
    /// ```
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.stored_at.elapsed() >= self.ttl
    }

    /// Get remaining time until expiration
    ///
    /// # Returns
    ///
    /// Duration until expiration, or zero if already expired
    #[must_use]
    pub fn remaining_ttl(&self) -> Duration {
        self.ttl.saturating_sub(self.stored_at.elapsed())
    }
}

/// DHT node
///
/// The main DHT node structure that maintains routing state, local storage,
/// and handles DHT operations.
#[derive(Debug)]
pub struct DhtNode {
    /// This node's identifier
    id: NodeId,
    /// This node's network address
    addr: SocketAddr,
    /// Routing table for peer discovery
    routing_table: RoutingTable,
    /// Local key-value storage
    storage: HashMap<[u8; 32], StoredValue>,
}

impl DhtNode {
    /// Create a new DHT node
    ///
    /// # Arguments
    ///
    /// * `id` - This node's identifier
    /// * `addr` - This node's network address
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    ///
    /// let id = NodeId::random();
    /// let addr = "127.0.0.1:8000".parse().unwrap();
    /// let node = DhtNode::new(id, addr);
    /// ```
    #[must_use]
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            routing_table: RoutingTable::new(id),
            storage: HashMap::new(),
        }
    }

    /// Get this node's identifier
    ///
    /// # Returns
    ///
    /// Reference to this node's NodeId
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    ///
    /// let id = NodeId::random();
    /// let node = DhtNode::new(id, "127.0.0.1:8000".parse().unwrap());
    /// assert_eq!(node.id(), &id);
    /// ```
    #[must_use]
    pub const fn id(&self) -> &NodeId {
        &self.id
    }

    /// Get this node's network address
    ///
    /// # Returns
    ///
    /// This node's socket address
    #[must_use]
    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get a reference to the routing table
    ///
    /// # Returns
    ///
    /// Reference to the routing table
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    ///
    /// let node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// assert_eq!(node.routing_table().peer_count(), 0);
    /// ```
    #[must_use]
    pub const fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Get a mutable reference to the routing table
    ///
    /// # Returns
    ///
    /// Mutable reference to the routing table
    #[must_use]
    pub fn routing_table_mut(&mut self) -> &mut RoutingTable {
        &mut self.routing_table
    }

    /// Store a value in the DHT
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key to store under
    /// * `value` - Value data
    /// * `ttl` - Time-to-live for this value
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    /// use std::time::Duration;
    ///
    /// let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// let key = [42u8; 32];
    /// node.store(key, vec![1, 2, 3], Duration::from_secs(3600));
    /// ```
    pub fn store(&mut self, key: [u8; 32], value: Vec<u8>, ttl: Duration) {
        let stored_value = StoredValue::new(value, ttl);
        self.storage.insert(key, stored_value);
    }

    /// Retrieve a value from local storage
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key to retrieve
    ///
    /// # Returns
    ///
    /// Cloned value data if found and not expired, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    /// use std::time::Duration;
    ///
    /// let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    /// let key = [42u8; 32];
    /// node.store(key, vec![1, 2, 3], Duration::from_secs(3600));
    ///
    /// let value = node.get(&key);
    /// assert_eq!(value, Some(vec![1, 2, 3]));
    /// ```
    #[must_use]
    pub fn get(&self, key: &[u8; 32]) -> Option<Vec<u8>> {
        self.storage.get(key).and_then(|stored| {
            if stored.is_expired() {
                None
            } else {
                Some(stored.data.clone())
            }
        })
    }

    /// Remove a value from local storage
    ///
    /// # Arguments
    ///
    /// * `key` - 32-byte key to remove
    ///
    /// # Returns
    ///
    /// The removed value if it existed
    pub fn remove(&mut self, key: &[u8; 32]) -> Option<StoredValue> {
        self.storage.remove(key)
    }

    /// Remove expired values from local storage
    ///
    /// Should be called periodically to clean up expired entries.
    /// Returns the number of values removed.
    ///
    /// # Returns
    ///
    /// Number of expired values removed
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{DhtNode, NodeId};
    /// use std::time::Duration;
    ///
    /// let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());
    ///
    /// // Store value with 0 TTL
    /// node.store([1u8; 32], vec![1, 2, 3], Duration::from_secs(0));
    /// std::thread::sleep(Duration::from_millis(10));
    ///
    /// let removed = node.prune_expired();
    /// assert_eq!(removed, 1);
    /// ```
    pub fn prune_expired(&mut self) -> usize {
        let before_count = self.storage.len();
        self.storage.retain(|_, stored| !stored.is_expired());
        before_count - self.storage.len()
    }

    /// Get the number of values in local storage
    ///
    /// # Returns
    ///
    /// Number of stored key-value pairs
    #[must_use]
    pub fn storage_count(&self) -> usize {
        self.storage.len()
    }

    /// Prune the routing table and storage
    ///
    /// Removes dead peers from the routing table and expired values
    /// from storage. Returns (peers_removed, values_removed).
    ///
    /// # Returns
    ///
    /// Tuple of (peers removed, values removed)
    pub fn prune_all(&mut self) -> (usize, usize) {
        let before_peers = self.routing_table.peer_count();
        self.routing_table.prune();
        let peers_removed = before_peers - self.routing_table.peer_count();

        let values_removed = self.prune_expired();

        (peers_removed, values_removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_state() {
        assert_eq!(NodeState::Good, NodeState::Good);
        assert_ne!(NodeState::Good, NodeState::Bad);
    }

    #[test]
    fn test_stored_value_creation() {
        let value = StoredValue::new(vec![1, 2, 3], Duration::from_secs(60));
        assert_eq!(value.data, vec![1, 2, 3]);
        assert!(!value.is_expired());
    }

    #[test]
    fn test_stored_value_expiration() {
        let value = StoredValue::new(vec![1, 2, 3], Duration::from_millis(10));
        assert!(!value.is_expired());

        std::thread::sleep(Duration::from_millis(20));
        assert!(value.is_expired());
    }

    #[test]
    fn test_stored_value_remaining_ttl() {
        let value = StoredValue::new(vec![1, 2, 3], Duration::from_secs(60));
        let remaining = value.remaining_ttl();
        assert!(remaining <= Duration::from_secs(60));
        assert!(remaining > Duration::from_secs(59));
    }

    #[test]
    fn test_dht_node_creation() {
        let id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let node = DhtNode::new(id, addr);

        assert_eq!(node.id(), &id);
        assert_eq!(node.addr(), addr);
        assert_eq!(node.routing_table().peer_count(), 0);
        assert_eq!(node.storage_count(), 0);
    }

    #[test]
    fn test_dht_node_store_get() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let key = [42u8; 32];
        let value = vec![1, 2, 3, 4, 5];

        node.store(key, value.clone(), Duration::from_secs(3600));
        assert_eq!(node.storage_count(), 1);

        let retrieved = node.get(&key);
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_dht_node_get_expired() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let key = [42u8; 32];
        let value = vec![1, 2, 3];

        node.store(key, value, Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(20));

        let retrieved = node.get(&key);
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_dht_node_remove() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        let key = [42u8; 32];
        node.store(key, vec![1, 2, 3], Duration::from_secs(3600));
        assert_eq!(node.storage_count(), 1);

        let removed = node.remove(&key);
        assert!(removed.is_some());
        assert_eq!(node.storage_count(), 0);
    }

    #[test]
    fn test_dht_node_prune_expired() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        // Store some values with different TTLs
        node.store([1u8; 32], vec![1], Duration::from_secs(3600));
        node.store([2u8; 32], vec![2], Duration::from_millis(10));
        node.store([3u8; 32], vec![3], Duration::from_millis(10));

        std::thread::sleep(Duration::from_millis(20));

        let removed = node.prune_expired();
        assert_eq!(removed, 2);
        assert_eq!(node.storage_count(), 1);
    }

    #[test]
    fn test_dht_node_prune_all() {
        let mut node = DhtNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap());

        // Add some storage
        node.store([1u8; 32], vec![1], Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(20));

        let (_peers_removed, values_removed) = node.prune_all();
        assert_eq!(values_removed, 1);
    }
}
