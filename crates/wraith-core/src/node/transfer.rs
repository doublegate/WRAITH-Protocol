//! Multi-peer file transfer coordination
//!
//! Coordinates file downloads from multiple peers in parallel with chunk assignment.

use crate::node::node::TransferId;
use crate::node::session::PeerId;
use crate::node::{Node, NodeError};
use crate::transfer::TransferSession;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use wraith_files::chunker::FileReassembler;
use wraith_files::tree_hash::compute_tree_hash;

/// File metadata for transfers
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,

    /// Total number of chunks
    pub total_chunks: usize,

    /// Chunk size
    pub chunk_size: usize,

    /// Root hash for verification
    pub root_hash: [u8; 32],

    /// File name
    pub name: String,
}

impl Node {
    /// Download file from multiple peers in parallel
    ///
    /// Coordinates chunk assignment and parallel downloads from multiple sources.
    ///
    /// # Arguments
    ///
    /// * `file_hash` - Root hash of the file to download
    /// * `peers` - List of peer IDs to download from
    /// * `output_path` - Where to save the downloaded file
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No peers provided
    /// - File metadata fetch fails
    /// - Download fails
    /// - Hash verification fails
    pub async fn download_from_peers(
        &self,
        file_hash: &[u8; 32],
        peers: Vec<PeerId>,
        output_path: &Path,
    ) -> Result<TransferId, NodeError> {
        if peers.is_empty() {
            return Err(NodeError::Transfer("No peers provided".to_string()));
        }

        tracing::info!("Starting multi-peer download from {} peers", peers.len());

        // 1. Get file metadata from first available peer
        let metadata = self.fetch_file_metadata(file_hash, &peers).await?;

        tracing::debug!(
            "File metadata: {} bytes, {} chunks",
            metadata.size,
            metadata.total_chunks
        );

        // 2. Create reassembler
        let reassembler = Arc::new(Mutex::new(
            FileReassembler::new(output_path, metadata.size, metadata.chunk_size)
                .map_err(NodeError::Io)?,
        ));

        // 3. Create multi-peer transfer session
        let transfer_id = Node::generate_transfer_id();

        let mut transfer_session = TransferSession::new_receive(
            transfer_id,
            output_path.to_path_buf(),
            metadata.size,
            metadata.chunk_size,
        );

        // Add peer tracking for multi-peer download
        for peer_id in &peers {
            transfer_session.add_peer(*peer_id);
        }

        // Create tree hash for verification
        let tree_hash = wraith_files::tree_hash::FileTreeHash {
            root: *file_hash,
            chunks: Vec::new(), // Will be populated as chunks are verified
        };

        // Store transfer context
        let context = Arc::new(
            crate::node::file_transfer::FileTransferContext::new_receive(
                transfer_id,
                Arc::new(tokio::sync::RwLock::new(transfer_session)),
                reassembler.clone(),
                tree_hash,
            ),
        );
        self.inner.transfers.insert(transfer_id, context.clone());

        // 4. Assign chunks to peers
        let chunk_assignments = self.assign_chunks(&metadata, &peers);

        tracing::debug!(
            "Chunk assignments: {:?}",
            chunk_assignments
                .iter()
                .map(|(p, c)| (p, c.len()))
                .collect::<Vec<_>>()
        );

        // 5. Spawn download tasks for each peer
        let handles: Vec<_> = chunk_assignments
            .into_iter()
            .map(|(peer_id, chunks)| {
                let node = self.clone();
                let context_clone = context.clone();

                tokio::spawn(async move {
                    node.download_chunks_from_peer(peer_id, chunks, context_clone)
                        .await
                })
            })
            .collect();

        // 6. Wait for all downloads to complete
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(())) => {
                    tracing::debug!("Download task {} completed successfully", i);
                }
                Ok(Err(e)) => {
                    tracing::error!("Download task {} failed: {}", i, e);
                    return Err(e);
                }
                Err(e) => {
                    tracing::error!("Download task {} panicked: {}", i, e);
                    return Err(NodeError::Other(format!("Task join error: {}", e)));
                }
            }
        }

        // 7. Verify complete file
        tracing::info!("All chunks downloaded, verifying file integrity");

        let computed_hash =
            compute_tree_hash(output_path, metadata.chunk_size).map_err(NodeError::Io)?;

        if computed_hash.root != *file_hash {
            tracing::error!(
                "Hash mismatch: expected {:?}, got {:?}",
                file_hash,
                computed_hash.root
            );
            return Err(NodeError::Other("Hash verification failed".to_string()));
        }

        // 8. Transfer should be automatically marked complete when all chunks are transferred

        tracing::info!(
            "Multi-peer download complete: {:?} ({} bytes)",
            transfer_id,
            metadata.size
        );

        Ok(transfer_id)
    }

    /// Fetch file metadata from any available peer
    #[allow(clippy::never_loop)] // Temporary: placeholder always returns on first iteration
    async fn fetch_file_metadata(
        &self,
        file_hash: &[u8; 32],
        peers: &[PeerId],
    ) -> Result<FileMetadata, NodeError> {
        for peer_id in peers {
            tracing::debug!("Requesting metadata from peer {:?}", peer_id);

            // TODO: Integrate with actual protocol
            // For now, return mock metadata:
            //
            // match self.request_metadata(peer_id, file_hash).await {
            //     Ok(metadata) => return Ok(metadata),
            //     Err(e) => {
            //         tracing::warn!("Failed to get metadata from {:?}: {}", peer_id, e);
            //         continue;
            //     }
            // }

            // Placeholder: 1 MB file with 256 KB chunks
            return Ok(FileMetadata {
                size: 1024 * 1024, // 1 MB
                total_chunks: 4,   // 256 KB * 4
                chunk_size: 256 * 1024,
                root_hash: *file_hash,
                name: "test.dat".to_string(),
            });
        }

        Err(NodeError::Transfer(
            "Failed to fetch metadata from any peer".to_string(),
        ))
    }

    /// Assign chunks to peers using round-robin
    fn assign_chunks(
        &self,
        metadata: &FileMetadata,
        peers: &[PeerId],
    ) -> HashMap<PeerId, Vec<usize>> {
        let mut assignments: HashMap<PeerId, Vec<usize>> = HashMap::new();

        // Round-robin assignment for load balancing
        for (chunk_idx, peer_id) in (0..metadata.total_chunks).zip(peers.iter().cycle()) {
            assignments.entry(*peer_id).or_default().push(chunk_idx);
        }

        assignments
    }

    /// Download chunks from a specific peer
    async fn download_chunks_from_peer(
        &self,
        peer_id: PeerId,
        chunks: Vec<usize>,
        context: Arc<crate::node::file_transfer::FileTransferContext>,
    ) -> Result<(), NodeError> {
        tracing::debug!(
            "Downloading {} chunks from peer {:?}",
            chunks.len(),
            peer_id
        );

        // Get or establish session
        let _session = self.get_or_establish_session(&peer_id).await?;

        for chunk_idx in chunks {
            // TODO: Request chunk via protocol
            // For now, simulate chunk download:
            //
            // let chunk_data = session
            //     .request_chunk(chunk_idx)
            //     .await
            //     .map_err(|e| NodeError::Transport(e.to_string()))?;

            // Placeholder: generate fake chunk data
            let chunk_data = vec![0u8; 256 * 1024];

            // Write to reassembler
            if let Some(reassembler) = &context.reassembler {
                reassembler
                    .lock()
                    .await
                    .write_chunk(chunk_idx as u64, &chunk_data)
                    .map_err(NodeError::Io)?;
            }

            // Update progress
            context
                .transfer_session
                .write()
                .await
                .mark_chunk_transferred(chunk_idx as u64, chunk_data.len());

            tracing::trace!("Chunk {} downloaded from peer {:?}", chunk_idx, peer_id);
        }

        tracing::debug!("All chunks downloaded from peer {:?}", peer_id);

        Ok(())
    }

    /// Upload chunks to a requesting peer
    ///
    /// Serves chunks from a file being seeded.
    pub async fn upload_chunks_to_peer(
        &self,
        _peer_id: &PeerId,
        _file_path: &Path,
        _chunks: Vec<usize>,
    ) -> Result<(), NodeError> {
        // TODO: Implement upload logic
        // For now, this is a placeholder
        Ok(())
    }

    /// Get list of files available for download
    ///
    /// Returns list of files this node can serve.
    pub async fn list_available_files(&self) -> Vec<FileMetadata> {
        // TODO: Implement file listing
        // For now, return empty list
        Vec::new()
    }

    /// Announce availability of a file for seeding
    ///
    /// Advertises that this node has a complete file available for download.
    pub async fn announce_file(&self, _file_path: &Path) -> Result<[u8; 32], NodeError> {
        // TODO: Implement file announcement
        // For now, return placeholder hash
        Ok([0u8; 32])
    }

    /// Remove file from available files
    ///
    /// Stops seeding a file.
    pub async fn unannounce_file(&self, _file_hash: &[u8; 32]) -> Result<(), NodeError> {
        // TODO: Implement file removal
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata {
            size: 1024 * 1024,
            total_chunks: 4,
            chunk_size: 256 * 1024,
            root_hash: [42u8; 32],
            name: "test.dat".to_string(),
        };

        assert_eq!(metadata.size, 1024 * 1024);
        assert_eq!(metadata.total_chunks, 4);
        assert_eq!(metadata.chunk_size, 256 * 1024);
        assert_eq!(metadata.name, "test.dat");
    }

    #[tokio::test]
    async fn test_assign_chunks_single_peer() {
        let node = Node::new_random().await.unwrap();

        let metadata = FileMetadata {
            size: 1024 * 1024,
            total_chunks: 4,
            chunk_size: 256 * 1024,
            root_hash: [0u8; 32],
            name: "test.dat".to_string(),
        };

        let peers = vec![[1u8; 32]];

        let assignments = node.assign_chunks(&metadata, &peers);

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments.get(&[1u8; 32]).unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_assign_chunks_multiple_peers() {
        let node = Node::new_random().await.unwrap();

        let metadata = FileMetadata {
            size: 1024 * 1024,
            total_chunks: 8,
            chunk_size: 128 * 1024,
            root_hash: [0u8; 32],
            name: "test.dat".to_string(),
        };

        let peers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        let assignments = node.assign_chunks(&metadata, &peers);

        // Should distribute chunks evenly
        assert_eq!(assignments.len(), 3);

        let total_chunks: usize = assignments.values().map(|v| v.len()).sum();
        assert_eq!(total_chunks, 8);

        // Each peer should get 2-3 chunks (8 chunks / 3 peers)
        for chunks in assignments.values() {
            assert!(chunks.len() >= 2 && chunks.len() <= 3);
        }
    }

    #[tokio::test]
    async fn test_fetch_file_metadata() {
        let node = Node::new_random().await.unwrap();

        let file_hash = [42u8; 32];
        let peers = vec![[1u8; 32], [2u8; 32]];

        let result = node.fetch_file_metadata(&file_hash, &peers).await;

        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.root_hash, file_hash);
    }

    #[tokio::test]
    async fn test_fetch_file_metadata_no_peers() {
        let node = Node::new_random().await.unwrap();

        let file_hash = [42u8; 32];
        let peers = vec![];

        let result = node.fetch_file_metadata(&file_hash, &peers).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_available_files() {
        let node = Node::new_random().await.unwrap();

        let files = node.list_available_files().await;

        // Placeholder returns empty list
        assert_eq!(files.len(), 0);
    }

    #[tokio::test]
    async fn test_announce_file() {
        let node = Node::new_random().await.unwrap();
        let file_path = Path::new("/tmp/test.dat");

        let result = node.announce_file(file_path).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unannounce_file() {
        let node = Node::new_random().await.unwrap();
        let file_hash = [42u8; 32];

        let result = node.unannounce_file(&file_hash).await;

        assert!(result.is_ok());
    }
}
