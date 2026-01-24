//! WRAITH Recon - Network Reconnaissance and Data Exfiltration Assessment Platform
//!
//! This application provides authorized security testing capabilities with comprehensive
//! safety controls and tamper-evident audit logging.
//!
//! ## Features
//! - Rules of Engagement (RoE) enforcement with Ed25519 signatures
//! - Target scope enforcement (CIDR/domain whitelists)
//! - Time-bounded execution windows
//! - Kill switch mechanism with signed halt signals
//! - Tamper-evident cryptographic audit logging (Merkle chain)
//! - Passive and active reconnaissance
//! - Multi-path exfiltration channels
//!
//! ## Safety Controls
//! All reconnaissance and exfiltration operations are:
//! - Validated against scope before execution
//! - Logged with MITRE ATT&CK technique mapping
//! - Subject to engagement timing constraints
//! - Immediately halted by kill switch activation
//!
//! ## Architecture
//! - Frontend: React 18 + TypeScript + Tailwind CSS v4
//! - Backend: Tauri 2.0 + Rust
//! - Storage: SQLite with WAL mode
//! - Crypto: Ed25519 signatures, BLAKE3 hashing

// Core modules
pub mod error;

// Safety modules
pub mod audit;
pub mod killswitch;
pub mod roe;
pub mod scope;
pub mod timing;

// Reconnaissance modules
pub mod active;
pub mod channels;
pub mod passive;

// Infrastructure modules
pub mod commands;
pub mod database;
pub mod state;

// Integration tests will be added in Task #8
// #[cfg(test)]
// mod integration_tests;

use std::sync::Arc;
use tauri::Manager;

/// Initialize and run the Tauri application
pub fn run() {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        // SAFETY: This is called at the start of main before any threads are spawned
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Rules of Engagement commands
            commands::load_roe,
            commands::load_roe_file,
            commands::get_roe,
            commands::validate_roe,
            // Engagement commands
            commands::start_engagement,
            commands::stop_engagement,
            commands::pause_engagement,
            commands::resume_engagement,
            commands::get_engagement_status,
            // Scope commands
            commands::validate_target,
            commands::add_custom_target,
            commands::get_scope_summary,
            // Kill switch commands
            commands::activate_kill_switch,
            commands::process_kill_switch_signal,
            commands::is_kill_switch_active,
            // Passive reconnaissance commands
            commands::start_passive_recon,
            commands::stop_passive_recon,
            commands::get_passive_recon_stats,
            commands::get_discovered_assets,
            // Active reconnaissance commands
            commands::start_active_scan,
            commands::stop_active_scan,
            commands::get_active_scan_progress,
            commands::get_active_scan_results,
            // Channel commands
            commands::open_channel,
            commands::send_through_channel,
            commands::close_channel,
            commands::list_channels,
            commands::get_channel_stats,
            // Audit commands
            commands::get_audit_entries,
            commands::verify_audit_chain,
            commands::export_audit_log,
            commands::add_audit_note,
            // Node commands
            commands::start_node,
            commands::stop_node,
            commands::get_node_status,
            commands::get_peer_id,
            // Statistics commands
            commands::get_statistics,
            commands::get_database_stats,
            commands::get_timing_info,
            // Operator commands
            commands::set_operator_id,
            commands::get_operator_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Set up the application
fn setup_app(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Setting up WRAITH Recon application");

    // Get app data directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir)?;

    // Open database
    let db_path = app_data_dir.join("recon.db");
    tracing::info!("Database path: {:?}", db_path);

    let db = database::Database::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Get operator ID from environment or use default
    let operator_id = std::env::var("WRAITH_OPERATOR_ID").unwrap_or_else(|_| whoami::username());

    tracing::info!("Operator ID: {}", operator_id);

    // Create application state
    let app_state = Arc::new(state::AppState::new(db, operator_id));

    // Log application startup
    let entry = app_state.audit.info(
        audit::AuditCategory::System,
        "WRAITH Recon application started",
    );
    app_state.statistics.record_audit_entry();

    // Store audit entry (async)
    let state_clone = app_state.clone();
    let entry_clone = entry;
    tauri::async_runtime::spawn(async move {
        let db = state_clone.db.lock();
        if let Err(e) = db.store_audit_entry(&entry_clone) {
            tracing::error!("Failed to store startup audit entry: {}", e);
        }
    });

    // Register state with Tauri
    app.manage(app_state);

    tracing::info!("WRAITH Recon setup complete");
    tracing::warn!("IMPORTANT: This tool is for AUTHORIZED security testing ONLY");
    tracing::warn!("All operations are logged and require valid Rules of Engagement");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_modules_compile() {
        // This test just verifies that all modules compile correctly
        // The actual functionality is tested in each module's tests
        assert!(true);
    }
}
