//! Link Sharing
//!
//! Enables public link sharing with optional password protection,
//! expiration dates, and download limits.

use crate::database::{ActivityEvent, Database, ShareLink, SharedFile};
use crate::error::{ShareError, ShareResult};
use crate::state::AppState;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Link share manager handles public link operations
pub struct LinkShareManager {
    db: Arc<Database>,
    state: Arc<AppState>,
}

/// Share link info for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLinkInfo {
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub has_password: bool,
    pub max_downloads: Option<i64>,
    pub download_count: i64,
    pub revoked: bool,
    pub is_expired: bool,
    pub is_active: bool,
    pub share_url: String,
}

/// Options for creating a share link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLinkOptions {
    pub expires_in_hours: Option<i64>,
    pub password: Option<String>,
    pub max_downloads: Option<i64>,
}

impl LinkShareManager {
    /// Create a new link share manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Create a new share link
    pub fn create_share_link(
        &self,
        file_id: &str,
        options: ShareLinkOptions,
    ) -> ShareResult<ShareLinkInfo> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get file to verify it exists and get permissions
        let file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        // Verify permission (must have read access at minimum)
        let member = self
            .db
            .get_group_member(&file.group_id, &peer_id)?
            .ok_or_else(|| {
                ShareError::PermissionDenied("Not a member of this group".to_string())
            })?;

        // Only admin or file uploader can create share links
        if member.role != "admin" && file.uploaded_by != peer_id {
            return Err(ShareError::PermissionDenied(
                "Only admins or the file uploader can create share links".to_string(),
            ));
        }

        // Generate unique link ID
        let link_id = self.generate_link_id();

        // Calculate expiration
        let expires_at = options
            .expires_in_hours
            .map(|hours| Utc::now().timestamp() + hours * 3600);

        // Hash password if provided
        let password_hash = if let Some(ref password) = options.password {
            Some(self.hash_password(password)?)
        } else {
            None
        };

        // Create share link record
        let link = ShareLink {
            id: link_id.clone(),
            file_id: file_id.to_string(),
            created_by: peer_id.clone(),
            created_at: Utc::now().timestamp(),
            expires_at,
            password_hash,
            max_downloads: options.max_downloads,
            download_count: 0,
            revoked: false,
        };

        self.db.create_share_link(&link)?;

        // Log activity
        self.log_activity(
            &file.group_id,
            "link_created",
            &peer_id,
            Some(file_id),
            Some(&file.name),
            Some(&format!(
                "Expires: {}",
                expires_at
                    .map(|t| chrono::DateTime::from_timestamp(t, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "never".to_string()))
                    .unwrap_or_else(|| "never".to_string())
            )),
        )?;

        info!("Created share link for file: {}", file.name);

        self.to_share_link_info(&link, &file)
    }

    /// Get a share link by ID
    pub fn get_share_link(&self, link_id: &str) -> ShareResult<Option<ShareLinkInfo>> {
        let link = match self.db.get_share_link(link_id)? {
            Some(l) => l,
            None => return Ok(None),
        };

        let file = match self.db.get_shared_file(&link.file_id)? {
            Some(f) => f,
            None => return Err(ShareError::FileNotFound(link.file_id.clone())),
        };

        Ok(Some(self.to_share_link_info(&link, &file)?))
    }

    /// List all share links for a file
    pub fn list_file_share_links(&self, file_id: &str) -> ShareResult<Vec<ShareLinkInfo>> {
        let file = self
            .db
            .get_shared_file(file_id)?
            .ok_or_else(|| ShareError::FileNotFound(file_id.to_string()))?;

        let links = self.db.list_file_share_links(file_id)?;

        let link_infos: Result<Vec<_>, _> = links
            .into_iter()
            .map(|l| self.to_share_link_info(&l, &file))
            .collect();

        link_infos
    }

    /// Revoke a share link
    pub fn revoke_share_link(&self, link_id: &str) -> ShareResult<()> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get link
        let link = self
            .db
            .get_share_link(link_id)?
            .ok_or_else(|| ShareError::LinkNotFound(link_id.to_string()))?;

        // Get file
        let file = self
            .db
            .get_shared_file(&link.file_id)?
            .ok_or_else(|| ShareError::FileNotFound(link.file_id.clone()))?;

        // Verify permission (admin or link creator)
        let member = self
            .db
            .get_group_member(&file.group_id, &peer_id)?
            .ok_or_else(|| {
                ShareError::PermissionDenied("Not a member of this group".to_string())
            })?;

        if member.role != "admin" && link.created_by != peer_id {
            return Err(ShareError::PermissionDenied(
                "Only admins or the link creator can revoke share links".to_string(),
            ));
        }

        self.db.revoke_share_link(link_id)?;

        // Log activity
        self.log_activity(
            &file.group_id,
            "link_revoked",
            &peer_id,
            Some(&link.file_id),
            Some(&file.name),
            None,
        )?;

        info!("Revoked share link: {}", link_id);

        Ok(())
    }

    /// Validate and access a share link (for downloading)
    pub fn access_share_link(
        &self,
        link_id: &str,
        password: Option<&str>,
    ) -> ShareResult<SharedFile> {
        // Get link
        let link = self
            .db
            .get_share_link(link_id)?
            .ok_or_else(|| ShareError::LinkNotFound(link_id.to_string()))?;

        // Check if revoked
        if link.revoked {
            return Err(ShareError::AccessRevoked);
        }

        // Check expiration
        if let Some(expires_at) = link.expires_at
            && Utc::now().timestamp() > expires_at
        {
            return Err(ShareError::LinkExpired);
        }

        // Check download limit
        if let Some(max_downloads) = link.max_downloads
            && link.download_count >= max_downloads
        {
            return Err(ShareError::DownloadLimitExceeded);
        }

        // Verify password if required
        if let Some(ref password_hash) = link.password_hash {
            let provided_password = password.ok_or(ShareError::InvalidPassword)?;
            self.verify_password(provided_password, password_hash)?;
        }

        // Get file
        let file = self
            .db
            .get_shared_file(&link.file_id)?
            .ok_or_else(|| ShareError::FileNotFound(link.file_id.clone()))?;

        // Increment download count
        self.db.increment_link_download_count(link_id)?;

        // Log activity
        self.log_activity(
            &file.group_id,
            "link_accessed",
            "anonymous",
            Some(&file.id),
            Some(&file.name),
            None,
        )?;

        info!("Share link accessed: {}", link_id);

        Ok(file)
    }

    /// Check if a share link is valid (without incrementing download count)
    pub fn validate_share_link(&self, link_id: &str) -> ShareResult<bool> {
        let link = match self.db.get_share_link(link_id)? {
            Some(l) => l,
            None => return Ok(false),
        };

        // Check if revoked
        if link.revoked {
            return Ok(false);
        }

        // Check expiration
        if let Some(expires_at) = link.expires_at
            && Utc::now().timestamp() > expires_at
        {
            return Ok(false);
        }

        // Check download limit
        if let Some(max_downloads) = link.max_downloads
            && link.download_count >= max_downloads
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if a share link requires a password
    pub fn requires_password(&self, link_id: &str) -> ShareResult<bool> {
        let link = self
            .db
            .get_share_link(link_id)?
            .ok_or_else(|| ShareError::LinkNotFound(link_id.to_string()))?;

        Ok(link.password_hash.is_some())
    }

    /// Generate a unique link ID
    fn generate_link_id(&self) -> String {
        let uuid = Uuid::new_v4();
        BASE64_URL.encode(uuid.as_bytes())
    }

    /// Hash a password using Argon2
    fn hash_password(&self, password: &str) -> ShareResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| ShareError::Crypto("Password hashing failed".to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password against a hash
    fn verify_password(&self, password: &str, hash: &str) -> ShareResult<()> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| ShareError::Crypto("Invalid password hash".to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| ShareError::InvalidPassword)?;

        Ok(())
    }

    /// Convert database link to UI-friendly format
    fn to_share_link_info(
        &self,
        link: &ShareLink,
        file: &SharedFile,
    ) -> ShareResult<ShareLinkInfo> {
        let now = Utc::now().timestamp();
        let is_expired = link
            .expires_at
            .map(|expires| now > expires)
            .unwrap_or(false);

        let download_limit_exceeded = link
            .max_downloads
            .map(|max| link.download_count >= max)
            .unwrap_or(false);

        let is_active = !link.revoked && !is_expired && !download_limit_exceeded;

        // Generate share URL (this would be customized based on deployment)
        let share_url = format!("wraith://share/{}", link.id);

        Ok(ShareLinkInfo {
            id: link.id.clone(),
            file_id: link.file_id.clone(),
            file_name: file.name.clone(),
            created_by: link.created_by.clone(),
            created_at: link.created_at,
            expires_at: link.expires_at,
            has_password: link.password_hash.is_some(),
            max_downloads: link.max_downloads,
            download_count: link.download_count,
            revoked: link.revoked,
            is_expired,
            is_active,
            share_url,
        })
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Group, GroupMember};
    use tempfile::tempdir;

    fn create_test_env() -> (Arc<Database>, Arc<AppState>, LinkShareManager) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let manager = LinkShareManager::new(db.clone(), state.clone());

        // Create test group and member
        let peer_id = state.get_peer_id().unwrap();
        let public_key = state.get_public_key_bytes().unwrap();

        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: peer_id.clone(),
        };
        db.create_group(&group).unwrap();

        let member = GroupMember {
            group_id: group.id.clone(),
            peer_id: peer_id.clone(),
            display_name: Some("Test User".to_string()),
            role: "admin".to_string(),
            joined_at: Utc::now().timestamp(),
            invited_by: peer_id.clone(),
            public_key,
        };
        db.add_group_member(&member).unwrap();

        (db, state, manager)
    }

    fn create_test_file(db: &Database) -> SharedFile {
        let file = SharedFile {
            id: "test-file".to_string(),
            group_id: "test-group".to_string(),
            name: "document.pdf".to_string(),
            path: "/docs/document.pdf".to_string(),
            size: 1024,
            mime_type: Some("application/pdf".to_string()),
            uploaded_by: db.get_local_identity().unwrap().unwrap().peer_id,
            uploaded_at: Utc::now().timestamp(),
            current_version: 1,
            hash: "abc123".to_string(),
        };
        db.create_shared_file(&file).unwrap();
        file
    }

    #[test]
    fn test_create_share_link() {
        let (db, _state, manager) = create_test_env();
        let file = create_test_file(&db);

        let options = ShareLinkOptions {
            expires_in_hours: Some(24),
            password: None,
            max_downloads: Some(10),
        };

        let link = manager.create_share_link(&file.id, options).unwrap();

        assert_eq!(link.file_id, file.id);
        assert_eq!(link.file_name, file.name);
        assert!(!link.has_password);
        assert_eq!(link.max_downloads, Some(10));
        assert_eq!(link.download_count, 0);
        assert!(link.is_active);
    }

    #[test]
    fn test_share_link_with_password() {
        let (db, _state, manager) = create_test_env();
        let file = create_test_file(&db);

        let options = ShareLinkOptions {
            expires_in_hours: None,
            password: Some("secret123".to_string()),
            max_downloads: None,
        };

        let link = manager.create_share_link(&file.id, options).unwrap();

        assert!(link.has_password);
        assert!(manager.requires_password(&link.id).unwrap());

        // Access without password should fail
        let result = manager.access_share_link(&link.id, None);
        assert!(result.is_err());

        // Access with wrong password should fail
        let result = manager.access_share_link(&link.id, Some("wrong"));
        assert!(result.is_err());

        // Access with correct password should succeed
        let accessed_file = manager
            .access_share_link(&link.id, Some("secret123"))
            .unwrap();
        assert_eq!(accessed_file.id, file.id);
    }

    #[test]
    fn test_share_link_download_limit() {
        let (db, _state, manager) = create_test_env();
        let file = create_test_file(&db);

        let options = ShareLinkOptions {
            expires_in_hours: None,
            password: None,
            max_downloads: Some(2),
        };

        let link = manager.create_share_link(&file.id, options).unwrap();

        // First download
        manager.access_share_link(&link.id, None).unwrap();

        // Second download
        manager.access_share_link(&link.id, None).unwrap();

        // Third download should fail
        let result = manager.access_share_link(&link.id, None);
        assert!(matches!(result, Err(ShareError::DownloadLimitExceeded)));
    }

    #[test]
    fn test_revoke_share_link() {
        let (db, _state, manager) = create_test_env();
        let file = create_test_file(&db);

        let options = ShareLinkOptions {
            expires_in_hours: None,
            password: None,
            max_downloads: None,
        };

        let link = manager.create_share_link(&file.id, options).unwrap();
        assert!(link.is_active);

        // Revoke
        manager.revoke_share_link(&link.id).unwrap();

        // Verify it's revoked
        let updated_link = manager.get_share_link(&link.id).unwrap().unwrap();
        assert!(updated_link.revoked);
        assert!(!updated_link.is_active);

        // Access should fail
        let result = manager.access_share_link(&link.id, None);
        assert!(matches!(result, Err(ShareError::AccessRevoked)));
    }

    #[test]
    fn test_validate_share_link() {
        let (db, _state, manager) = create_test_env();
        let file = create_test_file(&db);

        let options = ShareLinkOptions {
            expires_in_hours: Some(24),
            password: None,
            max_downloads: None,
        };

        let link = manager.create_share_link(&file.id, options).unwrap();

        assert!(manager.validate_share_link(&link.id).unwrap());

        // Revoke and check again
        manager.revoke_share_link(&link.id).unwrap();
        assert!(!manager.validate_share_link(&link.id).unwrap());
    }
}
