//! Network Monitor Module
//!
//! Collects real-time network topology data including connected peers,
//! link metrics, and DHT statistics for visualization.

use crate::error::{MeshError, MeshResult};
use crate::state::AppState;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

/// Peer type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerType {
    /// The local node (self)
    #[serde(rename = "self")]
    SelfNode,
    /// Directly connected peer
    Direct,
    /// Relay server
    Relay,
    /// Indirectly known peer (DHT)
    Indirect,
}

impl std::fmt::Display for PeerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerType::SelfNode => write!(f, "self"),
            PeerType::Direct => write!(f, "direct"),
            PeerType::Relay => write!(f, "relay"),
            PeerType::Indirect => write!(f, "indirect"),
        }
    }
}

/// Information about a peer in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier
    pub id: String,
    /// Display label for the peer
    pub label: String,
    /// Peer classification
    pub peer_type: PeerType,
    /// Unix timestamp when connected
    pub connected_at: i64,
    /// Unix timestamp of last activity
    pub last_seen: i64,
    /// Geographic location hint (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Information about a link between two peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    /// Source peer ID
    pub source: String,
    /// Target peer ID
    pub target: String,
    /// Round-trip latency in milliseconds
    pub latency_ms: u64,
    /// Measured bandwidth in Mbps
    pub bandwidth_mbps: f64,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss: f64,
    /// Connection strength indicator (0.0 - 1.0)
    pub strength: f64,
}

/// DHT statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtStats {
    /// Total known nodes in DHT
    pub total_nodes: usize,
    /// Size of local routing table
    pub routing_table_size: usize,
    /// Number of locally stored keys
    pub stored_keys: usize,
    /// Lookup count in last hour
    pub lookup_count_1h: u64,
    /// Average lookup latency in milliseconds
    pub avg_lookup_latency_ms: f64,
}

/// Complete network snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    /// Unix timestamp of this snapshot
    pub timestamp: i64,
    /// All known nodes
    pub nodes: Vec<PeerInfo>,
    /// All known links
    pub links: Vec<LinkInfo>,
    /// DHT statistics
    pub dht_stats: DhtStats,
    /// Overall network health (0.0 - 1.0)
    pub health_score: f64,
}

/// Metrics history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsEntry {
    /// Unix timestamp
    pub timestamp: i64,
    /// Connected peer count
    pub peer_count: usize,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Total bandwidth
    pub total_bandwidth_mbps: f64,
    /// Packet loss rate
    pub packet_loss_rate: f64,
}

/// Network monitor for collecting topology data
pub struct NetworkMonitor {
    /// Application state
    state: Arc<AppState>,
    /// Current network snapshot
    snapshot: Arc<RwLock<NetworkSnapshot>>,
    /// Metrics history (rolling window)
    metrics_history: Arc<RwLock<Vec<MetricsEntry>>>,
    /// Simulated peers for demonstration
    simulated_peers: Arc<RwLock<HashMap<String, SimulatedPeer>>>,
    /// Last update time
    last_update: Arc<RwLock<Instant>>,
}

/// Simulated peer for demonstration purposes
#[derive(Debug, Clone)]
struct SimulatedPeer {
    id: String,
    label: String,
    peer_type: PeerType,
    base_latency: u64,
    base_bandwidth: f64,
    jitter: f64,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new(state: Arc<AppState>) -> Self {
        let empty_snapshot = NetworkSnapshot {
            timestamp: chrono::Utc::now().timestamp(),
            nodes: Vec::new(),
            links: Vec::new(),
            dht_stats: DhtStats {
                total_nodes: 0,
                routing_table_size: 0,
                stored_keys: 0,
                lookup_count_1h: 0,
                avg_lookup_latency_ms: 0.0,
            },
            health_score: 1.0,
        };

        Self {
            state,
            snapshot: Arc::new(RwLock::new(empty_snapshot)),
            metrics_history: Arc::new(RwLock::new(Vec::with_capacity(3600))),
            simulated_peers: Arc::new(RwLock::new(HashMap::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Initialize simulated network for demonstration
    pub fn initialize_demo_network(&self) -> MeshResult<()> {
        let local_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| MeshError::NotInitialized("Local peer ID not set".to_string()))?;

        let mut peers = self.simulated_peers.write();

        // Add some simulated direct peers
        for i in 0..5 {
            let id = format!("{:032x}", rand_u128());
            peers.insert(
                id.clone(),
                SimulatedPeer {
                    id: id.clone(),
                    label: format!("Direct-{}", i + 1),
                    peer_type: PeerType::Direct,
                    base_latency: 20 + (i as u64 * 15),
                    base_bandwidth: 50.0 + (i as f64 * 20.0),
                    jitter: 5.0 + (i as f64 * 2.0),
                },
            );
        }

        // Add relay servers
        for i in 0..2 {
            let id = format!("{:032x}", rand_u128());
            peers.insert(
                id.clone(),
                SimulatedPeer {
                    id: id.clone(),
                    label: format!("Relay-{}", i + 1),
                    peer_type: PeerType::Relay,
                    base_latency: 50 + (i as u64 * 30),
                    base_bandwidth: 100.0 + (i as f64 * 50.0),
                    jitter: 10.0,
                },
            );
        }

        // Add indirect DHT peers
        for i in 0..8 {
            let id = format!("{:032x}", rand_u128());
            peers.insert(
                id.clone(),
                SimulatedPeer {
                    id: id.clone(),
                    label: format!("DHT-{}", &id[..8]),
                    peer_type: PeerType::Indirect,
                    base_latency: 100 + (i as u64 * 25),
                    base_bandwidth: 20.0 + (i as f64 * 10.0),
                    jitter: 15.0,
                },
            );
        }

        info!(
            "Initialized demo network with {} peers (local: {})",
            peers.len(),
            &local_id[..8]
        );
        Ok(())
    }

    /// Update the network snapshot
    pub fn update_snapshot(&self) -> MeshResult<NetworkSnapshot> {
        let local_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| MeshError::NotInitialized("Local peer ID not set".to_string()))?;

        let now = chrono::Utc::now().timestamp();
        let peers = self.simulated_peers.read();

        // Build nodes list
        let mut nodes = Vec::with_capacity(peers.len() + 1);

        // Add self node
        nodes.push(PeerInfo {
            id: local_id.clone(),
            label: "Me".to_string(),
            peer_type: PeerType::SelfNode,
            connected_at: now - 3600, // 1 hour ago
            last_seen: now,
            location: Some("Local".to_string()),
        });

        // Add all peers
        for peer in peers.values() {
            nodes.push(PeerInfo {
                id: peer.id.clone(),
                label: peer.label.clone(),
                peer_type: peer.peer_type,
                connected_at: now - rand_range(60, 3600) as i64,
                last_seen: now - rand_range(0, 30) as i64,
                location: None,
            });
        }

        // Build links list (from self to direct/relay peers)
        let mut links = Vec::new();
        let mut total_latency = 0.0;
        let mut total_bandwidth = 0.0;
        let mut total_loss = 0.0;
        let mut direct_count = 0;

        for peer in peers.values() {
            if peer.peer_type == PeerType::Direct || peer.peer_type == PeerType::Relay {
                let latency = peer.base_latency + rand_range(0, peer.jitter as u64);
                let bandwidth = peer.base_bandwidth * (0.8 + rand_f64() * 0.4);
                let packet_loss = rand_f64() * 0.02; // 0-2% loss
                let strength = 1.0 - (latency as f64 / 200.0).min(0.9);

                links.push(LinkInfo {
                    source: local_id.clone(),
                    target: peer.id.clone(),
                    latency_ms: latency,
                    bandwidth_mbps: bandwidth,
                    packet_loss,
                    strength,
                });

                total_latency += latency as f64;
                total_bandwidth += bandwidth;
                total_loss += packet_loss;
                direct_count += 1;
            }
        }

        // Add some inter-peer links for indirect nodes
        let peer_ids: Vec<_> = peers
            .values()
            .filter(|p| p.peer_type == PeerType::Direct)
            .map(|p| p.id.clone())
            .collect();

        for peer in peers.values() {
            if peer.peer_type == PeerType::Indirect && !peer_ids.is_empty() {
                // Connect to a random direct peer
                let parent_idx = rand_range(0, peer_ids.len() as u64) as usize;
                let latency = peer.base_latency + rand_range(0, peer.jitter as u64);
                let bandwidth = peer.base_bandwidth * (0.6 + rand_f64() * 0.4);

                links.push(LinkInfo {
                    source: peer_ids[parent_idx].clone(),
                    target: peer.id.clone(),
                    latency_ms: latency,
                    bandwidth_mbps: bandwidth,
                    packet_loss: rand_f64() * 0.03,
                    strength: 0.5 + rand_f64() * 0.3,
                });
            }
        }

        // Calculate DHT stats
        let indirect_count = peers
            .values()
            .filter(|p| p.peer_type == PeerType::Indirect)
            .count();

        let dht_stats = DhtStats {
            total_nodes: nodes.len() + indirect_count * 5, // Estimate total DHT
            routing_table_size: indirect_count + direct_count,
            stored_keys: rand_range(10, 100) as usize,
            lookup_count_1h: rand_range(50, 200),
            avg_lookup_latency_ms: 150.0 + rand_f64() * 100.0,
        };

        // Calculate health score
        let avg_latency = if direct_count > 0 {
            total_latency / direct_count as f64
        } else {
            0.0
        };
        let avg_loss = if direct_count > 0 {
            total_loss / direct_count as f64
        } else {
            0.0
        };

        let health_score =
            (1.0 - (avg_latency / 500.0).min(0.5) - (avg_loss * 10.0).min(0.5)).clamp(0.0, 1.0);

        let snapshot = NetworkSnapshot {
            timestamp: now,
            nodes,
            links,
            dht_stats,
            health_score,
        };

        // Update stored snapshot
        *self.snapshot.write() = snapshot.clone();
        *self.last_update.write() = Instant::now();

        // Update metrics history
        if direct_count > 0 {
            let mut history = self.metrics_history.write();
            history.push(MetricsEntry {
                timestamp: now,
                peer_count: direct_count,
                avg_latency_ms: avg_latency,
                total_bandwidth_mbps: total_bandwidth,
                packet_loss_rate: avg_loss,
            });

            // Keep only last 3600 entries (1 hour at 1Hz)
            if history.len() > 3600 {
                let drain_count = history.len() - 3600;
                history.drain(0..drain_count);
            }
        }

        debug!(
            "Updated network snapshot: {} nodes, {} links, health={:.2}",
            snapshot.nodes.len(),
            snapshot.links.len(),
            snapshot.health_score
        );

        Ok(snapshot)
    }

    /// Get the current network snapshot
    pub fn get_snapshot(&self) -> NetworkSnapshot {
        self.snapshot.read().clone()
    }

    /// Get metrics history
    pub fn get_metrics_history(&self, limit: usize) -> Vec<MetricsEntry> {
        let history = self.metrics_history.read();
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Add a simulated peer (for testing)
    pub fn add_peer(&self, peer_type: PeerType) -> MeshResult<String> {
        let id = format!("{:032x}", rand_u128());
        let label = match peer_type {
            PeerType::Direct => format!("Direct-{}", &id[..4]),
            PeerType::Relay => format!("Relay-{}", &id[..4]),
            PeerType::Indirect => format!("DHT-{}", &id[..8]),
            PeerType::SelfNode => {
                return Err(MeshError::Network("Cannot add self node".to_string()));
            }
        };

        let peer = SimulatedPeer {
            id: id.clone(),
            label,
            peer_type,
            base_latency: rand_range(20, 150),
            base_bandwidth: rand_f64() * 100.0 + 20.0,
            jitter: rand_f64() * 20.0,
        };

        self.simulated_peers.write().insert(id.clone(), peer);
        Ok(id)
    }

    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &str) -> MeshResult<()> {
        if self.simulated_peers.write().remove(peer_id).is_some() {
            Ok(())
        } else {
            Err(MeshError::PeerNotFound(peer_id.to_string()))
        }
    }
}

// Simple random number generation for demo purposes
fn rand_u128() -> u128 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

fn rand_range(min: u64, max: u64) -> u64 {
    if min >= max {
        return min;
    }
    let range = max - min;
    min + ((rand_u128() as u64) % range)
}

fn rand_f64() -> f64 {
    (rand_u128() as f64 % 1000.0) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    fn create_test_monitor() -> NetworkMonitor {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = Arc::new(AppState::new(db, dir.path().to_path_buf()));
        state.initialize().unwrap();
        NetworkMonitor::new(state)
    }

    #[test]
    fn test_network_monitor_creation() {
        let monitor = create_test_monitor();
        let snapshot = monitor.get_snapshot();
        assert!(snapshot.nodes.is_empty());
    }

    #[test]
    fn test_demo_network_initialization() {
        let monitor = create_test_monitor();
        monitor.initialize_demo_network().unwrap();

        let snapshot = monitor.update_snapshot().unwrap();
        assert!(snapshot.nodes.len() > 1);
        assert!(!snapshot.links.is_empty());
    }

    #[test]
    fn test_add_remove_peer() {
        let monitor = create_test_monitor();

        let peer_id = monitor.add_peer(PeerType::Direct).unwrap();
        assert!(!peer_id.is_empty());

        monitor.remove_peer(&peer_id).unwrap();
        assert!(monitor.remove_peer(&peer_id).is_err());
    }

    #[test]
    fn test_metrics_history() {
        let monitor = create_test_monitor();
        monitor.initialize_demo_network().unwrap();

        // Generate some snapshots
        for _ in 0..5 {
            monitor.update_snapshot().unwrap();
        }

        let history = monitor.get_metrics_history(10);
        assert!(!history.is_empty());
    }
}
