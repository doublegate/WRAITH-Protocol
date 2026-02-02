//! Discovery Manager
//!
//! Unified manager that orchestrates DHT, NAT traversal, and relay infrastructure
//! to provide seamless peer discovery and connection establishment.

use crate::dht::{DhtNode, NodeId};
use crate::nat::{
    Candidate, HolePuncher, IceGatherer, NatDetector, NatType, StunDnsResolver, StunServerSpec,
    default_stun_servers, fallback_stun_ips,
};
use crate::relay::client::{RelayClient, RelayClientState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

/// Discovery manager errors
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// DHT operation failed
    #[error("DHT operation failed: {0}")]
    DhtFailed(String),

    /// NAT traversal failed
    #[error("NAT traversal failed: {0}")]
    NatTraversalFailed(String),

    /// Relay connection failed
    #[error("Relay connection failed: {0}")]
    RelayFailed(String),

    /// Connection failed (all methods exhausted)
    #[error("Connection failed: all methods exhausted")]
    ConnectionFailed,

    /// Peer not found in DHT
    #[error("Peer not found in DHT")]
    PeerNotFound,

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    InvalidConfig(String),
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Local node ID
    pub node_id: NodeId,
    /// Local listen address
    pub listen_addr: SocketAddr,
    /// Bootstrap DHT nodes
    pub bootstrap_nodes: Vec<SocketAddr>,
    /// STUN servers for NAT detection
    pub stun_servers: Vec<SocketAddr>,
    /// Relay servers (address, node_id)
    pub relay_servers: Vec<RelayInfo>,
    /// Enable NAT detection
    pub nat_detection_enabled: bool,
    /// Enable relay fallback
    pub relay_enabled: bool,
    /// Connection timeout
    pub connection_timeout: Duration,
}

/// Relay server information
#[derive(Debug, Clone)]
pub struct RelayInfo {
    /// Relay server address
    pub addr: SocketAddr,
    /// Relay server node ID
    pub node_id: NodeId,
    /// Relay server public key
    pub public_key: [u8; 32],
}

impl DiscoveryConfig {
    /// Create a new discovery configuration
    ///
    /// Uses default STUN server IPs (fallback IPs from well-known STUN providers).
    /// For DNS-based STUN resolution, use `with_dns_resolution()` instead.
    #[must_use]
    pub fn new(node_id: NodeId, listen_addr: SocketAddr) -> Self {
        // Use fallback STUN IPs from the dns module
        let default_stun_servers = fallback_stun_ips();

        Self {
            node_id,
            listen_addr,
            bootstrap_nodes: Vec::new(),
            stun_servers: default_stun_servers,
            relay_servers: Vec::new(),
            nat_detection_enabled: true,
            relay_enabled: true,
            connection_timeout: Duration::from_secs(10),
        }
    }

    /// Create a new discovery configuration with DNS resolution for STUN servers
    ///
    /// Resolves STUN server hostnames (like stun.l.google.com) to IP addresses
    /// using DNS. Falls back to hardcoded IPs if DNS resolution fails completely.
    ///
    /// This is the preferred method when DNS is available, as it allows STUN
    /// servers to update their IPs without requiring client updates.
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn with_dns_resolution(
        node_id: NodeId,
        listen_addr: SocketAddr,
    ) -> Result<Self, DiscoveryError> {
        Self::with_custom_stun_dns_resolution(node_id, listen_addr, &default_stun_servers()).await
    }

    /// Create a new discovery configuration with DNS resolution for custom STUN servers
    ///
    /// Resolves the provided STUN server specifications (hostnames or IPs) using DNS.
    /// Falls back to hardcoded IPs if DNS resolution fails for all servers.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The local node's identifier
    /// * `listen_addr` - The local address to listen on
    /// * `stun_specs` - STUN server specifications (hostnames or IP addresses)
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn with_custom_stun_dns_resolution(
        node_id: NodeId,
        listen_addr: SocketAddr,
        stun_specs: &[StunServerSpec],
    ) -> Result<Self, DiscoveryError> {
        let resolver = StunDnsResolver::new()
            .await
            .map_err(|e| DiscoveryError::InvalidConfig(format!("DNS resolver init failed: {e}")))?;

        let mut resolved_servers = resolver.resolve_many(stun_specs).await;

        // Fall back to hardcoded IPs if DNS resolution failed for all servers
        if resolved_servers.is_empty() {
            tracing::warn!("DNS resolution failed for all STUN servers, using fallback IPs");
            resolved_servers = fallback_stun_ips();
        } else {
            tracing::info!(
                "Resolved {} STUN server addresses via DNS",
                resolved_servers.len()
            );
        }

        Ok(Self {
            node_id,
            listen_addr,
            bootstrap_nodes: Vec::new(),
            stun_servers: resolved_servers,
            relay_servers: Vec::new(),
            nat_detection_enabled: true,
            relay_enabled: true,
            connection_timeout: Duration::from_secs(10),
        })
    }

    /// Add a bootstrap DHT node
    pub fn add_bootstrap_node(&mut self, addr: SocketAddr) {
        self.bootstrap_nodes.push(addr);
    }

    /// Add a STUN server
    pub fn add_stun_server(&mut self, addr: SocketAddr) {
        self.stun_servers.push(addr);
    }

    /// Add a relay server
    pub fn add_relay_server(&mut self, info: RelayInfo) {
        self.relay_servers.push(info);
    }

    /// Replace STUN servers with DNS-resolved addresses
    ///
    /// This method allows resolving STUN server hostnames after initial
    /// configuration. Useful for refreshing DNS entries periodically.
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver initialization fails
    pub async fn resolve_stun_servers(
        &mut self,
        stun_specs: &[StunServerSpec],
    ) -> Result<(), DiscoveryError> {
        let resolver = StunDnsResolver::new()
            .await
            .map_err(|e| DiscoveryError::InvalidConfig(format!("DNS resolver init failed: {e}")))?;

        let resolved = resolver.resolve_many(stun_specs).await;

        if resolved.is_empty() {
            tracing::warn!("DNS resolution failed for all STUN servers, keeping existing servers");
        } else {
            self.stun_servers = resolved;
            tracing::info!(
                "Updated STUN servers: {} addresses resolved",
                self.stun_servers.len()
            );
        }

        Ok(())
    }
}

/// Peer connection information
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// Peer node ID
    pub peer_id: NodeId,
    /// Connection address
    pub addr: SocketAddr,
    /// Connection type
    pub connection_type: ConnectionType,
}

/// Type of connection established
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// Direct connection (no NAT or public IP)
    Direct,
    /// NAT hole-punched connection
    HolePunched,
    /// Relayed through DERP server
    Relayed(NodeId),
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Direct => write!(f, "Direct"),
            Self::HolePunched => write!(f, "HolePunched"),
            Self::Relayed(_id) => write!(f, "Relayed"),
        }
    }
}

/// Discovery manager state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryState {
    /// Not started
    Stopped,
    /// Starting up
    Starting,
    /// Running and ready
    Running,
    /// Shutting down
    Stopping,
}

/// Unified discovery manager
///
/// Orchestrates DHT, NAT traversal, and relay infrastructure to provide
/// seamless peer discovery and connection establishment.
pub struct DiscoveryManager {
    /// Configuration
    config: DiscoveryConfig,
    /// DHT node
    dht: Arc<RwLock<DhtNode>>,
    /// NAT detector
    nat_detector: Option<NatDetector>,
    /// ICE gatherer
    ice_gatherer: IceGatherer,
    /// Hole puncher
    hole_puncher: Option<Arc<HolePuncher>>,
    /// Relay clients (one per relay server)
    relay_clients: Arc<RwLock<Vec<RelayClient>>>,
    /// Detected NAT type
    nat_type: Arc<RwLock<Option<NatType>>>,
    /// Manager state
    state: Arc<RwLock<DiscoveryState>>,
}

impl DiscoveryManager {
    /// Create a new discovery manager
    ///
    /// Uses the STUN servers configured in `DiscoveryConfig`. For DNS-based
    /// resolution, use `DiscoveryConfig::with_dns_resolution()` to create the config.
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails
    pub async fn new(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        // Create DHT node
        let dht = Arc::new(RwLock::new(DhtNode::new(
            config.node_id,
            config.listen_addr,
        )));

        // Create NAT detector if enabled
        let nat_detector = if config.nat_detection_enabled {
            Some(NatDetector::with_servers(config.stun_servers.clone()))
        } else {
            None
        };

        // Create ICE gatherer
        let ice_gatherer = IceGatherer::with_stun_servers(config.stun_servers.clone());

        // Create hole puncher
        let hole_puncher = HolePuncher::new(config.listen_addr)
            .await
            .ok()
            .map(Arc::new);

        Ok(Self {
            config,
            dht,
            nat_detector,
            ice_gatherer,
            hole_puncher,
            relay_clients: Arc::new(RwLock::new(Vec::new())),
            nat_type: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(DiscoveryState::Stopped)),
        })
    }

    /// Create a new discovery manager with DNS resolution for STUN servers
    ///
    /// This is the preferred constructor when DNS is available. It resolves
    /// STUN server hostnames (like stun.l.google.com) to IP addresses, with
    /// automatic fallback to hardcoded IPs if DNS resolution fails.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The local node's identifier
    /// * `listen_addr` - The local address to listen on
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver or discovery manager initialization fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use wraith_discovery::{DiscoveryManager, dht::NodeId};
    /// use std::net::SocketAddr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let node_id = NodeId::random();
    /// let listen_addr: SocketAddr = "0.0.0.0:0".parse()?;
    ///
    /// // Uses DNS to resolve STUN servers with fallback to hardcoded IPs
    /// let manager = DiscoveryManager::with_dns_resolution(node_id, listen_addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_dns_resolution(
        node_id: NodeId,
        listen_addr: SocketAddr,
    ) -> Result<Self, DiscoveryError> {
        let config = DiscoveryConfig::with_dns_resolution(node_id, listen_addr).await?;
        Self::new(config).await
    }

    /// Create a new discovery manager with custom STUN server DNS resolution
    ///
    /// Resolves custom STUN server specifications using DNS, with fallback
    /// to hardcoded IPs if all resolutions fail.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The local node's identifier
    /// * `listen_addr` - The local address to listen on
    /// * `stun_specs` - Custom STUN server specifications (hostnames or IPs)
    ///
    /// # Errors
    ///
    /// Returns error if DNS resolver or discovery manager initialization fails
    pub async fn with_custom_stun_dns_resolution(
        node_id: NodeId,
        listen_addr: SocketAddr,
        stun_specs: &[StunServerSpec],
    ) -> Result<Self, DiscoveryError> {
        let config =
            DiscoveryConfig::with_custom_stun_dns_resolution(node_id, listen_addr, stun_specs)
                .await?;
        Self::new(config).await
    }

    /// Start the discovery manager
    ///
    /// Performs:
    /// - DHT bootstrap
    /// - NAT type detection
    /// - Relay registration
    ///
    /// # Errors
    ///
    /// Returns error if startup fails
    pub async fn start(&self) -> Result<(), DiscoveryError> {
        *self.state.write().await = DiscoveryState::Starting;

        // 1. Bootstrap DHT
        self.bootstrap_dht().await?;

        // 2. Detect NAT type (non-blocking - failure is not fatal)
        if let Some(detector) = &self.nat_detector {
            match detector.detect().await {
                Ok(nat_type) => {
                    *self.nat_type.write().await = Some(nat_type);
                    println!("Detected NAT type: {nat_type:?}");
                }
                Err(e) => {
                    eprintln!(
                        "Warning: NAT detection failed ({}), continuing in local mode",
                        e
                    );
                    eprintln!(
                        "Note: This is not fatal - the node will still function for local connections"
                    );
                    *self.nat_type.write().await = Some(NatType::Unknown);
                }
            }
        }

        // 3. Connect to relay servers
        if self.config.relay_enabled {
            self.connect_relays().await?;
        }

        *self.state.write().await = DiscoveryState::Running;
        Ok(())
    }

    /// Bootstrap the DHT
    async fn bootstrap_dht(&self) -> Result<(), DiscoveryError> {
        let _dht = self.dht.write().await;

        for bootstrap_addr in &self.config.bootstrap_nodes {
            // In a real implementation, we would send FIND_NODE requests
            // to bootstrap nodes and populate the routing table
            println!("Bootstrapping from {bootstrap_addr}");
        }

        Ok(())
    }

    /// Connect to all relay servers
    async fn connect_relays(&self) -> Result<(), DiscoveryError> {
        let mut clients = Vec::new();

        for relay_info in &self.config.relay_servers {
            match RelayClient::connect(relay_info.addr, *self.config.node_id.as_bytes()).await {
                Ok(mut client) => {
                    // Register with relay
                    if let Err(e) = client.register(&relay_info.public_key).await {
                        eprintln!("Failed to register with relay {}: {:?}", relay_info.addr, e);
                        continue;
                    }

                    // Spawn receiver task
                    client.spawn_receiver();

                    clients.push(client);
                    println!("Connected to relay: {}", relay_info.addr);
                }
                Err(e) => {
                    eprintln!("Failed to connect to relay {}: {:?}", relay_info.addr, e);
                }
            }
        }

        *self.relay_clients.write().await = clients;
        Ok(())
    }

    /// Discover a peer and establish connection
    ///
    /// Attempts connection in this order:
    /// 1. DHT lookup to find peer
    /// 2. Direct connection (if peer has public IP)
    /// 3. Hole punching (if both behind NAT)
    /// 4. Relay fallback (if direct fails)
    ///
    /// # Errors
    ///
    /// Returns error if all connection methods fail
    pub async fn connect_to_peer(&self, peer_id: NodeId) -> Result<PeerConnection, DiscoveryError> {
        // 1. Look up peer in DHT
        let peer_addrs = self.dht_lookup(peer_id).await?;

        if peer_addrs.is_empty() {
            return Err(DiscoveryError::PeerNotFound);
        }

        // 2. Gather local ICE candidates
        let local_candidates = self
            .ice_gatherer
            .gather(self.config.listen_addr)
            .await
            .unwrap_or_default();

        // 3. Try direct connection
        for peer_addr in &peer_addrs {
            if let Some(conn) = self.try_direct_connection(*peer_addr).await {
                return Ok(conn);
            }
        }

        // 4. Try hole punching
        if let Some(hole_puncher) = &self.hole_puncher
            && let Some(conn) = self
                .try_hole_punch(hole_puncher.clone(), &peer_addrs, &local_candidates)
                .await
        {
            return Ok(conn);
        }

        // 5. Fall back to relay
        if self.config.relay_enabled
            && let Some(conn) = self.connect_via_relay(peer_id).await
        {
            return Ok(conn);
        }

        Err(DiscoveryError::ConnectionFailed)
    }

    /// Perform DHT lookup for peer
    async fn dht_lookup(&self, peer_id: NodeId) -> Result<Vec<SocketAddr>, DiscoveryError> {
        let mut dht = self.dht.write().await;

        // Use iterative FIND_NODE to locate peer
        let closest_peers = dht.iterative_find_node(&peer_id).await;

        // Extract addresses
        let addrs = closest_peers.iter().map(|p| p.addr).collect();

        Ok(addrs)
    }

    /// Try direct connection to peer
    async fn try_direct_connection(&self, peer_addr: SocketAddr) -> Option<PeerConnection> {
        // Simple connectivity check (in real implementation, would attempt handshake)
        if self.is_reachable(peer_addr).await {
            Some(PeerConnection {
                peer_id: NodeId::from_bytes([0u8; 32]), // Would be actual peer ID
                addr: peer_addr,
                connection_type: ConnectionType::Direct,
            })
        } else {
            None
        }
    }

    /// Try NAT hole punching
    async fn try_hole_punch(
        &self,
        hole_puncher: Arc<HolePuncher>,
        peer_addrs: &[SocketAddr],
        _local_candidates: &[Candidate],
    ) -> Option<PeerConnection> {
        for peer_addr in peer_addrs {
            // Attempt hole punching with timeout
            match tokio::time::timeout(Duration::from_secs(5), hole_puncher.punch(*peer_addr, None))
                .await
            {
                Ok(Ok(punched_addr)) => {
                    return Some(PeerConnection {
                        peer_id: NodeId::from_bytes([0u8; 32]), // Would be actual peer ID
                        addr: punched_addr,
                        connection_type: ConnectionType::HolePunched,
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("Hole punch failed: {e:?}");
                }
                Err(_) => {
                    eprintln!("Hole punch timeout");
                }
            }
        }

        None
    }

    /// Connect via relay server
    async fn connect_via_relay(&self, peer_id: NodeId) -> Option<PeerConnection> {
        let clients = self.relay_clients.read().await;

        for client in clients.iter() {
            if client.state().await == RelayClientState::Connected {
                // In real implementation, would negotiate relay connection
                // For now, return placeholder
                return Some(PeerConnection {
                    peer_id,
                    addr: client.relay_addr(),
                    connection_type: ConnectionType::Relayed(NodeId::from_bytes([0u8; 32])),
                });
            }
        }

        None
    }

    /// Check if peer address is reachable
    async fn is_reachable(&self, _addr: SocketAddr) -> bool {
        // Placeholder: in real implementation, would send ping/probe
        false
    }

    /// Shutdown the discovery manager
    ///
    /// # Errors
    ///
    /// Returns error if shutdown fails
    pub async fn shutdown(&self) -> Result<(), DiscoveryError> {
        *self.state.write().await = DiscoveryState::Stopping;

        // Disconnect from all relays
        let mut clients = self.relay_clients.write().await;
        for client in clients.iter_mut() {
            let _ = client.disconnect().await;
        }
        clients.clear();

        *self.state.write().await = DiscoveryState::Stopped;
        Ok(())
    }

    /// Get current manager state
    #[must_use]
    pub async fn state(&self) -> DiscoveryState {
        *self.state.read().await
    }

    /// Get detected NAT type
    #[must_use]
    pub async fn nat_type(&self) -> Option<NatType> {
        *self.nat_type.read().await
    }

    /// Get DHT node reference
    #[must_use]
    pub fn dht(&self) -> Arc<RwLock<DhtNode>> {
        self.dht.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_creation() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();

        let config = DiscoveryConfig::new(node_id, addr);

        assert_eq!(config.node_id, node_id);
        assert_eq!(config.listen_addr, addr);
        assert!(config.nat_detection_enabled);
        assert!(config.relay_enabled);
    }

    #[test]
    fn test_discovery_config_builders() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);

        config.add_bootstrap_node("127.0.0.1:9000".parse().unwrap());
        config.add_stun_server("1.1.1.1:3478".parse().unwrap());

        assert_eq!(config.bootstrap_nodes.len(), 1);
        assert_eq!(config.stun_servers.len(), 6); // 5 default + 1 added
    }

    #[test]
    fn test_connection_type_display() {
        assert_eq!(ConnectionType::Direct.to_string(), "Direct");
        assert_eq!(ConnectionType::HolePunched.to_string(), "HolePunched");

        let relay_id = NodeId::from_bytes([1u8; 32]);
        let relayed = ConnectionType::Relayed(relay_id);
        assert!(relayed.to_string().contains("Relayed"));
    }

    #[tokio::test]
    async fn test_discovery_manager_creation() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert_eq!(manager.state().await, DiscoveryState::Stopped);
    }

    #[tokio::test]
    async fn test_discovery_manager_state_transitions() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8001".parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await.unwrap();

        assert_eq!(manager.state().await, DiscoveryState::Stopped);

        // Start would change to Starting then Running, but needs network
        // Just verify state transitions work
        *manager.state.write().await = DiscoveryState::Running;
        assert_eq!(manager.state().await, DiscoveryState::Running);
    }

    #[tokio::test]
    async fn test_discovery_manager_nat_type() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8002".parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await.unwrap();

        assert!(manager.nat_type().await.is_none());

        *manager.nat_type.write().await = Some(NatType::FullCone);
        assert_eq!(manager.nat_type().await, Some(NatType::FullCone));
    }

    #[tokio::test]
    async fn test_discovery_manager_shutdown() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8003".parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await.unwrap();

        assert_eq!(manager.state().await, DiscoveryState::Stopped);

        let result = manager.shutdown().await;
        assert!(result.is_ok());
        assert_eq!(manager.state().await, DiscoveryState::Stopped);
    }

    #[test]
    fn test_relay_info_creation() {
        let addr = "127.0.0.1:443".parse().unwrap();
        let node_id = NodeId::random();
        let public_key = [42u8; 32];

        let relay_info = RelayInfo {
            addr,
            node_id,
            public_key,
        };

        assert_eq!(relay_info.addr, addr);
        assert_eq!(relay_info.node_id, node_id);
        assert_eq!(relay_info.public_key, public_key);
    }

    #[test]
    fn test_discovery_error_display() {
        let err = DiscoveryError::DhtFailed("test error".to_string());
        assert!(err.to_string().contains("DHT operation failed"));

        let err = DiscoveryError::NatTraversalFailed("test".to_string());
        assert!(err.to_string().contains("NAT traversal failed"));

        let err = DiscoveryError::RelayFailed("test".to_string());
        assert!(err.to_string().contains("Relay connection failed"));

        let err = DiscoveryError::ConnectionFailed;
        assert_eq!(err.to_string(), "Connection failed: all methods exhausted");

        let err = DiscoveryError::PeerNotFound;
        assert_eq!(err.to_string(), "Peer not found in DHT");
    }

    #[test]
    fn test_discovery_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let discovery_err: DiscoveryError = io_err.into();
        assert!(matches!(discovery_err, DiscoveryError::Io(_)));
    }

    #[test]
    fn test_discovery_state_all_variants() {
        let states = vec![
            DiscoveryState::Stopped,
            DiscoveryState::Starting,
            DiscoveryState::Running,
            DiscoveryState::Stopping,
        ];

        for state in states {
            // Ensure all states can be created and compared
            assert_eq!(state, state);
        }
    }

    #[test]
    fn test_connection_type_all_variants() {
        let relay_id = NodeId::from_bytes([1u8; 32]);

        assert_eq!(ConnectionType::Direct, ConnectionType::Direct);
        assert_eq!(ConnectionType::HolePunched, ConnectionType::HolePunched);
        assert_eq!(
            ConnectionType::Relayed(relay_id),
            ConnectionType::Relayed(relay_id)
        );

        assert_ne!(ConnectionType::Direct, ConnectionType::HolePunched);
        assert_ne!(ConnectionType::Direct, ConnectionType::Relayed(relay_id));
    }

    #[test]
    fn test_peer_connection_creation() {
        let peer_id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();

        let conn = PeerConnection {
            peer_id,
            addr,
            connection_type: ConnectionType::Direct,
        };

        assert_eq!(conn.peer_id, peer_id);
        assert_eq!(conn.addr, addr);
        assert_eq!(conn.connection_type, ConnectionType::Direct);
    }

    #[tokio::test]
    async fn test_discovery_manager_dht_access() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8004".parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await.unwrap();
        let dht = manager.dht();

        // Verify DHT is accessible
        let dht_lock = dht.read().await;
        assert_eq!(*dht_lock.id(), node_id);
    }

    #[test]
    fn test_discovery_config_multiple_additions() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8000".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);

        // Add multiple bootstrap nodes
        for i in 0..5 {
            config.add_bootstrap_node(format!("127.0.0.1:{}", 9000 + i).parse().unwrap());
        }
        assert_eq!(config.bootstrap_nodes.len(), 5);

        // Add multiple STUN servers
        for i in 0..3 {
            config.add_stun_server(format!("10.0.0.{}:3478", i + 1).parse().unwrap());
        }
        assert_eq!(config.stun_servers.len(), 8); // 5 default + 3 added

        // Add multiple relay servers
        for i in 0..3 {
            let relay = RelayInfo {
                addr: format!("203.0.113.{}:443", i + 1).parse().unwrap(),
                node_id: NodeId::random(),
                public_key: [i; 32],
            };
            config.add_relay_server(relay);
        }
        assert_eq!(config.relay_servers.len(), 3);
    }

    #[tokio::test]
    async fn test_discovery_manager_creation_with_custom_config() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8005".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);

        config.nat_detection_enabled = false;
        config.relay_enabled = false;

        let manager = DiscoveryManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_discovery_config_with_dns_resolution() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8006".parse().unwrap();

        // Test DNS resolution config constructor
        let result = DiscoveryConfig::with_dns_resolution(node_id, addr).await;
        assert!(result.is_ok());

        let config = result.unwrap();
        // Should have resolved addresses or fallback IPs
        assert!(!config.stun_servers.is_empty());
        assert!(config.nat_detection_enabled);
        assert!(config.relay_enabled);
    }

    #[tokio::test]
    async fn test_discovery_config_with_custom_stun_dns() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8007".parse().unwrap();

        // Test with IP-based specs (no actual DNS needed)
        let specs = vec![
            StunServerSpec::ip("1.2.3.4:3478".parse().unwrap()),
            StunServerSpec::ip("5.6.7.8:3478".parse().unwrap()),
        ];

        let result = DiscoveryConfig::with_custom_stun_dns_resolution(node_id, addr, &specs).await;
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.stun_servers.len(), 2);
    }

    #[tokio::test]
    async fn test_discovery_config_resolve_stun_servers() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8008".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);

        let initial_count = config.stun_servers.len();

        // Test resolving new STUN servers (IP-based, no actual DNS)
        let specs = vec![
            StunServerSpec::ip("10.0.0.1:3478".parse().unwrap()),
            StunServerSpec::ip("10.0.0.2:3478".parse().unwrap()),
            StunServerSpec::ip("10.0.0.3:3478".parse().unwrap()),
        ];

        let result = config.resolve_stun_servers(&specs).await;
        assert!(result.is_ok());

        // Should have replaced the servers
        assert_eq!(config.stun_servers.len(), 3);
        assert_ne!(config.stun_servers.len(), initial_count);
    }

    #[tokio::test]
    async fn test_discovery_manager_with_dns_resolution() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8009".parse().unwrap();

        // Test DNS resolution manager constructor
        let result = DiscoveryManager::with_dns_resolution(node_id, addr).await;
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert_eq!(manager.state().await, DiscoveryState::Stopped);
    }

    #[tokio::test]
    async fn test_discovery_manager_with_custom_stun_dns() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8010".parse().unwrap();

        // Test with IP-based specs (no actual DNS needed)
        let specs = vec![
            StunServerSpec::ip("1.2.3.4:3478".parse().unwrap()),
            StunServerSpec::ip("5.6.7.8:19302".parse().unwrap()),
        ];

        let result = DiscoveryManager::with_custom_stun_dns_resolution(node_id, addr, &specs).await;
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert_eq!(manager.state().await, DiscoveryState::Stopped);
    }

    #[test]
    fn test_discovery_error_invalid_config() {
        let err = DiscoveryError::InvalidConfig("bad config".to_string());
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("bad config"));
    }

    #[test]
    fn test_discovery_state_debug() {
        let state = DiscoveryState::Starting;
        let debug = format!("{:?}", state);
        assert!(debug.contains("Starting"));
    }

    #[test]
    fn test_discovery_state_copy_clone() {
        let state = DiscoveryState::Running;
        let copied = state;
        let cloned = state.clone();
        assert_eq!(state, copied);
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_connection_type_copy_clone() {
        let ct = ConnectionType::Direct;
        let copied = ct;
        let cloned = ct.clone();
        assert_eq!(ct, copied);
        assert_eq!(ct, cloned);
    }

    #[test]
    fn test_peer_connection_debug_clone() {
        let conn = PeerConnection {
            peer_id: NodeId::random(),
            addr: "127.0.0.1:8000".parse().unwrap(),
            connection_type: ConnectionType::Direct,
        };
        let debug = format!("{:?}", conn);
        assert!(debug.contains("Direct"));
        let cloned = conn.clone();
        assert_eq!(cloned.addr, conn.addr);
    }

    #[test]
    fn test_relay_info_debug_clone() {
        let info = RelayInfo {
            addr: "1.2.3.4:443".parse().unwrap(),
            node_id: NodeId::random(),
            public_key: [0u8; 32],
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("1.2.3.4"));
        let cloned = info.clone();
        assert_eq!(cloned.addr, info.addr);
    }

    #[test]
    fn test_discovery_config_debug_clone() {
        let config = DiscoveryConfig::new(NodeId::random(), "127.0.0.1:0".parse().unwrap());
        let debug = format!("{:?}", config);
        assert!(debug.contains("nat_detection_enabled"));
        let cloned = config.clone();
        assert_eq!(cloned.listen_addr, config.listen_addr);
    }

    #[test]
    fn test_discovery_config_timeout() {
        let config = DiscoveryConfig::new(NodeId::random(), "127.0.0.1:0".parse().unwrap());
        assert_eq!(config.connection_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_discovery_manager_start_no_bootstrap() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);
        config.nat_detection_enabled = false;
        config.relay_enabled = false;

        let manager = DiscoveryManager::new(config).await.unwrap();
        let result = manager.start().await;
        assert!(result.is_ok());
        assert_eq!(manager.state().await, DiscoveryState::Running);
    }

    #[tokio::test]
    async fn test_discovery_manager_connect_to_peer_not_found() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);
        config.nat_detection_enabled = false;
        config.relay_enabled = false;

        let manager = DiscoveryManager::new(config).await.unwrap();
        let peer_id = NodeId::random();
        let result = manager.connect_to_peer(peer_id).await;
        // DHT lookup returns empty -> PeerNotFound
        assert!(matches!(result, Err(DiscoveryError::PeerNotFound)));
    }

    #[tokio::test]
    async fn test_discovery_manager_shutdown_after_start() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);
        config.nat_detection_enabled = false;
        config.relay_enabled = false;

        let manager = DiscoveryManager::new(config).await.unwrap();
        manager.start().await.unwrap();
        assert_eq!(manager.state().await, DiscoveryState::Running);

        manager.shutdown().await.unwrap();
        assert_eq!(manager.state().await, DiscoveryState::Stopped);
    }

    #[tokio::test]
    async fn test_discovery_manager_with_relay_config_no_server() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut config = DiscoveryConfig::new(node_id, addr);
        config.nat_detection_enabled = false;
        config.relay_enabled = true;
        // Add a relay that won't be reachable for register
        config.add_relay_server(RelayInfo {
            addr: "127.0.0.1:1".parse().unwrap(), // unreachable port
            node_id: NodeId::random(),
            public_key: [0u8; 32],
        });

        let manager = DiscoveryManager::new(config).await.unwrap();
        // Start will attempt relay connections but should not fail fatally
        let result = manager.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_fallback_on_empty_resolution() {
        let node_id = NodeId::random();
        let addr = "127.0.0.1:8011".parse().unwrap();

        // Test with hostname that won't resolve (will fallback to hardcoded IPs)
        // Note: This uses a hostname that's unlikely to resolve
        let specs = vec![StunServerSpec::hostname(
            "nonexistent.invalid.example",
            3478,
        )];

        let result = DiscoveryConfig::with_custom_stun_dns_resolution(node_id, addr, &specs).await;
        assert!(result.is_ok());

        let config = result.unwrap();
        // Should have fallen back to hardcoded IPs
        assert!(!config.stun_servers.is_empty());
    }
}
