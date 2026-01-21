// WRAITH Android Discovery Module
//
// Provides DHT peer discovery and NAT traversal for mobile clients.
// Handles mobile-specific network characteristics like frequent IP changes,
// carrier-grade NAT, and battery optimization.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
#[allow(unused_imports)]
use wraith_discovery::dht::NodeId;
#[allow(unused_imports)]
use wraith_discovery::{
    DiscoveryConfig, DiscoveryManager, DiscoveryState, NatDetector, NatType, StunClient,
    fallback_stun_ips,
};

use crate::error::Error;

/// Maximum time before re-detecting NAT type after network change
const NAT_REDETECT_INTERVAL_SECS: u64 = 300;

/// Mobile network state for adaptive behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileNetworkType {
    /// WiFi connection
    Wifi,
    /// Cellular data (4G/5G)
    Cellular,
    /// Unknown or no connection
    Unknown,
}

/// Discovery configuration for mobile clients
#[derive(Clone)]
pub struct MobileDiscoveryConfig {
    /// Local node ID
    pub node_id: NodeId,
    /// Local listen address
    pub listen_addr: SocketAddr,
    /// Bootstrap DHT nodes
    pub bootstrap_nodes: Vec<SocketAddr>,
    /// STUN servers for NAT detection
    pub stun_servers: Vec<SocketAddr>,
}

impl Default for MobileDiscoveryConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId::random(),
            listen_addr: "0.0.0.0:0".parse().unwrap(),
            bootstrap_nodes: Vec::new(),
            stun_servers: Vec::new(),
        }
    }
}

/// Peer information for discovery results
#[derive(Clone, Debug)]
pub struct PeerInfo {
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
#[derive(Clone, Debug)]
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

/// Mobile-specific keep-alive interval (30 seconds for aggressive NAT)
#[allow(dead_code)]
pub const MOBILE_KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Background keep-alive interval when app is backgrounded (60 seconds)
#[allow(dead_code)]
pub const BACKGROUND_KEEPALIVE_INTERVAL_SECS: u64 = 60;

/// Mobile discovery client
///
/// Provides a simplified interface for mobile clients to:
/// - Discover peers via DHT
/// - Detect NAT type and external address
/// - Maintain connections through NAT
pub struct MobileDiscoveryClient {
    /// Configuration
    config: MobileDiscoveryConfig,
    /// Discovery manager (wraith-discovery)
    manager: Option<Arc<Mutex<DiscoveryManager>>>,
    /// Current NAT type
    nat_type: RwLock<Option<NatType>>,
    /// External address (from STUN)
    external_addr: RwLock<Option<SocketAddr>>,
    /// Last NAT detection time
    last_nat_detection: RwLock<Option<Instant>>,
    /// Current network type
    current_network: RwLock<MobileNetworkType>,
    /// Is app in background
    is_backgrounded: RwLock<bool>,
}

impl MobileDiscoveryClient {
    /// Create a new mobile discovery client
    pub fn new(config: MobileDiscoveryConfig) -> Self {
        Self {
            config,
            manager: None,
            nat_type: RwLock::new(None),
            external_addr: RwLock::new(None),
            last_nat_detection: RwLock::new(None),
            current_network: RwLock::new(MobileNetworkType::Unknown),
            is_backgrounded: RwLock::new(false),
        }
    }

    /// Initialize and start the discovery service
    ///
    /// This will:
    /// 1. Create the discovery manager
    /// 2. Detect NAT type
    /// 3. Bootstrap into the DHT
    pub async fn start(&mut self) -> Result<(), Error> {
        // Build discovery config
        let mut disc_config = DiscoveryConfig::new(self.config.node_id, self.config.listen_addr);

        for node in &self.config.bootstrap_nodes {
            disc_config.add_bootstrap_node(*node);
        }

        for server in &self.config.stun_servers {
            disc_config.add_stun_server(*server);
        }

        // Create discovery manager
        let manager = DiscoveryManager::new(disc_config)
            .await
            .map_err(|e| Error::Other(format!("Failed to create discovery manager: {}", e)))?;

        // Start the manager
        manager
            .start()
            .await
            .map_err(|e| Error::Other(format!("Failed to start discovery: {}", e)))?;

        // Get initial NAT type
        if let Some(nat_type) = manager.nat_type().await {
            *self.nat_type.write().await = Some(nat_type);
        }

        *self.last_nat_detection.write().await = Some(Instant::now());
        self.manager = Some(Arc::new(Mutex::new(manager)));

        log::info!("Mobile discovery service started");
        Ok(())
    }

    /// Stop the discovery service
    pub async fn stop(&mut self) -> Result<(), Error> {
        if let Some(manager) = self.manager.take() {
            let manager = manager.lock().await;
            manager
                .shutdown()
                .await
                .map_err(|e| Error::Other(format!("Failed to shutdown discovery: {}", e)))?;
        }

        log::info!("Mobile discovery service stopped");
        Ok(())
    }

    /// Discover a peer by node ID
    ///
    /// Performs DHT lookup to find peer endpoints.
    pub async fn discover_peer(&self, peer_id_hex: &str) -> Result<PeerInfo, Error> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| Error::Other("Discovery not started".to_string()))?;

        // Parse peer ID
        let peer_id_bytes = hex::decode(peer_id_hex)
            .map_err(|e| Error::Other(format!("Invalid peer ID hex: {}", e)))?;
        let peer_id_array: [u8; 32] = peer_id_bytes
            .try_into()
            .map_err(|_| Error::Other("Peer ID must be 32 bytes".to_string()))?;
        let peer_id = NodeId::from_bytes(peer_id_array);

        // Attempt connection via discovery manager
        let manager = manager.lock().await;
        match manager.connect_to_peer(peer_id).await {
            Ok(connection) => Ok(PeerInfo {
                peer_id: peer_id_hex.to_string(),
                address: connection.addr.to_string(),
                connection_type: connection.connection_type.to_string(),
                last_seen: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            }),
            Err(e) => Err(Error::Other(format!("Peer discovery failed: {}", e))),
        }
    }

    /// Detect NAT type and get external address
    ///
    /// Uses STUN to detect NAT type and discover external address.
    /// Results are cached and reused unless network changes.
    pub async fn detect_nat(&self) -> Result<NatInfo, Error> {
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
            self.refresh_nat_info().await?;
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

    /// Refresh NAT information (forced re-detection)
    pub async fn refresh_nat_info(&self) -> Result<(), Error> {
        // Get STUN servers from config or use defaults
        let stun_servers = if self.config.stun_servers.is_empty() {
            wraith_discovery::fallback_stun_ips()
        } else {
            self.config.stun_servers.clone()
        };

        if stun_servers.is_empty() {
            return Err(Error::Other("No STUN servers available".to_string()));
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

    /// Notify that the network has changed
    ///
    /// Call this when Android detects network connectivity changes.
    /// This will trigger re-detection of NAT type and external address.
    pub async fn on_network_changed(&self, network_type: MobileNetworkType) {
        let previous = *self.current_network.read().await;
        if previous != network_type {
            *self.current_network.write().await = network_type;
            log::info!("Network changed from {:?} to {:?}", previous, network_type);

            // Trigger NAT re-detection
            if let Err(e) = self.refresh_nat_info().await {
                log::warn!("Failed to refresh NAT info after network change: {}", e);
            }
        }
    }

    /// Notify that the app has been backgrounded
    ///
    /// Adjusts keep-alive intervals to be more battery-friendly.
    pub async fn on_app_backgrounded(&self) {
        *self.is_backgrounded.write().await = true;
        log::info!("App backgrounded - switching to battery-saving mode");
    }

    /// Notify that the app has been foregrounded
    ///
    /// Restores normal keep-alive intervals.
    pub async fn on_app_foregrounded(&self) {
        *self.is_backgrounded.write().await = false;
        log::info!("App foregrounded - restoring normal operation");

        // Refresh NAT info if needed
        let should_refresh = {
            let last = self.last_nat_detection.read().await;
            match *last {
                None => true,
                Some(instant) => instant.elapsed() > Duration::from_secs(60),
            }
        };

        if should_refresh {
            if let Err(e) = self.refresh_nat_info().await {
                log::warn!("Failed to refresh NAT info after foreground: {}", e);
            }
        }
    }

    /// Get the current keep-alive interval based on app state
    #[allow(dead_code)]
    pub async fn get_keepalive_interval(&self) -> Duration {
        let is_backgrounded = *self.is_backgrounded.read().await;
        if is_backgrounded {
            Duration::from_secs(BACKGROUND_KEEPALIVE_INTERVAL_SECS)
        } else {
            Duration::from_secs(MOBILE_KEEPALIVE_INTERVAL_SECS)
        }
    }

    /// Get cached NAT type (without re-detection)
    #[allow(dead_code)]
    pub async fn get_cached_nat_type(&self) -> Option<String> {
        self.nat_type.read().await.map(|t| format!("{}", t))
    }

    /// Get cached external address (without re-detection)
    #[allow(dead_code)]
    pub async fn get_cached_external_addr(&self) -> Option<String> {
        self.external_addr.read().await.map(|a| a.to_string())
    }

    /// Check if hole punching is likely to work with the given NAT type
    fn is_hole_punch_capable(nat_type: NatType) -> bool {
        matches!(
            nat_type,
            NatType::Open
                | NatType::FullCone
                | NatType::RestrictedCone
                | NatType::PortRestrictedCone
        )
    }

    /// Get the discovery manager state
    pub async fn get_state(&self) -> String {
        match &self.manager {
            Some(manager) => {
                let manager = manager.lock().await;
                match manager.state().await {
                    DiscoveryState::Stopped => "stopped".to_string(),
                    DiscoveryState::Starting => "starting".to_string(),
                    DiscoveryState::Running => "running".to_string(),
                    DiscoveryState::Stopping => "stopping".to_string(),
                }
            }
            None => "not_initialized".to_string(),
        }
    }
}

/// Global discovery client instance
static DISCOVERY_CLIENT: Mutex<Option<MobileDiscoveryClient>> = Mutex::const_new(None);

/// Initialize the global discovery client
pub async fn init_discovery(config: MobileDiscoveryConfig) -> Result<(), Error> {
    let mut client = MobileDiscoveryClient::new(config);
    client.start().await?;
    *DISCOVERY_CLIENT.lock().await = Some(client);
    Ok(())
}

/// Shutdown the global discovery client
pub async fn shutdown_discovery() -> Result<(), Error> {
    let mut guard = DISCOVERY_CLIENT.lock().await;
    if let Some(ref mut client) = *guard {
        client.stop().await?;
    }
    *guard = None;
    Ok(())
}

/// Discover a peer by ID using the global client
pub async fn discover_peer(peer_id_hex: &str) -> Result<PeerInfo, Error> {
    let guard = DISCOVERY_CLIENT.lock().await;
    match guard.as_ref() {
        Some(client) => client.discover_peer(peer_id_hex).await,
        None => Err(Error::Other("Discovery not initialized".to_string())),
    }
}

/// Detect NAT using the global client
pub async fn detect_nat() -> Result<NatInfo, Error> {
    let guard = DISCOVERY_CLIENT.lock().await;
    match guard.as_ref() {
        Some(client) => client.detect_nat().await,
        None => Err(Error::Other("Discovery not initialized".to_string())),
    }
}

/// Notify network change to the global client
pub async fn on_network_changed(network_type: MobileNetworkType) {
    let guard = DISCOVERY_CLIENT.lock().await;
    if let Some(client) = guard.as_ref() {
        client.on_network_changed(network_type).await;
    }
}

/// Notify app backgrounded to the global client
pub async fn on_app_backgrounded() {
    let guard = DISCOVERY_CLIENT.lock().await;
    if let Some(client) = guard.as_ref() {
        client.on_app_backgrounded().await;
    }
}

/// Notify app foregrounded to the global client
pub async fn on_app_foregrounded() {
    let guard = DISCOVERY_CLIENT.lock().await;
    if let Some(client) = guard.as_ref() {
        client.on_app_foregrounded().await;
    }
}

/// Get discovery state from the global client
pub async fn get_discovery_state() -> String {
    let guard = DISCOVERY_CLIENT.lock().await;
    match guard.as_ref() {
        Some(client) => client.get_state().await,
        None => "not_initialized".to_string(),
    }
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
        assert!(config.bootstrap_nodes.is_empty());
        assert!(config.stun_servers.is_empty());
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
    fn test_peer_info_creation() {
        let info = PeerInfo {
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

    #[tokio::test]
    async fn test_mobile_discovery_client_creation() {
        let config = MobileDiscoveryConfig::default();
        let client = MobileDiscoveryClient::new(config);
        assert_eq!(client.get_state().await, "not_initialized");
    }

    #[tokio::test]
    async fn test_keepalive_interval_default() {
        let config = MobileDiscoveryConfig::default();
        let client = MobileDiscoveryClient::new(config);

        let interval = client.get_keepalive_interval().await;
        assert_eq!(
            interval,
            Duration::from_secs(MOBILE_KEEPALIVE_INTERVAL_SECS)
        );
    }

    #[tokio::test]
    async fn test_keepalive_interval_backgrounded() {
        let config = MobileDiscoveryConfig::default();
        let client = MobileDiscoveryClient::new(config);

        client.on_app_backgrounded().await;
        let interval = client.get_keepalive_interval().await;
        assert_eq!(
            interval,
            Duration::from_secs(BACKGROUND_KEEPALIVE_INTERVAL_SECS)
        );
    }

    #[tokio::test]
    async fn test_keepalive_intervals_constants() {
        // Verify constants are set correctly for mobile networks
        assert_eq!(MOBILE_KEEPALIVE_INTERVAL_SECS, 30);
        assert_eq!(BACKGROUND_KEEPALIVE_INTERVAL_SECS, 60);
    }

    #[tokio::test]
    async fn test_network_change_tracking() {
        let config = MobileDiscoveryConfig::default();
        let client = MobileDiscoveryClient::new(config);

        // Initial state
        assert_eq!(
            *client.current_network.read().await,
            MobileNetworkType::Unknown
        );

        // Update doesn't fail even without manager
        client.on_network_changed(MobileNetworkType::Wifi).await;
        assert_eq!(
            *client.current_network.read().await,
            MobileNetworkType::Wifi
        );

        client.on_network_changed(MobileNetworkType::Cellular).await;
        assert_eq!(
            *client.current_network.read().await,
            MobileNetworkType::Cellular
        );
    }
}
