//! Zstandard Compression for WRAITH Vault
//!
//! Provides high-ratio compression with configurable levels.

use crate::error::{VaultError, VaultResult};

/// Default compression level (good balance of speed and ratio)
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Maximum compression level (best ratio, slowest)
pub const MAX_COMPRESSION_LEVEL: i32 = 22;

/// Zstandard compressor
pub struct Compressor {
    /// Compression level (1-22)
    level: i32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new(DEFAULT_COMPRESSION_LEVEL)
    }
}

impl Compressor {
    /// Create a new compressor with the specified level
    pub fn new(level: i32) -> Self {
        let level = level.clamp(1, MAX_COMPRESSION_LEVEL);
        Self { level }
    }

    /// Compress data using Zstandard
    pub fn compress(&self, data: &[u8]) -> VaultResult<Vec<u8>> {
        zstd::encode_all(data, self.level)
            .map_err(|e| VaultError::Compression(format!("Compression failed: {}", e)))
    }

    /// Decompress Zstandard data
    pub fn decompress(&self, data: &[u8]) -> VaultResult<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| VaultError::Compression(format!("Decompression failed: {}", e)))
    }

    /// Compress data with a size hint for the output buffer
    pub fn compress_with_hint(&self, data: &[u8], size_hint: usize) -> VaultResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size_hint);
        let mut encoder = zstd::stream::Encoder::new(&mut buffer, self.level)
            .map_err(|e| VaultError::Compression(format!("Failed to create encoder: {}", e)))?;

        std::io::copy(&mut std::io::Cursor::new(data), &mut encoder)
            .map_err(|e| VaultError::Compression(format!("Compression copy failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| VaultError::Compression(format!("Failed to finish compression: {}", e)))?;

        Ok(buffer)
    }

    /// Calculate the compression ratio
    pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
        if compressed_size == 0 {
            return 0.0;
        }
        original_size as f64 / compressed_size as f64
    }

    /// Estimate compressed size (rough approximation)
    pub fn estimate_compressed_size(&self, original_size: usize) -> usize {
        // Zstd typically achieves 2:1 to 4:1 ratio for typical data
        // Use conservative estimate
        (original_size * 2) / 3
    }

    /// Get the compression level
    pub fn level(&self) -> i32 {
        self.level
    }

    /// Set the compression level
    pub fn set_level(&mut self, level: i32) {
        self.level = level.clamp(1, MAX_COMPRESSION_LEVEL);
    }
}

/// Statistics about compression operations
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    /// Total bytes before compression
    pub total_original: u64,
    /// Total bytes after compression
    pub total_compressed: u64,
    /// Number of items compressed
    pub item_count: u64,
}

impl CompressionStats {
    /// Add a compression operation to the stats
    pub fn add(&mut self, original_size: usize, compressed_size: usize) {
        self.total_original += original_size as u64;
        self.total_compressed += compressed_size as u64;
        self.item_count += 1;
    }

    /// Get the overall compression ratio
    pub fn ratio(&self) -> f64 {
        if self.total_compressed == 0 {
            return 0.0;
        }
        self.total_original as f64 / self.total_compressed as f64
    }

    /// Get the space saved
    pub fn space_saved(&self) -> u64 {
        self.total_original.saturating_sub(self.total_compressed)
    }

    /// Get the space saved as a percentage
    pub fn space_saved_percent(&self) -> f64 {
        if self.total_original == 0 {
            return 0.0;
        }
        (self.space_saved() as f64 / self.total_original as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let compressor = Compressor::default();
        let data = b"Hello, WRAITH Vault! This is some test data that should compress well.";

        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_large_data() {
        let compressor = Compressor::default();
        // Repeating data compresses very well
        let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

        let compressed = compressor.compress(&data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(decompressed, data);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = Compressor::compression_ratio(1000, 250);
        assert!((ratio - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_compression_levels() {
        let data = b"Test data for compression level comparison";

        let low = Compressor::new(1);
        let high = Compressor::new(19);

        let low_compressed = low.compress(data).unwrap();
        let high_compressed = high.compress(data).unwrap();

        // Higher level should produce smaller or equal output
        assert!(high_compressed.len() <= low_compressed.len());

        // Both should decompress correctly
        assert_eq!(low.decompress(&low_compressed).unwrap(), data);
        assert_eq!(high.decompress(&high_compressed).unwrap(), data);
    }

    #[test]
    fn test_compression_stats() {
        let mut stats = CompressionStats::default();

        stats.add(1000, 250);
        stats.add(2000, 600);

        assert_eq!(stats.total_original, 3000);
        assert_eq!(stats.total_compressed, 850);
        assert_eq!(stats.item_count, 2);
        assert!((stats.ratio() - 3.53).abs() < 0.1);
        assert_eq!(stats.space_saved(), 2150);
    }

    #[test]
    fn test_empty_data() {
        let compressor = Compressor::default();
        let data: &[u8] = b"";

        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_level_clamping() {
        let low = Compressor::new(-5);
        assert_eq!(low.level(), 1);

        let high = Compressor::new(50);
        assert_eq!(high.level(), MAX_COMPRESSION_LEVEL);
    }
}
