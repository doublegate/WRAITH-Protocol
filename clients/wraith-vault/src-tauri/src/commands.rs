//! Tauri IPC Commands for WRAITH Vault
//!
//! This module provides all the IPC commands for the WRAITH Vault application,
//! including secret management, guardian management, and recovery operations.

use crate::database::VaultStats;
use crate::guardian::{Guardian, GuardianCapabilities, GuardianStatus, HealthCheckResult};
use crate::recovery::{RecoveryProgress, RecoveryResult};
use crate::secrets::{CreateSecretRequest, SecretInfo, SecretType};
use crate::shard::{DistributionStatus, EncryptedShard};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// =============================================================================
// Response Types
// =============================================================================

/// Distribution result returned from prepare_distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    pub shards: Vec<EncryptedShard>,
    pub status: DistributionStatus,
}

/// Secret creation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretCreationResult {
    pub secret: SecretInfo,
    pub encryption_key: String, // Hex-encoded for frontend
    pub distribution_ready: bool,
}

/// Runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    pub secrets_created: u64,
    pub total_recoveries: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub average_recovery_time_ms: Option<f64>,
    pub key_rotations: u64,
    pub shards_distributed: u64,
    pub recovery_success_rate: Option<f64>,
}

// =============================================================================
// Secret Commands
// =============================================================================

/// Create a new secret with Shamir secret sharing
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_secret(
    state: State<'_, Arc<AppState>>,
    name: String,
    secret_data: Vec<u8>,
    secret_type: String,
    description: Option<String>,
    threshold: u8,
    total_shares: u8,
    tags: Vec<String>,
    password: Option<String>,
) -> Result<SecretCreationResult, String> {
    let request = CreateSecretRequest {
        name,
        secret_data,
        secret_type: parse_secret_type(&secret_type),
        description,
        threshold,
        total_shares,
        tags,
        password,
    };

    let result = {
        let mut secrets = state.secrets.lock().await;
        secrets.create_secret(request).map_err(|e| e.to_string())?
    };

    // Save to database
    {
        let db = state.db.lock();
        db.create_secret(&result.secret)
            .map_err(|e| e.to_string())?;
    }

    // Update statistics
    state.statistics.record_secret_created();

    Ok(SecretCreationResult {
        secret: result.secret,
        encryption_key: hex::encode(result.encryption_key),
        distribution_ready: true,
    })
}

/// Get a secret by ID
#[tauri::command]
pub async fn get_secret(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
) -> Result<Option<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    match secrets.get_secret(&secret_id) {
        Ok(secret) => Ok(Some(secret)),
        Err(_) => {
            // Try database
            let db = state.db.lock();
            db.get_secret(&secret_id).map_err(|e| e.to_string())
        }
    }
}

/// List all secrets
#[tauri::command]
pub async fn list_secrets(state: State<'_, Arc<AppState>>) -> Result<Vec<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    Ok(secrets.list_secrets())
}

/// List secrets by type
#[tauri::command]
pub async fn list_secrets_by_type(
    state: State<'_, Arc<AppState>>,
    secret_type: String,
) -> Result<Vec<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    Ok(secrets.list_secrets_by_type(parse_secret_type(&secret_type)))
}

/// List secrets by tag
#[tauri::command]
pub async fn list_secrets_by_tag(
    state: State<'_, Arc<AppState>>,
    tag: String,
) -> Result<Vec<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    Ok(secrets.list_secrets_by_tag(&tag))
}

/// Search secrets by name or description
#[tauri::command]
pub async fn search_secrets(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    Ok(secrets.search_secrets(&query))
}

/// Update secret metadata
#[tauri::command]
pub async fn update_secret(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
    name: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<SecretInfo, String> {
    let updated = {
        let mut secrets = state.secrets.lock().await;
        secrets
            .update_secret(&secret_id, name, description, tags)
            .map_err(|e| e.to_string())?
    };

    // Update database
    {
        let db = state.db.lock();
        db.update_secret(&updated).map_err(|e| e.to_string())?;
    }

    Ok(updated)
}

/// Delete a secret
#[tauri::command]
pub async fn delete_secret(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
) -> Result<(), String> {
    // Delete from manager
    {
        let mut secrets = state.secrets.lock().await;
        secrets
            .delete_secret(&secret_id)
            .map_err(|e| e.to_string())?;
    }

    // Delete from database
    {
        let db = state.db.lock();
        db.delete_secret(&secret_id).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get secrets needing rotation
#[tauri::command]
pub async fn get_secrets_needing_rotation(
    state: State<'_, Arc<AppState>>,
    max_age_days: i64,
) -> Result<Vec<SecretInfo>, String> {
    let secrets = state.secrets.lock().await;
    Ok(secrets.get_secrets_needing_rotation(max_age_days))
}

// =============================================================================
// Guardian Commands
// =============================================================================

/// Add a new guardian
#[tauri::command]
pub async fn add_guardian(
    state: State<'_, Arc<AppState>>,
    name: String,
    peer_id: String,
    public_key: String,
    notes: Option<String>,
) -> Result<Guardian, String> {
    let mut guardian = Guardian::new(name, peer_id, public_key);
    guardian.notes = notes;
    guardian.capabilities = GuardianCapabilities {
        can_store: true,
        can_recover: true,
        max_storage: 0, // Unlimited
        supports_encryption: true,
        supports_auto_refresh: false,
    };

    // Add to manager
    state
        .guardians
        .add_guardian(guardian.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Save to database
    {
        let db = state.db.lock();
        db.create_guardian(&guardian).map_err(|e| e.to_string())?;
    }

    Ok(guardian)
}

/// Get a guardian by ID
#[tauri::command]
pub async fn get_guardian(
    state: State<'_, Arc<AppState>>,
    guardian_id: String,
) -> Result<Guardian, String> {
    state
        .guardians
        .get_guardian(&guardian_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get a guardian by peer ID
#[tauri::command]
pub async fn get_guardian_by_peer_id(
    state: State<'_, Arc<AppState>>,
    peer_id: String,
) -> Result<Guardian, String> {
    state
        .guardians
        .get_guardian_by_peer_id(&peer_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all guardians
#[tauri::command]
pub async fn list_guardians(state: State<'_, Arc<AppState>>) -> Result<Vec<Guardian>, String> {
    Ok(state.guardians.list_guardians().await)
}

/// List guardians by status
#[tauri::command]
pub async fn list_guardians_by_status(
    state: State<'_, Arc<AppState>>,
    status: String,
) -> Result<Vec<Guardian>, String> {
    let status = parse_guardian_status(&status);
    Ok(state.guardians.list_guardians_by_status(status).await)
}

/// List available guardians (online and trusted)
#[tauri::command]
pub async fn list_available_guardians(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Guardian>, String> {
    Ok(state.guardians.list_available_guardians().await)
}

/// Update a guardian
#[tauri::command]
pub async fn update_guardian(
    state: State<'_, Arc<AppState>>,
    guardian: Guardian,
) -> Result<(), String> {
    // Update in manager
    state
        .guardians
        .update_guardian(guardian.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Update in database
    {
        let db = state.db.lock();
        db.update_guardian(&guardian).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Update guardian status
#[tauri::command]
pub async fn update_guardian_status(
    state: State<'_, Arc<AppState>>,
    guardian_id: String,
    status: String,
) -> Result<(), String> {
    let status = parse_guardian_status(&status);

    // Update in manager
    state
        .guardians
        .update_guardian_status(&guardian_id, status)
        .await
        .map_err(|e| e.to_string())?;

    // Get updated guardian and save to database
    let guardian = state
        .guardians
        .get_guardian(&guardian_id)
        .await
        .map_err(|e| e.to_string())?;

    {
        let db = state.db.lock();
        db.update_guardian(&guardian).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Remove a guardian
#[tauri::command]
pub async fn remove_guardian(
    state: State<'_, Arc<AppState>>,
    guardian_id: String,
) -> Result<Guardian, String> {
    // Remove from manager
    let guardian = state
        .guardians
        .remove_guardian(&guardian_id)
        .await
        .map_err(|e| e.to_string())?;

    // Remove from database
    {
        let db = state.db.lock();
        db.delete_guardian(&guardian_id)
            .map_err(|e| e.to_string())?;
    }

    Ok(guardian)
}

/// Record health check result
#[tauri::command]
pub async fn record_health_check(
    state: State<'_, Arc<AppState>>,
    guardian_id: String,
    success: bool,
    response_time_ms: Option<u64>,
    error: Option<String>,
) -> Result<(), String> {
    let result = HealthCheckResult {
        guardian_id: guardian_id.clone(),
        success,
        response_time_ms,
        error,
        checked_at: Utc::now().timestamp(),
    };

    // Record in manager
    state
        .guardians
        .record_health_check(&result)
        .await
        .map_err(|e| e.to_string())?;

    // Update guardian in database
    let guardian = state
        .guardians
        .get_guardian(&guardian_id)
        .await
        .map_err(|e| e.to_string())?;

    {
        let db = state.db.lock();
        db.update_guardian(&guardian).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Select guardians for distribution
#[tauri::command]
pub async fn select_guardians_for_distribution(
    state: State<'_, Arc<AppState>>,
    count: usize,
) -> Result<Vec<Guardian>, String> {
    state
        .guardians
        .select_guardians_for_distribution(count)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// Distribution Commands
// =============================================================================

/// Prepare secret for distribution
#[tauri::command]
pub async fn prepare_distribution(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
    encryption_key: String,
    guardian_ids: Vec<String>,
) -> Result<DistributionResult, String> {
    // Decode encryption key
    let key_bytes =
        hex::decode(&encryption_key).map_err(|e| format!("Invalid encryption key: {}", e))?;
    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }
    let mut encryption_key_arr = [0u8; 32];
    encryption_key_arr.copy_from_slice(&key_bytes);

    // Get guardians
    let mut guardians = Vec::new();
    for id in &guardian_ids {
        let guardian = state
            .guardians
            .get_guardian(id)
            .await
            .map_err(|e| e.to_string())?;
        guardians.push(guardian);
    }

    // Get secret and prepare distribution
    let (shards, status) = {
        let mut secrets = state.secrets.lock().await;
        let secret = secrets.get_secret(&secret_id).map_err(|e| e.to_string())?;

        // Create shares from secret data
        let sss = crate::shamir::ShamirSecretSharing::new(secret.shamir_config);

        // Note: In a real implementation, we'd retrieve the actual secret data
        // For now, we create placeholder shares as the secret data is not stored locally
        let placeholder_data = vec![0u8; 32]; // This would come from the user/storage
        let shares = sss.split(&placeholder_data).map_err(|e| e.to_string())?;

        secrets
            .prepare_distribution(&secret_id, &shares, &encryption_key_arr, &guardians)
            .map_err(|e| e.to_string())?
    };

    // Store shards in database
    {
        let db = state.db.lock();
        for shard in &shards {
            db.store_encrypted_shard(shard).map_err(|e| e.to_string())?;
        }
        db.upsert_distribution_status(&status)
            .map_err(|e| e.to_string())?;
    }

    // Update statistics
    state
        .statistics
        .record_shards_distributed(shards.len() as u64);

    Ok(DistributionResult { shards, status })
}

/// Mark shard as delivered to guardian
#[tauri::command]
pub async fn mark_shard_delivered(
    state: State<'_, Arc<AppState>>,
    shard_id: String,
    guardian_id: String,
) -> Result<(), String> {
    let db = state.db.lock();
    db.update_shard_assignment_delivery(&shard_id, &guardian_id, true, None)
        .map_err(|e| e.to_string())
}

/// Get distribution status for a secret
#[tauri::command]
pub async fn get_distribution_status(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
) -> Result<Option<DistributionStatus>, String> {
    let db = state.db.lock();
    db.get_distribution_status(&secret_id)
        .map_err(|e| e.to_string())
}

/// Get shard assignments for a secret
#[tauri::command]
pub async fn get_shard_assignments(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
) -> Result<Vec<crate::shard::ShardAssignment>, String> {
    let db = state.db.lock();
    db.get_shard_assignments(&secret_id)
        .map_err(|e| e.to_string())
}

/// Request a shard from a guardian
#[tauri::command]
pub async fn request_shard_from_guardian(
    state: State<'_, Arc<AppState>>,
    guardian_id: String,
    secret_id: String,
) -> Result<Option<EncryptedShard>, String> {
    // In a real implementation, this would:
    // 1. Connect to the guardian via P2P network
    // 2. Request the shard using the guardian's public key
    // 3. Return the encrypted shard

    // For now, try to get the shard from local storage
    let db = state.db.lock();
    db.get_shard_by_guardian(&secret_id, &guardian_id)
        .map_err(|e| e.to_string())
}

// =============================================================================
// Recovery Commands
// =============================================================================

/// Start a recovery session
#[tauri::command]
pub async fn start_recovery(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
) -> Result<String, String> {
    // Get secret configuration
    let shamir_config = {
        let secrets = state.secrets.lock().await;
        let secret = secrets.get_secret(&secret_id).map_err(|e| e.to_string())?;
        secret.shamir_config
    };

    // Start recovery session
    let mut recovery = state.recovery.lock().await;
    let session_id = recovery
        .start_recovery_sync(&secret_id, shamir_config)
        .map_err(|e| e.to_string())?;

    Ok(session_id)
}

/// Add a shard to recovery session
#[tauri::command]
pub async fn add_recovery_shard(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    shard: EncryptedShard,
    encryption_key: String,
) -> Result<RecoveryProgress, String> {
    // Decode encryption key
    let key_bytes =
        hex::decode(&encryption_key).map_err(|e| format!("Invalid encryption key: {}", e))?;
    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }
    let mut encryption_key_arr = [0u8; 32];
    encryption_key_arr.copy_from_slice(&key_bytes);

    let mut recovery = state.recovery.lock().await;
    recovery
        .add_shard(&session_id, &shard, &encryption_key_arr)
        .map_err(|e| e.to_string())
}

/// Complete recovery and reconstruct secret
#[tauri::command]
pub async fn complete_recovery(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<RecoveryResult, String> {
    let start_time = std::time::Instant::now();

    let result = {
        let mut recovery = state.recovery.lock().await;
        recovery
            .reconstruct(&session_id)
            .map_err(|e| e.to_string())?
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Update statistics
    state
        .statistics
        .record_recovery(result.success, duration_ms);

    // Update guardian trust levels on success
    if result.success {
        for guardian_id in &result.contributing_guardians {
            if let Ok(mut guardian) = state.guardians.get_guardian(guardian_id).await {
                guardian.record_successful_recovery();
                let _ = state.guardians.update_guardian(guardian.clone()).await;

                // Update in database
                let db = state.db.lock();
                let _ = db.update_guardian(&guardian);
            }
        }

        // Mark secret as accessed
        let mut secrets = state.secrets.lock().await;
        if let Ok(secret) = secrets.get_secret_mut(&result.secret_id) {
            secret.mark_accessed();
            let db = state.db.lock();
            let _ = db.update_secret(secret);
        }
    }

    Ok(result)
}

/// Get recovery session progress
#[tauri::command]
pub async fn get_recovery_progress(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<RecoveryProgress, String> {
    let recovery = state.recovery.lock().await;
    recovery
        .get_progress(&session_id)
        .map_err(|e| e.to_string())
}

/// Cancel a recovery session
#[tauri::command]
pub async fn cancel_recovery(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<(), String> {
    let mut recovery = state.recovery.lock().await;
    recovery.cancel(&session_id).map_err(|e| e.to_string())
}

/// List active recovery sessions
#[tauri::command]
pub async fn list_recovery_sessions(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let recovery = state.recovery.lock().await;
    Ok(recovery.list_active_sessions())
}

// =============================================================================
// Key Rotation Commands
// =============================================================================

/// Rotate key for a secret
#[tauri::command]
pub async fn rotate_secret_key(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
    recovered_secret: Vec<u8>,
    new_password: Option<String>,
) -> Result<SecretCreationResult, String> {
    let result = {
        let mut secrets = state.secrets.lock().await;
        secrets
            .rotate_key(&secret_id, &recovered_secret, new_password.as_deref())
            .map_err(|e| e.to_string())?
    };

    // Update statistics
    state.statistics.record_key_rotation();

    Ok(SecretCreationResult {
        secret: result.secret,
        encryption_key: hex::encode(result.encryption_key),
        distribution_ready: true,
    })
}

/// Record key rotation completion
#[tauri::command]
pub async fn record_rotation(
    state: State<'_, Arc<AppState>>,
    secret_id: String,
    guardian_ids: Vec<String>,
) -> Result<SecretInfo, String> {
    let updated = {
        let mut secrets = state.secrets.lock().await;
        secrets
            .record_rotation(&secret_id, guardian_ids)
            .map_err(|e| e.to_string())?
    };

    // Update database
    {
        let db = state.db.lock();
        db.update_secret(&updated).map_err(|e| e.to_string())?;
    }

    Ok(updated)
}

// =============================================================================
// Node Commands
// =============================================================================

/// Start the WRAITH node
#[tauri::command]
pub async fn start_node(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut node = state.node.lock().await;
    if node.node().is_none() {
        node.initialize().await?;
    }
    node.start().await
}

/// Stop the WRAITH node
#[tauri::command]
pub async fn stop_node(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut node = state.node.lock().await;
    node.stop().await
}

/// Get node status
#[tauri::command]
pub async fn get_node_status(state: State<'_, Arc<AppState>>) -> Result<NodeStatus, String> {
    let node = state.node.lock().await;
    Ok(NodeStatus {
        running: node.is_running(),
        peer_id: node.peer_id(),
        active_routes: node.active_route_count(),
    })
}

/// Get local peer ID
#[tauri::command]
pub async fn get_peer_id(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let node = state.node.lock().await;
    Ok(node.peer_id())
}

// =============================================================================
// Statistics Commands
// =============================================================================

/// Get vault statistics
#[tauri::command]
pub async fn get_vault_stats(state: State<'_, Arc<AppState>>) -> Result<VaultStats, String> {
    state.get_vault_stats()
}

/// Get runtime statistics
#[tauri::command]
pub async fn get_runtime_statistics(
    state: State<'_, Arc<AppState>>,
) -> Result<RuntimeStatistics, String> {
    let stats = &state.statistics;
    Ok(RuntimeStatistics {
        secrets_created: stats.secrets_created(),
        total_recoveries: stats.total_recoveries(),
        successful_recoveries: stats.successful_recoveries(),
        failed_recoveries: stats.failed_recoveries(),
        average_recovery_time_ms: stats.average_recovery_time_ms(),
        key_rotations: stats.key_rotations(),
        shards_distributed: stats.shards_distributed(),
        recovery_success_rate: stats.recovery_success_rate(),
    })
}

// =============================================================================
// Helper Types
// =============================================================================

/// Node status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub peer_id: Option<String>,
    pub active_routes: usize,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_secret_type(s: &str) -> SecretType {
    match s.to_lowercase().as_str() {
        "generic" => SecretType::Generic,
        "cryptokey" | "crypto_key" => SecretType::CryptoKey,
        "password" => SecretType::Password,
        "recoveryphrase" | "recovery_phrase" => SecretType::RecoveryPhrase,
        "certificate" => SecretType::Certificate,
        "apikey" | "api_key" => SecretType::ApiKey,
        "documentkey" | "document_key" => SecretType::DocumentKey,
        "sshkey" | "ssh_key" => SecretType::SshKey,
        "pgpkey" | "pgp_key" => SecretType::PgpKey,
        _ => SecretType::Generic,
    }
}

fn parse_guardian_status(s: &str) -> GuardianStatus {
    match s.to_lowercase().as_str() {
        "online" => GuardianStatus::Online,
        "offline" => GuardianStatus::Offline,
        "pending" => GuardianStatus::Pending,
        "failed" => GuardianStatus::Failed,
        "revoked" => GuardianStatus::Revoked,
        _ => GuardianStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_secret_type() {
        assert!(matches!(parse_secret_type("generic"), SecretType::Generic));
        assert!(matches!(
            parse_secret_type("password"),
            SecretType::Password
        ));
        assert!(matches!(
            parse_secret_type("crypto_key"),
            SecretType::CryptoKey
        ));
        assert!(matches!(
            parse_secret_type("CryptoKey"),
            SecretType::CryptoKey
        ));
        assert!(matches!(parse_secret_type("unknown"), SecretType::Generic));
    }

    #[test]
    fn test_parse_guardian_status() {
        assert!(matches!(
            parse_guardian_status("online"),
            GuardianStatus::Online
        ));
        assert!(matches!(
            parse_guardian_status("offline"),
            GuardianStatus::Offline
        ));
        assert!(matches!(
            parse_guardian_status("PENDING"),
            GuardianStatus::Pending
        ));
        assert!(matches!(
            parse_guardian_status("unknown"),
            GuardianStatus::Pending
        ));
    }
}
