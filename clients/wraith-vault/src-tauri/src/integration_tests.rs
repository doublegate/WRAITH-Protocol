//! Integration Tests for WRAITH Vault
//!
//! These tests verify the complete workflow of secret creation,
//! distribution, and recovery using the vault system.

use crate::database::Database;
use crate::guardian::{
    Guardian, GuardianCapabilities, GuardianManager, GuardianStatus, TrustLevel,
};
use crate::recovery::RecoveryManager;
use crate::secrets::{CreateSecretRequest, SecretManager, SecretType};
use crate::shamir::{ShamirConfig, ShamirSecretSharing};
use crate::shard::ShardManager;
use tempfile::tempdir;

/// Create test guardians
fn create_test_guardians(count: usize) -> Vec<Guardian> {
    (0..count)
        .map(|i| {
            let mut guardian = Guardian::new(
                format!("Test Guardian {}", i),
                format!("peer_{:04x}", i),
                format!("pubkey_{:04x}", i),
            );
            guardian.status = GuardianStatus::Online;
            guardian.trust_level = TrustLevel::Trusted;
            guardian.capabilities = GuardianCapabilities {
                can_store: true,
                can_recover: true,
                max_storage: 0,
                supports_encryption: true,
                supports_auto_refresh: false,
            };
            guardian
        })
        .collect()
}

#[test]
fn test_secret_creation_and_splitting() {
    // Create a secret manager
    let mut secret_manager = SecretManager::new();

    // Create a secret
    let request = CreateSecretRequest {
        name: "Test Secret".to_string(),
        secret_data: b"my super secret data that needs protection".to_vec(),
        secret_type: SecretType::Generic,
        description: Some("A test secret for integration testing".to_string()),
        threshold: 3,
        total_shares: 5,
        tags: vec!["test".to_string(), "integration".to_string()],
        password: None,
    };

    let result = secret_manager.create_secret(request).unwrap();

    // Verify result
    assert_eq!(result.secret.name, "Test Secret");
    assert_eq!(result.shares.len(), 5);
    assert_eq!(result.secret.shamir_config.threshold, 3);
    assert_eq!(result.secret.shamir_config.total_shares, 5);
    assert_eq!(result.encryption_key.len(), 32);
}

#[test]
fn test_shamir_secret_reconstruction() {
    let secret_data = b"The quick brown fox jumps over the lazy dog";
    let config = ShamirConfig::new(3, 5).unwrap();
    let sss = ShamirSecretSharing::new(config);

    // Split secret
    let shares = sss.split(secret_data).unwrap();
    assert_eq!(shares.len(), 5);

    // Reconstruct with exactly threshold shares
    let threshold_shares: Vec<_> = shares.iter().take(3).cloned().collect();
    let reconstructed = sss.combine(&threshold_shares).unwrap();
    assert_eq!(reconstructed, secret_data);

    // Reconstruct with more than threshold shares
    let extra_shares: Vec<_> = shares.iter().take(4).cloned().collect();
    let reconstructed = sss.combine(&extra_shares).unwrap();
    assert_eq!(reconstructed, secret_data);

    // Reconstruction should fail with fewer than threshold shares
    let insufficient_shares: Vec<_> = shares.iter().take(2).cloned().collect();
    let result = sss.combine(&insufficient_shares);
    assert!(result.is_err());
}

#[test]
fn test_shard_encryption_decryption() {
    let shard_manager = ShardManager::new();
    let secret_id = "test-secret-123";
    let encryption_key = ShardManager::generate_random_key();

    // Create a share
    let share = crate::shamir::Share {
        index: 1,
        data: vec![0x42; 32],
    };

    // Encrypt the share
    let encrypted_shard = shard_manager
        .encrypt_share(secret_id, &share, &encryption_key)
        .unwrap();

    // Verify encryption worked
    assert_eq!(encrypted_shard.secret_id, secret_id);
    assert_eq!(encrypted_shard.share_index, 1);
    assert!(!encrypted_shard.encrypted_data.is_empty());
    assert_ne!(encrypted_shard.encrypted_data, share.data);

    // Decrypt the shard
    let decrypted_share = shard_manager
        .decrypt_shard(&encrypted_shard, &encryption_key)
        .unwrap();

    // Verify decryption matches original
    assert_eq!(decrypted_share.index, share.index);
    assert_eq!(decrypted_share.data, share.data);
}

#[tokio::test]
async fn test_guardian_management() {
    let manager = GuardianManager::new();

    // Add guardians
    let guardians = create_test_guardians(5);
    for guardian in &guardians {
        manager.add_guardian(guardian.clone()).await.unwrap();
    }

    // List all guardians
    let all = manager.list_guardians().await;
    assert_eq!(all.len(), 5);

    // List available guardians
    let available = manager.list_available_guardians().await;
    assert_eq!(available.len(), 5);

    // Select guardians for distribution
    let selected = manager.select_guardians_for_distribution(3).await.unwrap();
    assert_eq!(selected.len(), 3);

    // Update guardian status
    let first_id = guardians[0].id.clone();
    manager
        .update_guardian_status(&first_id, GuardianStatus::Offline)
        .await
        .unwrap();

    // Verify status changed
    let updated = manager.get_guardian(&first_id).await.unwrap();
    assert_eq!(updated.status, GuardianStatus::Offline);

    // Available count should decrease
    let available = manager.list_available_guardians().await;
    assert_eq!(available.len(), 4);
}

#[test]
fn test_database_secret_operations() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).unwrap();

    // Create a secret
    let secret = crate::secrets::SecretInfo::new(
        "Database Test Secret".to_string(),
        SecretType::Password,
        ShamirConfig::new(2, 3).unwrap(),
    );

    db.create_secret(&secret).unwrap();

    // Retrieve the secret
    let retrieved = db.get_secret(&secret.id).unwrap().unwrap();
    assert_eq!(retrieved.name, "Database Test Secret");
    assert!(matches!(retrieved.secret_type, SecretType::Password));

    // List secrets
    let secrets = db.list_secrets().unwrap();
    assert_eq!(secrets.len(), 1);

    // Delete the secret
    db.delete_secret(&secret.id).unwrap();
    let deleted = db.get_secret(&secret.id).unwrap();
    assert!(deleted.is_none());
}

#[test]
fn test_database_guardian_operations() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).unwrap();

    // Create a guardian
    let guardian = Guardian::new(
        "Database Test Guardian".to_string(),
        "db_test_peer".to_string(),
        "db_test_pubkey".to_string(),
    );

    db.create_guardian(&guardian).unwrap();

    // Retrieve the guardian
    let retrieved = db.get_guardian(&guardian.id).unwrap().unwrap();
    assert_eq!(retrieved.name, "Database Test Guardian");
    assert_eq!(retrieved.peer_id, "db_test_peer");

    // List guardians
    let guardians = db.list_guardians().unwrap();
    assert_eq!(guardians.len(), 1);

    // Update the guardian
    let mut updated = retrieved.clone();
    updated.status = GuardianStatus::Online;
    updated.trust_level = TrustLevel::Trusted;
    db.update_guardian(&updated).unwrap();

    // Verify update
    let retrieved = db.get_guardian(&guardian.id).unwrap().unwrap();
    assert_eq!(retrieved.status, GuardianStatus::Online);
    assert_eq!(retrieved.trust_level, TrustLevel::Trusted);

    // Delete the guardian
    db.delete_guardian(&guardian.id).unwrap();
    let deleted = db.get_guardian(&guardian.id).unwrap();
    assert!(deleted.is_none());
}

#[test]
fn test_complete_secret_workflow() {
    // This test simulates the complete workflow:
    // 1. Create a secret with Shamir splitting
    // 2. Encrypt shares
    // 3. Distribute to guardians
    // 4. Recover the secret

    let secret_data = b"This is the secret to be protected";

    // Step 1: Create secret manager and create secret
    let mut secret_manager = SecretManager::new();
    let request = CreateSecretRequest {
        name: "Complete Workflow Secret".to_string(),
        secret_data: secret_data.to_vec(),
        secret_type: SecretType::CryptoKey,
        description: None,
        threshold: 3,
        total_shares: 5,
        tags: vec![],
        password: None,
    };

    let creation_result = secret_manager.create_secret(request).unwrap();
    let secret_id = creation_result.secret.id.clone();
    let encryption_key = creation_result.encryption_key;

    // Step 2: Create guardians
    let guardians = create_test_guardians(5);

    // Step 3: Prepare distribution (encrypt shares)
    let (encrypted_shards, distribution_status) = secret_manager
        .prepare_distribution(
            &secret_id,
            &creation_result.shares,
            &encryption_key,
            &guardians,
        )
        .unwrap();

    assert_eq!(encrypted_shards.len(), 5);
    assert_eq!(distribution_status.total_shards, 5);

    // Step 4: Create recovery manager and start recovery
    let mut recovery_manager = RecoveryManager::new();
    let session_id = recovery_manager
        .start_recovery_sync(&secret_id, creation_result.secret.shamir_config)
        .unwrap();

    // Step 5: Add threshold number of shards
    for shard in encrypted_shards.iter().take(3) {
        let progress = recovery_manager
            .add_shard(&session_id, shard, &encryption_key)
            .unwrap();

        if progress.shards_received >= 3 {
            assert!(progress.ready_for_reconstruction);
        }
    }

    // Step 6: Reconstruct the secret
    let recovery_result = recovery_manager.reconstruct(&session_id).unwrap();

    assert!(recovery_result.success);
    assert_eq!(recovery_result.recovered_data, Some(secret_data.to_vec()));
    assert_eq!(recovery_result.contributing_guardians.len(), 3);
}

#[test]
fn test_key_derivation_consistency() {
    let password = "test_password_123";
    let salt = ShardManager::generate_salt();

    // Derive key twice with same salt
    let key1 = ShardManager::derive_key_from_password(password, &salt).unwrap();
    let key2 = ShardManager::derive_key_from_password(password, &salt).unwrap();

    // Keys should be identical
    assert_eq!(key1, key2);

    // Different salt should produce different key
    let salt2 = ShardManager::generate_salt();
    let key3 = ShardManager::derive_key_from_password(password, &salt2).unwrap();
    assert_ne!(key1, key3);
}

#[test]
fn test_vault_statistics() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).unwrap();

    // Initially all stats should be zero
    let stats = db.get_vault_stats().unwrap();
    assert_eq!(stats.secret_count, 0);
    assert_eq!(stats.guardian_count, 0);
    assert_eq!(stats.online_guardians, 0);
    assert_eq!(stats.total_shards, 0);

    // Add some data
    let secret = crate::secrets::SecretInfo::new(
        "Stats Test Secret".to_string(),
        SecretType::Generic,
        ShamirConfig::new(2, 3).unwrap(),
    );
    db.create_secret(&secret).unwrap();

    let mut guardian = Guardian::new(
        "Stats Test Guardian".to_string(),
        "stats_peer".to_string(),
        "stats_pubkey".to_string(),
    );
    guardian.status = GuardianStatus::Online;
    db.create_guardian(&guardian).unwrap();

    // Check updated stats
    let stats = db.get_vault_stats().unwrap();
    assert_eq!(stats.secret_count, 1);
    assert_eq!(stats.guardian_count, 1);
    assert_eq!(stats.online_guardians, 1);
}
