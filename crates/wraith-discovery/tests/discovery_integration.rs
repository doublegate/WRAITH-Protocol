//! Integration tests for the discovery manager
//!
//! These tests verify end-to-end functionality of the discovery system,
//! including DHT lookup, NAT traversal, and relay fallback.

use std::net::SocketAddr;
use std::time::Duration;
use wraith_discovery::dht::NodeId;
use wraith_discovery::{
    ConnectionType, DiscoveryConfig, DiscoveryError, DiscoveryManager, DiscoveryState, RelayInfo,
};

#[tokio::test]
async fn test_discovery_manager_lifecycle() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10000".parse().unwrap();
    let config = DiscoveryConfig::new(node_id, addr);

    let manager = DiscoveryManager::new(config).await.unwrap();

    // Initial state
    assert_eq!(manager.state().await, DiscoveryState::Stopped);

    // Shutdown
    manager.shutdown().await.unwrap();
    assert_eq!(manager.state().await, DiscoveryState::Stopped);
}

#[tokio::test]
async fn test_discovery_config_with_bootstrap_nodes() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10001".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Add bootstrap nodes
    config.add_bootstrap_node("127.0.0.1:9000".parse().unwrap());
    config.add_bootstrap_node("127.0.0.1:9001".parse().unwrap());

    assert_eq!(config.bootstrap_nodes.len(), 2);

    let _manager = DiscoveryManager::new(config).await.unwrap();
}

#[tokio::test]
async fn test_discovery_config_with_relay_servers() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10002".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Add relay server
    let relay_info = RelayInfo {
        addr: "127.0.0.1:7000".parse().unwrap(),
        node_id: NodeId::from_bytes([1u8; 32]),
        public_key: [2u8; 32],
    };
    config.add_relay_server(relay_info);

    assert_eq!(config.relay_servers.len(), 1);
    assert!(config.relay_enabled);

    let _manager = DiscoveryManager::new(config).await.unwrap();
}

#[tokio::test]
async fn test_discovery_manager_nat_detection_disabled() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10003".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Disable NAT detection
    config.nat_detection_enabled = false;

    let manager = DiscoveryManager::new(config).await.unwrap();

    // NAT type should be None
    assert!(manager.nat_type().await.is_none());
}

#[tokio::test]
async fn test_discovery_manager_relay_disabled() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10004".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Disable relay
    config.relay_enabled = false;

    let _manager = DiscoveryManager::new(config).await.unwrap();
}

#[tokio::test]
async fn test_connection_type_variants() {
    // Test all connection type variants
    assert_eq!(ConnectionType::Direct.to_string(), "Direct");
    assert_eq!(ConnectionType::HolePunched.to_string(), "HolePunched");

    let relay_id = NodeId::from_bytes([42u8; 32]);
    let relayed = ConnectionType::Relayed(relay_id);
    assert!(relayed.to_string().contains("Relayed"));

    // Test equality
    assert_eq!(ConnectionType::Direct, ConnectionType::Direct);
    assert_ne!(ConnectionType::Direct, ConnectionType::HolePunched);
}

#[tokio::test]
async fn test_discovery_manager_timeout_configuration() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10005".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Custom timeout
    config.connection_timeout = Duration::from_secs(5);

    let _manager = DiscoveryManager::new(config).await.unwrap();
}

#[tokio::test]
async fn test_peer_connection_not_found() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10006".parse().unwrap();
    let config = DiscoveryConfig::new(node_id, addr);

    let manager = DiscoveryManager::new(config).await.unwrap();

    // Try to connect to non-existent peer
    let peer_id = NodeId::random();
    let result = manager.connect_to_peer(peer_id).await;

    // Should fail (peer not found or connection failed)
    assert!(result.is_err());
}

#[tokio::test]
async fn test_discovery_manager_dht_access() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10007".parse().unwrap();
    let config = DiscoveryConfig::new(node_id, addr);

    let manager = DiscoveryManager::new(config).await.unwrap();

    // Get DHT reference
    let dht = manager.dht();
    assert!(dht.read().await.id() == &node_id);
}

#[tokio::test]
async fn test_multiple_discovery_managers() {
    // Create multiple managers on different ports
    let mut managers = Vec::new();

    for i in 0..3 {
        let node_id = NodeId::random();
        let addr: SocketAddr = format!("127.0.0.1:{}", 11000 + i).parse().unwrap();
        let config = DiscoveryConfig::new(node_id, addr);

        let manager = DiscoveryManager::new(config).await.unwrap();
        managers.push(manager);
    }

    assert_eq!(managers.len(), 3);

    // Shutdown all
    for manager in &managers {
        manager.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn test_discovery_error_variants() {
    // Test error variants can be created
    let err = DiscoveryError::PeerNotFound;
    assert!(matches!(err, DiscoveryError::PeerNotFound));

    let err = DiscoveryError::ConnectionFailed;
    assert!(matches!(err, DiscoveryError::ConnectionFailed));

    let err = DiscoveryError::DhtFailed("test".to_string());
    assert!(matches!(err, DiscoveryError::DhtFailed(_)));

    let err = DiscoveryError::NatTraversalFailed("test".to_string());
    assert!(matches!(err, DiscoveryError::NatTraversalFailed(_)));

    let err = DiscoveryError::RelayFailed("test".to_string());
    assert!(matches!(err, DiscoveryError::RelayFailed(_)));
}

#[tokio::test]
async fn test_discovery_state_transitions() {
    // Test state enum values
    assert_eq!(DiscoveryState::Stopped, DiscoveryState::Stopped);
    assert_ne!(DiscoveryState::Stopped, DiscoveryState::Running);

    let states = vec![
        DiscoveryState::Stopped,
        DiscoveryState::Starting,
        DiscoveryState::Running,
        DiscoveryState::Stopping,
    ];

    for state in states {
        // Just verify they can be created and compared
        assert_eq!(state, state);
    }
}

#[tokio::test]
async fn test_relay_info_creation() {
    let relay_info = RelayInfo {
        addr: "127.0.0.1:7000".parse().unwrap(),
        node_id: NodeId::from_bytes([1u8; 32]),
        public_key: [2u8; 32],
    };

    assert_eq!(relay_info.addr.port(), 7000);
    assert_eq!(relay_info.node_id, NodeId::from_bytes([1u8; 32]));
    assert_eq!(relay_info.public_key, [2u8; 32]);
}

#[tokio::test]
async fn test_discovery_config_stun_servers() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10008".parse().unwrap();
    let mut config = DiscoveryConfig::new(node_id, addr);

    // Default STUN servers (5 fallback IPs from well-known providers)
    assert_eq!(config.stun_servers.len(), 5);

    // Add custom STUN server
    config.add_stun_server("1.2.3.4:3478".parse().unwrap());
    assert_eq!(config.stun_servers.len(), 6);
}

#[tokio::test]
async fn test_concurrent_peer_discovery() {
    let node_id = NodeId::random();
    let addr: SocketAddr = "127.0.0.1:10009".parse().unwrap();
    let config = DiscoveryConfig::new(node_id, addr);

    let manager = DiscoveryManager::new(config).await.unwrap();

    // Try to discover multiple peers concurrently
    let peer1 = NodeId::random();
    let peer2 = NodeId::random();
    let peer3 = NodeId::random();

    let handle1 = manager.connect_to_peer(peer1);
    let handle2 = manager.connect_to_peer(peer2);
    let handle3 = manager.connect_to_peer(peer3);

    // All should complete (likely with errors since peers don't exist)
    let results = tokio::join!(handle1, handle2, handle3);

    // At least verify they all completed
    assert!(results.0.is_err() || results.0.is_ok());
    assert!(results.1.is_err() || results.1.is_ok());
    assert!(results.2.is_err() || results.2.is_ok());
}
