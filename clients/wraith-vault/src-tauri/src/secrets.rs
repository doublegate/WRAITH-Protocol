//! Secret Management for WRAITH Vault
//!
//! This module provides high-level secret management:
//! - Secret creation with automatic SSS splitting
//! - Secret storage and retrieval
//! - Key rotation
//! - Secret deletion and secure erasure

use crate::error::{VaultError, VaultResult};
use crate::guardian::Guardian;
use crate::shamir::{ShamirConfig, ShamirSecretSharing, Share};
use crate::shard::{DistributionStatus, EncryptedShard, ShardManager};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Secret type for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    /// Generic secret data
    #[default]
    Generic,
    /// Cryptographic key (private key, seed, etc.)
    CryptoKey,
    /// Password or credential
    Password,
    /// Recovery phrase or mnemonic
    RecoveryPhrase,
    /// Certificate or X.509 key material
    Certificate,
    /// API key or token
    ApiKey,
    /// Document or file encryption key
    DocumentKey,
    /// SSH private key
    SshKey,
    /// PGP/GPG private key
    PgpKey,
}

/// Secret metadata (stored locally, does not contain the secret itself)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    /// Unique secret identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Secret type
    pub secret_type: SecretType,
    /// Shamir configuration (k-of-n)
    pub shamir_config: ShamirConfig,
    /// Creation timestamp
    pub created_at: i64,
    /// Last modified timestamp
    pub modified_at: i64,
    /// Last access timestamp
    pub last_accessed_at: Option<i64>,
    /// Key rotation count
    pub rotation_count: u32,
    /// Last key rotation timestamp
    pub last_rotated_at: Option<i64>,
    /// Salt used for key derivation (if applicable)
    pub key_salt: Option<[u8; 32]>,
    /// IDs of guardians holding shares
    pub guardian_ids: Vec<String>,
    /// Distribution status
    pub distribution_complete: bool,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Optional metadata (JSON)
    pub metadata: Option<String>,
}

impl SecretInfo {
    /// Create new secret info
    pub fn new(name: String, secret_type: SecretType, shamir_config: ShamirConfig) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            secret_type,
            shamir_config,
            created_at: now,
            modified_at: now,
            last_accessed_at: None,
            rotation_count: 0,
            last_rotated_at: None,
            key_salt: None,
            guardian_ids: Vec::new(),
            distribution_complete: false,
            tags: Vec::new(),
            metadata: None,
        }
    }

    /// Mark as accessed
    pub fn mark_accessed(&mut self) {
        self.last_accessed_at = Some(Utc::now().timestamp());
    }

    /// Mark distribution as complete
    pub fn mark_distributed(&mut self, guardian_ids: Vec<String>) {
        self.guardian_ids = guardian_ids;
        self.distribution_complete = true;
        self.modified_at = Utc::now().timestamp();
    }

    /// Record key rotation
    pub fn record_rotation(&mut self, new_guardian_ids: Vec<String>) {
        self.rotation_count += 1;
        self.last_rotated_at = Some(Utc::now().timestamp());
        self.guardian_ids = new_guardian_ids;
        self.modified_at = Utc::now().timestamp();
    }
}

/// Request to create a new secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSecretRequest {
    /// Secret name
    pub name: String,
    /// Secret data (will be split and distributed)
    pub secret_data: Vec<u8>,
    /// Secret type
    pub secret_type: SecretType,
    /// Optional description
    pub description: Option<String>,
    /// Shamir threshold (k)
    pub threshold: u8,
    /// Total shares (n)
    pub total_shares: u8,
    /// Optional tags
    pub tags: Vec<String>,
    /// Password for key derivation (if not using random key)
    pub password: Option<String>,
}

/// Result of creating a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSecretResult {
    /// Created secret info
    pub secret: SecretInfo,
    /// Generated shares (before encryption)
    pub shares: Vec<Share>,
    /// Encryption key used (for local backup)
    pub encryption_key: [u8; 32],
    /// Key salt (if password-derived)
    pub key_salt: Option<[u8; 32]>,
}

/// Secret manager for orchestrating secret operations
pub struct SecretManager {
    /// Shard manager for encryption/decryption
    shard_manager: ShardManager,
    /// In-memory secret info cache
    secrets: HashMap<String, SecretInfo>,
}

impl SecretManager {
    /// Create a new secret manager
    pub fn new() -> Self {
        Self {
            shard_manager: ShardManager::new(),
            secrets: HashMap::new(),
        }
    }

    /// Create a new secret with Shamir splitting
    pub fn create_secret(
        &mut self,
        request: CreateSecretRequest,
    ) -> VaultResult<CreateSecretResult> {
        // Validate request
        if request.name.is_empty() {
            return Err(VaultError::Secret(
                "Secret name cannot be empty".to_string(),
            ));
        }
        if request.secret_data.is_empty() {
            return Err(VaultError::Secret(
                "Secret data cannot be empty".to_string(),
            ));
        }

        // Create Shamir configuration
        let shamir_config = ShamirConfig::new(request.threshold, request.total_shares)?;
        let sss = ShamirSecretSharing::new(shamir_config);

        // Split secret into shares
        let shares = sss.split(&request.secret_data)?;

        // Generate or derive encryption key
        let (encryption_key, key_salt) = if let Some(password) = &request.password {
            let salt = ShardManager::generate_salt();
            let key = ShardManager::derive_key_from_password(password, &salt)?;
            (key, Some(salt))
        } else {
            (ShardManager::generate_random_key(), None)
        };

        // Create secret info
        let mut secret = SecretInfo::new(request.name, request.secret_type, shamir_config);
        secret.description = request.description;
        secret.tags = request.tags;
        secret.key_salt = key_salt;

        // Store in cache
        self.secrets.insert(secret.id.clone(), secret.clone());

        Ok(CreateSecretResult {
            secret,
            shares,
            encryption_key,
            key_salt,
        })
    }

    /// Encrypt shares and prepare for distribution
    pub fn prepare_distribution(
        &mut self,
        secret_id: &str,
        shares: &[Share],
        encryption_key: &[u8; 32],
        guardians: &[Guardian],
    ) -> VaultResult<(Vec<EncryptedShard>, DistributionStatus)> {
        if shares.len() != guardians.len() {
            return Err(VaultError::Secret(format!(
                "Share count ({}) must match guardian count ({})",
                shares.len(),
                guardians.len()
            )));
        }

        // Create encrypted shards
        let mut shards =
            self.shard_manager
                .create_encrypted_shards(secret_id, shares, encryption_key)?;

        // Set guardian_id on each shard
        for (shard, guardian) in shards.iter_mut().zip(guardians.iter()) {
            shard.guardian_id = guardian.id.clone();
            shard.recipient_public_key = guardian.public_key.clone();
        }

        // Initialize distribution
        let status = self
            .shard_manager
            .init_distribution(secret_id, &shards, guardians)?;

        Ok((shards, status))
    }

    /// Mark distribution complete
    pub fn complete_distribution(
        &mut self,
        secret_id: &str,
        guardian_ids: Vec<String>,
    ) -> VaultResult<SecretInfo> {
        let secret = self
            .secrets
            .get_mut(secret_id)
            .ok_or_else(|| VaultError::SecretNotFound(secret_id.to_string()))?;

        secret.mark_distributed(guardian_ids);
        Ok(secret.clone())
    }

    /// Get secret info by ID
    pub fn get_secret(&self, id: &str) -> VaultResult<SecretInfo> {
        self.secrets
            .get(id)
            .cloned()
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))
    }

    /// Get secret info by ID (mutable, for marking access)
    pub fn get_secret_mut(&mut self, id: &str) -> VaultResult<&mut SecretInfo> {
        self.secrets
            .get_mut(id)
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))
    }

    /// List all secrets
    pub fn list_secrets(&self) -> Vec<SecretInfo> {
        self.secrets.values().cloned().collect()
    }

    /// List secrets by type
    pub fn list_secrets_by_type(&self, secret_type: SecretType) -> Vec<SecretInfo> {
        self.secrets
            .values()
            .filter(|s| s.secret_type == secret_type)
            .cloned()
            .collect()
    }

    /// List secrets by tag
    pub fn list_secrets_by_tag(&self, tag: &str) -> Vec<SecretInfo> {
        self.secrets
            .values()
            .filter(|s| s.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Search secrets by name
    pub fn search_secrets(&self, query: &str) -> Vec<SecretInfo> {
        let query_lower = query.to_lowercase();
        self.secrets
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Update secret metadata
    pub fn update_secret(
        &mut self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> VaultResult<SecretInfo> {
        let secret = self
            .secrets
            .get_mut(id)
            .ok_or_else(|| VaultError::SecretNotFound(id.to_string()))?;

        if let Some(name) = name {
            secret.name = name;
        }
        if let Some(desc) = description {
            secret.description = Some(desc);
        }
        if let Some(tags) = tags {
            secret.tags = tags;
        }
        secret.modified_at = Utc::now().timestamp();

        Ok(secret.clone())
    }

    /// Delete a secret
    pub fn delete_secret(&mut self, id: &str) -> VaultResult<()> {
        if self.secrets.remove(id).is_none() {
            return Err(VaultError::SecretNotFound(id.to_string()));
        }

        // Clean up local shards
        self.shard_manager.remove_local_shards(id);

        tracing::info!("Deleted secret {}", id);
        Ok(())
    }

    /// Rotate key for a secret
    ///
    /// This creates new shares with a new encryption key and requires
    /// redistributing to guardians.
    pub fn rotate_key(
        &mut self,
        secret_id: &str,
        recovered_secret: &[u8],
        new_password: Option<&str>,
    ) -> VaultResult<CreateSecretResult> {
        let secret = self
            .secrets
            .get(secret_id)
            .ok_or_else(|| VaultError::SecretNotFound(secret_id.to_string()))?;

        // Create new shares
        let sss = ShamirSecretSharing::new(secret.shamir_config);
        let shares = sss.split(recovered_secret)?;

        // Generate new encryption key
        let (encryption_key, key_salt) = if let Some(password) = new_password {
            let salt = ShardManager::generate_salt();
            let key = ShardManager::derive_key_from_password(password, &salt)?;
            (key, Some(salt))
        } else {
            (ShardManager::generate_random_key(), None)
        };

        // Update secret metadata
        let secret = self.secrets.get_mut(secret_id).unwrap();
        secret.key_salt = key_salt;
        // Note: rotation_count and last_rotated_at updated after redistribution

        Ok(CreateSecretResult {
            secret: secret.clone(),
            shares,
            encryption_key,
            key_salt,
        })
    }

    /// Record successful key rotation after redistribution
    pub fn record_rotation(
        &mut self,
        secret_id: &str,
        guardian_ids: Vec<String>,
    ) -> VaultResult<SecretInfo> {
        let secret = self
            .secrets
            .get_mut(secret_id)
            .ok_or_else(|| VaultError::SecretNotFound(secret_id.to_string()))?;

        secret.record_rotation(guardian_ids);
        Ok(secret.clone())
    }

    /// Get shard manager reference
    pub fn shard_manager(&self) -> &ShardManager {
        &self.shard_manager
    }

    /// Get mutable shard manager reference
    pub fn shard_manager_mut(&mut self) -> &mut ShardManager {
        &mut self.shard_manager
    }

    /// Load secrets from database
    pub fn load_secrets(&mut self, secrets: Vec<SecretInfo>) {
        for secret in secrets {
            self.secrets.insert(secret.id.clone(), secret);
        }
    }

    /// Get secret count
    pub fn count(&self) -> usize {
        self.secrets.len()
    }

    /// Get secrets needing rotation (not rotated in specified days)
    pub fn get_secrets_needing_rotation(&self, max_age_days: i64) -> Vec<SecretInfo> {
        let now = Utc::now().timestamp();
        let max_age_seconds = max_age_days * 24 * 60 * 60;

        self.secrets
            .values()
            .filter(|s| {
                let last_rotation = s.last_rotated_at.unwrap_or(s.created_at);
                (now - last_rotation) > max_age_seconds
            })
            .cloned()
            .collect()
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request(name: &str, data: &[u8]) -> CreateSecretRequest {
        CreateSecretRequest {
            name: name.to_string(),
            secret_data: data.to_vec(),
            secret_type: SecretType::Generic,
            description: Some("Test secret".to_string()),
            threshold: 3,
            total_shares: 5,
            tags: vec!["test".to_string()],
            password: None,
        }
    }

    #[test]
    fn test_create_secret() {
        let mut manager = SecretManager::new();
        let request = create_test_request("My Secret", b"secret data 123");

        let result = manager.create_secret(request).unwrap();

        assert_eq!(result.secret.name, "My Secret");
        assert_eq!(result.shares.len(), 5);
        assert_eq!(result.secret.shamir_config.threshold, 3);
        assert_eq!(result.secret.shamir_config.total_shares, 5);
    }

    #[test]
    fn test_create_secret_with_password() {
        let mut manager = SecretManager::new();
        let request = CreateSecretRequest {
            name: "Password Secret".to_string(),
            secret_data: b"protected data".to_vec(),
            secret_type: SecretType::Password,
            description: None,
            threshold: 2,
            total_shares: 3,
            tags: vec![],
            password: Some("my-password".to_string()),
        };

        let result = manager.create_secret(request).unwrap();

        assert!(result.key_salt.is_some());
        assert!(result.secret.key_salt.is_some());
    }

    #[test]
    fn test_create_secret_validation() {
        let mut manager = SecretManager::new();

        // Empty name
        let request = create_test_request("", b"data");
        assert!(manager.create_secret(request).is_err());

        // Empty data
        let request = create_test_request("Name", b"");
        assert!(manager.create_secret(request).is_err());
    }

    #[test]
    fn test_get_and_list_secrets() {
        let mut manager = SecretManager::new();

        // Create multiple secrets
        for i in 0..3 {
            let request = create_test_request(&format!("Secret {}", i), b"data");
            manager.create_secret(request).unwrap();
        }

        assert_eq!(manager.list_secrets().len(), 3);

        let secrets = manager.list_secrets();
        let first = manager.get_secret(&secrets[0].id).unwrap();
        assert!(first.name.starts_with("Secret"));
    }

    #[test]
    fn test_list_by_type() {
        let mut manager = SecretManager::new();

        // Create different types
        let request = create_test_request("Generic", b"data");
        manager.create_secret(request).unwrap();

        let mut request = create_test_request("Key", b"key");
        request.secret_type = SecretType::CryptoKey;
        manager.create_secret(request).unwrap();

        let mut request = create_test_request("Key 2", b"key2");
        request.secret_type = SecretType::CryptoKey;
        manager.create_secret(request).unwrap();

        let generic = manager.list_secrets_by_type(SecretType::Generic);
        let keys = manager.list_secrets_by_type(SecretType::CryptoKey);

        assert_eq!(generic.len(), 1);
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_list_by_tag() {
        let mut manager = SecretManager::new();

        let mut request = create_test_request("Tagged", b"data");
        request.tags = vec!["important".to_string(), "work".to_string()];
        manager.create_secret(request).unwrap();

        let mut request = create_test_request("Other", b"data2");
        request.tags = vec!["personal".to_string()];
        manager.create_secret(request).unwrap();

        let important = manager.list_secrets_by_tag("important");
        let personal = manager.list_secrets_by_tag("personal");
        let none = manager.list_secrets_by_tag("nonexistent");

        assert_eq!(important.len(), 1);
        assert_eq!(personal.len(), 1);
        assert_eq!(none.len(), 0);
    }

    #[test]
    fn test_search_secrets() {
        let mut manager = SecretManager::new();

        let mut request = create_test_request("Database Password", b"data");
        request.description = Some("Production DB credentials".to_string());
        manager.create_secret(request).unwrap();

        let mut request = create_test_request("API Key", b"data2");
        request.description = Some("Service API key".to_string());
        manager.create_secret(request).unwrap();

        let results = manager.search_secrets("database");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Database Password");

        let results = manager.search_secrets("production");
        assert_eq!(results.len(), 1);

        let results = manager.search_secrets("api");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_update_secret() {
        let mut manager = SecretManager::new();
        let request = create_test_request("Original", b"data");
        let result = manager.create_secret(request).unwrap();

        let updated = manager
            .update_secret(
                &result.secret.id,
                Some("Updated Name".to_string()),
                Some("New description".to_string()),
                Some(vec!["new-tag".to_string()]),
            )
            .unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("New description".to_string()));
        assert!(updated.tags.contains(&"new-tag".to_string()));
    }

    #[test]
    fn test_delete_secret() {
        let mut manager = SecretManager::new();
        let request = create_test_request("ToDelete", b"data");
        let result = manager.create_secret(request).unwrap();
        let id = result.secret.id.clone();

        assert_eq!(manager.count(), 1);

        manager.delete_secret(&id).unwrap();

        assert_eq!(manager.count(), 0);
        assert!(manager.get_secret(&id).is_err());
    }

    #[test]
    fn test_key_rotation() {
        let mut manager = SecretManager::new();
        let request = create_test_request("Rotating", b"original data");
        let result = manager.create_secret(request).unwrap();
        let id = result.secret.id.clone();

        // Simulate recovery (using original data for test)
        let recovered = b"original data";
        let rotated = manager.rotate_key(&id, recovered, None).unwrap();

        assert_eq!(rotated.shares.len(), 5);

        // Record rotation
        let guardian_ids: Vec<String> = (0..5).map(|i| format!("guardian_{}", i)).collect();
        let updated = manager.record_rotation(&id, guardian_ids).unwrap();

        assert_eq!(updated.rotation_count, 1);
        assert!(updated.last_rotated_at.is_some());
    }

    #[test]
    fn test_secrets_needing_rotation() {
        let mut manager = SecretManager::new();

        // Create a secret that appears old
        let request = create_test_request("Old Secret", b"data");
        let result = manager.create_secret(request).unwrap();

        // Manually set created_at to 100 days ago
        let secret = manager.get_secret_mut(&result.secret.id).unwrap();
        secret.created_at = Utc::now().timestamp() - (100 * 24 * 60 * 60);

        // Create a recent secret
        let request = create_test_request("New Secret", b"data2");
        manager.create_secret(request).unwrap();

        // Get secrets needing rotation (older than 90 days)
        let needing_rotation = manager.get_secrets_needing_rotation(90);

        assert_eq!(needing_rotation.len(), 1);
        assert_eq!(needing_rotation[0].name, "Old Secret");
    }

    #[test]
    fn test_prepare_distribution() {
        let mut manager = SecretManager::new();
        let request = create_test_request("Distributed", b"secret data");
        let result = manager.create_secret(request).unwrap();

        // Create mock guardians
        let guardians: Vec<Guardian> = (0..5)
            .map(|i| {
                crate::guardian::Guardian::new(
                    format!("Guardian {}", i),
                    format!("peer{}", i),
                    format!("pubkey{}", i),
                )
            })
            .collect();

        let (shards, status) = manager
            .prepare_distribution(
                &result.secret.id,
                &result.shares,
                &result.encryption_key,
                &guardians,
            )
            .unwrap();

        assert_eq!(shards.len(), 5);
        assert_eq!(status.total_shards, 5);
    }

    #[test]
    fn test_complete_distribution() {
        let mut manager = SecretManager::new();
        let request = create_test_request("Complete", b"data");
        let result = manager.create_secret(request).unwrap();

        let guardian_ids: Vec<String> = (0..5).map(|i| format!("g{}", i)).collect();
        let updated = manager
            .complete_distribution(&result.secret.id, guardian_ids.clone())
            .unwrap();

        assert!(updated.distribution_complete);
        assert_eq!(updated.guardian_ids, guardian_ids);
    }
}
