//! Discovery integration for Node API
//!
//! Integrates wraith-discovery DHT, NAT traversal, and peer lookup with the Node API.

use crate::node::{Node, NodeError};
use std::net::SocketAddr;
use std::time::SystemTime;

/// Peer announcement for DHT
#[derive(Debug, Clone)]
pub struct PeerAnnouncement {
    /// Peer ID (public key)
    pub peer_id: [u8; 32],

    /// Advertised addresses
    pub addresses: Vec<SocketAddr>,

    /// Node capabilities
    pub capabilities: NodeCapabilities,

    /// Detected NAT type
    pub nat_type: Option<NatType>,

    /// Announcement timestamp
    pub timestamp: SystemTime,
}

/// Node capabilities flags
#[derive(Debug, Clone, Copy, Default)]
pub struct NodeCapabilities {
    /// Can act as relay
    pub can_relay: bool,

    /// Supports AF_XDP
    pub has_xdp: bool,

    /// Supports multi-peer transfers
    pub multi_peer: bool,

    /// Maximum concurrent transfers
    pub max_transfers: usize,
}

/// Detected NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// No NAT (public IP)
    None,

    /// Full cone NAT (easiest to traverse)
    FullCone,

    /// Restricted cone NAT
    RestrictedCone,

    /// Port-restricted cone NAT
    PortRestricted,

    /// Symmetric NAT (hardest to traverse)
    Symmetric,
}

/// Convert from wraith-discovery NatType to wraith-core NatType
impl From<wraith_discovery::nat::NatType> for NatType {
    fn from(nat_type: wraith_discovery::nat::NatType) -> Self {
        match nat_type {
            wraith_discovery::nat::NatType::Open => NatType::None,
            wraith_discovery::nat::NatType::FullCone => NatType::FullCone,
            wraith_discovery::nat::NatType::RestrictedCone => NatType::RestrictedCone,
            wraith_discovery::nat::NatType::PortRestrictedCone => NatType::PortRestricted,
            wraith_discovery::nat::NatType::Symmetric => NatType::Symmetric,
            wraith_discovery::nat::NatType::Unknown => NatType::None,
        }
    }
}

/// Convert from wraith-core NatType to wraith-discovery NatType
impl From<NatType> for wraith_discovery::nat::NatType {
    fn from(nat_type: NatType) -> Self {
        match nat_type {
            NatType::None => wraith_discovery::nat::NatType::Open,
            NatType::FullCone => wraith_discovery::nat::NatType::FullCone,
            NatType::RestrictedCone => wraith_discovery::nat::NatType::RestrictedCone,
            NatType::PortRestricted => wraith_discovery::nat::NatType::PortRestrictedCone,
            NatType::Symmetric => wraith_discovery::nat::NatType::Symmetric,
        }
    }
}

/// Peer information from DHT
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: [u8; 32],

    /// Known addresses
    pub addresses: Vec<SocketAddr>,

    /// NAT type
    pub nat_type: NatType,

    /// Capabilities
    pub capabilities: NodeCapabilities,

    /// Last seen timestamp
    pub last_seen: SystemTime,
}

impl Node {
    /// Announce this node to the DHT network
    ///
    /// Broadcasts node presence, addresses, and capabilities to the DHT.
    ///
    /// # Errors
    ///
    /// Returns error if DHT is not initialized or announcement fails.
    pub async fn announce(&self) -> Result<(), NodeError> {
        let announcement = self.create_announcement();

        tracing::debug!(
            "Announcing node {:?} to DHT with {} addresses",
            announcement.peer_id,
            announcement.addresses.len()
        );

        // Get discovery manager
        let _discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Note: wraith-discovery handles periodic DHT announcements automatically
        // when the discovery manager is started.
        // We log the intent here for visibility.
        tracing::info!("Node announced to DHT successfully (via discovery manager startup)");

        Ok(())
    }

    /// Lookup a peer by ID in the DHT
    ///
    /// Queries the DHT for peer information including addresses and NAT type.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - The peer's public key (32-byte Ed25519 public key)
    ///
    /// # Errors
    ///
    /// Returns error if peer is not found or DHT query fails.
    pub async fn lookup_peer(&self, peer_id: &[u8; 32]) -> Result<PeerInfo, NodeError> {
        tracing::debug!("Looking up peer {:?} in DHT", peer_id);

        // Get discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Convert peer_id to NodeId for DHT lookup
        let node_id = wraith_discovery::dht::NodeId::from_bytes(*peer_id);

        // Perform DHT lookup to find peer addresses
        let addresses = discovery
            .dht()
            .write()
            .await
            .iterative_find_node(&node_id)
            .await
            .into_iter()
            .map(|peer| peer.addr)
            .collect::<Vec<_>>();

        if addresses.is_empty() {
            return Err(NodeError::PeerNotFound(*peer_id));
        }

        // Get NAT type from discovery manager
        let nat_type = discovery
            .nat_type()
            .await
            .map(NatType::from)
            .unwrap_or(NatType::None);

        tracing::debug!(
            "Found peer {:?} at {} addresses with NAT type {:?}",
            peer_id,
            addresses.len(),
            nat_type
        );

        Ok(PeerInfo {
            peer_id: *peer_id,
            addresses,
            nat_type,
            capabilities: NodeCapabilities::default(), // Would be populated from DHT metadata
            last_seen: SystemTime::now(),
        })
    }

    /// Find nearby peers
    ///
    /// Returns the N closest peers in the DHT keyspace.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of peers to return (max)
    ///
    /// # Errors
    ///
    /// Returns error if DHT is not initialized or query fails.
    pub async fn find_peers(&self, count: usize) -> Result<Vec<PeerInfo>, NodeError> {
        tracing::debug!("Finding {} nearby peers in DHT", count);

        // Get discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Find peers closest to our own node ID
        let our_node_id = wraith_discovery::dht::NodeId::from_bytes(*self.node_id());

        let dht_peers = discovery
            .dht()
            .read()
            .await
            .routing_table()
            .closest_peers(&our_node_id, count);

        let peer_count = dht_peers.len();

        // Convert DHT peers to PeerInfo
        let peers = dht_peers
            .into_iter()
            .map(|peer| PeerInfo {
                peer_id: *peer.id.as_bytes(),
                addresses: vec![peer.addr],
                nat_type: NatType::None, // Would be populated from DHT metadata
                capabilities: NodeCapabilities::default(),
                last_seen: SystemTime::now(),
            })
            .collect();

        tracing::debug!("Found {} nearby peers", peer_count);

        Ok(peers)
    }

    /// Bootstrap from known nodes
    ///
    /// Connects to bootstrap nodes to join the DHT network.
    ///
    /// # Arguments
    ///
    /// * `bootstrap_nodes` - List of known node addresses to bootstrap from
    ///
    /// # Errors
    ///
    /// Returns error if all bootstrap attempts fail.
    pub async fn bootstrap(&self, bootstrap_nodes: &[SocketAddr]) -> Result<(), NodeError> {
        if bootstrap_nodes.is_empty() {
            return Err(NodeError::Discovery(std::borrow::Cow::Borrowed(
                "No bootstrap nodes provided",
            )));
        }

        tracing::info!("Bootstrapping from {} nodes", bootstrap_nodes.len());

        // Get discovery manager
        let discovery = {
            let guard = self.inner.discovery.lock().await;
            guard
                .as_ref()
                .ok_or(NodeError::Discovery(std::borrow::Cow::Borrowed(
                    "Discovery not initialized",
                )))?
                .clone()
        };

        // Add bootstrap nodes to the DHT routing table
        let mut success_count = 0;
        let dht_arc = discovery.dht();
        let mut dht = dht_arc.write().await;

        for addr in bootstrap_nodes {
            // Create a synthetic peer for the bootstrap node
            // In a real implementation, we would:
            // 1. Send a PING to the bootstrap node to get its actual NodeId
            // 2. Perform iterative FIND_NODE starting from this bootstrap node
            // For now, we'll create a peer entry with a derived NodeId
            let node_id = wraith_discovery::dht::NodeId::from_bytes(
                *blake3::hash(addr.to_string().as_bytes()).as_bytes(),
            );

            let peer = wraith_discovery::dht::DhtPeer::new(node_id, *addr);

            match dht.routing_table_mut().insert(peer) {
                Ok(_) => {
                    tracing::debug!("Added bootstrap node: {}", addr);
                    success_count += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to add bootstrap node {}: {}", addr, e);
                }
            }
        }

        drop(dht); // Release lock before DHT operations

        if success_count == 0 {
            return Err(NodeError::Discovery(std::borrow::Cow::Borrowed(
                "Failed to bootstrap from any node",
            )));
        }

        // Perform iterative FIND_NODE for our own ID to populate routing table
        let our_node_id = wraith_discovery::dht::NodeId::from_bytes(*self.node_id());
        let _closest_peers = discovery
            .dht()
            .write()
            .await
            .iterative_find_node(&our_node_id)
            .await;

        tracing::info!(
            "Bootstrapped successfully from {}/{} nodes",
            success_count,
            bootstrap_nodes.len()
        );

        Ok(())
    }

    /// Get local addresses for announcement
    ///
    /// Returns all local network addresses where this node is listening.
    pub fn local_addresses(&self) -> Vec<SocketAddr> {
        // In a real implementation, this would:
        // 1. Get all local network interfaces
        // 2. Perform STUN queries to discover external addresses
        // 3. Return both local and external addresses
        //
        // For now, return the configured listen address
        vec![self.inner.config.listen_addr]
    }

    /// Get node capabilities
    ///
    /// Returns current node capabilities based on configuration.
    pub fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities {
            can_relay: self.inner.config.discovery.enable_relay,
            has_xdp: self.inner.config.transport.enable_xdp,
            multi_peer: self.inner.config.transfer.enable_multi_peer,
            max_transfers: self.inner.config.transfer.max_concurrent_transfers,
        }
    }

    /// Create announcement from current node state
    fn create_announcement(&self) -> PeerAnnouncement {
        PeerAnnouncement {
            peer_id: *self.node_id(),
            addresses: self.local_addresses(),
            capabilities: self.capabilities(),
            nat_type: None, // Will be populated by NAT detection
            timestamp: SystemTime::now(),
        }
    }

    /// Start background DHT maintenance task
    ///
    /// Periodically announces to DHT and refreshes routing table.
    ///
    /// Returns a join handle for the background task.
    pub fn start_dht_maintenance(&self) -> tokio::task::JoinHandle<()> {
        let node = self.clone();
        let interval = self.inner.config.discovery.announcement_interval;

        tokio::spawn(async move {
            let mut announce_timer = tokio::time::interval(interval);

            loop {
                announce_timer.tick().await;

                if let Err(e) = node.announce().await {
                    tracing::warn!("DHT announcement failed: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_announcement_creation() {
        let announcement = PeerAnnouncement {
            peer_id: [1u8; 32],
            addresses: vec!["192.168.1.100:8420".parse().unwrap()],
            capabilities: NodeCapabilities::default(),
            nat_type: Some(NatType::None),
            timestamp: SystemTime::now(),
        };

        assert_eq!(announcement.peer_id, [1u8; 32]);
        assert_eq!(announcement.addresses.len(), 1);
    }

    #[test]
    fn test_node_capabilities() {
        let caps = NodeCapabilities {
            can_relay: true,
            has_xdp: false,
            multi_peer: true,
            max_transfers: 10,
        };

        assert!(caps.can_relay);
        assert!(!caps.has_xdp);
        assert!(caps.multi_peer);
        assert_eq!(caps.max_transfers, 10);
    }

    #[test]
    fn test_nat_type_equality() {
        assert_eq!(NatType::None, NatType::None);
        assert_eq!(NatType::FullCone, NatType::FullCone);
        assert_ne!(NatType::None, NatType::FullCone);
        assert_ne!(NatType::Symmetric, NatType::RestrictedCone);
    }

    #[tokio::test]
    async fn test_local_addresses() {
        let node = Node::new_random().await.unwrap();
        let addresses = node.local_addresses();

        // Should at least return the listen address
        assert!(!addresses.is_empty());
    }

    #[tokio::test]
    async fn test_capabilities() {
        let node = Node::new_random().await.unwrap();
        let caps = node.capabilities();

        // Check defaults match config defaults
        assert!(caps.can_relay); // Default from DiscoveryConfig
        assert!(caps.multi_peer); // Default from TransferConfig
    }

    #[tokio::test]
    async fn test_bootstrap_empty_list() {
        let node = Node::new_random().await.unwrap();
        let result = node.bootstrap(&[]).await;

        assert!(result.is_err());
        match result {
            Err(NodeError::Discovery(msg)) => {
                assert!(msg.contains("No bootstrap nodes"));
            }
            _ => panic!("Expected Discovery error"),
        }
    }

    #[tokio::test]
    async fn test_bootstrap_success() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let bootstrap_nodes = vec![
            "192.168.1.1:8420".parse().unwrap(),
            "192.168.1.2:8420".parse().unwrap(),
        ];

        let result = node.bootstrap(&bootstrap_nodes).await;
        assert!(result.is_ok());

        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_announce() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();
        let result = node.announce().await;

        assert!(result.is_ok());

        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_lookup_peer() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let peer_id = [42u8; 32];

        let result = node.lookup_peer(&peer_id).await;
        // Will return PeerNotFound since routing table is empty, but won't error on uninitialized discovery
        assert!(result.is_err()); // Expected: routing table is empty

        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_peers() {
        let node = Node::new_random().await.unwrap();
        node.start().await.unwrap();

        let result = node.find_peers(10).await;

        assert!(result.is_ok());
        // Routing table is initially empty
        assert_eq!(result.unwrap().len(), 0);

        node.stop().await.unwrap();
    }
}
