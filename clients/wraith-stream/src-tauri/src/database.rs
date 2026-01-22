//! SQLite Database for WRAITH Stream
//!
//! Manages streams, segments, views, and subtitles.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager for stream data
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the stream database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Configure SQLite for better performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 10000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// Create all database tables
    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Streams table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS streams (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL,
                thumbnail_hash TEXT,
                duration INTEGER,
                is_live INTEGER DEFAULT 0,
                status TEXT DEFAULT 'processing' CHECK (status IN ('processing', 'ready', 'failed', 'live')),
                view_count INTEGER DEFAULT 0,
                category TEXT,
                tags TEXT
            )",
            [],
        )?;

        // Stream segments table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stream_segments (
                stream_id TEXT NOT NULL,
                segment_name TEXT NOT NULL,
                segment_hash TEXT NOT NULL,
                segment_size INTEGER NOT NULL,
                quality TEXT NOT NULL,
                sequence_number INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                encrypted INTEGER DEFAULT 1,
                PRIMARY KEY (stream_id, segment_name),
                FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Stream quality levels table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stream_qualities (
                stream_id TEXT NOT NULL,
                quality TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                video_bitrate INTEGER NOT NULL,
                audio_bitrate INTEGER NOT NULL,
                segment_count INTEGER DEFAULT 0,
                PRIMARY KEY (stream_id, quality),
                FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Stream views table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stream_views (
                stream_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                watch_time INTEGER DEFAULT 0,
                last_position INTEGER DEFAULT 0,
                quality TEXT,
                PRIMARY KEY (stream_id, peer_id, started_at),
                FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Subtitles table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subtitles (
                stream_id TEXT NOT NULL,
                language TEXT NOT NULL,
                label TEXT NOT NULL,
                content TEXT NOT NULL,
                format TEXT NOT NULL CHECK (format IN ('srt', 'vtt')),
                PRIMARY KEY (stream_id, language),
                FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Local identity table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS local_identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                peer_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_streams_created_by
             ON streams(created_by)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_streams_status
             ON streams(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stream_segments_stream
             ON stream_segments(stream_id, quality)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stream_views_stream
             ON stream_views(stream_id)",
            [],
        )?;

        Ok(())
    }

    // =========================================================================
    // Local Identity Operations
    // =========================================================================

    /// Get local identity
    pub fn get_local_identity(&self) -> Result<Option<LocalIdentity>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT peer_id, display_name, created_at
             FROM local_identity WHERE id = 1",
            [],
            |row| {
                Ok(LocalIdentity {
                    peer_id: row.get(0)?,
                    display_name: row.get(1)?,
                    created_at: row.get(2)?,
                })
            },
        )
        .optional()
        .context("Failed to get local identity")
    }

    /// Save local identity
    pub fn save_local_identity(&self, identity: &LocalIdentity) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO local_identity (id, peer_id, display_name, created_at)
             VALUES (1, ?1, ?2, ?3)",
            params![identity.peer_id, identity.display_name, identity.created_at,],
        )?;
        Ok(())
    }

    // =========================================================================
    // Stream Operations
    // =========================================================================

    /// Create a new stream
    pub fn create_stream(&self, stream: &Stream) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO streams (id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, category, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                stream.id,
                stream.title,
                stream.description,
                stream.created_at,
                stream.created_by,
                stream.thumbnail_hash,
                stream.duration,
                stream.is_live as i32,
                stream.status,
                stream.category,
                stream.tags,
            ],
        )?;
        Ok(())
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: &str) -> Result<Option<Stream>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, view_count, category, tags
             FROM streams WHERE id = ?1",
            params![stream_id],
            |row| {
                Ok(Stream {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    thumbnail_hash: row.get(5)?,
                    duration: row.get(6)?,
                    is_live: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                    view_count: row.get(9)?,
                    category: row.get(10)?,
                    tags: row.get(11)?,
                })
            },
        )
        .optional()
        .context("Failed to get stream")
    }

    /// List all streams
    pub fn list_streams(&self, limit: i64, offset: i64) -> Result<Vec<Stream>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, view_count, category, tags
             FROM streams WHERE status = 'ready' OR status = 'live'
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let streams = stmt
            .query_map(params![limit, offset], |row| {
                Ok(Stream {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    thumbnail_hash: row.get(5)?,
                    duration: row.get(6)?,
                    is_live: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                    view_count: row.get(9)?,
                    category: row.get(10)?,
                    tags: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(streams)
    }

    /// List streams by creator
    pub fn list_streams_by_creator(&self, creator_id: &str) -> Result<Vec<Stream>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, view_count, category, tags
             FROM streams WHERE created_by = ?1
             ORDER BY created_at DESC",
        )?;

        let streams = stmt
            .query_map(params![creator_id], |row| {
                Ok(Stream {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    thumbnail_hash: row.get(5)?,
                    duration: row.get(6)?,
                    is_live: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                    view_count: row.get(9)?,
                    category: row.get(10)?,
                    tags: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(streams)
    }

    /// Search streams by title or description
    pub fn search_streams(&self, query: &str, limit: i64) -> Result<Vec<Stream>> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, view_count, category, tags
             FROM streams
             WHERE (status = 'ready' OR status = 'live')
               AND (title LIKE ?1 OR description LIKE ?1 OR tags LIKE ?1)
             ORDER BY view_count DESC
             LIMIT ?2",
        )?;

        let streams = stmt
            .query_map(params![pattern, limit], |row| {
                Ok(Stream {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    thumbnail_hash: row.get(5)?,
                    duration: row.get(6)?,
                    is_live: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                    view_count: row.get(9)?,
                    category: row.get(10)?,
                    tags: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(streams)
    }

    /// Get trending streams (by view count)
    pub fn get_trending_streams(&self, limit: i64) -> Result<Vec<Stream>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, created_at, created_by, thumbnail_hash, duration, is_live, status, view_count, category, tags
             FROM streams
             WHERE status = 'ready' OR status = 'live'
             ORDER BY view_count DESC
             LIMIT ?1",
        )?;

        let streams = stmt
            .query_map(params![limit], |row| {
                Ok(Stream {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    thumbnail_hash: row.get(5)?,
                    duration: row.get(6)?,
                    is_live: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                    view_count: row.get(9)?,
                    category: row.get(10)?,
                    tags: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(streams)
    }

    /// Update stream
    pub fn update_stream(&self, stream: &Stream) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE streams SET title = ?1, description = ?2, thumbnail_hash = ?3, duration = ?4, is_live = ?5, status = ?6, category = ?7, tags = ?8
             WHERE id = ?9",
            params![
                stream.title,
                stream.description,
                stream.thumbnail_hash,
                stream.duration,
                stream.is_live as i32,
                stream.status,
                stream.category,
                stream.tags,
                stream.id,
            ],
        )?;
        Ok(())
    }

    /// Delete a stream
    pub fn delete_stream(&self, stream_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM streams WHERE id = ?1", params![stream_id])?;
        Ok(())
    }

    /// Increment view count
    pub fn increment_view_count(&self, stream_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE streams SET view_count = view_count + 1 WHERE id = ?1",
            params![stream_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Segment Operations
    // =========================================================================

    /// Add a segment
    pub fn add_segment(&self, segment: &StreamSegment) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO stream_segments (stream_id, segment_name, segment_hash, segment_size, quality, sequence_number, duration_ms, encrypted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                segment.stream_id,
                segment.segment_name,
                segment.segment_hash,
                segment.segment_size,
                segment.quality,
                segment.sequence_number,
                segment.duration_ms,
                segment.encrypted as i32,
            ],
        )?;
        Ok(())
    }

    /// Get segment by name
    pub fn get_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
    ) -> Result<Option<StreamSegment>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT stream_id, segment_name, segment_hash, segment_size, quality, sequence_number, duration_ms, encrypted
             FROM stream_segments WHERE stream_id = ?1 AND segment_name = ?2",
            params![stream_id, segment_name],
            |row| {
                Ok(StreamSegment {
                    stream_id: row.get(0)?,
                    segment_name: row.get(1)?,
                    segment_hash: row.get(2)?,
                    segment_size: row.get(3)?,
                    quality: row.get(4)?,
                    sequence_number: row.get(5)?,
                    duration_ms: row.get(6)?,
                    encrypted: row.get::<_, i32>(7)? != 0,
                })
            },
        )
        .optional()
        .context("Failed to get segment")
    }

    /// List segments for a stream quality
    pub fn list_segments(&self, stream_id: &str, quality: &str) -> Result<Vec<StreamSegment>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT stream_id, segment_name, segment_hash, segment_size, quality, sequence_number, duration_ms, encrypted
             FROM stream_segments WHERE stream_id = ?1 AND quality = ?2
             ORDER BY sequence_number ASC",
        )?;

        let segments = stmt
            .query_map(params![stream_id, quality], |row| {
                Ok(StreamSegment {
                    stream_id: row.get(0)?,
                    segment_name: row.get(1)?,
                    segment_hash: row.get(2)?,
                    segment_size: row.get(3)?,
                    quality: row.get(4)?,
                    sequence_number: row.get(5)?,
                    duration_ms: row.get(6)?,
                    encrypted: row.get::<_, i32>(7)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(segments)
    }

    /// Delete all segments for a stream
    pub fn delete_stream_segments(&self, stream_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM stream_segments WHERE stream_id = ?1",
            params![stream_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Quality Level Operations
    // =========================================================================

    /// Add or update a quality level
    pub fn upsert_quality(&self, quality: &StreamQuality) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO stream_qualities (stream_id, quality, width, height, video_bitrate, audio_bitrate, segment_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                quality.stream_id,
                quality.quality,
                quality.width,
                quality.height,
                quality.video_bitrate,
                quality.audio_bitrate,
                quality.segment_count,
            ],
        )?;
        Ok(())
    }

    /// List quality levels for a stream
    pub fn list_qualities(&self, stream_id: &str) -> Result<Vec<StreamQuality>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT stream_id, quality, width, height, video_bitrate, audio_bitrate, segment_count
             FROM stream_qualities WHERE stream_id = ?1
             ORDER BY video_bitrate ASC",
        )?;

        let qualities = stmt
            .query_map(params![stream_id], |row| {
                Ok(StreamQuality {
                    stream_id: row.get(0)?,
                    quality: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    video_bitrate: row.get(4)?,
                    audio_bitrate: row.get(5)?,
                    segment_count: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(qualities)
    }

    // =========================================================================
    // View Operations
    // =========================================================================

    /// Record a view
    pub fn record_view(&self, view: &StreamView) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO stream_views (stream_id, peer_id, started_at, watch_time, last_position, quality)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                view.stream_id,
                view.peer_id,
                view.started_at,
                view.watch_time,
                view.last_position,
                view.quality,
            ],
        )?;
        Ok(())
    }

    /// Get views for a stream
    pub fn get_stream_views(&self, stream_id: &str, limit: i64) -> Result<Vec<StreamView>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT stream_id, peer_id, started_at, watch_time, last_position, quality
             FROM stream_views WHERE stream_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )?;

        let views = stmt
            .query_map(params![stream_id, limit], |row| {
                Ok(StreamView {
                    stream_id: row.get(0)?,
                    peer_id: row.get(1)?,
                    started_at: row.get(2)?,
                    watch_time: row.get(3)?,
                    last_position: row.get(4)?,
                    quality: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(views)
    }

    // =========================================================================
    // Subtitle Operations
    // =========================================================================

    /// Add subtitles
    pub fn add_subtitles(&self, subtitle: &Subtitle) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO subtitles (stream_id, language, label, content, format)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                subtitle.stream_id,
                subtitle.language,
                subtitle.label,
                subtitle.content,
                subtitle.format,
            ],
        )?;
        Ok(())
    }

    /// Get subtitles for a stream
    pub fn get_subtitles(&self, stream_id: &str, language: &str) -> Result<Option<Subtitle>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT stream_id, language, label, content, format
             FROM subtitles WHERE stream_id = ?1 AND language = ?2",
            params![stream_id, language],
            |row| {
                Ok(Subtitle {
                    stream_id: row.get(0)?,
                    language: row.get(1)?,
                    label: row.get(2)?,
                    content: row.get(3)?,
                    format: row.get(4)?,
                })
            },
        )
        .optional()
        .context("Failed to get subtitles")
    }

    /// List available subtitle languages
    pub fn list_subtitle_languages(&self, stream_id: &str) -> Result<Vec<SubtitleInfo>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT language, label, format FROM subtitles WHERE stream_id = ?1")?;

        let subtitles = stmt
            .query_map(params![stream_id], |row| {
                Ok(SubtitleInfo {
                    language: row.get(0)?,
                    label: row.get(1)?,
                    format: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(subtitles)
    }
}

// =============================================================================
// Data Models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIdentity {
    pub peer_id: String,
    pub display_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
    pub thumbnail_hash: Option<String>,
    pub duration: Option<i64>,
    pub is_live: bool,
    pub status: String,
    pub view_count: i64,
    pub category: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSegment {
    pub stream_id: String,
    pub segment_name: String,
    pub segment_hash: String,
    pub segment_size: i64,
    pub quality: String,
    pub sequence_number: i64,
    pub duration_ms: i64,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQuality {
    pub stream_id: String,
    pub quality: String,
    pub width: i64,
    pub height: i64,
    pub video_bitrate: i64,
    pub audio_bitrate: i64,
    pub segment_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamView {
    pub stream_id: String,
    pub peer_id: String,
    pub started_at: i64,
    pub watch_time: i64,
    pub last_position: i64,
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitle {
    pub stream_id: String,
    pub language: String,
    pub label: String,
    pub content: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleInfo {
    pub language: String,
    pub label: String,
    pub format: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Verify tables exist
        let streams = db.list_streams(10, 0).unwrap();
        assert!(streams.is_empty());
    }

    #[test]
    fn test_stream_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let stream = Stream {
            id: "test-stream-id".to_string(),
            title: "Test Stream".to_string(),
            description: Some("A test stream".to_string()),
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
            thumbnail_hash: None,
            duration: Some(3600),
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: Some("Technology".to_string()),
            tags: Some("test,demo".to_string()),
        };

        db.create_stream(&stream).unwrap();

        let retrieved = db.get_stream(&stream.id).unwrap().unwrap();
        assert_eq!(retrieved.title, stream.title);
        assert_eq!(retrieved.description, stream.description);
    }

    #[test]
    fn test_segment_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create stream first
        let stream = Stream {
            id: "test-stream-id".to_string(),
            title: "Test Stream".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
            thumbnail_hash: None,
            duration: None,
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: None,
            tags: None,
        };
        db.create_stream(&stream).unwrap();

        // Add segment
        let segment = StreamSegment {
            stream_id: stream.id.clone(),
            segment_name: "720p_001.ts".to_string(),
            segment_hash: "abc123".to_string(),
            segment_size: 1024000,
            quality: "720p".to_string(),
            sequence_number: 1,
            duration_ms: 6000,
            encrypted: true,
        };

        db.add_segment(&segment).unwrap();

        let retrieved = db
            .get_segment(&stream.id, &segment.segment_name)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.segment_hash, segment.segment_hash);
        assert_eq!(retrieved.quality, "720p");
    }

    #[test]
    fn test_subtitle_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create stream first
        let stream = Stream {
            id: "test-stream-id".to_string(),
            title: "Test Stream".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
            thumbnail_hash: None,
            duration: None,
            is_live: false,
            status: "ready".to_string(),
            view_count: 0,
            category: None,
            tags: None,
        };
        db.create_stream(&stream).unwrap();

        // Add subtitles
        let subtitle = Subtitle {
            stream_id: stream.id.clone(),
            language: "en".to_string(),
            label: "English".to_string(),
            content: "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello World".to_string(),
            format: "vtt".to_string(),
        };

        db.add_subtitles(&subtitle).unwrap();

        let retrieved = db.get_subtitles(&stream.id, "en").unwrap().unwrap();
        assert_eq!(retrieved.label, "English");

        let languages = db.list_subtitle_languages(&stream.id).unwrap();
        assert_eq!(languages.len(), 1);
        assert_eq!(languages[0].language, "en");
    }
}
