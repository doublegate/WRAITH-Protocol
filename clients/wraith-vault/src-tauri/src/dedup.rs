//! BLAKE3-based Deduplication for WRAITH Vault
//!
//! Tracks chunk references for efficient storage and deduplication.

use crate::chunker::Chunk;
use crate::database::Database;
use crate::error::VaultResult;
use std::sync::Arc;
use tracing::debug;

/// Deduplication index for tracking unique chunks
pub struct DedupIndex {
    db: Arc<Database>,
}

impl DedupIndex {
    /// Create a new deduplication index
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Check if a chunk with the given hash exists
    pub fn has_chunk(&self, hash: &[u8; 32]) -> VaultResult<bool> {
        self.db.chunk_exists(hash)
    }

    /// Add a chunk to the index, returning true if it's a new chunk
    pub fn add_chunk(&self, chunk: &Chunk, compressed_size: i64) -> VaultResult<bool> {
        if self.has_chunk(&chunk.hash)? {
            // Chunk exists, increment reference count
            self.db.increment_chunk_ref(&chunk.hash)?;
            debug!("Chunk {} deduplicated (existing)", hex::encode(chunk.hash));
            Ok(false)
        } else {
            // New chunk, insert it
            self.db
                .insert_chunk(&chunk.hash, chunk.size as i64, compressed_size)?;
            debug!("Chunk {} added (new)", hex::encode(chunk.hash));
            Ok(true)
        }
    }

    /// Get a chunk by hash
    pub fn get_chunk(&self, hash: &[u8; 32]) -> VaultResult<Option<ChunkInfo>> {
        self.db.get_chunk(hash)
    }

    /// Remove a reference to a chunk, returning true if chunk should be deleted
    pub fn remove_chunk_ref(&self, hash: &[u8; 32]) -> VaultResult<bool> {
        let should_delete = self.db.decrement_chunk_ref(hash)?;
        if should_delete {
            debug!("Chunk {} marked for deletion", hex::encode(hash));
        }
        Ok(should_delete)
    }

    /// Calculate deduplication statistics
    pub fn dedup_stats(&self) -> VaultResult<DedupStats> {
        let stats = self.db.get_dedup_stats()?;
        Ok(stats)
    }

    /// Get the deduplication ratio (higher is better)
    pub fn dedup_ratio(&self) -> VaultResult<f64> {
        let stats = self.dedup_stats()?;
        if stats.unique_chunks == 0 {
            return Ok(1.0);
        }
        Ok(stats.total_references as f64 / stats.unique_chunks as f64)
    }

    /// Verify chunk integrity by comparing stored hash with computed hash
    pub fn verify_chunk(&self, hash: &[u8; 32], data: &[u8]) -> bool {
        let computed = blake3::hash(data);
        computed.as_bytes() == hash
    }

    /// Mark a chunk as verified
    pub fn mark_verified(&self, hash: &[u8; 32]) -> VaultResult<()> {
        self.db.mark_chunk_verified(hash)
    }

    /// Get chunks that haven't been verified recently
    pub fn get_unverified_chunks(&self, older_than_days: i64) -> VaultResult<Vec<[u8; 32]>> {
        self.db.get_unverified_chunks(older_than_days)
    }
}

/// Information about a stored chunk
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub hash: [u8; 32],
    pub size: i64,
    pub compressed_size: i64,
    pub ref_count: i64,
    pub created_at: i64,
    pub verified_at: Option<i64>,
}

/// Deduplication statistics
#[derive(Debug, Clone, Default)]
pub struct DedupStats {
    /// Total number of chunk references
    pub total_references: i64,
    /// Number of unique chunks
    pub unique_chunks: i64,
    /// Total bytes (before dedup)
    pub total_bytes: i64,
    /// Unique bytes (after dedup)
    pub unique_bytes: i64,
    /// Total compressed bytes
    pub compressed_bytes: i64,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Deduplication ratio
    pub dedup_ratio: f64,
    /// Space saved
    pub space_saved: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_db() -> Arc<Database> {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        Arc::new(Database::open(&db_path).unwrap())
    }

    #[test]
    fn test_add_new_chunk() {
        let db = setup_db();
        let dedup = DedupIndex::new(db);

        let chunk = Chunk::new(vec![1, 2, 3, 4, 5]);
        let is_new = dedup.add_chunk(&chunk, 5).unwrap();

        assert!(is_new);
        assert!(dedup.has_chunk(&chunk.hash).unwrap());
    }

    #[test]
    fn test_deduplicate_chunk() {
        let db = setup_db();
        let dedup = DedupIndex::new(db);

        let chunk = Chunk::new(vec![1, 2, 3, 4, 5]);

        // First add
        let is_new1 = dedup.add_chunk(&chunk, 5).unwrap();
        assert!(is_new1);

        // Second add (should deduplicate)
        let is_new2 = dedup.add_chunk(&chunk, 5).unwrap();
        assert!(!is_new2);

        // Check ref count
        let info = dedup.get_chunk(&chunk.hash).unwrap().unwrap();
        assert_eq!(info.ref_count, 2);
    }

    #[test]
    fn test_remove_chunk_ref() {
        let db = setup_db();
        let dedup = DedupIndex::new(db);

        let chunk = Chunk::new(vec![1, 2, 3, 4, 5]);

        // Add twice
        dedup.add_chunk(&chunk, 5).unwrap();
        dedup.add_chunk(&chunk, 5).unwrap();

        // Remove once
        let should_delete1 = dedup.remove_chunk_ref(&chunk.hash).unwrap();
        assert!(!should_delete1);

        // Remove again (should be deleted)
        let should_delete2 = dedup.remove_chunk_ref(&chunk.hash).unwrap();
        assert!(should_delete2);
    }

    #[test]
    fn test_verify_chunk() {
        let db = setup_db();
        let dedup = DedupIndex::new(db);

        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(data.clone());

        // Correct data
        assert!(dedup.verify_chunk(&chunk.hash, &data));

        // Incorrect data
        assert!(!dedup.verify_chunk(&chunk.hash, &[1, 2, 3, 4, 6]));
    }

    #[test]
    fn test_dedup_ratio() {
        let db = setup_db();
        let dedup = DedupIndex::new(db);

        // Add same chunk 5 times
        let chunk = Chunk::new(vec![1, 2, 3, 4, 5]);
        for _ in 0..5 {
            dedup.add_chunk(&chunk, 5).unwrap();
        }

        let ratio = dedup.dedup_ratio().unwrap();
        assert!((ratio - 5.0).abs() < 0.01);
    }
}
