//! Secret Recovery Workflow for WRAITH Vault
//!
//! This module implements the recovery process for secrets:
//! - Shard collection from guardians
//! - Threshold verification
//! - Secret reconstruction using Shamir's scheme
//! - Recovery session management
//!
//! Recovery target: < 10 seconds for complete recovery

use crate::error::{VaultError, VaultResult};
use crate::guardian::Guardian;
use crate::shamir::{ShamirConfig, ShamirSecretSharing, Share};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Recovery session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    /// Session created, waiting to start
    Initialized,
    /// Collecting shards from guardians
    CollectingShards,
    /// Sufficient shards collected, ready to reconstruct
    ReadyToReconstruct,
    /// Reconstruction in progress
    Reconstructing,
    /// Recovery completed successfully
    Completed,
    /// Recovery failed
    Failed,
    /// Recovery cancelled by user
    Cancelled,
    /// Session timed out
    TimedOut,
}

impl std::fmt::Display for RecoveryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryState::Initialized => write!(f, "initialized"),
            RecoveryState::CollectingShards => write!(f, "collecting_shards"),
            RecoveryState::ReadyToReconstruct => write!(f, "ready"),
            RecoveryState::Reconstructing => write!(f, "reconstructing"),
            RecoveryState::Completed => write!(f, "completed"),
            RecoveryState::Failed => write!(f, "failed"),
            RecoveryState::Cancelled => write!(f, "cancelled"),
            RecoveryState::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Information about a shard request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardRequest {
    /// Guardian ID to request from
    pub guardian_id: String,
    /// Guardian's peer ID
    pub guardian_peer_id: String,
    /// Expected share index
    pub share_index: u8,
    /// Request timestamp
    pub requested_at: i64,
    /// Response received
    pub received: bool,
    /// Response timestamp
    pub received_at: Option<i64>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error if request failed
    pub error: Option<String>,
}

/// Collected shard during recovery
#[derive(Debug, Clone)]
pub struct CollectedShard {
    /// The decrypted share
    pub share: Share,
    /// Source guardian ID
    pub guardian_id: String,
    /// Collection timestamp
    pub collected_at: i64,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Recovery session for a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySession {
    /// Unique session ID
    pub id: String,
    /// Secret ID being recovered
    pub secret_id: String,
    /// Secret name for display
    pub secret_name: String,
    /// Current state
    pub state: RecoveryState,
    /// Shamir configuration (threshold and total)
    pub shamir_config: ShamirConfig,
    /// Number of shards collected
    pub shards_collected: u32,
    /// Number of shards needed (threshold)
    pub shards_needed: u32,
    /// Shard requests to guardians
    pub requests: Vec<ShardRequest>,
    /// Session creation timestamp
    pub created_at: i64,
    /// Session start timestamp
    pub started_at: Option<i64>,
    /// Session completion timestamp
    pub completed_at: Option<i64>,
    /// Total recovery time in milliseconds
    pub total_time_ms: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Timeout duration in seconds
    pub timeout_seconds: u64,
}

impl RecoverySession {
    /// Create a new recovery session
    pub fn new(
        secret_id: String,
        secret_name: String,
        shamir_config: ShamirConfig,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            secret_id,
            secret_name,
            state: RecoveryState::Initialized,
            shamir_config,
            shards_collected: 0,
            shards_needed: shamir_config.threshold as u32,
            requests: Vec::new(),
            created_at: Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            total_time_ms: None,
            error: None,
            timeout_seconds,
        }
    }

    /// Check if threshold is met
    pub fn threshold_met(&self) -> bool {
        self.shards_collected >= self.shards_needed
    }

    /// Check if session has timed out
    pub fn is_timed_out(&self) -> bool {
        if let Some(started) = self.started_at {
            let elapsed = Utc::now().timestamp() - started;
            elapsed > self.timeout_seconds as i64
        } else {
            false
        }
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            RecoveryState::Initialized
                | RecoveryState::CollectingShards
                | RecoveryState::ReadyToReconstruct
                | RecoveryState::Reconstructing
        )
    }

    /// Calculate progress percentage
    pub fn progress_percent(&self) -> f64 {
        if self.shards_needed == 0 {
            return 100.0;
        }
        ((self.shards_collected as f64 / self.shards_needed as f64) * 100.0).min(100.0)
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecoveryStatistics {
    /// Total recovery attempts
    pub total_attempts: u64,
    /// Successful recoveries
    pub successful: u64,
    /// Failed recoveries
    pub failed: u64,
    /// Timed out recoveries
    pub timed_out: u64,
    /// Average recovery time in milliseconds
    pub avg_recovery_time_ms: u64,
    /// Minimum recovery time in milliseconds
    pub min_recovery_time_ms: u64,
    /// Maximum recovery time in milliseconds
    pub max_recovery_time_ms: u64,
    /// Average shards collected per recovery
    pub avg_shards_collected: f64,
}

impl RecoveryStatistics {
    /// Record a completed recovery
    pub fn record_recovery(&mut self, duration_ms: u64, shards_collected: u32, success: bool) {
        self.total_attempts += 1;

        if success {
            self.successful += 1;

            // Update timing statistics
            if self.successful == 1 {
                self.avg_recovery_time_ms = duration_ms;
                self.min_recovery_time_ms = duration_ms;
                self.max_recovery_time_ms = duration_ms;
            } else {
                // Running average
                self.avg_recovery_time_ms = (self.avg_recovery_time_ms * (self.successful - 1)
                    + duration_ms)
                    / self.successful;
                self.min_recovery_time_ms = self.min_recovery_time_ms.min(duration_ms);
                self.max_recovery_time_ms = self.max_recovery_time_ms.max(duration_ms);
            }
        } else {
            self.failed += 1;
        }

        // Update average shards collected
        let total = self.total_attempts as f64;
        self.avg_shards_collected =
            (self.avg_shards_collected * (total - 1.0) + shards_collected as f64) / total;
    }

    /// Record a timeout
    pub fn record_timeout(&mut self) {
        self.total_attempts += 1;
        self.timed_out += 1;
    }
}

/// Recovery manager for orchestrating secret recovery
pub struct RecoveryManager {
    /// Active recovery sessions
    sessions: Arc<RwLock<HashMap<String, RecoverySession>>>,
    /// Collected shares (session_id -> shares)
    collected_shares: Arc<RwLock<HashMap<String, Vec<CollectedShard>>>>,
    /// Recovery statistics
    statistics: Arc<RwLock<RecoveryStatistics>>,
    /// Default timeout in seconds
    default_timeout_seconds: u64,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            collected_shares: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(RecoveryStatistics::default())),
            default_timeout_seconds: 30, // 30 second default timeout
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout_seconds: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            collected_shares: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(RecoveryStatistics::default())),
            default_timeout_seconds: timeout_seconds,
        }
    }

    /// Start a new recovery session
    pub async fn start_recovery(
        &self,
        secret_id: String,
        secret_name: String,
        shamir_config: ShamirConfig,
        guardians: Vec<Guardian>,
    ) -> VaultResult<RecoverySession> {
        // Check for existing active session for this secret
        {
            let sessions = self.sessions.read().await;
            if let Some(existing) = sessions
                .values()
                .find(|s| s.secret_id == secret_id && s.is_active())
            {
                return Err(VaultError::Recovery(format!(
                    "Active recovery session already exists for secret: {}",
                    existing.id
                )));
            }
        }

        let mut session = RecoverySession::new(
            secret_id.clone(),
            secret_name,
            shamir_config,
            self.default_timeout_seconds,
        );
        session.state = RecoveryState::CollectingShards;
        session.started_at = Some(Utc::now().timestamp());

        // Create shard requests for each guardian
        for (i, guardian) in guardians.iter().enumerate() {
            session.requests.push(ShardRequest {
                guardian_id: guardian.id.clone(),
                guardian_peer_id: guardian.peer_id.clone(),
                share_index: (i + 1) as u8, // Share indices are 1-based
                requested_at: Utc::now().timestamp(),
                received: false,
                received_at: None,
                response_time_ms: None,
                error: None,
            });
        }

        // Store session
        let session_id = session.id.clone();
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        // Initialize collected shares
        {
            let mut collected = self.collected_shares.write().await;
            collected.insert(session_id.clone(), Vec::new());
        }

        tracing::info!(
            "Started recovery session {} for secret {}",
            session_id,
            secret_id
        );

        Ok(session)
    }

    /// Submit a recovered shard
    pub async fn submit_shard(
        &self,
        session_id: &str,
        guardian_id: &str,
        share: Share,
        response_time_ms: u64,
    ) -> VaultResult<RecoverySession> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| VaultError::Recovery(format!("Session not found: {}", session_id)))?;

        // Check session is still active
        if !session.is_active() {
            return Err(VaultError::Recovery(format!(
                "Session {} is not active (state: {})",
                session_id, session.state
            )));
        }

        // Check for timeout
        if session.is_timed_out() {
            session.state = RecoveryState::TimedOut;
            let mut stats = self.statistics.write().await;
            stats.record_timeout();
            return Err(VaultError::Recovery(
                "Recovery session timed out".to_string(),
            ));
        }

        // Find and update the request
        let request = session
            .requests
            .iter_mut()
            .find(|r| r.guardian_id == guardian_id)
            .ok_or_else(|| {
                VaultError::Recovery(format!(
                    "No request for guardian {} in session",
                    guardian_id
                ))
            })?;

        if request.received {
            return Err(VaultError::Recovery(format!(
                "Shard from guardian {} already received",
                guardian_id
            )));
        }

        request.received = true;
        request.received_at = Some(Utc::now().timestamp());
        request.response_time_ms = Some(response_time_ms);
        session.shards_collected += 1;

        // Store collected shard
        drop(sessions); // Release lock before acquiring collected_shares lock
        {
            let mut collected = self.collected_shares.write().await;
            if let Some(shards) = collected.get_mut(session_id) {
                shards.push(CollectedShard {
                    share,
                    guardian_id: guardian_id.to_string(),
                    collected_at: Utc::now().timestamp(),
                    response_time_ms,
                });
            }
        }

        // Re-acquire session lock and check threshold
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).unwrap();

        if session.threshold_met() && session.state == RecoveryState::CollectingShards {
            session.state = RecoveryState::ReadyToReconstruct;
            tracing::info!(
                "Recovery session {} reached threshold with {} shards",
                session_id,
                session.shards_collected
            );
        }

        Ok(session.clone())
    }

    /// Mark a shard request as failed
    pub async fn mark_request_failed(
        &self,
        session_id: &str,
        guardian_id: &str,
        error: &str,
    ) -> VaultResult<RecoverySession> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| VaultError::Recovery(format!("Session not found: {}", session_id)))?;

        let request = session
            .requests
            .iter_mut()
            .find(|r| r.guardian_id == guardian_id)
            .ok_or_else(|| {
                VaultError::Recovery(format!(
                    "No request for guardian {} in session",
                    guardian_id
                ))
            })?;

        request.error = Some(error.to_string());

        Ok(session.clone())
    }

    /// Complete recovery by reconstructing the secret
    pub async fn complete_recovery(
        &self,
        session_id: &str,
        shamir: &ShamirSecretSharing,
    ) -> VaultResult<Vec<u8>> {
        let start = Instant::now();

        // Get session and verify state
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| VaultError::Recovery(format!("Session not found: {}", session_id)))?;

        if session.state != RecoveryState::ReadyToReconstruct {
            return Err(VaultError::Recovery(format!(
                "Session {} not ready for reconstruction (state: {})",
                session_id, session.state
            )));
        }

        session.state = RecoveryState::Reconstructing;
        drop(sessions);

        // Get collected shares
        let collected = self.collected_shares.read().await;
        let shards = collected
            .get(session_id)
            .ok_or_else(|| VaultError::Recovery("No collected shards found".to_string()))?;

        // Extract shares for reconstruction
        let shares: Vec<Share> = shards.iter().map(|c| c.share.clone()).collect();

        // Perform reconstruction
        let secret = shamir.combine(&shares)?;

        let duration = start.elapsed();
        let total_time_ms = duration.as_millis() as u64;

        // Update session state
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).unwrap();
        session.state = RecoveryState::Completed;
        session.completed_at = Some(Utc::now().timestamp());
        session.total_time_ms = Some(total_time_ms);

        // Update statistics
        let shards_collected = session.shards_collected;
        drop(sessions);

        let mut stats = self.statistics.write().await;
        stats.record_recovery(total_time_ms, shards_collected, true);

        tracing::info!(
            "Recovery session {} completed in {} ms",
            session_id,
            total_time_ms
        );

        // Check performance target (< 10 seconds)
        if total_time_ms > 10_000 {
            tracing::warn!(
                "Recovery took {} ms, exceeding 10 second target",
                total_time_ms
            );
        }

        Ok(secret)
    }

    /// Cancel a recovery session
    pub async fn cancel_recovery(&self, session_id: &str) -> VaultResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| VaultError::Recovery(format!("Session not found: {}", session_id)))?;

        if !session.is_active() {
            return Err(VaultError::Recovery(format!(
                "Cannot cancel inactive session (state: {})",
                session.state
            )));
        }

        session.state = RecoveryState::Cancelled;
        session.completed_at = Some(Utc::now().timestamp());

        // Clean up collected shares
        drop(sessions);
        let mut collected = self.collected_shares.write().await;
        collected.remove(session_id);

        tracing::info!("Recovery session {} cancelled", session_id);
        Ok(())
    }

    /// Get recovery session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<RecoverySession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get active session for a secret
    pub async fn get_active_session_for_secret(&self, secret_id: &str) -> Option<RecoverySession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .find(|s| s.secret_id == secret_id && s.is_active())
            .cloned()
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<RecoverySession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get recovery statistics
    pub async fn get_statistics(&self) -> RecoveryStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    /// Clean up old sessions
    pub async fn cleanup_old_sessions(&self, max_age_seconds: i64) {
        let now = Utc::now().timestamp();
        let mut sessions = self.sessions.write().await;
        let mut collected = self.collected_shares.write().await;

        let old_session_ids: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| {
                !s.is_active() && (now - s.completed_at.unwrap_or(s.created_at)) > max_age_seconds
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in old_session_ids {
            sessions.remove(&id);
            collected.remove(&id);
        }
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Synchronous Wrapper Types for IPC Commands
// =============================================================================

/// Recovery progress returned from IPC commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryProgress {
    /// Session ID
    pub session_id: String,
    /// Secret ID being recovered
    pub secret_id: String,
    /// Current recovery state
    pub state: RecoveryState,
    /// Number of shards required (threshold)
    pub threshold_required: u32,
    /// Number of shards received
    pub shards_received: u32,
    /// Number of shards collected (same as received for compatibility)
    pub shards_collected: u32,
    /// Ready for reconstruction
    pub ready_for_reconstruction: bool,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
    /// Guardian IDs that have contributed shards
    pub contributing_guardian_ids: Vec<String>,
    /// Guardian responses
    pub guardian_responses: Vec<GuardianResponse>,
}

/// Guardian response in recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianResponse {
    pub guardian_id: String,
    pub received: bool,
    pub received_at: Option<i64>,
}

/// Recovery result returned from IPC commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// Session ID
    pub session_id: String,
    /// Secret ID recovered
    pub secret_id: String,
    /// Whether recovery was successful
    pub success: bool,
    /// Recovered data (if successful)
    pub recovered_data: Option<Vec<u8>>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Recovery time (same as duration_ms for compatibility)
    pub recovery_time_ms: u64,
    /// Guardian IDs that contributed shards
    pub contributing_guardians: Vec<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl From<&RecoverySession> for RecoveryProgress {
    fn from(session: &RecoverySession) -> Self {
        Self {
            session_id: session.id.clone(),
            secret_id: session.secret_id.clone(),
            state: session.state,
            threshold_required: session.shards_needed,
            shards_received: session.shards_collected,
            shards_collected: session.shards_collected,
            ready_for_reconstruction: session.state == RecoveryState::ReadyToReconstruct,
            elapsed_ms: session.total_time_ms.unwrap_or(0),
            contributing_guardian_ids: session
                .requests
                .iter()
                .filter(|r| r.received)
                .map(|r| r.guardian_id.clone())
                .collect(),
            guardian_responses: session
                .requests
                .iter()
                .map(|r| GuardianResponse {
                    guardian_id: r.guardian_id.clone(),
                    received: r.received,
                    received_at: r.received_at,
                })
                .collect(),
        }
    }
}

// Synchronous interface for commands.rs
impl RecoveryManager {
    /// Start recovery (synchronous wrapper for commands)
    pub fn start_recovery_sync(
        &mut self,
        secret_id: &str,
        shamir_config: crate::shamir::ShamirConfig,
    ) -> crate::error::VaultResult<String> {
        let session = RecoverySession::new(
            secret_id.to_string(),
            String::new(),
            shamir_config,
            self.default_timeout_seconds,
        );
        let session_id = session.id.clone();

        // Use blocking lock for the synchronous interface
        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
            let mut collected = self.collected_shares.write().await;
            collected.insert(session_id.clone(), Vec::new());
        });

        Ok(session_id)
    }

    /// Add shard to recovery (synchronous wrapper for commands)
    pub fn add_shard(
        &mut self,
        session_id: &str,
        shard: &crate::shard::EncryptedShard,
        encryption_key: &[u8; 32],
    ) -> crate::error::VaultResult<RecoveryProgress> {
        let shard_manager = crate::shard::ShardManager::new();

        // Decrypt the shard
        let share = shard_manager.decrypt_shard(shard, encryption_key)?;

        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async {
            // Check if guardian request exists, if not add one dynamically
            {
                let mut sessions = self.sessions.write().await;
                if let Some(session) = sessions.get_mut(session_id) {
                    if !session
                        .requests
                        .iter()
                        .any(|r| r.guardian_id == shard.guardian_id)
                    {
                        // Dynamically add a guardian request
                        session.requests.push(ShardRequest {
                            guardian_id: shard.guardian_id.clone(),
                            guardian_peer_id: String::new(), // Unknown for dynamic registration
                            share_index: shard.share_index,
                            requested_at: chrono::Utc::now().timestamp(),
                            received: false,
                            received_at: None,
                            response_time_ms: None,
                            error: None,
                        });
                        // Start collecting if not already
                        if session.state == RecoveryState::Initialized {
                            session.state = RecoveryState::CollectingShards;
                            session.started_at = Some(chrono::Utc::now().timestamp());
                        }
                    }
                }
            }

            let session = self
                .submit_shard(session_id, &shard.guardian_id, share, 0)
                .await?;
            Ok(RecoveryProgress::from(&session))
        })
    }

    /// Reconstruct secret (synchronous wrapper for commands)
    pub fn reconstruct(&mut self, session_id: &str) -> crate::error::VaultResult<RecoveryResult> {
        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async {
            let session = self.get_session(session_id).await.ok_or_else(|| {
                crate::error::VaultError::Recovery(format!("Session not found: {}", session_id))
            })?;

            let sss = crate::shamir::ShamirSecretSharing::new(session.shamir_config);

            match self.complete_recovery(session_id, &sss).await {
                Ok(data) => {
                    let final_session = self.get_session(session_id).await.unwrap();
                    let duration = final_session.total_time_ms.unwrap_or(0);
                    Ok(RecoveryResult {
                        session_id: session_id.to_string(),
                        secret_id: session.secret_id,
                        success: true,
                        recovered_data: Some(data),
                        duration_ms: duration,
                        recovery_time_ms: duration,
                        contributing_guardians: final_session
                            .requests
                            .iter()
                            .filter(|r| r.received)
                            .map(|r| r.guardian_id.clone())
                            .collect(),
                        error: None,
                    })
                }
                Err(e) => Ok(RecoveryResult {
                    session_id: session_id.to_string(),
                    secret_id: session.secret_id,
                    success: false,
                    recovered_data: None,
                    duration_ms: 0,
                    recovery_time_ms: 0,
                    contributing_guardians: vec![],
                    error: Some(e.to_string()),
                }),
            }
        })
    }

    /// Get progress (synchronous wrapper for commands)
    pub fn get_progress(&self, session_id: &str) -> crate::error::VaultResult<RecoveryProgress> {
        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async {
            let session = self.get_session(session_id).await.ok_or_else(|| {
                crate::error::VaultError::Recovery(format!("Session not found: {}", session_id))
            })?;
            Ok(RecoveryProgress::from(&session))
        })
    }

    /// Cancel session (synchronous wrapper for commands)
    pub fn cancel(&mut self, session_id: &str) -> crate::error::VaultResult<()> {
        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async { self.cancel_recovery(session_id).await })
    }

    /// List active sessions (synchronous wrapper for commands)
    pub fn list_active_sessions(&self) -> Vec<String> {
        let rt = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());

        rt.block_on(async {
            let sessions = self.sessions.read().await;
            sessions
                .iter()
                .filter(|(_, s)| s.is_active())
                .map(|(id, _)| id.clone())
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guardian::GuardianStatus;

    fn create_test_guardian(id: &str, peer_id: &str) -> Guardian {
        let mut g = Guardian::new(
            format!("Guardian {}", id),
            peer_id.to_string(),
            format!("pubkey_{}", id),
        );
        g.id = id.to_string();
        g.status = GuardianStatus::Online;
        g
    }

    #[tokio::test]
    async fn test_start_recovery_session() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(3, 5).unwrap();
        let guardians: Vec<Guardian> = (0..5)
            .map(|i| create_test_guardian(&format!("g{}", i), &format!("peer{}", i)))
            .collect();

        let session = manager
            .start_recovery("secret1".into(), "Test Secret".into(), config, guardians)
            .await
            .unwrap();

        assert_eq!(session.state, RecoveryState::CollectingShards);
        assert_eq!(session.shards_needed, 3);
        assert_eq!(session.shards_collected, 0);
        assert_eq!(session.requests.len(), 5);
    }

    #[tokio::test]
    async fn test_submit_shards() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(2, 3).unwrap();
        let guardians: Vec<Guardian> = (0..3)
            .map(|i| create_test_guardian(&format!("g{}", i), &format!("peer{}", i)))
            .collect();

        let session = manager
            .start_recovery("secret1".into(), "Test".into(), config, guardians)
            .await
            .unwrap();

        // Submit first shard
        let share1 = Share::new(1, vec![1, 2, 3]);
        let updated = manager
            .submit_shard(&session.id, "g0", share1, 50)
            .await
            .unwrap();
        assert_eq!(updated.shards_collected, 1);
        assert_eq!(updated.state, RecoveryState::CollectingShards);

        // Submit second shard (should reach threshold)
        let share2 = Share::new(2, vec![4, 5, 6]);
        let updated = manager
            .submit_shard(&session.id, "g1", share2, 45)
            .await
            .unwrap();
        assert_eq!(updated.shards_collected, 2);
        assert_eq!(updated.state, RecoveryState::ReadyToReconstruct);
    }

    #[tokio::test]
    async fn test_complete_recovery() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);

        // Create shares
        let secret = b"My secret data";
        let shares = sss.split(secret).unwrap();

        let guardians: Vec<Guardian> = (0..3)
            .map(|i| create_test_guardian(&format!("g{}", i), &format!("peer{}", i)))
            .collect();

        let session = manager
            .start_recovery("secret1".into(), "Test".into(), config, guardians)
            .await
            .unwrap();

        // Submit threshold number of shards
        manager
            .submit_shard(&session.id, "g0", shares[0].clone(), 50)
            .await
            .unwrap();
        manager
            .submit_shard(&session.id, "g1", shares[1].clone(), 45)
            .await
            .unwrap();

        // Complete recovery
        let recovered = manager.complete_recovery(&session.id, &sss).await.unwrap();
        assert_eq!(recovered, secret);

        // Verify session state
        let final_session = manager.get_session(&session.id).await.unwrap();
        assert_eq!(final_session.state, RecoveryState::Completed);
        assert!(final_session.total_time_ms.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_shard_submission() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(2, 3).unwrap();
        let guardians: Vec<Guardian> = (0..3)
            .map(|i| create_test_guardian(&format!("g{}", i), &format!("peer{}", i)))
            .collect();

        let session = manager
            .start_recovery("secret1".into(), "Test".into(), config, guardians)
            .await
            .unwrap();

        let share = Share::new(1, vec![1, 2, 3]);
        manager
            .submit_shard(&session.id, "g0", share.clone(), 50)
            .await
            .unwrap();

        // Duplicate submission should fail
        let result = manager.submit_shard(&session.id, "g0", share, 50).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_recovery() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(2, 3).unwrap();
        let guardians: Vec<Guardian> = (0..3)
            .map(|i| create_test_guardian(&format!("g{}", i), &format!("peer{}", i)))
            .collect();

        let session = manager
            .start_recovery("secret1".into(), "Test".into(), config, guardians)
            .await
            .unwrap();

        manager.cancel_recovery(&session.id).await.unwrap();

        let final_session = manager.get_session(&session.id).await.unwrap();
        assert_eq!(final_session.state, RecoveryState::Cancelled);
    }

    #[tokio::test]
    async fn test_recovery_statistics() {
        let manager = RecoveryManager::new();
        let config = ShamirConfig::new(2, 3).unwrap();
        let sss = ShamirSecretSharing::new(config);
        let shares = sss.split(b"secret").unwrap();

        // Perform a few recoveries
        for i in 0..3 {
            let guardians: Vec<Guardian> = (0..3)
                .map(|j| create_test_guardian(&format!("g{}{}", i, j), &format!("peer{}{}", i, j)))
                .collect();

            let session = manager
                .start_recovery(format!("secret{}", i), "Test".into(), config, guardians)
                .await
                .unwrap();

            manager
                .submit_shard(&session.id, &format!("g{}0", i), shares[0].clone(), 50)
                .await
                .unwrap();
            manager
                .submit_shard(&session.id, &format!("g{}1", i), shares[1].clone(), 45)
                .await
                .unwrap();

            manager.complete_recovery(&session.id, &sss).await.unwrap();
        }

        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_attempts, 3);
        assert_eq!(stats.successful, 3);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_recovery_session_progress() {
        let config = ShamirConfig::new(3, 5).unwrap();
        let mut session = RecoverySession::new("s1".into(), "Test".into(), config, 30);

        assert_eq!(session.progress_percent(), 0.0);

        session.shards_collected = 1;
        assert!((session.progress_percent() - 33.33).abs() < 0.1);

        session.shards_collected = 3;
        assert_eq!(session.progress_percent(), 100.0);

        // Even with extra shards, max is 100%
        session.shards_collected = 5;
        assert_eq!(session.progress_percent(), 100.0);
    }

    #[test]
    fn test_session_timeout_check() {
        let config = ShamirConfig::new(2, 3).unwrap();
        let mut session = RecoverySession::new("s1".into(), "Test".into(), config, 1);

        // Not started = not timed out
        assert!(!session.is_timed_out());

        // Just started = not timed out
        session.started_at = Some(Utc::now().timestamp());
        assert!(!session.is_timed_out());

        // Started 2 seconds ago with 1 second timeout = timed out
        session.started_at = Some(Utc::now().timestamp() - 2);
        assert!(session.is_timed_out());
    }
}
