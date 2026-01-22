//! Shard Encryption and Distribution for WRAITH Vault
//!
//! This module handles:
//! - Encryption of secret shares before distribution
//! - Distribution of encrypted shards to guardians
//! - Retrieval and decryption of shards during recovery

use crate::error::{VaultError, VaultResult};
use crate::guardian::Guardian;
use crate::shamir::Share;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use chrono::Utc;
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Encrypted shard containing a secret share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedShard {
    /// Unique identifier for this shard
    pub id: String,
    /// ID of the secret this shard belongs to
    pub secret_id: String,
    /// ID of the guardian holding this shard
    pub guardian_id: String,
    /// Share index (from Shamir)
    pub share_index: u8,
    /// Encrypted share data
    pub encrypted_data: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: [u8; 24],
    /// Public key used for encryption (recipient's key)
    pub recipient_public_key: String,
    /// Shard creation timestamp
    pub created_at: i64,
    /// Hash of original share for verification
    pub share_hash: [u8; 32],
}

impl EncryptedShard {
    /// Verify shard integrity
    pub fn verify_hash(&self, share_data: &[u8]) -> bool {
        let computed_hash = blake3::hash(share_data);
        self.share_hash == *computed_hash.as_bytes()
    }
}

/// Shard distribution assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardAssignment {
    /// Shard ID
    pub shard_id: String,
    /// Guardian ID assigned to hold this shard
    pub guardian_id: String,
    /// Guardian's peer ID for network communication
    pub guardian_peer_id: String,
    /// Whether the shard has been successfully delivered
    pub delivered: bool,
    /// Delivery timestamp (if delivered)
    pub delivered_at: Option<i64>,
    /// Last delivery attempt timestamp
    pub last_attempt_at: Option<i64>,
    /// Number of delivery attempts
    pub attempt_count: u32,
    /// Last error message (if failed)
    pub last_error: Option<String>,
}

/// Distribution status for a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStatus {
    /// Secret ID
    pub secret_id: String,
    /// Total shards created
    pub total_shards: u32,
    /// Shards successfully delivered
    pub delivered_shards: u32,
    /// Shards pending delivery
    pub pending_shards: u32,
    /// Shards that failed delivery
    pub failed_shards: u32,
    /// Individual shard assignments
    pub assignments: Vec<ShardAssignment>,
    /// Distribution start timestamp
    pub started_at: i64,
    /// Distribution completion timestamp (if complete)
    pub completed_at: Option<i64>,
    /// Overall status
    pub status: DistributionState,
}

/// State of distribution process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionState {
    /// Distribution not yet started
    Pending,
    /// Distribution in progress
    InProgress,
    /// All shards successfully delivered
    Complete,
    /// Some shards failed but threshold met
    PartialSuccess,
    /// Distribution failed (threshold not met)
    Failed,
}

/// Encryption configuration for shards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardEncryptionConfig {
    /// Whether to use recipient's public key for encryption
    pub use_recipient_key: bool,
    /// Additional authenticated data (secret metadata)
    pub include_metadata: bool,
    /// Key derivation iterations (for password-based encryption)
    pub kdf_iterations: u32,
}

impl Default for ShardEncryptionConfig {
    fn default() -> Self {
        Self {
            use_recipient_key: true,
            include_metadata: true,
            kdf_iterations: 100_000,
        }
    }
}

/// Shard manager for encryption, distribution, and retrieval
#[allow(dead_code)]
pub struct ShardManager {
    /// Encryption configuration
    config: ShardEncryptionConfig,
    /// In-memory shard storage (for testing/local operation)
    local_shards: HashMap<String, EncryptedShard>,
    /// Distribution status tracking
    distributions: HashMap<String, DistributionStatus>,
}

impl ShardManager {
    /// Create a new shard manager
    pub fn new() -> Self {
        Self {
            config: ShardEncryptionConfig::default(),
            local_shards: HashMap::new(),
            distributions: HashMap::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ShardEncryptionConfig) -> Self {
        Self {
            config,
            local_shards: HashMap::new(),
            distributions: HashMap::new(),
        }
    }

    /// Encrypt a share for a specific guardian
    pub fn encrypt_share(
        &self,
        secret_id: &str,
        share: &Share,
        encryption_key: &[u8; 32],
    ) -> VaultResult<EncryptedShard> {
        // Generate random nonce
        let mut nonce = [0u8; 24];
        OsRng.fill_bytes(&mut nonce);

        // Create cipher
        let cipher = XChaCha20Poly1305::new(encryption_key.into());

        // Serialize share for encryption
        let share_bytes = serde_json::to_vec(share)
            .map_err(|e| VaultError::Shard(format!("Failed to serialize share: {}", e)))?;

        // Compute hash before encryption for verification
        let share_hash = *blake3::hash(&share_bytes).as_bytes();

        // Encrypt
        let xnonce = XNonce::from_slice(&nonce);
        let encrypted_data = cipher
            .encrypt(xnonce, share_bytes.as_ref())
            .map_err(|e| VaultError::Shard(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedShard {
            id: Uuid::new_v4().to_string(),
            secret_id: secret_id.to_string(),
            guardian_id: String::new(), // Set by caller during distribution
            share_index: share.index,
            encrypted_data,
            nonce,
            recipient_public_key: String::new(), // Set by caller
            created_at: Utc::now().timestamp(),
            share_hash,
        })
    }

    /// Decrypt a shard to recover the share
    pub fn decrypt_shard(
        &self,
        shard: &EncryptedShard,
        decryption_key: &[u8; 32],
    ) -> VaultResult<Share> {
        // Create cipher
        let cipher = XChaCha20Poly1305::new(decryption_key.into());

        // Decrypt
        let xnonce = XNonce::from_slice(&shard.nonce);
        let decrypted = cipher
            .decrypt(xnonce, shard.encrypted_data.as_ref())
            .map_err(|e| VaultError::Shard(format!("Decryption failed: {}", e)))?;

        // Verify hash
        let computed_hash = *blake3::hash(&decrypted).as_bytes();
        if computed_hash != shard.share_hash {
            return Err(VaultError::Shard(
                "Shard integrity check failed".to_string(),
            ));
        }

        // Deserialize share
        let share: Share = serde_json::from_slice(&decrypted)
            .map_err(|e| VaultError::Shard(format!("Failed to deserialize share: {}", e)))?;

        Ok(share)
    }

    /// Create encrypted shards for all shares
    pub fn create_encrypted_shards(
        &mut self,
        secret_id: &str,
        shares: &[Share],
        encryption_key: &[u8; 32],
    ) -> VaultResult<Vec<EncryptedShard>> {
        let mut shards = Vec::with_capacity(shares.len());

        for share in shares {
            let shard = self.encrypt_share(secret_id, share, encryption_key)?;
            self.local_shards.insert(shard.id.clone(), shard.clone());
            shards.push(shard);
        }

        Ok(shards)
    }

    /// Initialize distribution for a secret
    pub fn init_distribution(
        &mut self,
        secret_id: &str,
        shards: &[EncryptedShard],
        guardians: &[Guardian],
    ) -> VaultResult<DistributionStatus> {
        if shards.len() != guardians.len() {
            return Err(VaultError::Shard(format!(
                "Shard count ({}) does not match guardian count ({})",
                shards.len(),
                guardians.len()
            )));
        }

        let mut assignments = Vec::with_capacity(shards.len());

        for (shard, guardian) in shards.iter().zip(guardians.iter()) {
            assignments.push(ShardAssignment {
                shard_id: shard.id.clone(),
                guardian_id: guardian.id.clone(),
                guardian_peer_id: guardian.peer_id.clone(),
                delivered: false,
                delivered_at: None,
                last_attempt_at: None,
                attempt_count: 0,
                last_error: None,
            });
        }

        let status = DistributionStatus {
            secret_id: secret_id.to_string(),
            total_shards: shards.len() as u32,
            delivered_shards: 0,
            pending_shards: shards.len() as u32,
            failed_shards: 0,
            assignments,
            started_at: Utc::now().timestamp(),
            completed_at: None,
            status: DistributionState::InProgress,
        };

        self.distributions
            .insert(secret_id.to_string(), status.clone());
        Ok(status)
    }

    /// Mark a shard as delivered
    pub fn mark_shard_delivered(
        &mut self,
        secret_id: &str,
        shard_id: &str,
    ) -> VaultResult<DistributionStatus> {
        let status = self.distributions.get_mut(secret_id).ok_or_else(|| {
            VaultError::Shard(format!("No distribution for secret {}", secret_id))
        })?;

        let assignment = status
            .assignments
            .iter_mut()
            .find(|a| a.shard_id == shard_id)
            .ok_or_else(|| {
                VaultError::Shard(format!("Shard {} not found in distribution", shard_id))
            })?;

        if !assignment.delivered {
            assignment.delivered = true;
            assignment.delivered_at = Some(Utc::now().timestamp());
            status.delivered_shards += 1;
            status.pending_shards -= 1;
        }

        // Update overall status
        self.update_distribution_state(secret_id)?;

        Ok(self.distributions.get(secret_id).unwrap().clone())
    }

    /// Mark a shard delivery as failed
    pub fn mark_shard_failed(
        &mut self,
        secret_id: &str,
        shard_id: &str,
        error: &str,
    ) -> VaultResult<DistributionStatus> {
        let status = self.distributions.get_mut(secret_id).ok_or_else(|| {
            VaultError::Shard(format!("No distribution for secret {}", secret_id))
        })?;

        let assignment = status
            .assignments
            .iter_mut()
            .find(|a| a.shard_id == shard_id)
            .ok_or_else(|| VaultError::Shard(format!("Shard {} not found", shard_id)))?;

        assignment.last_attempt_at = Some(Utc::now().timestamp());
        assignment.attempt_count += 1;
        assignment.last_error = Some(error.to_string());

        if assignment.attempt_count >= 3 && !assignment.delivered {
            status.failed_shards += 1;
            status.pending_shards -= 1;
        }

        // Update overall status
        self.update_distribution_state(secret_id)?;

        Ok(self.distributions.get(secret_id).unwrap().clone())
    }

    /// Update distribution state based on current assignments
    fn update_distribution_state(&mut self, secret_id: &str) -> VaultResult<()> {
        let status = self.distributions.get_mut(secret_id).ok_or_else(|| {
            VaultError::Shard(format!("No distribution for secret {}", secret_id))
        })?;

        // Check if all shards are delivered or failed
        if status.pending_shards == 0 {
            status.completed_at = Some(Utc::now().timestamp());

            if status.delivered_shards == status.total_shards {
                status.status = DistributionState::Complete;
            } else if status.delivered_shards > 0 {
                status.status = DistributionState::PartialSuccess;
            } else {
                status.status = DistributionState::Failed;
            }
        }

        Ok(())
    }

    /// Get distribution status for a secret
    pub fn get_distribution_status(&self, secret_id: &str) -> Option<&DistributionStatus> {
        self.distributions.get(secret_id)
    }

    /// Store a shard locally
    pub fn store_local_shard(&mut self, shard: EncryptedShard) {
        self.local_shards.insert(shard.id.clone(), shard);
    }

    /// Retrieve a shard from local storage
    pub fn get_local_shard(&self, shard_id: &str) -> Option<&EncryptedShard> {
        self.local_shards.get(shard_id)
    }

    /// Get all local shards for a secret
    pub fn get_local_shards_for_secret(&self, secret_id: &str) -> Vec<&EncryptedShard> {
        self.local_shards
            .values()
            .filter(|s| s.secret_id == secret_id)
            .collect()
    }

    /// Remove local shards for a secret
    pub fn remove_local_shards(&mut self, secret_id: &str) {
        self.local_shards.retain(|_, s| s.secret_id != secret_id);
    }

    /// Derive encryption key from password
    pub fn derive_key_from_password(password: &str, salt: &[u8; 32]) -> VaultResult<[u8; 32]> {
        use blake3::Hasher;

        // Use BLAKE3 key derivation
        let mut hasher = Hasher::new_derive_key("WRAITH-Vault-Shard-Key-v1");
        hasher.update(password.as_bytes());
        hasher.update(salt);

        let mut key = [0u8; 32];
        hasher.finalize_xof().fill(&mut key);

        Ok(key)
    }

    /// Generate a random encryption key
    pub fn generate_random_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Generate a random salt
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }
}

impl Default for ShardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shamir::{ShamirConfig, ShamirSecretSharing};

    #[test]
    fn test_encrypt_decrypt_share() {
        let manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        let share = Share::new(1, vec![1, 2, 3, 4, 5]);
        let shard = manager.encrypt_share("secret1", &share, &key).unwrap();

        let decrypted = manager.decrypt_shard(&shard, &key).unwrap();
        assert_eq!(decrypted, share);
    }

    #[test]
    fn test_wrong_decryption_key() {
        let manager = ShardManager::new();
        let key1 = ShardManager::generate_random_key();
        let key2 = ShardManager::generate_random_key();

        let share = Share::new(1, vec![1, 2, 3, 4, 5]);
        let shard = manager.encrypt_share("secret1", &share, &key1).unwrap();

        let result = manager.decrypt_shard(&shard, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_shard_integrity() {
        let manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        let share = Share::new(1, vec![1, 2, 3, 4, 5]);
        let mut shard = manager.encrypt_share("secret1", &share, &key).unwrap();

        // Tamper with encrypted data
        if !shard.encrypted_data.is_empty() {
            shard.encrypted_data[0] ^= 0xFF;
        }

        let result = manager.decrypt_shard(&shard, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_encrypted_shards() {
        let mut manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        let config = ShamirConfig::new(3, 5).unwrap();
        let sss = ShamirSecretSharing::new(config);
        let shares = sss.split(b"My secret data").unwrap();

        let shards = manager
            .create_encrypted_shards("secret1", &shares, &key)
            .unwrap();

        assert_eq!(shards.len(), 5);

        // Verify all shards can be decrypted
        for shard in &shards {
            let decrypted = manager.decrypt_shard(shard, &key).unwrap();
            assert!(shares.iter().any(|s| s == &decrypted));
        }
    }

    #[test]
    fn test_key_derivation() {
        let password = "my-secure-password";
        let salt = ShardManager::generate_salt();

        let key1 = ShardManager::derive_key_from_password(password, &salt).unwrap();
        let key2 = ShardManager::derive_key_from_password(password, &salt).unwrap();

        // Same password + salt = same key
        assert_eq!(key1, key2);

        // Different salt = different key
        let salt2 = ShardManager::generate_salt();
        let key3 = ShardManager::derive_key_from_password(password, &salt2).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_distribution_workflow() {
        let mut manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        // Create shares and shards
        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);
        let shares = sss.split(b"Test secret").unwrap();
        let shards = manager
            .create_encrypted_shards("secret1", &shares, &key)
            .unwrap();

        // Create mock guardians
        let guardians: Vec<Guardian> = (0..3)
            .map(|i| {
                crate::guardian::Guardian::new(
                    format!("Guardian {}", i),
                    format!("peer{}", i),
                    format!("pubkey{}", i),
                )
            })
            .collect();

        // Initialize distribution
        let status = manager
            .init_distribution("secret1", &shards, &guardians)
            .unwrap();

        assert_eq!(status.total_shards, 3);
        assert_eq!(status.delivered_shards, 0);
        assert_eq!(status.pending_shards, 3);
        assert_eq!(status.status, DistributionState::InProgress);

        // Mark shards as delivered
        for shard in &shards {
            manager.mark_shard_delivered("secret1", &shard.id).unwrap();
        }

        let final_status = manager.get_distribution_status("secret1").unwrap();
        assert_eq!(final_status.status, DistributionState::Complete);
        assert_eq!(final_status.delivered_shards, 3);
        assert_eq!(final_status.pending_shards, 0);
    }

    #[test]
    fn test_partial_distribution() {
        let mut manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);
        let shares = sss.split(b"Test").unwrap();
        let shards = manager
            .create_encrypted_shards("secret1", &shares, &key)
            .unwrap();

        let guardians: Vec<Guardian> = (0..3)
            .map(|i| {
                crate::guardian::Guardian::new(
                    format!("Guardian {}", i),
                    format!("peer{}", i),
                    format!("pubkey{}", i),
                )
            })
            .collect();

        manager
            .init_distribution("secret1", &shards, &guardians)
            .unwrap();

        // Deliver 2 shards, fail 1
        manager
            .mark_shard_delivered("secret1", &shards[0].id)
            .unwrap();
        manager
            .mark_shard_delivered("secret1", &shards[1].id)
            .unwrap();

        // Mark as failed after 3 attempts
        for _ in 0..3 {
            manager
                .mark_shard_failed("secret1", &shards[2].id, "Connection failed")
                .unwrap();
        }

        let final_status = manager.get_distribution_status("secret1").unwrap();
        assert_eq!(final_status.status, DistributionState::PartialSuccess);
        assert_eq!(final_status.delivered_shards, 2);
        assert_eq!(final_status.failed_shards, 1);
    }

    #[test]
    fn test_local_shard_storage() {
        let mut manager = ShardManager::new();
        let key = ShardManager::generate_random_key();

        let share = Share::new(1, vec![1, 2, 3]);
        let shard = manager.encrypt_share("secret1", &share, &key).unwrap();
        let shard_id = shard.id.clone();

        manager.store_local_shard(shard);

        assert!(manager.get_local_shard(&shard_id).is_some());

        let shards = manager.get_local_shards_for_secret("secret1");
        assert_eq!(shards.len(), 1);

        manager.remove_local_shards("secret1");
        assert!(manager.get_local_shard(&shard_id).is_none());
    }
}
