//! WRAITH Vault - Distributed Secret Storage
//!
//! This application provides distributed secret storage using Shamir's Secret Sharing
//! scheme with a guardian network for secure shard distribution and recovery.
//!
//! ## Features
//! - Shamir's Secret Sharing (k-of-n threshold scheme)
//! - Guardian peer network management
//! - Encrypted shard distribution
//! - Recovery with <10 second target
//! - Key rotation support
//!
//! ## Architecture
//! - Frontend: React 18 + TypeScript + Tailwind CSS v4
//! - Backend: Tauri 2.0 + Rust
//! - Storage: SQLite with WAL mode

// Existing backup/restore modules
pub mod backup;
pub mod chunker;
pub mod compression;
pub mod database;
pub mod dedup;
pub mod erasure;
pub mod error;
pub mod restore;

// Secret storage modules (Phase 24)
pub mod commands;
pub mod guardian;
pub mod recovery;
pub mod secrets;
pub mod shamir;
pub mod shard;
pub mod state;

#[cfg(test)]
mod integration_tests;

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
            // Secret management commands
            commands::create_secret,
            commands::get_secret,
            commands::list_secrets,
            commands::list_secrets_by_type,
            commands::list_secrets_by_tag,
            commands::search_secrets,
            commands::update_secret,
            commands::delete_secret,
            commands::get_secrets_needing_rotation,
            // Guardian management commands
            commands::add_guardian,
            commands::get_guardian,
            commands::get_guardian_by_peer_id,
            commands::list_guardians,
            commands::list_guardians_by_status,
            commands::list_available_guardians,
            commands::update_guardian,
            commands::update_guardian_status,
            commands::remove_guardian,
            commands::record_health_check,
            commands::select_guardians_for_distribution,
            // Distribution commands
            commands::prepare_distribution,
            commands::mark_shard_delivered,
            commands::get_distribution_status,
            commands::get_shard_assignments,
            commands::request_shard_from_guardian,
            // Recovery commands
            commands::start_recovery,
            commands::add_recovery_shard,
            commands::complete_recovery,
            commands::get_recovery_progress,
            commands::cancel_recovery,
            commands::list_recovery_sessions,
            // Key rotation commands
            commands::rotate_secret_key,
            commands::record_rotation,
            // Node commands
            commands::start_node,
            commands::stop_node,
            commands::get_node_status,
            commands::get_peer_id,
            // Statistics commands
            commands::get_vault_stats,
            commands::get_runtime_statistics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Set up the application
fn setup_app(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Setting up WRAITH Vault application");

    // Get app data directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir)?;

    // Open database
    let db_path = app_data_dir.join("vault.db");
    tracing::info!("Database path: {:?}", db_path);

    let db = database::Database::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Create application state
    let app_state = Arc::new(state::AppState::new(db));

    // Initialize state from database (async)
    let state_clone = app_state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = state_clone.initialize().await {
            tracing::error!("Failed to initialize application state: {}", e);
        }
    });

    // Register state with Tauri
    app.manage(app_state);

    tracing::info!("WRAITH Vault setup complete");
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
