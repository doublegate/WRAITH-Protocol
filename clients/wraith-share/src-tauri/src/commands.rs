//! Tauri IPC Commands for WRAITH Share
//!
//! Provides the command interface between the frontend and backend.

use crate::access_control::AccessController;
use crate::activity::{ActivityInfo, ActivityLogger, ActivityStats};
use crate::database::{Group, GroupMember, SharedFile};
use crate::error::ShareError;
use crate::file_transfer::FileTransfer;
use crate::group::{ExportedInvitation, GroupManager};
use crate::link_share::{LinkShareManager, ShareLinkInfo, ShareLinkOptions};
use crate::state::AppState;
use crate::versioning::{VersionInfo, VersionManager, VersionSummary};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// Application result type for Tauri commands
type CmdResult<T> = Result<T, ShareError>;

/// Shared managers state
pub struct Managers {
    pub group_manager: GroupManager,
    pub access_control: Arc<AccessController>,
    pub file_transfer: FileTransfer,
    pub version_manager: VersionManager,
    pub activity_logger: ActivityLogger,
    pub link_share_manager: LinkShareManager,
}

// =============================================================================
// Group Commands
// =============================================================================

/// Create a new group
#[tauri::command]
pub async fn create_group(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    name: String,
    description: Option<String>,
) -> CmdResult<Group> {
    managers
        .group_manager
        .create_group(&name, description.as_deref())
}

/// Delete a group
#[tauri::command]
pub async fn delete_group(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<()> {
    managers.group_manager.delete_group(&group_id)
}

/// Get a group by ID
#[tauri::command]
pub async fn get_group(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<Option<Group>> {
    managers.group_manager.get_group(&group_id)
}

/// List all groups
#[tauri::command]
pub async fn list_groups(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
) -> CmdResult<Vec<Group>> {
    managers.group_manager.list_groups()
}

/// Invite a member to a group
#[tauri::command]
pub async fn invite_member(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    peer_id: Option<String>,
    role: String,
) -> CmdResult<ExportedInvitation> {
    managers
        .group_manager
        .invite_member(&group_id, peer_id.as_deref(), &role)
}

/// Accept an invitation to join a group
#[tauri::command]
pub async fn accept_invitation(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    invitation: ExportedInvitation,
) -> CmdResult<GroupMember> {
    managers.group_manager.accept_invitation(&invitation)
}

/// Remove a member from a group
#[tauri::command]
pub async fn remove_member(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    peer_id: String,
) -> CmdResult<()> {
    managers.group_manager.remove_member(&group_id, &peer_id)
}

/// Set a member's role
#[tauri::command]
pub async fn set_member_role(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    peer_id: String,
    role: String,
) -> CmdResult<()> {
    managers
        .group_manager
        .set_member_role(&group_id, &peer_id, &role)
}

/// List members of a group
#[tauri::command]
pub async fn list_members(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<Vec<GroupMember>> {
    managers.group_manager.list_members(&group_id)
}

// =============================================================================
// File Commands
// =============================================================================

/// Upload a file to a group
#[tauri::command]
pub async fn upload_file(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    path: String,
    data: Vec<u8>,
) -> CmdResult<SharedFile> {
    managers
        .file_transfer
        .upload_file(&group_id, &path, data)
        .await
}

/// Download a file
#[tauri::command]
pub async fn download_file(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
) -> CmdResult<Vec<u8>> {
    managers.file_transfer.download_file(&file_id).await
}

/// Delete a file
#[tauri::command]
pub async fn delete_file(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
) -> CmdResult<()> {
    managers.file_transfer.delete_file(&file_id).await
}

/// List files in a group
#[tauri::command]
pub async fn list_files(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<Vec<SharedFile>> {
    managers.file_transfer.list_files(&group_id)
}

/// Search files in a group
#[tauri::command]
pub async fn search_files(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    query: String,
) -> CmdResult<Vec<SharedFile>> {
    managers.file_transfer.search_files(&group_id, &query)
}

// =============================================================================
// Version Commands
// =============================================================================

/// Get file versions
#[tauri::command]
pub async fn get_file_versions(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
) -> CmdResult<Vec<VersionInfo>> {
    managers.version_manager.get_file_versions(&file_id)
}

/// Restore a file to a previous version
#[tauri::command]
pub async fn restore_version(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
    version: i64,
) -> CmdResult<SharedFile> {
    managers
        .file_transfer
        .restore_version(&file_id, version)
        .await
}

/// Get version summary for a file
#[tauri::command]
pub async fn get_version_summary(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
) -> CmdResult<VersionSummary> {
    managers.version_manager.get_version_summary(&file_id)
}

// =============================================================================
// Activity Commands
// =============================================================================

/// Get activity log for a group
#[tauri::command]
pub async fn get_activity_log(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    limit: i64,
    offset: i64,
) -> CmdResult<Vec<ActivityInfo>> {
    managers
        .activity_logger
        .get_activity_log(&group_id, limit, offset)
}

/// Get recent activity across all groups
#[tauri::command]
pub async fn get_recent_activity(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    limit: i64,
) -> CmdResult<Vec<ActivityInfo>> {
    managers.activity_logger.get_recent_activity(limit)
}

/// Search activity log
#[tauri::command]
pub async fn search_activity(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
    query: String,
    limit: i64,
) -> CmdResult<Vec<ActivityInfo>> {
    managers
        .activity_logger
        .search_activity(&group_id, &query, limit)
}

/// Get activity statistics for a group
#[tauri::command]
pub async fn get_activity_stats(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<ActivityStats> {
    managers.activity_logger.get_activity_stats(&group_id)
}

// =============================================================================
// Link Sharing Commands
// =============================================================================

/// Create a share link
#[tauri::command]
pub async fn create_share_link(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
    expires_in_hours: Option<i64>,
    password: Option<String>,
    max_downloads: Option<i64>,
) -> CmdResult<ShareLinkInfo> {
    let options = ShareLinkOptions {
        expires_in_hours,
        password,
        max_downloads,
    };

    managers
        .link_share_manager
        .create_share_link(&file_id, options)
}

/// Get a share link by ID
#[tauri::command]
pub async fn get_share_link(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    link_id: String,
) -> CmdResult<Option<ShareLinkInfo>> {
    managers.link_share_manager.get_share_link(&link_id)
}

/// Revoke a share link
#[tauri::command]
pub async fn revoke_share_link(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    link_id: String,
) -> CmdResult<()> {
    managers.link_share_manager.revoke_share_link(&link_id)
}

/// Download via share link
#[tauri::command]
pub async fn download_via_link(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    link_id: String,
    password: Option<String>,
) -> CmdResult<Vec<u8>> {
    // Validate and access the link
    let file = managers
        .link_share_manager
        .access_share_link(&link_id, password.as_deref())?;

    // Download the file
    managers.file_transfer.download_file(&file.id).await
}

/// List share links for a file
#[tauri::command]
pub async fn list_file_share_links(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    file_id: String,
) -> CmdResult<Vec<ShareLinkInfo>> {
    managers.link_share_manager.list_file_share_links(&file_id)
}

/// Check if a share link requires a password
#[tauri::command]
pub async fn link_requires_password(
    _state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    link_id: String,
) -> CmdResult<bool> {
    managers.link_share_manager.requires_password(&link_id)
}

// =============================================================================
// Identity Commands
// =============================================================================

/// Get local peer ID
#[tauri::command]
pub async fn get_peer_id(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    state
        .get_peer_id()
        .ok_or_else(|| ShareError::Group("Identity not initialized".to_string()))
}

/// Get display name
#[tauri::command]
pub async fn get_display_name(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    Ok(state.get_display_name())
}

/// Set display name
#[tauri::command]
pub async fn set_display_name(state: State<'_, Arc<AppState>>, name: String) -> CmdResult<()> {
    state.set_display_name(&name)
}

// =============================================================================
// Group Info Types
// =============================================================================

/// Group info with member count and file count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
    pub member_count: i64,
    pub file_count: usize,
    pub is_admin: bool,
    pub my_role: String,
}

/// Get group info with additional details
#[tauri::command]
pub async fn get_group_info(
    state: State<'_, Arc<AppState>>,
    managers: State<'_, Managers>,
    group_id: String,
) -> CmdResult<Option<GroupInfo>> {
    let group = match managers.group_manager.get_group(&group_id)? {
        Some(g) => g,
        None => return Ok(None),
    };

    let member_count = managers.group_manager.member_count(&group_id)?;
    let files = managers.file_transfer.list_files(&group_id)?;
    let is_admin = managers.group_manager.is_admin(&group_id)?;

    let peer_id = state
        .get_peer_id()
        .ok_or_else(|| ShareError::Group("Identity not initialized".to_string()))?;

    let my_role = if let Some(member) = state.db.get_group_member(&group_id, &peer_id)? {
        member.role
    } else {
        "none".to_string()
    };

    Ok(Some(GroupInfo {
        id: group.id,
        name: group.name,
        description: group.description,
        created_at: group.created_at,
        created_by: group.created_by,
        member_count,
        file_count: files.len(),
        is_admin,
        my_role,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_info_serialization() {
        let info = GroupInfo {
            id: "test-id".to_string(),
            name: "Test Group".to_string(),
            description: Some("A test group".to_string()),
            created_at: 1234567890,
            created_by: "peer-123".to_string(),
            member_count: 5,
            file_count: 10,
            is_admin: true,
            my_role: "admin".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"Test Group\""));
        assert!(json.contains("\"is_admin\":true"));
    }
}
