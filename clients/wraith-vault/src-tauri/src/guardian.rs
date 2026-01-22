//! Guardian Peer Management for WRAITH Vault
//!
//! Guardians are trusted peers who hold shares of secrets. This module manages:
//! - Guardian registration and verification
//! - Trust levels and capabilities
//! - Health monitoring and availability
//! - Secure communication with guardians

use crate::error::{VaultError, VaultResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Guardian status indicating availability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuardianStatus {
    /// Guardian is online and responsive
    Online,
    /// Guardian is offline or unreachable
    Offline,
    /// Guardian is being verified
    Pending,
    /// Guardian connection failed
    Failed,
    /// Guardian has been revoked
    Revoked,
}

impl std::fmt::Display for GuardianStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardianStatus::Online => write!(f, "online"),
            GuardianStatus::Offline => write!(f, "offline"),
            GuardianStatus::Pending => write!(f, "pending"),
            GuardianStatus::Failed => write!(f, "failed"),
            GuardianStatus::Revoked => write!(f, "revoked"),
        }
    }
}

/// Trust level for a guardian
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    /// Untrusted guardian (pending verification)
    #[default]
    Untrusted = 0,
    /// Basic trust (verified identity)
    Basic = 1,
    /// Trusted (multiple successful interactions)
    Trusted = 2,
    /// High trust (long-term relationship)
    High = 3,
    /// Ultimate trust (e.g., self or hardware device)
    Ultimate = 4,
}

/// Guardian capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardianCapabilities {
    /// Can store secret shares
    pub can_store: bool,
    /// Can participate in recovery
    pub can_recover: bool,
    /// Maximum storage in bytes (0 = unlimited)
    pub max_storage: u64,
    /// Supports encrypted channels
    pub supports_encryption: bool,
    /// Supports automatic refresh
    pub supports_auto_refresh: bool,
}

/// Information about a guardian peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guardian {
    /// Unique identifier for the guardian
    pub id: String,
    /// Human-readable name/alias
    pub name: String,
    /// WRAITH peer ID for network communication
    pub peer_id: String,
    /// Guardian's public key (Ed25519)
    pub public_key: String,
    /// Current status
    pub status: GuardianStatus,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Guardian capabilities
    pub capabilities: GuardianCapabilities,
    /// When the guardian was added
    pub created_at: i64,
    /// Last successful contact timestamp
    pub last_seen: Option<i64>,
    /// Last health check timestamp
    pub last_health_check: Option<i64>,
    /// Number of shares currently held by this guardian
    pub shares_held: u32,
    /// Number of successful recoveries assisted
    pub successful_recoveries: u32,
    /// Optional notes about the guardian
    pub notes: Option<String>,
}

impl Guardian {
    /// Create a new guardian
    pub fn new(name: String, peer_id: String, public_key: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            peer_id,
            public_key,
            status: GuardianStatus::Pending,
            trust_level: TrustLevel::Untrusted,
            capabilities: GuardianCapabilities::default(),
            created_at: Utc::now().timestamp(),
            last_seen: None,
            last_health_check: None,
            shares_held: 0,
            successful_recoveries: 0,
            notes: None,
        }
    }

    /// Check if guardian is available for operations
    pub fn is_available(&self) -> bool {
        matches!(self.status, GuardianStatus::Online) && self.trust_level >= TrustLevel::Basic
    }

    /// Check if guardian can store shares
    pub fn can_store_shares(&self) -> bool {
        self.is_available() && self.capabilities.can_store
    }

    /// Check if guardian can participate in recovery
    pub fn can_participate_in_recovery(&self) -> bool {
        self.is_available() && self.capabilities.can_recover
    }

    /// Update guardian status
    pub fn update_status(&mut self, status: GuardianStatus) {
        self.status = status;
        if status == GuardianStatus::Online {
            self.last_seen = Some(Utc::now().timestamp());
        }
    }

    /// Increment shares held count
    pub fn add_share(&mut self) {
        self.shares_held += 1;
    }

    /// Decrement shares held count
    pub fn remove_share(&mut self) {
        if self.shares_held > 0 {
            self.shares_held -= 1;
        }
    }

    /// Record a successful recovery
    pub fn record_successful_recovery(&mut self) {
        self.successful_recoveries += 1;
        // Automatically upgrade trust level based on successful recoveries
        if self.successful_recoveries >= 10 && self.trust_level < TrustLevel::High {
            self.trust_level = TrustLevel::High;
        } else if self.successful_recoveries >= 3 && self.trust_level < TrustLevel::Trusted {
            self.trust_level = TrustLevel::Trusted;
        }
    }
}

/// Result of a guardian health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Guardian ID
    pub guardian_id: String,
    /// Whether the check was successful
    pub success: bool,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Timestamp of the check
    pub checked_at: i64,
}

/// Guardian selection strategy for share distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// Select guardians with highest trust
    #[default]
    HighestTrust,
    /// Round-robin distribution
    RoundRobin,
    /// Random selection
    Random,
    /// Geographic diversity (when location is known)
    Geographic,
    /// Most recently seen first
    MostRecentlySeen,
}

/// Guardian manager for handling guardian operations
pub struct GuardianManager {
    /// Cached guardians (ID -> Guardian)
    guardians: Arc<RwLock<HashMap<String, Guardian>>>,
    /// Selection strategy for share distribution
    selection_strategy: SelectionStrategy,
}

impl GuardianManager {
    /// Create a new guardian manager
    pub fn new() -> Self {
        Self {
            guardians: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy: SelectionStrategy::default(),
        }
    }

    /// Create with a specific selection strategy
    pub fn with_strategy(strategy: SelectionStrategy) -> Self {
        Self {
            guardians: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy: strategy,
        }
    }

    /// Add a new guardian
    pub async fn add_guardian(&self, guardian: Guardian) -> VaultResult<()> {
        let mut guardians = self.guardians.write().await;

        // Check for duplicate peer ID
        if guardians
            .values()
            .any(|g| g.peer_id == guardian.peer_id && g.id != guardian.id)
        {
            return Err(VaultError::Guardian(format!(
                "Guardian with peer ID {} already exists",
                guardian.peer_id
            )));
        }

        guardians.insert(guardian.id.clone(), guardian);
        Ok(())
    }

    /// Get a guardian by ID
    pub async fn get_guardian(&self, id: &str) -> VaultResult<Guardian> {
        let guardians = self.guardians.read().await;
        guardians
            .get(id)
            .cloned()
            .ok_or_else(|| VaultError::GuardianNotFound(id.to_string()))
    }

    /// Get a guardian by peer ID
    pub async fn get_guardian_by_peer_id(&self, peer_id: &str) -> VaultResult<Guardian> {
        let guardians = self.guardians.read().await;
        guardians
            .values()
            .find(|g| g.peer_id == peer_id)
            .cloned()
            .ok_or_else(|| VaultError::GuardianNotFound(format!("peer:{}", peer_id)))
    }

    /// Update a guardian
    pub async fn update_guardian(&self, guardian: Guardian) -> VaultResult<()> {
        let mut guardians = self.guardians.write().await;

        if !guardians.contains_key(&guardian.id) {
            return Err(VaultError::GuardianNotFound(guardian.id.clone()));
        }

        guardians.insert(guardian.id.clone(), guardian);
        Ok(())
    }

    /// Remove a guardian
    pub async fn remove_guardian(&self, id: &str) -> VaultResult<Guardian> {
        let mut guardians = self.guardians.write().await;
        guardians
            .remove(id)
            .ok_or_else(|| VaultError::GuardianNotFound(id.to_string()))
    }

    /// List all guardians
    pub async fn list_guardians(&self) -> Vec<Guardian> {
        let guardians = self.guardians.read().await;
        guardians.values().cloned().collect()
    }

    /// List guardians with a specific status
    pub async fn list_guardians_by_status(&self, status: GuardianStatus) -> Vec<Guardian> {
        let guardians = self.guardians.read().await;
        guardians
            .values()
            .filter(|g| g.status == status)
            .cloned()
            .collect()
    }

    /// List available guardians (online and trusted)
    pub async fn list_available_guardians(&self) -> Vec<Guardian> {
        let guardians = self.guardians.read().await;
        guardians
            .values()
            .filter(|g| g.is_available())
            .cloned()
            .collect()
    }

    /// Select guardians for share distribution
    pub async fn select_guardians_for_distribution(
        &self,
        count: usize,
    ) -> VaultResult<Vec<Guardian>> {
        let available = self.list_available_guardians().await;

        if available.len() < count {
            return Err(VaultError::Guardian(format!(
                "Not enough available guardians: need {}, have {}",
                count,
                available.len()
            )));
        }

        let mut selected = available;

        match self.selection_strategy {
            SelectionStrategy::HighestTrust => {
                // Sort by trust level (descending), then by successful recoveries
                selected.sort_by(|a, b| {
                    b.trust_level
                        .cmp(&a.trust_level)
                        .then_with(|| b.successful_recoveries.cmp(&a.successful_recoveries))
                });
            }
            SelectionStrategy::MostRecentlySeen => {
                // Sort by last seen (most recent first)
                selected.sort_by(|a, b| b.last_seen.unwrap_or(0).cmp(&a.last_seen.unwrap_or(0)));
            }
            SelectionStrategy::Random => {
                // Shuffle using Fisher-Yates
                use rand::seq::SliceRandom;
                selected.shuffle(&mut rand::thread_rng());
            }
            SelectionStrategy::RoundRobin => {
                // Sort by shares held (fewest first) for load balancing
                selected.sort_by(|a, b| a.shares_held.cmp(&b.shares_held));
            }
            SelectionStrategy::Geographic => {
                // For now, fall back to highest trust (geographic info not implemented)
                selected.sort_by(|a, b| b.trust_level.cmp(&a.trust_level));
            }
        }

        selected.truncate(count);
        Ok(selected)
    }

    /// Update guardian status
    pub async fn update_guardian_status(
        &self,
        id: &str,
        status: GuardianStatus,
    ) -> VaultResult<()> {
        let mut guardians = self.guardians.write().await;

        let guardian = guardians
            .get_mut(id)
            .ok_or_else(|| VaultError::GuardianNotFound(id.to_string()))?;

        guardian.update_status(status);
        Ok(())
    }

    /// Record health check result
    pub async fn record_health_check(&self, result: &HealthCheckResult) -> VaultResult<()> {
        let mut guardians = self.guardians.write().await;

        let guardian = guardians
            .get_mut(&result.guardian_id)
            .ok_or_else(|| VaultError::GuardianNotFound(result.guardian_id.clone()))?;

        guardian.last_health_check = Some(result.checked_at);

        if result.success {
            guardian.update_status(GuardianStatus::Online);
        } else {
            guardian.update_status(GuardianStatus::Failed);
        }

        Ok(())
    }

    /// Get guardian count by status
    pub async fn get_status_counts(&self) -> HashMap<GuardianStatus, usize> {
        let guardians = self.guardians.read().await;
        let mut counts = HashMap::new();

        for guardian in guardians.values() {
            *counts.entry(guardian.status).or_insert(0) += 1;
        }

        counts
    }

    /// Load guardians from database records
    pub async fn load_guardians(&self, guardians_data: Vec<Guardian>) {
        let mut guardians = self.guardians.write().await;
        for guardian in guardians_data {
            guardians.insert(guardian.id.clone(), guardian);
        }
    }

    /// Clear all guardians (for testing)
    #[cfg(test)]
    pub async fn clear(&self) {
        let mut guardians = self.guardians.write().await;
        guardians.clear();
    }
}

impl Default for GuardianManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_guardian(name: &str, peer_id: &str) -> Guardian {
        let mut guardian = Guardian::new(
            name.to_string(),
            peer_id.to_string(),
            format!("pubkey_{}", peer_id),
        );
        guardian.status = GuardianStatus::Online;
        guardian.trust_level = TrustLevel::Basic;
        guardian.capabilities.can_store = true;
        guardian.capabilities.can_recover = true;
        guardian
    }

    #[tokio::test]
    async fn test_add_and_get_guardian() {
        let manager = GuardianManager::new();
        let guardian = create_test_guardian("Test Guardian", "peer123");
        let id = guardian.id.clone();

        manager.add_guardian(guardian).await.unwrap();

        let retrieved = manager.get_guardian(&id).await.unwrap();
        assert_eq!(retrieved.name, "Test Guardian");
        assert_eq!(retrieved.peer_id, "peer123");
    }

    #[tokio::test]
    async fn test_duplicate_peer_id() {
        let manager = GuardianManager::new();

        let guardian1 = create_test_guardian("Guardian 1", "peer123");
        let guardian2 = create_test_guardian("Guardian 2", "peer123");

        manager.add_guardian(guardian1).await.unwrap();
        let result = manager.add_guardian(guardian2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_available_guardians() {
        let manager = GuardianManager::new();

        let mut online_guardian = create_test_guardian("Online", "peer1");
        online_guardian.status = GuardianStatus::Online;
        online_guardian.trust_level = TrustLevel::Basic;

        let mut offline_guardian = create_test_guardian("Offline", "peer2");
        offline_guardian.status = GuardianStatus::Offline;

        let mut untrusted_guardian = create_test_guardian("Untrusted", "peer3");
        untrusted_guardian.trust_level = TrustLevel::Untrusted;

        manager.add_guardian(online_guardian).await.unwrap();
        manager.add_guardian(offline_guardian).await.unwrap();
        manager.add_guardian(untrusted_guardian).await.unwrap();

        let available = manager.list_available_guardians().await;
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "Online");
    }

    #[tokio::test]
    async fn test_select_guardians_by_trust() {
        let manager = GuardianManager::with_strategy(SelectionStrategy::HighestTrust);

        for i in 0..5 {
            let mut guardian =
                create_test_guardian(&format!("Guardian {}", i), &format!("peer{}", i));
            guardian.trust_level = match i {
                0 => TrustLevel::Basic,
                1 => TrustLevel::High,
                2 => TrustLevel::Trusted,
                3 => TrustLevel::Ultimate,
                _ => TrustLevel::Basic,
            };
            manager.add_guardian(guardian).await.unwrap();
        }

        let selected = manager.select_guardians_for_distribution(3).await.unwrap();

        assert_eq!(selected.len(), 3);
        // Should be sorted by trust: Ultimate, High, Trusted
        assert_eq!(selected[0].trust_level, TrustLevel::Ultimate);
        assert_eq!(selected[1].trust_level, TrustLevel::High);
        assert_eq!(selected[2].trust_level, TrustLevel::Trusted);
    }

    #[tokio::test]
    async fn test_select_guardians_round_robin() {
        let manager = GuardianManager::with_strategy(SelectionStrategy::RoundRobin);

        for i in 0..5 {
            let mut guardian =
                create_test_guardian(&format!("Guardian {}", i), &format!("peer{}", i));
            guardian.shares_held = (5 - i) as u32; // Reverse order
            manager.add_guardian(guardian).await.unwrap();
        }

        let selected = manager.select_guardians_for_distribution(3).await.unwrap();

        assert_eq!(selected.len(), 3);
        // Should prefer guardians with fewer shares
        assert!(selected[0].shares_held <= selected[1].shares_held);
        assert!(selected[1].shares_held <= selected[2].shares_held);
    }

    #[tokio::test]
    async fn test_insufficient_guardians() {
        let manager = GuardianManager::new();

        let guardian = create_test_guardian("Solo Guardian", "peer1");
        manager.add_guardian(guardian).await.unwrap();

        let result = manager.select_guardians_for_distribution(3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_guardian_trust_upgrade() {
        let mut guardian = create_test_guardian("Test", "peer1");
        guardian.trust_level = TrustLevel::Basic;

        // Simulate successful recoveries
        for _ in 0..3 {
            guardian.record_successful_recovery();
        }
        assert_eq!(guardian.trust_level, TrustLevel::Trusted);

        for _ in 0..7 {
            guardian.record_successful_recovery();
        }
        assert_eq!(guardian.trust_level, TrustLevel::High);
    }

    #[tokio::test]
    async fn test_health_check_recording() {
        let manager = GuardianManager::new();
        let guardian = create_test_guardian("Test", "peer1");
        let id = guardian.id.clone();
        manager.add_guardian(guardian).await.unwrap();

        let result = HealthCheckResult {
            guardian_id: id.clone(),
            success: true,
            response_time_ms: Some(50),
            error: None,
            checked_at: Utc::now().timestamp(),
        };

        manager.record_health_check(&result).await.unwrap();

        let updated = manager.get_guardian(&id).await.unwrap();
        assert!(updated.last_health_check.is_some());
        assert_eq!(updated.status, GuardianStatus::Online);
    }

    #[test]
    fn test_guardian_availability() {
        let mut guardian = Guardian::new(
            "Test".to_string(),
            "peer1".to_string(),
            "pubkey".to_string(),
        );

        // Initially not available (pending status, untrusted)
        assert!(!guardian.is_available());

        // Online but still untrusted
        guardian.status = GuardianStatus::Online;
        assert!(!guardian.is_available());

        // Now should be available
        guardian.trust_level = TrustLevel::Basic;
        assert!(guardian.is_available());

        // Offline = not available
        guardian.status = GuardianStatus::Offline;
        assert!(!guardian.is_available());
    }
}
