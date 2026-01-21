//! Application State Management
//!
//! Manages shared state across Tauri commands.

use crate::config::{AppSettings, ConfigManager};
use crate::database::Database;
use crate::error::SyncResult;
use crate::sync_engine::{FolderSyncStatus, SyncEngine, SyncEngineConfig, SyncStatus};
use crate::watcher::{FileSystemWatcher, WatcherConfig};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use wraith_core::node::Node;

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// Sync engine
    pub sync_engine: Arc<RwLock<SyncEngine>>,
    /// File system watcher
    pub watcher: Arc<RwLock<Option<FileSystemWatcher>>>,
    /// Configuration manager
    pub config_manager: Arc<ConfigManager>,
    /// WRAITH node (optional, for network sync)
    pub node: Arc<RwLock<Option<Node>>>,
    /// Whether sync is globally paused
    pub paused: Arc<RwLock<bool>>,
    /// Application data directory
    pub app_data_dir: PathBuf,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database, app_data_dir: PathBuf) -> Self {
        let db = Arc::new(db);
        let config_manager = Arc::new(ConfigManager::new(db.clone()));

        // Load settings and create sync engine config
        let settings = config_manager.load_settings().unwrap_or_default();
        let engine_config = SyncEngineConfig {
            debounce_ms: settings.debounce_ms,
            max_concurrent_ops: 4,
            upload_limit: settings.upload_limit,
            download_limit: settings.download_limit,
            conflict_strategy: settings.parse_conflict_strategy(),
            max_versions: settings.max_versions,
            version_retention_days: settings.version_retention_days,
            enable_delta_sync: settings.enable_delta_sync,
            delta_sync_min_size: 10 * 1024,
        };

        let sync_engine = Arc::new(RwLock::new(SyncEngine::new(db.clone(), engine_config)));

        Self {
            db,
            sync_engine,
            watcher: Arc::new(RwLock::new(None)),
            config_manager,
            node: Arc::new(RwLock::new(None)),
            paused: Arc::new(RwLock::new(false)),
            app_data_dir,
        }
    }

    /// Initialize the file system watcher
    pub fn init_watcher(&self) -> SyncResult<()> {
        let settings = self.config_manager.load_settings()?;
        let ignored_patterns = self.db.get_ignored_patterns(None)?;

        let config = WatcherConfig {
            debounce_ms: settings.debounce_ms,
            ignored_patterns,
            max_pending_events: 1000,
        };

        let watcher = FileSystemWatcher::new(config)?;
        *self.watcher.write() = Some(watcher);

        info!("File system watcher initialized");
        Ok(())
    }

    /// Start watching all enabled folders
    pub fn start_watching(&self) -> SyncResult<()> {
        let folders = self.db.list_sync_folders()?;
        let mut watcher_guard = self.watcher.write();

        if let Some(watcher) = watcher_guard.as_mut() {
            for folder in folders {
                if folder.enabled && !folder.paused {
                    watcher.watch_path(&PathBuf::from(&folder.local_path))?;
                    info!("Started watching: {}", folder.local_path);
                }
            }
        }

        Ok(())
    }

    /// Stop watching all folders
    pub fn stop_watching(&self) -> SyncResult<()> {
        let folders = self.db.list_sync_folders()?;
        let mut watcher_guard = self.watcher.write();

        if let Some(watcher) = watcher_guard.as_mut() {
            for folder in folders {
                let path = PathBuf::from(&folder.local_path);
                if watcher.is_watching(&path) {
                    watcher.unwatch_path(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Get global sync status
    pub fn get_sync_status(&self) -> SyncStatus {
        if *self.paused.read() {
            SyncStatus::Paused
        } else {
            self.sync_engine.read().status()
        }
    }

    /// Get all folder statuses
    pub fn get_folder_statuses(&self) -> Vec<FolderSyncStatus> {
        self.sync_engine.read().folder_statuses()
    }

    /// Pause global sync
    pub fn pause_sync(&self) {
        *self.paused.write() = true;
        info!("Global sync paused");
    }

    /// Resume global sync
    pub fn resume_sync(&self) {
        *self.paused.write() = false;
        info!("Global sync resumed");
    }

    /// Check if sync is paused
    pub fn is_paused(&self) -> bool {
        *self.paused.read()
    }

    /// Get application settings
    pub fn get_settings(&self) -> SyncResult<AppSettings> {
        self.config_manager.load_settings()
    }

    /// Update application settings
    pub fn update_settings(&self, settings: &AppSettings) -> SyncResult<()> {
        self.config_manager.save_settings(settings)?;

        // Update sync engine config
        let engine_config = SyncEngineConfig {
            debounce_ms: settings.debounce_ms,
            max_concurrent_ops: 4,
            upload_limit: settings.upload_limit,
            download_limit: settings.download_limit,
            conflict_strategy: settings.parse_conflict_strategy(),
            max_versions: settings.max_versions,
            version_retention_days: settings.version_retention_days,
            enable_delta_sync: settings.enable_delta_sync,
            delta_sync_min_size: 10 * 1024,
        };

        self.sync_engine.write().update_config(engine_config);

        info!("Settings updated");
        Ok(())
    }

    /// Get queue size
    pub fn queue_size(&self) -> SyncResult<i64> {
        self.sync_engine.read().queue_size()
    }

    /// Get unresolved conflict count
    pub fn conflict_count(&self) -> SyncResult<i64> {
        self.sync_engine.read().conflict_count()
    }

    /// Get version storage path
    pub fn version_storage_path(&self) -> PathBuf {
        self.app_data_dir.join("versions")
    }

    /// Add a folder to sync (async-safe wrapper)
    pub async fn add_folder_async(&self, local_path: &str, remote_path: &str) -> SyncResult<i64> {
        use crate::database::NewSyncFolder;
        use crate::sync_engine::FolderSyncStatus;

        // Verify path exists
        let path = PathBuf::from(local_path);
        if !path.exists() {
            return Err(crate::error::SyncError::FolderNotFound(
                local_path.to_string(),
            ));
        }

        if !path.is_dir() {
            return Err(crate::error::SyncError::Config(format!(
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
        {
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

            self.sync_engine
                .write()
                .set_folder_status(folder_id, status);
        }

        // Perform initial scan
        let ignored_patterns = self.db.get_ignored_patterns(Some(folder_id))?;
        self.scan_folder_async(folder_id, local_path, &ignored_patterns)
            .await?;

        info!("Added sync folder: {} -> {}", local_path, remote_path);
        Ok(folder_id)
    }

    /// Scan a folder asynchronously without holding locks across await
    pub async fn scan_folder_async(
        &self,
        folder_id: i64,
        base_path: &str,
        ignored_patterns: &[String],
    ) -> SyncResult<usize> {
        let base = PathBuf::from(base_path);
        let mut file_count = 0;

        self.scan_directory_async(&base, &base, folder_id, ignored_patterns, &mut file_count)
            .await?;

        // Update folder status
        self.sync_engine
            .write()
            .update_folder_file_count(folder_id, file_count);

        info!("Scanned folder {} ({} files)", base_path, file_count);
        Ok(file_count)
    }

    /// Recursively scan a directory
    async fn scan_directory_async(
        &self,
        base_path: &std::path::Path,
        current_path: &std::path::Path,
        folder_id: i64,
        ignored_patterns: &[String],
        file_count: &mut usize,
    ) -> SyncResult<()> {
        use crate::database::FileMetadata;
        use std::time::UNIX_EPOCH;
        use tokio::fs;

        let mut entries = fs::read_dir(current_path).await.map_err(|e| {
            crate::error::SyncError::FileSystem(format!("Failed to read directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            crate::error::SyncError::FileSystem(format!("Failed to read entry: {}", e))
        })? {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|_| {
                    crate::error::SyncError::FileSystem("Failed to get relative path".to_string())
                })?
                .to_string_lossy()
                .to_string();

            // Check if should be ignored
            if self.should_ignore(&relative_path, ignored_patterns) {
                continue;
            }

            let metadata = entry.metadata().await.map_err(|e| {
                crate::error::SyncError::FileSystem(format!("Failed to get metadata: {}", e))
            })?;

            let modified_at = metadata
                .modified()
                .map_err(|e| {
                    crate::error::SyncError::FileSystem(format!("Failed to get mtime: {}", e))
                })?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            if metadata.is_file() {
                // Calculate file hash
                let hash = self.compute_file_hash_async(&path).await?;

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
                Box::pin(self.scan_directory_async(
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
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');
                let has_prefix = prefix.is_empty() || path.contains(prefix);
                let has_suffix = suffix.is_empty() || path.contains(suffix);
                return has_prefix && has_suffix;
            }
        }

        if pattern.contains('*') && !pattern.contains("**") {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return path.starts_with(parts[0]) && path.ends_with(parts[1]);
            }
        }

        path.contains(pattern)
    }

    /// Compute BLAKE3 hash for a file
    async fn compute_file_hash_async(&self, path: &std::path::Path) -> SyncResult<Vec<u8>> {
        use tokio::fs;

        let data = fs::read(path).await.map_err(|e| {
            crate::error::SyncError::FileSystem(format!("Failed to read file for hashing: {}", e))
        })?;

        let hash = blake3::hash(&data);
        Ok(hash.as_bytes().to_vec())
    }
}
