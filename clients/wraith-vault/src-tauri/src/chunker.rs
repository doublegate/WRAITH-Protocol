//! Content-Defined Chunking for WRAITH Vault
//!
//! Implements variable-size chunking using a rolling hash (Rabin fingerprint)
//! for better deduplication across similar files.

use crate::error::{VaultError, VaultResult};
use std::io::Read;

/// Minimum chunk size (4 KB)
pub const MIN_CHUNK_SIZE: usize = 4 * 1024;

/// Average chunk size (16 KB)
pub const AVG_CHUNK_SIZE: usize = 16 * 1024;

/// Maximum chunk size (64 KB)
pub const MAX_CHUNK_SIZE: usize = 64 * 1024;

/// Rolling hash window size
const WINDOW_SIZE: usize = 64;

/// Mask for average chunk boundary detection
const CHUNK_MASK: u32 = (AVG_CHUNK_SIZE - 1) as u32;

/// A chunk of data with its BLAKE3 hash
#[derive(Clone, Debug)]
pub struct Chunk {
    /// BLAKE3 hash of the chunk data
    pub hash: [u8; 32],
    /// Raw chunk data
    pub data: Vec<u8>,
    /// Size of the chunk in bytes
    pub size: usize,
}

impl Chunk {
    /// Create a new chunk from data
    pub fn new(data: Vec<u8>) -> Self {
        let hash = blake3::hash(&data);
        let size = data.len();
        Self {
            hash: *hash.as_bytes(),
            data,
            size,
        }
    }

    /// Get the hash as a hex string
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash)
    }
}

/// Content-defined chunker using rolling hash
pub struct Chunker {
    /// Minimum chunk size
    min_chunk_size: usize,
    /// Maximum chunk size
    max_chunk_size: usize,
    /// Rolling hash value
    rolling_hash: u32,
    /// Sliding window for rolling hash
    window: Vec<u8>,
    /// Window position
    window_pos: usize,
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    /// Create a new chunker with default settings
    pub fn new() -> Self {
        Self {
            min_chunk_size: MIN_CHUNK_SIZE,
            max_chunk_size: MAX_CHUNK_SIZE,
            rolling_hash: 0,
            window: vec![0u8; WINDOW_SIZE],
            window_pos: 0,
        }
    }

    /// Create a chunker with custom chunk sizes
    pub fn with_sizes(min: usize, _avg: usize, max: usize) -> Self {
        Self {
            min_chunk_size: min,
            max_chunk_size: max,
            rolling_hash: 0,
            window: vec![0u8; WINDOW_SIZE],
            window_pos: 0,
        }
    }

    /// Reset the chunker state
    fn reset(&mut self) {
        self.rolling_hash = 0;
        self.window.fill(0);
        self.window_pos = 0;
    }

    /// Update the rolling hash with a new byte
    fn update_hash(&mut self, byte: u8) {
        // Remove old byte's contribution
        let old_byte = self.window[self.window_pos];

        // Rabin fingerprint update
        self.rolling_hash = self.rolling_hash.rotate_left(1);
        self.rolling_hash ^= old_byte as u32;
        self.rolling_hash ^= byte as u32;

        // Store new byte
        self.window[self.window_pos] = byte;
        self.window_pos = (self.window_pos + 1) % WINDOW_SIZE;
    }

    /// Check if current position is a chunk boundary
    fn is_boundary(&self) -> bool {
        (self.rolling_hash & CHUNK_MASK) == 0
    }

    /// Chunk a file from a reader
    pub fn chunk_file<R: Read>(&mut self, reader: &mut R) -> VaultResult<Vec<Chunk>> {
        self.reset();

        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.max_chunk_size * 2];
        let mut chunk_data = Vec::with_capacity(self.max_chunk_size);
        let mut total_read = 0;

        loop {
            let bytes_read = reader
                .read(&mut buffer[total_read..])
                .map_err(|e| VaultError::Chunk(format!("Failed to read: {}", e)))?;

            if bytes_read == 0 && total_read == 0 {
                break;
            }

            total_read += bytes_read;

            let mut pos = 0;
            while pos < total_read {
                let byte = buffer[pos];
                chunk_data.push(byte);
                self.update_hash(byte);
                pos += 1;

                let chunk_len = chunk_data.len();

                // Check for chunk boundary
                let should_split = (chunk_len >= self.min_chunk_size && self.is_boundary())
                    || chunk_len >= self.max_chunk_size;

                if should_split {
                    chunks.push(Chunk::new(std::mem::take(&mut chunk_data)));
                    chunk_data = Vec::with_capacity(self.max_chunk_size);
                    self.reset();
                }
            }

            // If we've processed all data but more might come
            if bytes_read == 0 {
                break;
            }

            // Keep unprocessed data for next iteration
            total_read = 0;
        }

        // Handle remaining data as final chunk
        if !chunk_data.is_empty() {
            chunks.push(Chunk::new(chunk_data));
        }

        Ok(chunks)
    }

    /// Chunk raw data
    pub fn chunk_data(&mut self, data: &[u8]) -> Vec<Chunk> {
        self.reset();

        let mut chunks = Vec::new();
        let mut chunk_data = Vec::with_capacity(self.max_chunk_size);

        for &byte in data {
            chunk_data.push(byte);
            self.update_hash(byte);

            let chunk_len = chunk_data.len();
            let should_split = (chunk_len >= self.min_chunk_size && self.is_boundary())
                || chunk_len >= self.max_chunk_size;

            if should_split {
                chunks.push(Chunk::new(std::mem::take(&mut chunk_data)));
                chunk_data = Vec::with_capacity(self.max_chunk_size);
                self.reset();
            }
        }

        // Final chunk
        if !chunk_data.is_empty() {
            chunks.push(Chunk::new(chunk_data));
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_chunk_small_data() {
        let mut chunker = Chunker::new();
        let data = vec![0u8; 1024]; // 1 KB - smaller than min chunk

        let chunks = chunker.chunk_data(&data);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, 1024);
    }

    #[test]
    fn test_chunk_large_data() {
        let mut chunker = Chunker::new();
        let data = vec![0u8; MAX_CHUNK_SIZE * 3]; // 3x max chunk size

        let chunks = chunker.chunk_data(&data);

        // Should create at least 3 chunks
        assert!(chunks.len() >= 3);

        // Total size should match
        let total: usize = chunks.iter().map(|c| c.size).sum();
        assert_eq!(total, data.len());
    }

    #[test]
    fn test_chunk_file() {
        let mut chunker = Chunker::new();
        let data = vec![42u8; 50_000]; // 50 KB
        let mut cursor = Cursor::new(data.clone());

        let chunks = chunker.chunk_file(&mut cursor).unwrap();

        // Reconstruct and verify
        let reconstructed: Vec<u8> = chunks.iter().flat_map(|c| c.data.iter().copied()).collect();

        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_chunk_hash_consistency() {
        let mut chunker = Chunker::new();
        let data = b"Hello, WRAITH Vault!";

        let chunks1 = chunker.chunk_data(data);
        let chunks2 = chunker.chunk_data(data);

        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(c1.hash, c2.hash);
        }
    }

    #[test]
    fn test_chunk_deterministic() {
        let mut chunker1 = Chunker::new();
        let mut chunker2 = Chunker::new();

        // Random-ish data
        let data: Vec<u8> = (0..100_000).map(|i| (i * 7 + 13) as u8).collect();

        let chunks1 = chunker1.chunk_data(&data);
        let chunks2 = chunker2.chunk_data(&data);

        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(c1.hash, c2.hash);
            assert_eq!(c1.size, c2.size);
        }
    }
}
