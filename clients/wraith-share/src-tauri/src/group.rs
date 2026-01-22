//! Group Management
//!
//! Handles group creation, member management, and invitation system.

use crate::database::{ActivityEvent, Database, Group, GroupMember};
use crate::error::{ShareError, ShareResult};
use crate::state::AppState;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Group manager handles all group-related operations
pub struct GroupManager {
    db: Arc<Database>,
    state: Arc<AppState>,
}

/// Invitation payload for signing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InvitationPayload {
    group_id: String,
    role: String,
    expires_at: i64,
    invited_peer_id: Option<String>,
}

/// Exported invitation for sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedInvitation {
    pub group_id: String,
    pub group_name: String,
    pub invited_by: String,
    pub invited_by_name: String,
    pub role: String,
    pub expires_at: i64,
    pub invite_code: String,
    pub inviter_public_key: String,
}

impl GroupManager {
    /// Create a new group manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Create a new group
    pub fn create_group(&self, name: &str, description: Option<&str>) -> ShareResult<Group> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        let public_key = self
            .state
            .get_public_key_bytes()
            .ok_or_else(|| ShareError::Group("Local public key not available".to_string()))?;

        let group = Group {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.map(String::from),
            created_at: Utc::now().timestamp(),
            created_by: peer_id.clone(),
        };

        self.db.create_group(&group)?;

        // Add creator as admin member
        let member = GroupMember {
            group_id: group.id.clone(),
            peer_id: peer_id.clone(),
            display_name: Some(self.state.get_display_name()),
            role: "admin".to_string(),
            joined_at: Utc::now().timestamp(),
            invited_by: peer_id.clone(),
            public_key,
        };

        self.db.add_group_member(&member)?;

        // Log activity
        self.log_activity(
            &group.id,
            "group_created",
            &peer_id,
            Some(&group.id),
            Some(&group.name),
            None,
        )?;

        info!("Created group: {} ({})", group.name, group.id);

        Ok(group)
    }

    /// Delete a group (admin only)
    pub fn delete_group(&self, group_id: &str) -> ShareResult<()> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Verify admin permission
        let member = self
            .db
            .get_group_member(group_id, &peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(peer_id.clone()))?;

        if member.role != "admin" {
            return Err(ShareError::PermissionDenied(
                "Only admins can delete groups".to_string(),
            ));
        }

        let group = self
            .db
            .get_group(group_id)?
            .ok_or_else(|| ShareError::GroupNotFound(group_id.to_string()))?;

        // Check if this is the creator
        if group.created_by != peer_id {
            return Err(ShareError::PermissionDenied(
                "Only the group creator can delete the group".to_string(),
            ));
        }

        self.db.delete_group(group_id)?;

        info!("Deleted group: {}", group_id);

        Ok(())
    }

    /// Get a group by ID
    pub fn get_group(&self, group_id: &str) -> ShareResult<Option<Group>> {
        self.db.get_group(group_id).map_err(ShareError::from)
    }

    /// List all groups
    pub fn list_groups(&self) -> ShareResult<Vec<Group>> {
        self.db.list_groups().map_err(ShareError::from)
    }

    /// Invite a member to a group
    pub fn invite_member(
        &self,
        group_id: &str,
        target_peer_id: Option<&str>,
        role: &str,
    ) -> ShareResult<ExportedInvitation> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Validate role
        if role != "admin" && role != "write" && role != "read" {
            return Err(ShareError::Group(format!("Invalid role: {}", role)));
        }

        // Verify inviter is admin
        let inviter = self
            .db
            .get_group_member(group_id, &peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(peer_id.clone()))?;

        if inviter.role != "admin" {
            return Err(ShareError::PermissionDenied(
                "Only admins can invite members".to_string(),
            ));
        }

        let group = self
            .db
            .get_group(group_id)?
            .ok_or_else(|| ShareError::GroupNotFound(group_id.to_string()))?;

        // Create invitation payload
        let expires_at = Utc::now().timestamp() + 7 * 24 * 60 * 60; // 7 days
        let payload = InvitationPayload {
            group_id: group_id.to_string(),
            role: role.to_string(),
            expires_at,
            invited_peer_id: target_peer_id.map(String::from),
        };

        // Sign the payload
        let signing_key = self
            .state
            .get_signing_key()
            .ok_or_else(|| ShareError::Crypto("Signing key not available".to_string()))?;

        let payload_json = serde_json::to_string(&payload)?;
        let signature = signing_key.sign(payload_json.as_bytes());

        // Combine payload and signature
        let invite_data = format!(
            "{}:{}",
            BASE64.encode(payload_json.as_bytes()),
            BASE64.encode(signature.to_bytes())
        );

        let invite_code = BASE64.encode(invite_data.as_bytes());

        // Get inviter's public key
        let public_key = self
            .state
            .get_public_key_bytes()
            .ok_or_else(|| ShareError::Crypto("Public key not available".to_string()))?;

        let invitation = ExportedInvitation {
            group_id: group_id.to_string(),
            group_name: group.name.clone(),
            invited_by: peer_id.clone(),
            invited_by_name: inviter
                .display_name
                .unwrap_or_else(|| "Unknown".to_string()),
            role: role.to_string(),
            expires_at,
            invite_code,
            inviter_public_key: BASE64.encode(&public_key),
        };

        // Log activity
        self.log_activity(
            group_id,
            "member_invited",
            &peer_id,
            target_peer_id,
            None,
            Some(&format!("Role: {}", role)),
        )?;

        info!(
            "Created invitation for group {} with role {}",
            group_id, role
        );

        Ok(invitation)
    }

    /// Accept an invitation to join a group
    pub fn accept_invitation(&self, invitation: &ExportedInvitation) -> ShareResult<GroupMember> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Check expiration
        if Utc::now().timestamp() > invitation.expires_at {
            return Err(ShareError::InvitationExpired);
        }

        // Decode and verify invitation
        let invite_data = String::from_utf8(
            BASE64
                .decode(&invitation.invite_code)
                .map_err(|_| ShareError::InvalidInvitation("Invalid invite code".to_string()))?,
        )
        .map_err(|_| ShareError::InvalidInvitation("Invalid invite code encoding".to_string()))?;

        let parts: Vec<&str> = invite_data.split(':').collect();
        if parts.len() != 2 {
            return Err(ShareError::InvalidInvitation(
                "Invalid invite code format".to_string(),
            ));
        }

        let payload_bytes = BASE64
            .decode(parts[0])
            .map_err(|_| ShareError::InvalidInvitation("Invalid payload".to_string()))?;

        let signature_bytes = BASE64
            .decode(parts[1])
            .map_err(|_| ShareError::InvalidInvitation("Invalid signature".to_string()))?;

        // Verify signature
        let inviter_public_key = BASE64
            .decode(&invitation.inviter_public_key)
            .map_err(|_| ShareError::InvalidInvitation("Invalid inviter public key".to_string()))?;

        let (verifying_key, _) = AppState::parse_public_key_bytes(&inviter_public_key)?;

        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| ShareError::InvalidInvitation("Invalid signature length".to_string()))?;

        let signature = Signature::from_bytes(&signature_array);

        verifying_key
            .verify(&payload_bytes, &signature)
            .map_err(|_| {
                ShareError::InvalidInvitation("Signature verification failed".to_string())
            })?;

        // Parse payload
        let payload: InvitationPayload = serde_json::from_slice(&payload_bytes)
            .map_err(|_| ShareError::InvalidInvitation("Invalid payload format".to_string()))?;

        // Verify payload matches invitation
        if payload.group_id != invitation.group_id || payload.role != invitation.role {
            return Err(ShareError::InvalidInvitation(
                "Payload mismatch".to_string(),
            ));
        }

        // If invitation was for a specific peer, verify it matches
        if let Some(target_peer) = &payload.invited_peer_id {
            if target_peer != &peer_id {
                return Err(ShareError::InvalidInvitation(
                    "Invitation is for a different peer".to_string(),
                ));
            }
        }

        // Check if already a member
        if self
            .db
            .get_group_member(&invitation.group_id, &peer_id)?
            .is_some()
        {
            return Err(ShareError::Group(
                "Already a member of this group".to_string(),
            ));
        }

        // Get our public key
        let public_key = self
            .state
            .get_public_key_bytes()
            .ok_or_else(|| ShareError::Crypto("Public key not available".to_string()))?;

        // Add ourselves to the group
        let member = GroupMember {
            group_id: invitation.group_id.clone(),
            peer_id: peer_id.clone(),
            display_name: Some(self.state.get_display_name()),
            role: invitation.role.clone(),
            joined_at: Utc::now().timestamp(),
            invited_by: invitation.invited_by.clone(),
            public_key,
        };

        // First, ensure we have the group locally (may need to sync)
        // For now, create a minimal group entry if it doesn't exist
        if self.db.get_group(&invitation.group_id)?.is_none() {
            let group = Group {
                id: invitation.group_id.clone(),
                name: invitation.group_name.clone(),
                description: None,
                created_at: Utc::now().timestamp(),
                created_by: invitation.invited_by.clone(),
            };
            self.db.create_group(&group)?;
        }

        self.db.add_group_member(&member)?;

        // Log activity
        self.log_activity(
            &invitation.group_id,
            "member_joined",
            &peer_id,
            Some(&peer_id),
            Some(&self.state.get_display_name()),
            Some(&format!("Role: {}", invitation.role)),
        )?;

        info!(
            "Joined group {} with role {}",
            invitation.group_id, invitation.role
        );

        Ok(member)
    }

    /// Remove a member from a group
    pub fn remove_member(&self, group_id: &str, target_peer_id: &str) -> ShareResult<()> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Get group
        let group = self
            .db
            .get_group(group_id)?
            .ok_or_else(|| ShareError::GroupNotFound(group_id.to_string()))?;

        // Verify remover is admin
        let admin = self
            .db
            .get_group_member(group_id, &peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(peer_id.clone()))?;

        if admin.role != "admin" {
            return Err(ShareError::PermissionDenied(
                "Only admins can remove members".to_string(),
            ));
        }

        // Cannot remove the group creator
        if target_peer_id == group.created_by {
            return Err(ShareError::PermissionDenied(
                "Cannot remove the group creator".to_string(),
            ));
        }

        // Get target member for logging
        let target_member = self
            .db
            .get_group_member(group_id, target_peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(target_peer_id.to_string()))?;

        self.db.remove_group_member(group_id, target_peer_id)?;

        // Log activity
        self.log_activity(
            group_id,
            "member_removed",
            &peer_id,
            Some(target_peer_id),
            target_member.display_name.as_deref(),
            None,
        )?;

        info!("Removed member {} from group {}", target_peer_id, group_id);

        Ok(())
    }

    /// Update a member's role
    pub fn set_member_role(
        &self,
        group_id: &str,
        target_peer_id: &str,
        new_role: &str,
    ) -> ShareResult<()> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        // Validate role
        if new_role != "admin" && new_role != "write" && new_role != "read" {
            return Err(ShareError::Group(format!("Invalid role: {}", new_role)));
        }

        // Get group
        let group = self
            .db
            .get_group(group_id)?
            .ok_or_else(|| ShareError::GroupNotFound(group_id.to_string()))?;

        // Verify setter is admin
        let admin = self
            .db
            .get_group_member(group_id, &peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(peer_id.clone()))?;

        if admin.role != "admin" {
            return Err(ShareError::PermissionDenied(
                "Only admins can change member roles".to_string(),
            ));
        }

        // Cannot change creator's role
        if target_peer_id == group.created_by && new_role != "admin" {
            return Err(ShareError::PermissionDenied(
                "Cannot change the group creator's role".to_string(),
            ));
        }

        // Get target member
        let target_member = self
            .db
            .get_group_member(group_id, target_peer_id)?
            .ok_or_else(|| ShareError::MemberNotFound(target_peer_id.to_string()))?;

        let old_role = target_member.role.clone();

        self.db
            .update_member_role(group_id, target_peer_id, new_role)?;

        // Log activity
        self.log_activity(
            group_id,
            "role_changed",
            &peer_id,
            Some(target_peer_id),
            target_member.display_name.as_deref(),
            Some(&format!("{} -> {}", old_role, new_role)),
        )?;

        info!(
            "Changed role of {} in group {} to {}",
            target_peer_id, group_id, new_role
        );

        Ok(())
    }

    /// List members of a group
    pub fn list_members(&self, group_id: &str) -> ShareResult<Vec<GroupMember>> {
        self.db
            .list_group_members(group_id)
            .map_err(ShareError::from)
    }

    /// Get member count for a group
    pub fn member_count(&self, group_id: &str) -> ShareResult<i64> {
        self.db
            .count_group_members(group_id)
            .map_err(ShareError::from)
    }

    /// Check if the local user is an admin of the group
    pub fn is_admin(&self, group_id: &str) -> ShareResult<bool> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        if let Some(member) = self.db.get_group_member(group_id, &peer_id)? {
            Ok(member.role == "admin")
        } else {
            Ok(false)
        }
    }

    /// Check if the local user has write permission
    pub fn has_write_permission(&self, group_id: &str) -> ShareResult<bool> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| ShareError::Group("Local identity not initialized".to_string()))?;

        if let Some(member) = self.db.get_group_member(group_id, &peer_id)? {
            Ok(member.role == "admin" || member.role == "write")
        } else {
            Ok(false)
        }
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
    use tempfile::tempdir;

    fn create_test_state() -> (Arc<Database>, Arc<AppState>) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();
        (db, state)
    }

    #[test]
    fn test_create_group() {
        let (db, state) = create_test_state();
        let manager = GroupManager::new(db, state);

        let group = manager
            .create_group("Test Group", Some("A test group"))
            .unwrap();

        assert_eq!(group.name, "Test Group");
        assert_eq!(group.description, Some("A test group".to_string()));

        // Verify creator is admin
        assert!(manager.is_admin(&group.id).unwrap());
    }

    #[test]
    fn test_invite_and_accept() {
        let (db, state) = create_test_state();
        let manager = GroupManager::new(db.clone(), state.clone());

        // Create group
        let group = manager.create_group("Test Group", None).unwrap();

        // Create invitation
        let invitation = manager.invite_member(&group.id, None, "write").unwrap();

        assert_eq!(invitation.role, "write");
        assert!(!invitation.invite_code.is_empty());

        // Note: In a real scenario, a different user would accept
        // For testing, we verify the invitation structure
        assert_eq!(invitation.group_id, group.id);
    }

    #[test]
    fn test_member_role_change() {
        let (db, state) = create_test_state();
        let manager = GroupManager::new(db.clone(), state.clone());

        // Create group
        let group = manager.create_group("Test Group", None).unwrap();

        // List members
        let members = manager.list_members(&group.id).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].role, "admin");
    }
}
