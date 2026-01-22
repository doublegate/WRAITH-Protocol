//! WRAITH Stream - Encrypted Peer-to-Peer Media Streaming
//!
//! This application provides encrypted video streaming using the WRAITH protocol
//! with adaptive bitrate, HLS segments, and end-to-end encryption.

pub mod commands;
pub mod database;
pub mod discovery;
pub mod error;
pub mod player;
pub mod segment_storage;
pub mod state;
pub mod stream_manager;
pub mod subtitles;
pub mod transcoder;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wraith_stream=info".into()),
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
            // Stream management
            commands::create_stream,
            commands::upload_video,
            commands::delete_stream,
            commands::list_streams,
            commands::get_stream,
            commands::update_stream,
            commands::get_stream_status,
            // Playback
            commands::load_stream,
            commands::get_segment,
            commands::get_manifest,
            commands::set_quality,
            commands::get_playback_info,
            // Discovery
            commands::search_streams,
            commands::get_trending_streams,
            commands::get_my_streams,
            // Subtitles
            commands::load_subtitles,
            commands::add_subtitles,
            commands::list_subtitle_languages,
            // Views
            commands::record_view,
            commands::get_stream_views,
            // Identity
            commands::get_peer_id,
            commands::get_display_name,
            commands::set_display_name,
            // Transcoding status
            commands::get_transcode_progress,
            commands::cancel_transcode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Stream");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("wraith_stream.db");
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
    let managers = commands::Managers {
        stream_manager: stream_manager::StreamManager::new(db.clone(), app_state.clone()),
        segment_storage: segment_storage::SegmentStorage::new(db.clone(), app_state.clone()),
        transcoder: transcoder::Transcoder::new(app_state.clone()),
        player: player::Player::new(db.clone(), app_state.clone()),
        discovery: discovery::StreamDiscovery::new(db.clone()),
        subtitles: subtitles::SubtitleManager::new(db.clone()),
    };

    app.manage(app_state);
    app.manage(managers);

    info!("WRAITH Stream initialized successfully");
    Ok(())
}
