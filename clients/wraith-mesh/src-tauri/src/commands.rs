//! Tauri IPC Commands
//!
//! Defines all commands exposed to the frontend for network monitoring,
//! DHT inspection, diagnostics, and data export.

use crate::dht_inspector::{DhtInspector, LookupHop, LookupResult, RoutingBucket, StoredKey};
use crate::diagnostics::{
    BandwidthResult, Diagnostics, HealthReport, NatDetectionResult, PingResult,
};
use crate::error::MeshError;
use crate::export::{ExportFormat, export_metrics_history, export_network_snapshot};
use crate::network_monitor::{MetricsEntry, NetworkMonitor, NetworkSnapshot, PeerType};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

/// Managers wrapper for Tauri state
pub struct Managers {
    pub network_monitor: NetworkMonitor,
    pub dht_inspector: DhtInspector,
    pub diagnostics: Diagnostics,
}

// ============================================================================
// Network Monitoring Commands
// ============================================================================

/// Get the current network snapshot
#[tauri::command]
pub fn get_network_snapshot(managers: State<'_, Managers>) -> Result<NetworkSnapshot, MeshError> {
    managers.network_monitor.update_snapshot()
}

/// Get metrics history
#[tauri::command]
pub fn get_metrics_history(
    managers: State<'_, Managers>,
    limit: Option<usize>,
) -> Vec<MetricsEntry> {
    managers
        .network_monitor
        .get_metrics_history(limit.unwrap_or(60))
}

/// Initialize demo network for testing
#[tauri::command]
pub fn initialize_demo_network(managers: State<'_, Managers>) -> Result<(), MeshError> {
    managers.network_monitor.initialize_demo_network()
}

/// Add a peer to the network (for testing)
#[tauri::command]
pub fn add_peer(managers: State<'_, Managers>, peer_type: String) -> Result<String, MeshError> {
    let pt = match peer_type.to_lowercase().as_str() {
        "direct" => PeerType::Direct,
        "relay" => PeerType::Relay,
        "indirect" => PeerType::Indirect,
        _ => return Err(MeshError::Network("Invalid peer type".to_string())),
    };
    managers.network_monitor.add_peer(pt)
}

/// Remove a peer from the network
#[tauri::command]
pub fn remove_peer(managers: State<'_, Managers>, peer_id: String) -> Result<(), MeshError> {
    managers.network_monitor.remove_peer(&peer_id)
}

// ============================================================================
// DHT Inspection Commands
// ============================================================================

/// Get the DHT routing table
#[tauri::command]
pub fn get_routing_table(managers: State<'_, Managers>) -> Result<Vec<RoutingBucket>, MeshError> {
    managers.dht_inspector.get_routing_table()
}

/// Look up a key in the DHT
#[tauri::command]
pub fn lookup_key(managers: State<'_, Managers>, key: String) -> Result<LookupResult, MeshError> {
    managers.dht_inspector.lookup_key(&key)
}

/// Trace a DHT lookup path
#[tauri::command]
pub fn trace_lookup(
    managers: State<'_, Managers>,
    key: String,
) -> Result<Vec<LookupHop>, MeshError> {
    managers.dht_inspector.trace_lookup(&key)
}

/// Get stored keys from local DHT
#[tauri::command]
pub fn get_stored_keys(managers: State<'_, Managers>) -> Result<Vec<StoredKey>, MeshError> {
    managers.dht_inspector.get_stored_keys()
}

/// Calculate XOR distance between two node IDs
#[tauri::command]
pub fn calculate_distance(
    managers: State<'_, Managers>,
    id1: String,
    id2: String,
) -> Result<String, MeshError> {
    managers.dht_inspector.calculate_distance(&id1, &id2)
}

// ============================================================================
// Diagnostic Commands
// ============================================================================

/// Ping a peer
#[tauri::command]
pub async fn ping_peer(
    managers: State<'_, Managers>,
    peer_id: String,
    count: Option<u32>,
) -> Result<PingResult, MeshError> {
    managers
        .diagnostics
        .ping(&peer_id, count.unwrap_or(5))
        .await
}

/// Run a bandwidth test
#[tauri::command]
pub async fn bandwidth_test(
    managers: State<'_, Managers>,
    peer_id: String,
) -> Result<BandwidthResult, MeshError> {
    managers.diagnostics.bandwidth_test(&peer_id).await
}

/// Check connection health
#[tauri::command]
pub async fn check_connection_health(
    managers: State<'_, Managers>,
    peer_id: String,
) -> Result<HealthReport, MeshError> {
    managers.diagnostics.check_connection_health(&peer_id).await
}

/// Detect NAT type
#[tauri::command]
pub async fn detect_nat_type(
    managers: State<'_, Managers>,
) -> Result<NatDetectionResult, MeshError> {
    managers.diagnostics.detect_nat_type().await
}

// ============================================================================
// Export Commands
// ============================================================================

/// Export network snapshot
#[tauri::command]
pub fn export_snapshot(managers: State<'_, Managers>, format: String) -> Result<String, MeshError> {
    let snapshot = managers.network_monitor.get_snapshot();
    let fmt = match format.to_lowercase().as_str() {
        "json" => ExportFormat::Json,
        "csv" => ExportFormat::Csv,
        _ => return Err(MeshError::Network("Invalid export format".to_string())),
    };
    export_network_snapshot(&snapshot, fmt)
}

/// Export metrics history as CSV
#[tauri::command]
pub fn export_metrics(
    managers: State<'_, Managers>,
    limit: Option<usize>,
) -> Result<String, MeshError> {
    let history = managers
        .network_monitor
        .get_metrics_history(limit.unwrap_or(3600));
    export_metrics_history(&history)
}

// ============================================================================
// State Commands
// ============================================================================

/// Get the local peer ID
#[tauri::command]
pub fn get_peer_id(state: State<'_, Arc<AppState>>) -> Option<String> {
    state.get_peer_id()
}

/// Check if monitoring is active
#[tauri::command]
pub fn is_monitoring_active(state: State<'_, Arc<AppState>>) -> bool {
    state.is_monitoring_active()
}

/// Set monitoring active state
#[tauri::command]
pub fn set_monitoring_active(state: State<'_, Arc<AppState>>, active: bool) {
    state.set_monitoring_active(active);
}

/// Get monitor interval
#[tauri::command]
pub fn get_monitor_interval(state: State<'_, Arc<AppState>>) -> u64 {
    state.get_monitor_interval()
}

/// Set monitor interval
#[tauri::command]
pub fn set_monitor_interval(state: State<'_, Arc<AppState>>, interval_ms: u64) {
    state.set_monitor_interval(interval_ms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    #[allow(dead_code)]
    fn create_test_managers() -> (Arc<AppState>, Managers) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = Arc::new(AppState::new(db, dir.path().to_path_buf()));
        state.initialize().unwrap();

        let managers = Managers {
            network_monitor: NetworkMonitor::new(state.clone()),
            dht_inspector: DhtInspector::new(state.clone()),
            diagnostics: Diagnostics::new(state.clone()),
        };

        (state, managers)
    }

    // Tests would require Tauri runtime, so they're integration tests
}
