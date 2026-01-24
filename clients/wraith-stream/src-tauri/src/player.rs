//! Playback Engine
//!
//! Manages video playback, quality selection, and buffer management.

use crate::database::Database;
use crate::error::{StreamError, StreamResult};
use crate::state::AppState;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Player state for a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub stream_id: String,
    pub current_quality: String,
    pub position_ms: i64,
    pub is_playing: bool,
    pub buffered_segments: Vec<String>,
    pub available_qualities: Vec<String>,
}

/// Buffered segment info
#[derive(Debug, Clone)]
struct BufferedSegment {
    segment_name: String,
    quality: String,
    data: Vec<u8>,
}

/// Playback player
pub struct Player {
    db: Arc<Database>,
    #[allow(dead_code)]
    state: Arc<AppState>,
    player_states: RwLock<HashMap<String, PlayerState>>,
    buffered_segments: RwLock<HashMap<String, Vec<BufferedSegment>>>,
}

impl Player {
    /// Create a new player
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self {
            db,
            state,
            player_states: RwLock::new(HashMap::new()),
            buffered_segments: RwLock::new(HashMap::new()),
        }
    }

    /// Load a stream for playback
    pub fn load_stream(&self, stream_id: &str) -> StreamResult<PlayerState> {
        // Get stream info
        let stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        if stream.status != "ready" && stream.status != "live" {
            return Err(StreamError::Playback(format!(
                "Stream not ready: status = {}",
                stream.status
            )));
        }

        // Get available qualities
        let qualities = self.db.list_qualities(stream_id)?;
        let available_qualities: Vec<String> =
            qualities.iter().map(|q| q.quality.clone()).collect();

        if available_qualities.is_empty() {
            return Err(StreamError::Playback(
                "No quality levels available".to_string(),
            ));
        }

        // Select initial quality (prefer 720p or highest available)
        let initial_quality = available_qualities
            .iter()
            .find(|q| q.as_str() == "720p")
            .or_else(|| available_qualities.last())
            .cloned()
            .unwrap_or_else(|| "480p".to_string());

        let player_state = PlayerState {
            stream_id: stream_id.to_string(),
            current_quality: initial_quality,
            position_ms: 0,
            is_playing: false,
            buffered_segments: Vec::new(),
            available_qualities,
        };

        // Store player state
        self.player_states
            .write()
            .insert(stream_id.to_string(), player_state.clone());

        debug!("Loaded stream {} for playback", stream_id);
        Ok(player_state)
    }

    /// Set playback quality
    pub fn set_quality(&self, stream_id: &str, quality: &str) -> StreamResult<PlayerState> {
        let mut states = self.player_states.write();
        let state = states
            .get_mut(stream_id)
            .ok_or_else(|| StreamError::Playback("Stream not loaded".to_string()))?;

        if !state.available_qualities.contains(&quality.to_string()) {
            return Err(StreamError::Playback(format!(
                "Quality {} not available",
                quality
            )));
        }

        state.current_quality = quality.to_string();

        // Clear buffered segments (will rebuffer at new quality)
        self.buffered_segments.write().remove(stream_id);

        debug!("Set quality to {} for stream {}", quality, stream_id);
        Ok(state.clone())
    }

    /// Update playback position
    pub fn update_position(&self, stream_id: &str, position_ms: i64) -> StreamResult<()> {
        let mut states = self.player_states.write();
        let state = states
            .get_mut(stream_id)
            .ok_or_else(|| StreamError::Playback("Stream not loaded".to_string()))?;

        state.position_ms = position_ms;
        Ok(())
    }

    /// Set playing state
    pub fn set_playing(&self, stream_id: &str, is_playing: bool) -> StreamResult<()> {
        let mut states = self.player_states.write();
        let state = states
            .get_mut(stream_id)
            .ok_or_else(|| StreamError::Playback("Stream not loaded".to_string()))?;

        state.is_playing = is_playing;
        Ok(())
    }

    /// Get current player state
    pub fn get_state(&self, stream_id: &str) -> StreamResult<PlayerState> {
        let states = self.player_states.read();
        states
            .get(stream_id)
            .cloned()
            .ok_or_else(|| StreamError::Playback("Stream not loaded".to_string()))
    }

    /// Buffer a segment
    pub fn buffer_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        quality: &str,
        data: Vec<u8>,
    ) -> StreamResult<()> {
        let mut buffers = self.buffered_segments.write();
        let buffer = buffers.entry(stream_id.to_string()).or_default();

        // Check if already buffered
        if buffer
            .iter()
            .any(|s| s.segment_name == segment_name && s.quality == quality)
        {
            return Ok(());
        }

        buffer.push(BufferedSegment {
            segment_name: segment_name.to_string(),
            quality: quality.to_string(),
            data,
        });

        // Update player state
        if let Some(state) = self.player_states.write().get_mut(stream_id)
            && !state.buffered_segments.contains(&segment_name.to_string())
        {
            state.buffered_segments.push(segment_name.to_string());
        }

        // Limit buffer size (keep last 10 segments)
        if buffer.len() > 10 {
            buffer.remove(0);
        }

        Ok(())
    }

    /// Get buffered segment
    pub fn get_buffered_segment(&self, stream_id: &str, segment_name: &str) -> Option<Vec<u8>> {
        let buffers = self.buffered_segments.read();
        buffers.get(stream_id).and_then(|buffer| {
            buffer
                .iter()
                .find(|s| s.segment_name == segment_name)
                .map(|s| s.data.clone())
        })
    }

    /// Clear buffer for a stream
    pub fn clear_buffer(&self, stream_id: &str) {
        self.buffered_segments.write().remove(stream_id);

        if let Some(state) = self.player_states.write().get_mut(stream_id) {
            state.buffered_segments.clear();
        }
    }

    /// Unload a stream
    pub fn unload_stream(&self, stream_id: &str) {
        self.player_states.write().remove(stream_id);
        self.buffered_segments.write().remove(stream_id);
        debug!("Unloaded stream {}", stream_id);
    }

    /// Get list of loaded streams
    pub fn get_loaded_streams(&self) -> Vec<String> {
        self.player_states.read().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Stream, StreamQuality};
    use chrono::Utc;
    use tempfile::tempdir;

    fn create_test_player() -> (Player, Arc<Database>, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        (Player::new(db.clone(), state), db, dir)
    }

    #[test]
    fn test_load_stream() {
        let (player, db, _dir) = create_test_player();

        // Create a test stream
        let stream = Stream {
            id: "test-stream".to_string(),
            title: "Test".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-1".to_string(),
            thumbnail_hash: None,
            duration: Some(3600),
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: None,
            tags: None,
        };
        db.create_stream(&stream).unwrap();

        // Add quality level
        let quality = StreamQuality {
            stream_id: "test-stream".to_string(),
            quality: "720p".to_string(),
            width: 1280,
            height: 720,
            video_bitrate: 2500000,
            audio_bitrate: 128000,
            segment_count: 100,
        };
        db.upsert_quality(&quality).unwrap();

        // Load stream
        let state = player.load_stream("test-stream").unwrap();
        assert_eq!(state.stream_id, "test-stream");
        assert_eq!(state.current_quality, "720p");
        assert!(!state.is_playing);
    }

    #[test]
    fn test_set_quality() {
        let (player, db, _dir) = create_test_player();

        // Create a test stream with multiple qualities
        let stream = Stream {
            id: "test-stream".to_string(),
            title: "Test".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-1".to_string(),
            thumbnail_hash: None,
            duration: Some(3600),
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: None,
            tags: None,
        };
        db.create_stream(&stream).unwrap();

        for (name, width, height) in [("480p", 854, 480), ("720p", 1280, 720)] {
            let quality = StreamQuality {
                stream_id: "test-stream".to_string(),
                quality: name.to_string(),
                width,
                height,
                video_bitrate: 2500000,
                audio_bitrate: 128000,
                segment_count: 100,
            };
            db.upsert_quality(&quality).unwrap();
        }

        // Load and change quality
        player.load_stream("test-stream").unwrap();
        let state = player.set_quality("test-stream", "480p").unwrap();
        assert_eq!(state.current_quality, "480p");
    }

    #[test]
    fn test_buffer_segment() {
        let (player, db, _dir) = create_test_player();

        // Create stream
        let stream = Stream {
            id: "test-stream".to_string(),
            title: "Test".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-1".to_string(),
            thumbnail_hash: None,
            duration: Some(3600),
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: None,
            tags: None,
        };
        db.create_stream(&stream).unwrap();

        let quality = StreamQuality {
            stream_id: "test-stream".to_string(),
            quality: "720p".to_string(),
            width: 1280,
            height: 720,
            video_bitrate: 2500000,
            audio_bitrate: 128000,
            segment_count: 100,
        };
        db.upsert_quality(&quality).unwrap();

        // Load stream
        player.load_stream("test-stream").unwrap();

        // Buffer segment
        let data = vec![1, 2, 3, 4, 5];
        player
            .buffer_segment("test-stream", "720p_001.ts", "720p", data.clone())
            .unwrap();

        // Retrieve buffered segment
        let retrieved = player.get_buffered_segment("test-stream", "720p_001.ts");
        assert_eq!(retrieved, Some(data));
    }
}
