//! Reed-Solomon Erasure Coding for WRAITH Vault
//!
//! Provides fault tolerance by encoding data into shards that can
//! be reconstructed even with some shards missing.

use crate::error::{VaultError, VaultResult};
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Default number of data shards
pub const DEFAULT_DATA_SHARDS: usize = 16;

/// Default number of parity shards
pub const DEFAULT_PARITY_SHARDS: usize = 4;

/// A shard of erasure-coded data
#[derive(Clone, Debug)]
pub struct Shard {
    /// Shard index (0..total_shards)
    pub index: usize,
    /// Shard data
    pub data: Vec<u8>,
}

impl Shard {
    /// Create a new shard
    pub fn new(index: usize, data: Vec<u8>) -> Self {
        Self { index, data }
    }
}

/// Reed-Solomon erasure coder
pub struct ErasureCoder {
    /// Number of data shards
    data_shards: usize,
    /// Number of parity shards
    parity_shards: usize,
    /// Reed-Solomon encoder
    encoder: ReedSolomon,
}

impl Default for ErasureCoder {
    fn default() -> Self {
        Self::new(DEFAULT_DATA_SHARDS, DEFAULT_PARITY_SHARDS).unwrap()
    }
}

impl ErasureCoder {
    /// Create a new erasure coder with the specified shard counts
    pub fn new(data_shards: usize, parity_shards: usize) -> VaultResult<Self> {
        let encoder = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| VaultError::ErasureCoding(format!("Failed to create encoder: {}", e)))?;

        Ok(Self {
            data_shards,
            parity_shards,
            encoder,
        })
    }

    /// Get the number of data shards
    pub fn data_shards(&self) -> usize {
        self.data_shards
    }

    /// Get the number of parity shards
    pub fn parity_shards(&self) -> usize {
        self.parity_shards
    }

    /// Get the total number of shards
    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }

    /// Get the minimum number of shards required for reconstruction
    pub fn min_shards_required(&self) -> usize {
        self.data_shards
    }

    /// Encode data into shards
    pub fn encode(&self, data: &[u8]) -> VaultResult<Vec<Shard>> {
        // Handle empty data as a special case - Reed-Solomon doesn't support zero-length data
        if data.is_empty() {
            return Ok((0..self.total_shards())
                .map(|i| Shard::new(i, Vec::new()))
                .collect());
        }

        // Calculate shard size (pad to be divisible by data_shards)
        let shard_size = data.len().div_ceil(self.data_shards);

        // Create data shards
        let mut shards: Vec<Vec<u8>> = (0..self.data_shards)
            .map(|i| {
                let start = i * shard_size;
                let end = std::cmp::min(start + shard_size, data.len());

                if start < data.len() {
                    let mut shard = data[start..end].to_vec();
                    // Pad to shard_size
                    shard.resize(shard_size, 0);
                    shard
                } else {
                    vec![0u8; shard_size]
                }
            })
            .collect();

        // Add empty parity shards
        for _ in 0..self.parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // Compute parity
        self.encoder
            .encode(&mut shards)
            .map_err(|e| VaultError::ErasureCoding(format!("Encoding failed: {}", e)))?;

        // Convert to Shard structs
        let result = shards
            .into_iter()
            .enumerate()
            .map(|(index, data)| Shard::new(index, data))
            .collect();

        Ok(result)
    }

    /// Decode shards back into original data
    pub fn decode(
        &self,
        shards: &mut [Option<Vec<u8>>],
        original_size: usize,
    ) -> VaultResult<Vec<u8>> {
        // Handle empty data as a special case
        if original_size == 0 {
            return Ok(Vec::new());
        }

        // Count available shards
        let available = shards.iter().filter(|s| s.is_some()).count();
        if available < self.data_shards {
            return Err(VaultError::InsufficientShards {
                available,
                required: self.data_shards,
            });
        }

        // Reconstruct missing shards
        self.encoder
            .reconstruct(shards)
            .map_err(|e| VaultError::ErasureCoding(format!("Reconstruction failed: {}", e)))?;

        // Concatenate data shards
        let mut data = Vec::with_capacity(original_size);
        for (i, shard) in shards.iter().enumerate().take(self.data_shards) {
            if let Some(shard_data) = shard {
                data.extend_from_slice(shard_data);
            } else {
                return Err(VaultError::ErasureCoding(format!(
                    "Missing data shard {} after reconstruction",
                    i
                )));
            }
        }

        // Trim to original size
        data.truncate(original_size);

        Ok(data)
    }

    /// Decode from a subset of available shards
    pub fn decode_from_shards(
        &self,
        available_shards: &[Shard],
        original_size: usize,
    ) -> VaultResult<Vec<u8>> {
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; self.total_shards()];

        for shard in available_shards {
            if shard.index < self.total_shards() {
                shards[shard.index] = Some(shard.data.clone());
            }
        }

        self.decode(&mut shards, original_size)
    }

    /// Verify that shards are consistent (parity check)
    pub fn verify(&self, shards: &[Option<Vec<u8>>]) -> VaultResult<bool> {
        let available = shards.iter().filter(|s| s.is_some()).count();
        if available < self.total_shards() {
            // Can't verify with missing shards
            return Ok(false);
        }

        // Convert Option<Vec<u8>> to Vec<u8> for verification
        // Only verify if all shards are present
        let shards_vec: Vec<&[u8]> = shards
            .iter()
            .filter_map(|s| s.as_ref().map(|v| v.as_slice()))
            .collect();

        if shards_vec.len() != self.total_shards() {
            return Ok(false);
        }

        self.encoder
            .verify(&shards_vec)
            .map_err(|e| VaultError::ErasureCoding(format!("Verification failed: {}", e)))
    }

    /// Calculate overhead ratio (total shards / data shards)
    pub fn overhead_ratio(&self) -> f64 {
        self.total_shards() as f64 / self.data_shards as f64
    }

    /// Calculate fault tolerance (number of shards that can be lost)
    pub fn fault_tolerance(&self) -> usize {
        self.parity_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let coder = ErasureCoder::default();
        let data = vec![1u8; 1024]; // 1 KB

        // Encode
        let shards = coder.encode(&data).unwrap();
        assert_eq!(shards.len(), 20); // 16 + 4

        // Convert to Option format
        let mut shard_opts: Vec<Option<Vec<u8>>> =
            shards.into_iter().map(|s| Some(s.data)).collect();

        // Decode
        let recovered = coder.decode(&mut shard_opts, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_encode_decode_with_loss() {
        let coder = ErasureCoder::default();
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();

        // Encode
        let shards = coder.encode(&data).unwrap();

        // Simulate loss of 4 shards (maximum tolerable)
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                // Lose shards 0, 5, 10, 15
                if i % 5 == 0 { None } else { Some(s.data) }
            })
            .collect();

        // Decode should still work
        let recovered = coder.decode(&mut shard_opts, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_insufficient_shards() {
        let coder = ErasureCoder::default();
        let data = vec![1u8; 1024];

        let shards = coder.encode(&data).unwrap();

        // Lose 5 shards (more than parity shards)
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards
            .into_iter()
            .enumerate()
            .map(|(i, s)| if i < 5 { None } else { Some(s.data) })
            .collect();

        // Decode should fail
        let result = coder.decode(&mut shard_opts, data.len());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_from_shards() {
        let coder = ErasureCoder::default();
        let data = vec![42u8; 5000];

        let all_shards = coder.encode(&data).unwrap();

        // Only use data shards (minimum required)
        let available: Vec<Shard> = all_shards.into_iter().take(16).collect();

        let recovered = coder.decode_from_shards(&available, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_verify_shards() {
        let coder = ErasureCoder::default();
        let data = vec![1u8; 1024];

        let shards = coder.encode(&data).unwrap();
        let shard_opts: Vec<Option<Vec<u8>>> = shards.into_iter().map(|s| Some(s.data)).collect();

        let is_valid = coder.verify(&shard_opts).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_custom_shard_counts() {
        // 8 data + 2 parity (can lose 2 shards)
        let coder = ErasureCoder::new(8, 2).unwrap();

        assert_eq!(coder.data_shards(), 8);
        assert_eq!(coder.parity_shards(), 2);
        assert_eq!(coder.total_shards(), 10);
        assert_eq!(coder.min_shards_required(), 8);
        assert_eq!(coder.fault_tolerance(), 2);
    }

    #[test]
    fn test_overhead_ratio() {
        let coder = ErasureCoder::new(16, 4).unwrap();
        let ratio = coder.overhead_ratio();
        assert!((ratio - 1.25).abs() < 0.01);
    }

    #[test]
    fn test_empty_data() {
        let coder = ErasureCoder::default();
        let data: Vec<u8> = vec![];

        let shards = coder.encode(&data).unwrap();
        let mut shard_opts: Vec<Option<Vec<u8>>> =
            shards.into_iter().map(|s| Some(s.data)).collect();

        let recovered = coder.decode(&mut shard_opts, 0).unwrap();
        assert!(recovered.is_empty());
    }
}
