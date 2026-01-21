//! WRAITH Sync - Decentralized File Synchronization
//!
//! This application provides Dropbox-like file synchronization using the WRAITH protocol
//! with end-to-end encryption, delta sync, and conflict resolution.

pub mod commands;
pub mod config;
pub mod database;
pub mod delta;
pub mod error;
pub mod state;
pub mod sync_engine;
pub mod watcher;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wraith_sync=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize Tauri application
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Status commands
            commands::get_status,
            commands::pause_sync,
            commands::resume_sync,
            // Folder commands
            commands::add_folder,
            commands::remove_folder,
            commands::list_folders,
            commands::get_folder,
            commands::pause_folder,
            commands::resume_folder,
            commands::force_sync_folder,
            // Conflict commands
            commands::list_conflicts,
            commands::resolve_conflict,
            // Version history commands
            commands::get_file_versions,
            commands::restore_version,
            // Device commands
            commands::list_devices,
            commands::remove_device,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
            // Ignored patterns commands
            commands::get_ignored_patterns,
            commands::add_ignored_pattern,
            // File browser commands
            commands::list_folder_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Sync");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("wraith_sync.db");
    info!("Opening database at: {:?}", db_path);

    let db = database::Database::open(&db_path)?;
    let app_state = Arc::new(state::AppState::new(db, app_data_dir.clone()));

    // Initialize watcher
    if let Err(e) = app_state.init_watcher() {
        tracing::warn!("Failed to initialize file watcher: {}", e);
    }

    // Start watching folders if auto-start is enabled
    if let Ok(settings) = app_state.get_settings() {
        if settings.auto_start {
            if let Err(e) = app_state.start_watching() {
                tracing::warn!("Failed to start watching folders: {}", e);
            }
        }
    }

    app.manage(app_state);

    info!("WRAITH Sync initialized successfully");
    Ok(())
}
