//! Application state management

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use wraith_core::node::Node;

use crate::TransferInfo;

/// Application state shared across all Tauri commands
#[derive(Default)]
pub struct AppState {
    /// The WRAITH node instance
    pub node: Arc<RwLock<Option<Node>>>,

    /// Active transfers (for UI tracking)
    pub transfers: Arc<RwLock<HashMap<String, TransferInfo>>>,

    /// Download directory
    pub download_dir: Arc<RwLock<Option<std::path::PathBuf>>>,
}

impl AppState {
    /// Create a new app state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the node is running
    pub async fn is_node_running(&self) -> bool {
        let node = self.node.read().await;
        node.as_ref().map(|n| n.is_running()).unwrap_or(false)
    }

    /// Get the node ID as a hex string
    pub async fn get_node_id_hex(&self) -> Option<String> {
        let node = self.node.read().await;
        node.as_ref().map(|n| hex::encode(n.node_id()))
    }

    /// Get the number of active sessions
    pub async fn active_session_count(&self) -> usize {
        let node = self.node.read().await;
        if let Some(n) = node.as_ref() {
            n.active_sessions().await.len()
        } else {
            0
        }
    }

    /// Get the number of active transfers
    pub async fn active_transfer_count(&self) -> usize {
        let node = self.node.read().await;
        if let Some(n) = node.as_ref() {
            n.active_transfers().await.len()
        } else {
            0
        }
    }
}
