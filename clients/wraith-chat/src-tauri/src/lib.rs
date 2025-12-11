// WRAITH-Chat - Secure End-to-End Encrypted Messaging
//
// This application provides secure messaging using the WRAITH protocol with Double Ratchet
// encryption (Signal Protocol) for end-to-end encrypted communications.

pub mod commands;
pub mod crypto;
pub mod database;
pub mod state;

use std::sync::Arc;
use tauri::{Manager, Runtime};

/// Initialize Tauri application
pub fn run() {
    env_logger::init();

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
            commands::create_contact,
            commands::get_contact,
            commands::list_contacts,
            commands::create_conversation,
            commands::get_conversation,
            commands::list_conversations,
            commands::send_message,
            commands::receive_message,
            commands::get_messages,
            commands::mark_as_read,
            commands::start_node,
            commands::get_node_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("wraith_chat.db");

    // TODO: Get password from secure storage or prompt user
    let password = "temporary-password-change-me"; // SECURITY: This should be user-provided

    log::info!("Opening database at: {:?}", db_path);

    let db = database::Database::open(db_path, password)?;
    let state = Arc::new(state::AppState::new(db));

    app.manage(state);

    Ok(())
}
