// WRAITH iOS UniFFI Bindings
//
// This library provides Swift bindings for the WRAITH protocol using UniFFI.
// Uses proc-macro based binding generation for cleaner code.

// Note: The function pointer comparison warning is a known UniFFI issue in the
// setup_scaffolding! macro and can be safely ignored.
#![allow(unpredictable_function_pointer_comparisons)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Runtime;
use wraith_core::node::{Node as CoreNode, NodeConfig as CoreNodeConfig};

// Setup UniFFI scaffolding using proc macros
uniffi::setup_scaffolding!();

mod discovery;
mod error;
#[cfg(test)]
mod integration_tests;
mod keychain;
mod push;

use error::{Result, WraithError};

// Re-export discovery types for Swift
pub use discovery::{
    DiscoveryPeerInfo, DiscoveryStatus, MobileDiscoveryClient, MobileDiscoveryConfig,
    MobileNetworkType, NatInfo, create_discovery_client, create_discovery_client_with_config,
};

// Re-export keychain types for Swift
pub use keychain::{
    KeychainError, KeychainKeyInfo, SecureKeyStorage, create_secure_storage,
    device_has_secure_enclave,
};

// Re-export push notification types for Swift
pub use push::{
    NotificationContent, PushAction, PushError, PushNotificationManager, PushPlatform,
    PushSettings, PushToken, create_push_manager, format_notification, get_push_settings,
    get_stored_push_token, handle_push_notification, process_background_push, register_push_token,
    unregister_push_token, update_push_settings,
};

/// Global Tokio runtime for async operations
static RUNTIME: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);

/// Global counter for active transfers
static ACTIVE_TRANSFERS: AtomicU32 = AtomicU32::new(0);

/// Transfer state for tracking
#[derive(Clone)]
#[allow(dead_code)]
struct TransferState {
    peer_id: [u8; 32],
    file_path: String,
    total_bytes: u64,
    bytes_transferred: u64,
    is_complete: bool,
    is_cancelled: bool,
}

/// Transfer tracking map
static TRANSFER_MAP: RwLock<Option<HashMap<[u8; 32], TransferState>>> = RwLock::new(None);

/// Increment active transfer count
fn increment_transfers() {
    ACTIVE_TRANSFERS.fetch_add(1, Ordering::SeqCst);
}

/// Decrement active transfer count
fn decrement_transfers() {
    let current = ACTIVE_TRANSFERS.load(Ordering::SeqCst);
    if current > 0 {
        ACTIVE_TRANSFERS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Get current active transfer count
fn get_active_transfers() -> u32 {
    ACTIVE_TRANSFERS.load(Ordering::SeqCst)
}

/// Initialize transfer map
fn init_transfer_map() {
    if let Ok(mut map) = TRANSFER_MAP.write() {
        if map.is_none() {
            *map = Some(HashMap::new());
        }
    }
}

/// Get or create the global runtime
fn get_or_create_runtime() -> Result<Arc<Runtime>> {
    let mut rt_lock = RUNTIME
        .lock()
        .map_err(|e| WraithError::InitializationFailed {
            message: format!("Failed to acquire runtime lock: {}", e),
        })?;

    if rt_lock.is_none() {
        let rt = Runtime::new().map_err(|e| WraithError::InitializationFailed {
            message: format!("Failed to create Tokio runtime: {}", e),
        })?;
        *rt_lock = Some(Arc::new(rt));
    }

    rt_lock
        .clone()
        .ok_or_else(|| WraithError::InitializationFailed {
            message: "Runtime not available".to_string(),
        })
}

/// Node configuration
#[derive(Clone, uniffi::Record)]
pub struct NodeConfig {
    pub listen_addr: String,
    pub max_sessions: u32,
    pub max_transfers: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:0".to_string(),
            max_sessions: 100,
            max_transfers: 10,
        }
    }
}

impl From<&NodeConfig> for CoreNodeConfig {
    fn from(config: &NodeConfig) -> Self {
        let mut core_config = CoreNodeConfig::default();
        if let Ok(addr) = config.listen_addr.parse() {
            core_config.listen_addr = addr;
        }
        core_config
    }
}

/// Session information
#[derive(Clone, uniffi::Record)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_id: String,
    pub peer_addr: String,
    pub connected: bool,
}

/// Transfer status
#[derive(Clone, uniffi::Enum)]
pub enum TransferStatus {
    Pending,
    Sending,
    Receiving,
    Completed,
    Failed,
    Cancelled,
}

/// Transfer information
#[derive(Clone, uniffi::Record)]
pub struct TransferInfo {
    pub transfer_id: String,
    pub peer_id: String,
    pub file_path: String,
    pub file_size: u64,
    pub bytes_transferred: u64,
    pub status: TransferStatus,
}

/// Transfer progress
#[derive(Clone, uniffi::Record)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub total_bytes: u64,
    pub bytes_transferred: u64,
    pub progress_percent: f64,
    pub speed_bytes_per_sec: f64,
    pub eta_seconds: u64,
    pub is_complete: bool,
}

/// Node status
#[derive(Clone, uniffi::Record)]
pub struct NodeStatus {
    pub running: bool,
    pub local_peer_id: String,
    pub session_count: u32,
    pub active_transfers: u32,
}

/// Session statistics
#[derive(Clone, uniffi::Record)]
pub struct SessionStats {
    pub peer_id: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub rtt_us: u64,
    pub loss_rate: f64,
}

/// Main WRAITH node interface
#[derive(uniffi::Object)]
pub struct WraithNode {
    inner: Arc<Mutex<Option<Arc<CoreNode>>>>,
    runtime: Arc<Runtime>,
}

#[uniffi::export]
impl WraithNode {
    /// Create a new WRAITH node with the given configuration
    #[uniffi::constructor]
    pub fn new(_config: NodeConfig) -> Result<Arc<Self>> {
        init_transfer_map();
        let runtime = get_or_create_runtime()?;

        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(None)),
            runtime,
        }))
    }

    /// Start the WRAITH node
    pub fn start(&self, listen_addr: String) -> Result<()> {
        let addr: std::net::SocketAddr =
            listen_addr
                .parse()
                .map_err(|e| WraithError::InitializationFailed {
                    message: format!("Invalid listen address: {}", e),
                })?;

        let config = CoreNodeConfig {
            listen_addr: addr,
            ..CoreNodeConfig::default()
        };

        let node = self.runtime.block_on(async {
            CoreNode::new_with_config(config)
                .await
                .map_err(|e| WraithError::InitializationFailed {
                    message: e.to_string(),
                })
        })?;

        self.runtime.block_on(async {
            node.start()
                .await
                .map_err(|e| WraithError::InitializationFailed {
                    message: e.to_string(),
                })
        })?;

        let mut inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;
        *inner = Some(Arc::new(node));

        Ok(())
    }

    /// Shutdown the WRAITH node
    pub fn shutdown(&self) -> Result<()> {
        let mut inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        if let Some(node) = inner.take() {
            self.runtime.block_on(async {
                node.stop().await.map_err(|e| WraithError::Other {
                    message: e.to_string(),
                })
            })?;
        }

        // Clear transfer map
        if let Ok(mut map) = TRANSFER_MAP.write() {
            *map = None;
        }

        Ok(())
    }

    /// Establish a session with a remote peer
    pub fn establish_session(&self, peer_id: String, peer_addr: String) -> Result<SessionInfo> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let peer_id_bytes = hex::decode(&peer_id).map_err(|e| WraithError::InvalidPeerId {
            message: format!("Invalid peer ID hex: {}", e),
        })?;

        let peer_id_array: [u8; 32] =
            peer_id_bytes
                .try_into()
                .map_err(|_| WraithError::InvalidPeerId {
                    message: "Peer ID must be 32 bytes".to_string(),
                })?;

        let peer_socket_addr: std::net::SocketAddr =
            peer_addr.parse().map_err(|e| WraithError::InvalidPeerId {
                message: format!("Invalid peer address: {}", e),
            })?;

        let session_id = self.runtime.block_on(async {
            node.establish_session_with_addr(&peer_id_array, peer_socket_addr)
                .await
                .map_err(|e| WraithError::SessionFailed {
                    message: e.to_string(),
                })
        })?;

        Ok(SessionInfo {
            session_id: hex::encode(session_id),
            peer_id: hex::encode(peer_id_array),
            peer_addr,
            connected: true,
        })
    }

    /// Close a session with a peer
    pub fn close_session(&self, peer_id: String) -> Result<()> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let peer_id_bytes = hex::decode(&peer_id).map_err(|e| WraithError::InvalidPeerId {
            message: format!("Invalid peer ID hex: {}", e),
        })?;

        let peer_id_array: [u8; 32] =
            peer_id_bytes
                .try_into()
                .map_err(|_| WraithError::InvalidPeerId {
                    message: "Peer ID must be 32 bytes".to_string(),
                })?;

        self.runtime.block_on(async {
            node.close_session(&peer_id_array)
                .await
                .map_err(|e| WraithError::SessionFailed {
                    message: e.to_string(),
                })
        })?;

        Ok(())
    }

    /// Send a file to a peer
    pub fn send_file(&self, peer_id: String, file_path: String) -> Result<TransferInfo> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let peer_id_bytes = hex::decode(&peer_id).map_err(|e| WraithError::InvalidPeerId {
            message: format!("Invalid peer ID hex: {}", e),
        })?;

        let peer_id_array: [u8; 32] =
            peer_id_bytes
                .try_into()
                .map_err(|_| WraithError::InvalidPeerId {
                    message: "Peer ID must be 32 bytes".to_string(),
                })?;

        let path = PathBuf::from(&file_path);

        // Get actual file size from filesystem
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Track transfer start
        increment_transfers();

        let transfer_id = match self.runtime.block_on(async {
            node.send_file(path.clone(), &peer_id_array)
                .await
                .map_err(|e| WraithError::TransferFailed {
                    message: e.to_string(),
                })
        }) {
            Ok(id) => id,
            Err(e) => {
                decrement_transfers(); // Decrement on failure
                return Err(e);
            }
        };

        // Track the transfer
        if let Ok(mut map) = TRANSFER_MAP.write() {
            if let Some(ref mut m) = *map {
                m.insert(
                    transfer_id,
                    TransferState {
                        peer_id: peer_id_array,
                        file_path: file_path.clone(),
                        total_bytes: file_size,
                        bytes_transferred: 0,
                        is_complete: false,
                        is_cancelled: false,
                    },
                );
            }
        }

        Ok(TransferInfo {
            transfer_id: hex::encode(transfer_id),
            peer_id: hex::encode(peer_id_array),
            file_path,
            file_size,
            bytes_transferred: 0,
            status: TransferStatus::Sending,
        })
    }

    /// Get transfer progress
    pub fn get_transfer_progress(&self, transfer_id: String) -> Result<TransferProgress> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let transfer_id_bytes =
            hex::decode(&transfer_id).map_err(|e| WraithError::TransferFailed {
                message: format!("Invalid transfer ID hex: {}", e),
            })?;

        let transfer_id_array: [u8; 32] =
            transfer_id_bytes
                .try_into()
                .map_err(|_| WraithError::TransferFailed {
                    message: "Transfer ID must be 32 bytes".to_string(),
                })?;

        let progress = self
            .runtime
            .block_on(async { node.get_transfer_progress(&transfer_id_array).await });

        match progress {
            Some(p) => Ok(TransferProgress {
                transfer_id,
                total_bytes: p.bytes_total,
                bytes_transferred: p.bytes_sent,
                progress_percent: p.progress_percent,
                speed_bytes_per_sec: p.speed_bytes_per_sec,
                eta_seconds: p.eta.map(|d| d.as_secs()).unwrap_or(0),
                is_complete: p.is_complete(),
            }),
            None => Err(WraithError::TransferFailed {
                message: "Transfer not found".to_string(),
            }),
        }
    }

    /// Cancel an active transfer
    pub fn cancel_transfer(&self, transfer_id: String) -> Result<()> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let transfer_id_bytes =
            hex::decode(&transfer_id).map_err(|e| WraithError::TransferFailed {
                message: format!("Invalid transfer ID hex: {}", e),
            })?;

        let transfer_id_array: [u8; 32] =
            transfer_id_bytes
                .try_into()
                .map_err(|_| WraithError::TransferFailed {
                    message: "Transfer ID must be 32 bytes".to_string(),
                })?;

        self.runtime.block_on(async {
            node.cancel_transfer(&transfer_id_array).await.map_err(|e| {
                WraithError::TransferFailed {
                    message: e.to_string(),
                }
            })
        })?;

        // Update tracking
        if let Ok(mut map) = TRANSFER_MAP.write() {
            if let Some(ref mut m) = *map {
                if let Some(state) = m.get_mut(&transfer_id_array) {
                    state.is_cancelled = true;
                }
            }
        }

        decrement_transfers();

        Ok(())
    }

    /// Get session statistics
    pub fn get_session_stats(&self, peer_id: String) -> Result<SessionStats> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        let peer_id_bytes = hex::decode(&peer_id).map_err(|e| WraithError::InvalidPeerId {
            message: format!("Invalid peer ID hex: {}", e),
        })?;

        let peer_id_array: [u8; 32] =
            peer_id_bytes
                .try_into()
                .map_err(|_| WraithError::InvalidPeerId {
                    message: "Peer ID must be 32 bytes".to_string(),
                })?;

        let stats =
            node.get_connection_stats(&peer_id_array)
                .ok_or(WraithError::SessionFailed {
                    message: "Session not found".to_string(),
                })?;

        Ok(SessionStats {
            peer_id,
            bytes_sent: stats.bytes_sent,
            bytes_received: stats.bytes_received,
            packets_sent: stats.packets_sent,
            packets_received: stats.packets_received,
            rtt_us: stats.rtt_us.unwrap_or(0),
            loss_rate: stats.loss_rate,
        })
    }

    /// Get the current node status
    pub fn get_status(&self) -> Result<NodeStatus> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;

        let node = inner.as_ref().ok_or(WraithError::NotStarted {
            message: "Node not started".to_string(),
        })?;

        Ok(NodeStatus {
            running: node.is_running(),
            local_peer_id: hex::encode(node.node_id()),
            session_count: node.active_route_count() as u32,
            active_transfers: get_active_transfers(),
        })
    }

    /// Check if the node is running
    pub fn is_running(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        inner.as_ref().is_some_and(|n| n.is_running())
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> String {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return String::new(),
        };
        inner
            .as_ref()
            .map(|n| hex::encode(n.node_id()))
            .unwrap_or_default()
    }
}

/// Create a new WRAITH node (convenience function)
#[uniffi::export]
pub fn create_node(listen_addr: String, config: NodeConfig) -> Result<Arc<WraithNode>> {
    let node = WraithNode::new(config)?;
    node.start(listen_addr)?;
    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_tracking() {
        init_transfer_map();

        // Reset counter for test isolation
        ACTIVE_TRANSFERS.store(0, Ordering::SeqCst);

        assert_eq!(get_active_transfers(), 0);
        increment_transfers();
        assert_eq!(get_active_transfers(), 1);
        increment_transfers();
        assert_eq!(get_active_transfers(), 2);
        decrement_transfers();
        assert_eq!(get_active_transfers(), 1);
        decrement_transfers();
        assert_eq!(get_active_transfers(), 0);
        // Should not underflow
        decrement_transfers();
        assert_eq!(get_active_transfers(), 0);
    }

    #[test]
    fn test_runtime_creation() {
        let rt = get_or_create_runtime();
        assert!(rt.is_ok());

        // Second call should return the same runtime
        let rt2 = get_or_create_runtime();
        assert!(rt2.is_ok());
    }

    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        assert_eq!(config.listen_addr, "0.0.0.0:0");
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.max_transfers, 10);
    }

    #[test]
    fn test_transfer_map_initialization() {
        init_transfer_map();

        // Verify map is initialized
        let map = TRANSFER_MAP.read().expect("Should read transfer map");
        assert!(map.is_some());
    }

    #[test]
    fn test_transfer_state_creation() {
        let state = TransferState {
            peer_id: [1u8; 32],
            file_path: "/test/file.txt".to_string(),
            total_bytes: 1000,
            bytes_transferred: 500,
            is_complete: false,
            is_cancelled: false,
        };

        assert_eq!(state.total_bytes, 1000);
        assert_eq!(state.bytes_transferred, 500);
        assert!(!state.is_complete);
        assert!(!state.is_cancelled);
    }

    #[test]
    fn test_session_info_creation() {
        let session_info = SessionInfo {
            session_id: "abc123".to_string(),
            peer_id: "def456".to_string(),
            peer_addr: "127.0.0.1:8420".to_string(),
            connected: true,
        };

        assert_eq!(session_info.session_id, "abc123");
        assert!(session_info.connected);
    }

    #[test]
    fn test_transfer_info_creation() {
        let transfer_info = TransferInfo {
            transfer_id: "transfer123".to_string(),
            peer_id: "peer456".to_string(),
            file_path: "/test/file.txt".to_string(),
            file_size: 1024,
            bytes_transferred: 512,
            status: TransferStatus::Sending,
        };

        assert_eq!(transfer_info.file_size, 1024);
        assert_eq!(transfer_info.bytes_transferred, 512);
    }

    #[test]
    fn test_node_status_creation() {
        let status = NodeStatus {
            running: true,
            local_peer_id: "local123".to_string(),
            session_count: 5,
            active_transfers: 2,
        };

        assert!(status.running);
        assert_eq!(status.session_count, 5);
        assert_eq!(status.active_transfers, 2);
    }

    #[test]
    fn test_session_stats_creation() {
        let stats = SessionStats {
            peer_id: "peer123".to_string(),
            bytes_sent: 1000,
            bytes_received: 500,
            packets_sent: 100,
            packets_received: 50,
            rtt_us: 25000,
            loss_rate: 0.01,
        };

        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.bytes_received, 500);
        assert_eq!(stats.rtt_us, 25000);
    }

    #[test]
    fn test_peer_id_hex_encoding() {
        let peer_id: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];

        let encoded = hex::encode(peer_id);
        assert_eq!(encoded.len(), 64);

        let decoded = hex::decode(&encoded).expect("Should decode");
        assert_eq!(decoded, peer_id);
    }

    #[test]
    fn test_invalid_peer_id_hex() {
        let invalid_hex = "not_valid_hex";
        let result = hex::decode(invalid_hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_id_wrong_length() {
        let short_hex = "0102030405"; // Only 5 bytes
        let decoded = hex::decode(short_hex).expect("Should decode hex");
        let result: std::result::Result<[u8; 32], _> = decoded.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_address_parsing() {
        let valid_addr = "127.0.0.1:8420";
        let parsed: std::result::Result<std::net::SocketAddr, _> = valid_addr.parse();
        assert!(parsed.is_ok());

        let invalid_addr = "not_an_address";
        let parsed: std::result::Result<std::net::SocketAddr, _> = invalid_addr.parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_transfer_progress_creation() {
        let progress = TransferProgress {
            transfer_id: "transfer123".to_string(),
            total_bytes: 1000,
            bytes_transferred: 500,
            progress_percent: 50.0,
            speed_bytes_per_sec: 100.0,
            eta_seconds: 5,
            is_complete: false,
        };

        assert_eq!(progress.progress_percent, 50.0);
        assert!(!progress.is_complete);
    }

    #[test]
    fn test_error_variants() {
        let err1 = WraithError::InitializationFailed {
            message: "test".to_string(),
        };
        assert!(err1.to_string().contains("test"));

        let err2 = WraithError::SessionFailed {
            message: "session error".to_string(),
        };
        assert!(err2.to_string().contains("session"));

        let err3 = WraithError::TransferFailed {
            message: "transfer error".to_string(),
        };
        assert!(err3.to_string().contains("transfer"));

        let err4 = WraithError::NotStarted {
            message: "not started".to_string(),
        };
        assert!(err4.to_string().contains("started"));

        let err5 = WraithError::InvalidPeerId {
            message: "invalid peer".to_string(),
        };
        assert!(err5.to_string().contains("peer"));
    }

    #[test]
    fn test_concurrent_atomic_operations() {
        use std::sync::atomic::AtomicU32;
        use std::thread;

        // Test that AtomicU32 operations are thread-safe using an isolated counter
        // We don't use the global ACTIVE_TRANSFERS here to avoid race conditions
        // with other tests running in parallel
        let test_counter = std::sync::Arc::new(AtomicU32::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = test_counter.clone();
            handles.push(thread::spawn(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all 10 increments happened atomically
        assert_eq!(test_counter.load(Ordering::SeqCst), 10);
    }
}
