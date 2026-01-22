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

    // ==================== Sprint 18.2 Edge Case Tests ====================

    /// Test 1: Message ordering with concurrent sends
    /// Tests that messages from multiple senders maintain correct ordering
    #[test]
    fn test_message_ordering_with_concurrent_sends() {
        // Create a group with multiple members
        let mut alice_session = GroupSession::new(
            "ordering-test".to_string(),
            "Ordering Test".to_string(),
            "alice".to_string(),
            Some("Alice".to_string()),
        );

        // Bob joins
        let alice_dist = alice_session.get_my_distribution();
        let mut bob_session = GroupSession::join(
            "ordering-test".to_string(),
            "Ordering Test".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            alice_dist.clone(),
        );

        // Charlie joins
        let mut charlie_session = GroupSession::join(
            "ordering-test".to_string(),
            "Ordering Test".to_string(),
            "charlie".to_string(),
            "alice".to_string(),
            alice_dist,
        );

        // Exchange keys
        let bob_dist = bob_session.get_my_distribution();
        let charlie_dist = charlie_session.get_my_distribution();

        alice_session
            .add_member_key(
                "bob",
                bob_dist.clone(),
                Some("Bob".to_string()),
                GroupRole::Member,
            )
            .unwrap();
        alice_session
            .add_member_key(
                "charlie",
                charlie_dist.clone(),
                Some("Charlie".to_string()),
                GroupRole::Member,
            )
            .unwrap();

        bob_session
            .add_member_key(
                "charlie",
                charlie_dist,
                Some("Charlie".to_string()),
                GroupRole::Member,
            )
            .unwrap();
        charlie_session
            .add_member_key("bob", bob_dist, Some("Bob".to_string()), GroupRole::Member)
            .unwrap();

        // Simulate concurrent message sends
        let messages: Vec<(String, GroupEncryptedMessage)> = vec![
            ("bob", bob_session.encrypt(b"Message 1 from Bob").unwrap()),
            (
                "charlie",
                charlie_session.encrypt(b"Message 2 from Charlie").unwrap(),
            ),
            ("bob", bob_session.encrypt(b"Message 3 from Bob").unwrap()),
            (
                "charlie",
                charlie_session.encrypt(b"Message 4 from Charlie").unwrap(),
            ),
            ("bob", bob_session.encrypt(b"Message 5 from Bob").unwrap()),
        ]
        .into_iter()
        .map(|(sender, msg)| (sender.to_string(), msg))
        .collect();

        // Alice receives all messages - each sender's messages should maintain their iteration order
        for (_, encrypted) in &messages {
            let decrypted = alice_session.decrypt(encrypted);
            assert!(
                decrypted.is_ok(),
                "Should decrypt all messages successfully"
            );
        }

        // Verify Bob's messages maintain iteration sequence
        assert_eq!(messages[0].1.iteration, 0); // Bob's first message
        assert_eq!(messages[2].1.iteration, 1); // Bob's second message
        assert_eq!(messages[4].1.iteration, 2); // Bob's third message

        // Verify Charlie's messages maintain iteration sequence
        assert_eq!(messages[1].1.iteration, 0); // Charlie's first message
        assert_eq!(messages[3].1.iteration, 1); // Charlie's second message
    }

    /// Test 2: Sender key rotation during active conversation
    /// Tests key rotation while messages are being exchanged
    #[test]
    fn test_sender_key_rotation_during_conversation() {
        let mut alice_session = GroupSession::new(
            "rotation-test".to_string(),
            "Rotation Test".to_string(),
            "alice".to_string(),
            None,
        );

        let alice_dist = alice_session.get_my_distribution();
        let mut bob_session = GroupSession::join(
            "rotation-test".to_string(),
            "Rotation Test".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            alice_dist,
        );

        let bob_dist = bob_session.get_my_distribution();
        alice_session
            .add_member_key("bob", bob_dist, None, GroupRole::Member)
            .unwrap();

        // Send some messages before rotation
        let msg1 = bob_session.encrypt(b"Message before rotation").unwrap();
        let decrypted1 = alice_session.decrypt(&msg1).unwrap();
        assert_eq!(decrypted1, b"Message before rotation");

        let old_generation = bob_session.my_sender_key.generation;

        // Bob rotates his sender key
        bob_session.rotate_sender_key();
        assert_eq!(bob_session.my_sender_key.generation, old_generation + 1);

        // Alice needs Bob's new key distribution
        let new_bob_dist = bob_session.get_my_distribution();
        alice_session
            .add_member_key("bob", new_bob_dist, None, GroupRole::Member)
            .unwrap();

        // Send message after rotation
        let msg2 = bob_session.encrypt(b"Message after rotation").unwrap();
        assert_eq!(msg2.key_generation, old_generation + 1);

        let decrypted2 = alice_session.decrypt(&msg2).unwrap();
        assert_eq!(decrypted2, b"Message after rotation");

        // Verify old message with old key generation fails (stale key)
        // Create a stale message manually
        let stale_msg = GroupEncryptedMessage {
            group_id: msg2.group_id.clone(),
            sender_peer_id: msg2.sender_peer_id.clone(),
            key_generation: old_generation, // Old generation
            iteration: msg2.iteration,
            nonce: msg2.nonce.clone(),
            ciphertext: msg2.ciphertext.clone(),
        };

        let result = alice_session.decrypt(&stale_msg);
        assert!(result.is_err());
        if let Err(GroupError::StaleKeyGeneration { expected, actual }) = result {
            assert_eq!(expected, old_generation + 1);
            assert_eq!(actual, old_generation);
        } else {
            panic!("Expected StaleKeyGeneration error");
        }
    }

    /// Test 3: Member join during key distribution
    /// Tests handling of a new member joining while keys are being distributed
    #[test]
    fn test_member_join_during_key_distribution() {
        let mut alice_session = GroupSession::new(
            "join-test".to_string(),
            "Join Test".to_string(),
            "alice".to_string(),
            None,
        );

        // Bob joins first
        let alice_dist = alice_session.get_my_distribution();
        let mut bob_session = GroupSession::join(
            "join-test".to_string(),
            "Join Test".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            alice_dist.clone(),
        );

        let bob_dist = bob_session.get_my_distribution();
        alice_session
            .add_member_key("bob", bob_dist.clone(), None, GroupRole::Member)
            .unwrap();

        // Charlie joins while Bob's key is being distributed
        let mut charlie_session = GroupSession::join(
            "join-test".to_string(),
            "Join Test".to_string(),
            "charlie".to_string(),
            "alice".to_string(),
            alice_dist,
        );

        // Charlie needs Bob's key too
        charlie_session
            .add_member_key("bob", bob_dist, None, GroupRole::Member)
            .unwrap();

        let charlie_dist = charlie_session.get_my_distribution();
        alice_session
            .add_member_key("charlie", charlie_dist.clone(), None, GroupRole::Member)
            .unwrap();
        bob_session
            .add_member_key("charlie", charlie_dist, None, GroupRole::Member)
            .unwrap();

        // Now all members should be able to communicate
        let msg = bob_session.encrypt(b"Hello everyone!").unwrap();
        let decrypted_alice = alice_session.decrypt(&msg).unwrap();
        let decrypted_charlie = charlie_session.decrypt(&msg).unwrap();

        assert_eq!(decrypted_alice, b"Hello everyone!");
        assert_eq!(decrypted_charlie, b"Hello everyone!");
    }

    /// Test 4: Offline member key catch-up
    /// Tests handling of messages from a sender whose key iterations have advanced
    #[test]
    fn test_offline_member_key_catchup() {
        let mut alice_session = GroupSession::new(
            "catchup-test".to_string(),
            "Catchup Test".to_string(),
            "alice".to_string(),
            None,
        );

        let alice_dist = alice_session.get_my_distribution();
        let mut bob_session = GroupSession::join(
            "catchup-test".to_string(),
            "Catchup Test".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            alice_dist,
        );

        let bob_dist = bob_session.get_my_distribution();
        alice_session
            .add_member_key("bob", bob_dist, None, GroupRole::Member)
            .unwrap();

        // Bob sends multiple messages while Alice is "offline"
        // (simulated by not decrypting them immediately)
        let _msg1 = bob_session.encrypt(b"Message 1").unwrap();
        let _msg2 = bob_session.encrypt(b"Message 2").unwrap();
        let _msg3 = bob_session.encrypt(b"Message 3").unwrap();
        let _msg4 = bob_session.encrypt(b"Message 4").unwrap();
        let msg5 = bob_session.encrypt(b"Message 5").unwrap();

        // Alice comes back online and receives only message 5
        // The chain should fast-forward to catch up
        let decrypted = alice_session.decrypt(&msg5).unwrap();
        assert_eq!(decrypted, b"Message 5");

        // Alice's copy of Bob's sender key should have advanced
        let bob_key = alice_session.member_sender_keys.get("bob").unwrap();
        assert_eq!(bob_key.iteration, 5); // After processing msg5 (iterations 0-4)
    }

    /// Test 5: Large group (100+ members) simulation
    /// Tests performance and correctness with many members
    #[test]
    fn test_large_group_simulation() {
        let mut admin_session = GroupSession::new(
            "large-group".to_string(),
            "Large Group".to_string(),
            "admin".to_string(),
            Some("Admin".to_string()),
        );

        let admin_dist = admin_session.get_my_distribution();

        // Add 100 members
        let mut member_sessions: Vec<GroupSession> = Vec::new();
        for i in 0..100 {
            let peer_id = format!("member-{}", i);
            let member_session = GroupSession::join(
                "large-group".to_string(),
                "Large Group".to_string(),
                peer_id.clone(),
                "admin".to_string(),
                admin_dist.clone(),
            );

            let member_dist = member_session.get_my_distribution();
            admin_session
                .add_member_key(
                    &peer_id,
                    member_dist.clone(),
                    Some(format!("Member {}", i)),
                    GroupRole::Member,
                )
                .unwrap();

            // Each member gets the admin's key (already have from join)
            // Add to our collection
            member_sessions.push(member_session);
        }

        // Verify member count (admin + 100 members)
        assert_eq!(admin_session.get_members().len(), 101);

        // Admin sends a message
        let admin_msg = admin_session.encrypt(b"Hello to all 100 members!").unwrap();

        // All members should be able to decrypt
        for (i, member_session) in member_sessions.iter_mut().enumerate() {
            let decrypted = member_session.decrypt(&admin_msg);
            assert!(
                decrypted.is_ok(),
                "Member {} should decrypt admin message",
                i
            );
            assert_eq!(decrypted.unwrap(), b"Hello to all 100 members!");
        }

        // Test that we cannot exceed MAX_GROUP_MEMBERS
        let result = admin_session.add_member_key(
            "member-overflow",
            SenderKeyState::new().to_distribution(),
            None,
            GroupRole::Member,
        );
        // Should still succeed since we have 101 < 1000 (MAX_GROUP_MEMBERS)
        assert!(result.is_ok());

        // Verify group info
        let info = admin_session.get_info();
        assert_eq!(info.member_count, 102);
        assert!(info.am_i_admin);
    }

    /// Test 6: Admin role transfer
    /// Tests promoting a member to admin and demoting the original admin
    #[test]
    fn test_admin_role_transfer() {
        let mut admin_session = GroupSession::new(
            "admin-transfer".to_string(),
            "Admin Transfer Test".to_string(),
            "original-admin".to_string(),
            Some("Original Admin".to_string()),
        );

        // Verify original admin status
        assert!(admin_session.am_i_admin());
        assert!(admin_session.is_admin("original-admin"));

        // Add a member
        let member_key = SenderKeyState::new();
        admin_session
            .add_member_key(
                "new-admin",
                member_key.to_distribution(),
                Some("New Admin".to_string()),
                GroupRole::Member,
            )
            .unwrap();

        // Verify member is not admin initially
        assert!(!admin_session.is_admin("new-admin"));

        // Promote member to admin
        admin_session.promote_to_admin("new-admin").unwrap();
        assert!(admin_session.is_admin("new-admin"));

        // Now we have 2 admins
        let members = admin_session.get_members();
        let admin_count = members
            .iter()
            .filter(|m| m.role == GroupRole::Admin)
            .count();
        assert_eq!(admin_count, 2);

        // Demote original admin (now that we have another admin)
        admin_session.demote_from_admin("original-admin").unwrap();
        assert!(!admin_session.is_admin("original-admin"));
        assert!(!admin_session.am_i_admin()); // We are original-admin

        // Verify only one admin now
        let members = admin_session.get_members();
        let admins: Vec<_> = members
            .iter()
            .filter(|m| m.role == GroupRole::Admin)
            .collect();
        assert_eq!(admins.len(), 1);
        assert_eq!(admins[0].peer_id, "new-admin");
    }

    /// Test 7: Cannot demote last admin
    /// Tests that the last admin cannot be demoted
    #[test]
    fn test_cannot_demote_last_admin() {
        let mut session = GroupSession::new(
            "last-admin".to_string(),
            "Last Admin Test".to_string(),
            "sole-admin".to_string(),
            None,
        );

        // Add a regular member
        let member_key = SenderKeyState::new();
        session
            .add_member_key(
                "member",
                member_key.to_distribution(),
                None,
                GroupRole::Member,
            )
            .unwrap();

        // Try to demote the only admin
        let result = session.demote_from_admin("sole-admin");
        assert!(result.is_err());

        if let Err(GroupError::NotAuthorized(msg)) = result {
            assert!(msg.contains("last admin"));
        } else {
            panic!("Expected NotAuthorized error");
        }

        // Admin should still be admin
        assert!(session.is_admin("sole-admin"));
    }

    /// Test 8: Group settings update
    /// Tests updating group name, description, and avatar
    #[test]
    fn test_group_settings_update() {
        let mut session = GroupSession::new(
            "settings-test".to_string(),
            "Original Name".to_string(),
            "admin".to_string(),
            None,
        );

        assert_eq!(session.name, "Original Name");
        assert!(session.description.is_none());
        assert!(session.avatar.is_none());

        // Update name only
        session.update_settings(Some("New Name".to_string()), None, None);
        assert_eq!(session.name, "New Name");

        // Update description
        session.update_settings(None, Some("Group description".to_string()), None);
        assert_eq!(session.description, Some("Group description".to_string()));

        // Update avatar
        let avatar_data = vec![0u8, 1, 2, 3, 4, 5];
        session.update_settings(None, None, Some(avatar_data.clone()));
        assert_eq!(session.avatar, Some(avatar_data));

        // Update all at once
        session.update_settings(
            Some("Final Name".to_string()),
            Some("Final description".to_string()),
            Some(vec![10, 20, 30]),
        );
        assert_eq!(session.name, "Final Name");
        assert_eq!(session.description, Some("Final description".to_string()));
        assert_eq!(session.avatar, Some(vec![10, 20, 30]));
    }

    /// Test 9: Session serialization and deserialization
    /// Tests JSON serialization preserves all session data
    #[test]
    fn test_session_serialization() {
        let mut session = GroupSession::new(
            "serialize-test".to_string(),
            "Serialization Test".to_string(),
            "admin".to_string(),
            Some("Admin Name".to_string()),
        );

        session.update_settings(
            None,
            Some("Test description".to_string()),
            Some(vec![1, 2, 3]),
        );

        // Add a member
        let member_key = SenderKeyState::new();
        session
            .add_member_key(
                "member1",
                member_key.to_distribution(),
                Some("Member One".to_string()),
                GroupRole::Member,
            )
            .unwrap();

        // Serialize
        let json = session.to_json().unwrap();

        // Deserialize
        let restored = GroupSession::from_json(&json).unwrap();

        // Verify all data preserved
        assert_eq!(restored.group_id, session.group_id);
        assert_eq!(restored.name, session.name);
        assert_eq!(restored.description, session.description);
        assert_eq!(restored.avatar, session.avatar);
        assert_eq!(restored.get_members().len(), session.get_members().len());
        assert!(restored.am_i_admin());

        // Verify encryption still works after deserialization
        let mut restored = restored;
        let encrypted = restored.encrypt(b"Test message").unwrap();
        assert!(!encrypted.ciphertext.is_empty());
    }

    /// Test 10: Key rotation on member removal
    /// Tests that keys are rotated for forward secrecy when members leave
    #[test]
    fn test_key_rotation_on_member_removal() {
        let mut session = GroupSession::new(
            "removal-rotation".to_string(),
            "Removal Test".to_string(),
            "admin".to_string(),
            None,
        );

        // Add members
        for i in 0..3 {
            let key = SenderKeyState::new();
            session
                .add_member_key(
                    &format!("member-{}", i),
                    key.to_distribution(),
                    None,
                    GroupRole::Member,
                )
                .unwrap();
        }

        assert_eq!(session.get_members().len(), 4);

        let initial_generation = session.my_sender_key.generation;

        // Remove each member and verify key rotates
        for i in 0..3 {
            let gen_before = session.my_sender_key.generation;
            session.remove_member(&format!("member-{}", i)).unwrap();
            let gen_after = session.my_sender_key.generation;

            // Key should rotate on each removal
            assert_eq!(gen_after, gen_before + 1);
        }

        // Final generation should be initial + 3
        assert_eq!(session.my_sender_key.generation, initial_generation + 3);
        assert_eq!(session.get_members().len(), 1); // Only admin left
    }

    /// Test 11: Error handling for non-existent member
    /// Tests error handling when operations target non-existent members
    #[test]
    fn test_nonexistent_member_operations() {
        let mut session = GroupSession::new(
            "error-test".to_string(),
            "Error Test".to_string(),
            "admin".to_string(),
            None,
        );

        // Try to remove non-existent member
        let result = session.remove_member("ghost");
        assert!(result.is_err());
        if let Err(GroupError::MemberNotFound(id)) = result {
            assert_eq!(id, "ghost");
        } else {
            panic!("Expected MemberNotFound error");
        }

        // Try to promote non-existent member
        let result = session.promote_to_admin("ghost");
        assert!(result.is_err());

        // Try to demote non-existent member
        let result = session.demote_from_admin("ghost");
        assert!(result.is_err());

        // Try to get non-existent member info
        let member = session.get_member("ghost");
        assert!(member.is_none());
    }

    /// Test 12: Stale key rotation manager
    /// Tests the automatic key rotation for groups that haven't rotated recently
    #[test]
    fn test_stale_key_rotation_manager() {
        let mut manager = GroupSessionManager::new();

        // Create multiple groups
        let _ = manager.create_group("Group 1".to_string(), "admin".to_string(), None);
        let _ = manager.create_group("Group 2".to_string(), "admin".to_string(), None);
        let _ = manager.create_group("Group 3".to_string(), "admin".to_string(), None);

        assert_eq!(manager.list_groups().len(), 3);

        // Immediately after creation, no groups should need rotation
        let rotated = manager.rotate_stale_keys();
        assert!(
            rotated.is_empty(),
            "No groups should need rotation immediately"
        );

        // Manually set last_key_rotation to a time > 7 days ago for testing
        for session in manager.sessions.values_mut() {
            session.last_key_rotation = chrono::Utc::now().timestamp() - (8 * 86400); // 8 days ago
        }

        // Now all groups should need rotation
        for session in manager.sessions.values() {
            assert!(session.needs_key_rotation());
        }

        // Rotate stale keys
        let rotated = manager.rotate_stale_keys();
        assert_eq!(rotated.len(), 3, "All 3 groups should be rotated");

        // Verify rotation happened
        for session in manager.sessions.values() {
            assert!(!session.needs_key_rotation());
            assert_eq!(session.my_sender_key.generation, 1); // Rotated once
        }
    }
}
