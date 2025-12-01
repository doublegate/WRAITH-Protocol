//! Kademlia DHT Implementation
//!
//! This module provides a privacy-enhanced Kademlia DHT for peer discovery
//! in the WRAITH protocol. Key features include:
//!
//! - 256-bit node identifiers derived from public keys using BLAKE3
//! - XOR distance metric for efficient routing
//! - K-bucket routing table with LRU eviction (k=20)
//! - Encrypted DHT messages using wraith-crypto AEAD
//! - Iterative lookup with alpha parallelism (α=3)
//! - Bootstrap mechanism for network join
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use wraith_discovery::dht::{DhtNode, NodeId, BootstrapConfig};
//! use std::time::Duration;
//!
//! // Create a DHT node
//! let id = NodeId::random();
//! let addr = "127.0.0.1:8000".parse().unwrap();
//! let mut node = DhtNode::new(id, addr);
//!
//! // Store a value
//! let key = [42u8; 32];
//! let value = vec![1, 2, 3];
//! node.store(key, value, Duration::from_secs(3600));
//!
//! // Retrieve a value
//! if let Some(data) = node.get(&key) {
//!     println!("Found value: {:?}", data);
//! }
//! ```

// Module declarations
pub mod bootstrap;
pub mod messages;
pub mod node;
pub mod node_id;
pub mod operations;
pub mod routing;

// Re-exports for convenience
pub use bootstrap::{Bootstrap, BootstrapConfig, BootstrapError, BootstrapNode};
pub use messages::{
    CompactPeer, DhtMessage, FindNodeRequest, FindValueRequest, FoundNodesResponse,
    FoundValueResponse, MessageError, PingRequest, PongResponse, StoreAckResponse, StoreRequest,
};
pub use node::{DhtNode, NodeState, StoredValue};
pub use node_id::NodeId;
pub use operations::{ALPHA, DhtOperations, OperationError};
pub use routing::{DhtError, DhtPeer, K, KBucket, NUM_BUCKETS, RoutingTable};

/// DHT key derivation for announcements
///
/// Derives a 160-bit (20-byte) announcement key from group secret and file hash
/// using BLAKE3 hashing with domain separation.
///
/// This function is used to generate privacy-enhanced DHT keys for announcing
/// file availability without revealing file contents or group membership.
///
/// # Arguments
///
/// * `group_secret` - Shared secret known to group members
/// * `file_hash` - Hash of the file being announced
///
/// # Returns
///
/// A 20-byte announcement key suitable for DHT storage
///
/// # Security
///
/// The announcement key is derived using BLAKE3 with domain separation
/// to prevent key reuse attacks. The key reveals nothing about the
/// file contents or group membership to observers.
///
/// # Examples
///
/// ```
/// use wraith_discovery::dht::derive_announce_key;
///
/// let group_secret = b"shared-secret";
/// let file_hash = b"file-hash-value";
///
/// let announce_key = derive_announce_key(group_secret, file_hash);
/// assert_eq!(announce_key.len(), 20);
/// ```
#[must_use]
pub fn derive_announce_key(group_secret: &[u8], file_hash: &[u8]) -> [u8; 20] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(group_secret);
    hasher.update(file_hash);
    hasher.update(b"wraith-dht-announce"); // Domain separation

    let hash = hasher.finalize();
    let mut key = [0u8; 20];
    key.copy_from_slice(&hash.as_bytes()[..20]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_announce_key() {
        let group_secret = b"test-group-secret";
        let file_hash = b"test-file-hash";

        let key1 = derive_announce_key(group_secret, file_hash);
        let key2 = derive_announce_key(group_secret, file_hash);

        // Same inputs produce same key
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 20);
    }

    #[test]
    fn test_derive_announce_key_different_inputs() {
        let group_secret = b"test-group-secret";
        let file_hash1 = b"file-hash-1";
        let file_hash2 = b"file-hash-2";

        let key1 = derive_announce_key(group_secret, file_hash1);
        let key2 = derive_announce_key(group_secret, file_hash2);

        // Different inputs produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_module_exports() {
        // Test that all re-exports are available
        let _id = NodeId::random();
        let _config = BootstrapConfig::new();
        let _bootstrap = Bootstrap::with_defaults();

        // Test constants
        assert_eq!(K, 20);
        assert_eq!(NUM_BUCKETS, 256);
        assert_eq!(ALPHA, 3);
    }
}
