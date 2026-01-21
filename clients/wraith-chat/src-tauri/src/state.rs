// Application State Management

use crate::crypto::DoubleRatchet;
use crate::database::Database;
use crate::group::GroupSessionManager;
use crate::video_call::VideoCallManager;
use crate::voice_call::VoiceCallManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wraith_core::node::{Node, NodeConfig};

/// WRAITH node wrapper for thread-safe access
pub struct WraithNode {
    /// The actual WRAITH node
    node: Option<Node>,
    /// Whether the node is running
    running: bool,
}

impl WraithNode {
    /// Create a new uninitialized WRAITH node wrapper
    pub fn new() -> Self {
        Self {
            node: None,
            running: false,
        }
    }

    /// Initialize the WRAITH node with default configuration
    pub async fn initialize(&mut self) -> Result<(), String> {
        if self.node.is_some() {
            return Err("Node already initialized".to_string());
        }

        let config = NodeConfig::default();
        let node = Node::new_with_config(config)
            .await
            .map_err(|e| format!("Failed to create node: {}", e))?;

        self.node = Some(node);
        Ok(())
    }

    /// Initialize the WRAITH node with custom configuration
    pub async fn initialize_with_config(&mut self, config: NodeConfig) -> Result<(), String> {
        if self.node.is_some() {
            return Err("Node already initialized".to_string());
        }

        let node = Node::new_with_config(config)
            .await
            .map_err(|e| format!("Failed to create node: {}", e))?;

        self.node = Some(node);
        Ok(())
    }

    /// Start the WRAITH node
    pub async fn start(&mut self) -> Result<(), String> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| "Node not initialized".to_string())?;

        node.start()
            .await
            .map_err(|e| format!("Failed to start node: {}", e))?;

        self.running = true;
        Ok(())
    }

    /// Stop the WRAITH node
    pub async fn stop(&mut self) -> Result<(), String> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| "Node not initialized".to_string())?;

        node.stop()
            .await
            .map_err(|e| format!("Failed to stop node: {}", e))?;

        self.running = false;
        Ok(())
    }

    /// Check if the node is running
    pub fn is_running(&self) -> bool {
        self.running && self.node.as_ref().is_some_and(|n| n.is_running())
    }

    /// Get the node's peer ID (32-byte Ed25519 public key as hex string)
    pub fn peer_id(&self) -> Option<String> {
        self.node.as_ref().map(|n| hex::encode(n.node_id()))
    }

    /// Get the node's peer ID as raw bytes
    pub fn peer_id_bytes(&self) -> Option<[u8; 32]> {
        self.node.as_ref().map(|n| *n.node_id())
    }

    /// Get access to the underlying node for advanced operations
    pub fn node(&self) -> Option<&Node> {
        self.node.as_ref()
    }

    /// Get the number of active sessions
    pub fn active_route_count(&self) -> usize {
        self.node.as_ref().map_or(0, |n| n.active_route_count())
    }

    /// Establish a session with a peer
    ///
    /// Returns the session ID if successful.
    pub async fn establish_session(&self, peer_id: &[u8; 32]) -> Result<[u8; 32], String> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| "Node not initialized".to_string())?;

        let session_id = node
            .establish_session(peer_id)
            .await
            .map_err(|e| format!("Failed to establish session: {}", e))?;

        Ok(session_id)
    }

    /// Send data to a peer
    ///
    /// The data will be encrypted and sent via the WRAITH protocol.
    pub async fn send_data(&self, peer_id: &[u8; 32], data: &[u8]) -> Result<(), String> {
        let node = self
            .node
            .as_ref()
            .ok_or_else(|| "Node not initialized".to_string())?;

        node.send_data(peer_id, data)
            .await
            .map_err(|e| format!("Failed to send data: {}", e))
    }

    /// Get the X25519 public key for key exchange
    pub fn x25519_public_key(&self) -> Option<[u8; 32]> {
        self.node.as_ref().map(|n| *n.x25519_public_key())
    }
}

impl Default for WraithNode {
    fn default() -> Self {
        Self::new()
    }
}

/// Global application state
pub struct AppState {
    /// Database connection
    pub db: Mutex<Database>,

    /// Double Ratchet states for each peer
    pub ratchets: Mutex<HashMap<String, DoubleRatchet>>,

    /// Local peer ID (cached for quick access)
    pub local_peer_id: Mutex<String>,

    /// WRAITH protocol node
    pub node: Arc<Mutex<WraithNode>>,

    /// Voice call manager (Sprint 17.5)
    pub voice_calls: Arc<VoiceCallManager>,

    /// Video call manager (Sprint 17.6)
    pub video_calls: Arc<VideoCallManager>,

    /// Group session manager (Sprint 17.7)
    pub group_sessions: Arc<Mutex<GroupSessionManager>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database) -> Self {
        let voice_calls = Arc::new(VoiceCallManager::new());
        // Create video call manager that uses the same voice manager
        let video_calls = Arc::new(VideoCallManager::with_voice_manager(voice_calls.clone()));

        Self {
            db: Mutex::new(db),
            ratchets: Mutex::new(HashMap::new()),
            local_peer_id: Mutex::new(String::new()),
            node: Arc::new(Mutex::new(WraithNode::new())),
            voice_calls,
            video_calls,
            group_sessions: Arc::new(Mutex::new(GroupSessionManager::new())),
        }
    }
}
