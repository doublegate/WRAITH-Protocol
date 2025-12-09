//! WRAITH Transfer - Tauri Backend
//!
//! This module provides the IPC commands for the WRAITH Transfer desktop application,
//! integrating with wraith-core for secure peer-to-peer file transfers.

use serde::{Deserialize, Serialize};

mod commands;
mod error;
mod state;

pub use error::AppError;
pub use state::AppState;

/// Application result type
pub type AppResult<T> = Result<T, AppError>;

/// Node status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub node_id: Option<String>,
    pub active_sessions: usize,
    pub active_transfers: usize,
}

/// Transfer information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferInfo {
    pub id: String,
    pub peer_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub progress: f32,
    pub status: String,
    pub direction: String, // "upload" or "download"
}

/// Session information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub peer_id: String,
    pub established_at: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Initialize the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_node_status,
            commands::start_node,
            commands::stop_node,
            commands::get_node_id,
            commands::get_sessions,
            commands::close_session,
            commands::send_file,
            commands::get_transfers,
            commands::get_transfer_progress,
            commands::cancel_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Transfer");
}
