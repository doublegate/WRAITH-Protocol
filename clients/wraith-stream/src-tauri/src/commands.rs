//! Tauri IPC Commands for WRAITH Stream
//!
//! Provides the command interface between the frontend and backend.

use crate::database::Stream;
use crate::discovery::{SearchResults, StreamDiscovery, StreamSummary};
use crate::error::StreamError;
use crate::player::{Player, PlayerState};
use crate::segment_storage::SegmentStorage;
use crate::state::{AppState, TranscodeStatus};
use crate::stream_manager::{CreateStreamOptions, PlaybackInfo, StreamInfo, StreamManager};
use crate::subtitles::{SubtitleCue, SubtitleManager};
use crate::transcoder::Transcoder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tracing::info;

/// Application result type for Tauri commands
type CmdResult<T> = Result<T, StreamError>;

/// Shared managers state
pub struct Managers {
    pub stream_manager: StreamManager,
    pub segment_storage: SegmentStorage,
    pub transcoder: Transcoder,
    pub player: Player,
    pub discovery: StreamDiscovery,
    pub subtitles: SubtitleManager,
}

// =============================================================================
// Stream Management Commands
// =============================================================================

/// Create a new stream
#[tauri::command]
pub async fn create_stream(
    managers: State<'_, Managers>,
    title: String,
    description: Option<String>,
    category: Option<String>,
    tags: Option<String>,
) -> CmdResult<Stream> {
    let options = CreateStreamOptions {
        title,
        description,
        category,
        tags,
    };

    managers.stream_manager.create_stream(options)
}

/// Upload and transcode a video
#[tauri::command]
pub async fn upload_video(
    state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    stream_id: String,
    file_path: String,
) -> CmdResult<StreamInfo> {
    let input_path = PathBuf::from(&file_path);

    if !input_path.exists() {
        return Err(StreamError::FileSystem(format!(
            "File not found: {}",
            file_path
        )));
    }

    // Create output directory
    let output_dir = state.get_temp_path(&stream_id);
    std::fs::create_dir_all(&output_dir)?;

    // Generate thumbnail
    let thumbnail_path = state.get_thumbnail_path(&stream_id);
    if let Err(e) = Transcoder::generate_thumbnail(&input_path, &thumbnail_path).await {
        tracing::warn!("Failed to generate thumbnail: {}", e);
    }

    // Transcode video
    let manifest = managers
        .transcoder
        .transcode(&stream_id, &input_path, &output_dir)
        .await?;

    info!(
        "Transcoded {} to {} quality levels",
        stream_id,
        manifest.qualities.len()
    );

    // Generate encryption key
    let stream_key = SegmentStorage::generate_stream_key();

    // Upload segments
    let segment_count = managers
        .segment_storage
        .upload_stream_segments(&stream_id, &output_dir, &stream_key)
        .await?;

    info!(
        "Uploaded {} segments for stream {}",
        segment_count, stream_id
    );

    // Update stream metadata
    managers
        .stream_manager
        .set_stream_duration(&stream_id, manifest.duration_secs)?;

    // Add quality levels
    for quality in &manifest.qualities {
        managers.stream_manager.add_quality(
            &stream_id,
            &quality.profile.name,
            quality.profile.width as i64,
            quality.profile.height as i64,
            quality.profile.video_bitrate as i64,
            quality.profile.audio_bitrate as i64,
            quality.segment_count as i64,
        )?;
    }

    // Set status to ready
    managers
        .stream_manager
        .set_stream_status(&stream_id, "ready")?;

    // Clean up temp directory
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir).ok();
    }

    // Return stream info
    managers
        .stream_manager
        .get_stream(&stream_id)?
        .ok_or(StreamError::StreamNotFound(stream_id))
}

/// Delete a stream
#[tauri::command]
pub async fn delete_stream(managers: State<'_, Managers>, stream_id: String) -> CmdResult<()> {
    // Delete segments first
    managers
        .segment_storage
        .delete_stream_segments(&stream_id)?;

    // Delete stream
    managers.stream_manager.delete_stream(&stream_id)
}

/// List all streams
#[tauri::command]
pub async fn list_streams(
    managers: State<'_, Managers>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> CmdResult<Vec<StreamInfo>> {
    managers
        .stream_manager
        .list_streams(limit.unwrap_or(50), offset.unwrap_or(0))
}

/// Get a stream by ID
#[tauri::command]
pub async fn get_stream(
    managers: State<'_, Managers>,
    stream_id: String,
) -> CmdResult<Option<StreamInfo>> {
    managers.stream_manager.get_stream(&stream_id)
}

/// Update stream metadata
#[tauri::command]
pub async fn update_stream(
    managers: State<'_, Managers>,
    stream_id: String,
    title: Option<String>,
    description: Option<String>,
    category: Option<String>,
    tags: Option<String>,
) -> CmdResult<StreamInfo> {
    managers
        .stream_manager
        .update_stream(&stream_id, title, description, category, tags)
}

/// Get stream status
#[tauri::command]
pub async fn get_stream_status(
    managers: State<'_, Managers>,
    stream_id: String,
) -> CmdResult<String> {
    let stream = managers
        .stream_manager
        .get_stream(&stream_id)?
        .ok_or(StreamError::StreamNotFound(stream_id))?;

    Ok(stream.status)
}

// =============================================================================
// Playback Commands
// =============================================================================

/// Load a stream for playback
#[tauri::command]
pub async fn load_stream(
    managers: State<'_, Managers>,
    stream_id: String,
) -> CmdResult<PlayerState> {
    managers.player.load_stream(&stream_id)
}

/// Get a segment
#[tauri::command]
pub async fn get_segment(
    managers: State<'_, Managers>,
    stream_id: String,
    segment_name: String,
) -> CmdResult<Vec<u8>> {
    // Check if already buffered
    if let Some(data) = managers
        .player
        .get_buffered_segment(&stream_id, &segment_name)
    {
        return Ok(data);
    }

    // For now, return raw segment (unencrypted for demo)
    // In production, would use a proper key management system
    let stream_key = [0u8; 32]; // Placeholder key

    let data = managers
        .segment_storage
        .download_segment(&stream_id, &segment_name, &stream_key)
        .await?;

    // Buffer the segment
    let quality = segment_name
        .split('_')
        .next()
        .unwrap_or("unknown")
        .to_string();
    managers
        .player
        .buffer_segment(&stream_id, &segment_name, &quality, data.clone())?;

    Ok(data)
}

/// Get manifest content
#[tauri::command]
pub async fn get_manifest(managers: State<'_, Managers>, stream_id: String) -> CmdResult<String> {
    managers
        .segment_storage
        .get_manifest(&stream_id, "master.m3u8")
}

/// Set playback quality
#[tauri::command]
pub async fn set_quality(
    managers: State<'_, Managers>,
    stream_id: String,
    quality: String,
) -> CmdResult<PlayerState> {
    managers.player.set_quality(&stream_id, &quality)
}

/// Get playback info
#[tauri::command]
pub async fn get_playback_info(
    managers: State<'_, Managers>,
    stream_id: String,
) -> CmdResult<PlaybackInfo> {
    managers.stream_manager.get_playback_info(&stream_id)
}

// =============================================================================
// Discovery Commands
// =============================================================================

/// Search streams
#[tauri::command]
pub async fn search_streams(
    managers: State<'_, Managers>,
    query: String,
    limit: Option<i64>,
) -> CmdResult<SearchResults> {
    managers.discovery.search(&query, limit.unwrap_or(20))
}

/// Get trending streams
#[tauri::command]
pub async fn get_trending_streams(
    managers: State<'_, Managers>,
    limit: Option<i64>,
) -> CmdResult<Vec<StreamSummary>> {
    managers.discovery.get_trending(limit.unwrap_or(10))
}

/// Get my streams
#[tauri::command]
pub async fn get_my_streams(managers: State<'_, Managers>) -> CmdResult<Vec<StreamInfo>> {
    managers.stream_manager.get_my_streams()
}

// =============================================================================
// Subtitle Commands
// =============================================================================

/// Load subtitles for a stream
#[tauri::command]
pub async fn load_subtitles(
    managers: State<'_, Managers>,
    stream_id: String,
    language: String,
) -> CmdResult<Vec<SubtitleCue>> {
    managers.subtitles.load_subtitles(&stream_id, &language)
}

/// Add subtitles to a stream
#[tauri::command]
pub async fn add_subtitles(
    managers: State<'_, Managers>,
    stream_id: String,
    language: String,
    label: String,
    content: String,
) -> CmdResult<()> {
    managers
        .subtitles
        .add_subtitles(&stream_id, &language, &label, &content)?;
    Ok(())
}

/// List subtitle languages
#[tauri::command]
pub async fn list_subtitle_languages(
    managers: State<'_, Managers>,
    stream_id: String,
) -> CmdResult<Vec<crate::database::SubtitleInfo>> {
    managers.subtitles.list_languages(&stream_id)
}

// =============================================================================
// View Commands
// =============================================================================

/// Record a view
#[tauri::command]
pub async fn record_view(managers: State<'_, Managers>, stream_id: String) -> CmdResult<()> {
    managers.stream_manager.record_view(&stream_id)
}

/// Get stream views
#[tauri::command]
pub async fn get_stream_views(
    managers: State<'_, Managers>,
    stream_id: String,
    limit: Option<i64>,
) -> CmdResult<Vec<crate::database::StreamView>> {
    managers
        .stream_manager
        .db
        .get_stream_views(&stream_id, limit.unwrap_or(100))
        .map_err(|e| StreamError::Database(e.to_string()))
}

// =============================================================================
// Identity Commands
// =============================================================================

/// Get local peer ID
#[tauri::command]
pub async fn get_peer_id(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    state
        .get_peer_id()
        .ok_or_else(|| StreamError::NotInitialized("Identity not initialized".to_string()))
}

/// Get display name
#[tauri::command]
pub async fn get_display_name(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    Ok(state.get_display_name())
}

/// Set display name
#[tauri::command]
pub async fn set_display_name(state: State<'_, Arc<AppState>>, name: String) -> CmdResult<()> {
    state.set_display_name(&name)
}

// =============================================================================
// Transcode Progress Commands
// =============================================================================

/// Get transcode progress
#[tauri::command]
pub async fn get_transcode_progress(
    state: State<'_, Arc<AppState>>,
    stream_id: String,
) -> CmdResult<Option<TranscodeProgressInfo>> {
    Ok(state
        .get_transcode_progress(&stream_id)
        .map(|p| TranscodeProgressInfo {
            stream_id: p.stream_id,
            progress: p.progress,
            current_profile: p.current_profile,
            status: match p.status {
                TranscodeStatus::Pending => "pending".to_string(),
                TranscodeStatus::Transcoding => "transcoding".to_string(),
                TranscodeStatus::Completed => "completed".to_string(),
                TranscodeStatus::Failed(msg) => format!("failed: {}", msg),
                TranscodeStatus::Cancelled => "cancelled".to_string(),
            },
        }))
}

/// Cancel a transcode job
#[tauri::command]
pub async fn cancel_transcode(state: State<'_, Arc<AppState>>, stream_id: String) -> CmdResult<()> {
    state.cancel_transcode(&stream_id);
    Ok(())
}

/// Transcode progress info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeProgressInfo {
    pub stream_id: String,
    pub progress: f32,
    pub current_profile: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcode_progress_info() {
        let info = TranscodeProgressInfo {
            stream_id: "test-stream".to_string(),
            progress: 0.5,
            current_profile: "720p".to_string(),
            status: "transcoding".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"progress\":0.5"));
        assert!(json.contains("\"status\":\"transcoding\""));
    }
}
