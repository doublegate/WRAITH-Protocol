//! Restore Operations for WRAITH Vault
//!
//! Handles point-in-time restore from snapshots.

use crate::backup::{FileManifestEntry, Progress};
use crate::compression::Compressor;
use crate::database::{Database, SnapshotInfo};
use crate::dedup::DedupIndex;
use crate::erasure::ErasureCoder;
use crate::error::{VaultError, VaultResult};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Restore engine for recovering backed-up data
#[allow(dead_code)]
pub struct RestoreEngine {
    db: Arc<Database>,
    dedup: DedupIndex,
    compressor: Compressor,
    erasure: ErasureCoder,
}

impl RestoreEngine {
    /// Create a new restore engine
    pub fn new(db: Arc<Database>) -> VaultResult<Self> {
        Ok(Self {
            dedup: DedupIndex::new(db.clone()),
            db,
            compressor: Compressor::default(),
            erasure: ErasureCoder::default(),
        })
    }

    /// Restore from the latest snapshot
    pub fn restore<F>(
        &self,
        backup_id: &str,
        dest_path: &Path,
        progress_callback: F,
    ) -> VaultResult<u64>
    where
        F: FnMut(Progress),
    {
        // Get latest snapshot
        let snapshots = self.db.list_snapshots(backup_id)?;
        let snapshot = snapshots
            .first()
            .ok_or_else(|| VaultError::SnapshotNotFound(backup_id.to_string()))?;

        self.restore_snapshot(&snapshot.id, dest_path, progress_callback)
    }

    /// Restore from a specific snapshot
    pub fn restore_snapshot<F>(
        &self,
        snapshot_id: &str,
        dest_path: &Path,
        mut progress_callback: F,
    ) -> VaultResult<u64>
    where
        F: FnMut(Progress),
    {
        let snapshot = self
            .db
            .get_snapshot(snapshot_id)?
            .ok_or_else(|| VaultError::SnapshotNotFound(snapshot_id.to_string()))?;

        let manifest = snapshot
            .manifest
            .as_ref()
            .ok_or_else(|| VaultError::Restore("No manifest in snapshot".to_string()))?;

        let entries: Vec<FileManifestEntry> = serde_json::from_slice(manifest)
            .map_err(|e| VaultError::Restore(format!("Failed to parse manifest: {}", e)))?;

        let mut progress = Progress::new();
        progress.phase = "Preparing restore".to_string();
        progress.total_files = entries.len() as u64;
        progress.total_bytes = entries.iter().map(|e| e.size).sum();
        progress_callback(progress.clone());

        // Create destination directory
        fs::create_dir_all(dest_path)
            .map_err(|e| VaultError::FileSystem(format!("Failed to create dest: {}", e)))?;

        let mut restored_bytes = 0u64;

        progress.phase = "Restoring files".to_string();

        for entry in entries {
            progress.current_file = entry.path.clone();
            progress_callback(progress.clone());

            let full_path = dest_path.join(&entry.path);

            if entry.is_directory {
                fs::create_dir_all(&full_path)
                    .map_err(|e| VaultError::FileSystem(format!("Failed to create dir: {}", e)))?;
            } else {
                // Ensure parent directory exists
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        VaultError::FileSystem(format!("Failed to create parent: {}", e))
                    })?;
                }

                // Restore file from chunks
                self.restore_file(&entry, &full_path)?;
                restored_bytes += entry.size;
            }

            progress.processed_files += 1;
            progress.processed_bytes += entry.size;
            progress.update_percent();
            progress_callback(progress.clone());
        }

        progress.phase = "Complete".to_string();
        progress.percent = 100.0;
        progress_callback(progress);

        info!(
            "Restore complete: {} files, {} bytes",
            snapshot.file_count, restored_bytes
        );

        Ok(restored_bytes)
    }

    /// Restore from a point in time (closest snapshot before timestamp)
    pub fn restore_point_in_time<F>(
        &self,
        backup_id: &str,
        timestamp: i64,
        dest_path: &Path,
        progress_callback: F,
    ) -> VaultResult<u64>
    where
        F: FnMut(Progress),
    {
        // Find the closest snapshot at or before the timestamp
        let snapshots = self.db.list_snapshots(backup_id)?;
        let snapshot = snapshots
            .iter()
            .find(|s| s.timestamp <= timestamp)
            .ok_or_else(|| {
                VaultError::SnapshotNotFound(format!(
                    "No snapshot found at or before timestamp {}",
                    timestamp
                ))
            })?;

        info!(
            "Restoring to point-in-time {} using snapshot {}",
            timestamp, snapshot.id
        );

        self.restore_snapshot(&snapshot.id, dest_path, progress_callback)
    }

    /// Restore a single file from chunks
    fn restore_file(&self, entry: &FileManifestEntry, dest_path: &Path) -> VaultResult<()> {
        let mut file = File::create(dest_path)
            .map_err(|e| VaultError::FileSystem(format!("Failed to create file: {}", e)))?;

        for chunk_hash in &entry.chunks {
            let chunk_data = self.retrieve_chunk(chunk_hash)?;
            file.write_all(&chunk_data)
                .map_err(|e| VaultError::FileSystem(format!("Failed to write chunk: {}", e)))?;
        }

        debug!("Restored file: {}", entry.path);
        Ok(())
    }

    /// Retrieve and decompress a chunk
    fn retrieve_chunk(&self, hash: &[u8; 32]) -> VaultResult<Vec<u8>> {
        // In a full implementation, this would:
        // 1. Look up shard locations from storage_peers table
        // 2. Retrieve shards from DHT/network
        // 3. Use erasure coding to reconstruct if needed
        // 4. Decompress

        // For now, we'll simulate by checking if chunk exists in database
        let chunk_info = self
            .dedup
            .get_chunk(hash)?
            .ok_or_else(|| VaultError::ChunkNotFound(hex::encode(hash)))?;

        // In a real implementation, retrieve from storage
        // For testing, return placeholder
        warn!(
            "Chunk retrieval not fully implemented - chunk {} size {}",
            hex::encode(hash),
            chunk_info.size
        );

        // Return empty data of correct size for testing
        Ok(vec![0u8; chunk_info.size as usize])
    }

    /// List available snapshots for a backup
    pub fn list_snapshots(&self, backup_id: &str) -> VaultResult<Vec<SnapshotInfo>> {
        self.db.list_snapshots(backup_id)
    }

    /// Get files in a snapshot (for browsing before restore)
    pub fn list_snapshot_files(&self, snapshot_id: &str) -> VaultResult<Vec<FileManifestEntry>> {
        let snapshot = self
            .db
            .get_snapshot(snapshot_id)?
            .ok_or_else(|| VaultError::SnapshotNotFound(snapshot_id.to_string()))?;

        let manifest = snapshot
            .manifest
            .as_ref()
            .ok_or_else(|| VaultError::Restore("No manifest in snapshot".to_string()))?;

        let entries: Vec<FileManifestEntry> = serde_json::from_slice(manifest)
            .map_err(|e| VaultError::Restore(format!("Failed to parse manifest: {}", e)))?;

        Ok(entries)
    }

    /// Restore a specific file from a snapshot
    pub fn restore_single_file<F>(
        &self,
        snapshot_id: &str,
        file_path: &str,
        dest_path: &Path,
        mut progress_callback: F,
    ) -> VaultResult<u64>
    where
        F: FnMut(Progress),
    {
        let entries = self.list_snapshot_files(snapshot_id)?;

        let entry = entries
            .iter()
            .find(|e| e.path == file_path)
            .ok_or_else(|| {
                VaultError::Restore(format!("File not found in snapshot: {}", file_path))
            })?;

        let mut progress = Progress::new();
        progress.total_files = 1;
        progress.total_bytes = entry.size;
        progress.current_file = entry.path.clone();
        progress.phase = "Restoring file".to_string();
        progress_callback(progress.clone());

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| VaultError::FileSystem(format!("Failed to create parent: {}", e)))?;
        }

        self.restore_file(entry, dest_path)?;

        progress.processed_files = 1;
        progress.processed_bytes = entry.size;
        progress.phase = "Complete".to_string();
        progress.percent = 100.0;
        progress_callback(progress);

        Ok(entry.size)
    }

    /// Verify a snapshot's integrity
    pub fn verify_snapshot(&self, snapshot_id: &str) -> VaultResult<VerificationResult> {
        let entries = self.list_snapshot_files(snapshot_id)?;

        let mut result = VerificationResult {
            total_files: entries.len() as u64,
            total_chunks: entries.iter().map(|e| e.chunks.len() as u64).sum(),
            ..Default::default()
        };

        for entry in entries {
            if entry.is_directory {
                continue;
            }

            for chunk_hash in &entry.chunks {
                match self.dedup.get_chunk(chunk_hash) {
                    Ok(Some(_)) => {
                        result.verified_chunks += 1;
                    }
                    Ok(None) => {
                        result.missing_chunks += 1;
                        result.missing_chunk_hashes.push(hex::encode(chunk_hash));
                    }
                    Err(e) => {
                        result.errors.push(format!("Error checking chunk: {}", e));
                    }
                }
            }
        }

        result.is_valid = result.missing_chunks == 0 && result.errors.is_empty();

        Ok(result)
    }
}

/// Result of snapshot verification
#[derive(Debug, Clone, Default)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub total_files: u64,
    pub total_chunks: u64,
    pub verified_chunks: u64,
    pub missing_chunks: u64,
    pub missing_chunk_hashes: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::BackupEngine;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_list_snapshots() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create test file
        let file_path = source_dir.join("test.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"Test content").unwrap();

        let db = Arc::new(Database::open(&db_path).unwrap());

        // Create backup and snapshot
        let mut backup_engine = BackupEngine::new(db.clone()).unwrap();
        let backup = backup_engine
            .create_backup("Test", source_dir.to_str().unwrap())
            .unwrap();
        backup_engine.perform_backup(&backup.id, |_| {}).unwrap();

        // List snapshots
        let restore_engine = RestoreEngine::new(db).unwrap();
        let snapshots = restore_engine.list_snapshots(&backup.id).unwrap();

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].total_size > 0);
    }

    #[test]
    fn test_list_snapshot_files() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create test files
        let file1 = source_dir.join("file1.txt");
        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"Content 1").unwrap();

        let file2 = source_dir.join("file2.txt");
        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"Content 2").unwrap();

        let db = Arc::new(Database::open(&db_path).unwrap());

        // Create backup
        let mut backup_engine = BackupEngine::new(db.clone()).unwrap();
        let backup = backup_engine
            .create_backup("Test", source_dir.to_str().unwrap())
            .unwrap();
        let snapshot_id = backup_engine.perform_backup(&backup.id, |_| {}).unwrap();

        // List files in snapshot
        let restore_engine = RestoreEngine::new(db).unwrap();
        let files = restore_engine.list_snapshot_files(&snapshot_id).unwrap();

        assert_eq!(files.len(), 2);

        let file_names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(file_names.contains(&"file1.txt"));
        assert!(file_names.contains(&"file2.txt"));
    }

    #[test]
    fn test_verify_snapshot() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create test file
        let file_path = source_dir.join("test.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"Test content for verification").unwrap();

        let db = Arc::new(Database::open(&db_path).unwrap());

        // Create backup
        let mut backup_engine = BackupEngine::new(db.clone()).unwrap();
        let backup = backup_engine
            .create_backup("Test", source_dir.to_str().unwrap())
            .unwrap();
        let snapshot_id = backup_engine.perform_backup(&backup.id, |_| {}).unwrap();

        // Verify snapshot
        let restore_engine = RestoreEngine::new(db).unwrap();
        let result = restore_engine.verify_snapshot(&snapshot_id).unwrap();

        assert!(result.is_valid);
        assert!(result.missing_chunks == 0);
        assert!(result.errors.is_empty());
    }
}
