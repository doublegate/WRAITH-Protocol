//! DHT-Based Distributed Storage
//!
//! Provides content storage and retrieval through the WRAITH DHT network.
//! Implements content addressing with BLAKE3 hashing (IPFS-like CIDs).

use crate::error::{PublishError, PublishResult};
use crate::signatures::SignedContent;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Number of DHT replicas for content
    pub replication_factor: usize,
    /// Maximum content size in bytes
    pub max_content_size: usize,
    /// Content TTL in seconds (0 = permanent)
    pub content_ttl: u64,
    /// Enable local pinning
    pub enable_pinning: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            max_content_size: 10 * 1024 * 1024, // 10 MB
            content_ttl: 0,                     // Permanent
            enable_pinning: true,
        }
    }
}

/// Content stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredContent {
    /// Content identifier (BLAKE3 hash)
    pub cid: String,
    /// Signed content with signature
    pub signed_content: SignedContent,
    /// Storage timestamp
    pub stored_at: i64,
    /// Number of confirmed replicas
    pub replica_count: usize,
    /// Whether content is pinned locally
    pub pinned: bool,
}

/// Local content store (simulates DHT in development)
#[derive(Debug, Default)]
struct LocalStore {
    content: HashMap<String, StoredContent>,
    pinned: HashMap<String, bool>,
}

/// DHT storage manager
pub struct DhtStorage {
    config: StorageConfig,
    local_store: Arc<RwLock<LocalStore>>,
    // In production, this would be: dht: Arc<wraith_discovery::dht::Dht>,
}

impl DhtStorage {
    /// Create a new DHT storage manager
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            local_store: Arc::new(RwLock::new(LocalStore::default())),
        }
    }

    /// Store signed content in DHT
    pub async fn store(&self, signed_content: SignedContent) -> PublishResult<StoredContent> {
        // Validate content size
        if signed_content.content.len() > self.config.max_content_size {
            return Err(PublishError::InvalidContent(format!(
                "Content too large: {} bytes (max: {} bytes)",
                signed_content.content.len(),
                self.config.max_content_size
            )));
        }

        let cid = signed_content.cid.clone();
        let now = chrono::Utc::now().timestamp();

        let stored = StoredContent {
            cid: cid.clone(),
            signed_content,
            stored_at: now,
            replica_count: 1, // Local replica
            pinned: self.config.enable_pinning,
        };

        // Store locally
        {
            let mut store = self.local_store.write();
            store.content.insert(cid.clone(), stored.clone());
            if self.config.enable_pinning {
                store.pinned.insert(cid.clone(), true);
            }
        }

        info!("Stored content: {} (local)", cid);

        // In production, would also store in DHT:
        // self.dht.put(&format!("content:{}", cid), payload).await?;

        // Simulate replication (in production, DHT handles this)
        let replicated = self.simulate_replication(&cid).await;

        let mut result = stored;
        result.replica_count = replicated;

        Ok(result)
    }

    /// Retrieve content by CID
    pub async fn retrieve(&self, cid: &str) -> PublishResult<Option<StoredContent>> {
        // Check local store first
        {
            let store = self.local_store.read();
            if let Some(content) = store.content.get(cid) {
                debug!("Retrieved content from local store: {}", cid);
                return Ok(Some(content.clone()));
            }
        }

        // In production, would fetch from DHT:
        // let payload = self.dht.get(&format!("content:{}", cid)).await?;

        warn!("Content not found in local store: {}", cid);
        Ok(None)
    }

    /// Pin content locally (prevents garbage collection)
    pub fn pin(&self, cid: &str) -> PublishResult<bool> {
        let mut store = self.local_store.write();

        if let Some(content) = store.content.get_mut(cid) {
            content.pinned = true;
            store.pinned.insert(cid.to_string(), true);
            info!("Pinned content: {}", cid);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Unpin content (allows garbage collection)
    pub fn unpin(&self, cid: &str) -> PublishResult<bool> {
        let mut store = self.local_store.write();

        if let Some(content) = store.content.get_mut(cid) {
            content.pinned = false;
            store.pinned.remove(cid);
            info!("Unpinned content: {}", cid);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if content is pinned
    pub fn is_pinned(&self, cid: &str) -> bool {
        let store = self.local_store.read();
        store.pinned.get(cid).copied().unwrap_or(false)
    }

    /// List all pinned content CIDs
    pub fn list_pinned(&self) -> Vec<String> {
        let store = self.local_store.read();
        store.pinned.keys().cloned().collect()
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        let store = self.local_store.read();

        let total_size: usize = store
            .content
            .values()
            .map(|c| c.signed_content.content.len())
            .sum();

        StorageStats {
            total_items: store.content.len(),
            pinned_items: store.pinned.len(),
            total_bytes: total_size,
            replication_factor: self.config.replication_factor,
        }
    }

    /// Delete content from local store
    pub fn delete(&self, cid: &str) -> PublishResult<bool> {
        let mut store = self.local_store.write();

        if store.content.remove(cid).is_some() {
            store.pinned.remove(cid);
            info!("Deleted content: {}", cid);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Simulate DHT replication (development only)
    async fn simulate_replication(&self, _cid: &str) -> usize {
        // In production, the DHT handles replication automatically
        // This simulates successful replication for development
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.config.replication_factor
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of stored items
    pub total_items: usize,
    /// Number of pinned items
    pub pinned_items: usize,
    /// Total storage size in bytes
    pub total_bytes: usize,
    /// Configured replication factor
    pub replication_factor: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::ContentSigner;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_storage() -> DhtStorage {
        DhtStorage::new(StorageConfig::default())
    }

    fn create_signed_content(content: &[u8]) -> SignedContent {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));
        signer.sign(content).unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let storage = create_test_storage();
        let signed = create_signed_content(b"Test content");
        let cid = signed.cid.clone();

        // Store
        let stored = storage.store(signed).await.unwrap();
        assert_eq!(stored.cid, cid);
        assert!(stored.replica_count > 0);

        // Retrieve
        let retrieved = storage.retrieve(&cid).await.unwrap().unwrap();
        assert_eq!(retrieved.cid, cid);
    }

    #[tokio::test]
    async fn test_pinning() {
        let storage = create_test_storage();
        let signed = create_signed_content(b"Pinned content");
        let cid = signed.cid.clone();

        storage.store(signed).await.unwrap();

        // Content should be pinned by default
        assert!(storage.is_pinned(&cid));

        // Unpin
        storage.unpin(&cid).unwrap();
        assert!(!storage.is_pinned(&cid));

        // Re-pin
        storage.pin(&cid).unwrap();
        assert!(storage.is_pinned(&cid));
    }

    #[tokio::test]
    async fn test_content_too_large() {
        let config = StorageConfig {
            max_content_size: 100,
            ..Default::default()
        };
        let storage = DhtStorage::new(config);

        let large_content = vec![0u8; 200];
        let signed = create_signed_content(&large_content);

        let result = storage.store(signed).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stats() {
        let storage = create_test_storage();

        // Store some content
        let signed1 = create_signed_content(b"Content 1");
        let signed2 = create_signed_content(b"Content 2");

        storage.store(signed1).await.unwrap();
        storage.store(signed2).await.unwrap();

        let stats = storage.stats();
        assert_eq!(stats.total_items, 2);
        assert_eq!(stats.pinned_items, 2);
        assert!(stats.total_bytes > 0);
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = create_test_storage();
        let signed = create_signed_content(b"To be deleted");
        let cid = signed.cid.clone();

        storage.store(signed).await.unwrap();
        assert!(storage.retrieve(&cid).await.unwrap().is_some());

        // Delete
        assert!(storage.delete(&cid).unwrap());
        assert!(storage.retrieve(&cid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_pinned() {
        let storage = create_test_storage();

        let signed1 = create_signed_content(b"Pinned 1");
        let signed2 = create_signed_content(b"Pinned 2");
        let cid1 = signed1.cid.clone();
        let cid2 = signed2.cid.clone();

        storage.store(signed1).await.unwrap();
        storage.store(signed2).await.unwrap();

        let pinned = storage.list_pinned();
        assert_eq!(pinned.len(), 2);
        assert!(pinned.contains(&cid1));
        assert!(pinned.contains(&cid2));
    }
}
