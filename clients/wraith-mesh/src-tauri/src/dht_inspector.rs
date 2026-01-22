//! DHT Inspector Module
//!
//! Provides inspection capabilities for the Kademlia DHT including
//! routing table visualization, key lookups, and path tracing.

use crate::error::{MeshError, MeshResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// A k-bucket in the routing table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingBucket {
    /// Bucket index (0-255)
    pub index: usize,
    /// XOR distance prefix for this bucket
    pub distance_prefix: String,
    /// Peers in this bucket
    pub peers: Vec<BucketPeer>,
    /// Maximum capacity (k value)
    pub capacity: usize,
}

/// A peer in a k-bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketPeer {
    /// Peer ID
    pub id: String,
    /// Network address
    pub address: String,
    /// Last seen timestamp
    pub last_seen: i64,
    /// Round-trip time in milliseconds
    pub rtt_ms: Option<u64>,
    /// Whether peer is considered alive
    pub is_alive: bool,
}

/// Result of a DHT key lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    /// The key that was looked up
    pub key: String,
    /// Whether the lookup was successful
    pub found: bool,
    /// The value if found
    pub value: Option<Vec<u8>>,
    /// Peer that holds the value
    pub holder: Option<String>,
    /// Number of hops taken
    pub hops: usize,
    /// Total lookup duration in milliseconds
    pub duration_ms: u64,
}

/// A hop in a DHT lookup trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupHop {
    /// Hop number (0-indexed)
    pub hop: usize,
    /// Peer ID contacted
    pub peer_id: String,
    /// Peer label
    pub peer_label: String,
    /// XOR distance to target
    pub distance: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Peers returned by this hop
    pub returned_peers: Vec<String>,
    /// Whether this was the final hop
    pub is_final: bool,
}

/// A stored key in the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredKey {
    /// The key (hash)
    pub key: String,
    /// Size of the value in bytes
    pub value_size: usize,
    /// When the key was stored
    pub stored_at: i64,
    /// Time-to-live in seconds
    pub ttl_seconds: i64,
    /// Remaining TTL in seconds
    pub remaining_ttl: i64,
}

/// DHT Inspector for examining routing table and stored data
pub struct DhtInspector {
    /// Application state
    state: Arc<AppState>,
}

impl DhtInspector {
    /// Create a new DHT inspector
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Get the routing table as a list of k-buckets
    pub fn get_routing_table(&self) -> MeshResult<Vec<RoutingBucket>> {
        let _local_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| MeshError::NotInitialized("Local peer ID not set".to_string()))?;

        let mut buckets = Vec::new();

        // Generate simulated routing table (for demonstration)
        // In a real implementation, this would query the actual DHT
        for i in 0..256 {
            let peer_count = if i < 10 {
                rand_range(0, 3) as usize
            } else if i < 50 {
                rand_range(0, 2) as usize
            } else {
                0
            };

            if peer_count > 0 {
                let mut peers = Vec::with_capacity(peer_count);
                for j in 0..peer_count {
                    let peer_id = format!("{:032x}", rand_u128());
                    peers.push(BucketPeer {
                        id: peer_id.clone(),
                        address: format!(
                            "192.168.{}.{}:{}",
                            i % 256,
                            j + 1,
                            8000 + rand_range(0, 1000)
                        ),
                        last_seen: chrono::Utc::now().timestamp() - rand_range(0, 900) as i64,
                        rtt_ms: Some(rand_range(10, 200)),
                        is_alive: rand_range(0, 10) > 1,
                    });
                }

                buckets.push(RoutingBucket {
                    index: i,
                    distance_prefix: format!("2^{}", 255 - i),
                    peers,
                    capacity: 20,
                });
            }
        }

        debug!("Retrieved {} non-empty routing buckets", buckets.len());
        Ok(buckets)
    }

    /// Look up a key in the DHT
    pub fn lookup_key(&self, key: &str) -> MeshResult<LookupResult> {
        let start = std::time::Instant::now();

        // Simulate a lookup (for demonstration)
        let found = rand_range(0, 10) > 3;
        let hops = rand_range(2, 6) as usize;

        let result = LookupResult {
            key: key.to_string(),
            found,
            value: if found {
                Some(vec![0u8; rand_range(10, 1000) as usize])
            } else {
                None
            },
            holder: if found {
                Some(format!("{:032x}", rand_u128()))
            } else {
                None
            },
            hops,
            duration_ms: start.elapsed().as_millis() as u64 + rand_range(50, 300),
        };

        debug!(
            "DHT lookup for key {}: found={}, hops={}, duration={}ms",
            &key[..8.min(key.len())],
            result.found,
            result.hops,
            result.duration_ms
        );

        Ok(result)
    }

    /// Trace a DHT lookup path
    pub fn trace_lookup(&self, key: &str) -> MeshResult<Vec<LookupHop>> {
        let mut hops = Vec::new();
        let num_hops = rand_range(3, 7) as usize;

        for i in 0..num_hops {
            let peer_id = format!("{:032x}", rand_u128());
            let returned_count = if i < num_hops - 1 {
                rand_range(1, 4) as usize
            } else {
                0
            };

            let mut returned_peers = Vec::with_capacity(returned_count);
            for _ in 0..returned_count {
                returned_peers.push(format!("{:016x}", rand_u128()));
            }

            hops.push(LookupHop {
                hop: i,
                peer_id: peer_id.clone(),
                peer_label: format!("Peer-{}", &peer_id[..8]),
                distance: format!("2^{}", 255 - (i * 20).min(255)),
                response_time_ms: rand_range(10, 100),
                returned_peers,
                is_final: i == num_hops - 1,
            });
        }

        debug!(
            "Traced DHT lookup for key {}: {} hops",
            &key[..8.min(key.len())],
            hops.len()
        );

        Ok(hops)
    }

    /// Get list of locally stored keys
    pub fn get_stored_keys(&self) -> MeshResult<Vec<StoredKey>> {
        let mut keys = Vec::new();
        let num_keys = rand_range(5, 20) as usize;
        let now = chrono::Utc::now().timestamp();

        for _ in 0..num_keys {
            let ttl = rand_range(300, 7200) as i64;
            let stored_ago = rand_range(0, ttl as u64 / 2) as i64;

            keys.push(StoredKey {
                key: format!("{:064x}", rand_u128()),
                value_size: rand_range(100, 10000) as usize,
                stored_at: now - stored_ago,
                ttl_seconds: ttl,
                remaining_ttl: ttl - stored_ago,
            });
        }

        debug!("Retrieved {} stored keys", keys.len());
        Ok(keys)
    }

    /// Calculate XOR distance between two node IDs
    pub fn calculate_distance(&self, id1: &str, id2: &str) -> MeshResult<String> {
        if id1.len() != id2.len() {
            return Err(MeshError::Dht("IDs must have same length".to_string()));
        }

        let bytes1: Result<Vec<u8>, _> = (0..id1.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&id1[i..i + 2], 16))
            .collect();

        let bytes2: Result<Vec<u8>, _> = (0..id2.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&id2[i..i + 2], 16))
            .collect();

        let bytes1 = bytes1.map_err(|e| MeshError::Dht(format!("Invalid hex ID: {}", e)))?;
        let bytes2 = bytes2.map_err(|e| MeshError::Dht(format!("Invalid hex ID: {}", e)))?;

        let xor: Vec<u8> = bytes1
            .iter()
            .zip(bytes2.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        Ok(hex::encode(xor))
    }
}

// Simple random helpers
fn rand_u128() -> u128 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

fn rand_range(min: u64, max: u64) -> u64 {
    if min >= max {
        return min;
    }
    let range = max - min;
    min + ((rand_u128() as u64) % range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    fn create_test_inspector() -> DhtInspector {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = Arc::new(AppState::new(db, dir.path().to_path_buf()));
        state.initialize().unwrap();
        DhtInspector::new(state)
    }

    #[test]
    fn test_get_routing_table() {
        let inspector = create_test_inspector();
        let buckets = inspector.get_routing_table().unwrap();
        // Some buckets should be non-empty in simulation
        assert!(buckets.iter().any(|b| !b.peers.is_empty()));
    }

    #[test]
    fn test_lookup_key() {
        let inspector = create_test_inspector();
        let result = inspector.lookup_key("test_key_12345").unwrap();
        assert!(!result.key.is_empty());
        assert!(result.hops > 0);
    }

    #[test]
    fn test_trace_lookup() {
        let inspector = create_test_inspector();
        let hops = inspector.trace_lookup("test_key_12345").unwrap();
        assert!(!hops.is_empty());
        assert!(hops.last().unwrap().is_final);
    }

    #[test]
    fn test_get_stored_keys() {
        let inspector = create_test_inspector();
        let keys = inspector.get_stored_keys().unwrap();
        assert!(!keys.is_empty());
    }

    #[test]
    fn test_calculate_distance() {
        let inspector = create_test_inspector();
        let id1 = "abcd1234";
        let id2 = "12345678";
        let distance = inspector.calculate_distance(id1, id2).unwrap();
        assert_eq!(distance.len(), id1.len());
    }
}
