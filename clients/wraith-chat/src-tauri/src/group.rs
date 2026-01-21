// Group Messaging with Sender Keys Protocol (Sprint 17.7)
//
// Implements efficient group encryption using Sender Keys, where each member
// maintains their own sender key that encrypts O(1) instead of O(n) pairwise.
//
// Based on Signal's Sender Keys specification.

use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use thiserror::Error;

/// Maximum members in a group
pub const MAX_GROUP_MEMBERS: usize = 1000;

/// Key rotation interval in days
pub const KEY_ROTATION_DAYS: u64 = 7;

/// Group messaging errors
#[derive(Debug, Error)]
pub enum GroupError {
    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Member not found: {0}")]
    MemberNotFound(String),

    #[error("Not authorized: {0}")]
    NotAuthorized(String),

    #[error("Group full (max {0} members)")]
    GroupFull(usize),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key derivation failed")]
    KdfFailed,

    #[error("Invalid sender key")]
    InvalidSenderKey,

    #[error("Serialization failed")]
    SerializationFailed,

    #[error("Deserialization failed")]
    DeserializationFailed,

    #[error("Stale key generation: expected >= {expected}, got {actual}")]
    StaleKeyGeneration { expected: u32, actual: u32 },
}

/// Role in a group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupRole {
    /// Administrator - can add/remove members, change settings
    Admin,
    /// Regular member - can send/receive messages
    Member,
}

/// Group member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    /// Peer ID of the member
    pub peer_id: String,
    /// Display name
    pub display_name: Option<String>,
    /// Role in the group
    pub role: GroupRole,
    /// When the member joined (Unix timestamp)
    pub joined_at: i64,
    /// Current sender key generation number
    pub key_generation: u32,
}

/// Sender key state for a single sender
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKeyState {
    /// Key generation number (incremented on rotation)
    pub generation: u32,
    /// Chain key for deriving message keys
    #[serde(with = "serde_bytes")]
    chain_key: Vec<u8>,
    /// Current iteration in the chain
    pub iteration: u32,
    /// Signing key (Ed25519 public key) for authentication
    #[serde(with = "serde_bytes")]
    pub signing_key: Vec<u8>,
}

impl SenderKeyState {
    /// Create a new sender key state
    pub fn new() -> Self {
        let mut chain_key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut chain_key);

        let mut signing_key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut signing_key);

        Self {
            generation: 0,
            chain_key,
            iteration: 0,
            signing_key,
        }
    }

    /// Rotate the sender key (new generation)
    pub fn rotate(&mut self) {
        let mut new_chain_key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut new_chain_key);
        self.chain_key = new_chain_key;
        self.generation += 1;
        self.iteration = 0;
    }

    /// Derive message key and advance the chain
    pub fn derive_message_key(&mut self) -> Result<Vec<u8>, GroupError> {
        let hkdf = Hkdf::<Sha256>::new(None, &self.chain_key);
        let mut output = vec![0u8; 64];
        hkdf.expand(b"sender-key-message-key", &mut output)
            .map_err(|_| GroupError::KdfFailed)?;

        // New chain key is first 32 bytes
        self.chain_key = output[..32].to_vec();
        // Message key is last 32 bytes
        let message_key = output[32..].to_vec();
        self.iteration += 1;

        Ok(message_key)
    }

    /// Get the distribution message for sharing this key
    pub fn to_distribution(&self) -> SenderKeyDistribution {
        SenderKeyDistribution {
            generation: self.generation,
            chain_key: self.chain_key.clone(),
            iteration: self.iteration,
            signing_key: self.signing_key.clone(),
        }
    }

    /// Create from a distribution message
    pub fn from_distribution(dist: &SenderKeyDistribution) -> Self {
        Self {
            generation: dist.generation,
            chain_key: dist.chain_key.clone(),
            iteration: dist.iteration,
            signing_key: dist.signing_key.clone(),
        }
    }
}

impl Default for SenderKeyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Sender key distribution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKeyDistribution {
    /// Key generation number
    pub generation: u32,
    /// Chain key
    #[serde(with = "serde_bytes")]
    pub chain_key: Vec<u8>,
    /// Current iteration
    pub iteration: u32,
    /// Signing key
    #[serde(with = "serde_bytes")]
    pub signing_key: Vec<u8>,
}

/// Group session for managing group encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSession {
    /// Group identifier
    pub group_id: String,
    /// Group name
    pub name: String,
    /// Group description
    pub description: Option<String>,
    /// Group avatar (encoded)
    #[serde(with = "serde_bytes_option")]
    pub avatar: Option<Vec<u8>>,
    /// Our sender key for this group
    my_sender_key: SenderKeyState,
    /// Sender keys from other members (peer_id -> SenderKeyState)
    member_sender_keys: HashMap<String, SenderKeyState>,
    /// Group members
    members: HashMap<String, GroupMember>,
    /// When the group was created (Unix timestamp)
    pub created_at: i64,
    /// When keys were last rotated (Unix timestamp)
    pub last_key_rotation: i64,
    /// Our peer ID (for identifying ourselves)
    our_peer_id: String,
}

impl GroupSession {
    /// Create a new group session as admin
    pub fn new(
        group_id: String,
        name: String,
        our_peer_id: String,
        our_display_name: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        let mut members = HashMap::new();
        members.insert(
            our_peer_id.clone(),
            GroupMember {
                peer_id: our_peer_id.clone(),
                display_name: our_display_name,
                role: GroupRole::Admin,
                joined_at: now,
                key_generation: 0,
            },
        );

        Self {
            group_id,
            name,
            description: None,
            avatar: None,
            my_sender_key: SenderKeyState::new(),
            member_sender_keys: HashMap::new(),
            members,
            created_at: now,
            last_key_rotation: now,
            our_peer_id,
        }
    }

    /// Join an existing group
    pub fn join(
        group_id: String,
        name: String,
        our_peer_id: String,
        creator_peer_id: String,
        creator_sender_key: SenderKeyDistribution,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        let mut members = HashMap::new();
        members.insert(
            creator_peer_id.clone(),
            GroupMember {
                peer_id: creator_peer_id.clone(),
                display_name: None,
                role: GroupRole::Admin,
                joined_at: now,
                key_generation: creator_sender_key.generation,
            },
        );

        let mut member_sender_keys = HashMap::new();
        member_sender_keys.insert(
            creator_peer_id,
            SenderKeyState::from_distribution(&creator_sender_key),
        );

        Self {
            group_id,
            name,
            description: None,
            avatar: None,
            my_sender_key: SenderKeyState::new(),
            member_sender_keys,
            members,
            created_at: now,
            last_key_rotation: now,
            our_peer_id,
        }
    }

    /// Get our sender key distribution for sharing
    pub fn get_my_distribution(&self) -> SenderKeyDistribution {
        self.my_sender_key.to_distribution()
    }

    /// Add a member's sender key
    pub fn add_member_key(
        &mut self,
        peer_id: &str,
        distribution: SenderKeyDistribution,
        display_name: Option<String>,
        role: GroupRole,
    ) -> Result<(), GroupError> {
        if self.members.len() >= MAX_GROUP_MEMBERS {
            return Err(GroupError::GroupFull(MAX_GROUP_MEMBERS));
        }

        let now = chrono::Utc::now().timestamp();

        // Add or update member
        self.members.insert(
            peer_id.to_string(),
            GroupMember {
                peer_id: peer_id.to_string(),
                display_name,
                role,
                joined_at: now,
                key_generation: distribution.generation,
            },
        );

        // Store their sender key
        self.member_sender_keys.insert(
            peer_id.to_string(),
            SenderKeyState::from_distribution(&distribution),
        );

        Ok(())
    }

    /// Remove a member
    ///
    /// This will also rotate our sender key for forward secrecy.
    pub fn remove_member(&mut self, peer_id: &str) -> Result<(), GroupError> {
        if !self.members.contains_key(peer_id) {
            return Err(GroupError::MemberNotFound(peer_id.to_string()));
        }

        self.members.remove(peer_id);
        self.member_sender_keys.remove(peer_id);

        // Rotate our key for forward secrecy
        self.rotate_sender_key();

        Ok(())
    }

    /// Rotate our sender key
    pub fn rotate_sender_key(&mut self) {
        self.my_sender_key.rotate();
        self.last_key_rotation = chrono::Utc::now().timestamp();

        // Update our own member entry
        if let Some(member) = self.members.get_mut(&self.our_peer_id) {
            member.key_generation = self.my_sender_key.generation;
        }
    }

    /// Check if key rotation is needed (based on time)
    pub fn needs_key_rotation(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        let days_since_rotation = (now - self.last_key_rotation) / 86400;
        days_since_rotation >= KEY_ROTATION_DAYS as i64
    }

    /// Encrypt a message for the group
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<GroupEncryptedMessage, GroupError> {
        // Derive message key from our sender key
        let message_key = self.my_sender_key.derive_message_key()?;

        // Encrypt with ChaCha20-Poly1305
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = ChaCha20Poly1305::new_from_slice(&message_key)
            .map_err(|e| GroupError::EncryptionFailed(e.to_string()))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| GroupError::EncryptionFailed(e.to_string()))?;

        Ok(GroupEncryptedMessage {
            group_id: self.group_id.clone(),
            sender_peer_id: self.our_peer_id.clone(),
            key_generation: self.my_sender_key.generation,
            iteration: self.my_sender_key.iteration - 1, // Already advanced
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        })
    }

    /// Decrypt a message from a group member
    pub fn decrypt(&mut self, message: &GroupEncryptedMessage) -> Result<Vec<u8>, GroupError> {
        // Get the sender's key
        let sender_key = self
            .member_sender_keys
            .get_mut(&message.sender_peer_id)
            .ok_or_else(|| GroupError::MemberNotFound(message.sender_peer_id.clone()))?;

        // Check key generation
        if message.key_generation < sender_key.generation {
            return Err(GroupError::StaleKeyGeneration {
                expected: sender_key.generation,
                actual: message.key_generation,
            });
        }

        // If generation is newer, we need to update (we missed a rotation)
        if message.key_generation > sender_key.generation {
            return Err(GroupError::InvalidSenderKey);
        }

        // Fast-forward chain if needed
        while sender_key.iteration < message.iteration {
            let _ = sender_key.derive_message_key()?;
        }

        // Derive the message key
        let message_key = sender_key.derive_message_key()?;

        // Decrypt
        let cipher = ChaCha20Poly1305::new_from_slice(&message_key)
            .map_err(|e| GroupError::DecryptionFailed(e.to_string()))?;

        let nonce = Nonce::from_slice(&message.nonce);

        cipher
            .decrypt(nonce, message.ciphertext.as_slice())
            .map_err(|e| GroupError::DecryptionFailed(e.to_string()))
    }

    /// Check if a peer is an admin
    pub fn is_admin(&self, peer_id: &str) -> bool {
        self.members
            .get(peer_id)
            .is_some_and(|m| m.role == GroupRole::Admin)
    }

    /// Check if we are an admin
    pub fn am_i_admin(&self) -> bool {
        self.is_admin(&self.our_peer_id)
    }

    /// Get list of members
    pub fn get_members(&self) -> Vec<&GroupMember> {
        self.members.values().collect()
    }

    /// Get member info
    pub fn get_member(&self, peer_id: &str) -> Option<&GroupMember> {
        self.members.get(peer_id)
    }

    /// Promote a member to admin
    pub fn promote_to_admin(&mut self, peer_id: &str) -> Result<(), GroupError> {
        let member = self
            .members
            .get_mut(peer_id)
            .ok_or_else(|| GroupError::MemberNotFound(peer_id.to_string()))?;
        member.role = GroupRole::Admin;
        Ok(())
    }

    /// Demote an admin to member
    pub fn demote_from_admin(&mut self, peer_id: &str) -> Result<(), GroupError> {
        // Can't demote the last admin
        let admin_count = self
            .members
            .values()
            .filter(|m| m.role == GroupRole::Admin)
            .count();
        if admin_count <= 1 && self.is_admin(peer_id) {
            return Err(GroupError::NotAuthorized(
                "Cannot demote the last admin".to_string(),
            ));
        }

        let member = self
            .members
            .get_mut(peer_id)
            .ok_or_else(|| GroupError::MemberNotFound(peer_id.to_string()))?;
        member.role = GroupRole::Member;
        Ok(())
    }

    /// Update group settings
    pub fn update_settings(
        &mut self,
        name: Option<String>,
        description: Option<String>,
        avatar: Option<Vec<u8>>,
    ) {
        if let Some(n) = name {
            self.name = n;
        }
        if description.is_some() {
            self.description = description;
        }
        if avatar.is_some() {
            self.avatar = avatar;
        }
    }

    /// Get group info
    pub fn get_info(&self) -> GroupInfo {
        GroupInfo {
            group_id: self.group_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            member_count: self.members.len(),
            created_at: self.created_at,
            am_i_admin: self.am_i_admin(),
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, GroupError> {
        serde_json::to_string(self).map_err(|_| GroupError::SerializationFailed)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, GroupError> {
        serde_json::from_str(json).map_err(|_| GroupError::DeserializationFailed)
    }
}

/// Encrypted message for group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupEncryptedMessage {
    /// Group identifier
    pub group_id: String,
    /// Sender peer ID
    pub sender_peer_id: String,
    /// Sender key generation
    pub key_generation: u32,
    /// Chain iteration
    pub iteration: u32,
    /// Nonce for AEAD
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Ciphertext
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}

/// Group information (public)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub group_id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: usize,
    pub created_at: i64,
    pub am_i_admin: bool,
}

/// Group session manager
pub struct GroupSessionManager {
    /// Active group sessions by group ID
    sessions: HashMap<String, GroupSession>,
}

impl GroupSessionManager {
    /// Create a new group session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new group
    pub fn create_group(
        &mut self,
        name: String,
        our_peer_id: String,
        our_display_name: Option<String>,
    ) -> GroupInfo {
        let group_id = uuid::Uuid::new_v4().to_string();
        let session = GroupSession::new(group_id.clone(), name, our_peer_id, our_display_name);
        let info = session.get_info();
        self.sessions.insert(group_id, session);
        info
    }

    /// Get a group session
    pub fn get_session(&self, group_id: &str) -> Option<&GroupSession> {
        self.sessions.get(group_id)
    }

    /// Get a mutable group session
    pub fn get_session_mut(&mut self, group_id: &str) -> Option<&mut GroupSession> {
        self.sessions.get_mut(group_id)
    }

    /// Add a group session
    pub fn add_session(&mut self, session: GroupSession) {
        self.sessions.insert(session.group_id.clone(), session);
    }

    /// Remove a group session
    pub fn remove_session(&mut self, group_id: &str) -> Option<GroupSession> {
        self.sessions.remove(group_id)
    }

    /// List all groups
    pub fn list_groups(&self) -> Vec<GroupInfo> {
        self.sessions.values().map(|s| s.get_info()).collect()
    }

    /// Check and rotate keys for all groups that need it
    pub fn rotate_stale_keys(&mut self) -> Vec<String> {
        let mut rotated = Vec::new();
        for (group_id, session) in &mut self.sessions {
            if session.needs_key_rotation() {
                session.rotate_sender_key();
                rotated.push(group_id.clone());
            }
        }
        rotated
    }
}

impl Default for GroupSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

mod serde_bytes_option {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(b) => serializer.serialize_some(&hex::encode(b)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        opt.map(|s| hex::decode(&s).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sender_key_creation() {
        let key = SenderKeyState::new();
        assert_eq!(key.generation, 0);
        assert_eq!(key.iteration, 0);
        assert_eq!(key.chain_key.len(), 32);
    }

    #[test]
    fn test_sender_key_rotation() {
        let mut key = SenderKeyState::new();
        let old_chain = key.chain_key.clone();

        key.rotate();

        assert_eq!(key.generation, 1);
        assert_eq!(key.iteration, 0);
        assert_ne!(key.chain_key, old_chain);
    }

    #[test]
    fn test_message_key_derivation() {
        let mut key = SenderKeyState::new();

        let msg_key1 = key.derive_message_key().unwrap();
        let msg_key2 = key.derive_message_key().unwrap();

        assert_eq!(key.iteration, 2);
        assert_ne!(msg_key1, msg_key2);
    }

    #[test]
    fn test_group_session_creation() {
        let session = GroupSession::new(
            "test-group".to_string(),
            "Test Group".to_string(),
            "peer-123".to_string(),
            Some("Alice".to_string()),
        );

        assert_eq!(session.group_id, "test-group");
        assert_eq!(session.name, "Test Group");
        assert!(session.am_i_admin());
        assert_eq!(session.get_members().len(), 1);
    }

    #[test]
    fn test_group_encrypt_decrypt() {
        let mut alice_session = GroupSession::new(
            "test-group".to_string(),
            "Test Group".to_string(),
            "alice".to_string(),
            Some("Alice".to_string()),
        );

        // Bob joins the group
        let alice_dist = alice_session.get_my_distribution();
        let mut bob_session = GroupSession::join(
            "test-group".to_string(),
            "Test Group".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            alice_dist,
        );

        // Alice adds Bob
        let bob_dist = bob_session.get_my_distribution();
        alice_session
            .add_member_key("bob", bob_dist, Some("Bob".to_string()), GroupRole::Member)
            .unwrap();

        // Bob sends a message
        let plaintext = b"Hello from Bob!";
        let encrypted = bob_session.encrypt(plaintext).unwrap();

        // Alice decrypts
        let decrypted = alice_session.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_group_member_removal() {
        let mut session = GroupSession::new(
            "test-group".to_string(),
            "Test Group".to_string(),
            "admin".to_string(),
            None,
        );

        // Add a member
        let member_key = SenderKeyState::new();
        session
            .add_member_key(
                "member1",
                member_key.to_distribution(),
                None,
                GroupRole::Member,
            )
            .unwrap();

        assert_eq!(session.get_members().len(), 2);

        let old_generation = session.my_sender_key.generation;

        // Remove member
        session.remove_member("member1").unwrap();

        assert_eq!(session.get_members().len(), 1);
        // Key should be rotated
        assert_eq!(session.my_sender_key.generation, old_generation + 1);
    }

    #[test]
    fn test_group_session_manager() {
        let mut manager = GroupSessionManager::new();

        let info = manager.create_group(
            "Test Group".to_string(),
            "peer-123".to_string(),
            Some("Alice".to_string()),
        );

        assert!(!info.group_id.is_empty());
        assert_eq!(info.name, "Test Group");

        let groups = manager.list_groups();
        assert_eq!(groups.len(), 1);
    }
}
