//! File Versioning
//!
//! Provides version history management and restoration capabilities.

use crate::database::{Database, FileVersion};
use crate::error::{ShareError, ShareResult};
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

/// Maximum versions to keep per file (default)
pub const DEFAULT_MAX_VERSIONS: i64 = 10;

/// Version manager handles file version operations
pub struct VersionManager {
    db: Arc<Database>,
    state: Arc<AppState>,
}

/// Version info for UI display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    pub file_id: String,
    pub version: i64,
    pub size: i64,
    pub hash: String,
    pub uploaded_by: String,
    pub uploaded_at: i64,
    pub is_current: bool,
}

impl VersionManager {
    /// Create a new version manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Get all versions of a file
    pub fn get_file_versions(&self, file_id: &str) -> ShareResult<Vec<VersionInfo>> {
        // Get file to determine current version
        let file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Get all versions
        let versions = self.db.get_file_versions(file_id)?;

        let version_infos = versions
            .into_iter()
            .map(|v| VersionInfo {
                file_id: v.file_id,
                version: v.version,
                size: v.size,
                hash: v.hash,
                uploaded_by: v.uploaded_by,
                uploaded_at: v.uploaded_at,
                is_current: v.version == file.current_version,
            })
            .collect();

        Ok(version_infos)
    }

    /// Get a specific version
    pub fn get_version(&self, file_id: &str, version: i64) -> ShareResult<Option<FileVersion>> {
        self.db
            .get_file_version(file_id, version)
            .map_err(ShareError::from)
    }

    /// Get the current version of a file
    pub fn get_current_version(&self, file_id: &str) -> ShareResult<FileVersion> {
        let file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        self.db
            .get_file_version(file_id, file.current_version)?
            .ok_or_else(|| {
                ShareError::VersionNotFound(format!("{}v{}", file_id, file.current_version))
            })
    }

    /// Get version count for a file
    pub fn version_count(&self, file_id: &str) -> ShareResult<usize> {
        let versions = self.db.get_file_versions(file_id)?;
        Ok(versions.len())
    }

    /// Prune old versions beyond the retention limit
    pub async fn prune_versions(&self, file_id: &str, max_versions: i64) -> ShareResult<usize> {
        let paths = self.db.prune_old_versions(file_id, max_versions)?;
        let count = paths.len();

        // Delete the actual files
        for path in paths {
            if let Err(e) = fs::remove_file(&path).await {
                tracing::warn!("Failed to delete old version file {}: {}", path, e);
            }
        }

        info!("Pruned {} old versions of file {}", count, file_id);

        Ok(count)
    }

    /// Get storage path for a version
    pub fn get_version_storage_path(&self, file_id: &str, version: i64) -> PathBuf {
        self.state.get_version_storage_path(file_id, version)
    }

    /// Check if a specific version exists
    pub fn version_exists(&self, file_id: &str, version: i64) -> ShareResult<bool> {
        self.db
            .get_file_version(file_id, version)
            .map(|v| v.is_some())
            .map_err(ShareError::from)
    }

    /// Get total storage used by all versions of a file
    pub fn get_total_version_size(&self, file_id: &str) -> ShareResult<i64> {
        let versions = self.db.get_file_versions(file_id)?;
        Ok(versions.iter().map(|v| v.size).sum())
    }

    /// Compare two versions (returns size difference)
    pub fn compare_versions(&self, file_id: &str, v1: i64, v2: i64) -> ShareResult<i64> {
        let version1 = self
            .db
            .get_file_version(file_id, v1)?
            .ok_or_else(|| ShareError::VersionNotFound(format!("{}v{}", file_id, v1)))?;

        let version2 = self
            .db
            .get_file_version(file_id, v2)?
            .ok_or_else(|| ShareError::VersionNotFound(format!("{}v{}", file_id, v2)))?;

        Ok(version2.size - version1.size)
    }

    /// Get version history summary for a file
    pub fn get_version_summary(&self, file_id: &str) -> ShareResult<VersionSummary> {
        let file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        let versions = self.db.get_file_versions(file_id)?;

        let total_size: i64 = versions.iter().map(|v| v.size).sum();
        let oldest = versions.last().map(|v| v.uploaded_at);
        let newest = versions.first().map(|v| v.uploaded_at);

        Ok(VersionSummary {
            file_id: file_id.to_string(),
            file_name: file.name,
            current_version: file.current_version,
            total_versions: versions.len(),
            total_storage_bytes: total_size,
            oldest_version_at: oldest,
            newest_version_at: newest,
        })
    }
}

/// Summary of version history for a file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionSummary {
    pub file_id: String,
    pub file_name: String,
    pub current_version: i64,
    pub total_versions: usize,
    pub total_storage_bytes: i64,
    pub oldest_version_at: Option<i64>,
    pub newest_version_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Group, SharedFile};
    use chrono::Utc;
    use tempfile::tempdir;

    fn create_test_env() -> (Arc<Database>, Arc<AppState>, VersionManager) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let version_manager = VersionManager::new(db.clone(), state.clone());

        (db, state, version_manager)
    }

    #[test]
    fn test_get_file_versions() {
        let (db, _state, version_manager) = create_test_env();

        // Create group and file
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        let file = SharedFile {
            id: "test-file".to_string(),
            group_id: group.id.clone(),
            name: "test.txt".to_string(),
            path: "/test.txt".to_string(),
            size: 1024,
            mime_type: Some("text/plain".to_string()),
            uploaded_by: "peer-123".to_string(),
            uploaded_at: Utc::now().timestamp(),
            current_version: 2,
            hash: "abc123".to_string(),
        };
        db.create_shared_file(&file).unwrap();

        // Create versions
        for v in 1..=2 {
            let version = FileVersion {
                file_id: file.id.clone(),
                version: v,
                size: 1024 * v as i64,
                hash: format!("hash-{}", v),
                uploaded_by: "peer-123".to_string(),
                uploaded_at: Utc::now().timestamp(),
                storage_path: None,
            };
            db.create_file_version(&version).unwrap();
        }

        // Get versions
        let versions = version_manager.get_file_versions(&file.id).unwrap();
        assert_eq!(versions.len(), 2);

        // Check current version flag
        let current = versions.iter().find(|v| v.is_current).unwrap();
        assert_eq!(current.version, 2);
    }

    #[test]
    fn test_version_summary() {
        let (db, _state, version_manager) = create_test_env();

        // Create group and file
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        let file = SharedFile {
            id: "test-file".to_string(),
            group_id: group.id.clone(),
            name: "document.pdf".to_string(),
            path: "/docs/document.pdf".to_string(),
            size: 2048,
            mime_type: Some("application/pdf".to_string()),
            uploaded_by: "peer-123".to_string(),
            uploaded_at: Utc::now().timestamp(),
            current_version: 3,
            hash: "abc123".to_string(),
        };
        db.create_shared_file(&file).unwrap();

        // Create versions
        for v in 1..=3 {
            let version = FileVersion {
                file_id: file.id.clone(),
                version: v,
                size: 1024 * v as i64,
                hash: format!("hash-{}", v),
                uploaded_by: "peer-123".to_string(),
                uploaded_at: Utc::now().timestamp(),
                storage_path: None,
            };
            db.create_file_version(&version).unwrap();
        }

        // Get summary
        let summary = version_manager.get_version_summary(&file.id).unwrap();
        assert_eq!(summary.file_name, "document.pdf");
        assert_eq!(summary.current_version, 3);
        assert_eq!(summary.total_versions, 3);
        assert_eq!(summary.total_storage_bytes, 1024 + 2048 + 3072);
    }
}
