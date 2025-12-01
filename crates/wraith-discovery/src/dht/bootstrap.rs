//! DHT Bootstrap Mechanism
//!
//! This module provides bootstrap functionality for joining the DHT network.
//! Bootstrap nodes are well-known entry points that help new nodes populate
//! their routing tables.

use super::node_id::NodeId;
use super::routing::DhtPeer;
use std::net::SocketAddr;
use thiserror::Error;

/// Bootstrap node configuration
///
/// Represents a known bootstrap node that can be used to join the DHT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapNode {
    /// Node identifier
    pub id: NodeId,
    /// Network address
    pub addr: SocketAddr,
    /// Human-readable name (optional)
    pub name: Option<String>,
}

impl BootstrapNode {
    /// Create a new bootstrap node
    ///
    /// # Arguments
    ///
    /// * `id` - The node's identifier
    /// * `addr` - The node's network address
    /// * `name` - Optional human-readable name
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{BootstrapNode, NodeId};
    ///
    /// let id = NodeId::random();
    /// let addr = "127.0.0.1:8000".parse().unwrap();
    /// let node = BootstrapNode::new(id, addr, Some("bootstrap-1".to_string()));
    /// ```
    #[must_use]
    pub fn new(id: NodeId, addr: SocketAddr, name: Option<String>) -> Self {
        Self { id, addr, name }
    }

    /// Convert to a DhtPeer
    ///
    /// # Returns
    ///
    /// DhtPeer representation of this bootstrap node
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{BootstrapNode, NodeId};
    ///
    /// let id = NodeId::random();
    /// let node = BootstrapNode::new(id, "127.0.0.1:8000".parse().unwrap(), None);
    /// let peer = node.to_peer();
    /// assert_eq!(peer.id, id);
    /// ```
    #[must_use]
    pub fn to_peer(&self) -> DhtPeer {
        DhtPeer::new(self.id, self.addr)
    }
}

/// Bootstrap configuration
///
/// Maintains a list of bootstrap nodes and provides methods for
/// managing the bootstrap process.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// List of known bootstrap nodes
    nodes: Vec<BootstrapNode>,
}

impl BootstrapConfig {
    /// Create a new empty bootstrap configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::BootstrapConfig;
    ///
    /// let config = BootstrapConfig::new();
    /// assert_eq!(config.node_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Create bootstrap configuration with default nodes
    ///
    /// Returns a configuration with well-known WRAITH bootstrap nodes.
    /// In production, these would be stable, long-running nodes.
    ///
    /// Currently returns an empty configuration. In production, this would
    /// include well-known bootstrap nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::BootstrapConfig;
    ///
    /// let config = BootstrapConfig::with_defaults();
    /// // Currently no default nodes, returns empty config
    /// assert_eq!(config.node_count(), 0);
    /// ```
    #[must_use]
    pub fn with_defaults() -> Self {
        // In production, these would be real bootstrap nodes
        // For now, we provide examples that show the structure

        // Example bootstrap node format:
        // let mut config = Self::new();
        // let id = NodeId::from_public_key(&PUBLIC_KEY);
        // let addr = "bootstrap.wraith.network:8000".parse().unwrap();
        // config.add_node(BootstrapNode::new(id, addr, Some("official-1".to_string())));
        // config

        Self::new()
    }

    /// Add a bootstrap node to the configuration
    ///
    /// # Arguments
    ///
    /// * `node` - The bootstrap node to add
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{BootstrapConfig, BootstrapNode, NodeId};
    ///
    /// let mut config = BootstrapConfig::new();
    /// let node = BootstrapNode::new(
    ///     NodeId::random(),
    ///     "127.0.0.1:8000".parse().unwrap(),
    ///     None
    /// );
    /// config.add_node(node);
    /// assert_eq!(config.node_count(), 1);
    /// ```
    pub fn add_node(&mut self, node: BootstrapNode) {
        self.nodes.push(node);
    }

    /// Remove a bootstrap node by address
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the node to remove
    ///
    /// # Returns
    ///
    /// `true` if a node was removed, `false` otherwise
    pub fn remove_node(&mut self, addr: &SocketAddr) -> bool {
        let before_len = self.nodes.len();
        self.nodes.retain(|n| n.addr != *addr);
        before_len != self.nodes.len()
    }

    /// Get all bootstrap nodes
    ///
    /// # Returns
    ///
    /// Slice of all bootstrap nodes
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::BootstrapConfig;
    ///
    /// let config = BootstrapConfig::new();
    /// let nodes = config.nodes();
    /// assert_eq!(nodes.len(), 0);
    /// ```
    #[must_use]
    pub fn nodes(&self) -> &[BootstrapNode] {
        &self.nodes
    }

    /// Get bootstrap nodes as DhtPeers
    ///
    /// # Returns
    ///
    /// Vector of DhtPeers for all bootstrap nodes
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{BootstrapConfig, BootstrapNode, NodeId};
    ///
    /// let mut config = BootstrapConfig::new();
    /// config.add_node(BootstrapNode::new(
    ///     NodeId::random(),
    ///     "127.0.0.1:8000".parse().unwrap(),
    ///     None
    /// ));
    ///
    /// let peers = config.as_peers();
    /// assert_eq!(peers.len(), 1);
    /// ```
    #[must_use]
    pub fn as_peers(&self) -> Vec<DhtPeer> {
        self.nodes.iter().map(|n| n.to_peer()).collect()
    }

    /// Get the number of bootstrap nodes
    ///
    /// # Returns
    ///
    /// Number of configured bootstrap nodes
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the configuration is empty
    ///
    /// # Returns
    ///
    /// `true` if no bootstrap nodes are configured
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Bootstrap process manager
///
/// Manages the bootstrap process of joining the DHT network.
pub struct Bootstrap {
    /// Bootstrap configuration
    config: BootstrapConfig,
}

impl Bootstrap {
    /// Create a new bootstrap manager
    ///
    /// # Arguments
    ///
    /// * `config` - Bootstrap configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{Bootstrap, BootstrapConfig};
    ///
    /// let config = BootstrapConfig::new();
    /// let bootstrap = Bootstrap::new(config);
    /// ```
    #[must_use]
    pub fn new(config: BootstrapConfig) -> Self {
        Self { config }
    }

    /// Create bootstrap manager with default configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::Bootstrap;
    ///
    /// let bootstrap = Bootstrap::with_defaults();
    /// ```
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BootstrapConfig::with_defaults())
    }

    /// Get the bootstrap configuration
    ///
    /// # Returns
    ///
    /// Reference to the bootstrap configuration
    #[must_use]
    pub const fn config(&self) -> &BootstrapConfig {
        &self.config
    }

    /// Get initial peers for bootstrapping
    ///
    /// Returns all configured bootstrap nodes as DhtPeers that can
    /// be added to a routing table.
    ///
    /// # Returns
    ///
    /// Vector of bootstrap peers
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_discovery::dht::{Bootstrap, BootstrapConfig, BootstrapNode, NodeId};
    ///
    /// let mut config = BootstrapConfig::new();
    /// config.add_node(BootstrapNode::new(
    ///     NodeId::random(),
    ///     "127.0.0.1:8000".parse().unwrap(),
    ///     None
    /// ));
    ///
    /// let bootstrap = Bootstrap::new(config);
    /// let peers = bootstrap.initial_peers();
    /// assert_eq!(peers.len(), 1);
    /// ```
    #[must_use]
    pub fn initial_peers(&self) -> Vec<DhtPeer> {
        self.config.as_peers()
    }
}

/// Bootstrap errors
#[derive(Debug, Error)]
pub enum BootstrapError {
    /// No bootstrap nodes configured
    #[error("No bootstrap nodes configured")]
    NoBootstrapNodes,

    /// All bootstrap nodes failed
    #[error("All bootstrap nodes failed to respond")]
    AllNodesFailed,

    /// Network error
    #[error("Network error: {0}")]
    Network(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_node_creation() {
        let id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let name = Some("test-node".to_string());

        let node = BootstrapNode::new(id, addr, name.clone());

        assert_eq!(node.id, id);
        assert_eq!(node.addr, addr);
        assert_eq!(node.name, name);
    }

    #[test]
    fn test_bootstrap_node_to_peer() {
        let id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();

        let node = BootstrapNode::new(id, addr, None);
        let peer = node.to_peer();

        assert_eq!(peer.id, id);
        assert_eq!(peer.addr, addr);
    }

    #[test]
    fn test_bootstrap_config_new() {
        let config = BootstrapConfig::new();
        assert_eq!(config.node_count(), 0);
        assert!(config.is_empty());
    }

    #[test]
    fn test_bootstrap_config_add_node() {
        let mut config = BootstrapConfig::new();

        let node = BootstrapNode::new(NodeId::random(), "127.0.0.1:8000".parse().unwrap(), None);

        config.add_node(node);
        assert_eq!(config.node_count(), 1);
        assert!(!config.is_empty());
    }

    #[test]
    fn test_bootstrap_config_remove_node() {
        let mut config = BootstrapConfig::new();

        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let node = BootstrapNode::new(NodeId::random(), addr, None);

        config.add_node(node);
        assert_eq!(config.node_count(), 1);

        let removed = config.remove_node(&addr);
        assert!(removed);
        assert_eq!(config.node_count(), 0);
    }

    #[test]
    fn test_bootstrap_config_remove_nonexistent() {
        let mut config = BootstrapConfig::new();

        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let removed = config.remove_node(&addr);
        assert!(!removed);
    }

    #[test]
    fn test_bootstrap_config_as_peers() {
        let mut config = BootstrapConfig::new();

        let id1 = NodeId::random();
        let id2 = NodeId::random();

        config.add_node(BootstrapNode::new(
            id1,
            "127.0.0.1:8000".parse().unwrap(),
            None,
        ));
        config.add_node(BootstrapNode::new(
            id2,
            "127.0.0.1:8001".parse().unwrap(),
            None,
        ));

        let peers = config.as_peers();
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].id, id1);
        assert_eq!(peers[1].id, id2);
    }

    #[test]
    fn test_bootstrap_creation() {
        let config = BootstrapConfig::new();
        let bootstrap = Bootstrap::new(config);
        assert_eq!(bootstrap.config().node_count(), 0);
    }

    #[test]
    fn test_bootstrap_with_defaults() {
        let bootstrap = Bootstrap::with_defaults();
        // Default config is currently empty
        assert_eq!(bootstrap.config().node_count(), 0);
    }

    #[test]
    fn test_bootstrap_initial_peers() {
        let mut config = BootstrapConfig::new();

        config.add_node(BootstrapNode::new(
            NodeId::random(),
            "127.0.0.1:8000".parse().unwrap(),
            Some("node-1".to_string()),
        ));
        config.add_node(BootstrapNode::new(
            NodeId::random(),
            "127.0.0.1:8001".parse().unwrap(),
            Some("node-2".to_string()),
        ));

        let bootstrap = Bootstrap::new(config);
        let peers = bootstrap.initial_peers();

        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn test_bootstrap_config_default() {
        let config = BootstrapConfig::default();
        assert_eq!(config.node_count(), 0);
    }
}
