//! File transfer coordination for Node.
//!
//! This module provides helpers for coordinating file transfers between nodes:
//! - Metadata message serialization/deserialization
//! - Chunk-to-frame conversion
//! - Progress tracking integration

use crate::FRAME_HEADER_SIZE;
use crate::frame::{FrameBuilder, FrameType};
use crate::node::error::{NodeError, Result};
use crate::transfer::session::TransferSession;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use wraith_files::chunker::FileReassembler;
use wraith_files::tree_hash::FileTreeHash;

/// File transfer context consolidating all per-transfer state
///
/// This struct combines the transfer session, file reassembler (for receives),
/// and tree hash into a single context object, reducing HashMap lookups and
/// simplifying transfer state management.
#[derive(Clone)]
pub struct FileTransferContext {
    /// Transfer ID (32 bytes)
    pub transfer_id: [u8; 32],

    /// Transfer session (send/receive state, progress, peers)
    pub transfer_session: Arc<RwLock<TransferSession>>,

    /// File reassembler for receive transfers (None for send transfers)
    pub reassembler: Option<Arc<Mutex<FileReassembler>>>,

    /// Tree hash for integrity verification
    pub tree_hash: FileTreeHash,
}

impl FileTransferContext {
    /// Create context for send transfer
    pub fn new_send(
        transfer_id: [u8; 32],
        transfer_session: Arc<RwLock<TransferSession>>,
        tree_hash: FileTreeHash,
    ) -> Self {
        Self {
            transfer_id,
            transfer_session,
            reassembler: None,
            tree_hash,
        }
    }

    /// Create context for receive transfer
    pub fn new_receive(
        transfer_id: [u8; 32],
        transfer_session: Arc<RwLock<TransferSession>>,
        reassembler: Arc<Mutex<FileReassembler>>,
        tree_hash: FileTreeHash,
    ) -> Self {
        Self {
            transfer_id,
            transfer_session,
            reassembler: Some(reassembler),
            tree_hash,
        }
    }
}

/// File transfer metadata sent in StreamOpen frame
///
/// This struct is serialized and sent as the payload of a StreamOpen frame
/// to initiate a file transfer. It contains all necessary information for
/// the receiver to prepare for the incoming file.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Transfer ID (32 bytes)
    pub transfer_id: [u8; 32],
    /// File name (UTF-8 encoded, max 255 bytes)
    pub file_name: String,
    /// File size in bytes
    pub file_size: u64,
    /// Chunk size in bytes
    pub chunk_size: u32,
    /// Total number of chunks
    pub total_chunks: u64,
    /// BLAKE3 root hash (32 bytes)
    pub root_hash: [u8; 32],
}

impl FileMetadata {
    /// Create metadata from file path and tree hash
    pub fn from_path_and_hash(
        transfer_id: [u8; 32],
        path: &Path,
        file_size: u64,
        chunk_size: usize,
        tree_hash: &FileTreeHash,
    ) -> Result<Self> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| NodeError::InvalidState("Invalid file name".to_string()))?
            .to_string();

        if file_name.len() > 255 {
            return Err(NodeError::InvalidState(
                "File name too long (max 255 bytes)".to_string(),
            ));
        }

        let total_chunks = file_size.div_ceil(chunk_size as u64);

        Ok(Self {
            transfer_id,
            file_name,
            file_size,
            chunk_size: chunk_size as u32,
            total_chunks,
            root_hash: tree_hash.root,
        })
    }

    /// Serialize metadata to bytes
    ///
    /// Format:
    /// - 32 bytes: transfer_id
    /// - 1 byte: file_name length
    /// - N bytes: file_name (UTF-8)
    /// - 8 bytes: file_size (big-endian)
    /// - 4 bytes: chunk_size (big-endian)
    /// - 8 bytes: total_chunks (big-endian)
    /// - 32 bytes: root_hash
    ///
    /// Total: 85 + file_name.len() bytes
    pub fn serialize(&self) -> Vec<u8> {
        let file_name_bytes = self.file_name.as_bytes();
        let file_name_len = file_name_bytes.len() as u8;

        let mut buf = Vec::with_capacity(85 + file_name_bytes.len());

        // Transfer ID (32 bytes)
        buf.extend_from_slice(&self.transfer_id);

        // File name length and data (1 + N bytes)
        buf.push(file_name_len);
        buf.extend_from_slice(file_name_bytes);

        // File size (8 bytes, big-endian)
        buf.extend_from_slice(&self.file_size.to_be_bytes());

        // Chunk size (4 bytes, big-endian)
        buf.extend_from_slice(&self.chunk_size.to_be_bytes());

        // Total chunks (8 bytes, big-endian)
        buf.extend_from_slice(&self.total_chunks.to_be_bytes());

        // Root hash (32 bytes)
        buf.extend_from_slice(&self.root_hash);

        buf
    }

    /// Deserialize metadata from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 85 {
            return Err(NodeError::InvalidState(
                "Metadata too short (min 85 bytes)".to_string(),
            ));
        }

        let mut offset = 0;

        // Transfer ID (32 bytes)
        let mut transfer_id = [0u8; 32];
        transfer_id.copy_from_slice(&data[offset..offset + 32]);
        offset += 32;

        // File name length and data
        let file_name_len = data[offset] as usize;
        offset += 1;

        if data.len() < 85 + file_name_len {
            return Err(NodeError::InvalidState(
                "Metadata truncated (file name)".to_string(),
            ));
        }

        let file_name = String::from_utf8(data[offset..offset + file_name_len].to_vec())
            .map_err(|e| NodeError::InvalidState(format!("Invalid file name UTF-8: {}", e)))?;
        offset += file_name_len;

        // File size (8 bytes)
        let file_size = u64::from_be_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| NodeError::InvalidState("Invalid file_size".to_string()))?,
        );
        offset += 8;

        // Chunk size (4 bytes)
        let chunk_size = u32::from_be_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| NodeError::InvalidState("Invalid chunk_size".to_string()))?,
        );
        offset += 4;

        // Total chunks (8 bytes)
        let total_chunks = u64::from_be_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| NodeError::InvalidState("Invalid total_chunks".to_string()))?,
        );
        offset += 8;

        // Root hash (32 bytes)
        let mut root_hash = [0u8; 32];
        root_hash.copy_from_slice(&data[offset..offset + 32]);

        Ok(Self {
            transfer_id,
            file_name,
            file_size,
            chunk_size,
            total_chunks,
            root_hash,
        })
    }
}

/// Build a metadata frame (StreamOpen) for file transfer
pub fn build_metadata_frame(stream_id: u16, metadata: &FileMetadata) -> Result<Vec<u8>> {
    let metadata_bytes = metadata.serialize();
    let frame_size = FRAME_HEADER_SIZE + metadata_bytes.len();

    FrameBuilder::new()
        .frame_type(FrameType::StreamOpen)
        .stream_id(stream_id)
        .sequence(0)
        .payload(&metadata_bytes)
        .build(frame_size)
        .map_err(|e| NodeError::InvalidState(format!("Failed to build metadata frame: {}", e)))
}

/// Build a data frame for file chunk
pub fn build_chunk_frame(stream_id: u16, chunk_index: u64, chunk_data: &[u8]) -> Result<Vec<u8>> {
    let frame_size = FRAME_HEADER_SIZE + chunk_data.len();

    // Use chunk_index as sequence number
    let sequence = chunk_index as u32;

    FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(stream_id)
        .sequence(sequence)
        .offset(chunk_index * chunk_data.len() as u64) // File offset
        .payload(chunk_data)
        .build(frame_size)
        .map_err(|e| NodeError::InvalidState(format!("Failed to build chunk frame: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization_roundtrip() {
        let metadata = FileMetadata {
            transfer_id: [42u8; 32],
            file_name: "test_file.txt".to_string(),
            file_size: 1024 * 1024,
            chunk_size: 256 * 1024,
            total_chunks: 4,
            root_hash: [0xAB; 32],
        };

        let serialized = metadata.serialize();
        let deserialized = FileMetadata::deserialize(&serialized).unwrap();

        assert_eq!(metadata.transfer_id, deserialized.transfer_id);
        assert_eq!(metadata.file_name, deserialized.file_name);
        assert_eq!(metadata.file_size, deserialized.file_size);
        assert_eq!(metadata.chunk_size, deserialized.chunk_size);
        assert_eq!(metadata.total_chunks, deserialized.total_chunks);
        assert_eq!(metadata.root_hash, deserialized.root_hash);
    }

    #[test]
    fn test_metadata_long_filename() {
        let metadata = FileMetadata {
            transfer_id: [1u8; 32],
            file_name: "a".repeat(255),
            file_size: 1000,
            chunk_size: 256,
            total_chunks: 4,
            root_hash: [2u8; 32],
        };

        let serialized = metadata.serialize();
        let deserialized = FileMetadata::deserialize(&serialized).unwrap();

        assert_eq!(metadata.file_name, deserialized.file_name);
    }

    #[test]
    fn test_metadata_deserialize_truncated() {
        let short_data = vec![0u8; 50]; // Too short
        assert!(FileMetadata::deserialize(&short_data).is_err());
    }

    #[test]
    fn test_build_metadata_frame() {
        let metadata = FileMetadata {
            transfer_id: [1u8; 32],
            file_name: "test.dat".to_string(),
            file_size: 1024,
            chunk_size: 256,
            total_chunks: 4,
            root_hash: [2u8; 32],
        };

        let frame_bytes = build_metadata_frame(42, &metadata).unwrap();

        // Verify frame can be parsed
        let frame = crate::frame::Frame::parse(&frame_bytes).unwrap();
        assert_eq!(frame.frame_type(), FrameType::StreamOpen);
        assert_eq!(frame.stream_id(), 42);
        assert_eq!(frame.sequence(), 0);

        // Verify metadata can be deserialized from payload
        let parsed_metadata = FileMetadata::deserialize(frame.payload()).unwrap();
        assert_eq!(metadata.file_name, parsed_metadata.file_name);
        assert_eq!(metadata.file_size, parsed_metadata.file_size);
    }

    #[test]
    fn test_build_chunk_frame() {
        let chunk_data = vec![0xAB; 1024];
        let chunk_index = 5;

        let frame_bytes = build_chunk_frame(100, chunk_index, &chunk_data).unwrap();

        // Verify frame
        let frame = crate::frame::Frame::parse(&frame_bytes).unwrap();
        assert_eq!(frame.frame_type(), FrameType::Data);
        assert_eq!(frame.stream_id(), 100);
        assert_eq!(frame.sequence(), chunk_index as u32);
        assert_eq!(frame.payload(), &chunk_data);
    }
}
