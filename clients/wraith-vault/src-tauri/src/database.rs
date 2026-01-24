//! SQLite Database for WRAITH Vault
//!
//! Manages backups, snapshots, chunks, schedules, secrets, and guardians.

use crate::dedup::{ChunkInfo, DedupStats};
use crate::error::{VaultError, VaultResult};
use crate::guardian::{Guardian, GuardianCapabilities, GuardianStatus, TrustLevel};
use crate::secrets::{SecretInfo, SecretType};
use crate::shamir::ShamirConfig;
use crate::shard::{DistributionState, DistributionStatus, EncryptedShard, ShardAssignment};
use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager for vault data
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the vault database
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

        // Backups table (backup configurations)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS backups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_backup_at INTEGER,
                total_size INTEGER DEFAULT 0,
                stored_size INTEGER DEFAULT 0,
                chunk_count INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'idle'
            )",
            [],
        )?;

        // Chunks table (deduplicated data blocks)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chunks (
                hash BLOB PRIMARY KEY,
                size INTEGER NOT NULL,
                compressed_size INTEGER NOT NULL,
                ref_count INTEGER DEFAULT 1,
                created_at INTEGER NOT NULL,
                verified_at INTEGER
            )",
            [],
        )?;

        // Backup chunks mapping
        conn.execute(
            "CREATE TABLE IF NOT EXISTS backup_chunks (
                backup_id TEXT NOT NULL,
                chunk_hash BLOB NOT NULL,
                file_path TEXT NOT NULL,
                chunk_offset INTEGER NOT NULL,
                PRIMARY KEY (backup_id, chunk_hash, file_path, chunk_offset),
                FOREIGN KEY (backup_id) REFERENCES backups(id) ON DELETE CASCADE,
                FOREIGN KEY (chunk_hash) REFERENCES chunks(hash)
            )",
            [],
        )?;

        // Snapshots table (point-in-time backups)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                backup_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                stored_size INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                manifest BLOB,
                FOREIGN KEY (backup_id) REFERENCES backups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Schedules table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schedules (
                id TEXT PRIMARY KEY,
                backup_id TEXT NOT NULL,
                frequency TEXT NOT NULL,
                retention_days INTEGER NOT NULL,
                last_run INTEGER,
                next_run INTEGER,
                enabled INTEGER DEFAULT 1,
                FOREIGN KEY (backup_id) REFERENCES backups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Storage peers table (shard locations)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS storage_peers (
                peer_id TEXT NOT NULL,
                chunk_hash BLOB NOT NULL,
                shard_index INTEGER NOT NULL,
                stored_at INTEGER NOT NULL,
                verified_at INTEGER,
                PRIMARY KEY (peer_id, chunk_hash, shard_index)
            )",
            [],
        )?;

        // Local identity table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS local_identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                peer_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                public_key BLOB NOT NULL,
                private_key BLOB NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // ====================
        // Secret Storage Tables
        // ====================

        // Secrets table (secret metadata, not the secret itself)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secrets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                secret_type TEXT NOT NULL DEFAULT 'generic',
                threshold INTEGER NOT NULL,
                total_shares INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                last_accessed_at INTEGER,
                rotation_count INTEGER DEFAULT 0,
                last_rotated_at INTEGER,
                key_salt BLOB,
                distribution_complete INTEGER DEFAULT 0,
                tags TEXT,
                metadata TEXT
            )",
            [],
        )?;

        // Guardians table (trusted peers holding shares)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS guardians (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                peer_id TEXT NOT NULL UNIQUE,
                public_key TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                trust_level INTEGER NOT NULL DEFAULT 0,
                can_store INTEGER DEFAULT 1,
                can_recover INTEGER DEFAULT 1,
                max_storage INTEGER DEFAULT 0,
                supports_encryption INTEGER DEFAULT 1,
                supports_auto_refresh INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_seen INTEGER,
                last_health_check INTEGER,
                shares_held INTEGER DEFAULT 0,
                successful_recoveries INTEGER DEFAULT 0,
                notes TEXT
            )",
            [],
        )?;

        // Secret-Guardian mapping (which guardians hold shares for which secrets)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secret_guardians (
                secret_id TEXT NOT NULL,
                guardian_id TEXT NOT NULL,
                share_index INTEGER NOT NULL,
                PRIMARY KEY (secret_id, guardian_id, share_index),
                FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
                FOREIGN KEY (guardian_id) REFERENCES guardians(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Encrypted shards table (for local caching)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS encrypted_shards (
                id TEXT PRIMARY KEY,
                secret_id TEXT NOT NULL,
                guardian_id TEXT NOT NULL DEFAULT '',
                share_index INTEGER NOT NULL,
                encrypted_data BLOB NOT NULL,
                nonce BLOB NOT NULL,
                recipient_public_key TEXT,
                created_at INTEGER NOT NULL,
                share_hash BLOB NOT NULL,
                FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Shard assignments (distribution tracking)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS shard_assignments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shard_id TEXT NOT NULL,
                guardian_id TEXT NOT NULL,
                guardian_peer_id TEXT NOT NULL,
                delivered INTEGER DEFAULT 0,
                delivered_at INTEGER,
                last_attempt_at INTEGER,
                attempt_count INTEGER DEFAULT 0,
                last_error TEXT,
                UNIQUE (shard_id, guardian_id),
                FOREIGN KEY (shard_id) REFERENCES encrypted_shards(id) ON DELETE CASCADE,
                FOREIGN KEY (guardian_id) REFERENCES guardians(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Distribution status (overall status per secret)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS distribution_status (
                secret_id TEXT PRIMARY KEY,
                total_shards INTEGER NOT NULL,
                delivered_shards INTEGER DEFAULT 0,
                pending_shards INTEGER DEFAULT 0,
                failed_shards INTEGER DEFAULT 0,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                status TEXT NOT NULL DEFAULT 'pending',
                FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_backup_chunks_backup
             ON backup_chunks(backup_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snapshots_backup
             ON snapshots(backup_id, timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_verified
             ON chunks(verified_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_schedules_next_run
             ON schedules(next_run, enabled)",
            [],
        )?;

        // Secrets indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secrets_type
             ON secrets(secret_type)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secrets_modified
             ON secrets(modified_at DESC)",
            [],
        )?;

        // Guardians indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_guardians_status
             ON guardians(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_guardians_trust
             ON guardians(trust_level DESC)",
            [],
        )?;

        // Secret-guardian mapping indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secret_guardians_secret
             ON secret_guardians(secret_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secret_guardians_guardian
             ON secret_guardians(guardian_id)",
            [],
        )?;

        // Encrypted shards indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_encrypted_shards_secret
             ON encrypted_shards(secret_id)",
            [],
        )?;

        Ok(())
    }

    // =========================================================================
    // Backup Operations
    // =========================================================================

    /// Create a new backup
    pub fn create_backup(&self, backup: &BackupInfo) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO backups (id, name, source_path, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                backup.id,
                backup.name,
                backup.source_path,
                backup.created_at,
                backup.status,
            ],
        )?;
        Ok(())
    }

    /// Get a backup by ID
    pub fn get_backup(&self, backup_id: &str) -> Result<Option<BackupInfo>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, name, source_path, created_at, last_backup_at,
                    total_size, stored_size, chunk_count, status
             FROM backups WHERE id = ?1",
            params![backup_id],
            |row| {
                Ok(BackupInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    created_at: row.get(3)?,
                    last_backup_at: row.get(4)?,
                    total_size: row.get(5)?,
                    stored_size: row.get(6)?,
                    chunk_count: row.get(7)?,
                    status: row.get(8)?,
                })
            },
        )
        .optional()
        .context("Failed to get backup")
    }

    /// List all backups
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, source_path, created_at, last_backup_at,
                    total_size, stored_size, chunk_count, status
             FROM backups ORDER BY created_at DESC",
        )?;

        let backups = stmt
            .query_map([], |row| {
                Ok(BackupInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    created_at: row.get(3)?,
                    last_backup_at: row.get(4)?,
                    total_size: row.get(5)?,
                    stored_size: row.get(6)?,
                    chunk_count: row.get(7)?,
                    status: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(backups)
    }

    /// Update backup status
    pub fn update_backup_status(&self, backup_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE backups SET status = ?1 WHERE id = ?2",
            params![status, backup_id],
        )?;
        Ok(())
    }

    /// Update backup stats after completion
    pub fn update_backup_stats(
        &self,
        backup_id: &str,
        total_size: i64,
        stored_size: i64,
        chunk_count: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE backups SET
                total_size = ?1,
                stored_size = ?2,
                chunk_count = ?3,
                last_backup_at = ?4
             WHERE id = ?5",
            params![
                total_size,
                stored_size,
                chunk_count,
                Utc::now().timestamp(),
                backup_id
            ],
        )?;
        Ok(())
    }

    /// Delete a backup
    pub fn delete_backup(&self, backup_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM backups WHERE id = ?1", params![backup_id])?;
        Ok(())
    }

    // =========================================================================
    // Chunk Operations
    // =========================================================================

    /// Check if a chunk exists
    pub fn chunk_exists(&self, hash: &[u8; 32]) -> VaultResult<bool> {
        let conn = self.conn.lock();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE hash = ?1",
                params![hash.as_slice()],
                |row| row.get(0),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// Insert a new chunk
    pub fn insert_chunk(
        &self,
        hash: &[u8; 32],
        size: i64,
        compressed_size: i64,
    ) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO chunks (hash, size, compressed_size, ref_count, created_at)
             VALUES (?1, ?2, ?3, 1, ?4)",
            params![
                hash.as_slice(),
                size,
                compressed_size,
                Utc::now().timestamp()
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get chunk info
    pub fn get_chunk(&self, hash: &[u8; 32]) -> VaultResult<Option<ChunkInfo>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT hash, size, compressed_size, ref_count, created_at, verified_at
             FROM chunks WHERE hash = ?1",
            params![hash.as_slice()],
            |row| {
                let hash_bytes: Vec<u8> = row.get(0)?;
                let mut hash_arr = [0u8; 32];
                hash_arr.copy_from_slice(&hash_bytes);
                Ok(ChunkInfo {
                    hash: hash_arr,
                    size: row.get(1)?,
                    compressed_size: row.get(2)?,
                    ref_count: row.get(3)?,
                    created_at: row.get(4)?,
                    verified_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| VaultError::Database(e.to_string()))
    }

    /// Increment chunk reference count
    pub fn increment_chunk_ref(&self, hash: &[u8; 32]) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE chunks SET ref_count = ref_count + 1 WHERE hash = ?1",
            params![hash.as_slice()],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement chunk reference count, return true if it should be deleted
    pub fn decrement_chunk_ref(&self, hash: &[u8; 32]) -> VaultResult<bool> {
        let conn = self.conn.lock();

        // Get current count
        let count: i64 = conn
            .query_row(
                "SELECT ref_count FROM chunks WHERE hash = ?1",
                params![hash.as_slice()],
                |row| row.get(0),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        if count <= 1 {
            // Delete the chunk
            conn.execute(
                "DELETE FROM chunks WHERE hash = ?1",
                params![hash.as_slice()],
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;
            Ok(true)
        } else {
            // Decrement
            conn.execute(
                "UPDATE chunks SET ref_count = ref_count - 1 WHERE hash = ?1",
                params![hash.as_slice()],
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;
            Ok(false)
        }
    }

    /// Mark a chunk as verified
    pub fn mark_chunk_verified(&self, hash: &[u8; 32]) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE chunks SET verified_at = ?1 WHERE hash = ?2",
            params![Utc::now().timestamp(), hash.as_slice()],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get chunks that haven't been verified recently
    pub fn get_unverified_chunks(&self, older_than_days: i64) -> VaultResult<Vec<[u8; 32]>> {
        let conn = self.conn.lock();
        let cutoff = Utc::now().timestamp() - (older_than_days * 86400);

        let mut stmt = conn
            .prepare(
                "SELECT hash FROM chunks
                 WHERE verified_at IS NULL OR verified_at < ?1
                 LIMIT 100",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let hashes = stmt
            .query_map(params![cutoff], |row| {
                let hash_bytes: Vec<u8> = row.get(0)?;
                let mut hash_arr = [0u8; 32];
                hash_arr.copy_from_slice(&hash_bytes);
                Ok(hash_arr)
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(hashes)
    }

    /// Get deduplication statistics
    pub fn get_dedup_stats(&self) -> VaultResult<DedupStats> {
        let conn = self.conn.lock();

        let (unique_chunks, total_refs, unique_bytes, compressed_bytes): (i64, i64, i64, i64) =
            conn.query_row(
                "SELECT
                    COUNT(*),
                    COALESCE(SUM(ref_count), 0),
                    COALESCE(SUM(size), 0),
                    COALESCE(SUM(compressed_size), 0)
                 FROM chunks",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        // Calculate total bytes (sum of size * ref_count for each chunk)
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size * ref_count), 0) FROM chunks",
                [],
                |row| row.get(0),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let dedup_ratio = if unique_chunks > 0 {
            total_refs as f64 / unique_chunks as f64
        } else {
            1.0
        };

        let compression_ratio = if compressed_bytes > 0 {
            unique_bytes as f64 / compressed_bytes as f64
        } else {
            1.0
        };

        let space_saved = total_bytes - compressed_bytes;

        Ok(DedupStats {
            total_references: total_refs,
            unique_chunks,
            total_bytes,
            unique_bytes,
            compressed_bytes,
            compression_ratio,
            dedup_ratio,
            space_saved,
        })
    }

    // =========================================================================
    // Backup Chunk Mapping
    // =========================================================================

    /// Add chunk to backup mapping
    pub fn add_backup_chunk(
        &self,
        backup_id: &str,
        chunk_hash: &[u8; 32],
        file_path: &str,
        chunk_offset: i64,
    ) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO backup_chunks
             (backup_id, chunk_hash, file_path, chunk_offset)
             VALUES (?1, ?2, ?3, ?4)",
            params![backup_id, chunk_hash.as_slice(), file_path, chunk_offset],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get chunks for a file in a backup
    pub fn get_file_chunks(&self, backup_id: &str, file_path: &str) -> VaultResult<Vec<[u8; 32]>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT chunk_hash FROM backup_chunks
                 WHERE backup_id = ?1 AND file_path = ?2
                 ORDER BY chunk_offset",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let hashes = stmt
            .query_map(params![backup_id, file_path], |row| {
                let hash_bytes: Vec<u8> = row.get(0)?;
                let mut hash_arr = [0u8; 32];
                hash_arr.copy_from_slice(&hash_bytes);
                Ok(hash_arr)
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(hashes)
    }

    /// Get all unique chunk hashes for a backup
    ///
    /// Returns a list of unique chunk hashes referenced by this backup.
    /// Used for decrementing chunk references when deleting a backup.
    pub fn get_backup_chunk_hashes(&self, backup_id: &str) -> VaultResult<Vec<[u8; 32]>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT chunk_hash FROM backup_chunks
                 WHERE backup_id = ?1",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let hashes = stmt
            .query_map(params![backup_id], |row| {
                let hash_bytes: Vec<u8> = row.get(0)?;
                let mut hash_arr = [0u8; 32];
                hash_arr.copy_from_slice(&hash_bytes);
                Ok(hash_arr)
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(hashes)
    }

    // =========================================================================
    // Snapshot Operations
    // =========================================================================

    /// Create a snapshot
    pub fn create_snapshot(&self, snapshot: &SnapshotInfo) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO snapshots
             (id, backup_id, timestamp, total_size, stored_size, file_count, manifest)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                snapshot.id,
                snapshot.backup_id,
                snapshot.timestamp,
                snapshot.total_size,
                snapshot.stored_size,
                snapshot.file_count,
                snapshot.manifest,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// List snapshots for a backup
    pub fn list_snapshots(&self, backup_id: &str) -> VaultResult<Vec<SnapshotInfo>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, backup_id, timestamp, total_size, stored_size, file_count, manifest
                 FROM snapshots WHERE backup_id = ?1 ORDER BY timestamp DESC",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let snapshots = stmt
            .query_map(params![backup_id], |row| {
                Ok(SnapshotInfo {
                    id: row.get(0)?,
                    backup_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    total_size: row.get(3)?,
                    stored_size: row.get(4)?,
                    file_count: row.get(5)?,
                    manifest: row.get(6)?,
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(snapshots)
    }

    /// Get a snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: &str) -> VaultResult<Option<SnapshotInfo>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, backup_id, timestamp, total_size, stored_size, file_count, manifest
             FROM snapshots WHERE id = ?1",
            params![snapshot_id],
            |row| {
                Ok(SnapshotInfo {
                    id: row.get(0)?,
                    backup_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    total_size: row.get(3)?,
                    stored_size: row.get(4)?,
                    file_count: row.get(5)?,
                    manifest: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| VaultError::Database(e.to_string()))
    }

    /// Delete old snapshots beyond retention
    pub fn prune_old_snapshots(
        &self,
        backup_id: &str,
        keep_count: i64,
    ) -> VaultResult<Vec<String>> {
        let conn = self.conn.lock();

        // Get snapshots to delete
        let mut stmt = conn
            .prepare(
                "SELECT id FROM snapshots
                 WHERE backup_id = ?1
                 ORDER BY timestamp DESC
                 LIMIT -1 OFFSET ?2",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![backup_id, keep_count], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete them
        for id in &ids {
            conn.execute("DELETE FROM snapshots WHERE id = ?1", params![id])
                .map_err(|e| VaultError::Database(e.to_string()))?;
        }

        Ok(ids)
    }

    // =========================================================================
    // Schedule Operations
    // =========================================================================

    /// Create a schedule
    pub fn create_schedule(&self, schedule: &ScheduleInfo) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO schedules
             (id, backup_id, frequency, retention_days, next_run, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                schedule.id,
                schedule.backup_id,
                schedule.frequency,
                schedule.retention_days,
                schedule.next_run,
                schedule.enabled as i32,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get schedule for a backup
    pub fn get_schedule(&self, backup_id: &str) -> VaultResult<Option<ScheduleInfo>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, backup_id, frequency, retention_days, last_run, next_run, enabled
             FROM schedules WHERE backup_id = ?1",
            params![backup_id],
            |row| {
                Ok(ScheduleInfo {
                    id: row.get(0)?,
                    backup_id: row.get(1)?,
                    frequency: row.get(2)?,
                    retention_days: row.get(3)?,
                    last_run: row.get(4)?,
                    next_run: row.get(5)?,
                    enabled: row.get::<_, i32>(6)? != 0,
                })
            },
        )
        .optional()
        .map_err(|e| VaultError::Database(e.to_string()))
    }

    /// List all schedules
    pub fn list_schedules(&self) -> VaultResult<Vec<ScheduleInfo>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, backup_id, frequency, retention_days, last_run, next_run, enabled
                 FROM schedules ORDER BY next_run",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let schedules = stmt
            .query_map([], |row| {
                Ok(ScheduleInfo {
                    id: row.get(0)?,
                    backup_id: row.get(1)?,
                    frequency: row.get(2)?,
                    retention_days: row.get(3)?,
                    last_run: row.get(4)?,
                    next_run: row.get(5)?,
                    enabled: row.get::<_, i32>(6)? != 0,
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(schedules)
    }

    /// Update schedule after run
    pub fn update_schedule_run(&self, schedule_id: &str, next_run: i64) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE schedules SET last_run = ?1, next_run = ?2 WHERE id = ?3",
            params![Utc::now().timestamp(), next_run, schedule_id],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Enable/disable schedule
    pub fn set_schedule_enabled(&self, schedule_id: &str, enabled: bool) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE schedules SET enabled = ?1 WHERE id = ?2",
            params![enabled as i32, schedule_id],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a schedule
    pub fn delete_schedule(&self, schedule_id: &str) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM schedules WHERE id = ?1", params![schedule_id])
            .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Storage Peers
    // =========================================================================

    /// Add shard location
    pub fn add_shard_location(
        &self,
        peer_id: &str,
        chunk_hash: &[u8; 32],
        shard_index: i32,
    ) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO storage_peers
             (peer_id, chunk_hash, shard_index, stored_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                peer_id,
                chunk_hash.as_slice(),
                shard_index,
                Utc::now().timestamp()
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get shard locations for a chunk
    pub fn get_shard_locations(&self, chunk_hash: &[u8; 32]) -> VaultResult<Vec<(i32, String)>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT shard_index, peer_id FROM storage_peers
                 WHERE chunk_hash = ?1 ORDER BY shard_index",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let locations = stmt
            .query_map(params![chunk_hash.as_slice()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(locations)
    }

    // =========================================================================
    // Local Identity
    // =========================================================================

    /// Get local identity
    pub fn get_local_identity(&self) -> Result<Option<LocalIdentity>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT peer_id, display_name, public_key, private_key, created_at
             FROM local_identity WHERE id = 1",
            [],
            |row| {
                Ok(LocalIdentity {
                    peer_id: row.get(0)?,
                    display_name: row.get(1)?,
                    public_key: row.get(2)?,
                    private_key: row.get(3)?,
                    created_at: row.get(4)?,
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
            "INSERT OR REPLACE INTO local_identity
             (id, peer_id, display_name, public_key, private_key, created_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                identity.peer_id,
                identity.display_name,
                identity.public_key,
                identity.private_key,
                identity.created_at,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Settings
    // =========================================================================

    /// Get a setting
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

    /// Set a setting
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // =========================================================================
    // Storage Statistics
    // =========================================================================

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> VaultResult<StorageStats> {
        let conn = self.conn.lock();

        let (total_backed_up, stored_size, chunk_count): (i64, i64, i64) = conn
            .query_row(
                "SELECT
                    COALESCE(SUM(total_size), 0),
                    COALESCE(SUM(stored_size), 0),
                    COALESCE(SUM(chunk_count), 0)
                 FROM backups",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let backup_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM backups", [], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let snapshot_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let dedup_stats = self.get_dedup_stats()?;

        Ok(StorageStats {
            total_backed_up,
            stored_size,
            chunk_count,
            backup_count,
            snapshot_count,
            dedup_ratio: dedup_stats.dedup_ratio,
            compression_ratio: dedup_stats.compression_ratio,
            space_saved: dedup_stats.space_saved,
        })
    }

    // =========================================================================
    // Secret Operations
    // =========================================================================

    /// Create a new secret
    pub fn create_secret(&self, secret: &SecretInfo) -> VaultResult<()> {
        let conn = self.conn.lock();
        let tags_json = serde_json::to_string(&secret.tags)
            .map_err(|e| VaultError::Database(format!("Failed to serialize tags: {}", e)))?;

        conn.execute(
            "INSERT INTO secrets (
                id, name, description, secret_type, threshold, total_shares,
                created_at, modified_at, last_accessed_at, rotation_count,
                last_rotated_at, key_salt, distribution_complete, tags, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                secret.id,
                secret.name,
                secret.description,
                format!("{:?}", secret.secret_type).to_lowercase(),
                secret.shamir_config.threshold,
                secret.shamir_config.total_shares,
                secret.created_at,
                secret.modified_at,
                secret.last_accessed_at,
                secret.rotation_count,
                secret.last_rotated_at,
                secret.key_salt.as_ref().map(|s| s.as_slice()),
                secret.distribution_complete as i32,
                tags_json,
                secret.metadata,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get a secret by ID
    pub fn get_secret(&self, secret_id: &str) -> VaultResult<Option<SecretInfo>> {
        let conn = self.conn.lock();

        let secret = conn
            .query_row(
                "SELECT id, name, description, secret_type, threshold, total_shares,
                    created_at, modified_at, last_accessed_at, rotation_count,
                    last_rotated_at, key_salt, distribution_complete, tags, metadata
             FROM secrets WHERE id = ?1",
                params![secret_id],
                |row| {
                    let secret_type_str: String = row.get(3)?;
                    let key_salt_bytes: Option<Vec<u8>> = row.get(11)?;
                    let tags_json: String = row.get(13)?;

                    Ok(SecretInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        secret_type: parse_secret_type(&secret_type_str),
                        shamir_config: ShamirConfig {
                            threshold: row.get(4)?,
                            total_shares: row.get(5)?,
                        },
                        created_at: row.get(6)?,
                        modified_at: row.get(7)?,
                        last_accessed_at: row.get(8)?,
                        rotation_count: row.get(9)?,
                        last_rotated_at: row.get(10)?,
                        key_salt: key_salt_bytes.and_then(|b| {
                            if b.len() == 32 {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&b);
                                Some(arr)
                            } else {
                                None
                            }
                        }),
                        distribution_complete: row.get::<_, i32>(12)? != 0,
                        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                        metadata: row.get(14)?,
                        guardian_ids: Vec::new(), // Loaded below
                    })
                },
            )
            .optional()
            .map_err(|e| VaultError::Database(e.to_string()))?;

        // Load guardian IDs if secret exists (within same lock scope to avoid deadlock)
        if let Some(mut secret) = secret {
            let mut stmt = conn
                .prepare("SELECT DISTINCT guardian_id FROM secret_guardians WHERE secret_id = ?1")
                .map_err(|e| VaultError::Database(e.to_string()))?;
            secret.guardian_ids = stmt
                .query_map(params![&secret.id], |row| row.get(0))
                .map_err(|e| VaultError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(Some(secret))
        } else {
            Ok(None)
        }
    }

    /// List all secrets
    pub fn list_secrets(&self) -> VaultResult<Vec<SecretInfo>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, secret_type, threshold, total_shares,
                        created_at, modified_at, last_accessed_at, rotation_count,
                        last_rotated_at, key_salt, distribution_complete, tags, metadata
                 FROM secrets ORDER BY modified_at DESC",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let secrets_raw = stmt
            .query_map([], |row| {
                let secret_type_str: String = row.get(3)?;
                let key_salt_bytes: Option<Vec<u8>> = row.get(11)?;
                let tags_json: String = row.get(13)?;

                Ok(SecretInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    secret_type: parse_secret_type(&secret_type_str),
                    shamir_config: ShamirConfig {
                        threshold: row.get(4)?,
                        total_shares: row.get(5)?,
                    },
                    created_at: row.get(6)?,
                    modified_at: row.get(7)?,
                    last_accessed_at: row.get(8)?,
                    rotation_count: row.get(9)?,
                    last_rotated_at: row.get(10)?,
                    key_salt: key_salt_bytes.and_then(|b| {
                        if b.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&b);
                            Some(arr)
                        } else {
                            None
                        }
                    }),
                    distribution_complete: row.get::<_, i32>(12)? != 0,
                    tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                    metadata: row.get(14)?,
                    guardian_ids: Vec::new(),
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        // Need to drop the statement and conn lock before calling get_secret_guardian_ids
        drop(stmt);
        drop(conn);

        // Load guardian IDs for each secret
        let mut secrets = Vec::new();
        for mut secret in secrets_raw {
            secret.guardian_ids = self.get_secret_guardian_ids(&secret.id)?;
            secrets.push(secret);
        }

        Ok(secrets)
    }

    /// Update secret metadata
    pub fn update_secret(&self, secret: &SecretInfo) -> VaultResult<()> {
        let conn = self.conn.lock();
        let tags_json = serde_json::to_string(&secret.tags)
            .map_err(|e| VaultError::Database(format!("Failed to serialize tags: {}", e)))?;

        conn.execute(
            "UPDATE secrets SET
                name = ?1, description = ?2, modified_at = ?3, last_accessed_at = ?4,
                rotation_count = ?5, last_rotated_at = ?6, distribution_complete = ?7,
                tags = ?8, metadata = ?9
             WHERE id = ?10",
            params![
                secret.name,
                secret.description,
                secret.modified_at,
                secret.last_accessed_at,
                secret.rotation_count,
                secret.last_rotated_at,
                secret.distribution_complete as i32,
                tags_json,
                secret.metadata,
                secret.id,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a secret
    pub fn delete_secret(&self, secret_id: &str) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM secrets WHERE id = ?1", params![secret_id])
            .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get guardian IDs for a secret
    fn get_secret_guardian_ids(&self, secret_id: &str) -> VaultResult<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT DISTINCT guardian_id FROM secret_guardians WHERE secret_id = ?1")
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let ids = stmt
            .query_map(params![secret_id], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    /// Add guardian to secret mapping
    pub fn add_secret_guardian(
        &self,
        secret_id: &str,
        guardian_id: &str,
        share_index: u8,
    ) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO secret_guardians (secret_id, guardian_id, share_index)
             VALUES (?1, ?2, ?3)",
            params![secret_id, guardian_id, share_index as i32],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Remove guardian from secret
    pub fn remove_secret_guardian(&self, secret_id: &str, guardian_id: &str) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM secret_guardians WHERE secret_id = ?1 AND guardian_id = ?2",
            params![secret_id, guardian_id],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Guardian Operations
    // =========================================================================

    /// Create a new guardian
    pub fn create_guardian(&self, guardian: &Guardian) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO guardians (
                id, name, peer_id, public_key, status, trust_level,
                can_store, can_recover, max_storage, supports_encryption,
                supports_auto_refresh, created_at, last_seen, last_health_check,
                shares_held, successful_recoveries, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                guardian.id,
                guardian.name,
                guardian.peer_id,
                guardian.public_key,
                guardian.status.to_string(),
                guardian.trust_level as i32,
                guardian.capabilities.can_store as i32,
                guardian.capabilities.can_recover as i32,
                guardian.capabilities.max_storage as i64,
                guardian.capabilities.supports_encryption as i32,
                guardian.capabilities.supports_auto_refresh as i32,
                guardian.created_at,
                guardian.last_seen,
                guardian.last_health_check,
                guardian.shares_held,
                guardian.successful_recoveries,
                guardian.notes,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get a guardian by ID
    pub fn get_guardian(&self, guardian_id: &str) -> VaultResult<Option<Guardian>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, name, peer_id, public_key, status, trust_level,
                    can_store, can_recover, max_storage, supports_encryption,
                    supports_auto_refresh, created_at, last_seen, last_health_check,
                    shares_held, successful_recoveries, notes
             FROM guardians WHERE id = ?1",
            params![guardian_id],
            |row| {
                Ok(Guardian {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    peer_id: row.get(2)?,
                    public_key: row.get(3)?,
                    status: parse_guardian_status(&row.get::<_, String>(4)?),
                    trust_level: parse_trust_level(row.get(5)?),
                    capabilities: GuardianCapabilities {
                        can_store: row.get::<_, i32>(6)? != 0,
                        can_recover: row.get::<_, i32>(7)? != 0,
                        max_storage: row.get::<_, i64>(8)? as u64,
                        supports_encryption: row.get::<_, i32>(9)? != 0,
                        supports_auto_refresh: row.get::<_, i32>(10)? != 0,
                    },
                    created_at: row.get(11)?,
                    last_seen: row.get(12)?,
                    last_health_check: row.get(13)?,
                    shares_held: row.get(14)?,
                    successful_recoveries: row.get(15)?,
                    notes: row.get(16)?,
                })
            },
        )
        .optional()
        .map_err(|e| VaultError::Database(e.to_string()))
    }

    /// List all guardians
    pub fn list_guardians(&self) -> VaultResult<Vec<Guardian>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, peer_id, public_key, status, trust_level,
                        can_store, can_recover, max_storage, supports_encryption,
                        supports_auto_refresh, created_at, last_seen, last_health_check,
                        shares_held, successful_recoveries, notes
                 FROM guardians ORDER BY trust_level DESC, name",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let guardians = stmt
            .query_map([], |row| {
                Ok(Guardian {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    peer_id: row.get(2)?,
                    public_key: row.get(3)?,
                    status: parse_guardian_status(&row.get::<_, String>(4)?),
                    trust_level: parse_trust_level(row.get(5)?),
                    capabilities: GuardianCapabilities {
                        can_store: row.get::<_, i32>(6)? != 0,
                        can_recover: row.get::<_, i32>(7)? != 0,
                        max_storage: row.get::<_, i64>(8)? as u64,
                        supports_encryption: row.get::<_, i32>(9)? != 0,
                        supports_auto_refresh: row.get::<_, i32>(10)? != 0,
                    },
                    created_at: row.get(11)?,
                    last_seen: row.get(12)?,
                    last_health_check: row.get(13)?,
                    shares_held: row.get(14)?,
                    successful_recoveries: row.get(15)?,
                    notes: row.get(16)?,
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(guardians)
    }

    /// Update a guardian
    pub fn update_guardian(&self, guardian: &Guardian) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE guardians SET
                name = ?1, status = ?2, trust_level = ?3, can_store = ?4,
                can_recover = ?5, max_storage = ?6, supports_encryption = ?7,
                supports_auto_refresh = ?8, last_seen = ?9, last_health_check = ?10,
                shares_held = ?11, successful_recoveries = ?12, notes = ?13
             WHERE id = ?14",
            params![
                guardian.name,
                guardian.status.to_string(),
                guardian.trust_level as i32,
                guardian.capabilities.can_store as i32,
                guardian.capabilities.can_recover as i32,
                guardian.capabilities.max_storage as i64,
                guardian.capabilities.supports_encryption as i32,
                guardian.capabilities.supports_auto_refresh as i32,
                guardian.last_seen,
                guardian.last_health_check,
                guardian.shares_held,
                guardian.successful_recoveries,
                guardian.notes,
                guardian.id,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a guardian
    pub fn delete_guardian(&self, guardian_id: &str) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM guardians WHERE id = ?1", params![guardian_id])
            .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Encrypted Shard Operations
    // =========================================================================

    /// Store an encrypted shard
    pub fn store_encrypted_shard(&self, shard: &EncryptedShard) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO encrypted_shards (
                id, secret_id, guardian_id, share_index, encrypted_data, nonce,
                recipient_public_key, created_at, share_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                shard.id,
                shard.secret_id,
                shard.guardian_id,
                shard.share_index as i32,
                shard.encrypted_data,
                shard.nonce.as_slice(),
                shard.recipient_public_key,
                shard.created_at,
                shard.share_hash.as_slice(),
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get encrypted shards for a secret
    pub fn get_encrypted_shards(&self, secret_id: &str) -> VaultResult<Vec<EncryptedShard>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, secret_id, guardian_id, share_index, encrypted_data, nonce,
                        recipient_public_key, created_at, share_hash
                 FROM encrypted_shards WHERE secret_id = ?1 ORDER BY share_index",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let shards = stmt
            .query_map(params![secret_id], |row| {
                let nonce_bytes: Vec<u8> = row.get(5)?;
                let hash_bytes: Vec<u8> = row.get(8)?;

                let mut nonce = [0u8; 24];
                if nonce_bytes.len() == 24 {
                    nonce.copy_from_slice(&nonce_bytes);
                }

                let mut share_hash = [0u8; 32];
                if hash_bytes.len() == 32 {
                    share_hash.copy_from_slice(&hash_bytes);
                }

                Ok(EncryptedShard {
                    id: row.get(0)?,
                    secret_id: row.get(1)?,
                    guardian_id: row.get(2)?,
                    share_index: row.get::<_, i32>(3)? as u8,
                    encrypted_data: row.get(4)?,
                    nonce,
                    recipient_public_key: row.get(6)?,
                    created_at: row.get(7)?,
                    share_hash,
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(shards)
    }

    /// Delete encrypted shards for a secret
    pub fn delete_encrypted_shards(&self, secret_id: &str) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM encrypted_shards WHERE secret_id = ?1",
            params![secret_id],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Distribution Status Operations
    // =========================================================================

    /// Create or update distribution status
    pub fn upsert_distribution_status(&self, status: &DistributionStatus) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO distribution_status (
                secret_id, total_shards, delivered_shards, pending_shards,
                failed_shards, started_at, completed_at, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                status.secret_id,
                status.total_shards,
                status.delivered_shards,
                status.pending_shards,
                status.failed_shards,
                status.started_at,
                status.completed_at,
                format!("{:?}", status.status).to_lowercase(),
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get distribution status for a secret
    pub fn get_distribution_status(
        &self,
        secret_id: &str,
    ) -> VaultResult<Option<DistributionStatus>> {
        let conn = self.conn.lock();

        let status = conn
            .query_row(
                "SELECT secret_id, total_shards, delivered_shards, pending_shards,
                    failed_shards, started_at, completed_at, status
             FROM distribution_status WHERE secret_id = ?1",
                params![secret_id],
                |row| {
                    let status_str: String = row.get(7)?;
                    Ok(DistributionStatus {
                        secret_id: row.get(0)?,
                        total_shards: row.get(1)?,
                        delivered_shards: row.get(2)?,
                        pending_shards: row.get(3)?,
                        failed_shards: row.get(4)?,
                        started_at: row.get(5)?,
                        completed_at: row.get(6)?,
                        status: parse_distribution_state(&status_str),
                        assignments: Vec::new(), // Loaded separately
                    })
                },
            )
            .optional()
            .map_err(|e| VaultError::Database(e.to_string()))?;

        // Load assignments if status exists
        if let Some(mut status) = status {
            status.assignments = self.get_shard_assignments_for_secret(&status.secret_id)?;
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    // =========================================================================
    // Shard Assignment Operations
    // =========================================================================

    /// Create a shard assignment
    pub fn create_shard_assignment(&self, assignment: &ShardAssignment) -> VaultResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO shard_assignments (
                shard_id, guardian_id, guardian_peer_id, delivered,
                delivered_at, last_attempt_at, attempt_count, last_error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                assignment.shard_id,
                assignment.guardian_id,
                assignment.guardian_peer_id,
                assignment.delivered as i32,
                assignment.delivered_at,
                assignment.last_attempt_at,
                assignment.attempt_count,
                assignment.last_error,
            ],
        )
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get shard assignments for a secret
    fn get_shard_assignments_for_secret(
        &self,
        secret_id: &str,
    ) -> VaultResult<Vec<ShardAssignment>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT sa.shard_id, sa.guardian_id, sa.guardian_peer_id, sa.delivered,
                        sa.delivered_at, sa.last_attempt_at, sa.attempt_count, sa.last_error
                 FROM shard_assignments sa
                 JOIN encrypted_shards es ON sa.shard_id = es.id
                 WHERE es.secret_id = ?1",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let assignments = stmt
            .query_map(params![secret_id], |row| {
                Ok(ShardAssignment {
                    shard_id: row.get(0)?,
                    guardian_id: row.get(1)?,
                    guardian_peer_id: row.get(2)?,
                    delivered: row.get::<_, i32>(3)? != 0,
                    delivered_at: row.get(4)?,
                    last_attempt_at: row.get(5)?,
                    attempt_count: row.get(6)?,
                    last_error: row.get(7)?,
                })
            })
            .map_err(|e| VaultError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(assignments)
    }

    /// Update shard assignment delivery status
    pub fn update_shard_assignment_delivery(
        &self,
        shard_id: &str,
        guardian_id: &str,
        delivered: bool,
        error: Option<&str>,
    ) -> VaultResult<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        if delivered {
            conn.execute(
                "UPDATE shard_assignments SET
                    delivered = 1, delivered_at = ?1, last_attempt_at = ?1,
                    attempt_count = attempt_count + 1, last_error = NULL
                 WHERE shard_id = ?2 AND guardian_id = ?3",
                params![now, shard_id, guardian_id],
            )
        } else {
            conn.execute(
                "UPDATE shard_assignments SET
                    last_attempt_at = ?1, attempt_count = attempt_count + 1, last_error = ?2
                 WHERE shard_id = ?3 AND guardian_id = ?4",
                params![now, error, shard_id, guardian_id],
            )
        }
        .map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get shard assignments for a secret (public interface)
    pub fn get_shard_assignments(&self, secret_id: &str) -> VaultResult<Vec<ShardAssignment>> {
        self.get_shard_assignments_for_secret(secret_id)
    }

    /// Get encrypted shard by guardian ID for a specific secret
    pub fn get_shard_by_guardian(
        &self,
        secret_id: &str,
        guardian_id: &str,
    ) -> VaultResult<Option<EncryptedShard>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, secret_id, guardian_id, share_index, encrypted_data, nonce,
                        recipient_public_key, created_at, share_hash
                 FROM encrypted_shards
                 WHERE secret_id = ?1 AND guardian_id = ?2",
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![secret_id, guardian_id], |row| {
                let nonce_bytes: Vec<u8> = row.get(5)?;
                let hash_bytes: Vec<u8> = row.get(8)?;

                let mut nonce = [0u8; 24];
                if nonce_bytes.len() == 24 {
                    nonce.copy_from_slice(&nonce_bytes);
                }

                let mut share_hash = [0u8; 32];
                if hash_bytes.len() == 32 {
                    share_hash.copy_from_slice(&hash_bytes);
                }

                Ok(EncryptedShard {
                    id: row.get(0)?,
                    secret_id: row.get(1)?,
                    guardian_id: row.get(2)?,
                    share_index: row.get::<_, i32>(3)? as u8,
                    encrypted_data: row.get(4)?,
                    nonce,
                    recipient_public_key: row.get(6)?,
                    created_at: row.get(7)?,
                    share_hash,
                })
            })
            .optional()
            .map_err(|e| VaultError::Database(e.to_string()))?;

        Ok(result)
    }

    // =========================================================================
    // Vault Statistics
    // =========================================================================

    /// Get vault statistics
    pub fn get_vault_stats(&self) -> VaultResult<VaultStats> {
        let conn = self.conn.lock();

        let secret_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM secrets", [], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let guardian_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM guardians", [], |row| row.get(0))
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let online_guardians: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM guardians WHERE status = 'online'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let total_shards: i64 = conn
            .query_row("SELECT COUNT(*) FROM encrypted_shards", [], |row| {
                row.get(0)
            })
            .map_err(|e| VaultError::Database(e.to_string()))?;

        let distributed_secrets: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM secrets WHERE distribution_complete = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| VaultError::Database(e.to_string()))?;

        Ok(VaultStats {
            secret_count,
            guardian_count,
            online_guardians,
            total_shards,
            distributed_secrets,
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_secret_type(s: &str) -> SecretType {
    match s.to_lowercase().as_str() {
        "generic" => SecretType::Generic,
        "cryptokey" | "crypto_key" => SecretType::CryptoKey,
        "password" => SecretType::Password,
        "recoveryphrase" | "recovery_phrase" => SecretType::RecoveryPhrase,
        "certificate" => SecretType::Certificate,
        "apikey" | "api_key" => SecretType::ApiKey,
        "documentkey" | "document_key" => SecretType::DocumentKey,
        "sshkey" | "ssh_key" => SecretType::SshKey,
        "pgpkey" | "pgp_key" => SecretType::PgpKey,
        _ => SecretType::Generic,
    }
}

fn parse_guardian_status(s: &str) -> GuardianStatus {
    match s.to_lowercase().as_str() {
        "online" => GuardianStatus::Online,
        "offline" => GuardianStatus::Offline,
        "pending" => GuardianStatus::Pending,
        "failed" => GuardianStatus::Failed,
        "revoked" => GuardianStatus::Revoked,
        _ => GuardianStatus::Pending,
    }
}

fn parse_trust_level(level: i32) -> TrustLevel {
    match level {
        0 => TrustLevel::Untrusted,
        1 => TrustLevel::Basic,
        2 => TrustLevel::Trusted,
        3 => TrustLevel::High,
        4 => TrustLevel::Ultimate,
        _ => TrustLevel::Untrusted,
    }
}

fn parse_distribution_state(s: &str) -> DistributionState {
    match s.to_lowercase().as_str() {
        "pending" => DistributionState::Pending,
        "inprogress" | "in_progress" => DistributionState::InProgress,
        "complete" => DistributionState::Complete,
        "partialsuccess" | "partial_success" => DistributionState::PartialSuccess,
        "failed" => DistributionState::Failed,
        _ => DistributionState::Pending,
    }
}

// =============================================================================
// Data Models
// =============================================================================

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub secret_count: i64,
    pub guardian_count: i64,
    pub online_guardians: i64,
    pub total_shards: i64,
    pub distributed_secrets: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIdentity {
    pub peer_id: String,
    pub display_name: String,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub created_at: i64,
    pub last_backup_at: Option<i64>,
    pub total_size: i64,
    pub stored_size: i64,
    pub chunk_count: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub backup_id: String,
    pub timestamp: i64,
    pub total_size: i64,
    pub stored_size: i64,
    pub file_count: i64,
    pub manifest: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub id: String,
    pub backup_id: String,
    pub frequency: String, // "hourly", "daily", "weekly"
    pub retention_days: i64,
    pub last_run: Option<i64>,
    pub next_run: Option<i64>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_backed_up: i64,
    pub stored_size: i64,
    pub chunk_count: i64,
    pub backup_count: i64,
    pub snapshot_count: i64,
    pub dedup_ratio: f64,
    pub compression_ratio: f64,
    pub space_saved: i64,
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

        let backups = db.list_backups().unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_backup_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let backup = BackupInfo {
            id: "test-backup-id".to_string(),
            name: "Test Backup".to_string(),
            source_path: "/home/user/documents".to_string(),
            created_at: Utc::now().timestamp(),
            last_backup_at: None,
            total_size: 0,
            stored_size: 0,
            chunk_count: 0,
            status: "idle".to_string(),
        };

        db.create_backup(&backup).unwrap();

        let retrieved = db.get_backup(&backup.id).unwrap().unwrap();
        assert_eq!(retrieved.name, backup.name);
        assert_eq!(retrieved.source_path, backup.source_path);
    }

    #[test]
    fn test_chunk_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let hash = [42u8; 32];

        // Insert chunk
        db.insert_chunk(&hash, 1024, 512).unwrap();
        assert!(db.chunk_exists(&hash).unwrap());

        // Get chunk info
        let info = db.get_chunk(&hash).unwrap().unwrap();
        assert_eq!(info.size, 1024);
        assert_eq!(info.compressed_size, 512);
        assert_eq!(info.ref_count, 1);

        // Increment ref
        db.increment_chunk_ref(&hash).unwrap();
        let info = db.get_chunk(&hash).unwrap().unwrap();
        assert_eq!(info.ref_count, 2);

        // Decrement ref
        let should_delete = db.decrement_chunk_ref(&hash).unwrap();
        assert!(!should_delete);

        let should_delete = db.decrement_chunk_ref(&hash).unwrap();
        assert!(should_delete);
        assert!(!db.chunk_exists(&hash).unwrap());
    }

    #[test]
    fn test_schedule_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create backup first
        let backup = BackupInfo {
            id: "test-backup-id".to_string(),
            name: "Test Backup".to_string(),
            source_path: "/home/user/documents".to_string(),
            created_at: Utc::now().timestamp(),
            last_backup_at: None,
            total_size: 0,
            stored_size: 0,
            chunk_count: 0,
            status: "idle".to_string(),
        };
        db.create_backup(&backup).unwrap();

        let schedule = ScheduleInfo {
            id: "schedule-1".to_string(),
            backup_id: backup.id.clone(),
            frequency: "daily".to_string(),
            retention_days: 30,
            last_run: None,
            next_run: Some(Utc::now().timestamp() + 86400),
            enabled: true,
        };

        db.create_schedule(&schedule).unwrap();

        let retrieved = db.get_schedule(&backup.id).unwrap().unwrap();
        assert_eq!(retrieved.frequency, "daily");
        assert!(retrieved.enabled);
    }
}
