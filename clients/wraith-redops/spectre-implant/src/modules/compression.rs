//! Tactic: Collection (TA0009)
//! Technique: T1560.001 (Archive via Utility - Data Compressed)

use alloc::vec::Vec;

pub struct Compression;

impl Compression {
    /// T1560.001: Data Compressed - DEFLATE compression.
    pub fn compress(&self, data: &[u8]) -> Vec<u8> {
        miniz_oxide::deflate::compress_to_vec(data, 6)
    }

    /// Decompress DEFLATE data.
    pub fn decompress(&self, data: &[u8]) -> Vec<u8> {
        miniz_oxide::inflate::decompress_to_vec(data).unwrap_or_default()
    }
}
