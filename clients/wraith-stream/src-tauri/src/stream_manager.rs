//! Stream Lifecycle Management
//!
//! Manages stream creation, updates, and deletion.

use crate::database::{Database, Stream, StreamQuality};
use crate::error::{StreamError, StreamResult};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Stream manager
pub struct StreamManager {
    pub(crate) db: Arc<Database>,
    state: Arc<AppState>,
}

/// Stream creation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStreamOptions {
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
}

/// Stream info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<i64>,
    pub is_live: bool,
    pub status: String,
    pub view_count: i64,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub qualities: Vec<String>,
}

/// Playback info for player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackInfo {
    pub stream_id: String,
    pub manifest_url: String,
    pub qualities: Vec<QualityInfo>,
    pub duration_secs: Option<f64>,
    pub subtitle_languages: Vec<String>,
}

/// Quality info for player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityInfo {
    pub name: String,
    pub width: i64,
    pub height: i64,
    pub bitrate: i64,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Create a new stream
    pub fn create_stream(&self, options: CreateStreamOptions) -> StreamResult<Stream> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| StreamError::NotInitialized("Identity not initialized".to_string()))?;

        let stream_id = Uuid::new_v4().to_string();

        let stream = Stream {
            id: stream_id.clone(),
            title: options.title,
            description: options.description,
            created_at: Utc::now().timestamp(),
            created_by: peer_id,
            thumbnail_hash: None,
            duration: None,
            is_live: false,
            status: "processing".to_string(),
            view_count: 0,
            category: options.category,
            tags: options.tags,
        };

        self.db.create_stream(&stream)?;

        info!("Created stream: {}", stream_id);
        Ok(stream)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: &str) -> StreamResult<Option<StreamInfo>> {
        let stream = match self.db.get_stream(stream_id)? {
            Some(s) => s,
            None => return Ok(None),
        };

        let qualities = self.db.list_qualities(stream_id)?;
        let quality_names: Vec<String> = qualities.iter().map(|q| q.quality.clone()).collect();

        Ok(Some(StreamInfo {
            id: stream.id,
            title: stream.title,
            description: stream.description,
            created_at: stream.created_at,
            created_by: stream.created_by,
            thumbnail_url: stream
                .thumbnail_hash
                .map(|h| format!("/thumbnails/{}.jpg", h)),
            duration: stream.duration,
            is_live: stream.is_live,
            status: stream.status,
            view_count: stream.view_count,
            category: stream.category,
            tags: stream.tags,
            qualities: quality_names,
        }))
    }

    /// List all streams
    pub fn list_streams(&self, limit: i64, offset: i64) -> StreamResult<Vec<StreamInfo>> {
        let streams = self.db.list_streams(limit, offset)?;

        let mut result = Vec::new();
        for stream in streams {
            let qualities = self.db.list_qualities(&stream.id)?;
            let quality_names: Vec<String> = qualities.iter().map(|q| q.quality.clone()).collect();

            result.push(StreamInfo {
                id: stream.id,
                title: stream.title,
                description: stream.description,
                created_at: stream.created_at,
                created_by: stream.created_by,
                thumbnail_url: stream
                    .thumbnail_hash
                    .map(|h| format!("/thumbnails/{}.jpg", h)),
                duration: stream.duration,
                is_live: stream.is_live,
                status: stream.status,
                view_count: stream.view_count,
                category: stream.category,
                tags: stream.tags,
                qualities: quality_names,
            });
        }

        Ok(result)
    }

    /// Get streams created by the current user
    pub fn get_my_streams(&self) -> StreamResult<Vec<StreamInfo>> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| StreamError::NotInitialized("Identity not initialized".to_string()))?;

        let streams = self.db.list_streams_by_creator(&peer_id)?;

        let mut result = Vec::new();
        for stream in streams {
            let qualities = self.db.list_qualities(&stream.id)?;
            let quality_names: Vec<String> = qualities.iter().map(|q| q.quality.clone()).collect();

            result.push(StreamInfo {
                id: stream.id,
                title: stream.title,
                description: stream.description,
                created_at: stream.created_at,
                created_by: stream.created_by,
                thumbnail_url: stream
                    .thumbnail_hash
                    .map(|h| format!("/thumbnails/{}.jpg", h)),
                duration: stream.duration,
                is_live: stream.is_live,
                status: stream.status,
                view_count: stream.view_count,
                category: stream.category,
                tags: stream.tags,
                qualities: quality_names,
            });
        }

        Ok(result)
    }

    /// Update stream metadata
    pub fn update_stream(
        &self,
        stream_id: &str,
        title: Option<String>,
        description: Option<String>,
        category: Option<String>,
        tags: Option<String>,
    ) -> StreamResult<StreamInfo> {
        let mut stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        // Verify ownership
        let peer_id = self.state.get_peer_id();
        if peer_id.as_ref() != Some(&stream.created_by) {
            return Err(StreamError::NotInitialized(
                "Not authorized to update this stream".to_string(),
            ));
        }

        // Update fields
        if let Some(t) = title {
            stream.title = t;
        }
        if let Some(d) = description {
            stream.description = Some(d);
        }
        if let Some(c) = category {
            stream.category = Some(c);
        }
        if let Some(t) = tags {
            stream.tags = Some(t);
        }

        self.db.update_stream(&stream)?;

        let qualities = self.db.list_qualities(stream_id)?;
        let quality_names: Vec<String> = qualities.iter().map(|q| q.quality.clone()).collect();

        Ok(StreamInfo {
            id: stream.id,
            title: stream.title,
            description: stream.description,
            created_at: stream.created_at,
            created_by: stream.created_by,
            thumbnail_url: stream
                .thumbnail_hash
                .map(|h| format!("/thumbnails/{}.jpg", h)),
            duration: stream.duration,
            is_live: stream.is_live,
            status: stream.status,
            view_count: stream.view_count,
            category: stream.category,
            tags: stream.tags,
            qualities: quality_names,
        })
    }

    /// Set stream status
    pub fn set_stream_status(&self, stream_id: &str, status: &str) -> StreamResult<()> {
        let mut stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        stream.status = status.to_string();
        self.db.update_stream(&stream)?;

        info!("Stream {} status set to {}", stream_id, status);
        Ok(())
    }

    /// Set stream duration
    pub fn set_stream_duration(&self, stream_id: &str, duration_secs: f64) -> StreamResult<()> {
        let mut stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        stream.duration = Some(duration_secs as i64);
        self.db.update_stream(&stream)?;

        Ok(())
    }

    /// Add quality level to stream
    #[allow(clippy::too_many_arguments)]
    pub fn add_quality(
        &self,
        stream_id: &str,
        quality: &str,
        width: i64,
        height: i64,
        video_bitrate: i64,
        audio_bitrate: i64,
        segment_count: i64,
    ) -> StreamResult<()> {
        let quality_record = StreamQuality {
            stream_id: stream_id.to_string(),
            quality: quality.to_string(),
            width,
            height,
            video_bitrate,
            audio_bitrate,
            segment_count,
        };

        self.db.upsert_quality(&quality_record)?;
        Ok(())
    }

    /// Delete a stream
    pub fn delete_stream(&self, stream_id: &str) -> StreamResult<()> {
        // Verify stream exists
        let stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        // Verify ownership
        let peer_id = self.state.get_peer_id();
        if peer_id.as_ref() != Some(&stream.created_by) {
            return Err(StreamError::NotInitialized(
                "Not authorized to delete this stream".to_string(),
            ));
        }

        // Delete from database (cascades to segments, qualities, views, subtitles)
        self.db.delete_stream(stream_id)?;

        // Delete files
        let stream_dir = self.state.get_stream_path(stream_id);
        if stream_dir.exists() {
            std::fs::remove_dir_all(&stream_dir)?;
        }

        let segment_dir = self.state.segments_dir.join(stream_id);
        if segment_dir.exists() {
            std::fs::remove_dir_all(&segment_dir)?;
        }

        let thumbnail = self.state.get_thumbnail_path(stream_id);
        if thumbnail.exists() {
            std::fs::remove_file(&thumbnail)?;
        }

        info!("Deleted stream: {}", stream_id);
        Ok(())
    }

    /// Get playback info for a stream
    pub fn get_playback_info(&self, stream_id: &str) -> StreamResult<PlaybackInfo> {
        let stream = self
            .db
            .get_stream(stream_id)?
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        let qualities = self.db.list_qualities(stream_id)?;
        let quality_infos: Vec<QualityInfo> = qualities
            .iter()
            .map(|q| QualityInfo {
                name: q.quality.clone(),
                width: q.width,
                height: q.height,
                bitrate: q.video_bitrate + q.audio_bitrate,
            })
            .collect();

        let subtitles = self.db.list_subtitle_languages(stream_id)?;
        let subtitle_languages: Vec<String> =
            subtitles.iter().map(|s| s.language.clone()).collect();

        Ok(PlaybackInfo {
            stream_id: stream_id.to_string(),
            manifest_url: format!("/segments/{}/master.m3u8", stream_id),
            qualities: quality_infos,
            duration_secs: stream.duration.map(|d| d as f64),
            subtitle_languages,
        })
    }

    /// Record a view
    pub fn record_view(&self, stream_id: &str) -> StreamResult<()> {
        // Increment view count
        self.db.increment_view_count(stream_id)?;

        // Record view details
        let peer_id = self
            .state
            .get_peer_id()
            .unwrap_or_else(|| "anonymous".to_string());

        let view = crate::database::StreamView {
            stream_id: stream_id.to_string(),
            peer_id,
            started_at: Utc::now().timestamp(),
            watch_time: 0,
            last_position: 0,
            quality: None,
        };

        self.db.record_view(&view)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manager() -> (StreamManager, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        (StreamManager::new(db, state), dir)
    }

    #[test]
    fn test_create_stream() {
        let (manager, _dir) = create_test_manager();

        let options = CreateStreamOptions {
            title: "Test Stream".to_string(),
            description: Some("A test stream".to_string()),
            category: Some("Technology".to_string()),
            tags: Some("test,demo".to_string()),
        };

        let stream = manager.create_stream(options).unwrap();
        assert_eq!(stream.title, "Test Stream");
        assert_eq!(stream.status, "processing");
    }

    #[test]
    fn test_get_stream() {
        let (manager, _dir) = create_test_manager();

        let options = CreateStreamOptions {
            title: "Test Stream".to_string(),
            description: None,
            category: None,
            tags: None,
        };

        let created = manager.create_stream(options).unwrap();
        let retrieved = manager.get_stream(&created.id).unwrap().unwrap();

        assert_eq!(retrieved.title, "Test Stream");
    }

    #[test]
    fn test_update_stream() {
        let (manager, _dir) = create_test_manager();

        let options = CreateStreamOptions {
            title: "Original Title".to_string(),
            description: None,
            category: None,
            tags: None,
        };

        let stream = manager.create_stream(options).unwrap();

        let updated = manager
            .update_stream(
                &stream.id,
                Some("Updated Title".to_string()),
                Some("New description".to_string()),
                None,
                None,
            )
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.description, Some("New description".to_string()));
    }
}
