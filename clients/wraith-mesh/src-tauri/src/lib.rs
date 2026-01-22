//! WRAITH Mesh - Network Topology Visualization and Diagnostics
//!
//! This application provides real-time visualization of the WRAITH network
//! topology, peer connections, and traffic flows. Features include:
//!
//! - Force-directed network graph visualization
//! - Real-time connection statistics dashboard
//! - DHT routing table inspection
//! - Network diagnostic tools (ping, bandwidth, NAT detection)
//! - Data export functionality

pub mod commands;
pub mod database;
pub mod dht_inspector;
pub mod diagnostics;
pub mod error;
pub mod export;
pub mod network_monitor;
pub mod state;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wraith_mesh=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize Tauri application
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Network monitoring
            commands::get_network_snapshot,
            commands::get_metrics_history,
            commands::initialize_demo_network,
            commands::add_peer,
            commands::remove_peer,
            // DHT inspection
            commands::get_routing_table,
            commands::lookup_key,
            commands::trace_lookup,
            commands::get_stored_keys,
            commands::calculate_distance,
            // Diagnostics
            commands::ping_peer,
            commands::bandwidth_test,
            commands::check_connection_health,
            commands::detect_nat_type,
            // Export
            commands::export_snapshot,
            commands::export_metrics,
            // State
            commands::get_peer_id,
            commands::is_monitoring_active,
            commands::set_monitoring_active,
            commands::get_monitor_interval,
            commands::set_monitor_interval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Mesh");
}

/// Setup application state and services
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("wraith_mesh.db");
    info!("Opening database at: {:?}", db_path);

    let db = database::Database::open(&db_path)?;
    let app_state = Arc::new(state::AppState::new(db, app_data_dir.clone()));

    // Initialize the application state
    app_state.initialize()?;

    // Create service managers
    let managers = commands::Managers {
        network_monitor: network_monitor::NetworkMonitor::new(app_state.clone()),
        dht_inspector: dht_inspector::DhtInspector::new(app_state.clone()),
        diagnostics: diagnostics::Diagnostics::new(app_state.clone()),
    };

    // Initialize demo network for testing
    if let Err(e) = managers.network_monitor.initialize_demo_network() {
        tracing::warn!("Failed to initialize demo network: {}", e);
    }

    app.manage(app_state);
    app.manage(managers);

    info!("WRAITH Mesh initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_imports() {
        // Ensure all modules compile correctly
        let _ = super::commands::get_network_snapshot;
        let _ = super::database::Database::open;
        let _ = super::dht_inspector::DhtInspector::new;
        let _ = super::diagnostics::Diagnostics::new;
        let _ = super::error::MeshError::Database;
        let _ = super::export::ExportFormat::Json;
        let _ = super::network_monitor::NetworkMonitor::new;
        let _ = super::state::AppState::new;
    }
}
