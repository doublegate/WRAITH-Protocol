//! Core Sync Engine
//!
//! Orchestrates file synchronization between local file system and remote peers.

// Allow dead_code for fields used by future implementations
#![allow(dead_code)]
// Allow many arguments for complex conflict resolution
#![allow(clippy::too_many_arguments)]

use crate::database::{Database, FileMetadata, NewConflict, NewSyncFolder, QueueItem};
use crate::delta::{DeltaPatch, DeltaSync, FileSignature};
use crate::error::{SyncError, SyncResult};
use crate::watcher::{FileChange, FileChangeType};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::fs;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Sync engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEngineConfig {
    /// Debounce interval for file changes (milliseconds)
    pub debounce_ms: u64,
    /// Maximum concurrent sync operations
    pub max_concurrent_ops: usize,
    /// Bandwidth limit for uploads (bytes/sec, 0 = unlimited)
    pub upload_limit: u64,
    /// Bandwidth limit for downloads (bytes/sec, 0 = unlimited)
    pub download_limit: u64,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Maximum file versions to keep
    pub max_versions: i64,
    /// Version retention days
    pub version_retention_days: i64,
    /// Enable delta sync
    pub enable_delta_sync: bool,
    /// Minimum file size for delta sync (bytes)
    pub delta_sync_min_size: u64,
}

impl Default for SyncEngineConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            max_concurrent_ops: 4,
            upload_limit: 0,
            download_limit: 0,
            conflict_strategy: ConflictStrategy::LastWriterWins,
            max_versions: 10,
            version_retention_days: 30,
            enable_delta_sync: true,
            delta_sync_min_size: 10 * 1024, // 10 KB
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Most recent modification wins
    LastWriterWins,
    /// Keep both versions (create conflict file)
    KeepBoth,
    /// Prompt user for manual resolution
    Manual,
}

/// Sync operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    Upload {
        folder_id: i64,
        relative_path: String,
        size: u64,
    },
    Download {
        folder_id: i64,
        relative_path: String,
        peer_id: String,
    },
    Delete {
        folder_id: i64,
        relative_path: String,
    },
    Conflict {
        folder_id: i64,
        relative_path: String,
        local_hash: Vec<u8>,
        remote_hash: Vec<u8>,
        local_modified: i64,
        remote_modified: i64,
        remote_device_id: String,
    },
}

/// Sync status for a folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSyncStatus {
    pub folder_id: i64,
    pub local_path: String,
    pub status: SyncStatus,
    pub total_files: usize,
    pub synced_files: usize,
    pub pending_uploads: usize,
    pub pending_downloads: usize,
    pub unresolved_conflicts: usize,
    pub last_sync_at: Option<i64>,
    pub current_operation: Option<String>,
}

/// Overall sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Paused,
    Error,
    Offline,
}

/// Sync progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub operation: String,
    pub file_path: String,
    pub bytes_transferred: u64,
    pub bytes_total: u64,
    pub progress_percent: f32,
}

/// Remote file state from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileState {
    pub relative_path: String,
    pub hash: Vec<u8>,
    pub size: u64,
    pub modified_at: i64,
    pub is_directory: bool,
    pub device_id: String,
}

/// Core sync engine
pub struct SyncEngine {
    db: Arc<Database>,
    config: Arc<RwLock<SyncEngineConfig>>,
    delta_sync: DeltaSync,
    status: Arc<RwLock<SyncStatus>>,
    folder_statuses: Arc<RwLock<HashMap<i64, FolderSyncStatus>>>,
    progress_tx: Option<mpsc::Sender<SyncProgress>>,
    running: Arc<RwLock<bool>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(db: Arc<Database>, config: SyncEngineConfig) -> Self {
        Self {
            db,
            config: Arc::new(RwLock::new(config)),
            delta_sync: DeltaSync::new(),
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            folder_statuses: Arc::new(RwLock::new(HashMap::new())),
            progress_tx: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Set progress channel for UI updates
    pub fn set_progress_channel(&mut self, tx: mpsc::Sender<SyncProgress>) {
        self.progress_tx = Some(tx);
    }

    /// Get current sync status
    pub fn status(&self) -> SyncStatus {
        *self.status.read()
    }

    /// Get folder sync statuses
    pub fn folder_statuses(&self) -> Vec<FolderSyncStatus> {
        self.folder_statuses.read().values().cloned().collect()
    }

    /// Get status for a specific folder
    pub fn folder_status(&self, folder_id: i64) -> Option<FolderSyncStatus> {
        self.folder_statuses.read().get(&folder_id).cloned()
    }

    /// Add a folder to sync
    pub async fn add_folder(&self, local_path: &str, remote_path: &str) -> SyncResult<i64> {
        // Verify path exists
        let path = PathBuf::from(local_path);
        if !path.exists() {
            return Err(SyncError::FolderNotFound(local_path.to_string()));
        }

        if !path.is_dir() {
            return Err(SyncError::Config(format!(
                "{} is not a directory",
                local_path
            )));
        }

        let folder = NewSyncFolder {
            local_path: local_path.to_string(),
            remote_path: remote_path.to_string(),
            enabled: true,
        };

        let folder_id = self.db.add_sync_folder(&folder)?;

        // Initialize folder status
        let status = FolderSyncStatus {
            folder_id,
            local_path: local_path.to_string(),
            status: SyncStatus::Idle,
            total_files: 0,
            synced_files: 0,
            pending_uploads: 0,
            pending_downloads: 0,
            unresolved_conflicts: 0,
            last_sync_at: None,
            current_operation: None,
        };

        self.folder_statuses.write().insert(folder_id, status);

        // Perform initial scan
        self.scan_folder(folder_id).await?;

        info!("Added sync folder: {} -> {}", local_path, remote_path);
        Ok(folder_id)
    }

    /// Remove a folder from sync
    pub fn remove_folder(&self, folder_id: i64) -> SyncResult<()> {
        self.db.remove_sync_folder(folder_id)?;
        self.folder_statuses.write().remove(&folder_id);
        info!("Removed sync folder: {}", folder_id);
        Ok(())
    }

    /// Pause sync for a folder
    pub fn pause_folder(&self, folder_id: i64) -> SyncResult<()> {
        self.db.set_folder_paused(folder_id, true)?;

        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.status = SyncStatus::Paused;
        }

        info!("Paused sync for folder: {}", folder_id);
        Ok(())
    }

    /// Resume sync for a folder
    pub fn resume_folder(&self, folder_id: i64) -> SyncResult<()> {
        self.db.set_folder_paused(folder_id, false)?;

        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.status = SyncStatus::Idle;
        }

        info!("Resumed sync for folder: {}", folder_id);
        Ok(())
    }

    /// Scan a folder and update file metadata
    pub async fn scan_folder(&self, folder_id: i64) -> SyncResult<usize> {
        let folder = self
            .db
            .get_sync_folder(folder_id)?
            .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

        let ignored_patterns = self.db.get_ignored_patterns(Some(folder_id))?;
        let mut file_count = 0;

        self.scan_directory(
            &PathBuf::from(&folder.local_path),
            &PathBuf::from(&folder.local_path),
            folder_id,
            &ignored_patterns,
            &mut file_count,
        )
        .await?;

        // Update folder status
        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.total_files = file_count;
        }

        info!(
            "Scanned folder {} ({} files)",
            folder.local_path, file_count
        );
        Ok(file_count)
    }

    /// Recursively scan a directory
    async fn scan_directory(
        &self,
        base_path: &Path,
        current_path: &Path,
        folder_id: i64,
        ignored_patterns: &[String],
        file_count: &mut usize,
    ) -> SyncResult<()> {
        let mut entries = fs::read_dir(current_path)
            .await
            .map_err(|e| SyncError::FileSystem(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SyncError::FileSystem(format!("Failed to read entry: {}", e)))?
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|_| SyncError::FileSystem("Failed to get relative path".to_string()))?
                .to_string_lossy()
                .to_string();

            // Check if should be ignored
            if self.should_ignore(&relative_path, ignored_patterns) {
                debug!("Ignoring: {}", relative_path);
                continue;
            }

            let metadata = entry
                .metadata()
                .await
                .map_err(|e| SyncError::FileSystem(format!("Failed to get metadata: {}", e)))?;

            let modified_at = metadata
                .modified()
                .map_err(|e| SyncError::FileSystem(format!("Failed to get mtime: {}", e)))?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            if metadata.is_file() {
                // Calculate file hash
                let hash = self.compute_file_hash(&path).await?;

                let file_meta = FileMetadata {
                    id: 0,
                    folder_id,
                    relative_path: relative_path.clone(),
                    size: metadata.len() as i64,
                    modified_at,
                    hash,
                    is_directory: false,
                    synced: false,
                    deleted: false,
                    created_at: 0,
                };

                self.db.upsert_file_metadata(&file_meta)?;
                *file_count += 1;
            } else if metadata.is_dir() {
                // Record directory
                let dir_meta = FileMetadata {
                    id: 0,
                    folder_id,
                    relative_path: relative_path.clone(),
                    size: 0,
                    modified_at,
                    hash: vec![],
                    is_directory: true,
                    synced: false,
                    deleted: false,
                    created_at: 0,
                };

                self.db.upsert_file_metadata(&dir_meta)?;

                // Recurse
                Box::pin(self.scan_directory(
                    base_path,
                    &path,
                    folder_id,
                    ignored_patterns,
                    file_count,
                ))
                .await?;
            }
        }

        Ok(())
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, relative_path: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            if self.matches_glob(relative_path, pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob matching
    fn matches_glob(&self, path: &str, pattern: &str) -> bool {
        // Handle **/ prefix and /** suffix patterns (e.g., **/.git/**)
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                // Both empty means match anything
                if prefix.is_empty() && suffix.is_empty() {
                    return true;
                }

                // Handle suffix with single wildcard (e.g., **/*.tmp)
                if !suffix.is_empty() && suffix.contains('*') && prefix.is_empty() {
                    // Pattern like **/*.tmp - the suffix is *.tmp
                    let ext_parts: Vec<&str> = suffix.split('*').collect();
                    if ext_parts.len() == 2 && ext_parts[0].is_empty() {
                        // Suffix is *.ext - check if filename ends with .ext
                        let ext = ext_parts[1];
                        // Get filename from path
                        let filename = path.rsplit('/').next().unwrap_or(path);
                        return filename.ends_with(ext);
                    }
                }

                // Check if the middle part exists in the path
                // For **/.git/** we want to match paths containing .git/
                if !prefix.is_empty() && !suffix.is_empty() {
                    // Pattern like "foo/**/bar" - path must contain both
                    return path.contains(prefix) && path.contains(suffix);
                }

                if !prefix.is_empty() {
                    // Pattern starts with something before **
                    return path.starts_with(prefix) || path.contains(&format!("/{}", prefix));
                }

                if !suffix.is_empty() {
                    // Pattern like **/.git/** - check if path starts with suffix or contains /suffix
                    return path.starts_with(suffix)
                        || path.starts_with(&format!("{}/", suffix))
                        || path.contains(&format!("/{}/", suffix))
                        || path.contains(&format!("/{}", suffix));
                }
            } else if parts.len() == 3 {
                // Pattern like **/foo/** - middle part must be in path
                let middle = parts[1].trim_matches('/');
                if !middle.is_empty() {
                    return path.starts_with(&format!("{}/", middle))
                        || path.contains(&format!("/{}/", middle))
                        || path.contains(&format!("/{}", middle))
                        || path == middle;
                }
            }
        }

        // Handle single * wildcards
        if pattern.contains('*') && !pattern.contains("**") {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];

                if prefix.is_empty() {
                    return path.ends_with(suffix);
                }
                if suffix.is_empty() {
                    return path.starts_with(prefix);
                }
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }

        // Exact match or simple contains
        path == pattern || path.contains(pattern)
    }

    /// Compute BLAKE3 hash for a file
    async fn compute_file_hash(&self, path: &Path) -> SyncResult<Vec<u8>> {
        let data = fs::read(path).await.map_err(|e| {
            SyncError::FileSystem(format!("Failed to read file for hashing: {}", e))
        })?;

        let hash = blake3::hash(&data);
        Ok(hash.as_bytes().to_vec())
    }

    /// Process a file change event
    pub async fn process_change(&self, folder_id: i64, change: FileChange) -> SyncResult<()> {
        let folder = self
            .db
            .get_sync_folder(folder_id)?
            .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

        let base_path = PathBuf::from(&folder.local_path);
        let relative_path = change
            .path
            .strip_prefix(&base_path)
            .map_err(|_| SyncError::FileSystem("Failed to get relative path".to_string()))?
            .to_string_lossy()
            .to_string();

        match change.change_type {
            FileChangeType::Created | FileChangeType::Modified => {
                if change.path.is_file() {
                    let metadata = fs::metadata(&change.path).await?;
                    let hash = self.compute_file_hash(&change.path).await?;
                    let modified_at = metadata
                        .modified()?
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    let file_meta = FileMetadata {
                        id: 0,
                        folder_id,
                        relative_path: relative_path.clone(),
                        size: metadata.len() as i64,
                        modified_at,
                        hash,
                        is_directory: false,
                        synced: false,
                        deleted: false,
                        created_at: 0,
                    };

                    self.db.upsert_file_metadata(&file_meta)?;

                    // Add to upload queue
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path,
                        operation: "upload".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
            }
            FileChangeType::Deleted => {
                if let Some(meta) = self.db.get_file_metadata(folder_id, &relative_path)? {
                    self.db.mark_file_deleted(meta.id)?;

                    // Add delete to queue
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path,
                        operation: "delete".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
            }
            FileChangeType::Renamed { from, to } => {
                // Handle as delete + create
                let from_relative = from
                    .strip_prefix(&base_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .ok();

                if let Some(from_rel) = from_relative {
                    if let Some(meta) = self.db.get_file_metadata(folder_id, &from_rel)? {
                        self.db.mark_file_deleted(meta.id)?;
                    }
                }

                // Process "to" as new file
                if to.is_file() {
                    let to_relative = to
                        .strip_prefix(&base_path)
                        .map_err(|_| {
                            SyncError::FileSystem("Failed to get relative path".to_string())
                        })?
                        .to_string_lossy()
                        .to_string();

                    let metadata = fs::metadata(&to).await?;
                    let hash = self.compute_file_hash(&to).await?;
                    let modified_at = metadata
                        .modified()?
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    let file_meta = FileMetadata {
                        id: 0,
                        folder_id,
                        relative_path: to_relative.clone(),
                        size: metadata.len() as i64,
                        modified_at,
                        hash,
                        is_directory: false,
                        synced: false,
                        deleted: false,
                        created_at: 0,
                    };

                    self.db.upsert_file_metadata(&file_meta)?;

                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path: to_relative,
                        operation: "upload".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
            }
        }

        Ok(())
    }

    /// Sync a folder with remote peers
    pub async fn sync_folder(
        &self,
        folder_id: i64,
        remote_states: &[RemoteFileState],
    ) -> SyncResult<()> {
        let folder = self
            .db
            .get_sync_folder(folder_id)?
            .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

        if folder.paused {
            debug!("Folder {} is paused, skipping sync", folder_id);
            return Ok(());
        }

        // Update status
        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.status = SyncStatus::Syncing;
        }

        let local_files = self.db.list_folder_files(folder_id)?;
        let config = self.config.read().clone();

        // Build remote file map
        let remote_map: HashMap<String, &RemoteFileState> = remote_states
            .iter()
            .map(|s| (s.relative_path.clone(), s))
            .collect();

        let mut operations = Vec::new();

        // Check local files against remote
        for local_file in &local_files {
            if local_file.is_directory {
                continue;
            }

            if let Some(remote) = remote_map.get(&local_file.relative_path) {
                // File exists on both sides
                if local_file.hash != remote.hash {
                    // Content differs - determine action
                    match local_file.modified_at.cmp(&remote.modified_at) {
                        std::cmp::Ordering::Greater => {
                            // Local is newer - upload
                            operations.push(SyncOperation::Upload {
                                folder_id,
                                relative_path: local_file.relative_path.clone(),
                                size: local_file.size as u64,
                            });
                        }
                        std::cmp::Ordering::Less => {
                            // Remote is newer - download
                            operations.push(SyncOperation::Download {
                                folder_id,
                                relative_path: local_file.relative_path.clone(),
                                peer_id: remote.device_id.clone(),
                            });
                        }
                        std::cmp::Ordering::Equal => {
                            // Same timestamp but different content - conflict
                            operations.push(SyncOperation::Conflict {
                                folder_id,
                                relative_path: local_file.relative_path.clone(),
                                local_hash: local_file.hash.clone(),
                                remote_hash: remote.hash.clone(),
                                local_modified: local_file.modified_at,
                                remote_modified: remote.modified_at,
                                remote_device_id: remote.device_id.clone(),
                            });
                        }
                    }
                }
            } else {
                // File only exists locally - upload
                operations.push(SyncOperation::Upload {
                    folder_id,
                    relative_path: local_file.relative_path.clone(),
                    size: local_file.size as u64,
                });
            }
        }

        // Check for files that exist only on remote
        let local_paths: std::collections::HashSet<String> = local_files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect();

        for remote in remote_states {
            if !local_paths.contains(&remote.relative_path) && !remote.is_directory {
                operations.push(SyncOperation::Download {
                    folder_id,
                    relative_path: remote.relative_path.clone(),
                    peer_id: remote.device_id.clone(),
                });
            }
        }

        // Process operations
        for op in operations {
            match op {
                SyncOperation::Upload {
                    folder_id,
                    relative_path,
                    ..
                } => {
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path,
                        operation: "upload".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
                SyncOperation::Download {
                    folder_id,
                    relative_path,
                    ..
                } => {
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path,
                        operation: "download".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
                SyncOperation::Delete {
                    folder_id,
                    relative_path,
                } => {
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path,
                        operation: "delete".to_string(),
                        priority: 0,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
                SyncOperation::Conflict {
                    folder_id,
                    relative_path,
                    local_hash,
                    remote_hash,
                    local_modified,
                    remote_modified,
                    remote_device_id,
                } => {
                    self.handle_conflict(
                        folder_id,
                        &relative_path,
                        &local_hash,
                        &remote_hash,
                        local_modified,
                        remote_modified,
                        &remote_device_id,
                        &config,
                    )?;
                }
            }
        }

        // Update last sync time
        self.db.update_last_sync(folder_id)?;

        // Update status
        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.status = SyncStatus::Idle;
            status.last_sync_at = Some(chrono::Utc::now().timestamp());
        }

        info!("Completed sync for folder {}", folder_id);
        Ok(())
    }

    /// Handle a file conflict
    fn handle_conflict(
        &self,
        folder_id: i64,
        relative_path: &str,
        local_hash: &[u8],
        remote_hash: &[u8],
        local_modified: i64,
        remote_modified: i64,
        remote_device_id: &str,
        config: &SyncEngineConfig,
    ) -> SyncResult<()> {
        let file_meta = self.db.get_file_metadata(folder_id, relative_path)?;
        let file_id = file_meta.map(|m| m.id).unwrap_or(0);

        match config.conflict_strategy {
            ConflictStrategy::LastWriterWins => {
                if local_modified > remote_modified {
                    // Local wins - upload
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path: relative_path.to_string(),
                        operation: "upload".to_string(),
                        priority: 1,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                } else {
                    // Remote wins - download
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path: relative_path.to_string(),
                        operation: "download".to_string(),
                        priority: 1,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
            }
            ConflictStrategy::KeepBoth | ConflictStrategy::Manual => {
                // Create conflict record
                let conflict = NewConflict {
                    file_id,
                    local_hash: local_hash.to_vec(),
                    remote_hash: remote_hash.to_vec(),
                    local_modified_at: local_modified,
                    remote_modified_at: remote_modified,
                    remote_device_id: remote_device_id.to_string(),
                };
                self.db.create_conflict(&conflict)?;

                if config.conflict_strategy == ConflictStrategy::KeepBoth {
                    // Rename local file to conflict version
                    // Download remote version to original path
                    let queue_item = QueueItem {
                        id: 0,
                        folder_id,
                        relative_path: relative_path.to_string(),
                        operation: "download".to_string(),
                        priority: 1,
                        retries: 0,
                        last_attempt: None,
                        error_message: None,
                        created_at: 0,
                    };
                    self.db.add_to_queue(&queue_item)?;
                }
            }
        }

        Ok(())
    }

    /// Get delta signature for a file
    pub fn get_file_signature(&self, path: &Path) -> SyncResult<FileSignature> {
        self.delta_sync.generate_signature(path)
    }

    /// Compute delta for a file
    pub fn compute_file_delta(
        &self,
        local_path: &Path,
        remote_signature: &FileSignature,
    ) -> SyncResult<DeltaPatch> {
        self.delta_sync.compute_delta(local_path, remote_signature)
    }

    /// Apply delta to reconstruct file
    pub fn apply_file_delta(
        &self,
        base_path: &Path,
        delta: &DeltaPatch,
        output_path: &Path,
    ) -> SyncResult<()> {
        self.delta_sync.apply_delta(base_path, delta, output_path)
    }

    /// Update configuration
    pub fn update_config(&self, config: SyncEngineConfig) {
        *self.config.write() = config;
    }

    /// Get current configuration
    pub fn config(&self) -> SyncEngineConfig {
        self.config.read().clone()
    }

    /// Get queue size
    pub fn queue_size(&self) -> SyncResult<i64> {
        Ok(self.db.queue_size()?)
    }

    /// Get unresolved conflict count
    pub fn conflict_count(&self) -> SyncResult<i64> {
        Ok(self.db.count_unresolved_conflicts()?)
    }

    /// Set folder status (used by AppState)
    pub fn set_folder_status(&mut self, folder_id: i64, status: FolderSyncStatus) {
        self.folder_statuses.write().insert(folder_id, status);
    }

    /// Update folder file count (used by AppState)
    pub fn update_folder_file_count(&mut self, folder_id: i64, file_count: usize) {
        if let Some(status) = self.folder_statuses.write().get_mut(&folder_id) {
            status.total_files = file_count;
        }
    }

    /// Get database reference
    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }

    /// Get delta sync reference
    pub fn delta_sync(&self) -> DeltaSync {
        self.delta_sync.clone()
    }

    /// Get folder statuses arc
    pub fn folder_statuses_arc(&self) -> Arc<RwLock<HashMap<i64, FolderSyncStatus>>> {
        self.folder_statuses.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_engine() -> (SyncEngine, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let engine = SyncEngine::new(db, SyncEngineConfig::default());
        (engine, dir)
    }

    #[tokio::test]
    async fn test_add_folder() {
        let (engine, dir) = create_test_engine();
        let folder_path = dir.path().to_str().unwrap();

        let folder_id = engine.add_folder(folder_path, "/sync").await.unwrap();
        assert!(folder_id > 0);

        let status = engine.folder_status(folder_id);
        assert!(status.is_some());
    }

    #[test]
    fn test_glob_matching() {
        let (engine, _dir) = create_test_engine();

        assert!(engine.matches_glob(".git/config", "**/.git/**"));
        assert!(engine.matches_glob("node_modules/foo", "**/node_modules/**"));
        assert!(engine.matches_glob("file.tmp", "**/*.tmp"));
        assert!(!engine.matches_glob("file.txt", "**/*.tmp"));
    }

    #[test]
    fn test_conflict_strategy() {
        let config = SyncEngineConfig::default();
        assert_eq!(config.conflict_strategy, ConflictStrategy::LastWriterWins);
    }
}
