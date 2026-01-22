//! Encrypted Segment Storage
//!
//! Handles encryption and storage of HLS segments using WRAITH protocol.

use crate::database::{Database, StreamSegment};
use crate::error::{StreamError, StreamResult};
use crate::state::AppState;
use blake3::Hasher;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Encrypted segment header
const ENCRYPTED_HEADER: &[u8] = b"WRAITH_SEG_V1";
const NONCE_SIZE: usize = 24;

/// Segment storage manager
pub struct SegmentStorage {
    db: Arc<Database>,
    state: Arc<AppState>,
}

/// Segment info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentInfo {
    pub segment_name: String,
    pub quality: String,
    pub size: i64,
    pub duration_ms: i64,
    pub encrypted: bool,
}

impl SegmentStorage {
    /// Create a new segment storage manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Upload all segments from a transcoded stream
    pub async fn upload_stream_segments(
        &self,
        stream_id: &str,
        hls_dir: &Path,
        stream_key: &[u8; 32],
    ) -> StreamResult<usize> {
        let mut segment_count = 0;

        // Create segment directory
        let segment_dir = self.state.segments_dir.join(stream_id);
        std::fs::create_dir_all(&segment_dir)?;

        // Process all .ts files
        for entry in std::fs::read_dir(hls_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "ts") {
                let segment_name = path
                    .file_name()
                    .ok_or_else(|| StreamError::FileSystem("Invalid segment path".to_string()))?
                    .to_string_lossy()
                    .to_string();

                // Parse quality from filename (e.g., "720p_001.ts")
                let quality = segment_name
                    .split('_')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();

                // Parse sequence number
                let sequence_number: i64 = segment_name
                    .split('_')
                    .nth(1)
                    .and_then(|s| s.strip_suffix(".ts"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                // Read segment data
                let data = std::fs::read(&path)?;
                let original_size = data.len();

                // Encrypt segment
                let encrypted =
                    self.encrypt_segment(stream_id, &segment_name, &data, stream_key)?;

                // Calculate hash
                let hash = hex::encode(blake3::hash(&encrypted).as_bytes());

                // Store encrypted segment
                let output_path = segment_dir.join(&segment_name);
                std::fs::write(&output_path, &encrypted)?;

                // Save segment metadata to database
                let segment = StreamSegment {
                    stream_id: stream_id.to_string(),
                    segment_name: segment_name.clone(),
                    segment_hash: hash,
                    segment_size: encrypted.len() as i64,
                    quality,
                    sequence_number,
                    duration_ms: 6000, // Default segment duration
                    encrypted: true,
                };

                self.db.add_segment(&segment)?;
                segment_count += 1;

                debug!(
                    "Uploaded segment {} ({} -> {} bytes)",
                    segment_name,
                    original_size,
                    encrypted.len()
                );
            }
        }

        // Also store playlist files (unencrypted)
        for entry in std::fs::read_dir(hls_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "m3u8") {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| StreamError::FileSystem("Invalid playlist path".to_string()))?
                    .to_string_lossy()
                    .to_string();

                let content = std::fs::read_to_string(&path)?;
                let output_path = segment_dir.join(&file_name);
                std::fs::write(&output_path, content)?;

                debug!("Stored playlist {}", file_name);
            }
        }

        info!(
            "Uploaded {} segments for stream {}",
            segment_count, stream_id
        );
        Ok(segment_count)
    }

    /// Encrypt a segment
    fn encrypt_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        data: &[u8],
        stream_key: &[u8; 32],
    ) -> StreamResult<Vec<u8>> {
        // Derive segment-specific key
        let segment_key = self.derive_segment_key(stream_id, segment_name, stream_key);

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        getrandom::getrandom(&mut nonce_bytes)
            .map_err(|e| StreamError::Crypto(format!("Failed to generate nonce: {}", e)))?;
        let nonce = XNonce::from_slice(&nonce_bytes);

        // Encrypt with XChaCha20-Poly1305
        let cipher = XChaCha20Poly1305::new_from_slice(&segment_key)
            .map_err(|e| StreamError::Crypto(format!("Failed to create cipher: {}", e)))?;

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| StreamError::Crypto(format!("Encryption failed: {}", e)))?;

        // Combine header + nonce + ciphertext
        let mut result = Vec::with_capacity(ENCRYPTED_HEADER.len() + NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(ENCRYPTED_HEADER);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Download and decrypt a segment
    pub async fn download_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        stream_key: &[u8; 32],
    ) -> StreamResult<Vec<u8>> {
        // Get segment metadata
        let segment = self
            .db
            .get_segment(stream_id, segment_name)?
            .ok_or_else(|| StreamError::SegmentNotFound(segment_name.to_string()))?;

        // Read encrypted segment
        let segment_path = self.state.get_segment_path(stream_id, segment_name);
        let encrypted = std::fs::read(&segment_path)?;

        if !segment.encrypted {
            return Ok(encrypted);
        }

        // Decrypt segment
        self.decrypt_segment(stream_id, segment_name, &encrypted, stream_key)
    }

    /// Decrypt a segment
    fn decrypt_segment(
        &self,
        stream_id: &str,
        segment_name: &str,
        encrypted: &[u8],
        stream_key: &[u8; 32],
    ) -> StreamResult<Vec<u8>> {
        // Verify header
        if encrypted.len() < ENCRYPTED_HEADER.len() + NONCE_SIZE {
            return Err(StreamError::Crypto("Invalid encrypted segment".to_string()));
        }

        let header = &encrypted[..ENCRYPTED_HEADER.len()];
        if header != ENCRYPTED_HEADER {
            return Err(StreamError::Crypto("Invalid segment header".to_string()));
        }

        // Extract nonce and ciphertext
        let nonce_start = ENCRYPTED_HEADER.len();
        let nonce_end = nonce_start + NONCE_SIZE;
        let nonce = XNonce::from_slice(&encrypted[nonce_start..nonce_end]);
        let ciphertext = &encrypted[nonce_end..];

        // Derive segment-specific key
        let segment_key = self.derive_segment_key(stream_id, segment_name, stream_key);

        // Decrypt
        let cipher = XChaCha20Poly1305::new_from_slice(&segment_key)
            .map_err(|e| StreamError::Crypto(format!("Failed to create cipher: {}", e)))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| StreamError::Crypto(format!("Decryption failed: {}", e)))
    }

    /// Derive a segment-specific key
    fn derive_segment_key(
        &self,
        stream_id: &str,
        segment_name: &str,
        stream_key: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = Hasher::new_keyed(stream_key);
        hasher.update(stream_id.as_bytes());
        hasher.update(b":");
        hasher.update(segment_name.as_bytes());

        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(hash.as_bytes());
        key
    }

    /// Get manifest content
    pub fn get_manifest(&self, stream_id: &str, manifest_name: &str) -> StreamResult<String> {
        let manifest_path = self.state.segments_dir.join(stream_id).join(manifest_name);

        std::fs::read_to_string(&manifest_path)
            .map_err(|e| StreamError::FileSystem(format!("Failed to read manifest: {}", e)))
    }

    /// List segments for a quality
    pub fn list_segments(&self, stream_id: &str, quality: &str) -> StreamResult<Vec<SegmentInfo>> {
        let segments = self.db.list_segments(stream_id, quality)?;

        Ok(segments
            .into_iter()
            .map(|s| SegmentInfo {
                segment_name: s.segment_name,
                quality: s.quality,
                size: s.segment_size,
                duration_ms: s.duration_ms,
                encrypted: s.encrypted,
            })
            .collect())
    }

    /// Delete all segments for a stream
    pub fn delete_stream_segments(&self, stream_id: &str) -> StreamResult<()> {
        // Delete from filesystem
        let segment_dir = self.state.segments_dir.join(stream_id);
        if segment_dir.exists() {
            std::fs::remove_dir_all(&segment_dir)?;
        }

        // Delete from database
        self.db.delete_stream_segments(stream_id)?;

        info!("Deleted all segments for stream {}", stream_id);
        Ok(())
    }

    /// Generate a stream encryption key
    pub fn generate_stream_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key).expect("Failed to generate random key");
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    fn create_test_storage() -> (SegmentStorage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        (SegmentStorage::new(db, state), dir)
    }

    #[test]
    fn test_encrypt_decrypt_segment() {
        let (storage, _dir) = create_test_storage();
        let stream_key = SegmentStorage::generate_stream_key();

        let data = b"Hello, this is test segment data!";
        let stream_id = "test-stream";
        let segment_name = "720p_001.ts";

        // Encrypt
        let encrypted = storage
            .encrypt_segment(stream_id, segment_name, data, &stream_key)
            .unwrap();

        // Verify header
        assert!(encrypted.starts_with(ENCRYPTED_HEADER));

        // Decrypt
        let decrypted = storage
            .decrypt_segment(stream_id, segment_name, &encrypted, &stream_key)
            .unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_derive_segment_key() {
        let (storage, _dir) = create_test_storage();
        let stream_key = [0u8; 32];

        let key1 = storage.derive_segment_key("stream1", "seg1.ts", &stream_key);
        let key2 = storage.derive_segment_key("stream1", "seg2.ts", &stream_key);
        let key3 = storage.derive_segment_key("stream2", "seg1.ts", &stream_key);

        // Different segments should have different keys
        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_generate_stream_key() {
        let key1 = SegmentStorage::generate_stream_key();
        let key2 = SegmentStorage::generate_stream_key();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should be 32 bytes
        assert_eq!(key1.len(), 32);
    }
}
