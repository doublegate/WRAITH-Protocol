//! WRAITH Share - Group File Sharing with Granular Access Control
//!
//! This application provides secure group file sharing using the WRAITH protocol
//! with end-to-end encryption, capability-based access control, and activity logging.

pub mod access_control;
pub mod activity;
pub mod commands;
pub mod database;
pub mod error;
pub mod file_transfer;
pub mod group;
pub mod link_share;
pub mod state;
pub mod versioning;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wraith_share=info".into()),
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
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Group commands
            commands::create_group,
            commands::delete_group,
            commands::get_group,
            commands::list_groups,
            commands::invite_member,
            commands::accept_invitation,
            commands::remove_member,
            commands::set_member_role,
            commands::list_members,
            commands::get_group_info,
            // File commands
            commands::upload_file,
            commands::download_file,
            commands::delete_file,
            commands::list_files,
            commands::search_files,
            // Version commands
            commands::get_file_versions,
            commands::restore_version,
            commands::get_version_summary,
            // Activity commands
            commands::get_activity_log,
            commands::get_recent_activity,
            commands::search_activity,
            commands::get_activity_stats,
            // Link sharing commands
            commands::create_share_link,
            commands::get_share_link,
            commands::revoke_share_link,
            commands::download_via_link,
            commands::list_file_share_links,
            commands::link_requires_password,
            // Identity commands
            commands::get_peer_id,
            commands::get_display_name,
            commands::set_display_name,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Share");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("wraith_share.db");
    info!("Opening database at: {:?}", db_path);

    let db = database::Database::open(&db_path)?;
    let db = Arc::new(db);

    let app_state = Arc::new(state::AppState::new(
        database::Database::open(&db_path)?,
        app_data_dir.clone(),
    ));

    // Initialize the application state (load or create identity)
    app_state.initialize()?;

    // Create managers
    let access_control = Arc::new(access_control::AccessController::new(
        db.clone(),
        app_state.clone(),
    ));

    let managers = commands::Managers {
        group_manager: group::GroupManager::new(db.clone(), app_state.clone()),
        access_control: access_control.clone(),
        file_transfer: file_transfer::FileTransfer::new(
            db.clone(),
            app_state.clone(),
            access_control.clone(),
        ),
        version_manager: versioning::VersionManager::new(db.clone(), app_state.clone()),
        activity_logger: activity::ActivityLogger::new(db.clone(), app_state.clone()),
        link_share_manager: link_share::LinkShareManager::new(db.clone(), app_state.clone()),
    };

    app.manage(app_state);
    app.manage(managers);

    info!("WRAITH Share initialized successfully");
    Ok(())
}
