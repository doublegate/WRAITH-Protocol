//! Subtitle Management
//!
//! Handles SRT and VTT subtitle parsing and synchronization.

use crate::database::{Database, Subtitle, SubtitleInfo};
use crate::error::{StreamError, StreamResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// Parsed subtitle cue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub index: i32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

/// Subtitle manager
pub struct SubtitleManager {
    db: Arc<Database>,
}

impl SubtitleManager {
    /// Create a new subtitle manager
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Add subtitles to a stream
    pub fn add_subtitles(
        &self,
        stream_id: &str,
        language: &str,
        label: &str,
        content: &str,
    ) -> StreamResult<SubtitleInfo> {
        // Detect format
        let format = if content.contains("WEBVTT") {
            "vtt"
        } else {
            "srt"
        };

        // Validate content can be parsed
        let _cues = self.parse_subtitles(content, format)?;

        let subtitle = Subtitle {
            stream_id: stream_id.to_string(),
            language: language.to_string(),
            label: label.to_string(),
            content: content.to_string(),
            format: format.to_string(),
        };

        self.db.add_subtitles(&subtitle)?;

        debug!(
            "Added {} subtitles for stream {} ({} format)",
            language, stream_id, format
        );

        Ok(SubtitleInfo {
            language: language.to_string(),
            label: label.to_string(),
            format: format.to_string(),
        })
    }

    /// Load subtitles for a stream
    pub fn load_subtitles(
        &self,
        stream_id: &str,
        language: &str,
    ) -> StreamResult<Vec<SubtitleCue>> {
        let subtitle = self.db.get_subtitles(stream_id, language)?.ok_or_else(|| {
            StreamError::Subtitle(format!("Subtitles not found for language: {}", language))
        })?;

        self.parse_subtitles(&subtitle.content, &subtitle.format)
    }

    /// List available subtitle languages
    pub fn list_languages(&self, stream_id: &str) -> StreamResult<Vec<SubtitleInfo>> {
        self.db
            .list_subtitle_languages(stream_id)
            .map_err(|e| StreamError::Database(e.to_string()))
    }

    /// Parse subtitles from content
    fn parse_subtitles(&self, content: &str, format: &str) -> StreamResult<Vec<SubtitleCue>> {
        match format {
            "vtt" => self.parse_vtt(content),
            "srt" => self.parse_srt(content),
            _ => Err(StreamError::Subtitle(format!(
                "Unsupported subtitle format: {}",
                format
            ))),
        }
    }

    /// Parse WebVTT format
    fn parse_vtt(&self, content: &str) -> StreamResult<Vec<SubtitleCue>> {
        let mut cues = Vec::new();
        let mut lines = content.lines().peekable();

        // Skip header
        while let Some(line) = lines.peek() {
            if line.contains("-->") {
                break;
            }
            lines.next();
        }

        let mut index = 1;
        while let Some(line) = lines.next() {
            // Check for timestamp line
            if line.contains("-->") {
                let times = self.parse_vtt_timestamp_line(line)?;
                let (start_ms, end_ms) = times;

                // Collect text lines until blank line or end
                let mut text_lines = Vec::new();
                while let Some(text_line) = lines.peek() {
                    if text_line.trim().is_empty() {
                        lines.next();
                        break;
                    }
                    text_lines.push(lines.next().unwrap().to_string());
                }

                if !text_lines.is_empty() {
                    cues.push(SubtitleCue {
                        index,
                        start_ms,
                        end_ms,
                        text: text_lines.join("\n"),
                    });
                    index += 1;
                }
            }
        }

        Ok(cues)
    }

    /// Parse SRT format
    fn parse_srt(&self, content: &str) -> StreamResult<Vec<SubtitleCue>> {
        let mut cues = Vec::new();
        let mut lines = content.lines().peekable();

        while lines.peek().is_some() {
            // Parse index
            let index_line = match lines.next() {
                Some(l) if !l.trim().is_empty() => l,
                _ => continue,
            };

            let index: i32 = index_line
                .trim()
                .parse()
                .map_err(|_| StreamError::Subtitle("Invalid cue index".to_string()))?;

            // Parse timestamp line
            let timestamp_line = lines
                .next()
                .ok_or_else(|| StreamError::Subtitle("Missing timestamp line".to_string()))?;

            let times = self.parse_srt_timestamp_line(timestamp_line)?;
            let (start_ms, end_ms) = times;

            // Collect text lines until blank line
            let mut text_lines = Vec::new();
            while let Some(text_line) = lines.peek() {
                if text_line.trim().is_empty() {
                    lines.next();
                    break;
                }
                text_lines.push(lines.next().unwrap().to_string());
            }

            cues.push(SubtitleCue {
                index,
                start_ms,
                end_ms,
                text: text_lines.join("\n"),
            });
        }

        Ok(cues)
    }

    /// Parse VTT timestamp line
    fn parse_vtt_timestamp_line(&self, line: &str) -> StreamResult<(i64, i64)> {
        let parts: Vec<&str> = line.split("-->").collect();
        if parts.len() != 2 {
            return Err(StreamError::Subtitle(
                "Invalid timestamp format".to_string(),
            ));
        }

        let start_ms = self.parse_vtt_time(parts[0].trim())?;
        // Handle positioning info after timestamp
        let end_part = parts[1]
            .split_whitespace()
            .next()
            .unwrap_or(parts[1].trim());
        let end_ms = self.parse_vtt_time(end_part)?;

        Ok((start_ms, end_ms))
    }

    /// Parse SRT timestamp line
    fn parse_srt_timestamp_line(&self, line: &str) -> StreamResult<(i64, i64)> {
        let parts: Vec<&str> = line.split("-->").collect();
        if parts.len() != 2 {
            return Err(StreamError::Subtitle(
                "Invalid timestamp format".to_string(),
            ));
        }

        let start_ms = self.parse_srt_time(parts[0].trim())?;
        let end_ms = self.parse_srt_time(parts[1].trim())?;

        Ok((start_ms, end_ms))
    }

    /// Parse VTT time format (00:00:00.000 or 00:00.000)
    fn parse_vtt_time(&self, time_str: &str) -> StreamResult<i64> {
        let parts: Vec<&str> = time_str.split(':').collect();

        let (hours, minutes, seconds_ms) = match parts.len() {
            2 => (0i64, parts[0], parts[1]),
            3 => (
                parts[0]
                    .parse()
                    .map_err(|_| StreamError::Subtitle("Invalid hours".to_string()))?,
                parts[1],
                parts[2],
            ),
            _ => return Err(StreamError::Subtitle("Invalid time format".to_string())),
        };

        let minutes: i64 = minutes
            .parse()
            .map_err(|_| StreamError::Subtitle("Invalid minutes".to_string()))?;

        let (seconds, millis) = if seconds_ms.contains('.') {
            let sec_parts: Vec<&str> = seconds_ms.split('.').collect();
            let secs: i64 = sec_parts[0]
                .parse()
                .map_err(|_| StreamError::Subtitle("Invalid seconds".to_string()))?;
            let ms_str = sec_parts[1];
            let ms: i64 = if ms_str.len() >= 3 {
                ms_str[..3]
                    .parse()
                    .map_err(|_| StreamError::Subtitle("Invalid milliseconds".to_string()))?
            } else {
                let padded = format!("{:0<3}", ms_str);
                padded
                    .parse()
                    .map_err(|_| StreamError::Subtitle("Invalid milliseconds".to_string()))?
            };
            (secs, ms)
        } else {
            let secs: i64 = seconds_ms
                .parse()
                .map_err(|_| StreamError::Subtitle("Invalid seconds".to_string()))?;
            (secs, 0)
        };

        Ok(hours * 3600000 + minutes * 60000 + seconds * 1000 + millis)
    }

    /// Parse SRT time format (00:00:00,000)
    fn parse_srt_time(&self, time_str: &str) -> StreamResult<i64> {
        // SRT uses comma for milliseconds
        let normalized = time_str.replace(',', ".");
        self.parse_vtt_time(&normalized)
    }

    /// Convert subtitles to VTT format
    pub fn to_vtt(&self, cues: &[SubtitleCue]) -> String {
        let mut output = String::from("WEBVTT\n\n");

        for cue in cues {
            output.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                cue.index,
                self.format_vtt_time(cue.start_ms),
                self.format_vtt_time(cue.end_ms),
                cue.text
            ));
        }

        output
    }

    /// Format milliseconds to VTT time
    fn format_vtt_time(&self, ms: i64) -> String {
        let hours = ms / 3600000;
        let minutes = (ms % 3600000) / 60000;
        let seconds = (ms % 60000) / 1000;
        let millis = ms % 1000;

        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manager() -> (SubtitleManager, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        (SubtitleManager::new(db), dir)
    }

    #[test]
    fn test_parse_vtt() {
        let (manager, _dir) = create_test_manager();

        let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:05.000
Hello World

00:00:05.000 --> 00:00:10.000
This is a test"#;

        let cues = manager.parse_vtt(vtt).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello World");
        assert_eq!(cues[0].start_ms, 0);
        assert_eq!(cues[0].end_ms, 5000);
    }

    #[test]
    fn test_parse_srt() {
        let (manager, _dir) = create_test_manager();

        let srt = r#"1
00:00:00,000 --> 00:00:05,000
Hello World

2
00:00:05,000 --> 00:00:10,000
This is a test"#;

        let cues = manager.parse_srt(srt).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello World");
        assert_eq!(cues[1].index, 2);
    }

    #[test]
    fn test_parse_vtt_time() {
        let (manager, _dir) = create_test_manager();

        assert_eq!(manager.parse_vtt_time("00:00:00.000").unwrap(), 0);
        assert_eq!(manager.parse_vtt_time("00:01:00.000").unwrap(), 60000);
        assert_eq!(manager.parse_vtt_time("01:00:00.000").unwrap(), 3600000);
        assert_eq!(manager.parse_vtt_time("00:00:30.500").unwrap(), 30500);

        // Short format (no hours)
        assert_eq!(manager.parse_vtt_time("00:30.500").unwrap(), 30500);
    }

    #[test]
    fn test_to_vtt() {
        let (manager, _dir) = create_test_manager();

        let cues = vec![
            SubtitleCue {
                index: 1,
                start_ms: 0,
                end_ms: 5000,
                text: "Hello".to_string(),
            },
            SubtitleCue {
                index: 2,
                start_ms: 5000,
                end_ms: 10000,
                text: "World".to_string(),
            },
        ];

        let vtt = manager.to_vtt(&cues);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:05.000"));
        assert!(vtt.contains("Hello"));
    }
}
