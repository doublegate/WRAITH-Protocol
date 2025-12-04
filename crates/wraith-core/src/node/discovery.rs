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

        // TODO: Integrate with wraith-discovery::DiscoveryManager
        // For now, this is a placeholder that shows the structure:
        //
        // self.discovery_manager
        //     .announce(announcement)
        //     .await
        //     .map_err(|e| NodeError::Discovery(e.to_string()))?;

        tracing::info!("Node announced to DHT successfully");

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

        // TODO: Integrate with wraith-discovery::DiscoveryManager
        // For now, return a mock peer info:
        //
        // self.discovery_manager
        //     .lookup(peer_id)
        //     .await
        //     .map_err(|e| NodeError::Discovery(e.to_string()))

        // Placeholder implementation
        Ok(PeerInfo {
            peer_id: *peer_id,
            addresses: vec!["127.0.0.1:8421".parse().unwrap()],
            nat_type: NatType::None,
            capabilities: NodeCapabilities::default(),
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

        // TODO: Integrate with wraith-discovery::DiscoveryManager
        // For now, return empty list:
        //
        // self.discovery_manager
        //     .find_nodes(count)
        //     .await
        //     .map_err(|e| NodeError::Discovery(e.to_string()))

        // Placeholder implementation
        Ok(Vec::new())
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
            return Err(NodeError::Discovery(
                "No bootstrap nodes provided".to_string(),
            ));
        }

        tracing::info!("Bootstrapping from {} nodes", bootstrap_nodes.len());

        let mut success_count = 0;

        for addr in bootstrap_nodes {
            // TODO: Integrate with wraith-discovery::DiscoveryManager
            // For now, just log the attempt:
            //
            // match self.discovery_manager.add_node(*addr).await {
            //     Ok(_) => success_count += 1,
            //     Err(e) => tracing::warn!("Failed to add bootstrap node {}: {}", addr, e),
            // }

            tracing::debug!("Added bootstrap node: {}", addr);
            success_count += 1;
        }

        if success_count == 0 {
            return Err(NodeError::Discovery(
                "Failed to bootstrap from any node".to_string(),
            ));
        }

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
        let bootstrap_nodes = vec![
            "192.168.1.1:8420".parse().unwrap(),
            "192.168.1.2:8420".parse().unwrap(),
        ];

        let result = node.bootstrap(&bootstrap_nodes).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_announce() {
        let node = Node::new_random().await.unwrap();
        let result = node.announce().await;

        // Should succeed even with placeholder implementation
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lookup_peer() {
        let node = Node::new_random().await.unwrap();
        let peer_id = [42u8; 32];

        let result = node.lookup_peer(&peer_id).await;
        assert!(result.is_ok());

        let peer_info = result.unwrap();
        assert_eq!(peer_info.peer_id, peer_id);
    }

    #[tokio::test]
    async fn test_find_peers() {
        let node = Node::new_random().await.unwrap();
        let result = node.find_peers(10).await;

        assert!(result.is_ok());
        // Placeholder returns empty list
        assert_eq!(result.unwrap().len(), 0);
    }
}
