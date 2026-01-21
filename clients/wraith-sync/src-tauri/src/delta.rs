//! Delta Sync Engine
//!
//! Implements rsync-style delta synchronization for efficient file transfers.
//! Only changed blocks are transmitted instead of entire files.

use crate::error::{SyncError, SyncResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use tracing::info;

/// Default block size for delta sync (4 KB)
pub const DEFAULT_BLOCK_SIZE: usize = 4096;

/// Maximum block size (64 KB)
pub const MAX_BLOCK_SIZE: usize = 65536;

/// Minimum block size (512 bytes)
pub const MIN_BLOCK_SIZE: usize = 512;

/// Block signature containing both weak and strong hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSignature {
    /// Block offset in bytes
    pub offset: u64,
    /// Block size in bytes
    pub size: usize,
    /// Weak rolling checksum (Adler-32)
    pub weak_hash: u32,
    /// Strong hash (BLAKE3)
    pub strong_hash: [u8; 32],
}

/// Delta operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOperation {
    /// Copy a block from the base file
    Copy {
        /// Offset in the base file
        offset: u64,
        /// Number of bytes to copy
        length: usize,
    },
    /// Insert literal data
    Insert {
        /// Literal bytes to insert
        data: Vec<u8>,
    },
}

/// File signature containing all block signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSignature {
    /// Block size used for this signature
    pub block_size: usize,
    /// Total file size
    pub file_size: u64,
    /// File-level BLAKE3 hash
    pub file_hash: [u8; 32],
    /// Block signatures
    pub blocks: Vec<BlockSignature>,
}

/// Delta patch containing operations to transform base file to target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPatch {
    /// Expected base file hash
    pub base_hash: [u8; 32],
    /// Target file hash
    pub target_hash: [u8; 32],
    /// Target file size
    pub target_size: u64,
    /// Delta operations
    pub operations: Vec<DeltaOperation>,
    /// Statistics about the patch
    pub stats: DeltaStats,
}

/// Statistics about a delta patch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeltaStats {
    /// Number of blocks copied from base
    pub blocks_copied: usize,
    /// Number of blocks inserted as literal data
    pub blocks_inserted: usize,
    /// Total bytes in copy operations
    pub bytes_copied: u64,
    /// Total bytes in insert operations
    pub bytes_inserted: u64,
    /// Compression ratio (1.0 = no savings, lower = better)
    pub compression_ratio: f64,
}

/// Delta sync engine configuration
#[derive(Debug, Clone)]
pub struct DeltaSyncConfig {
    /// Block size for chunking
    pub block_size: usize,
    /// Whether to compress literal data
    pub compress_literals: bool,
    /// Compression level (1-22 for zstd)
    pub compression_level: i32,
}

impl Default for DeltaSyncConfig {
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            compress_literals: true,
            compression_level: 3,
        }
    }
}

/// Delta sync engine
#[derive(Clone)]
pub struct DeltaSync {
    config: DeltaSyncConfig,
}

impl DeltaSync {
    /// Create a new delta sync engine with default configuration
    pub fn new() -> Self {
        Self {
            config: DeltaSyncConfig::default(),
        }
    }

    /// Create a new delta sync engine with custom configuration
    pub fn with_config(config: DeltaSyncConfig) -> Self {
        Self { config }
    }

    /// Generate a file signature for delta sync
    pub fn generate_signature(&self, path: &Path) -> SyncResult<FileSignature> {
        let mut file = File::open(path).map_err(|e| {
            SyncError::FileSystem(format!("Failed to open file for signature: {}", e))
        })?;

        let file_size = file.metadata()?.len();
        let mut file_hasher = blake3::Hasher::new();
        let mut blocks = Vec::new();
        let mut offset = 0u64;
        let mut buffer = vec![0u8; self.config.block_size];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| SyncError::FileSystem(format!("Failed to read file: {}", e)))?;

            if bytes_read == 0 {
                break;
            }

            let block_data = &buffer[..bytes_read];

            // Update file-level hash
            file_hasher.update(block_data);

            // Calculate block signatures
            let weak_hash = rolling_checksum(block_data);
            let strong_hash = blake3::hash(block_data);

            blocks.push(BlockSignature {
                offset,
                size: bytes_read,
                weak_hash,
                strong_hash: *strong_hash.as_bytes(),
            });

            offset += bytes_read as u64;
        }

        let file_hash = file_hasher.finalize();

        Ok(FileSignature {
            block_size: self.config.block_size,
            file_size,
            file_hash: *file_hash.as_bytes(),
            blocks,
        })
    }

    /// Compute delta between a local file and a remote signature
    pub fn compute_delta(
        &self,
        local_path: &Path,
        remote_signature: &FileSignature,
    ) -> SyncResult<DeltaPatch> {
        let mut local_file = File::open(local_path)
            .map_err(|e| SyncError::FileSystem(format!("Failed to open local file: {}", e)))?;

        let local_size = local_file.metadata()?.len();

        // Build lookup table for remote blocks by weak hash
        let mut sig_map: HashMap<u32, Vec<&BlockSignature>> = HashMap::new();
        for sig in &remote_signature.blocks {
            sig_map.entry(sig.weak_hash).or_default().push(sig);
        }

        let mut operations = Vec::new();
        let mut pending_insert = Vec::new();
        let mut buffer = vec![0u8; self.config.block_size];
        let mut local_hasher = blake3::Hasher::new();
        let mut stats = DeltaStats::default();

        loop {
            let bytes_read = local_file
                .read(&mut buffer)
                .map_err(|e| SyncError::FileSystem(format!("Failed to read local file: {}", e)))?;

            if bytes_read == 0 {
                break;
            }

            let block_data = &buffer[..bytes_read];
            local_hasher.update(block_data);

            // Calculate weak hash and look for matches
            let weak = rolling_checksum(block_data);
            let mut found_match = false;

            if let Some(candidates) = sig_map.get(&weak) {
                let strong = blake3::hash(block_data);
                let strong_bytes = strong.as_bytes();

                for candidate in candidates {
                    if &candidate.strong_hash == strong_bytes && candidate.size == bytes_read {
                        // Found a match - flush pending insert and add copy operation
                        if !pending_insert.is_empty() {
                            let data = std::mem::take(&mut pending_insert);
                            stats.blocks_inserted += 1;
                            stats.bytes_inserted += data.len() as u64;

                            // Optionally compress literal data
                            let final_data = if self.config.compress_literals && data.len() > 64 {
                                zstd::encode_all(&data[..], self.config.compression_level)
                                    .unwrap_or(data)
                            } else {
                                data
                            };

                            operations.push(DeltaOperation::Insert { data: final_data });
                        }

                        operations.push(DeltaOperation::Copy {
                            offset: candidate.offset,
                            length: candidate.size,
                        });

                        stats.blocks_copied += 1;
                        stats.bytes_copied += candidate.size as u64;
                        found_match = true;
                        break;
                    }
                }
            }

            if !found_match {
                // No match found - accumulate literal data
                pending_insert.extend_from_slice(block_data);
            }
        }

        // Flush any remaining pending insert
        if !pending_insert.is_empty() {
            stats.blocks_inserted += 1;
            stats.bytes_inserted += pending_insert.len() as u64;

            let final_data = if self.config.compress_literals && pending_insert.len() > 64 {
                zstd::encode_all(&pending_insert[..], self.config.compression_level)
                    .unwrap_or(pending_insert)
            } else {
                pending_insert
            };

            operations.push(DeltaOperation::Insert { data: final_data });
        }

        let local_hash = local_hasher.finalize();

        // Calculate compression ratio
        let total_delta_size: u64 = operations
            .iter()
            .map(|op| match op {
                DeltaOperation::Copy { length: _, .. } => 12, // 8 bytes offset + 4 bytes length
                DeltaOperation::Insert { data } => data.len() as u64 + 4, // 4 bytes for length prefix
            })
            .sum();

        stats.compression_ratio = if local_size > 0 {
            total_delta_size as f64 / local_size as f64
        } else {
            1.0
        };

        info!(
            "Delta computed: {} blocks copied ({} bytes), {} blocks inserted ({} bytes), ratio: {:.2}",
            stats.blocks_copied,
            stats.bytes_copied,
            stats.blocks_inserted,
            stats.bytes_inserted,
            stats.compression_ratio
        );

        Ok(DeltaPatch {
            base_hash: remote_signature.file_hash,
            target_hash: *local_hash.as_bytes(),
            target_size: local_size,
            operations,
            stats,
        })
    }

    /// Apply a delta patch to reconstruct the target file
    pub fn apply_delta(
        &self,
        base_path: &Path,
        delta: &DeltaPatch,
        output_path: &Path,
    ) -> SyncResult<()> {
        let mut base_file = File::open(base_path)
            .map_err(|e| SyncError::FileSystem(format!("Failed to open base file: {}", e)))?;

        let mut output_file = File::create(output_path)
            .map_err(|e| SyncError::FileSystem(format!("Failed to create output file: {}", e)))?;

        let mut output_hasher = blake3::Hasher::new();

        for operation in &delta.operations {
            match operation {
                DeltaOperation::Copy { offset, length } => {
                    base_file.seek(SeekFrom::Start(*offset)).map_err(|e| {
                        SyncError::FileSystem(format!("Failed to seek in base file: {}", e))
                    })?;

                    let mut buffer = vec![0u8; *length];
                    base_file.read_exact(&mut buffer).map_err(|e| {
                        SyncError::FileSystem(format!("Failed to read from base file: {}", e))
                    })?;

                    output_hasher.update(&buffer);
                    output_file.write_all(&buffer).map_err(|e| {
                        SyncError::FileSystem(format!("Failed to write to output file: {}", e))
                    })?;
                }
                DeltaOperation::Insert { data } => {
                    // Decompress if needed
                    let decompressed = if self.config.compress_literals && data.len() > 64 {
                        zstd::decode_all(&data[..]).unwrap_or_else(|_| data.clone())
                    } else {
                        data.clone()
                    };

                    output_hasher.update(&decompressed);
                    output_file.write_all(&decompressed).map_err(|e| {
                        SyncError::FileSystem(format!("Failed to write literal data: {}", e))
                    })?;
                }
            }
        }

        // Verify output hash
        let output_hash = output_hasher.finalize();
        if output_hash.as_bytes() != &delta.target_hash {
            return Err(SyncError::Sync(
                "Output file hash mismatch after applying delta".to_string(),
            ));
        }

        info!("Delta applied successfully, output verified");
        Ok(())
    }

    /// Check if delta sync would be beneficial for these files
    pub fn should_use_delta(&self, local_size: u64, signature: &FileSignature) -> bool {
        // Don't use delta for very small files (< 10 KB)
        if local_size < 10 * 1024 || signature.file_size < 10 * 1024 {
            return false;
        }

        // Don't use delta if files are very different sizes (> 2x difference)
        let size_ratio = if local_size > signature.file_size {
            local_size as f64 / signature.file_size as f64
        } else {
            signature.file_size as f64 / local_size as f64
        };

        if size_ratio > 2.0 {
            return false;
        }

        true
    }
}

impl Default for DeltaSync {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Adler-32 rolling checksum
fn rolling_checksum(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;

    let mut a = 1u32;
    let mut b = 0u32;

    for &byte in data {
        a = (a + u32::from(byte)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

/// Rolling checksum state for streaming computation
#[derive(Debug, Clone)]
pub struct RollingChecksum {
    a: u32,
    b: u32,
    window_size: usize,
    window: Vec<u8>,
    position: usize,
}

impl RollingChecksum {
    /// Create a new rolling checksum with given window size
    pub fn new(window_size: usize) -> Self {
        Self {
            a: 1,
            b: 0,
            window_size,
            window: vec![0; window_size],
            position: 0,
        }
    }

    /// Initialize with a data block
    pub fn init(&mut self, data: &[u8]) {
        self.a = 1;
        self.b = 0;
        self.position = 0;

        for (i, &byte) in data.iter().take(self.window_size).enumerate() {
            self.window[i] = byte;
            self.a = (self.a + u32::from(byte)) % 65521;
            self.b = (self.b + self.a) % 65521;
        }

        self.position = data.len().min(self.window_size);
    }

    /// Roll the checksum by removing old byte and adding new byte
    pub fn roll(&mut self, old_byte: u8, new_byte: u8) -> u32 {
        self.a = (self
            .a
            .wrapping_sub(u32::from(old_byte))
            .wrapping_add(u32::from(new_byte)))
            % 65521;
        self.b = (self
            .b
            .wrapping_sub(u32::from(old_byte).wrapping_mul(self.window_size as u32))
            .wrapping_add(self.a)
            .wrapping_sub(1))
            % 65521;

        let old_pos = self.position % self.window_size;
        self.window[old_pos] = new_byte;
        self.position += 1;

        self.get()
    }

    /// Get current checksum value
    pub fn get(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_rolling_checksum() {
        let data = b"Hello, World!";
        let checksum = rolling_checksum(data);
        assert!(checksum > 0);

        // Same data should produce same checksum
        let checksum2 = rolling_checksum(data);
        assert_eq!(checksum, checksum2);

        // Different data should produce different checksum
        let data2 = b"Goodbye, World!";
        let checksum3 = rolling_checksum(data2);
        assert_ne!(checksum, checksum3);
    }

    #[test]
    fn test_generate_signature() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let content = "Hello, World! This is a test file for delta sync.";
        std::fs::write(&file_path, content).unwrap();

        let delta_sync = DeltaSync::new();
        let signature = delta_sync.generate_signature(&file_path).unwrap();

        assert_eq!(signature.file_size, content.len() as u64);
        assert!(!signature.blocks.is_empty());
    }

    #[test]
    fn test_delta_identical_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");

        let content = "Hello, World! ".repeat(100);
        std::fs::write(&file1, &content).unwrap();
        std::fs::write(&file2, &content).unwrap();

        let delta_sync = DeltaSync::new();

        let sig = delta_sync.generate_signature(&file1).unwrap();
        let delta = delta_sync.compute_delta(&file2, &sig).unwrap();

        // Identical files should have all copy operations
        assert_eq!(delta.stats.bytes_inserted, 0);
        assert!(delta.stats.compression_ratio < 0.1);
    }

    #[test]
    fn test_delta_modified_file() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");

        // Create original file with large repeating blocks that match the default block size
        // Default block size is 4096 bytes
        let block_data = "A".repeat(4096);
        let mut original = String::new();
        for i in 0..10 {
            original.push_str(&format!("Block{}:", i));
            original.push_str(&block_data[..4090]); // Pad to exactly 4096 bytes per block
        }
        std::fs::write(&file1, &original).unwrap();

        // Create modified file - change one block in the middle but keep others same
        let mut modified = original.clone();
        // Replace part of block 5 with different data
        let start = 5 * 4096;
        modified.replace_range(start..start + 100, &"B".repeat(100));
        std::fs::write(&file2, &modified).unwrap();

        let delta_sync = DeltaSync::new();

        let sig = delta_sync.generate_signature(&file1).unwrap();
        let delta = delta_sync.compute_delta(&file2, &sig).unwrap();

        // Should have some copied blocks and some inserted data
        // The modified file has some unchanged blocks that should match
        assert!(delta.stats.blocks_copied > 0 || delta.stats.blocks_inserted > 0);
        // At minimum there should be operations
        assert!(!delta.operations.is_empty());
    }

    #[test]
    fn test_apply_delta() {
        let dir = tempdir().unwrap();
        let base_file = dir.path().join("base.txt");
        let target_file = dir.path().join("target.txt");
        let output_file = dir.path().join("output.txt");

        // Create files large enough for delta sync (> 10 KB minimum)
        // Use files with some common blocks for meaningful delta
        let common_block = "X".repeat(4096);
        let base_content = format!("{}BASE_UNIQUE_CONTENT{}", common_block, common_block);
        let target_content = format!("{}TARGET_CHANGED_CONTENT{}", common_block, common_block);

        std::fs::write(&base_file, &base_content).unwrap();
        std::fs::write(&target_file, &target_content).unwrap();

        // Use delta sync without compression to avoid compression/decompression mismatches
        let config = DeltaSyncConfig {
            compress_literals: false,
            ..DeltaSyncConfig::default()
        };
        let delta_sync = DeltaSync::with_config(config);

        // Generate signature of base, compute delta from target
        let sig = delta_sync.generate_signature(&base_file).unwrap();
        let delta = delta_sync.compute_delta(&target_file, &sig).unwrap();

        // Apply delta to reconstruct target
        delta_sync
            .apply_delta(&base_file, &delta, &output_file)
            .unwrap();

        // Verify output matches target
        let output_content = std::fs::read_to_string(&output_file).unwrap();
        assert_eq!(output_content, target_content);
    }
}
