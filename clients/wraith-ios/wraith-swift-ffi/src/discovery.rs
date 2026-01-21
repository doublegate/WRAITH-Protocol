// WRAITH iOS Discovery Module
//
// Provides DHT peer discovery and NAT traversal for iOS clients via UniFFI.
// Handles iOS-specific constraints like background task limits and network changes.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use wraith_discovery::dht::NodeId;
use wraith_discovery::{
    DiscoveryConfig, DiscoveryManager, DiscoveryState, NatDetector, NatType, StunClient,
    fallback_stun_ips,
};

use crate::error::{Result, WraithError};

/// Mobile-specific keep-alive interval (30 seconds for aggressive NAT)
const MOBILE_KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Background keep-alive interval when app is backgrounded (60 seconds)
const BACKGROUND_KEEPALIVE_INTERVAL_SECS: u64 = 60;

/// Maximum time before re-detecting NAT type after network change
const NAT_REDETECT_INTERVAL_SECS: u64 = 300;

/// iOS background task maximum duration (approximately 30 seconds)
const IOS_BACKGROUND_TASK_MAX_SECS: u64 = 30;

/// Mobile network state for adaptive behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileNetworkType {
    /// WiFi connection
    Wifi,
    /// Cellular data (4G/5G)
    Cellular,
    /// Unknown or no connection
    Unknown,
}

/// Peer information for discovery results
#[derive(Clone, Debug, uniffi::Record)]
pub struct DiscoveryPeerInfo {
    /// Peer node ID (hex encoded)
    pub peer_id: String,
    /// Peer address
    pub address: String,
    /// Connection type (direct, hole-punched, relayed)
    pub connection_type: String,
    /// Last seen timestamp (Unix millis)
    pub last_seen: u64,
}

/// NAT traversal result
#[derive(Clone, Debug, uniffi::Record)]
pub struct NatInfo {
    /// Detected NAT type
    pub nat_type: String,
    /// External (public) IP address
    pub external_ip: String,
    /// External port
    pub external_port: u16,
    /// Whether hole punching is likely to work
    pub hole_punch_capable: bool,
}

/// Discovery configuration for iOS clients
#[derive(Clone, uniffi::Record)]
pub struct MobileDiscoveryConfig {
    /// Local node ID (hex encoded)
    pub node_id_hex: String,
    /// Local listen address
    pub listen_addr: String,
    /// Bootstrap DHT nodes (comma-separated)
    pub bootstrap_nodes: String,
    /// STUN servers (comma-separated)
    pub stun_servers: String,
    /// Enable battery-saving mode
    pub battery_saving: bool,
    /// Keep-alive interval override in seconds (0 = use defaults)
    pub keepalive_interval_secs: u64,
}

impl Default for MobileDiscoveryConfig {
    fn default() -> Self {
        Self {
            node_id_hex: String::new(),
            listen_addr: "0.0.0.0:0".to_string(),
            bootstrap_nodes: String::new(),
            stun_servers: String::new(),
            battery_saving: false,
            keepalive_interval_secs: 0,
        }
    }
}

/// Discovery client status
#[derive(Clone, Debug, uniffi::Record)]
pub struct DiscoveryStatus {
    /// Current state
    pub state: String,
    /// NAT type (if detected)
    pub nat_type: Option<String>,
    /// External address (if known)
    pub external_address: Option<String>,
    /// Is app backgrounded
    pub is_backgrounded: bool,
    /// Current network type
    pub network_type: String,
}

/// Mobile discovery client for iOS
///
/// Provides a simplified interface for iOS clients to:
/// - Discover peers via DHT
/// - Detect NAT type and external address
/// - Maintain connections through NAT
#[derive(uniffi::Object)]
pub struct MobileDiscoveryClient {
    /// Node ID
    node_id: NodeId,
    /// Listen address
    listen_addr: SocketAddr,
    /// Bootstrap nodes
    bootstrap_nodes: Vec<SocketAddr>,
    /// STUN servers
    stun_servers: Vec<SocketAddr>,
    /// Battery saving mode (affects DHT maintenance frequency)
    #[allow(dead_code)]
    battery_saving: bool,
    /// Keep-alive interval override
    keepalive_interval_secs: u64,
    /// Discovery manager
    manager: Arc<Mutex<Option<DiscoveryManager>>>,
    /// Current NAT type
    nat_type: Arc<RwLock<Option<NatType>>>,
    /// External address (from STUN)
    external_addr: Arc<RwLock<Option<SocketAddr>>>,
    /// Last NAT detection time
    last_nat_detection: Arc<RwLock<Option<Instant>>>,
    /// Current network type
    current_network: Arc<RwLock<MobileNetworkType>>,
    /// Is app in background
    is_backgrounded: Arc<RwLock<bool>>,
}

#[uniffi::export]
impl MobileDiscoveryClient {
    /// Create a new mobile discovery client
    #[uniffi::constructor]
    pub fn new(config: MobileDiscoveryConfig) -> Result<Arc<Self>> {
        // Parse node ID
        let node_id = if config.node_id_hex.is_empty() {
            NodeId::random()
        } else {
            let node_id_bytes = hex::decode(&config.node_id_hex).map_err(|e| {
                WraithError::InitializationFailed {
                    message: format!("Invalid node ID hex: {}", e),
                }
            })?;
            let node_id_array: [u8; 32] =
                node_id_bytes
                    .try_into()
                    .map_err(|_| WraithError::InitializationFailed {
                        message: "Node ID must be 32 bytes".to_string(),
                    })?;
            NodeId::from_bytes(node_id_array)
        };

        // Parse listen address
        let listen_addr: SocketAddr =
            config
                .listen_addr
                .parse()
                .map_err(|e| WraithError::InitializationFailed {
                    message: format!("Invalid listen address: {}", e),
                })?;

        // Parse bootstrap nodes
        let bootstrap_nodes: Vec<SocketAddr> = config
            .bootstrap_nodes
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        // Parse STUN servers
        let stun_servers: Vec<SocketAddr> = config
            .stun_servers
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        Ok(Arc::new(Self {
            node_id,
            listen_addr,
            bootstrap_nodes,
            stun_servers,
            battery_saving: config.battery_saving,
            keepalive_interval_secs: config.keepalive_interval_secs,
            manager: Arc::new(Mutex::new(None)),
            nat_type: Arc::new(RwLock::new(None)),
            external_addr: Arc::new(RwLock::new(None)),
            last_nat_detection: Arc::new(RwLock::new(None)),
            current_network: Arc::new(RwLock::new(MobileNetworkType::Unknown)),
            is_backgrounded: Arc::new(RwLock::new(false)),
        }))
    }

    /// Initialize and start the discovery service
    pub fn start(&self) -> Result<()> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.start_async().await })
    }

    /// Stop the discovery service
    pub fn stop(&self) -> Result<()> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.stop_async().await })
    }

    /// Discover a peer by node ID (hex encoded)
    pub fn discover_peer(&self, peer_id_hex: String) -> Result<DiscoveryPeerInfo> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.discover_peer_async(&peer_id_hex).await })
    }

    /// Detect NAT type and get external address
    pub fn detect_nat(&self) -> Result<NatInfo> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.detect_nat_async().await })
    }

    /// Refresh NAT information (forced re-detection)
    pub fn refresh_nat(&self) -> Result<()> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.refresh_nat_async().await })
    }

    /// Notify that the network has changed
    pub fn on_network_changed(&self, network_type: MobileNetworkType) {
        if let Ok(rt) = crate::get_or_create_runtime() {
            rt.block_on(async {
                self.on_network_changed_async(network_type).await;
            });
        }
    }

    /// Notify that the app has been backgrounded
    pub fn on_app_backgrounded(&self) {
        if let Ok(rt) = crate::get_or_create_runtime() {
            rt.block_on(async {
                *self.is_backgrounded.write().await = true;
                log::info!("iOS app backgrounded - switching to battery-saving mode");
            });
        }
    }

    /// Notify that the app has been foregrounded
    pub fn on_app_foregrounded(&self) {
        if let Ok(rt) = crate::get_or_create_runtime() {
            rt.block_on(async {
                *self.is_backgrounded.write().await = false;
                log::info!("iOS app foregrounded - restoring normal operation");

                // Refresh NAT info if needed
                let should_refresh = {
                    let last = self.last_nat_detection.read().await;
                    match *last {
                        None => true,
                        Some(instant) => instant.elapsed() > Duration::from_secs(60),
                    }
                };

                if should_refresh {
                    let _ = self.refresh_nat_async().await;
                }
            });
        }
    }

    /// Get the current keep-alive interval in seconds
    pub fn get_keepalive_interval_secs(&self) -> u64 {
        if self.keepalive_interval_secs > 0 {
            return self.keepalive_interval_secs;
        }

        if let Ok(rt) = crate::get_or_create_runtime() {
            rt.block_on(async {
                let is_backgrounded = *self.is_backgrounded.read().await;
                if is_backgrounded {
                    BACKGROUND_KEEPALIVE_INTERVAL_SECS
                } else {
                    MOBILE_KEEPALIVE_INTERVAL_SECS
                }
            })
        } else {
            MOBILE_KEEPALIVE_INTERVAL_SECS
        }
    }

    /// Get iOS background task maximum duration in seconds
    pub fn get_ios_background_task_max_secs(&self) -> u64 {
        IOS_BACKGROUND_TASK_MAX_SECS
    }

    /// Get the current discovery status
    pub fn get_status(&self) -> Result<DiscoveryStatus> {
        let rt = crate::get_or_create_runtime()?;
        rt.block_on(async { self.get_status_async().await })
    }

    /// Get the local node ID (hex encoded)
    pub fn get_local_node_id(&self) -> String {
        hex::encode(self.node_id.as_bytes())
    }
}

// Internal async implementations
impl MobileDiscoveryClient {
    async fn start_async(&self) -> Result<()> {
        // Build discovery config
        let mut disc_config = DiscoveryConfig::new(self.node_id, self.listen_addr);

        for node in &self.bootstrap_nodes {
            disc_config.add_bootstrap_node(*node);
        }

        // Use fallback STUN servers if none provided
        let stun_servers = if self.stun_servers.is_empty() {
            fallback_stun_ips()
        } else {
            self.stun_servers.clone()
        };

        for server in &stun_servers {
            disc_config.add_stun_server(*server);
        }

        // Create discovery manager
        let manager = DiscoveryManager::new(disc_config).await.map_err(|e| {
            WraithError::InitializationFailed {
                message: format!("Failed to create discovery manager: {}", e),
            }
        })?;

        // Start the manager
        manager
            .start()
            .await
            .map_err(|e| WraithError::InitializationFailed {
                message: format!("Failed to start discovery: {}", e),
            })?;

        // Get initial NAT type
        if let Some(nat_type) = manager.nat_type().await {
            *self.nat_type.write().await = Some(nat_type);
        }

        *self.last_nat_detection.write().await = Some(Instant::now());
        *self.manager.lock().await = Some(manager);

        log::info!("iOS mobile discovery service started");
        Ok(())
    }

    async fn stop_async(&self) -> Result<()> {
        if let Some(manager) = self.manager.lock().await.take() {
            manager.shutdown().await.map_err(|e| WraithError::Other {
                message: format!("Failed to shutdown discovery: {}", e),
            })?;
        }

        log::info!("iOS mobile discovery service stopped");
        Ok(())
    }

    async fn discover_peer_async(&self, peer_id_hex: &str) -> Result<DiscoveryPeerInfo> {
        let manager_guard = self.manager.lock().await;
        let manager = manager_guard.as_ref().ok_or(WraithError::NotStarted {
            message: "Discovery not started".to_string(),
        })?;

        // Parse peer ID
        let peer_id_bytes = hex::decode(peer_id_hex).map_err(|e| WraithError::InvalidPeerId {
            message: format!("Invalid peer ID hex: {}", e),
        })?;
        let peer_id_array: [u8; 32] =
            peer_id_bytes
                .try_into()
                .map_err(|_| WraithError::InvalidPeerId {
                    message: "Peer ID must be 32 bytes".to_string(),
                })?;
        let peer_id = NodeId::from_bytes(peer_id_array);

        // Attempt connection via discovery manager
        match manager.connect_to_peer(peer_id).await {
            Ok(connection) => Ok(DiscoveryPeerInfo {
                peer_id: peer_id_hex.to_string(),
                address: connection.addr.to_string(),
                connection_type: connection.connection_type.to_string(),
                last_seen: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            }),
            Err(e) => Err(WraithError::SessionFailed {
                message: format!("Peer discovery failed: {}", e),
            }),
        }
    }

    async fn detect_nat_async(&self) -> Result<NatInfo> {
        // Check if we need to re-detect
        let should_detect = {
            let last_detection = self.last_nat_detection.read().await;
            match *last_detection {
                None => true,
                Some(instant) => {
                    instant.elapsed() > Duration::from_secs(NAT_REDETECT_INTERVAL_SECS)
                }
            }
        };

        if should_detect {
            self.refresh_nat_async().await?;
        }

        let nat_type = self.nat_type.read().await;
        let external_addr = self.external_addr.read().await;

        let nat_type_val = nat_type.unwrap_or(NatType::Unknown);
        let (external_ip, external_port) = match *external_addr {
            Some(addr) => (addr.ip().to_string(), addr.port()),
            None => (String::new(), 0),
        };

        Ok(NatInfo {
            nat_type: format!("{}", nat_type_val),
            external_ip,
            external_port,
            hole_punch_capable: Self::is_hole_punch_capable(nat_type_val),
        })
    }

    async fn refresh_nat_async(&self) -> Result<()> {
        // Get STUN servers
        let stun_servers = if self.stun_servers.is_empty() {
            fallback_stun_ips()
        } else {
            self.stun_servers.clone()
        };

        if stun_servers.is_empty() {
            return Err(WraithError::Other {
                message: "No STUN servers available".to_string(),
            });
        }

        // Detect NAT type
        let detector = NatDetector::with_servers(stun_servers.clone());
        let nat_type = detector.detect().await.unwrap_or(NatType::Unknown);
        *self.nat_type.write().await = Some(nat_type);

        // Get external address via STUN
        if let Ok(client) = StunClient::bind("0.0.0.0:0").await {
            for server in &stun_servers {
                if let Ok(addr) = client.get_mapped_address(*server).await {
                    *self.external_addr.write().await = Some(addr);
                    break;
                }
            }
        }

        *self.last_nat_detection.write().await = Some(Instant::now());
        log::info!("NAT detection refreshed: {:?}", nat_type);

        Ok(())
    }

    async fn on_network_changed_async(&self, network_type: MobileNetworkType) {
        let previous = *self.current_network.read().await;
        if previous != network_type {
            *self.current_network.write().await = network_type;
            log::info!("Network changed from {:?} to {:?}", previous, network_type);

            // Trigger NAT re-detection
            if let Err(e) = self.refresh_nat_async().await {
                log::warn!("Failed to refresh NAT info after network change: {:?}", e);
            }
        }
    }

    async fn get_status_async(&self) -> Result<DiscoveryStatus> {
        let state = {
            let manager_guard = self.manager.lock().await;
            match manager_guard.as_ref() {
                Some(manager) => match manager.state().await {
                    DiscoveryState::Stopped => "stopped".to_string(),
                    DiscoveryState::Starting => "starting".to_string(),
                    DiscoveryState::Running => "running".to_string(),
                    DiscoveryState::Stopping => "stopping".to_string(),
                },
                None => "not_initialized".to_string(),
            }
        };

        let nat_type = self.nat_type.read().await.map(|t| format!("{}", t));
        let external_address = self.external_addr.read().await.map(|a| a.to_string());
        let is_backgrounded = *self.is_backgrounded.read().await;
        let network_type = format!("{:?}", *self.current_network.read().await);

        Ok(DiscoveryStatus {
            state,
            nat_type,
            external_address,
            is_backgrounded,
            network_type,
        })
    }

    fn is_hole_punch_capable(nat_type: NatType) -> bool {
        matches!(
            nat_type,
            NatType::Open
                | NatType::FullCone
                | NatType::RestrictedCone
                | NatType::PortRestrictedCone
        )
    }
}

/// Create a discovery client with default configuration
#[uniffi::export]
pub fn create_discovery_client(
    node_id_hex: String,
    listen_addr: String,
) -> Result<Arc<MobileDiscoveryClient>> {
    let config = MobileDiscoveryConfig {
        node_id_hex,
        listen_addr,
        ..Default::default()
    };
    MobileDiscoveryClient::new(config)
}

/// Create a discovery client with full configuration
#[uniffi::export]
pub fn create_discovery_client_with_config(
    config: MobileDiscoveryConfig,
) -> Result<Arc<MobileDiscoveryClient>> {
    MobileDiscoveryClient::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_network_type() {
        assert_ne!(MobileNetworkType::Wifi, MobileNetworkType::Cellular);
        assert_eq!(MobileNetworkType::Unknown, MobileNetworkType::Unknown);
    }

    #[test]
    fn test_mobile_discovery_config_default() {
        let config = MobileDiscoveryConfig::default();
        assert!(config.node_id_hex.is_empty());
        assert_eq!(config.listen_addr, "0.0.0.0:0");
        assert!(config.bootstrap_nodes.is_empty());
        assert!(config.stun_servers.is_empty());
        assert!(!config.battery_saving);
        assert_eq!(config.keepalive_interval_secs, 0);
    }

    #[test]
    fn test_is_hole_punch_capable() {
        assert!(MobileDiscoveryClient::is_hole_punch_capable(NatType::Open));
        assert!(MobileDiscoveryClient::is_hole_punch_capable(
            NatType::FullCone
        ));
        assert!(MobileDiscoveryClient::is_hole_punch_capable(
            NatType::RestrictedCone
        ));
        assert!(MobileDiscoveryClient::is_hole_punch_capable(
            NatType::PortRestrictedCone
        ));
        assert!(!MobileDiscoveryClient::is_hole_punch_capable(
            NatType::Symmetric
        ));
        assert!(!MobileDiscoveryClient::is_hole_punch_capable(
            NatType::Unknown
        ));
    }

    #[test]
    fn test_discovery_peer_info_creation() {
        let info = DiscoveryPeerInfo {
            peer_id: "abc123".to_string(),
            address: "192.168.1.1:8080".to_string(),
            connection_type: "Direct".to_string(),
            last_seen: 1234567890,
        };
        assert_eq!(info.peer_id, "abc123");
        assert_eq!(info.address, "192.168.1.1:8080");
    }

    #[test]
    fn test_nat_info_creation() {
        let info = NatInfo {
            nat_type: "Full Cone NAT".to_string(),
            external_ip: "203.0.113.1".to_string(),
            external_port: 54321,
            hole_punch_capable: true,
        };
        assert_eq!(info.nat_type, "Full Cone NAT");
        assert!(info.hole_punch_capable);
    }

    #[test]
    fn test_discovery_status_creation() {
        let status = DiscoveryStatus {
            state: "running".to_string(),
            nat_type: Some("Full Cone NAT".to_string()),
            external_address: Some("203.0.113.1:54321".to_string()),
            is_backgrounded: false,
            network_type: "Wifi".to_string(),
        };
        assert_eq!(status.state, "running");
        assert!(!status.is_backgrounded);
    }

    #[test]
    fn test_ios_background_task_constant() {
        assert_eq!(IOS_BACKGROUND_TASK_MAX_SECS, 30);
    }

    #[test]
    fn test_keepalive_intervals() {
        assert_eq!(MOBILE_KEEPALIVE_INTERVAL_SECS, 30);
        assert_eq!(BACKGROUND_KEEPALIVE_INTERVAL_SECS, 60);
    }
}
