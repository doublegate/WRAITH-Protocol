//! Video Transcoding Module
//!
//! Implements FFmpeg-based transcoding to multiple quality levels (HLS).

use crate::error::{StreamError, StreamResult};
use crate::state::{AppState, TranscodeProgress, TranscodeStatus};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Transcode profile defining output quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeProfile {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub video_bitrate: u32, // in kbps
    pub audio_bitrate: u32, // in kbps
}

/// Default transcode profiles
pub fn default_profiles() -> Vec<TranscodeProfile> {
    vec![
        TranscodeProfile {
            name: "240p".to_string(),
            width: 426,
            height: 240,
            video_bitrate: 400,
            audio_bitrate: 64,
        },
        TranscodeProfile {
            name: "480p".to_string(),
            width: 854,
            height: 480,
            video_bitrate: 1000,
            audio_bitrate: 96,
        },
        TranscodeProfile {
            name: "720p".to_string(),
            width: 1280,
            height: 720,
            video_bitrate: 2500,
            audio_bitrate: 128,
        },
        TranscodeProfile {
            name: "1080p".to_string(),
            width: 1920,
            height: 1080,
            video_bitrate: 5000,
            audio_bitrate: 192,
        },
    ]
}

/// Stream manifest containing all quality levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamManifest {
    pub stream_id: String,
    pub master_playlist: String,
    pub qualities: Vec<QualityManifest>,
    pub duration_secs: f64,
}

/// Manifest for a single quality level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityManifest {
    pub profile: TranscodeProfile,
    pub playlist_path: PathBuf,
    pub segment_count: usize,
    pub segment_duration_secs: f64,
}

/// Video transcoder
pub struct Transcoder {
    state: Arc<AppState>,
    profiles: Vec<TranscodeProfile>,
    segment_duration: u32, // in seconds
}

impl Transcoder {
    /// Create a new transcoder
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            profiles: default_profiles(),
            segment_duration: 6,
        }
    }

    /// Check if FFmpeg is available
    pub async fn check_ffmpeg() -> StreamResult<String> {
        let output = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .await
            .map_err(|_| StreamError::FfmpegNotFound)?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let first_line = version.lines().next().unwrap_or("FFmpeg");
            Ok(first_line.to_string())
        } else {
            Err(StreamError::FfmpegNotFound)
        }
    }

    /// Get video duration using ffprobe
    pub async fn get_duration(input: &Path) -> StreamResult<f64> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(input)
            .output()
            .await
            .map_err(|e| StreamError::FfmpegError(format!("Failed to run ffprobe: {}", e)))?;

        if output.status.success() {
            let duration_str = String::from_utf8_lossy(&output.stdout);
            duration_str
                .trim()
                .parse::<f64>()
                .map_err(|e| StreamError::FfmpegError(format!("Failed to parse duration: {}", e)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StreamError::FfmpegError(format!(
                "ffprobe failed: {}",
                stderr
            )))
        }
    }

    /// Generate a thumbnail from the video
    pub async fn generate_thumbnail(input: &Path, output: &Path) -> StreamResult<()> {
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
            ])
            .arg(input)
            .args([
                "-ss", "00:00:05",  // Seek to 5 seconds
                "-vframes", "1",
                "-vf", "scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2",
                "-q:v", "2",
            ])
            .arg(output)
            .status()
            .await
            .map_err(|e| StreamError::FfmpegError(format!("Failed to generate thumbnail: {}", e)))?;

        if status.success() {
            Ok(())
        } else {
            Err(StreamError::FfmpegError(
                "Thumbnail generation failed".to_string(),
            ))
        }
    }

    /// Transcode video to multiple quality levels
    pub async fn transcode(
        &self,
        stream_id: &str,
        input: &Path,
        output_dir: &Path,
    ) -> StreamResult<StreamManifest> {
        // Check FFmpeg availability
        Self::check_ffmpeg().await?;

        // Create output directory
        std::fs::create_dir_all(output_dir)?;

        // Get video duration
        let duration = Self::get_duration(input).await?;
        info!("Video duration: {:.2} seconds", duration);

        // Initialize progress
        self.state.update_transcode_progress(
            stream_id,
            TranscodeProgress {
                stream_id: stream_id.to_string(),
                progress: 0.0,
                current_profile: "Starting".to_string(),
                status: TranscodeStatus::Transcoding,
            },
        );

        let mut qualities = Vec::new();
        let total_profiles = self.profiles.len();

        // Transcode each quality level
        for (idx, profile) in self.profiles.iter().enumerate() {
            // Check for cancellation
            if self.state.is_transcode_cancelled(stream_id) {
                self.state.clear_cancelled(stream_id);
                self.state.update_transcode_progress(
                    stream_id,
                    TranscodeProgress {
                        stream_id: stream_id.to_string(),
                        progress: 0.0,
                        current_profile: profile.name.clone(),
                        status: TranscodeStatus::Cancelled,
                    },
                );
                return Err(StreamError::TranscodeCancelled);
            }

            info!(
                "Transcoding to {} ({}/{})",
                profile.name,
                idx + 1,
                total_profiles
            );

            self.state.update_transcode_progress(
                stream_id,
                TranscodeProgress {
                    stream_id: stream_id.to_string(),
                    progress: idx as f32 / total_profiles as f32,
                    current_profile: profile.name.clone(),
                    status: TranscodeStatus::Transcoding,
                },
            );

            let quality_manifest = self
                .transcode_quality(stream_id, input, output_dir, profile, duration)
                .await?;
            qualities.push(quality_manifest);
        }

        // Generate master playlist
        let master_playlist = self.generate_master_playlist(stream_id, &qualities, output_dir)?;

        // Update progress to complete
        self.state.update_transcode_progress(
            stream_id,
            TranscodeProgress {
                stream_id: stream_id.to_string(),
                progress: 1.0,
                current_profile: "Complete".to_string(),
                status: TranscodeStatus::Completed,
            },
        );

        Ok(StreamManifest {
            stream_id: stream_id.to_string(),
            master_playlist,
            qualities,
            duration_secs: duration,
        })
    }

    /// Transcode to a single quality level
    async fn transcode_quality(
        &self,
        stream_id: &str,
        input: &Path,
        output_dir: &Path,
        profile: &TranscodeProfile,
        duration: f64,
    ) -> StreamResult<QualityManifest> {
        let playlist_name = format!("{}.m3u8", profile.name);
        let playlist_path = output_dir.join(&playlist_name);
        let segment_pattern = output_dir.join(format!("{}_%%03d.ts", profile.name));

        // Build FFmpeg command
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-i"])
            .arg(input)
            .args([
                // Video encoding
                "-c:v", "libx264",
                "-preset", "medium",
                "-profile:v", "main",
                "-level", "4.0",
                "-b:v", &format!("{}k", profile.video_bitrate),
                "-maxrate", &format!("{}k", (profile.video_bitrate as f32 * 1.5) as u32),
                "-bufsize", &format!("{}k", profile.video_bitrate * 2),
                "-vf", &format!("scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2",
                    profile.width, profile.height, profile.width, profile.height),
                // Audio encoding
                "-c:a", "aac",
                "-b:a", &format!("{}k", profile.audio_bitrate),
                "-ar", "48000",
                "-ac", "2",
                // HLS output
                "-f", "hls",
                "-hls_time", &self.segment_duration.to_string(),
                "-hls_playlist_type", "vod",
                "-hls_segment_filename",
            ])
            .arg(&segment_pattern)
            .arg(&playlist_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("FFmpeg command: {:?}", cmd);

        let mut child = cmd
            .spawn()
            .map_err(|e| StreamError::FfmpegError(format!("Failed to spawn FFmpeg: {}", e)))?;

        // Read stderr for progress
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Check for cancellation
                if self.state.is_transcode_cancelled(stream_id) {
                    child.kill().await.ok();
                    return Err(StreamError::TranscodeCancelled);
                }

                // Parse progress from FFmpeg output
                if line.contains("time=") {
                    if let Some(time_str) = extract_time(&line) {
                        let current_time = parse_time_to_seconds(&time_str);
                        let progress = (current_time / duration).min(1.0);
                        debug!("Transcode progress: {:.1}%", progress * 100.0);
                    }
                }

                // Log errors
                if line.contains("Error") || line.contains("error") {
                    warn!("FFmpeg: {}", line);
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| StreamError::FfmpegError(format!("FFmpeg process error: {}", e)))?;

        if !status.success() {
            return Err(StreamError::FfmpegError(format!(
                "FFmpeg exited with code: {:?}",
                status.code()
            )));
        }

        // Count generated segments
        let segment_count = std::fs::read_dir(output_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with(&profile.name)
                    && e.path().extension().is_some_and(|ext| ext == "ts")
            })
            .count();

        info!("Generated {} segments for {}", segment_count, profile.name);

        Ok(QualityManifest {
            profile: profile.clone(),
            playlist_path,
            segment_count,
            segment_duration_secs: self.segment_duration as f64,
        })
    }

    /// Generate master HLS playlist
    fn generate_master_playlist(
        &self,
        _stream_id: &str,
        qualities: &[QualityManifest],
        output_dir: &Path,
    ) -> StreamResult<String> {
        let mut master = String::from("#EXTM3U\n#EXT-X-VERSION:3\n\n");

        for quality in qualities {
            let bandwidth = (quality.profile.video_bitrate + quality.profile.audio_bitrate) * 1000;
            master.push_str(&format!(
                "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{}\n",
                bandwidth, quality.profile.width, quality.profile.height
            ));
            master.push_str(&format!("{}.m3u8\n", quality.profile.name));
        }

        let master_path = output_dir.join("master.m3u8");
        std::fs::write(&master_path, &master)?;

        info!("Generated master playlist at {:?}", master_path);
        Ok(master)
    }

    /// Get configured profiles
    pub fn get_profiles(&self) -> &[TranscodeProfile] {
        &self.profiles
    }
}

/// Extract time from FFmpeg progress line
fn extract_time(line: &str) -> Option<String> {
    let time_idx = line.find("time=")?;
    let start = time_idx + 5;
    let end = line[start..].find(' ')? + start;
    Some(line[start..end].to_string())
}

/// Parse time string (HH:MM:SS.ms) to seconds
fn parse_time_to_seconds(time_str: &str) -> f64 {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return 0.0;
    }

    let hours: f64 = parts[0].parse().unwrap_or(0.0);
    let minutes: f64 = parts[1].parse().unwrap_or(0.0);
    let seconds: f64 = parts[2].parse().unwrap_or(0.0);

    hours * 3600.0 + minutes * 60.0 + seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profiles() {
        let profiles = default_profiles();
        assert_eq!(profiles.len(), 4);
        assert_eq!(profiles[0].name, "240p");
        assert_eq!(profiles[3].name, "1080p");
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time_to_seconds("00:00:00.00"), 0.0);
        assert_eq!(parse_time_to_seconds("00:01:00.00"), 60.0);
        assert_eq!(parse_time_to_seconds("01:00:00.00"), 3600.0);
        assert_eq!(parse_time_to_seconds("00:00:30.50"), 30.5);
    }

    #[test]
    fn test_extract_time() {
        let line =
            "frame=  100 fps= 25 q=28.0 size=    1024kB time=00:00:04.00 bitrate=2097.2kbits/s";
        let time = extract_time(line);
        assert_eq!(time, Some("00:00:04.00".to_string()));
    }
}
