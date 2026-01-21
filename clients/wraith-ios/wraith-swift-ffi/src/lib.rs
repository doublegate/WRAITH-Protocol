// WRAITH iOS UniFFI Bindings
//
// This library provides Swift bindings for the WRAITH protocol using UniFFI.
// UniFFI automatically generates Swift code from the .udl interface definition.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use wraith_core::node::{Node as CoreNode, NodeConfig as CoreNodeConfig};

// Include generated UniFFI scaffolding
uniffi::include_scaffolding!("wraith");

mod error;
use error::{WraithError, Result};

/// Global Tokio runtime for async operations
static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// Global counter for active transfers
static ACTIVE_TRANSFERS: AtomicU32 = AtomicU32::new(0);

/// Increment active transfer count
fn increment_transfers() {
    ACTIVE_TRANSFERS.fetch_add(1, Ordering::SeqCst);
}

/// Decrement active transfer count
#[allow(dead_code)]
fn decrement_transfers() {
    ACTIVE_TRANSFERS.fetch_sub(1, Ordering::SeqCst);
}

/// Get current active transfer count
fn get_active_transfers() -> u32 {
    ACTIVE_TRANSFERS.load(Ordering::SeqCst)
}

/// Initialize the global runtime
fn get_or_create_runtime() -> std::result::Result<Arc<Runtime>, WraithError> {
    let mut rt_lock = RUNTIME.lock().map_err(|e| WraithError::InitializationFailed {
        message: format!("Failed to acquire runtime lock: {}", e),
    })?;
    if rt_lock.is_none() {
        let rt = Runtime::new().map_err(|e| WraithError::InitializationFailed {
            message: format!("Failed to create Tokio runtime: {}", e),
        })?;
        *rt_lock = Some(rt);
    }
    let handle = rt_lock.as_ref().ok_or(WraithError::InitializationFailed {
        message: "Runtime not initialized".to_string(),
    })?.handle().clone();
    Ok(Arc::new(handle.into()))
}

/// Node configuration
#[derive(Clone, uniffi::Record)]
pub struct NodeConfig {
    pub max_sessions: u32,
    pub max_transfers: u32,
    pub buffer_size: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            max_sessions: 100,
            max_transfers: 10,
            buffer_size: 65536,
        }
    }
}

impl From<NodeConfig> for CoreNodeConfig {
    fn from(config: NodeConfig) -> Self {
        CoreNodeConfig {
            max_sessions: config.max_sessions as usize,
            max_transfers: config.max_transfers as usize,
            buffer_size: config.buffer_size as usize,
        }
    }
}

/// Session information
#[derive(Clone, uniffi::Record)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_id: String,
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

/// Node status
#[derive(Clone, uniffi::Record)]
pub struct NodeStatus {
    pub running: bool,
    pub local_peer_id: String,
    pub session_count: u32,
    pub active_transfers: u32,
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
    pub fn new(_config: NodeConfig) -> Result<Self> {
        let runtime = get_or_create_runtime()?;

        Ok(Self {
            inner: Arc::new(Mutex::new(None)),
            runtime,
        })
    }

    /// Start the WRAITH node
    pub fn start(&self, listen_addr: String) -> Result<()> {
        let config = CoreNodeConfig::default();

        let node = self.runtime.block_on(async {
            CoreNode::new(config).await
                .map_err(|e| WraithError::InitializationFailed {
                    message: e.to_string(),
                })
        })?;

        let addr = listen_addr.parse()
            .map_err(|e| WraithError::InitializationFailed {
                message: format!("Invalid listen address: {}", e),
            })?;

        self.runtime.block_on(async {
            node.start_listening(addr).await
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
                node.shutdown().await
                    .map_err(|e| WraithError::Other {
                        message: e.to_string(),
                    })
            })?;
        }
        Ok(())
    }

    /// Establish a session with a remote peer
    pub fn establish_session(&self, peer_id: String) -> Result<SessionInfo> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;
        let node = inner.as_ref()
            .ok_or(WraithError::NotStarted {
                message: "Node not started".to_string(),
            })?;

        let peer_id_bytes = hex::decode(&peer_id)
            .map_err(|e| WraithError::InvalidPeerId {
                message: format!("Invalid peer ID hex: {}", e),
            })?;

        let peer_id_array: [u8; 32] = peer_id_bytes.try_into()
            .map_err(|_| WraithError::InvalidPeerId {
                message: "Peer ID must be 32 bytes".to_string(),
            })?;

        let session = self.runtime.block_on(async {
            node.establish_session(peer_id_array).await
                .map_err(|e| WraithError::SessionFailed {
                    message: e.to_string(),
                })
        })?;

        Ok(SessionInfo {
            session_id: hex::encode(session.id()),
            peer_id: hex::encode(peer_id_array),
            connected: true,
        })
    }

    /// Send a file to a peer
    pub fn send_file(&self, peer_id: String, file_path: String) -> Result<TransferInfo> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;
        let node = inner.as_ref()
            .ok_or(WraithError::NotStarted {
                message: "Node not started".to_string(),
            })?;

        let peer_id_bytes = hex::decode(&peer_id)
            .map_err(|e| WraithError::InvalidPeerId {
                message: format!("Invalid peer ID hex: {}", e),
            })?;

        let peer_id_array: [u8; 32] = peer_id_bytes.try_into()
            .map_err(|_| WraithError::InvalidPeerId {
                message: "Peer ID must be 32 bytes".to_string(),
            })?;

        use std::path::Path;
        use wraith_files::{FileTransfer, TransferConfig};

        // Track transfer start
        increment_transfers();

        let transfer_id = match self.runtime.block_on(async {
            let config = TransferConfig::default();
            let transfer = FileTransfer::new(config);

            transfer.send_file(
                node.as_ref(),
                peer_id_array,
                Path::new(&file_path),
            ).await
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

        // Get actual file size from filesystem
        let file_size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(TransferInfo {
            transfer_id: hex::encode(transfer_id),
            peer_id: hex::encode(peer_id_array),
            file_path,
            file_size,
            bytes_transferred: 0,
            status: TransferStatus::Sending,
        })
    }

    /// Get the current node status
    pub fn get_status(&self) -> Result<NodeStatus> {
        let inner = self.inner.lock().map_err(|e| WraithError::Other {
            message: format!("Failed to acquire node lock: {}", e),
        })?;
        let node = inner.as_ref()
            .ok_or(WraithError::NotStarted {
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
        inner.as_ref()
            .map(|n| hex::encode(n.local_peer_id()))
            .unwrap_or_default()
    }
}

/// Create a new WRAITH node (convenience function)
#[uniffi::export]
pub fn create_node(listen_addr: String, config: NodeConfig) -> Result<Arc<WraithNode>> {
    let node = WraithNode::new(config)?;
    node.start(listen_addr)?;
    Ok(Arc::new(node))
}
