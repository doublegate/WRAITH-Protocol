//! Transfer management for WRAITH nodes
//!
//! This module provides file transfer lifecycle management including:
//! - Transfer initiation and coordination
//! - Progress tracking
//! - Chunk management
//!
//! # Transfer Flow
//!
//! ```text
//! Sender                          Receiver
//!     |                               |
//!     |-- StreamOpen (metadata) ----->|
//!     |                               |
//!     |-- Data (chunk 0) ------------>|
//!     |-- Data (chunk 1) ------------>|
//!     |-- ...                         |
//!     |-- Data (chunk N) ------------>|
//!     |                               |
//!     |    [Transfer Complete]        |
//! ```

use crate::node::error::{NodeError, Result};
use crate::node::file_transfer::FileTransferContext;
use crate::node::identity::TransferId;
use crate::node::session::PeerConnection;
use crate::transfer::TransferSession;
use dashmap::DashMap;
use getrandom::getrandom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::tree_hash::{FileTreeHash, compute_tree_hash};

/// Transfer manager for WRAITH nodes
///
/// Coordinates file transfer operations including chunking, hashing,
/// and progress tracking. Thread-safe and designed for concurrent access.
pub struct TransferManager {
    /// Active file transfers (transfer_id -> transfer context)
    transfers: Arc<DashMap<TransferId, Arc<FileTransferContext>>>,

    /// Default chunk size for transfers
    chunk_size: usize,
}

impl TransferManager {
    /// Create a new transfer manager
    ///
    /// # Arguments
    ///
    /// * `transfers` - Shared transfer map
    /// * `chunk_size` - Default chunk size for file transfers
    pub fn new(
        transfers: Arc<DashMap<TransferId, Arc<FileTransferContext>>>,
        chunk_size: usize,
    ) -> Self {
        Self {
            transfers,
            chunk_size,
        }
    }

    /// Generate a random transfer ID
    ///
    /// # Panics
    ///
    /// Panics if the CSPRNG fails to generate random bytes (extremely unlikely).
    pub fn generate_transfer_id() -> TransferId {
        let mut id = [0u8; 32];
        getrandom(&mut id).expect("Failed to generate transfer ID");
        id
    }

    /// Start a new send transfer
    ///
    /// Initializes a file transfer by:
    /// 1. Reading file metadata
    /// 2. Computing BLAKE3 tree hash for integrity
    /// 3. Creating transfer session
    /// 4. Storing transfer context
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to send
    ///
    /// # Returns
    ///
    /// Returns (transfer_id, tree_hash, file_size, total_chunks) on success.
    pub fn init_send_transfer(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<(TransferId, FileTreeHash, u64, u64)> {
        let file_path = file_path.as_ref();

        // Get file metadata
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| NodeError::Io(e.to_string()))?
            .len();

        if file_size == 0 {
            return Err(NodeError::InvalidState("Cannot send empty file".into()));
        }

        // Compute tree hash for integrity verification
        tracing::debug!(
            "Computing BLAKE3 tree hash for {} ({} bytes, chunk_size={})",
            file_path.display(),
            file_size,
            self.chunk_size
        );

        let tree_hash = compute_tree_hash(file_path, self.chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        // Generate transfer ID
        let transfer_id = Self::generate_transfer_id();

        // Calculate total chunks
        let total_chunks = file_size.div_ceil(self.chunk_size as u64);

        // Create transfer session
        let mut transfer = TransferSession::new_send(
            transfer_id,
            file_path.to_path_buf(),
            file_size,
            self.chunk_size,
        );
        transfer.start();

        // Store transfer context
        let transfer_arc = Arc::new(RwLock::new(transfer));
        let context = Arc::new(FileTransferContext::new_send(
            transfer_id,
            Arc::clone(&transfer_arc),
            tree_hash.clone(),
        ));
        self.transfers.insert(transfer_id, Arc::clone(&context));

        tracing::info!(
            "Initialized send transfer {:?} for {} ({} bytes, {} chunks)",
            hex::encode(&transfer_id[..8]),
            file_path.display(),
            file_size,
            total_chunks
        );

        Ok((transfer_id, tree_hash, file_size, total_chunks))
    }

    /// Initialize a receive transfer from metadata
    ///
    /// Called when a StreamOpen frame is received with file metadata.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Transfer ID from metadata
    /// * `file_name` - Target file name
    /// * `file_size` - Expected file size
    /// * `chunk_size` - Chunk size for transfer
    /// * `root_hash` - Expected root hash for verification
    pub fn init_receive_transfer(
        &self,
        transfer_id: TransferId,
        file_name: &str,
        file_size: u64,
        chunk_size: usize,
        root_hash: [u8; 32],
    ) -> Result<()> {
        // Create receive transfer session
        let mut transfer = TransferSession::new_receive(
            transfer_id,
            PathBuf::from(file_name),
            file_size,
            chunk_size,
        );
        transfer.start();

        // Create file reassembler
        let reassembler = FileReassembler::new(file_name, file_size, chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        // Create tree hash (just root for now - we'll build full tree from chunks)
        let tree_hash = FileTreeHash {
            root: root_hash,
            chunks: Vec::new(),
        };

        // Store consolidated transfer context
        let context = Arc::new(FileTransferContext::new_receive(
            transfer_id,
            Arc::new(RwLock::new(transfer)),
            Arc::new(Mutex::new(reassembler)),
            tree_hash,
        ));
        self.transfers.insert(transfer_id, context);

        tracing::debug!(
            "Initialized receive transfer {:?} for {} ({} bytes)",
            hex::encode(&transfer_id[..8]),
            file_name,
            file_size
        );

        Ok(())
    }

    /// Send file chunks to a peer
    ///
    /// Called from a spawned task to send all chunks for a transfer.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Transfer ID
    /// * `file_path` - Path to source file
    /// * `stream_id` - Stream ID for the transfer
    /// * `connection` - Peer connection
    /// * `send_frame_fn` - Async function to send encrypted frames
    pub async fn send_file_chunks<F, Fut>(
        &self,
        transfer_id: TransferId,
        file_path: PathBuf,
        stream_id: u16,
        connection: Arc<PeerConnection>,
        send_frame_fn: F,
    ) -> Result<()>
    where
        F: Fn(Arc<PeerConnection>, Vec<u8>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        // Get transfer context
        let context = self
            .transfers
            .get(&transfer_id)
            .ok_or(NodeError::TransferNotFound(transfer_id))?
            .clone();

        // Create chunker
        let mut chunker = FileChunker::new(&file_path, self.chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        let total_chunks = chunker.num_chunks();

        tracing::debug!(
            "Sending {} chunks for transfer {:?}",
            total_chunks,
            hex::encode(&transfer_id[..8])
        );

        // Send each chunk
        for chunk_index in 0..total_chunks {
            // Read chunk
            let chunk_data = chunker
                .read_chunk_at(chunk_index)
                .map_err(|e| NodeError::Io(e.to_string()))?;
            let chunk_len = chunk_data.len();

            // Verify chunk hash against tree hash
            if chunk_index < context.tree_hash.chunks.len() as u64 {
                let computed_hash = blake3::hash(&chunk_data);
                if computed_hash.as_bytes() != &context.tree_hash.chunks[chunk_index as usize] {
                    tracing::error!("Chunk {} hash mismatch during send", chunk_index);
                    return Err(NodeError::InvalidState(
                        "Chunk hash verification failed".into(),
                    ));
                }
            }

            // Build chunk frame
            let chunk_frame =
                crate::node::file_transfer::build_chunk_frame(stream_id, chunk_index, &chunk_data)?;

            // Send encrypted frame
            send_frame_fn(Arc::clone(&connection), chunk_frame).await?;

            // Update transfer progress
            {
                let mut transfer = context.transfer_session.write().await;
                transfer.mark_chunk_transferred(chunk_index, chunk_len);
            }

            tracing::trace!(
                "Sent chunk {}/{} for transfer {:?} ({} bytes)",
                chunk_index + 1,
                total_chunks,
                hex::encode(&transfer_id[..8]),
                chunk_len
            );
        }

        tracing::info!(
            "File transfer {:?} completed ({} chunks sent)",
            hex::encode(&transfer_id[..8]),
            total_chunks
        );

        Ok(())
    }

    /// Process a received chunk
    ///
    /// Writes chunk to reassembler and updates progress.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Transfer ID
    /// * `chunk_index` - Index of the chunk
    /// * `chunk_data` - Chunk data
    pub async fn process_received_chunk(
        &self,
        transfer_id: TransferId,
        chunk_index: u64,
        chunk_data: &[u8],
    ) -> Result<bool> {
        let context = self
            .transfers
            .get(&transfer_id)
            .ok_or(NodeError::TransferNotFound(transfer_id))?
            .clone();

        // Write chunk to reassembler
        if let Some(reassembler_arc) = &context.reassembler {
            let mut reassembler = reassembler_arc.lock().await;
            reassembler
                .write_chunk(chunk_index, chunk_data)
                .map_err(|e| NodeError::Io(e.to_string()))?;

            tracing::trace!(
                "Wrote chunk {} to reassembler for transfer {:?}",
                chunk_index,
                hex::encode(&transfer_id[..8])
            );
        } else {
            return Err(NodeError::InvalidState(
                format!(
                    "No reassembler found for transfer {:?}",
                    hex::encode(&transfer_id[..8])
                )
                .into(),
            ));
        }

        // Verify chunk hash
        let tree_hash = &context.tree_hash;
        if chunk_index < tree_hash.chunks.len() as u64 {
            let computed_hash = blake3::hash(chunk_data);
            if computed_hash.as_bytes() != &tree_hash.chunks[chunk_index as usize] {
                tracing::error!(
                    "Chunk {} hash mismatch for transfer {:?}",
                    chunk_index,
                    hex::encode(&transfer_id[..8])
                );
                return Err(NodeError::InvalidState(
                    "Chunk hash verification failed".into(),
                ));
            }
        }

        // Update transfer progress
        let mut transfer = context.transfer_session.write().await;
        transfer.mark_chunk_transferred(chunk_index, chunk_data.len());

        // Check if transfer is complete
        let is_complete = transfer.is_complete();
        if is_complete {
            tracing::info!(
                "File transfer {:?} completed successfully ({} bytes received)",
                hex::encode(&transfer_id[..8]),
                transfer.file_size
            );
        }

        Ok(is_complete)
    }

    /// Wait for a transfer to complete
    ///
    /// Polls the transfer status until completion or error.
    pub async fn wait_for_transfer(&self, transfer_id: TransferId) -> Result<()> {
        loop {
            if let Some(context) = self.transfers.get(&transfer_id) {
                let transfer_guard = context.transfer_session.read().await;
                if transfer_guard.is_complete() {
                    return Ok(());
                }
            } else {
                return Err(NodeError::TransferNotFound(transfer_id));
            }

            // Wait before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get transfer progress (0.0 to 1.0)
    pub async fn get_transfer_progress(&self, transfer_id: &TransferId) -> Option<f64> {
        if let Some(context) = self.transfers.get(transfer_id) {
            let session = context.transfer_session.read().await;
            Some(session.progress())
        } else {
            None
        }
    }

    /// Get transfer context
    pub fn get_transfer(&self, transfer_id: &TransferId) -> Option<Arc<FileTransferContext>> {
        self.transfers.get(transfer_id).map(|e| Arc::clone(&e))
    }

    /// Find transfer by stream ID
    ///
    /// Stream ID is derived from transfer ID: `(transfer_id[0] << 8) | transfer_id[1]`
    pub fn find_transfer_by_stream_id(&self, stream_id: u16) -> Option<Arc<FileTransferContext>> {
        for entry in self.transfers.iter() {
            let tid = entry.key();
            let derived_stream_id = ((tid[0] as u16) << 8) | (tid[1] as u16);
            if derived_stream_id == stream_id {
                return Some(Arc::clone(entry.value()));
            }
        }
        None
    }

    /// List all active transfer IDs
    pub fn active_transfers(&self) -> Vec<TransferId> {
        self.transfers.iter().map(|entry| *entry.key()).collect()
    }

    /// Get number of active transfers
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }

    /// Remove a completed or failed transfer
    pub fn remove_transfer(&self, transfer_id: &TransferId) -> Option<Arc<FileTransferContext>> {
        self.transfers.remove(transfer_id).map(|(_, ctx)| ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> TransferManager {
        TransferManager::new(Arc::new(DashMap::new()), 256 * 1024)
    }

    #[test]
    fn test_transfer_manager_creation() {
        let manager = create_test_manager();
        assert_eq!(manager.transfer_count(), 0);
        assert!(manager.active_transfers().is_empty());
    }

    #[test]
    fn test_generate_transfer_id() {
        let id1 = TransferManager::generate_transfer_id();
        let id2 = TransferManager::generate_transfer_id();

        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_get_transfer_not_found() {
        let manager = create_test_manager();
        let transfer_id = [42u8; 32];
        assert!(manager.get_transfer(&transfer_id).is_none());
    }

    #[test]
    fn test_find_transfer_by_stream_id_not_found() {
        let manager = create_test_manager();
        assert!(manager.find_transfer_by_stream_id(1234).is_none());
    }

    #[test]
    fn test_remove_transfer_not_found() {
        let manager = create_test_manager();
        let transfer_id = [42u8; 32];
        assert!(manager.remove_transfer(&transfer_id).is_none());
    }

    #[tokio::test]
    async fn test_get_transfer_progress_not_found() {
        let manager = create_test_manager();
        let transfer_id = [42u8; 32];
        assert!(manager.get_transfer_progress(&transfer_id).await.is_none());
    }

    #[tokio::test]
    async fn test_wait_for_transfer_not_found() {
        let manager = create_test_manager();
        let transfer_id = [42u8; 32];

        // Should return error immediately
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            manager.wait_for_transfer(transfer_id),
        )
        .await;

        assert!(result.is_ok()); // Timeout didn't trigger
        assert!(matches!(
            result.unwrap(),
            Err(NodeError::TransferNotFound(_))
        ));
    }

    #[test]
    fn test_init_send_transfer_with_file() {
        use std::io::Write;

        let manager = create_test_manager();

        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_send_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0xABu8; 1024]).unwrap();
        drop(file);

        let result = manager.init_send_transfer(&file_path);
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_ok());
        let (transfer_id, tree_hash, file_size, total_chunks) = result.unwrap();
        assert_eq!(transfer_id.len(), 32);
        assert_eq!(file_size, 1024);
        assert_eq!(total_chunks, 1); // 1024 / (256*1024) rounds up to 1
        assert_ne!(tree_hash.root, [0u8; 32]);

        // Verify transfer was stored
        assert_eq!(manager.transfer_count(), 1);
        assert!(manager.get_transfer(&transfer_id).is_some());
    }

    #[test]
    fn test_init_send_transfer_empty_file() {
        let manager = create_test_manager();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_empty_test.dat");
        let file = std::fs::File::create(&file_path).unwrap();
        drop(file);

        let result = manager.init_send_transfer(&file_path);
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty file"));
    }

    #[test]
    fn test_init_send_transfer_nonexistent_file() {
        let manager = create_test_manager();
        let result = manager.init_send_transfer("/nonexistent/path/file.dat");
        assert!(result.is_err());
    }

    #[test]
    fn test_init_receive_transfer() {
        let manager = create_test_manager();
        let transfer_id = [99u8; 32];

        let temp_dir = std::env::temp_dir();
        let recv_path = temp_dir.join("wraith_tm_recv_test.dat");
        let recv_str = recv_path.to_str().unwrap();

        let result = manager.init_receive_transfer(transfer_id, recv_str, 4096, 1024, [0xAB; 32]);

        assert!(result.is_ok());
        assert_eq!(manager.transfer_count(), 1);
        assert!(manager.get_transfer(&transfer_id).is_some());

        // Cleanup
        let _ = std::fs::remove_file(&recv_path);
    }

    #[test]
    fn test_find_transfer_by_stream_id() {
        use std::io::Write;

        let manager = create_test_manager();

        // Create a file and init send transfer
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_stream_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0xABu8; 1024]).unwrap();
        drop(file);

        let (transfer_id, _, _, _) = manager.init_send_transfer(&file_path).unwrap();
        let _ = std::fs::remove_file(&file_path);

        // Derive stream_id the same way the code does
        let stream_id = ((transfer_id[0] as u16) << 8) | (transfer_id[1] as u16);

        let found = manager.find_transfer_by_stream_id(stream_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().transfer_id, transfer_id);
    }

    #[test]
    fn test_remove_transfer() {
        use std::io::Write;

        let manager = create_test_manager();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_remove_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0xABu8; 1024]).unwrap();
        drop(file);

        let (transfer_id, _, _, _) = manager.init_send_transfer(&file_path).unwrap();
        let _ = std::fs::remove_file(&file_path);

        assert_eq!(manager.transfer_count(), 1);
        let removed = manager.remove_transfer(&transfer_id);
        assert!(removed.is_some());
        assert_eq!(manager.transfer_count(), 0);
    }

    #[test]
    fn test_active_transfers() {
        use std::io::Write;

        let manager = create_test_manager();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_active_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0xABu8; 1024]).unwrap();
        drop(file);

        let (id1, _, _, _) = manager.init_send_transfer(&file_path).unwrap();
        // Create another file for second transfer
        let file_path2 = temp_dir.join("wraith_tm_active_test2.dat");
        let mut file2 = std::fs::File::create(&file_path2).unwrap();
        file2.write_all(&[0xCDu8; 2048]).unwrap();
        drop(file2);

        let (id2, _, _, _) = manager.init_send_transfer(&file_path2).unwrap();

        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_file(&file_path2);

        let active = manager.active_transfers();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&id1));
        assert!(active.contains(&id2));
    }

    #[tokio::test]
    async fn test_get_transfer_progress_exists() {
        use std::io::Write;

        let manager = create_test_manager();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_tm_progress_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0xABu8; 1024]).unwrap();
        drop(file);

        let (transfer_id, _, _, _) = manager.init_send_transfer(&file_path).unwrap();
        let _ = std::fs::remove_file(&file_path);

        let progress = manager.get_transfer_progress(&transfer_id).await;
        assert!(progress.is_some());
        // New transfer should be at 0% progress
        let p = progress.unwrap();
        assert!(p >= 0.0 && p <= 1.0);
    }

    #[tokio::test]
    async fn test_process_received_chunk() {
        let manager = create_test_manager();
        let transfer_id = [99u8; 32];

        let temp_dir = std::env::temp_dir();
        let chunk_path = temp_dir.join("wraith_tm_chunk_test.dat");
        let chunk_str = chunk_path.to_str().unwrap();

        // Init a receive transfer
        manager
            .init_receive_transfer(transfer_id, chunk_str, 2048, 1024, [0xAB; 32])
            .unwrap();

        // Process a chunk
        let chunk_data = vec![0xFFu8; 1024];
        let result = manager
            .process_received_chunk(transfer_id, 0, &chunk_data)
            .await;

        // Cleanup
        let _ = std::fs::remove_file(&chunk_path);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_received_chunk_not_found() {
        let manager = create_test_manager();
        let transfer_id = [99u8; 32];

        let result = manager
            .process_received_chunk(transfer_id, 0, &[0u8; 100])
            .await;

        assert!(matches!(result, Err(NodeError::TransferNotFound(_))));
    }
}
