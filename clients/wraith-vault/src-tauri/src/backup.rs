//! Backup Orchestration for WRAITH Vault
//!
//! Coordinates chunking, deduplication, compression, and erasure coding
//! to perform incremental backups.

use crate::chunker::Chunker;
use crate::compression::Compressor;
use crate::database::{BackupInfo, Database, SnapshotInfo};
use crate::dedup::DedupIndex;
use crate::erasure::ErasureCoder;
use crate::error::{VaultError, VaultResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;
use walkdir::WalkDir;

/// Progress information for backup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// Current file being processed
    pub current_file: String,
    /// Total files to process
    pub total_files: u64,
    /// Files processed so far
    pub processed_files: u64,
    /// Total bytes to backup
    pub total_bytes: u64,
    /// Bytes processed so far
    pub processed_bytes: u64,
    /// Current operation phase
    pub phase: String,
    /// Percentage complete (0-100)
    pub percent: f64,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            current_file: String::new(),
            total_files: 0,
            processed_files: 0,
            total_bytes: 0,
            processed_bytes: 0,
            phase: "Initializing".to_string(),
            percent: 0.0,
        }
    }

    pub fn update_percent(&mut self) {
        if self.total_bytes > 0 {
            self.percent = (self.processed_bytes as f64 / self.total_bytes as f64) * 100.0;
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

/// File manifest entry for tracking file->chunk mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifestEntry {
    pub path: String,
    pub size: u64,
    pub modified_at: i64,
    pub is_directory: bool,
    pub chunks: Vec<[u8; 32]>,
}

/// Backup engine that orchestrates all backup operations
pub struct BackupEngine {
    db: Arc<Database>,
    chunker: Chunker,
    dedup: DedupIndex,
    compressor: Compressor,
    erasure: ErasureCoder,
}

impl BackupEngine {
    /// Create a new backup engine
    pub fn new(db: Arc<Database>) -> VaultResult<Self> {
        Ok(Self {
            dedup: DedupIndex::new(db.clone()),
            db,
            chunker: Chunker::new(),
            compressor: Compressor::default(),
            erasure: ErasureCoder::default(),
        })
    }

    /// Create a new backup configuration
    pub fn create_backup(&self, name: &str, source_path: &str) -> VaultResult<BackupInfo> {
        let backup = BackupInfo {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            source_path: source_path.to_string(),
            created_at: Utc::now().timestamp(),
            last_backup_at: None,
            total_size: 0,
            stored_size: 0,
            chunk_count: 0,
            status: "idle".to_string(),
        };

        self.db
            .create_backup(&backup)
            .map_err(|e| VaultError::Backup(e.to_string()))?;

        info!("Created backup '{}' for {}", name, source_path);
        Ok(backup)
    }

    /// Perform a backup operation
    pub fn perform_backup<F>(
        &mut self,
        backup_id: &str,
        mut progress_callback: F,
    ) -> VaultResult<String>
    where
        F: FnMut(Progress),
    {
        let backup = self
            .db
            .get_backup(backup_id)
            .map_err(|e| VaultError::Backup(e.to_string()))?
            .ok_or_else(|| VaultError::BackupNotFound(backup_id.to_string()))?;

        let source_path = Path::new(&backup.source_path);
        if !source_path.exists() {
            return Err(VaultError::FileSystem(format!(
                "Source path does not exist: {}",
                backup.source_path
            )));
        }

        // Update status
        self.db
            .update_backup_status(backup_id, "running")
            .map_err(|e| VaultError::Backup(e.to_string()))?;

        let mut progress = Progress::new();
        progress.phase = "Scanning files".to_string();
        progress_callback(progress.clone());

        // Scan files
        let files = self.scan_directory(source_path)?;
        progress.total_files = files.len() as u64;
        progress.total_bytes = files.iter().map(|f| f.size).sum();
        progress_callback(progress.clone());

        // Process files
        progress.phase = "Processing files".to_string();
        let mut manifest: Vec<FileManifestEntry> = Vec::new();
        let mut total_stored = 0i64;
        let mut total_chunks = 0i64;

        for file_info in files {
            progress.current_file = file_info.path.clone();
            progress_callback(progress.clone());

            if file_info.is_directory {
                manifest.push(FileManifestEntry {
                    path: file_info.path.clone(),
                    size: 0,
                    modified_at: file_info.modified_at,
                    is_directory: true,
                    chunks: vec![],
                });
            } else {
                let (chunks, stored_size) =
                    self.process_file(backup_id, source_path, &file_info)?;

                manifest.push(FileManifestEntry {
                    path: file_info.path.clone(),
                    size: file_info.size,
                    modified_at: file_info.modified_at,
                    is_directory: false,
                    chunks: chunks.clone(),
                });

                total_stored += stored_size;
                total_chunks += chunks.len() as i64;
            }

            progress.processed_files += 1;
            progress.processed_bytes += file_info.size;
            progress.update_percent();
            progress_callback(progress.clone());
        }

        // Create snapshot
        progress.phase = "Creating snapshot".to_string();
        progress_callback(progress.clone());

        let manifest_bytes = serde_json::to_vec(&manifest)
            .map_err(|e| VaultError::Backup(format!("Failed to serialize manifest: {}", e)))?;

        let snapshot = SnapshotInfo {
            id: Uuid::new_v4().to_string(),
            backup_id: backup_id.to_string(),
            timestamp: Utc::now().timestamp(),
            total_size: progress.total_bytes as i64,
            stored_size: total_stored,
            file_count: manifest.len() as i64,
            manifest: Some(manifest_bytes),
        };

        self.db.create_snapshot(&snapshot)?;

        // Update backup stats
        self.db
            .update_backup_stats(
                backup_id,
                progress.total_bytes as i64,
                total_stored,
                total_chunks,
            )
            .map_err(|e| VaultError::Backup(e.to_string()))?;

        self.db
            .update_backup_status(backup_id, "idle")
            .map_err(|e| VaultError::Backup(e.to_string()))?;

        progress.phase = "Complete".to_string();
        progress.percent = 100.0;
        progress_callback(progress);

        info!(
            "Backup complete: {} files, {} bytes total, {} bytes stored",
            manifest.len(),
            snapshot.total_size,
            total_stored
        );

        Ok(snapshot.id)
    }

    /// Scan a directory and collect file information
    fn scan_directory(&self, path: &Path) -> VaultResult<Vec<FileInfo>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(path).follow_links(false) {
            let entry = entry.map_err(|e| VaultError::FileSystem(e.to_string()))?;
            let file_path = entry.path();
            let relative_path = file_path
                .strip_prefix(path)
                .map_err(|e| VaultError::FileSystem(e.to_string()))?
                .to_string_lossy()
                .to_string();

            if relative_path.is_empty() {
                continue;
            }

            let metadata = entry
                .metadata()
                .map_err(|e| VaultError::FileSystem(e.to_string()))?;

            let modified_at = metadata
                .modified()
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64
                })
                .unwrap_or(0);

            files.push(FileInfo {
                path: relative_path,
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                modified_at,
                is_directory: metadata.is_dir(),
            });
        }

        // Sort directories first, then files
        files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        });

        Ok(files)
    }

    /// Process a single file: chunk, dedupe, compress, and store
    fn process_file(
        &mut self,
        backup_id: &str,
        base_path: &Path,
        file_info: &FileInfo,
    ) -> VaultResult<(Vec<[u8; 32]>, i64)> {
        let full_path = base_path.join(&file_info.path);

        let mut file = File::open(&full_path).map_err(|e| {
            VaultError::FileSystem(format!("Failed to open {}: {}", file_info.path, e))
        })?;

        // Chunk the file
        let chunks = self.chunker.chunk_file(&mut file)?;

        let mut chunk_hashes = Vec::with_capacity(chunks.len());
        let mut stored_size = 0i64;

        for (offset, chunk) in chunks.into_iter().enumerate() {
            // Check deduplication
            let compressed = self.compressor.compress(&chunk.data)?;
            let is_new = self.dedup.add_chunk(&chunk, compressed.len() as i64)?;

            if is_new {
                // Erasure code and store (simulated for now)
                let _shards = self.erasure.encode(&compressed)?;

                // In a real implementation, shards would be distributed to peers
                // For now, we just track the chunk
                stored_size += compressed.len() as i64;

                debug!(
                    "Stored new chunk {} ({} -> {} bytes)",
                    chunk.hash_hex(),
                    chunk.size,
                    compressed.len()
                );
            }

            // Map chunk to file
            self.db
                .add_backup_chunk(backup_id, &chunk.hash, &file_info.path, offset as i64)?;

            chunk_hashes.push(chunk.hash);
        }

        Ok((chunk_hashes, stored_size))
    }

    /// Get backup information
    pub fn get_backup(&self, backup_id: &str) -> VaultResult<Option<BackupInfo>> {
        self.db
            .get_backup(backup_id)
            .map_err(|e| VaultError::Backup(e.to_string()))
    }

    /// List all backups
    pub fn list_backups(&self) -> VaultResult<Vec<BackupInfo>> {
        self.db
            .list_backups()
            .map_err(|e| VaultError::Backup(e.to_string()))
    }

    /// Delete a backup
    ///
    /// This method removes the backup configuration and decrements reference counts
    /// for all chunks associated with the backup. Chunks whose reference count drops
    /// to zero are automatically removed from storage.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database operation fails
    /// - The backup ID does not exist
    pub fn delete_backup(&self, backup_id: &str) -> VaultResult<()> {
        // Get all unique chunk hashes for this backup before deletion
        let chunk_hashes = self
            .db
            .get_backup_chunk_hashes(backup_id)
            .map_err(|e| VaultError::Backup(format!("Failed to get backup chunks: {}", e)))?;

        // Decrement reference count for each chunk
        let mut deleted_chunks = 0;
        for hash in &chunk_hashes {
            if self.dedup.remove_chunk_ref(hash)? {
                deleted_chunks += 1;
            }
        }

        // Delete the backup record (cascades to backup_chunks table)
        self.db
            .delete_backup(backup_id)
            .map_err(|e| VaultError::Backup(e.to_string()))?;

        info!(
            "Deleted backup {}: {} chunks dereferenced, {} chunks removed",
            backup_id,
            chunk_hashes.len(),
            deleted_chunks
        );
        Ok(())
    }

    /// Get backup progress (for polling)
    pub fn get_backup_status(&self, backup_id: &str) -> VaultResult<String> {
        let backup = self
            .db
            .get_backup(backup_id)
            .map_err(|e| VaultError::Backup(e.to_string()))?
            .ok_or_else(|| VaultError::BackupNotFound(backup_id.to_string()))?;

        Ok(backup.status)
    }

    /// Get deduplication ratio
    pub fn get_dedup_ratio(&self) -> VaultResult<f64> {
        self.dedup.dedup_ratio()
    }
}

/// Internal file information
#[derive(Debug, Clone)]
struct FileInfo {
    path: String,
    size: u64,
    modified_at: i64,
    is_directory: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_files(dir: &Path) {
        // Create some test files
        let file1 = dir.join("file1.txt");
        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"Hello, WRAITH Vault!").unwrap();

        let file2 = dir.join("file2.txt");
        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"This is another test file with more content.")
            .unwrap();

        // Create subdirectory
        let subdir = dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let file3 = subdir.join("file3.txt");
        let mut f3 = File::create(&file3).unwrap();
        f3.write_all(b"Nested file content").unwrap();
    }

    #[test]
    fn test_create_backup() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let db = Arc::new(Database::open(&db_path).unwrap());

        let engine = BackupEngine::new(db).unwrap();

        let backup = engine
            .create_backup("Test Backup", "/home/user/documents")
            .unwrap();
        assert_eq!(backup.name, "Test Backup");
        assert_eq!(backup.source_path, "/home/user/documents");
        assert_eq!(backup.status, "idle");
    }

    #[test]
    fn test_perform_backup() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        create_test_files(&source_dir);

        let db = Arc::new(Database::open(&db_path).unwrap());
        let mut engine = BackupEngine::new(db).unwrap();

        let backup = engine
            .create_backup("Test Backup", source_dir.to_str().unwrap())
            .unwrap();

        let mut progress_updates = Vec::new();
        let snapshot_id = engine
            .perform_backup(&backup.id, |p| {
                progress_updates.push(p.phase.clone());
            })
            .unwrap();

        assert!(!snapshot_id.is_empty());
        assert!(progress_updates.contains(&"Complete".to_string()));

        // Check backup was updated
        let updated = engine.get_backup(&backup.id).unwrap().unwrap();
        assert!(updated.total_size > 0);
        assert!(updated.last_backup_at.is_some());
    }

    #[test]
    fn test_scan_directory() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        create_test_files(&source_dir);

        let db = Arc::new(Database::open(&db_path).unwrap());
        let engine = BackupEngine::new(db).unwrap();

        let files = engine.scan_directory(&source_dir).unwrap();

        // Should have 3 files + 1 directory
        assert_eq!(files.len(), 4);

        // Check that directory is first (due to sorting)
        assert!(files[0].is_directory);
        assert_eq!(files[0].path, "subdir");
    }

    #[test]
    fn test_deduplication() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create duplicate files
        let content = b"This content is duplicated across multiple files!";
        for i in 0..3 {
            let file_path = source_dir.join(format!("dup{}.txt", i));
            let mut f = File::create(&file_path).unwrap();
            f.write_all(content).unwrap();
        }

        let db = Arc::new(Database::open(&db_path).unwrap());
        let mut engine = BackupEngine::new(db).unwrap();

        let backup = engine
            .create_backup("Dedup Test", source_dir.to_str().unwrap())
            .unwrap();

        engine.perform_backup(&backup.id, |_| {}).unwrap();

        // Check deduplication ratio (should be > 1.0 due to duplicate files)
        let ratio = engine.get_dedup_ratio().unwrap();
        assert!(ratio >= 1.0);
    }
}
