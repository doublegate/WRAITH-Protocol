// WRAITH-Chat - Secure End-to-End Encrypted Messaging
//
// This application provides secure messaging using the WRAITH protocol with Double Ratchet
// encryption (Signal Protocol) for end-to-end encrypted communications.

pub mod audio;
pub mod commands;
pub mod crypto;
pub mod database;
pub mod group;
#[cfg(test)]
mod integration_tests;
pub mod secure_storage;
pub mod state;
pub mod video;
pub mod video_call;
pub mod voice_call;

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
            // Contact commands
            commands::create_contact,
            commands::get_contact,
            commands::list_contacts,
            // Conversation commands
            commands::create_conversation,
            commands::get_conversation,
            commands::list_conversations,
            // Message commands
            commands::send_message,
            commands::receive_message,
            commands::get_messages,
            commands::mark_as_read,
            // Node commands
            commands::start_node,
            commands::stop_node,
            commands::get_node_status,
            commands::get_peer_id,
            // Session commands
            commands::establish_session,
            commands::init_receiving_session,
            // Voice call commands (Sprint 17.5)
            commands::start_call,
            commands::answer_call,
            commands::reject_call,
            commands::end_call,
            commands::toggle_mute,
            commands::toggle_speaker,
            commands::get_call_info,
            commands::get_active_calls,
            commands::list_audio_input_devices,
            commands::list_audio_output_devices,
            commands::set_audio_input_device,
            commands::set_audio_output_device,
            // Group messaging commands (Sprint 17.7)
            commands::create_group,
            commands::get_group_info,
            commands::update_group_settings,
            commands::add_group_member,
            commands::remove_group_member,
            commands::leave_group,
            commands::promote_to_admin,
            commands::demote_from_admin,
            commands::send_group_message,
            commands::get_group_members,
            commands::rotate_group_keys,
            // Video call commands (Sprint 17.6)
            commands::start_video_call,
            commands::answer_video_call,
            commands::end_video_call,
            commands::enable_video,
            commands::disable_video,
            commands::switch_video_source,
            commands::switch_camera,
            commands::toggle_video_mute,
            commands::get_video_call_info,
            commands::get_active_video_calls,
            commands::list_cameras,
            commands::list_screen_sources,
            commands::select_camera,
            commands::select_screen_source,
            commands::set_video_quality,
            commands::request_keyframe,
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
