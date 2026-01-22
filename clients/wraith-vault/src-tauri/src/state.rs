//! Application State Management for WRAITH Vault
//!
//! Manages global state including database, secret manager, guardian manager,
//! and recovery sessions.

use crate::database::{Database, VaultStats};
use crate::guardian::GuardianManager;
use crate::recovery::RecoveryManager;
use crate::secrets::SecretManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Statistics tracker for vault operations
pub struct VaultStatistics {
    /// Total secrets created
    total_secrets_created: AtomicU64,
    /// Total recoveries performed
    total_recoveries: AtomicU64,
    /// Successful recoveries
    successful_recoveries: AtomicU64,
    /// Failed recoveries
    failed_recoveries: AtomicU64,
    /// Total key rotations
    total_key_rotations: AtomicU64,
    /// Total shards distributed
    total_shards_distributed: AtomicU64,
    /// Average recovery time (cumulative milliseconds)
    total_recovery_time_ms: AtomicU64,
    /// Recovery time sample count
    recovery_time_samples: AtomicU64,
}

impl VaultStatistics {
    /// Create a new statistics tracker
    pub fn new() -> Self {
        Self {
            total_secrets_created: AtomicU64::new(0),
            total_recoveries: AtomicU64::new(0),
            successful_recoveries: AtomicU64::new(0),
            failed_recoveries: AtomicU64::new(0),
            total_key_rotations: AtomicU64::new(0),
            total_shards_distributed: AtomicU64::new(0),
            total_recovery_time_ms: AtomicU64::new(0),
            recovery_time_samples: AtomicU64::new(0),
        }
    }

    /// Record a secret creation
    pub fn record_secret_created(&self) {
        self.total_secrets_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a recovery attempt
    pub fn record_recovery(&self, success: bool, duration_ms: u64) {
        self.total_recoveries.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_recoveries.fetch_add(1, Ordering::Relaxed);
            self.total_recovery_time_ms
                .fetch_add(duration_ms, Ordering::Relaxed);
            self.recovery_time_samples.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_recoveries.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a key rotation
    pub fn record_key_rotation(&self) {
        self.total_key_rotations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record shard distribution
    pub fn record_shards_distributed(&self, count: u64) {
        self.total_shards_distributed
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Get total secrets created
    pub fn secrets_created(&self) -> u64 {
        self.total_secrets_created.load(Ordering::Relaxed)
    }

    /// Get total recoveries
    pub fn total_recoveries(&self) -> u64 {
        self.total_recoveries.load(Ordering::Relaxed)
    }

    /// Get successful recoveries
    pub fn successful_recoveries(&self) -> u64 {
        self.successful_recoveries.load(Ordering::Relaxed)
    }

    /// Get failed recoveries
    pub fn failed_recoveries(&self) -> u64 {
        self.failed_recoveries.load(Ordering::Relaxed)
    }

    /// Get average recovery time in milliseconds
    pub fn average_recovery_time_ms(&self) -> Option<f64> {
        let samples = self.recovery_time_samples.load(Ordering::Relaxed);
        if samples == 0 {
            return None;
        }
        let total = self.total_recovery_time_ms.load(Ordering::Relaxed);
        Some(total as f64 / samples as f64)
    }

    /// Get total key rotations
    pub fn key_rotations(&self) -> u64 {
        self.total_key_rotations.load(Ordering::Relaxed)
    }

    /// Get total shards distributed
    pub fn shards_distributed(&self) -> u64 {
        self.total_shards_distributed.load(Ordering::Relaxed)
    }

    /// Get recovery success rate
    pub fn recovery_success_rate(&self) -> Option<f64> {
        let total = self.total_recoveries.load(Ordering::Relaxed);
        if total == 0 {
            return None;
        }
        let successful = self.successful_recoveries.load(Ordering::Relaxed);
        Some(successful as f64 / total as f64 * 100.0)
    }
}

impl Default for VaultStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Global application state for WRAITH Vault
pub struct AppState {
    /// Database connection
    pub db: Arc<parking_lot::Mutex<Database>>,

    /// Secret manager
    pub secrets: Mutex<SecretManager>,

    /// Guardian manager
    pub guardians: Arc<GuardianManager>,

    /// Recovery manager
    pub recovery: Mutex<RecoveryManager>,

    /// Local peer ID
    pub local_peer_id: Mutex<String>,

    /// WRAITH protocol node
    pub node: Arc<Mutex<WraithNode>>,

    /// Statistics tracker
    pub statistics: Arc<VaultStatistics>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(parking_lot::Mutex::new(db)),
            secrets: Mutex::new(SecretManager::new()),
            guardians: Arc::new(GuardianManager::new()),
            recovery: Mutex::new(RecoveryManager::new()),
            local_peer_id: Mutex::new(String::new()),
            node: Arc::new(Mutex::new(WraithNode::new())),
            statistics: Arc::new(VaultStatistics::new()),
        }
    }

    /// Initialize the application state from database
    pub async fn initialize(&self) -> Result<(), String> {
        // Load guardians from database
        let guardians = {
            let db = self.db.lock();
            db.list_guardians().map_err(|e| e.to_string())?
        };
        self.guardians.load_guardians(guardians).await;

        // Load secrets from database
        let secrets = {
            let db = self.db.lock();
            db.list_secrets().map_err(|e| e.to_string())?
        };
        {
            let mut secret_manager = self.secrets.lock().await;
            secret_manager.load_secrets(secrets);
        }

        tracing::info!("Application state initialized");
        Ok(())
    }

    /// Get vault statistics
    pub fn get_vault_stats(&self) -> Result<VaultStats, String> {
        let db = self.db.lock();
        db.get_vault_stats().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_app_state_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let state = AppState::new(db);
        assert!(state.local_peer_id.lock().await.is_empty());
    }

    #[test]
    fn test_vault_statistics() {
        let stats = VaultStatistics::new();

        // Record some operations
        stats.record_secret_created();
        stats.record_secret_created();
        assert_eq!(stats.secrets_created(), 2);

        stats.record_recovery(true, 500);
        stats.record_recovery(true, 300);
        stats.record_recovery(false, 0);

        assert_eq!(stats.total_recoveries(), 3);
        assert_eq!(stats.successful_recoveries(), 2);
        assert_eq!(stats.failed_recoveries(), 1);
        assert_eq!(stats.average_recovery_time_ms(), Some(400.0));
        assert!((stats.recovery_success_rate().unwrap() - 66.67).abs() < 0.1);

        stats.record_key_rotation();
        assert_eq!(stats.key_rotations(), 1);

        stats.record_shards_distributed(5);
        assert_eq!(stats.shards_distributed(), 5);
    }

    #[test]
    fn test_wraith_node_default() {
        let node = WraithNode::new();
        assert!(!node.is_running());
        assert!(node.peer_id().is_none());
    }
}
