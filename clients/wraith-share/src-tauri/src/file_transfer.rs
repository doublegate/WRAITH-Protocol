//! File Transfer
//!
//! Handles encrypted file upload and download with progress tracking.

use crate::access_control::{AccessController, EncryptedFileData};
use crate::database::{ActivityEvent, Database, FileVersion, SharedFile};
use crate::error::{ShareError, ShareResult};
use crate::state::AppState;
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::info;
use uuid::Uuid;

/// Maximum versions to keep per file
const MAX_VERSIONS: i64 = 10;

/// File transfer manager handles upload and download operations
pub struct FileTransfer {
    db: Arc<Database>,
    state: Arc<AppState>,
    access_control: Arc<AccessController>,
}

impl FileTransfer {
    /// Create a new file transfer manager
    pub fn new(
        db: Arc<Database>,
        state: Arc<AppState>,
        access_control: Arc<AccessController>,
    ) -> Self {
        Self {
            db,
            state,
            access_control,
        }
    }

    /// Upload a file to a group
    pub async fn upload_file(
        &self,
        group_id: &str,
        virtual_path: &str,
        file_data: Vec<u8>,
    ) -> ShareResult<SharedFile> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Verify write permission
        let member = self
            .db
            .get_group_member(group_id, &peer_id)?
            .ok_or_else(|| {
                ShareError::PermissionDenied("Not a member of this group".to_string())
            })?;

        if member.role == "read" {
            return Err(ShareError::PermissionDenied(
                "Read-only members cannot upload files".to_string(),
            ));
        }

        // Get all group members for encryption
        let members = self.db.list_group_members(group_id)?;

        // Generate file ID
        let file_id = Uuid::new_v4().to_string();

        // Calculate hash
        let hash = hex::encode(blake3::hash(&file_data).as_bytes());

        // Detect MIME type
        let mime_type = self.detect_mime_type(virtual_path);

        // Extract file name from path
        let name = Path::new(virtual_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        // Create shared file record FIRST (capabilities have FK reference to it)
        let shared_file = SharedFile {
            id: file_id.clone(),
            group_id: group_id.to_string(),
            name,
            path: virtual_path.to_string(),
            size: file_data.len() as i64,
            mime_type: Some(mime_type),
            uploaded_by: peer_id.clone(),
            uploaded_at: Utc::now().timestamp(),
            current_version: 1,
            hash: hash.clone(),
        };

        self.db.create_shared_file(&shared_file)?;

        // Encrypt file for all members
        let (encrypted_file, capabilities) = self
            .access_control
            .encrypt_file_for_group(&file_data, &file_id, group_id, &members)?;

        // Store capabilities (after shared_file exists due to FK constraint)
        self.access_control.store_capabilities(&capabilities)?;

        // Store encrypted file
        let storage_path = self.state.get_file_storage_path(&file_id);
        let encrypted_data = serde_json::to_vec(&encrypted_file)?;
        fs::write(&storage_path, &encrypted_data).await?;

        // Create initial version record
        let version = FileVersion {
            file_id: file_id.clone(),
            version: 1,
            size: file_data.len() as i64,
            hash: hash.clone(),
            uploaded_by: peer_id.clone(),
            uploaded_at: Utc::now().timestamp(),
            storage_path: Some(storage_path.to_string_lossy().to_string()),
        };

        self.db.create_file_version(&version)?;

        // Log activity
        self.log_activity(
            group_id,
            "file_uploaded",
            &peer_id,
            Some(&file_id),
            Some(&shared_file.name),
            Some(&format!("Size: {} bytes", file_data.len())),
        )?;

        info!("Uploaded file: {} to group {}", shared_file.name, group_id);

        Ok(shared_file)
    }

    /// Upload a new version of an existing file
    pub async fn upload_new_version(
        &self,
        file_id: &str,
        file_data: Vec<u8>,
    ) -> ShareResult<SharedFile> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get existing file
        let mut shared_file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Verify write permission
        let member = self
            .db
            .get_group_member(&shared_file.group_id, &peer_id)?
            .ok_or_else(|| {
                ShareError::PermissionDenied("Not a member of this group".to_string())
            })?;

        if member.role == "read" {
            return Err(ShareError::PermissionDenied(
                "Read-only members cannot update files".to_string(),
            ));
        }

        // Get all group members for encryption
        let members = self.db.list_group_members(&shared_file.group_id)?;

        // Encrypt new version for all members
        let (encrypted_file, capabilities) = self.access_control.encrypt_file_for_group(
            &file_data,
            file_id,
            &shared_file.group_id,
            &members,
        )?;

        // Update capabilities
        self.db.delete_file_capabilities(file_id)?;
        self.access_control.store_capabilities(&capabilities)?;

        // Calculate hash
        let hash = hex::encode(blake3::hash(&file_data).as_bytes());

        // Get next version number
        let new_version = self.db.get_next_version_number(file_id)?;

        // Store old version in versions directory before overwriting
        let current_storage_path = self.state.get_file_storage_path(file_id);
        let version_storage_path = self
            .state
            .get_version_storage_path(file_id, shared_file.current_version);

        if current_storage_path.exists() {
            fs::copy(&current_storage_path, &version_storage_path).await?;
        }

        // Store new encrypted file
        let encrypted_data = serde_json::to_vec(&encrypted_file)?;
        fs::write(&current_storage_path, &encrypted_data).await?;

        // Create version record
        let version = FileVersion {
            file_id: file_id.to_string(),
            version: new_version,
            size: file_data.len() as i64,
            hash: hash.clone(),
            uploaded_by: peer_id.clone(),
            uploaded_at: Utc::now().timestamp(),
            storage_path: Some(current_storage_path.to_string_lossy().to_string()),
        };

        self.db.create_file_version(&version)?;

        // Update file record
        self.db
            .update_file_version(file_id, new_version, &hash, file_data.len() as i64)?;

        // Prune old versions
        let old_paths = self.db.prune_old_versions(file_id, MAX_VERSIONS)?;
        for path in old_paths {
            let _ = fs::remove_file(&path).await;
        }

        // Log activity
        self.log_activity(
            &shared_file.group_id,
            "file_updated",
            &peer_id,
            Some(file_id),
            Some(&shared_file.name),
            Some(&format!("Version: {}", new_version)),
        )?;

        // Update shared file struct
        shared_file.current_version = new_version;
        shared_file.hash = hash;
        shared_file.size = file_data.len() as i64;
        shared_file.uploaded_at = Utc::now().timestamp();

        info!(
            "Uploaded version {} of file: {}",
            new_version, shared_file.name
        );

        Ok(shared_file)
    }

    /// Download a file
    pub async fn download_file(&self, file_id: &str) -> ShareResult<Vec<u8>> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get file
        let shared_file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Get our capability
        let capability = self
            .db
            .get_file_capability(file_id, &peer_id)?
            .ok_or_else(|| ShareError::PermissionDenied("No access to this file".to_string()))?;

        // Get granter's public key
        let granter = self
            .db
            .get_group_member(&shared_file.group_id, &capability.granted_by)?
            .ok_or_else(|| ShareError::Crypto("Granter not found".to_string()))?;

        // Read encrypted file
        let storage_path = self.state.get_file_storage_path(file_id);
        let encrypted_data = fs::read(&storage_path).await?;
        let encrypted_file: EncryptedFileData = serde_json::from_slice(&encrypted_data)?;

        // Decrypt file
        let plaintext = self.access_control.decrypt_file_with_capability(
            &encrypted_file,
            &capability,
            &granter.public_key,
        )?;

        // Verify hash
        let actual_hash = hex::encode(blake3::hash(&plaintext).as_bytes());
        if actual_hash != shared_file.hash {
            return Err(ShareError::Crypto(
                "File integrity check failed".to_string(),
            ));
        }

        // Log activity
        self.log_activity(
            &shared_file.group_id,
            "file_downloaded",
            &peer_id,
            Some(file_id),
            Some(&shared_file.name),
            None,
        )?;

        info!("Downloaded file: {}", shared_file.name);

        Ok(plaintext)
    }

    /// Download a specific version of a file
    pub async fn download_file_version(&self, file_id: &str, version: i64) -> ShareResult<Vec<u8>> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get file
        let shared_file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Get version
        let file_version = self
            .db
            .get_file_version(file_id, version)?
            .ok_or_else(|| ShareError::VersionNotFound(format!("{}v{}", file_id, version)))?;

        // Get our capability
        let capability = self
            .db
            .get_file_capability(file_id, &peer_id)?
            .ok_or_else(|| ShareError::PermissionDenied("No access to this file".to_string()))?;

        // Get granter's public key
        let granter = self
            .db
            .get_group_member(&shared_file.group_id, &capability.granted_by)?
            .ok_or_else(|| ShareError::Crypto("Granter not found".to_string()))?;

        // Determine storage path
        let storage_path = if version == shared_file.current_version {
            self.state.get_file_storage_path(file_id)
        } else {
            self.state.get_version_storage_path(file_id, version)
        };

        // Read encrypted file
        let encrypted_data = fs::read(&storage_path).await?;
        let encrypted_file: EncryptedFileData = serde_json::from_slice(&encrypted_data)?;

        // Decrypt file
        let plaintext = self.access_control.decrypt_file_with_capability(
            &encrypted_file,
            &capability,
            &granter.public_key,
        )?;

        // Verify hash
        let actual_hash = hex::encode(blake3::hash(&plaintext).as_bytes());
        if actual_hash != file_version.hash {
            return Err(ShareError::Crypto(
                "Version integrity check failed".to_string(),
            ));
        }

        info!(
            "Downloaded version {} of file: {}",
            version, shared_file.name
        );

        Ok(plaintext)
    }

    /// Delete a file
    pub async fn delete_file(&self, file_id: &str) -> ShareResult<()> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get file
        let shared_file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Verify permission (admin or uploader)
        let member = self
            .db
            .get_group_member(&shared_file.group_id, &peer_id)?
            .ok_or_else(|| {
                ShareError::PermissionDenied("Not a member of this group".to_string())
            })?;

        if member.role != "admin" && shared_file.uploaded_by != peer_id {
            return Err(ShareError::PermissionDenied(
                "Only admins or the uploader can delete files".to_string(),
            ));
        }

        // Delete encrypted files
        let storage_path = self.state.get_file_storage_path(file_id);
        let _ = fs::remove_file(&storage_path).await;

        // Delete version files
        let versions = self.db.get_file_versions(file_id)?;
        for version in versions {
            if let Some(path) = &version.storage_path {
                let _ = fs::remove_file(path).await;
            }
        }

        // Delete capabilities
        self.db.delete_file_capabilities(file_id)?;

        // Mark file as deleted
        self.db.delete_shared_file(file_id)?;

        // Log activity
        self.log_activity(
            &shared_file.group_id,
            "file_deleted",
            &peer_id,
            Some(file_id),
            Some(&shared_file.name),
            None,
        )?;

        info!("Deleted file: {}", shared_file.name);

        Ok(())
    }

    /// List files in a group
    pub fn list_files(&self, group_id: &str) -> ShareResult<Vec<SharedFile>> {
        self.db.list_group_files(group_id).map_err(ShareError::from)
    }

    /// Search files in a group
    pub fn search_files(&self, group_id: &str, query: &str) -> ShareResult<Vec<SharedFile>> {
        self.db
            .search_group_files(group_id, query)
            .map_err(ShareError::from)
    }

    /// Get file versions
    pub fn get_file_versions(&self, file_id: &str) -> ShareResult<Vec<FileVersion>> {
        self.db.get_file_versions(file_id).map_err(ShareError::from)
    }

    /// Restore a file to a previous version
    pub async fn restore_version(&self, file_id: &str, version: i64) -> ShareResult<SharedFile> {
        // Download the specified version
        let version_data = self.download_file_version(file_id, version).await?;

        // Upload as new version
        self.upload_new_version(file_id, version_data).await
    }

    /// Detect MIME type from file path
    fn detect_mime_type(&self, path: &str) -> String {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string()
    }

    /// Log an activity event
    fn log_activity(
        &self,
        group_id: &str,
        event_type: &str,
        actor_id: &str,
        target_id: Option<&str>,
        target_name: Option<&str>,
        details: Option<&str>,
    ) -> ShareResult<()> {
        let event = ActivityEvent {
            id: 0,
            group_id: group_id.to_string(),
            event_type: event_type.to_string(),
            actor_id: actor_id.to_string(),
            target_id: target_id.map(String::from),
            target_name: target_name.map(String::from),
            details: details.map(String::from),
            timestamp: Utc::now().timestamp(),
        };

        self.db.log_activity(&event)?;

        // Prune old events
        self.db
            .prune_activity_log(group_id, self.state.max_activity_events)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::GroupManager;
    use tempfile::{TempDir, tempdir};

    async fn create_test_env() -> (
        Arc<Database>,
        Arc<AppState>,
        FileTransfer,
        GroupManager,
        TempDir,
    ) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let access_control = Arc::new(AccessController::new(db.clone(), state.clone()));
        let file_transfer = FileTransfer::new(db.clone(), state.clone(), access_control);
        let group_manager = GroupManager::new(db.clone(), state.clone());

        (db, state, file_transfer, group_manager, dir)
    }

    #[tokio::test]
    async fn test_upload_download_file() {
        let (_db, _state, file_transfer, group_manager, _tmp) = create_test_env().await;

        // Create a group
        let group = group_manager.create_group("Test Group", None).unwrap();

        // Upload a file
        let file_data = b"Hello, WRAITH Share!".to_vec();
        let shared_file = file_transfer
            .upload_file(&group.id, "/documents/test.txt", file_data.clone())
            .await
            .unwrap();

        assert_eq!(shared_file.name, "test.txt");
        assert_eq!(shared_file.size, file_data.len() as i64);
        assert_eq!(shared_file.current_version, 1);

        // Download the file
        let downloaded = file_transfer.download_file(&shared_file.id).await.unwrap();
        assert_eq!(downloaded, file_data);
    }

    #[tokio::test]
    async fn test_upload_new_version() {
        let (_db, _state, file_transfer, group_manager, _tmp) = create_test_env().await;

        // Create a group
        let group = group_manager.create_group("Test Group", None).unwrap();

        // Upload initial file
        let file_data_v1 = b"Version 1 content".to_vec();
        let shared_file = file_transfer
            .upload_file(&group.id, "/test.txt", file_data_v1.clone())
            .await
            .unwrap();

        // Upload new version
        let file_data_v2 = b"Version 2 content - updated!".to_vec();
        let updated_file = file_transfer
            .upload_new_version(&shared_file.id, file_data_v2.clone())
            .await
            .unwrap();

        assert_eq!(updated_file.current_version, 2);

        // Download latest version
        let downloaded = file_transfer.download_file(&shared_file.id).await.unwrap();
        assert_eq!(downloaded, file_data_v2);

        // Check version history
        let versions = file_transfer.get_file_versions(&shared_file.id).unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_file() {
        let (db, _state, file_transfer, group_manager, _tmp) = create_test_env().await;

        // Create a group
        let group = group_manager.create_group("Test Group", None).unwrap();

        // Upload a file
        let file_data = b"To be deleted".to_vec();
        let shared_file = file_transfer
            .upload_file(&group.id, "/delete_me.txt", file_data)
            .await
            .unwrap();

        // Delete the file
        file_transfer.delete_file(&shared_file.id).await.unwrap();

        // Verify file is deleted
        let result = db.get_shared_file(&shared_file.id).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_files() {
        let (_db, _state, file_transfer, group_manager, _tmp) = create_test_env().await;

        // Create a group
        let group = group_manager.create_group("Test Group", None).unwrap();

        // Upload multiple files
        file_transfer
            .upload_file(&group.id, "/documents/report.pdf", b"PDF content".to_vec())
            .await
            .unwrap();
        file_transfer
            .upload_file(&group.id, "/documents/notes.txt", b"Notes content".to_vec())
            .await
            .unwrap();
        file_transfer
            .upload_file(&group.id, "/images/photo.jpg", b"Image data".to_vec())
            .await
            .unwrap();

        // Search for documents
        let results = file_transfer.search_files(&group.id, "documents").unwrap();
        assert_eq!(results.len(), 2);

        // Search for specific file
        let results = file_transfer.search_files(&group.id, "report").unwrap();
        assert_eq!(results.len(), 1);
    }
}
