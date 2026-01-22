//! Application State Management
//!
//! Manages shared state across Tauri commands for WRAITH Stream.

use crate::database::{Database, LocalIdentity};
use crate::error::StreamResult;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Transcode job progress
#[derive(Debug, Clone)]
pub struct TranscodeProgress {
    pub stream_id: String,
    pub progress: f32, // 0.0 to 1.0
    pub current_profile: String,
    pub status: TranscodeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranscodeStatus {
    Pending,
    Transcoding,
    Completed,
    Failed(String),
    Cancelled,
}

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// Application data directory
    pub app_data_dir: PathBuf,
    /// Streams directory
    pub streams_dir: PathBuf,
    /// Segments directory
    pub segments_dir: PathBuf,
    /// Thumbnails directory
    pub thumbnails_dir: PathBuf,
    /// Temporary directory for transcoding
    pub temp_dir: PathBuf,
    /// Local peer ID
    pub local_peer_id: Arc<RwLock<Option<String>>>,
    /// Display name
    pub display_name: Arc<RwLock<String>>,
    /// Active transcode jobs
    pub transcode_jobs: Arc<RwLock<HashMap<String, TranscodeProgress>>>,
    /// Cancelled transcode jobs
    pub cancelled_jobs: Arc<RwLock<Vec<String>>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database, app_data_dir: PathBuf) -> Self {
        let streams_dir = app_data_dir.join("streams");
        let segments_dir = app_data_dir.join("segments");
        let thumbnails_dir = app_data_dir.join("thumbnails");
        let temp_dir = app_data_dir.join("temp");

        Self {
            db: Arc::new(db),
            app_data_dir,
            streams_dir,
            segments_dir,
            thumbnails_dir,
            temp_dir,
            local_peer_id: Arc::new(RwLock::new(None)),
            display_name: Arc::new(RwLock::new("Anonymous".to_string())),
            transcode_jobs: Arc::new(RwLock::new(HashMap::new())),
            cancelled_jobs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize the application state (load or create identity)
    pub fn initialize(&self) -> StreamResult<()> {
        // Create directories
        std::fs::create_dir_all(&self.streams_dir)?;
        std::fs::create_dir_all(&self.segments_dir)?;
        std::fs::create_dir_all(&self.thumbnails_dir)?;
        std::fs::create_dir_all(&self.temp_dir)?;

        // Load or create identity
        if let Ok(Some(identity)) = self.db.get_local_identity() {
            self.load_identity(&identity)?;
            info!("Loaded existing identity: {}", identity.peer_id);
        } else {
            let identity = self.create_identity("Anonymous")?;
            info!("Created new identity: {}", identity.peer_id);
        }

        Ok(())
    }

    /// Load identity from database
    fn load_identity(&self, identity: &LocalIdentity) -> StreamResult<()> {
        *self.local_peer_id.write() = Some(identity.peer_id.clone());
        *self.display_name.write() = identity.display_name.clone();
        Ok(())
    }

    /// Create a new identity
    fn create_identity(&self, display_name: &str) -> StreamResult<LocalIdentity> {
        use rand::Rng;

        // Generate random peer ID
        let mut rng = rand::thread_rng();
        let mut id_bytes = [0u8; 16];
        rng.fill(&mut id_bytes);
        let peer_id = hex::encode(id_bytes);

        let identity = LocalIdentity {
            peer_id: peer_id.clone(),
            display_name: display_name.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };

        self.db.save_local_identity(&identity)?;

        *self.local_peer_id.write() = Some(peer_id);
        *self.display_name.write() = display_name.to_string();

        Ok(identity)
    }

    /// Get the local peer ID
    pub fn get_peer_id(&self) -> Option<String> {
        self.local_peer_id.read().clone()
    }

    /// Get the display name
    pub fn get_display_name(&self) -> String {
        self.display_name.read().clone()
    }

    /// Update the display name
    pub fn set_display_name(&self, name: &str) -> StreamResult<()> {
        if let Some(mut identity) = self.db.get_local_identity()? {
            identity.display_name = name.to_string();
            self.db.save_local_identity(&identity)?;
            *self.display_name.write() = name.to_string();
        }
        Ok(())
    }

    /// Get stream storage path
    pub fn get_stream_path(&self, stream_id: &str) -> PathBuf {
        self.streams_dir.join(stream_id)
    }

    /// Get segment storage path
    pub fn get_segment_path(&self, stream_id: &str, segment_name: &str) -> PathBuf {
        self.segments_dir.join(stream_id).join(segment_name)
    }

    /// Get thumbnail path
    pub fn get_thumbnail_path(&self, stream_id: &str) -> PathBuf {
        self.thumbnails_dir.join(format!("{}.jpg", stream_id))
    }

    /// Get temp path for transcoding
    pub fn get_temp_path(&self, stream_id: &str) -> PathBuf {
        self.temp_dir.join(stream_id)
    }

    /// Update transcode progress
    pub fn update_transcode_progress(&self, stream_id: &str, progress: TranscodeProgress) {
        self.transcode_jobs
            .write()
            .insert(stream_id.to_string(), progress);
    }

    /// Get transcode progress
    pub fn get_transcode_progress(&self, stream_id: &str) -> Option<TranscodeProgress> {
        self.transcode_jobs.read().get(stream_id).cloned()
    }

    /// Remove transcode job
    pub fn remove_transcode_job(&self, stream_id: &str) {
        self.transcode_jobs.write().remove(stream_id);
    }

    /// Cancel a transcode job
    pub fn cancel_transcode(&self, stream_id: &str) {
        self.cancelled_jobs.write().push(stream_id.to_string());
    }

    /// Check if a transcode job is cancelled
    pub fn is_transcode_cancelled(&self, stream_id: &str) -> bool {
        self.cancelled_jobs.read().contains(&stream_id.to_string())
    }

    /// Remove from cancelled list
    pub fn clear_cancelled(&self, stream_id: &str) {
        self.cancelled_jobs.write().retain(|id| id != stream_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_initialization() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        state.initialize().unwrap();

        assert!(state.get_peer_id().is_some());
        assert_eq!(state.get_display_name(), "Anonymous");
    }

    #[test]
    fn test_identity_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create identity
        {
            let db = Database::open(&db_path).unwrap();
            let state = AppState::new(db, dir.path().to_path_buf());
            state.initialize().unwrap();
            state.set_display_name("Test User").unwrap();
        }

        // Load identity
        {
            let db = Database::open(&db_path).unwrap();
            let state = AppState::new(db, dir.path().to_path_buf());
            state.initialize().unwrap();

            assert_eq!(state.get_display_name(), "Test User");
        }
    }

    #[test]
    fn test_transcode_progress() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        let progress = TranscodeProgress {
            stream_id: "stream-1".to_string(),
            progress: 0.5,
            current_profile: "720p".to_string(),
            status: TranscodeStatus::Transcoding,
        };

        state.update_transcode_progress("stream-1", progress.clone());

        let retrieved = state.get_transcode_progress("stream-1").unwrap();
        assert_eq!(retrieved.progress, 0.5);
        assert_eq!(retrieved.current_profile, "720p");
    }
}
