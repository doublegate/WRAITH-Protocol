//! Application State Management
//!
//! Manages shared state for WRAITH Mesh including network monitor,
//! DHT inspector, and diagnostic tools.

use crate::database::Database;
use crate::error::MeshResult;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// Application data directory
    pub app_data_dir: PathBuf,
    /// Local peer ID
    pub local_peer_id: Arc<RwLock<Option<String>>>,
    /// Whether monitoring is active
    pub monitoring_active: Arc<RwLock<bool>>,
    /// Monitoring interval in milliseconds
    pub monitor_interval_ms: Arc<RwLock<u64>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database, app_data_dir: PathBuf) -> Self {
        Self {
            db: Arc::new(db),
            app_data_dir,
            local_peer_id: Arc::new(RwLock::new(None)),
            monitoring_active: Arc::new(RwLock::new(false)),
            monitor_interval_ms: Arc::new(RwLock::new(1000)),
        }
    }

    /// Initialize the application state
    pub fn initialize(&self) -> MeshResult<()> {
        // Generate a demo peer ID for visualization purposes
        let peer_id = hex::encode(&uuid::Uuid::new_v4().as_bytes()[..16]);
        *self.local_peer_id.write() = Some(peer_id.clone());

        info!("Initialized WRAITH Mesh with peer ID: {}", peer_id);
        Ok(())
    }

    /// Get the local peer ID
    pub fn get_peer_id(&self) -> Option<String> {
        self.local_peer_id.read().clone()
    }

    /// Check if monitoring is active
    pub fn is_monitoring_active(&self) -> bool {
        *self.monitoring_active.read()
    }

    /// Set monitoring active state
    pub fn set_monitoring_active(&self, active: bool) {
        *self.monitoring_active.write() = active;
    }

    /// Get monitor interval in milliseconds
    pub fn get_monitor_interval(&self) -> u64 {
        *self.monitor_interval_ms.read()
    }

    /// Set monitor interval in milliseconds
    pub fn set_monitor_interval(&self, interval_ms: u64) {
        *self.monitor_interval_ms.write() = interval_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_initialization() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        state.initialize().unwrap();

        assert!(state.get_peer_id().is_some());
        assert!(!state.is_monitoring_active());
    }

    #[test]
    fn test_monitoring_state() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        assert!(!state.is_monitoring_active());
        state.set_monitoring_active(true);
        assert!(state.is_monitoring_active());
    }
}
