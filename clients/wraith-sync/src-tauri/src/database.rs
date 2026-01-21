//! SQLite Database for Sync Metadata Storage
//!
//! Manages sync state, file metadata, version history, conflicts, and device information.

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager for sync metadata
/// Uses Mutex for thread-safe access to the SQLite connection
pub struct Database {
    /// The underlying SQLite connection (wrapped in Mutex for thread safety)
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the sync database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Configure SQLite for better performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 10000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// Create database tables
    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock();
        // Synced folders configuration
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                local_path TEXT UNIQUE NOT NULL,
                remote_path TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                paused INTEGER DEFAULT 0,
                last_sync_at INTEGER,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // File metadata (one row per file per folder)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                hash BLOB NOT NULL,
                is_directory INTEGER DEFAULT 0,
                synced INTEGER DEFAULT 0,
                deleted INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (folder_id) REFERENCES sync_folders(id) ON DELETE CASCADE,
                UNIQUE (folder_id, relative_path)
            )",
            [],
        )?;

        // File versions for history
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                version_number INTEGER NOT NULL,
                hash BLOB NOT NULL,
                size INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                storage_path TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Devices in sync group
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT UNIQUE NOT NULL,
                device_name TEXT NOT NULL,
                public_key BLOB NOT NULL,
                last_seen INTEGER NOT NULL,
                is_self INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Sync conflicts
        conn.execute(
            "CREATE TABLE IF NOT EXISTS conflicts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                local_hash BLOB NOT NULL,
                remote_hash BLOB NOT NULL,
                local_modified_at INTEGER NOT NULL,
                remote_modified_at INTEGER NOT NULL,
                remote_device_id TEXT NOT NULL,
                resolution_strategy TEXT,
                resolved INTEGER DEFAULT 0,
                resolved_at INTEGER,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Offline sync queue
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                operation TEXT NOT NULL CHECK(operation IN ('upload', 'download', 'delete')),
                priority INTEGER DEFAULT 0,
                retries INTEGER DEFAULT 0,
                last_attempt INTEGER,
                error_message TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (folder_id) REFERENCES sync_folders(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Sync settings
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Ignored patterns
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ignored_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id INTEGER,
                pattern TEXT NOT NULL,
                is_global INTEGER DEFAULT 0,
                FOREIGN KEY (folder_id) REFERENCES sync_folders(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_metadata_folder
             ON file_metadata(folder_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_metadata_path
             ON file_metadata(folder_id, relative_path)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_versions_file
             ON file_versions(file_id, version_number DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conflicts_resolved
             ON conflicts(resolved, created_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sync_queue_priority
             ON sync_queue(priority DESC, created_at ASC)",
            [],
        )?;

        // Insert default ignored patterns
        let patterns = [
            "**/.git/**",
            "**/.svn/**",
            "**/.hg/**",
            "**/node_modules/**",
            "**/target/**",
            "**/__pycache__/**",
            "**/.DS_Store",
            "**/Thumbs.db",
            "**/*.tmp",
            "**/*.temp",
            "**/*.swp",
            "**/*~",
            "**/.wraith-sync/**",
        ];

        for pattern in patterns {
            conn.execute(
                "INSERT OR IGNORE INTO ignored_patterns (pattern, is_global) VALUES (?1, 1)",
                params![pattern],
            )?;
        }

        Ok(())
    }

    // MARK: - Sync Folder Operations

    /// Add a new sync folder
    pub fn add_sync_folder(&self, folder: &NewSyncFolder) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sync_folders (local_path, remote_path, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                folder.local_path,
                folder.remote_path,
                folder.enabled as i32,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a sync folder by ID
    pub fn get_sync_folder(&self, id: i64) -> Result<Option<SyncFolder>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, local_path, remote_path, enabled, paused, last_sync_at, created_at
             FROM sync_folders WHERE id = ?1",
            params![id],
            |row| {
                Ok(SyncFolder {
                    id: row.get(0)?,
                    local_path: row.get(1)?,
                    remote_path: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    paused: row.get::<_, i32>(4)? != 0,
                    last_sync_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to get sync folder")
    }

    /// Get a sync folder by local path
    pub fn get_sync_folder_by_path(&self, local_path: &str) -> Result<Option<SyncFolder>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, local_path, remote_path, enabled, paused, last_sync_at, created_at
             FROM sync_folders WHERE local_path = ?1",
            params![local_path],
            |row| {
                Ok(SyncFolder {
                    id: row.get(0)?,
                    local_path: row.get(1)?,
                    remote_path: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    paused: row.get::<_, i32>(4)? != 0,
                    last_sync_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to get sync folder by path")
    }

    /// List all sync folders
    pub fn list_sync_folders(&self) -> Result<Vec<SyncFolder>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, local_path, remote_path, enabled, paused, last_sync_at, created_at
             FROM sync_folders ORDER BY local_path ASC",
        )?;

        let folders = stmt
            .query_map([], |row| {
                Ok(SyncFolder {
                    id: row.get(0)?,
                    local_path: row.get(1)?,
                    remote_path: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    paused: row.get::<_, i32>(4)? != 0,
                    last_sync_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(folders)
    }

    /// Remove a sync folder
    pub fn remove_sync_folder(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sync_folders WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Update sync folder pause state
    pub fn set_folder_paused(&self, id: i64, paused: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sync_folders SET paused = ?1 WHERE id = ?2",
            params![paused as i32, id],
        )?;
        Ok(())
    }

    /// Update last sync timestamp
    pub fn update_last_sync(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sync_folders SET last_sync_at = ?1 WHERE id = ?2",
            params![Utc::now().timestamp(), id],
        )?;
        Ok(())
    }

    // MARK: - File Metadata Operations

    /// Insert or update file metadata
    pub fn upsert_file_metadata(&self, meta: &FileMetadata) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO file_metadata (folder_id, relative_path, size, modified_at, hash, is_directory, synced, deleted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(folder_id, relative_path) DO UPDATE SET
               size = excluded.size,
               modified_at = excluded.modified_at,
               hash = excluded.hash,
               is_directory = excluded.is_directory,
               synced = excluded.synced,
               deleted = excluded.deleted",
            params![
                meta.folder_id,
                meta.relative_path,
                meta.size,
                meta.modified_at,
                meta.hash,
                meta.is_directory as i32,
                meta.synced as i32,
                meta.deleted as i32,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get file metadata by path
    pub fn get_file_metadata(
        &self,
        folder_id: i64,
        relative_path: &str,
    ) -> Result<Option<FileMetadata>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, folder_id, relative_path, size, modified_at, hash, is_directory, synced, deleted, created_at
             FROM file_metadata WHERE folder_id = ?1 AND relative_path = ?2",
            params![folder_id, relative_path],
            |row| {
                Ok(FileMetadata {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    size: row.get(3)?,
                    modified_at: row.get(4)?,
                    hash: row.get(5)?,
                    is_directory: row.get::<_, i32>(6)? != 0,
                    synced: row.get::<_, i32>(7)? != 0,
                    deleted: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .context("Failed to get file metadata")
    }

    /// List all files in a folder
    pub fn list_folder_files(&self, folder_id: i64) -> Result<Vec<FileMetadata>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, relative_path, size, modified_at, hash, is_directory, synced, deleted, created_at
             FROM file_metadata WHERE folder_id = ?1 AND deleted = 0
             ORDER BY relative_path ASC",
        )?;

        let files = stmt
            .query_map(params![folder_id], |row| {
                Ok(FileMetadata {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    size: row.get(3)?,
                    modified_at: row.get(4)?,
                    hash: row.get(5)?,
                    is_directory: row.get::<_, i32>(6)? != 0,
                    synced: row.get::<_, i32>(7)? != 0,
                    deleted: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Mark file as synced
    pub fn mark_file_synced(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE file_metadata SET synced = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Mark file as deleted
    pub fn mark_file_deleted(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE file_metadata SET deleted = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Get count of unsynced files
    pub fn count_unsynced_files(&self, folder_id: i64) -> Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM file_metadata WHERE folder_id = ?1 AND synced = 0 AND deleted = 0",
            params![folder_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    // MARK: - Version History Operations

    /// Add a new version for a file
    pub fn add_file_version(&self, version: &FileVersion) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO file_versions (file_id, version_number, hash, size, modified_at, storage_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                version.file_id,
                version.version_number,
                version.hash,
                version.size,
                version.modified_at,
                version.storage_path,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get version history for a file
    pub fn get_file_versions(&self, file_id: i64) -> Result<Vec<FileVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, file_id, version_number, hash, size, modified_at, storage_path, created_at
             FROM file_versions WHERE file_id = ?1
             ORDER BY version_number DESC",
        )?;

        let versions = stmt
            .query_map(params![file_id], |row| {
                Ok(FileVersion {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    version_number: row.get(2)?,
                    hash: row.get(3)?,
                    size: row.get(4)?,
                    modified_at: row.get(5)?,
                    storage_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(versions)
    }

    /// Get next version number for a file
    pub fn get_next_version_number(&self, file_id: i64) -> Result<i64> {
        let conn = self.conn.lock();
        let max_version: Option<i64> = conn.query_row(
            "SELECT MAX(version_number) FROM file_versions WHERE file_id = ?1",
            params![file_id],
            |row| row.get(0),
        )?;
        Ok(max_version.unwrap_or(0) + 1)
    }

    /// Prune old versions beyond retention limit
    pub fn prune_old_versions(&self, file_id: i64, max_versions: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        // Get versions to delete
        let mut stmt = conn.prepare(
            "SELECT storage_path FROM file_versions
             WHERE file_id = ?1 AND storage_path IS NOT NULL
             ORDER BY version_number DESC
             LIMIT -1 OFFSET ?2",
        )?;

        let paths: Vec<String> = stmt
            .query_map(params![file_id, max_versions], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete old versions from database
        conn.execute(
            "DELETE FROM file_versions
             WHERE file_id = ?1 AND id NOT IN (
                SELECT id FROM file_versions
                WHERE file_id = ?1
                ORDER BY version_number DESC
                LIMIT ?2
             )",
            params![file_id, max_versions],
        )?;

        Ok(paths)
    }

    // MARK: - Conflict Operations

    /// Create a new conflict record
    pub fn create_conflict(&self, conflict: &NewConflict) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO conflicts (file_id, local_hash, remote_hash, local_modified_at, remote_modified_at, remote_device_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                conflict.file_id,
                conflict.local_hash,
                conflict.remote_hash,
                conflict.local_modified_at,
                conflict.remote_modified_at,
                conflict.remote_device_id,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// List unresolved conflicts
    pub fn list_unresolved_conflicts(&self) -> Result<Vec<Conflict>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.file_id, c.local_hash, c.remote_hash, c.local_modified_at,
                    c.remote_modified_at, c.remote_device_id, c.resolution_strategy,
                    c.resolved, c.resolved_at, c.created_at, f.relative_path, sf.local_path
             FROM conflicts c
             JOIN file_metadata f ON c.file_id = f.id
             JOIN sync_folders sf ON f.folder_id = sf.id
             WHERE c.resolved = 0
             ORDER BY c.created_at DESC",
        )?;

        let conflicts = stmt
            .query_map([], |row| {
                Ok(Conflict {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    local_hash: row.get(2)?,
                    remote_hash: row.get(3)?,
                    local_modified_at: row.get(4)?,
                    remote_modified_at: row.get(5)?,
                    remote_device_id: row.get(6)?,
                    resolution_strategy: row.get(7)?,
                    resolved: row.get::<_, i32>(8)? != 0,
                    resolved_at: row.get(9)?,
                    created_at: row.get(10)?,
                    relative_path: row.get(11)?,
                    folder_path: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conflicts)
    }

    /// Resolve a conflict
    pub fn resolve_conflict(&self, id: i64, strategy: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE conflicts SET resolved = 1, resolution_strategy = ?1, resolved_at = ?2 WHERE id = ?3",
            params![strategy, Utc::now().timestamp(), id],
        )?;
        Ok(())
    }

    /// Count unresolved conflicts
    pub fn count_unresolved_conflicts(&self) -> Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conflicts WHERE resolved = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    // MARK: - Sync Queue Operations

    /// Add item to sync queue
    pub fn add_to_queue(&self, item: &QueueItem) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sync_queue (folder_id, relative_path, operation, priority, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                item.folder_id,
                item.relative_path,
                item.operation,
                item.priority,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get next items from queue (up to limit)
    pub fn get_queue_items(&self, limit: i64) -> Result<Vec<QueueItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, relative_path, operation, priority, retries, last_attempt, error_message, created_at
             FROM sync_queue
             ORDER BY priority DESC, created_at ASC
             LIMIT ?1",
        )?;

        let items = stmt
            .query_map(params![limit], |row| {
                Ok(QueueItem {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    operation: row.get(3)?,
                    priority: row.get(4)?,
                    retries: row.get(5)?,
                    last_attempt: row.get(6)?,
                    error_message: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// Remove item from queue
    pub fn remove_from_queue(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sync_queue WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Update queue item retry count
    pub fn update_queue_retry(&self, id: i64, error_message: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sync_queue SET retries = retries + 1, last_attempt = ?1, error_message = ?2 WHERE id = ?3",
            params![Utc::now().timestamp(), error_message, id],
        )?;
        Ok(())
    }

    /// Get queue size
    pub fn queue_size(&self) -> Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sync_queue", [], |row| row.get(0))?;
        Ok(count)
    }

    // MARK: - Device Operations

    /// Add or update a device
    pub fn upsert_device(&self, device: &Device) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO devices (device_id, device_name, public_key, last_seen, is_self, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(device_id) DO UPDATE SET
               device_name = excluded.device_name,
               last_seen = excluded.last_seen",
            params![
                device.device_id,
                device.device_name,
                device.public_key,
                Utc::now().timestamp(),
                device.is_self as i32,
                Utc::now().timestamp()
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// List all devices
    pub fn list_devices(&self) -> Result<Vec<Device>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, device_id, device_name, public_key, last_seen, is_self, created_at
             FROM devices ORDER BY is_self DESC, device_name ASC",
        )?;

        let devices = stmt
            .query_map([], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    device_name: row.get(2)?,
                    public_key: row.get(3)?,
                    last_seen: row.get(4)?,
                    is_self: row.get::<_, i32>(5)? != 0,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(devices)
    }

    /// Remove a device
    pub fn remove_device(&self, device_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM devices WHERE device_id = ?1",
            params![device_id],
        )?;
        Ok(())
    }

    // MARK: - Settings Operations

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to get setting")
    }

    /// Set a setting value
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // MARK: - Ignored Patterns Operations

    /// Get all ignored patterns for a folder (including global)
    pub fn get_ignored_patterns(&self, folder_id: Option<i64>) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT pattern FROM ignored_patterns
             WHERE is_global = 1 OR folder_id = ?1
             ORDER BY is_global DESC",
        )?;

        let patterns: Vec<String> = stmt
            .query_map(params![folder_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(patterns)
    }

    /// Add an ignored pattern
    pub fn add_ignored_pattern(&self, folder_id: Option<i64>, pattern: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO ignored_patterns (folder_id, pattern, is_global) VALUES (?1, ?2, ?3)",
            params![folder_id, pattern, folder_id.is_none() as i32],
        )?;
        Ok(())
    }
}

// MARK: - Data Models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFolder {
    pub id: i64,
    pub local_path: String,
    pub remote_path: String,
    pub enabled: bool,
    pub paused: bool,
    pub last_sync_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSyncFolder {
    pub local_path: String,
    pub remote_path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    #[serde(default)]
    pub id: i64,
    pub folder_id: i64,
    pub relative_path: String,
    pub size: i64,
    pub modified_at: i64,
    pub hash: Vec<u8>,
    pub is_directory: bool,
    pub synced: bool,
    pub deleted: bool,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    #[serde(default)]
    pub id: i64,
    pub file_id: i64,
    pub version_number: i64,
    pub hash: Vec<u8>,
    pub size: i64,
    pub modified_at: i64,
    pub storage_path: Option<String>,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub id: i64,
    pub device_id: String,
    pub device_name: String,
    pub public_key: Vec<u8>,
    pub last_seen: i64,
    pub is_self: bool,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: i64,
    pub file_id: i64,
    pub local_hash: Vec<u8>,
    pub remote_hash: Vec<u8>,
    pub local_modified_at: i64,
    pub remote_modified_at: i64,
    pub remote_device_id: String,
    pub resolution_strategy: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<i64>,
    pub created_at: i64,
    // Joined fields for convenience
    pub relative_path: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConflict {
    pub file_id: i64,
    pub local_hash: Vec<u8>,
    pub remote_hash: Vec<u8>,
    pub local_modified_at: i64,
    pub remote_modified_at: i64,
    pub remote_device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    #[serde(default)]
    pub id: i64,
    pub folder_id: i64,
    pub relative_path: String,
    pub operation: String, // "upload", "download", "delete"
    pub priority: i64,
    pub retries: i64,
    pub last_attempt: Option<i64>,
    pub error_message: Option<String>,
    #[serde(default)]
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Verify tables exist
        let folders = db.list_sync_folders().unwrap();
        assert!(folders.is_empty());
    }

    #[test]
    fn test_sync_folder_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let folder = NewSyncFolder {
            local_path: "/home/user/Documents".to_string(),
            remote_path: "/Documents".to_string(),
            enabled: true,
        };

        let id = db.add_sync_folder(&folder).unwrap();
        assert!(id > 0);

        let retrieved = db.get_sync_folder(id).unwrap().unwrap();
        assert_eq!(retrieved.local_path, folder.local_path);
        assert_eq!(retrieved.remote_path, folder.remote_path);
        assert!(retrieved.enabled);
        assert!(!retrieved.paused);
    }

    #[test]
    fn test_file_metadata_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create a folder first
        let folder = NewSyncFolder {
            local_path: "/home/user/Documents".to_string(),
            remote_path: "/Documents".to_string(),
            enabled: true,
        };
        let folder_id = db.add_sync_folder(&folder).unwrap();

        // Add file metadata
        let meta = FileMetadata {
            id: 0,
            folder_id,
            relative_path: "test.txt".to_string(),
            size: 1024,
            modified_at: 1700000000,
            hash: vec![1, 2, 3, 4],
            is_directory: false,
            synced: false,
            deleted: false,
            created_at: 0,
        };

        db.upsert_file_metadata(&meta).unwrap();

        let retrieved = db
            .get_file_metadata(folder_id, "test.txt")
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.size, 1024);
        assert_eq!(retrieved.hash, vec![1, 2, 3, 4]);
        assert!(!retrieved.synced);

        // Mark as synced
        db.mark_file_synced(retrieved.id).unwrap();
        let updated = db
            .get_file_metadata(folder_id, "test.txt")
            .unwrap()
            .unwrap();
        assert!(updated.synced);
    }

    #[test]
    fn test_ignored_patterns() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let patterns = db.get_ignored_patterns(None).unwrap();
        assert!(!patterns.is_empty());
        assert!(patterns.contains(&"**/.git/**".to_string()));
        assert!(patterns.contains(&"**/node_modules/**".to_string()));
    }
}
