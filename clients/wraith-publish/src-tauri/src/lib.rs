//! WRAITH Publish - Censorship-Resistant Publishing Platform
//!
//! A decentralized content publishing application using the WRAITH protocol.
//! Features content addressing (CIDs), DHT storage with replication,
//! Ed25519 signatures, and RSS feed generation.

pub mod commands;
pub mod content;
pub mod database;
pub mod error;
pub mod markdown;
pub mod propagation;
pub mod publisher;
pub mod reader;
pub mod rss;
pub mod signatures;
pub mod state;
pub mod storage;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wraith_publish=info".into()),
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
            // Identity commands
            commands::get_peer_id,
            commands::get_display_name,
            commands::set_display_name,
            // Draft commands
            commands::create_draft,
            commands::update_draft,
            commands::delete_draft,
            commands::list_drafts,
            commands::get_draft,
            // Publish commands
            commands::publish_article,
            commands::unpublish_article,
            commands::list_published,
            commands::get_article,
            // Content commands
            commands::fetch_content,
            commands::verify_content,
            commands::search_articles,
            // Signature commands
            commands::sign_content,
            commands::verify_signature,
            // Storage commands
            commands::pin_content,
            commands::unpin_content,
            commands::list_pinned,
            commands::get_storage_stats,
            // Propagation commands
            commands::get_propagation_status,
            commands::list_active_propagations,
            // RSS commands
            commands::generate_rss_feed,
            commands::generate_author_feed,
            commands::generate_tag_feed,
            // Markdown commands
            commands::render_markdown,
            commands::extract_metadata,
            // Image commands
            commands::upload_image,
            commands::get_image,
            commands::delete_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Publish");
}

/// Setup application state and database
fn setup_app<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("wraith_publish.db");
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
    let storage_config = storage::StorageConfig::default();
    let dht_storage = Arc::new(storage::DhtStorage::new(storage_config));

    let reader_config = reader::ReaderConfig::default();
    let content_reader = reader::ContentReader::new(db.clone(), dht_storage.clone(), reader_config);

    let feed_config = rss::FeedConfig::default();
    let feed_generator = rss::FeedGenerator::new(db.clone(), feed_config);

    let propagation_tracker = propagation::PropagationTracker::new(3);

    let managers = commands::Managers {
        content_manager: content::ContentManager::new(db.clone(), app_state.clone()),
        publisher: publisher::Publisher::new(db.clone(), app_state.clone()),
        content_reader,
        dht_storage,
        feed_generator,
        propagation_tracker,
        markdown: markdown::MarkdownProcessor::new(),
    };

    app.manage(app_state);
    app.manage(db);
    app.manage(managers);

    info!("WRAITH Publish initialized successfully");
    Ok(())
}
