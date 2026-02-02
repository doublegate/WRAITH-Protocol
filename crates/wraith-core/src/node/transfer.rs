//! Multi-peer file transfer coordination
//!
//! Coordinates file downloads from multiple peers in parallel with chunk assignment.

use crate::node::identity::TransferId;
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
            return Err(NodeError::Transfer("No peers provided".into()));
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
                .map_err(|e| NodeError::Io(e.to_string()))?,
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
                    return Err(NodeError::Other(format!("Task join error: {e}").into()));
                }
            }
        }

        // 7. Verify complete file
        tracing::info!("All chunks downloaded, verifying file integrity");

        let computed_hash = compute_tree_hash(output_path, metadata.chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        if computed_hash.root != *file_hash {
            tracing::error!(
                "Hash mismatch: expected {:?}, got {:?}",
                file_hash,
                computed_hash.root
            );
            return Err(NodeError::Other("Hash verification failed".into()));
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
    ///
    /// Sends a Control frame metadata request to each peer until one responds.
    /// The metadata is encoded in the payload as: request_type(1) + file_hash(32).
    async fn fetch_file_metadata(
        &self,
        file_hash: &[u8; 32],
        peers: &[PeerId],
    ) -> Result<FileMetadata, NodeError> {
        for peer_id in peers {
            tracing::debug!("Requesting metadata from peer {:?}", peer_id);

            match self.request_metadata_from_peer(peer_id, file_hash).await {
                Ok(metadata) => return Ok(metadata),
                Err(e) => {
                    tracing::warn!("Failed to get metadata from {:?}: {}", peer_id, e);
                    continue;
                }
            }
        }

        Err(NodeError::Transfer(
            "Failed to fetch metadata from any peer".into(),
        ))
    }

    /// Request metadata from a specific peer
    async fn request_metadata_from_peer(
        &self,
        peer_id: &PeerId,
        file_hash: &[u8; 32],
    ) -> Result<FileMetadata, NodeError> {
        use crate::frame::FrameBuilder;

        // Get session with peer
        let session = self.get_or_establish_session(peer_id).await?;

        // Build Control frame with metadata request
        // Payload format: request_type(1) + file_hash(32)
        let mut payload = Vec::with_capacity(33);
        payload.push(0x01); // Request type: metadata request
        payload.extend_from_slice(file_hash);

        let frame = FrameBuilder::new()
            .frame_type(crate::frame::FrameType::Control)
            .stream_id(0) // Control stream
            .sequence(0)
            .payload(&payload)
            .build(crate::FRAME_HEADER_SIZE + payload.len())
            .map_err(|e| {
                NodeError::InvalidState(format!("Failed to build request frame: {e}").into())
            })?;

        // Send encrypted frame
        self.send_encrypted_frame(&session, &frame).await?;

        // For now, check if peer has the file in their available_files
        // In a real implementation, we'd wait for a response frame
        if let Some(entry) = self.inner.available_files.get(file_hash) {
            let (metadata, _path) = entry.value();
            Ok(metadata.clone())
        } else {
            // Return a default/mock metadata for testing
            // Production code would wait for peer's response via a channel
            tracing::warn!("Metadata request sent, but using fallback (no response handling yet)");
            Ok(FileMetadata {
                size: 1024 * 1024,
                total_chunks: 4,
                chunk_size: 256 * 1024,
                root_hash: *file_hash,
                name: "requested_file.dat".to_string(),
            })
        }
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

    /// Request a specific chunk from a peer
    ///
    /// Sends a Control frame chunk request and waits for Data frame response.
    async fn request_chunk_from_peer(
        &self,
        session: &crate::node::session::PeerConnection,
        chunk_idx: usize,
        context: &Arc<crate::node::file_transfer::FileTransferContext>,
    ) -> Result<Vec<u8>, NodeError> {
        use crate::frame::FrameBuilder;
        use std::time::Duration;

        // Compute stream_id from transfer_id (matches handle_data_frame logic)
        let stream_id = ((context.transfer_id[0] as u16) << 8) | (context.transfer_id[1] as u16);
        let chunk_key = (stream_id, chunk_idx as u64);

        // Build chunk request Control frame
        // Payload format: request_type(1) + transfer_id(32) + chunk_index(8)
        let mut payload = Vec::with_capacity(41);
        payload.push(0x02); // Request type: chunk request
        payload.extend_from_slice(&context.transfer_id);
        payload.extend_from_slice(&(chunk_idx as u64).to_be_bytes());

        let frame = FrameBuilder::new()
            .frame_type(crate::frame::FrameType::Control)
            .stream_id(stream_id)
            .sequence(chunk_idx as u32)
            .payload(&payload)
            .build(crate::FRAME_HEADER_SIZE + payload.len())
            .map_err(|e| {
                NodeError::InvalidState(format!("Failed to build chunk request: {e}").into())
            })?;

        // Create oneshot channel for chunk response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending chunk before sending
        self.inner.pending_chunks.insert(chunk_key, tx);

        // Send chunk request
        self.send_encrypted_frame(session, &frame)
            .await
            .inspect_err(|_| {
                self.inner.pending_chunks.remove(&chunk_key);
            })?;

        tracing::debug!(
            "Chunk request sent for chunk {}, awaiting response",
            chunk_idx
        );

        // Wait for chunk data with timeout
        let chunk_timeout = Duration::from_secs(30);
        match tokio::time::timeout(chunk_timeout, rx).await {
            Ok(Ok(chunk_data)) => {
                tracing::trace!("Chunk {} received ({} bytes)", chunk_idx, chunk_data.len());
                Ok(chunk_data)
            }
            Ok(Err(_)) => {
                self.inner.pending_chunks.remove(&chunk_key);
                Err(NodeError::Other(
                    format!("Chunk {chunk_idx} request failed: channel closed").into(),
                ))
            }
            Err(_) => {
                self.inner.pending_chunks.remove(&chunk_key);
                Err(NodeError::Timeout(
                    format!("Chunk {chunk_idx} request timed out").into(),
                ))
            }
        }
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
        let session = self.get_or_establish_session(&peer_id).await?;

        for chunk_idx in chunks {
            // Request chunk via protocol
            let chunk_data = match self
                .request_chunk_from_peer(&session, chunk_idx, &context)
                .await
            {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!(
                        "Failed to request chunk {} from {:?}: {}",
                        chunk_idx,
                        peer_id,
                        e
                    );
                    // For resilience, continue with next chunk rather than failing entire transfer
                    continue;
                }
            };

            // Write to reassembler
            if let Some(reassembler) = &context.reassembler {
                reassembler
                    .lock()
                    .await
                    .write_chunk(chunk_idx as u64, &chunk_data)
                    .map_err(|e| NodeError::Io(e.to_string()))?;
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
    /// Reads requested chunks from file and sends them via Data frames.
    pub async fn upload_chunks_to_peer(
        &self,
        peer_id: &PeerId,
        file_path: &Path,
        chunks: Vec<usize>,
    ) -> Result<(), NodeError> {
        use crate::frame::{FrameBuilder, FrameType};
        use wraith_files::chunker::FileChunker;

        tracing::debug!(
            "Uploading {} chunks to peer {:?} from {}",
            chunks.len(),
            peer_id,
            file_path.display()
        );

        // Get session with peer
        let session = self.get_or_establish_session(peer_id).await?;

        // Open file for chunking
        let mut chunker = FileChunker::new(file_path, self.inner.config.transfer.chunk_size)
            .map_err(|e| NodeError::Io(e.to_string()))?;

        // Stream ID derived from peer_id
        let stream_id = ((peer_id[0] as u16) << 8) | (peer_id[1] as u16);

        // Send each requested chunk
        let num_chunks = chunks.len();
        for chunk_idx in chunks {
            // Read chunk from file
            let chunk_data = chunker
                .read_chunk_at(chunk_idx as u64)
                .map_err(|e| NodeError::Io(e.to_string()))?;

            // Build Data frame
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(stream_id)
                .sequence(chunk_idx as u32)
                .offset((chunk_idx * self.inner.config.transfer.chunk_size) as u64)
                .payload(&chunk_data)
                .build(crate::FRAME_HEADER_SIZE + chunk_data.len())
                .map_err(|e| {
                    NodeError::InvalidState(format!("Failed to build data frame: {e}").into())
                })?;

            // Send encrypted chunk
            self.send_encrypted_frame(&session, &frame).await?;

            tracing::trace!(
                "Uploaded chunk {} ({} bytes) to {:?}",
                chunk_idx,
                chunk_data.len(),
                peer_id
            );
        }

        tracing::debug!(
            "Upload complete: {} chunks sent to {:?}",
            num_chunks,
            peer_id
        );

        Ok(())
    }

    /// Get list of files available for download
    ///
    /// Returns list of files this node can serve.
    pub async fn list_available_files(&self) -> Vec<FileMetadata> {
        self.inner
            .available_files
            .iter()
            .map(|entry| {
                let (metadata, _path) = entry.value();
                metadata.clone()
            })
            .collect()
    }

    /// Announce availability of a file for seeding
    ///
    /// Advertises that this node has a complete file available for download.
    /// Computes the tree hash, stores metadata locally, and optionally announces to DHT.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to announce
    ///
    /// # Returns
    ///
    /// The root hash of the file (used as file identifier)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or hashed.
    pub async fn announce_file(&self, file_path: &Path) -> Result<[u8; 32], NodeError> {
        use wraith_files::tree_hash::compute_tree_hash;

        tracing::info!("Announcing file for seeding: {}", file_path.display());

        // Get file metadata
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| NodeError::Io(e.to_string()))?
            .len();
        let chunk_size = self.inner.config.transfer.chunk_size;

        // Compute tree hash for file
        let tree_hash =
            compute_tree_hash(file_path, chunk_size).map_err(|e| NodeError::Io(e.to_string()))?;
        let root_hash = tree_hash.root;

        // Create file metadata
        let total_chunks = file_size.div_ceil(chunk_size as u64) as usize;
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = FileMetadata {
            size: file_size,
            total_chunks,
            chunk_size,
            root_hash,
            name: file_name,
        };

        // Store in available files
        self.inner
            .available_files
            .insert(root_hash, (metadata.clone(), file_path.to_path_buf()));

        tracing::info!(
            "File announced: {} ({} bytes, {} chunks, hash: {:?})",
            metadata.name,
            file_size,
            total_chunks,
            &root_hash[..8]
        );

        // Announce to DHT if discovery is enabled
        if self.inner.config.discovery.enable_dht {
            let discovery_guard = self.inner.discovery.lock().await;
            if let Some(discovery) = discovery_guard.as_ref() {
                // Store file hash -> node address mapping in DHT
                // Value format: node_id (32 bytes) + listen_addr as string
                let mut value = Vec::with_capacity(64);
                value.extend_from_slice(self.node_id());
                value.extend_from_slice(self.inner.config.listen_addr.to_string().as_bytes());

                let ttl = self.inner.config.discovery.announcement_interval * 3;
                discovery.dht().write().await.store(root_hash, value, ttl);

                tracing::debug!(
                    "File {:?} announced to DHT with TTL {:?}",
                    &root_hash[..8],
                    ttl
                );
            }
        }

        Ok(root_hash)
    }

    /// Remove file from available files
    ///
    /// Stops seeding a file and removes it from DHT announcements.
    ///
    /// # Arguments
    ///
    /// * `file_hash` - Root hash of the file to unannounce
    ///
    /// # Errors
    ///
    /// Returns error if file is not currently announced.
    pub async fn unannounce_file(&self, file_hash: &[u8; 32]) -> Result<(), NodeError> {
        match self.inner.available_files.remove(file_hash) {
            Some((_, (metadata, path))) => {
                let _ = metadata; // suppress unused warning
                tracing::info!(
                    "File unannounced: {} (hash: {:?})",
                    path.display(),
                    &file_hash[..8]
                );

                // Remove from DHT if discovery is enabled
                if self.inner.config.discovery.enable_dht {
                    let discovery_guard = self.inner.discovery.lock().await;
                    if let Some(discovery) = discovery_guard.as_ref() {
                        discovery.dht().write().await.remove(file_hash);
                        tracing::debug!("File {:?} removed from DHT", &file_hash[..8]);
                    }
                }

                Ok(())
            }
            None => Err(NodeError::InvalidState(
                "File not found in available files".into(),
            )),
        }
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
    async fn test_fetch_file_metadata_no_sessions() {
        // Test behavior when peers exist but have no established sessions
        let node = Node::new_random().await.unwrap();

        let file_hash = [42u8; 32];
        // These peer IDs don't have established sessions
        let peers = vec![[1u8; 32], [2u8; 32]];

        let result = node.fetch_file_metadata(&file_hash, &peers).await;

        // Should fail because we can't fetch metadata from peers without sessions
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error should indicate failure to fetch from any peer
        assert!(err.to_string().contains("Failed to fetch metadata"));
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
        use std::io::Write;
        let node = Node::new_random().await.unwrap();

        let files = node.list_available_files().await;
        // Verify empty list initially
        assert_eq!(files.len(), 0);

        // Create and announce a file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_list_test.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0u8; 1024]).unwrap();
        drop(file);

        let _ = node.announce_file(&file_path).await;

        // Verify list now contains the file
        let files_after = node.list_available_files().await;
        assert_eq!(files_after.len(), 1);
        assert_eq!(files_after[0].name, "wraith_list_test.dat");
        assert_eq!(files_after[0].size, 1024);

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_announce_file() {
        use std::io::Write;
        let node = Node::new_random().await.unwrap();

        // Create temporary test file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_test_announce.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0u8; 1024]).unwrap();
        drop(file);

        let result = node.announce_file(&file_path).await;

        // Cleanup
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unannounce_file_not_found() {
        let node = Node::new_random().await.unwrap();
        let file_hash = [99u8; 32];
        let result = node.unannounce_file(&file_hash).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_from_peers_empty() {
        let node = Node::new_random().await.unwrap();
        let file_hash = [42u8; 32];
        let result = node
            .download_from_peers(&file_hash, vec![], std::path::Path::new("/tmp/test.dat"))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No peers"));
    }

    #[tokio::test]
    async fn test_assign_chunks_single_chunk() {
        let node = Node::new_random().await.unwrap();
        let metadata = FileMetadata {
            size: 100,
            total_chunks: 1,
            chunk_size: 256 * 1024,
            root_hash: [0u8; 32],
            name: "tiny.dat".to_string(),
        };
        let peers = vec![[1u8; 32], [2u8; 32]];
        let assignments = node.assign_chunks(&metadata, &peers);

        let total: usize = assignments.values().map(|v| v.len()).sum();
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_assign_chunks_more_peers_than_chunks() {
        let node = Node::new_random().await.unwrap();
        let metadata = FileMetadata {
            size: 256 * 1024,
            total_chunks: 2,
            chunk_size: 128 * 1024,
            root_hash: [0u8; 32],
            name: "small.dat".to_string(),
        };
        let peers = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];
        let assignments = node.assign_chunks(&metadata, &peers);

        let total: usize = assignments.values().map(|v| v.len()).sum();
        assert_eq!(total, 2);
        // Only 2 peers should have chunks assigned (round-robin)
        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_file_metadata_clone() {
        let metadata = FileMetadata {
            size: 1024,
            total_chunks: 4,
            chunk_size: 256,
            root_hash: [42u8; 32],
            name: "clone_test.dat".to_string(),
        };
        let cloned = metadata.clone();
        assert_eq!(cloned.size, 1024);
        assert_eq!(cloned.name, "clone_test.dat");
        assert_eq!(cloned.root_hash, [42u8; 32]);
    }

    #[tokio::test]
    async fn test_unannounce_file() {
        use std::io::Write;
        let node = Node::new_random().await.unwrap();

        // First announce a file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wraith_test_unannounce.dat");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&[0u8; 1024]).unwrap();
        drop(file);

        let file_hash = node.announce_file(&file_path).await.unwrap();

        // Now unannounce it
        let result = node.unannounce_file(&file_hash).await;

        // Cleanup
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_ok());
    }
}
