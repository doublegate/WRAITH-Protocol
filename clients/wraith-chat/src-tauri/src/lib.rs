// WRAITH-Chat - Secure End-to-End Encrypted Messaging
//
// This application provides secure messaging using the WRAITH protocol with Double Ratchet
// encryption (Signal Protocol) for end-to-end encrypted communications.

pub mod commands;
pub mod crypto;
pub mod database;
pub mod secure_storage;
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
            commands::stop_node,
            commands::get_node_status,
            commands::get_peer_id,
            commands::establish_session,
            commands::init_receiving_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("wraith_chat.db");

    // Get database encryption key from secure storage
    let secure_storage = secure_storage::SecureStorage::new();
    let password = match secure_storage.get_or_create_db_key() {
        Ok(key) => key,
        Err(e) => {
            log::warn!(
                "Failed to access secure storage: {}. Using fallback key derivation.",
                e
            );
            // Fallback: derive a key from machine-specific info
            // This is less secure but allows the app to function
            derive_fallback_key(&app_dir)?
        }
    };

    log::info!("Opening database at: {:?}", db_path);

    let db = database::Database::open(db_path, &password)?;
    let state = Arc::new(state::AppState::new(db));

    app.manage(state);

    Ok(())
}

/// Derive a fallback encryption key when secure storage is unavailable
///
/// This uses machine-specific data to derive a deterministic key.
/// This is less secure than proper secure storage but provides some protection.
fn derive_fallback_key(app_dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    // Derive from app directory path (machine-specific)
    let mut hasher = Sha256::new();
    hasher.update(b"wraith-chat-fallback-key-v1");
    hasher.update(app_dir.to_string_lossy().as_bytes());

    // Add hostname if available
    if let Ok(hostname) = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| std::env::var("USER"))
    {
        hasher.update(hostname.as_bytes());
    }

    let hash = hasher.finalize();
    use base64::Engine;
    let key_b64 = base64::engine::general_purpose::STANDARD.encode(hash);

    log::warn!(
        "Using fallback key derivation. For better security, ensure secure storage is available."
    );

    Ok(key_b64)
}
