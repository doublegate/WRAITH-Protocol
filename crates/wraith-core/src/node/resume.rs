//! Resume state persistence and recovery
//!
//! Enables transfer resumption after interruptions such as:
//! - Sender/receiver restart
//! - Network partition and reconnect
//! - Peer address change
//! - Corrupted chunk detection

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

use crate::node::error::{NodeError, Result};

/// Transfer resume state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeState {
    /// Transfer ID
    pub transfer_id: [u8; 32],

    /// Peer ID
    pub peer_id: [u8; 32],

    /// File hash (for integrity verification)
    pub file_hash: [u8; 32],

    /// File size in bytes
    pub file_size: u64,

    /// Chunk size used for this transfer
    pub chunk_size: usize,

    /// Total number of chunks
    pub total_chunks: usize,

    /// Completed chunks (chunk indices)
    pub completed_chunks: HashSet<usize>,

    /// File path (for sender) or destination path (for receiver)
    pub file_path: PathBuf,

    /// Is this a send or receive transfer
    pub is_sender: bool,

    /// Last active timestamp (seconds since epoch)
    pub last_active: u64,

    /// Transfer creation timestamp
    pub created_at: u64,
}

impl ResumeState {
    /// Create new resume state
    pub fn new(
        transfer_id: [u8; 32],
        peer_id: [u8; 32],
        file_hash: [u8; 32],
        file_size: u64,
        chunk_size: usize,
        file_path: PathBuf,
        is_sender: bool,
    ) -> Self {
        let total_chunks = (file_size as usize).div_ceil(chunk_size);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            transfer_id,
            peer_id,
            file_hash,
            file_size,
            chunk_size,
            total_chunks,
            completed_chunks: HashSet::new(),
            file_path,
            is_sender,
            last_active: now,
            created_at: now,
        }
    }

    /// Mark a chunk as completed
    pub fn mark_chunk_complete(&mut self, chunk_index: usize) {
        self.completed_chunks.insert(chunk_index);
        self.update_last_active();
    }

    /// Mark multiple chunks as completed
    pub fn mark_chunks_complete(&mut self, chunk_indices: &[usize]) {
        for &index in chunk_indices {
            self.completed_chunks.insert(index);
        }
        self.update_last_active();
    }

    /// Check if a chunk is completed
    pub fn is_chunk_complete(&self, chunk_index: usize) -> bool {
        self.completed_chunks.contains(&chunk_index)
    }

    /// Get missing chunks
    pub fn missing_chunks(&self) -> Vec<usize> {
        (0..self.total_chunks)
            .filter(|i| !self.completed_chunks.contains(i))
            .collect()
    }

    /// Calculate progress percentage
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            return 100.0;
        }
        (self.completed_chunks.len() as f64 / self.total_chunks as f64) * 100.0
    }

    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        self.completed_chunks.len() == self.total_chunks
    }

    /// Update last active timestamp
    pub fn update_last_active(&mut self) {
        self.last_active = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get chunk bitmap for efficient resumption
    pub fn chunk_bitmap(&self) -> Vec<u8> {
        let bitmap_size = self.total_chunks.div_ceil(8);
        let mut bitmap = vec![0u8; bitmap_size];

        for &chunk_index in &self.completed_chunks {
            let byte_index = chunk_index / 8;
            let bit_index = chunk_index % 8;
            bitmap[byte_index] |= 1 << bit_index;
        }

        bitmap
    }

    /// Restore from chunk bitmap
    pub fn from_bitmap(&mut self, bitmap: &[u8]) {
        self.completed_chunks.clear();

        for (byte_index, &byte) in bitmap.iter().enumerate() {
            for bit_index in 0..8 {
                if byte & (1 << bit_index) != 0 {
                    let chunk_index = byte_index * 8 + bit_index;
                    if chunk_index < self.total_chunks {
                        self.completed_chunks.insert(chunk_index);
                    }
                }
            }
        }
    }
}

/// Resume state manager
pub struct ResumeManager {
    /// State directory
    state_dir: PathBuf,

    /// In-memory state cache
    states: Arc<RwLock<std::collections::HashMap<[u8; 32], ResumeState>>>,

    /// Maximum age of state files (in seconds)
    max_age: u64,
}

impl ResumeManager {
    /// Create a new resume manager
    pub fn new(state_dir: PathBuf, max_age_days: u64) -> Self {
        Self {
            state_dir,
            states: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_age: max_age_days * 24 * 60 * 60,
        }
    }

    /// Initialize the manager (create state directory)
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.state_dir).await?;
        Ok(())
    }

    /// Save resume state to disk
    pub async fn save_state(&self, state: &ResumeState) -> Result<()> {
        // Update in-memory cache
        {
            let mut states = self.states.write().await;
            states.insert(state.transfer_id, state.clone());
        }

        // Save to disk
        let path = self.state_file_path(&state.transfer_id);
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| NodeError::Other(format!("Failed to serialize state: {}", e)))?;

        fs::write(&path, json).await?;

        Ok(())
    }

    /// Load resume state from disk
    pub async fn load_state(&self, transfer_id: &[u8; 32]) -> Result<Option<ResumeState>> {
        // Check in-memory cache first
        {
            let states = self.states.read().await;
            if let Some(state) = states.get(transfer_id) {
                return Ok(Some(state.clone()));
            }
        }

        // Load from disk
        let path = self.state_file_path(transfer_id);
        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).await?;
        let state: ResumeState = serde_json::from_str(&json)
            .map_err(|e| NodeError::Other(format!("Failed to deserialize state: {}", e)))?;

        // Update cache
        {
            let mut states = self.states.write().await;
            states.insert(*transfer_id, state.clone());
        }

        Ok(Some(state))
    }

    /// Update resume state
    pub async fn update_state(&self, transfer_id: &[u8; 32], chunk_index: usize) -> Result<()> {
        let mut state = self
            .load_state(transfer_id)
            .await?
            .ok_or(NodeError::TransferNotFound(*transfer_id))?;

        state.mark_chunk_complete(chunk_index);
        self.save_state(&state).await?;

        Ok(())
    }

    /// Delete resume state
    pub async fn delete_state(&self, transfer_id: &[u8; 32]) -> Result<()> {
        // Remove from cache
        {
            let mut states = self.states.write().await;
            states.remove(transfer_id);
        }

        // Delete from disk
        let path = self.state_file_path(transfer_id);
        if path.exists() {
            fs::remove_file(&path).await?;
        }

        Ok(())
    }

    /// Clean up old state files
    pub async fn cleanup_old_states(&self) -> Result<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut removed = 0;
        let mut to_remove = Vec::new();
        let mut entries = fs::read_dir(&self.state_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Read state to check age
                if let Ok(json) = fs::read_to_string(&path).await {
                    if let Ok(state) = serde_json::from_str::<ResumeState>(&json) {
                        let age = now.saturating_sub(state.last_active);
                        if age > self.max_age {
                            fs::remove_file(&path).await?;
                            to_remove.push(state.transfer_id);
                            removed += 1;
                        }
                    }
                }
            }
        }

        // Remove from in-memory cache
        if !to_remove.is_empty() {
            let mut states = self.states.write().await;
            for transfer_id in to_remove {
                states.remove(&transfer_id);
            }
        }

        Ok(removed)
    }

    /// List all active resume states
    pub async fn list_states(&self) -> Result<Vec<ResumeState>> {
        let mut states = Vec::new();
        let mut entries = fs::read_dir(&self.state_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read_to_string(&path).await {
                    if let Ok(state) = serde_json::from_str::<ResumeState>(&json) {
                        states.push(state);
                    }
                }
            }
        }

        Ok(states)
    }

    /// Get state file path for a transfer
    fn state_file_path(&self, transfer_id: &[u8; 32]) -> PathBuf {
        let filename = format!("{}.json", hex::encode(transfer_id));
        self.state_dir.join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resume_state_creation() {
        let state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024 * 1024, // 1 MB
            256 * 1024,  // 256 KB chunks
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        assert_eq!(state.total_chunks, 4);
        assert_eq!(state.completed_chunks.len(), 0);
        assert_eq!(state.progress(), 0.0);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_resume_state_mark_complete() {
        let mut state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        state.mark_chunk_complete(0);
        assert!(state.is_chunk_complete(0));
        assert_eq!(state.progress(), 25.0); // 1 of 4 chunks

        state.mark_chunk_complete(1);
        assert_eq!(state.progress(), 50.0); // 2 of 4 chunks
    }

    #[test]
    fn test_resume_state_missing_chunks() {
        let mut state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        state.mark_chunks_complete(&[0, 2]);

        let missing = state.missing_chunks();
        assert_eq!(missing, vec![1, 3]);
    }

    #[test]
    fn test_resume_state_completion() {
        let mut state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        assert!(!state.is_complete());

        state.mark_chunks_complete(&[0, 1, 2, 3]);
        assert!(state.is_complete());
        assert_eq!(state.progress(), 100.0);
    }

    #[test]
    fn test_resume_state_bitmap() {
        let mut state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            2048,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        // Mark chunks 0, 2, 4, 6 complete
        state.mark_chunks_complete(&[0, 2, 4, 6]);

        let bitmap = state.chunk_bitmap();
        assert_eq!(bitmap.len(), 1); // 8 chunks fit in 1 byte

        // Verify bitmap: 01010101 = 0x55
        assert_eq!(bitmap[0], 0x55);

        // Restore from bitmap
        let mut new_state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            2048,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );
        new_state.from_bitmap(&bitmap);

        assert_eq!(new_state.completed_chunks, state.completed_chunks);
    }

    #[tokio::test]
    async fn test_resume_manager_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
        manager.initialize().await.unwrap();

        let state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        manager.save_state(&state).await.unwrap();

        let loaded = manager
            .load_state(&state.transfer_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.transfer_id, state.transfer_id);
        assert_eq!(loaded.total_chunks, state.total_chunks);
    }

    #[tokio::test]
    async fn test_resume_manager_update() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
        manager.initialize().await.unwrap();

        let state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        manager.save_state(&state).await.unwrap();
        manager.update_state(&state.transfer_id, 0).await.unwrap();

        let loaded = manager
            .load_state(&state.transfer_id)
            .await
            .unwrap()
            .unwrap();
        assert!(loaded.is_chunk_complete(0));
    }

    #[tokio::test]
    async fn test_resume_manager_delete() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
        manager.initialize().await.unwrap();

        let state = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test.bin"),
            true,
        );

        manager.save_state(&state).await.unwrap();
        manager.delete_state(&state.transfer_id).await.unwrap();

        let loaded = manager.load_state(&state.transfer_id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_resume_manager_list() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ResumeManager::new(temp_dir.path().to_path_buf(), 7);
        manager.initialize().await.unwrap();

        let state1 = ResumeState::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1024,
            256,
            PathBuf::from("/tmp/test1.bin"),
            true,
        );

        let state2 = ResumeState::new(
            [4u8; 32],
            [5u8; 32],
            [6u8; 32],
            2048,
            512,
            PathBuf::from("/tmp/test2.bin"),
            false,
        );

        manager.save_state(&state1).await.unwrap();
        manager.save_state(&state2).await.unwrap();

        let states = manager.list_states().await.unwrap();
        assert_eq!(states.len(), 2);
    }
}
