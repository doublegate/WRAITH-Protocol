//! Tauri IPC Commands for WRAITH Sync
//!
//! Provides the command interface between the frontend and backend.

use crate::config::AppSettings;
use crate::error::SyncError;
use crate::state::AppState;
use crate::sync_engine::{FolderSyncStatus, SyncStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tracing::info;

/// Application result type for Tauri commands
type CmdResult<T> = Result<T, SyncError>;

// ============================================================================
// Status Types
// ============================================================================

/// Overall sync status for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStatus {
    pub status: String,
    pub total_folders: usize,
    pub syncing_folders: usize,
    pub total_files: usize,
    pub pending_operations: i64,
    pub unresolved_conflicts: i64,
    pub is_paused: bool,
}

/// Folder info for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInfo {
    pub id: i64,
    pub local_path: String,
    pub remote_path: String,
    pub enabled: bool,
    pub paused: bool,
    pub status: String,
    pub total_files: usize,
    pub synced_files: usize,
    pub pending_operations: usize,
    pub last_sync_at: Option<i64>,
}

/// File info for version history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub relative_path: String,
    pub size: i64,
    pub modified_at: i64,
    pub synced: bool,
    pub versions: Vec<VersionInfo>,
}

/// Version info for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: i64,
    pub version_number: i64,
    pub size: i64,
    pub modified_at: i64,
    pub created_at: i64,
}

/// Conflict info for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub id: i64,
    pub file_path: String,
    pub folder_path: String,
    pub local_modified_at: i64,
    pub remote_modified_at: i64,
    pub remote_device: String,
    pub created_at: i64,
}

/// Device info for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: i64,
    pub device_id: String,
    pub device_name: String,
    pub last_seen: i64,
    pub is_self: bool,
}

// ============================================================================
// Status Commands
// ============================================================================

/// Get overall sync status
#[tauri::command]
pub async fn get_status(state: State<'_, Arc<AppState>>) -> CmdResult<OverallStatus> {
    let sync_status = state.get_sync_status();
    let folder_statuses = state.get_folder_statuses();
    let pending = state.queue_size().unwrap_or(0);
    let conflicts = state.conflict_count().unwrap_or(0);

    let status_str = match sync_status {
        SyncStatus::Idle => "idle",
        SyncStatus::Syncing => "syncing",
        SyncStatus::Paused => "paused",
        SyncStatus::Error => "error",
        SyncStatus::Offline => "offline",
    };

    let syncing_count = folder_statuses
        .iter()
        .filter(|s| s.status == SyncStatus::Syncing)
        .count();

    let total_files: usize = folder_statuses.iter().map(|s| s.total_files).sum();

    Ok(OverallStatus {
        status: status_str.to_string(),
        total_folders: folder_statuses.len(),
        syncing_folders: syncing_count,
        total_files,
        pending_operations: pending,
        unresolved_conflicts: conflicts,
        is_paused: state.is_paused(),
    })
}

/// Pause global sync
#[tauri::command]
pub async fn pause_sync(state: State<'_, Arc<AppState>>) -> CmdResult<()> {
    state.pause_sync();
    state.stop_watching()?;
    info!("Sync paused by user");
    Ok(())
}

/// Resume global sync
#[tauri::command]
pub async fn resume_sync(state: State<'_, Arc<AppState>>) -> CmdResult<()> {
    state.resume_sync();
    state.start_watching()?;
    info!("Sync resumed by user");
    Ok(())
}

// ============================================================================
// Folder Commands
// ============================================================================

/// Add a new sync folder
#[tauri::command]
pub async fn add_folder(
    state: State<'_, Arc<AppState>>,
    local_path: String,
    remote_path: String,
) -> CmdResult<FolderInfo> {
    // Use the async-safe wrapper
    let folder_id = state.add_folder_async(&local_path, &remote_path).await?;

    // Start watching the new folder
    if let Some(watcher) = state.watcher.write().as_mut() {
        watcher.watch_path(&PathBuf::from(&local_path))?;
    }

    let folder = state
        .db
        .get_sync_folder(folder_id)?
        .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

    let status = state.sync_engine.read().folder_status(folder_id);

    info!("Added folder: {} -> {}", local_path, remote_path);

    Ok(FolderInfo {
        id: folder.id,
        local_path: folder.local_path,
        remote_path: folder.remote_path,
        enabled: folder.enabled,
        paused: folder.paused,
        status: "idle".to_string(),
        total_files: status.as_ref().map(|s| s.total_files).unwrap_or(0),
        synced_files: status.as_ref().map(|s| s.synced_files).unwrap_or(0),
        pending_operations: status
            .as_ref()
            .map(|s| s.pending_uploads + s.pending_downloads)
            .unwrap_or(0),
        last_sync_at: folder.last_sync_at,
    })
}

/// Remove a sync folder
#[tauri::command]
pub async fn remove_folder(state: State<'_, Arc<AppState>>, folder_id: i64) -> CmdResult<()> {
    // Stop watching first
    if let Some(folder) = state.db.get_sync_folder(folder_id)?
        && let Some(watcher) = state.watcher.write().as_mut()
    {
        let _ = watcher.unwatch_path(&PathBuf::from(&folder.local_path));
    }

    state.sync_engine.read().remove_folder(folder_id)?;
    info!("Removed folder: {}", folder_id);
    Ok(())
}

/// List all sync folders
#[tauri::command]
pub async fn list_folders(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<FolderInfo>> {
    let folders = state.db.list_sync_folders()?;
    let statuses = state.get_folder_statuses();

    let status_map: std::collections::HashMap<i64, &FolderSyncStatus> =
        statuses.iter().map(|s| (s.folder_id, s)).collect();

    let infos = folders
        .into_iter()
        .map(|folder| {
            let status = status_map.get(&folder.id);
            let status_str = status
                .map(|s| match s.status {
                    SyncStatus::Idle => "idle",
                    SyncStatus::Syncing => "syncing",
                    SyncStatus::Paused => "paused",
                    SyncStatus::Error => "error",
                    SyncStatus::Offline => "offline",
                })
                .unwrap_or("idle");

            FolderInfo {
                id: folder.id,
                local_path: folder.local_path,
                remote_path: folder.remote_path,
                enabled: folder.enabled,
                paused: folder.paused,
                status: status_str.to_string(),
                total_files: status.map(|s| s.total_files).unwrap_or(0),
                synced_files: status.map(|s| s.synced_files).unwrap_or(0),
                pending_operations: status
                    .map(|s| s.pending_uploads + s.pending_downloads)
                    .unwrap_or(0),
                last_sync_at: folder.last_sync_at,
            }
        })
        .collect();

    Ok(infos)
}

/// Get folder details
#[tauri::command]
pub async fn get_folder(state: State<'_, Arc<AppState>>, folder_id: i64) -> CmdResult<FolderInfo> {
    let folder = state
        .db
        .get_sync_folder(folder_id)?
        .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

    let status = state.sync_engine.read().folder_status(folder_id);
    let status_str = status
        .as_ref()
        .map(|s| match s.status {
            SyncStatus::Idle => "idle",
            SyncStatus::Syncing => "syncing",
            SyncStatus::Paused => "paused",
            SyncStatus::Error => "error",
            SyncStatus::Offline => "offline",
        })
        .unwrap_or("idle");

    Ok(FolderInfo {
        id: folder.id,
        local_path: folder.local_path,
        remote_path: folder.remote_path,
        enabled: folder.enabled,
        paused: folder.paused,
        status: status_str.to_string(),
        total_files: status.as_ref().map(|s| s.total_files).unwrap_or(0),
        synced_files: status.as_ref().map(|s| s.synced_files).unwrap_or(0),
        pending_operations: status
            .as_ref()
            .map(|s| s.pending_uploads + s.pending_downloads)
            .unwrap_or(0),
        last_sync_at: folder.last_sync_at,
    })
}

/// Pause sync for a folder
#[tauri::command]
pub async fn pause_folder(state: State<'_, Arc<AppState>>, folder_id: i64) -> CmdResult<()> {
    state.sync_engine.read().pause_folder(folder_id)?;

    // Stop watching this folder
    if let Some(folder) = state.db.get_sync_folder(folder_id)?
        && let Some(watcher) = state.watcher.write().as_mut()
    {
        let _ = watcher.unwatch_path(&PathBuf::from(&folder.local_path));
    }

    info!("Paused folder: {}", folder_id);
    Ok(())
}

/// Resume sync for a folder
#[tauri::command]
pub async fn resume_folder(state: State<'_, Arc<AppState>>, folder_id: i64) -> CmdResult<()> {
    state.sync_engine.read().resume_folder(folder_id)?;

    // Resume watching this folder
    if let Some(folder) = state.db.get_sync_folder(folder_id)?
        && let Some(watcher) = state.watcher.write().as_mut()
    {
        watcher.watch_path(&PathBuf::from(&folder.local_path))?;
    }

    info!("Resumed folder: {}", folder_id);
    Ok(())
}

/// Force sync a folder
#[tauri::command]
pub async fn force_sync_folder(state: State<'_, Arc<AppState>>, folder_id: i64) -> CmdResult<()> {
    // Get folder info without holding locks
    let folder = state
        .db
        .get_sync_folder(folder_id)?
        .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

    let ignored_patterns = state.db.get_ignored_patterns(Some(folder_id))?;

    // Scan using state's async-safe helper method
    state
        .scan_folder_async(folder_id, &folder.local_path, &ignored_patterns)
        .await?;

    info!("Force synced folder: {}", folder_id);
    Ok(())
}

// ============================================================================
// Conflict Commands
// ============================================================================

/// List unresolved conflicts
#[tauri::command]
pub async fn list_conflicts(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<ConflictInfo>> {
    let conflicts = state.db.list_unresolved_conflicts()?;

    let infos = conflicts
        .into_iter()
        .map(|c| ConflictInfo {
            id: c.id,
            file_path: c.relative_path,
            folder_path: c.folder_path,
            local_modified_at: c.local_modified_at,
            remote_modified_at: c.remote_modified_at,
            remote_device: c.remote_device_id,
            created_at: c.created_at,
        })
        .collect();

    Ok(infos)
}

/// Resolve a conflict
#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, Arc<AppState>>,
    conflict_id: i64,
    resolution: String, // "local", "remote", "keep_both"
) -> CmdResult<()> {
    // Validate resolution
    let valid_resolutions = ["local", "remote", "keep_both"];
    if !valid_resolutions.contains(&resolution.as_str()) {
        return Err(SyncError::Conflict(format!(
            "Invalid resolution: {}. Must be one of: {:?}",
            resolution, valid_resolutions
        )));
    }

    state.db.resolve_conflict(conflict_id, &resolution)?;
    info!(
        "Resolved conflict {} with strategy: {}",
        conflict_id, resolution
    );
    Ok(())
}

// ============================================================================
// Version History Commands
// ============================================================================

/// Get version history for a file
#[tauri::command]
pub async fn get_file_versions(
    state: State<'_, Arc<AppState>>,
    folder_id: i64,
    relative_path: String,
) -> CmdResult<Vec<VersionInfo>> {
    let file_meta = state.db.get_file_metadata(folder_id, &relative_path)?;

    let file_id = match file_meta {
        Some(meta) => meta.id,
        None => return Err(SyncError::FileNotFound(relative_path)),
    };

    let versions = state.db.get_file_versions(file_id)?;

    let infos = versions
        .into_iter()
        .map(|v| VersionInfo {
            id: v.id,
            version_number: v.version_number,
            size: v.size,
            modified_at: v.modified_at,
            created_at: v.created_at,
        })
        .collect();

    Ok(infos)
}

/// Restore a file version
#[tauri::command]
pub async fn restore_version(
    state: State<'_, Arc<AppState>>,
    folder_id: i64,
    relative_path: String,
    version_id: i64,
) -> CmdResult<()> {
    // Get folder path
    let folder = state
        .db
        .get_sync_folder(folder_id)?
        .ok_or_else(|| SyncError::FolderNotFound(format!("Folder {} not found", folder_id)))?;

    // Get version info
    let file_meta = state.db.get_file_metadata(folder_id, &relative_path)?;
    let file_id = file_meta
        .map(|m| m.id)
        .ok_or_else(|| SyncError::FileNotFound(relative_path.clone()))?;

    let versions = state.db.get_file_versions(file_id)?;
    let version = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| SyncError::Version(format!("Version {} not found", version_id)))?;

    // Get storage path
    let storage_path = version
        .storage_path
        .as_ref()
        .ok_or_else(|| SyncError::Version("Version storage path not found".to_string()))?;

    // Copy version to current file location
    let target_path = PathBuf::from(&folder.local_path).join(&relative_path);
    tokio::fs::copy(storage_path, &target_path)
        .await
        .map_err(|e| SyncError::FileSystem(format!("Failed to restore version: {}", e)))?;

    info!("Restored version {} for {}", version_id, relative_path);
    Ok(())
}

// ============================================================================
// Device Commands
// ============================================================================

/// List all devices
#[tauri::command]
pub async fn list_devices(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<DeviceInfo>> {
    let devices = state.db.list_devices()?;

    let infos = devices
        .into_iter()
        .map(|d| DeviceInfo {
            id: d.id,
            device_id: d.device_id,
            device_name: d.device_name,
            last_seen: d.last_seen,
            is_self: d.is_self,
        })
        .collect();

    Ok(infos)
}

/// Remove a device
#[tauri::command]
pub async fn remove_device(state: State<'_, Arc<AppState>>, device_id: String) -> CmdResult<()> {
    state.db.remove_device(&device_id)?;
    info!("Removed device: {}", device_id);
    Ok(())
}

// ============================================================================
// Settings Commands
// ============================================================================

/// Get application settings
#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> CmdResult<AppSettings> {
    state.get_settings()
}

/// Update application settings
#[tauri::command]
pub async fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> CmdResult<()> {
    state.update_settings(&settings)?;
    info!("Settings updated");
    Ok(())
}

// ============================================================================
// Ignored Patterns Commands
// ============================================================================

/// Get ignored patterns
#[tauri::command]
pub async fn get_ignored_patterns(
    state: State<'_, Arc<AppState>>,
    folder_id: Option<i64>,
) -> CmdResult<Vec<String>> {
    let patterns = state.db.get_ignored_patterns(folder_id)?;
    Ok(patterns)
}

/// Add ignored pattern
#[tauri::command]
pub async fn add_ignored_pattern(
    state: State<'_, Arc<AppState>>,
    folder_id: Option<i64>,
    pattern: String,
) -> CmdResult<()> {
    state.db.add_ignored_pattern(folder_id, &pattern)?;

    // Update watcher patterns
    if let Some(watcher) = state.watcher.write().as_mut() {
        watcher.add_ignored_pattern(pattern.clone());
    }

    info!("Added ignored pattern: {}", pattern);
    Ok(())
}

// ============================================================================
// File Browser Commands
// ============================================================================

/// List files in a folder
#[tauri::command]
pub async fn list_folder_files(
    state: State<'_, Arc<AppState>>,
    folder_id: i64,
) -> CmdResult<Vec<FileInfo>> {
    let files = state.db.list_folder_files(folder_id)?;

    let mut infos = Vec::new();
    for file in files {
        if file.is_directory {
            continue;
        }

        let versions = state.db.get_file_versions(file.id)?;
        let version_infos: Vec<VersionInfo> = versions
            .into_iter()
            .map(|v| VersionInfo {
                id: v.id,
                version_number: v.version_number,
                size: v.size,
                modified_at: v.modified_at,
                created_at: v.created_at,
            })
            .collect();

        infos.push(FileInfo {
            relative_path: file.relative_path,
            size: file.size,
            modified_at: file.modified_at,
            synced: file.synced,
            versions: version_infos,
        });
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = OverallStatus {
            status: "idle".to_string(),
            total_folders: 2,
            syncing_folders: 0,
            total_files: 100,
            pending_operations: 5,
            unresolved_conflicts: 1,
            is_paused: false,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"idle\""));
    }
}
