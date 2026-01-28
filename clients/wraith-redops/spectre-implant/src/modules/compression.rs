//! Tactic: Collection (TA0009)
//! Technique: T1560.001 (Archive via Utility - Data Compressed)

use alloc::vec::Vec;

pub struct Compression;

impl Compression {
    /// T1560.001: Data Compressed - Simple RLE compression for demonstration.
    pub fn compress(&self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut compressed = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let mut count = 1;
            while i + count < data.len() && data[i + count] == data[i] && count < 255 {
                count += 1;
            }

            compressed.push(count as u8);
            compressed.push(data[i]);
            i += count;
        }

        compressed
    }

    /// Decompress RLE data.
    pub fn decompress(&self, data: &[u8]) -> Vec<u8> {
        let mut decompressed = Vec::new();
        let mut i = 0;

        while i + 1 < data.len() {
            let count = data[i] as usize;
            let val = data[i + 1];
            for _ in 0..count {
                decompressed.push(val);
            }
            i += 2;
        }

        decompressed
    }
}
